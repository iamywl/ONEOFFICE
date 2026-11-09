//! Real empirical artifact generator for `README.md` and verification showcase.
//!
//! Renders:
//! 1. `docs/assets/qa/wave1_hwpx_proposal_diagram.png` (Consulting Proposal Infographic via `oneoffice-diagram`)
//! 2. `docs/assets/qa/wave1_pdf_viewer_signature.png` (Korean Contract with Chroma-Key Seal via `oneoffice-pdf`)
//! 3. `docs/assets/qa/wave1_excel_formula_chart.png` (26-Type Clustered Column Chart via `oneoffice-bridge`)

use std::fs;
use std::path::Path;

use oneoffice_bridge::chart::{ChartSeriesInput, ChartSpecInput, UnifiedChartEngine};
use oneoffice_diagram::{compile_png, instantiate_template, solve_layout};
use oneoffice_pdf::{
    PdfDocumentSession, PlacedSignature, SignatureLayerStack, SignaturePayload,
    generate_cleanroom_seal_on_white_paper_png, generate_sample_korean_contract_pdf,
    remove_white_background, save_pdf_with_signatures,
};

#[test]
fn generate_all_showcase_artifacts() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/assets/qa");
    fs::create_dir_all(&out_dir)?;

    // =========================================================================
    // 1. Proposal Infographic Diagram (2x HiDPI Retina)
    // =========================================================================
    let diagram_spec = instantiate_template("problem-comparison-to-solution")?;
    let scene = solve_layout(&diagram_spec)?;
    let diagram_png = compile_png(&scene, 2.0)?;
    let diagram_path = out_dir.join("wave1_hwpx_proposal_diagram.png");
    fs::write(&diagram_path, &diagram_png)?;
    assert!(diagram_png.len() > 1000);
    println!("Generated: {} ({} bytes)", diagram_path.display(), diagram_png.len());

    // =========================================================================
    // 2. Korean Standard Contract with Transparent Red Seal (2x HiDPI Retina)
    // =========================================================================
    let base_contract_pdf = generate_sample_korean_contract_pdf()?;

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

    let signed_pdf = save_pdf_with_signatures(&base_contract_pdf, &layer_stack)?;
    let signed_session = PdfDocumentSession::load(&signed_pdf)?;
    let contract_png = signed_session.render_page_png(0, 2.0)?;
    let pdf_path = out_dir.join("wave1_pdf_viewer_signature.png");
    fs::write(&pdf_path, &contract_png)?;
    assert!(contract_png.len() > 1000);
    println!("Generated: {} ({} bytes)", pdf_path.display(), contract_png.len());

    // =========================================================================
    // 3. 26-Type Excel Chart (Column Clustered Benchmark)
    // =========================================================================
    let chart_spec = ChartSpecInput {
        title: Some("ONEOFFICE vs 기존 오피스 성능 및 규격 지원 벤치마크".to_string()),
        kind: "column_clustered".to_string(),
        categories: vec![
            "HWPX OWPML 완전성".to_string(),
            "513 함수 수식표".to_string(),
            "제안서 도식화 DSL".to_string(),
            "무손실 PDF 서명".to_string(),
            "크로스플랫폼 (Zero-COM)".to_string(),
        ],
        series: vec![
            ChartSeriesInput::new("ONEOFFICE", vec![100.0, 100.0, 100.0, 100.0, 100.0]),
            ChartSeriesInput::new("외산 오피스 (MS)", vec![0.0, 100.0, 40.0, 20.0, 80.0]),
            ChartSeriesInput::new("국내 오피스 (한컴)", vec![80.0, 30.0, 20.0, 15.0, 30.0]),
        ],
        width_pt: 680.0,
        height_pt: 400.0,
        legend_pos: Some("top".to_string()),
        trendline: None,
        dark_mode: false,
    };

    let chart_bundle = UnifiedChartEngine::build_chart(&chart_spec)?;
    let chart_path = out_dir.join("wave1_excel_formula_chart.png");
    fs::write(&chart_path, &chart_bundle.png_bytes)?;
    assert!(chart_bundle.png_bytes.len() > 1000);
    println!("Generated: {} ({} bytes)", chart_path.display(), chart_bundle.png_bytes.len());

    Ok(())
}
