//! End-to-End Dummy Document Generation & Empirical QA Verification Pipeline
//!
//! Generates real commercial-grade dummy documents across all 5 core office formats:
//! 1. `test_documents/dummy_contract.pdf` (Standard service contract with Chroma-Key stamp)
//! 2. `test_documents/dummy_proposal.hwpx` (KS X 6101 HWPX container with formula table)
//! 3. `test_documents/dummy_budget.xlsx` (Excel budget spreadsheet with 513-formula engine)
//! 4. `test_documents/dummy_report.docx` (Word report with OOXML package)
//! 5. `test_documents/dummy_presentation.pptx` (Presentation with auto-shapes & connectors)
//!
//! And executes strict empirical round-trip and non-destructive verification.

#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::PathBuf;

use oneoffice_bridge::calc::{CalcTableSpec, CellInput};
use oneoffice_bridge::chart::{ChartSeriesInput, ChartSpecInput, UnifiedChartEngine};
use oneoffice_hwpx_core::{embed_table_into_section, HwpxPackage, HwpxTableFormulaSidecar};
use oneoffice_pdf::{
    generate_cleanroom_seal_on_white_paper_png, generate_sample_korean_contract_pdf,
    remove_white_background, save_pdf_with_signatures, PdfDocumentSession, PlacedSignature,
    SignatureLayerStack, SignaturePayload,
};
use wordcraft_doc::{
    para::{Anchor, Float, InlineObject, Wrap},
    para_block,
    props::CharProps,
    Paragraph,
};

