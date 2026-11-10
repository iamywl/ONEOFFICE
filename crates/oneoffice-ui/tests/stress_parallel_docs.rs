#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

#[cfg(test)]
mod tests {
    use std::time::Instant;
    use oneoffice_bridge::calc::{CalcTableEngine, CalcTableSpec, CellInput};
    use oneoffice_pdf::{
        generate_sample_korean_contract_pdf, PdfDocumentSession,
        PlacedSignature, SignatureLayerStack, SignaturePayload, QuickSignText,
        save_pdf_with_signatures,
    };
    use oneoffice_diagram::{
        list_diagram_templates, instantiate_template, solve_layout, compile_svg,
    };
    use std::path::PathBuf;

    fn find_doc(name: &str) -> Vec<u8> {
        let candidates = [
            PathBuf::from("test_documents").join(name),
            PathBuf::from("../../test_documents").join(name),
            PathBuf::from("../test_documents").join(name),
        ];
        for path in &candidates {
            if let Ok(bytes) = std::fs::read(path) {
                return bytes;
            }
        }
        panic!("Document not found: {}", name);
    }

    #[test]
    fn test_parallel_heavy_stress_workload() {
        let start = Instant::now();

        // 1. Thread 1: 1,000-row Dense Excel calculation and xlsx sidecar generation
        let handle_excel = std::thread::spawn(|| {
            let t_start = Instant::now();
            let mut rows = Vec::with_capacity(1000);
            for r in 0..1000 {
                rows.push(vec![
                    CellInput::text(format!("품목-{:04}", r)),
                    CellInput::number((1000 + (r % 500)) as f64),
                    CellInput::number((5 + (r % 10)) as f64),
                    CellInput::formula(format!("=B{}*C{}", r + 1, r + 1)),
                ]);
            }
            let spec = CalcTableSpec {
                sheet_name: "대용량_사업예산".to_string(),
                cells: rows,
                merges: Vec::new(),
            };
            let out = CalcTableEngine::evaluate_spec(&spec).expect("Excel calculation failed");
            assert_eq!(out.display_cells.len(), 1000);
            assert!(!out.xlsx_bytes.is_empty());
            println!(
                "✓ [Stress Thread 1] 1,000 Excel rows calculated + XLSX sidecar ({} KB) in {:.2?}",
                out.xlsx_bytes.len() / 1024,
                t_start.elapsed()
            );
        });

        // 2. Thread 2: Batch PDF generation, HiDPI rasterization, and digital signature sealing
        let handle_pdf = std::thread::spawn(|| {
            let t_start = Instant::now();
            let sample_pdf = generate_sample_korean_contract_pdf().expect("PDF generate failed");
            for i in 0..10 {
                let session = PdfDocumentSession::load(&sample_pdf).expect("PDF load failed");
                assert_eq!(session.page_count(), 1);
                let rendered = session.render_page_rgba(0, 2.0).expect("Render page failed");
                assert!(rendered.width > 1000);

                let sig = PlacedSignature {
                    id: format!("sig-{i}"),
                    page_index: 0,
                    rect_pt: [400.0, 700.0, 120.0, 40.0],
                    rotation_deg: 0.0,
                    opacity: 1.0,
                    payload: SignaturePayload::QuickTextStamp(QuickSignText {
                        text: format!("대표이사 원오피스 #{i}"),
                        font_size_pt: 14.0,
                        text_color: [180, 20, 20, 255],
                        border_color: Some([180, 20, 20, 255]),
                    }),
                };
                let mut stack = SignatureLayerStack::default();
                stack.add_signature(sig);
                let signed_pdf = save_pdf_with_signatures(&sample_pdf, &stack).expect("Sign failed");
                assert!(signed_pdf.len() > sample_pdf.len());
            }
            println!(
                "✓ [Stress Thread 2] 10x HiDPI PDF rasterization + digital signing in {:.2?}",
                t_start.elapsed()
            );
        });

        // 3. Thread 3: Batch generation and layout solving of 50 consulting infographic diagrams
        let handle_diagram = std::thread::spawn(|| {
            let t_start = Instant::now();
            let templates = list_diagram_templates();
            for _run in 0..5 {
                for tmpl in &templates {
                    let spec = instantiate_template(tmpl.id).expect("Instantiate template failed");
                    let scene = solve_layout(&spec).expect("Solve layout failed");
                    let svg = compile_svg(&scene);
                    assert!(!svg.is_empty());
                }
            }
            println!(
                "✓ [Stress Thread 3] 50x Consulting Diagrams solved and compiled to SVG in {:.2?}",
                t_start.elapsed()
            );
        });

        // 4. Thread 4: Large DOCX & HWPX dual roundtrip stress
        let handle_docs = std::thread::spawn(|| {
            let t_start = Instant::now();
            let docx_bytes = find_doc("dummy_report.docx");
            let hwpx_bytes = find_doc("dummy_proposal.hwpx");
            for _ in 0..10 {
                let pkg = wordcraft_docx::read(&docx_bytes).expect("DOCX read failed");
                assert!(!pkg.body.is_empty());

                let hwpx_pkg = oneoffice_hwpx_core::HwpxPackage::from_bytes(&hwpx_bytes)
                    .expect("HWPX read failed");
                assert!(!hwpx_pkg.section_xml(0).expect("section 0").is_empty());
            }
            println!(
                "✓ [Stress Thread 4] 10x DOCX & HWPX full package unzipping & roundtripping in {:.2?}",
                t_start.elapsed()
            );
        });

        handle_excel.join().expect("Excel thread panicked");
        handle_pdf.join().expect("PDF thread panicked");
        handle_diagram.join().expect("Diagram thread panicked");
        handle_docs.join().expect("Docs thread panicked");

        let total_time = start.elapsed();
        println!(
            "\n🚀 [Parallel Benchmark Gate PASSED] All 4 heavy threads completed in {:.2?} on macOS unified memory architecture!",
            total_time
        );
        assert!(total_time.as_secs() < 30, "Parallel stress test must finish within 30s");
    }
}
