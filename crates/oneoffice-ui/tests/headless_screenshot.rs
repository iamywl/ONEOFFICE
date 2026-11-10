#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

#[cfg(test)]
mod tests {
    use egui_kittest::Harness;
    use oneoffice_ui::studio::ActiveDocumentKind;
    use oneoffice_ui::theme::StudioTheme;
    use oneoffice_ui::StudioApp;
    use std::path::{Path, PathBuf};

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
        panic!("Document not found: {} in {:?}", name, candidates);
    }

    fn verify_and_save_screenshot(harness: &mut Harness<'_, StudioApp>, filename: &str) {
        harness.run_steps(3);
        let img = harness.render().expect("Headless render failed");
        assert_eq!(img.width(), 1440);
        assert!(img.height() >= 900);

        // Verify non-black pixels and rich UI color depth
        let mut non_black_pixels = 0usize;
        let mut unique_colors = std::collections::HashSet::new();
        for pixel in img.pixels() {
            let [r, g, b, _] = pixel.0;
            if r > 10 || g > 10 || b > 10 {
                non_black_pixels += 1;
            }
            if unique_colors.len() < 100 {
                unique_colors.insert(pixel.0);
            }
        }

        println!(
            "[{}] Headless Render: {}/{} non-black pixels ({} unique colors)",
            filename,
            non_black_pixels,
            img.width() * img.height(),
            unique_colors.len()
        );
        assert!(
            non_black_pixels > 400_000,
            "Rendered screenshot [{}] must not be black! Found only {} non-black pixels",
            filename,
            non_black_pixels
        );
        assert!(
            unique_colors.len() >= 10,
            "Rendered screenshot [{}] must contain rich UI styling colors",
            filename
        );

        let out_dir = Path::new("../../docs/assets/qa");
        let _ = std::fs::create_dir_all(out_dir);
        let out_path = out_dir.join(filename);
        let save_res = img.save(&out_path);
        assert!(
            save_res.is_ok(),
            "Failed to save {}: {:?}",
            filename,
            save_res.err()
        );
    }

    #[test]
    fn test_headless_hwpx_studio() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Hwpx;
        app.document_title = "2026 차세대 사업계획서.hwpx".to_string();
        let bytes = find_doc("dummy_proposal.hwpx");
        app.load_hwpx_bytes(&bytes);

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 900.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "headless_hwpx_studio.png");
    }

    #[test]
    fn test_headless_docx_studio() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Docx;
        app.document_title = "표준용역계약서_실측보고.docx".to_string();
        let bytes = find_doc("dummy_report.docx");

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 900.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                if state.doc_textures.is_empty() {
                    let ctx = ui.ctx().clone();
                    state.load_docx_bytes(&ctx, &bytes);
                }
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "headless_docx_studio.png");
    }

    #[test]
    fn test_headless_xlsx_studio() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Xlsx;
        app.document_title = "2026_클라우드_사업예산.xlsx".to_string();
        let bytes = find_doc("dummy_budget.xlsx");
        app.load_xlsx_bytes(&bytes);

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 900.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "headless_xlsx_studio.png");
    }

    #[test]
    fn test_headless_pptx_studio() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Pptx;
        app.document_title = "ONEOFFICE_통합제안.pptx".to_string();
        let bytes = find_doc("dummy_presentation.pptx");
        app.load_pptx_bytes(&bytes);

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 900.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "headless_pptx_studio.png");
    }

    #[test]
    fn test_headless_pdf_studio() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Pdf;
        app.document_title = "전자서명_용역계약서.pdf".to_string();

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 900.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                let ctx = ui.ctx().clone();
                state.pdf_widget.ensure_sample_loaded(&ctx);
                let _ = state.pdf_widget.auto_stamp_anchor();
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "headless_pdf_studio.png");
    }

    #[test]
    fn test_headless_light_theme() {
        let mut app = StudioApp::new();
        app.theme = StudioTheme::Light;
        app.active_doc = ActiveDocumentKind::Hwpx;
        app.document_title = "2026 차세대 사업계획서 (Light Theme).hwpx".to_string();

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 900.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "headless_studio_light.png");
    }

    #[test]
    fn test_headless_find_replace_hud() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Xlsx;
        app.document_title = "2026_클라우드_사업예산 (Find & Replace).xlsx".to_string();
        app.find_replace.is_open = true;
        app.find_replace.find_query = "예산".to_string();
        app.find_replace.replace_query = "투자금액".to_string();

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 900.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "headless_find_replace_hud.png");
    }

    #[test]
    fn test_headless_table_and_diagram_rich() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Hwpx;
        app.document_title = "표와_도식화_통합제안서.hwpx".to_string();
        app.doc_model = None;
        app.table_data = vec![
            vec!["항목".into(), "단위".into(), "수량".into(), "단가(원)".into(), "소계(원)".into()],
            vec!["클라우드 인프라".into(), "식".into(), "1".into(), "12500000".into(), "12500000".into()],
            vec!["엔진 라이선스".into(), "카피".into(), "5".into(), "3000000".into(), "15000000".into()],
            vec!["유지보수 기술지원".into(), "월".into(), "12".into(), "500000".into(), "6000000".into()],
            vec!["합계 (SUM)".into(), "-".into(), "-".into(), "-".into(), "33500000".into()],
        ];
        app.recalculate_table_sums();
        app.chart_widget.is_visible = true;
        app.chart_widget.selected_kind = "column_clustered".to_string();

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1200.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "headless_rich_table_diagram.png");
    }
}