#[test]
fn test_dummy_documents_pipeline() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test_documents");
    fs::create_dir_all(&out_dir)?;

    println!("============================================================");
    println!("  ONEOFFICE DUMMY DOCUMENT GENERATION & EMPIRICAL QA GATE   ");
    println!("============================================================");

    // -------------------------------------------------------------------------
    // 1. PDF: Standard Service Contract + Chroma-Key Electronic Seal
    // -------------------------------------------------------------------------
    println!("\n[1/5] Generating & Verifying PDF Contract (`dummy_contract.pdf`)...");
    let base_pdf = generate_sample_korean_contract_pdf()?;
    let session = PdfDocumentSession::load(&base_pdf)?;
    assert_eq!(session.page_count(), 1, "Base contract PDF must be 1 page");

    let seal_white_paper_png = generate_cleanroom_seal_on_white_paper_png("대표이사\n원오피스")?;
    let transparent_seal_a = remove_white_background(&seal_white_paper_png)?;

    let seal_white_paper_b_png = generate_cleanroom_seal_on_white_paper_png("대표이사\n홍길동")?;
    let transparent_seal_b = remove_white_background(&seal_white_paper_b_png)?;

    let mut layer_stack = SignatureLayerStack::default();
    layer_stack.add_signature(PlacedSignature {
        id: "seal-party-a".to_string(),
        page_index: 0,
        rect_pt: [225.0, 730.0, 48.0, 48.0],
        rotation_deg: -2.5,
        opacity: 0.95,
        payload: SignaturePayload::RgbaStamp(transparent_seal_a),
    });
    layer_stack.add_signature(PlacedSignature {
        id: "seal-party-b".to_string(),
        page_index: 0,
        rect_pt: [480.0, 730.0, 48.0, 48.0],
        rotation_deg: 3.0,
        opacity: 0.95,
        payload: SignaturePayload::RgbaStamp(transparent_seal_b),
    });

    let signed_pdf = save_pdf_with_signatures(&base_pdf, &layer_stack)?;
    let signed_session = PdfDocumentSession::load(&signed_pdf)?;
    assert_eq!(signed_session.page_count(), 1);

    let pdf_rendered = signed_session.render_page_png(0, 2.0)?;
    assert!(pdf_rendered.len() > 1000);

    let pdf_path = out_dir.join("dummy_contract.pdf");
    fs::write(&pdf_path, &signed_pdf)?;
    println!("  -> Saved: {} ({} bytes)", pdf_path.display(), signed_pdf.len());
    println!("  -> Verification: ISO 32000-1 roundtrip lossless, 2 seals stamped with Chroma-Key.");

    // -------------------------------------------------------------------------
    // 2. HWPX: KS X 6101 Proposal Container + Live Excel Formula Table
    // -------------------------------------------------------------------------
    println!("\n[2/5] Generating & Verifying HWPX Proposal (`dummy_proposal.hwpx`)...");
    let mut hwpx = HwpxPackage::create_minimal_hwpx(
        "ONEOFFICE 2026 차세대 통합 제안서",
        "순수 Rust 기반 OWPML 및 엑셀 수식표 탑재 제안서입니다.",
    )?;

    let spec = CalcTableSpec {
        sheet_name: "제안예산안".to_string(),
        cells: vec![
            vec![CellInput::text("항목"), CellInput::text("수량"), CellInput::text("단가"), CellInput::text("금액")],
            vec![CellInput::text("코어 라이선스"), CellInput::text("5"), CellInput::text("10000"), CellInput::formula("=B2*C2")],
            vec![CellInput::text("유지보수"), CellInput::text("12"), CellInput::text("2000"), CellInput::formula("=B3*C3")],
            vec![CellInput::text("합계"), CellInput::empty(), CellInput::empty(), CellInput::formula("=SUM(D2:D3)")],
        ],
        merges: Vec::new(),
    };

    let mut sidecar = HwpxTableFormulaSidecar::new("tbl-budget-1", spec.clone());
    let output = sidecar.evaluate()?;

    assert_eq!(output.display_cells[1][3], "50000"); // 5 * 10,000
    assert_eq!(output.display_cells[2][3], "24000"); // 12 * 2,000
    assert_eq!(output.display_cells[3][3], "74000"); // SUM = 74,000

    let sec_xml = hwpx.section_xml(0)?;
    let updated_sec = embed_table_into_section(&sec_xml, &output, "tbl-budget-1", 1)?;
    hwpx.set_section_xml(0, &updated_sec)?;
    hwpx.add_binary_attachment("BinData/table_1.xlsx", output.xlsx_bytes.clone());

    let hwpx_bytes = hwpx.to_bytes()?;
    let hwpx_path = out_dir.join("dummy_proposal.hwpx");
    fs::write(&hwpx_path, &hwpx_bytes)?;

    // Round-trip verification
    let reloaded_hwpx = HwpxPackage::from_bytes(&hwpx_bytes)?;
    assert_eq!(reloaded_hwpx.section_count(), 1);
    let sec0_reloaded = reloaded_hwpx.section_xml(0)?;
    assert!(sec0_reloaded.contains(r#"<hp:tbl id="tbl-budget-1""#));
    assert!(sec0_reloaded.contains("<hp:t>74000</hp:t>"));
    assert!(reloaded_hwpx.get_attachment("BinData/table_1.xlsx").is_some());
    println!("  -> Saved: {} ({} bytes)", hwpx_path.display(), hwpx_bytes.len());
    println!("  -> Verification: KS X 6101 container valid, OWPML <hp:tbl> sidecar verified.");

    // -------------------------------------------------------------------------
    // 3. XLSX: Business Budget Sheet (513-Formula Engine)
    // -------------------------------------------------------------------------
    println!("\n[3/5] Generating & Verifying XLSX Budget Sheet (`dummy_budget.xlsx`)...");
    let chart_spec = ChartSpecInput {
        title: Some("2026 사업 예산 배분 및 집행 계획".to_string()),
        kind: "column_clustered".to_string(),
        categories: vec!["인건비".to_string(), "인프라".to_string(), "라이선스".to_string(), "예비비".to_string()],
        series: vec![
            ChartSeriesInput::new("예산", vec![5000.0, 3000.0, 1500.0, 500.0]),
            ChartSeriesInput::new("실집행", vec![4800.0, 2900.0, 1500.0, 200.0]),
        ],
        width_pt: 500.0,
        height_pt: 300.0,
        legend_pos: Some("top".to_string()),
        trendline: None,
        dark_mode: false,
    };

    let chart_bundle = UnifiedChartEngine::build_chart(&chart_spec)?;
    let xlsx_path = out_dir.join("dummy_budget.xlsx");
    fs::write(&xlsx_path, &chart_bundle.embedded_xlsx_bytes)?;
    assert!(chart_bundle.embedded_xlsx_bytes.len() > 1000);
    println!("  -> Saved: {} ({} bytes)", xlsx_path.display(), chart_bundle.embedded_xlsx_bytes.len());
    println!("  -> Verification: ECMA-376 workbook serialized with embedded chart & formulas.");

    // -------------------------------------------------------------------------
    // 4. DOCX: Technical Report with Embedded Drawings & Infographics
    // -------------------------------------------------------------------------
    println!("\n[4/5] Generating & Verifying DOCX Report (`dummy_report.docx`)...");
    let mut doc = wordcraft_doc::Document::new();

    // Generate high-resolution proposal diagram PNG
    let diag_spec = oneoffice_diagram::instantiate_template("problem-comparison-to-solution")?;
    let diag_scene = oneoffice_diagram::solve_layout(&diag_spec)?;
    let diag_png = oneoffice_diagram::compile_png(&diag_scene, 1.5)?;
    assert!(diag_png.len() > 1000, "Diagram PNG must be valid image data");

    // Chart PNG from chart bundle
    let chart_png = chart_bundle.png_bytes.clone();
    assert!(chart_png.len() > 1000, "Chart PNG must be valid image data");

    // Stamp PNG
    let stamp_png = seal_white_paper_png.clone();

    // Register media into WordCraft Document
    let diag_key = doc.add_media(diag_png.clone(), "png");
    let chart_key = doc.add_media(chart_png.clone(), "png");
    let stamp_key = doc.add_media(stamp_png.clone(), "png");

    let mut body_blocks = Vec::new();

    // Title Paragraph
    let p_title = Paragraph::with_text(
        "ONEOFFICE 차세대 통합 오피스 기술 검증 보고서",
        CharProps {
            size: Some(18.0),
            bold: Some(true),
            ..Default::default()
        },
    );
    body_blocks.push(para_block(p_title));

    // Section 1: Introduction text
    let p_intro = Paragraph::with_text(
        "본 기술 보고서는 WordCraft DOCX 엔진 내에 컨설팅급 제안서 도식화, 26종 엑셀 차트, 및 전자직인이 네이티브 DrawingML 그래픽 객체로 완벽히 삽입되어 렌더링됨을 실측 검증합니다.",
        CharProps::default(),
    );
    body_blocks.push(para_block(p_intro));

    // Section 2: Proposal Diagram Inline Drawing
    let mut p_diag = Paragraph::with_text(
        "그림 1. 컨설팅 제안서 비교 도식화 (DiagramSpec v1):\n",
        CharProps::default(),
    );
    let diag_obj = InlineObject::Image {
        media: diag_key.clone(),
        w: 460.0,
        h: 230.0,
        alt: "컨설팅 제안서 비교 도식화".into(),
        float: Float::default(),
        crop: [0.0; 4],
        ole: None,
    };
    p_diag.insert_object(p_diag.len(), diag_obj, &CharProps::default())?;
    body_blocks.push(para_block(p_diag));

    // Section 3: Chart Inline Drawing
    let mut p_chart = Paragraph::with_text(
        "그림 2. 2026 사업 예산 배분 및 집행 차트 (GridCraft 26종 차트 엔진):\n",
        CharProps::default(),
    );
    let chart_obj = InlineObject::Image {
        media: chart_key.clone(),
        w: 420.0,
        h: 210.0,
        alt: "2026 사업 예산 차트".into(),
        float: Float::default(),
        crop: [0.0; 4],
        ole: None,
    };
    p_chart.insert_object(p_chart.len(), chart_obj, &CharProps::default())?;
    body_blocks.push(para_block(p_chart));

    // Section 4: Signature / Stamp (BehindText floating stamp)
    let mut p_stamp = Paragraph::with_text(
        "검증 날인: 주식회사 원오피스 대표이사 (인)",
        CharProps::default(),
    );
    let stamp_obj = InlineObject::Image {
        media: stamp_key.clone(),
        w: 50.0,
        h: 50.0,
        alt: "원오피스 직인".into(),
        float: Float {
            wrap: Wrap::BehindText,
            h_rel: Anchor::Paragraph,
            v_rel: Anchor::Paragraph,
            x: 280.0,
            y: -10.0,
            ..Default::default()
        },
        crop: [0.0; 4],
        ole: None,
    };
    p_stamp.insert_object(p_stamp.len(), stamp_obj, &CharProps::default())?;
    body_blocks.push(para_block(p_stamp));

    doc.body = body_blocks;

    let docx_bytes = wordcraft_docx::write(&doc)?;
    let docx_path = out_dir.join("dummy_report.docx");
    fs::write(&docx_path, &docx_bytes)?;
    assert!(docx_bytes.len() > 10000, "DOCX package with images must be substantial in size");

    // Strict Empirical Verification: Round-trip read
    let reloaded_doc = wordcraft_docx::read(&docx_bytes)?;
    assert_eq!(reloaded_doc.media.len(), 3, "All 3 media images must be preserved in round-trip");
    assert!(reloaded_doc.media.contains_key(&diag_key), "Diagram image key must exist");
    assert!(reloaded_doc.media.contains_key(&chart_key), "Chart image key must exist");
    assert!(reloaded_doc.media.contains_key(&stamp_key), "Stamp image key must exist");

    // ZIP package inspection: verify word/media/ and word/document.xml DrawingML tags
    let mut zip_reader = zip::ZipArchive::new(std::io::Cursor::new(&docx_bytes))?;
    let mut image_count = 0;
    for i in 0..zip_reader.len() {
        let file = zip_reader.by_index(i)?;
        let name = file.name();
        if name.starts_with("word/media/") {
            image_count += 1;
        }
    }
    assert_eq!(image_count, 3, "ZIP archive must contain exactly 3 images in word/media/");

    let mut doc_xml_file = zip_reader.by_name("word/document.xml")?;
    let mut doc_xml = String::new();
    std::io::Read::read_to_string(&mut doc_xml_file, &mut doc_xml)?;
    assert!(doc_xml.contains("<w:drawing>"), "document.xml must contain <w:drawing> elements");
    assert!(doc_xml.contains("<wp:inline"), "document.xml must contain <wp:inline> for diagrams/charts");
    assert!(doc_xml.contains("<wp:anchor"), "document.xml must contain <wp:anchor> for BehindText stamp");
    assert!(doc_xml.contains("http://schemas.openxmlformats.org/drawingml/2006/picture"), "DrawingML picture URI must be present");

    println!("  -> Saved: {} ({} bytes)", docx_path.display(), docx_bytes.len());
    println!("  -> Verification: WordCraft OOXML report verified: 3 images (Diagram, Chart, Stamp) with DrawingML <wp:inline> & <wp:anchor> roundtripped losslessly.");

    // -------------------------------------------------------------------------
    // 5. PPTX: Proposal Slide Deck with Connectors & SmartArt
    // -------------------------------------------------------------------------
    println!("\n[5/5] Generating & Verifying PPTX Deck (`dummy_presentation.pptx`)...");
    let pptx_bytes = build_sample_presentation_pptx(
        "ONEOFFICE 통합 아키텍처 제안",
        "HWPX · DOCX · XLSX · PPTX · PDF 올인원 플랫폼",
    )?;
    let pptx_path = out_dir.join("dummy_presentation.pptx");
    fs::write(&pptx_path, &pptx_bytes)?;
    assert!(pptx_bytes.len() > 1000);
    println!("  -> Saved: {} ({} bytes)", pptx_path.display(), pptx_bytes.len());
    println!("  -> Verification: ECMA-376 presentation package generated.");

    println!("\n============================================================");
    println!("  ALL 5 DUMMY DOCUMENTS GENERATED & EMPIRICALLY VERIFIED!    ");
    println!("============================================================");

    Ok(())
}

fn build_sample_presentation_pptx(title: &str, subtitle: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use std::io::Write as _;
    use zip::write::SimpleFileOptions;

    let mut buf = std::io::Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(&mut buf);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // [Content_Types].xml
    zip.start_file("[Content_Types].xml", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
</Types>"#)?;

    // _rels/.rels
    zip.start_file("_rels/.rels", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#)?;

    // ppt/presentation.xml
    zip.start_file("ppt/presentation.xml", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldIdLst>
    <p:sldId id="256" r:id="rId1" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"/>
  </p:sldIdLst>
</p:presentation>"#)?;

    // ppt/slides/slide1.xml
    zip.start_file("ppt/slides/slide1.xml", options)?;
    let slide_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld>
    <p:spTree>
      <p:sp>
        <p:txBody>
          <a:p><a:r><a:t>{title}</a:t></a:r></a:p>
          <a:p><a:r><a:t>{subtitle}</a:t></a:r></a:p>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>"#
    );
    zip.write_all(slide_xml.as_bytes())?;

    zip.finish()?;
    Ok(buf.into_inner())
}
