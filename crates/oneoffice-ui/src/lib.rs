//! Unified ONEOFFICE Frosted Glass Studio UI components.

#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable
)]

pub mod diagram_widget;
pub mod formula_hud;
pub mod pdf_widget;
pub mod studio;
pub mod theme;

pub use diagram_widget::DiagramWidget;
pub use formula_hud::{FormulaHudEvent, FormulaHudWidget};
pub use pdf_widget::PdfViewerWidget;
pub use studio::{ActiveDocumentKind, StudioApp};
pub use theme::StudioTheme;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_studio_app_initialization() {
        let app = StudioApp::new();
        assert_eq!(app.active_doc, ActiveDocumentKind::Hwpx);
        assert_eq!(app.theme, StudioTheme::Dark);
        assert!(!app.formula_hud.cell_address.is_empty());
        assert!(app.diagram_widget.last_scene.is_some());
        assert!(app.spreadsheet_model.is_some());
        assert!(app.presentation_model.is_some());
    }

    #[test]
    fn test_docx_parsing_and_embedded_images() {
        let mut app = StudioApp::new();
        for path in [
            "test_documents/dummy_report.docx",
            "../../test_documents/dummy_report.docx",
        ] {
            if let Ok(bytes) = std::fs::read(path) {
                app.parse_docx_bytes_headless(&bytes);
                break;
            }
        }

        if let Some(doc) = &app.doc_model {
            assert!(!doc.paragraphs.is_empty(), "Parsed DOCX must have text paragraphs");
            assert_eq!(doc.images.len(), 3, "Parsed DOCX must have 3 images (Diagram, Chart, Stamp)");
        }
    }

    #[test]
    fn test_xlsx_spreadsheet_model_loading() {
        let mut app = StudioApp::new();
        for path in [
            "test_documents/dummy_budget.xlsx",
            "../../test_documents/dummy_budget.xlsx",
        ] {
            if let Ok(bytes) = std::fs::read(path) {
                app.load_xlsx_bytes(&bytes);
                break;
            }
        }

        let sheet_model = app.spreadsheet_model.as_ref().expect("Spreadsheet model must exist");
        assert!(!sheet_model.sheet_names.is_empty());
        assert!(!sheet_model.rows.is_empty());
    }

    #[test]
    fn test_document_save_and_dirty_flag() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Docx;
        app.current_file_path = Some("/tmp/oneoffice_test_save.docx".to_string());
        app.is_dirty = true;

        let result = app.save_current_document();
        assert!(result.is_ok(), "Document save pipeline must succeed");
        assert!(!app.is_dirty, "Dirty flag must be cleared after save");
        assert!(std::path::Path::new("/tmp/oneoffice_test_save.docx").exists());
        let _ = std::fs::remove_file("/tmp/oneoffice_test_save.docx");
    }

    #[test]
    fn test_in_place_paragraph_formatting() {
        let mut app = StudioApp::new();
        let mut doc = studio::LoadedDocument::default();
        doc.paragraphs.push(studio::DocParagraph {
            text: "원본 단락".to_string(),
            is_heading: false,
            font_size: 14.0,
            is_bold: false,
        });
        app.doc_model = Some(doc);
        app.active_paragraph_idx = Some(0);

        app.format_state.font_size = 20.0;
        app.format_state.is_bold = true;
        app.apply_format_to_active_paragraph();

        let updated_para = &app.doc_model.as_ref().unwrap().paragraphs[0];
        assert_eq!(updated_para.font_size, 20.0);
        assert!(updated_para.is_bold);
        assert!(app.is_dirty);
    }

    #[test]
    fn test_pptx_in_place_editing_and_save_roundtrip() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Pptx;
        let test_path = "/tmp/oneoffice_test_save.pptx";
        app.current_file_path = Some(test_path.to_string());

        let mut pres = app.presentation_model.take().expect("Presentation model must exist");
        pres.slides.clear();
        pres.slides.push(studio::SlideItem {
            slide_number: 1,
            title: "수정된 첫 번째 슬라이드 제목".to_string(),
            bullet_points: vec!["새로운 불릿 항목 추가".to_string()],
            template_id: "problem-comparison-to-solution".to_string(),
        });
        pres.slides.push(studio::SlideItem {
            slide_number: 2,
            title: "두 번째 슬라이드: 아키텍처".to_string(),
            bullet_points: vec!["순수 Rust 백엔드 연동".to_string(), "ECMA-376 패키징".to_string()],
            template_id: "problem-comparison-to-solution".to_string(),
        });
        app.presentation_model = Some(pres);
        app.is_dirty = true;

        let save_res = app.save_current_document();
        assert!(save_res.is_ok(), "PPTX save must succeed: {:?}", save_res);
        assert!(!app.is_dirty);

        let saved_bytes = std::fs::read(test_path).expect("Saved PPTX must exist on disk");
        assert!(saved_bytes.len() > 1000, "PPTX package must have valid size");

        // Verify round-trip loading
        let mut reload_app = StudioApp::new();
        reload_app.load_pptx_bytes(&saved_bytes);
        let loaded = reload_app.presentation_model.expect("Loaded presentation must exist");
        assert_eq!(loaded.slides.len(), 2, "Round-tripped PPTX must have 2 slides");
        assert_eq!(loaded.slides[0].title, "수정된 첫 번째 슬라이드 제목");
        assert!(loaded.slides[0].bullet_points.iter().any(|b| b == "새로운 불릿 항목 추가"));
        assert_eq!(loaded.slides[1].title, "두 번째 슬라이드: 아키텍처");

        let _ = std::fs::remove_file(test_path);
    }

    #[test]
    fn test_formula_hud_function_lookup() {
        let hud = FormulaHudWidget::new("B2", "=AVERAGE(");
        assert_eq!(hud.cell_address, "B2");
        assert_eq!(hud.formula_text, "=AVERAGE(");
    }

    #[test]
    fn test_pdf_viewer_widget_stamping_roundtrip() {
        let mut widget = PdfViewerWidget::new();
        assert_eq!(widget.current_page, 0);

        let contract_pdf = oneoffice_pdf::generate_sample_korean_contract_pdf()
            .expect("generate sample contract failed");

        // Simulate session loading without GPU context
        let session = oneoffice_pdf::PdfDocumentSession::load(&contract_pdf)
            .expect("load pdf session failed");
        widget.session = Some(session);

        widget
            .stamp_seal_at(200.0, 700.0)
            .expect("stamp seal failed");
        assert_eq!(widget.layer_stack.signatures().len(), 1);

        let signed_pdf = widget.save_signed_pdf().expect("save signed pdf failed");
        assert!(!signed_pdf.is_empty());

        let signed_session = oneoffice_pdf::PdfDocumentSession::load(&signed_pdf)
            .expect("load signed pdf failed");
        assert_eq!(signed_session.page_count(), 1);
    }

    #[test]
    fn test_interactive_table_recalculation() {
        let mut app = StudioApp::new();
        // Initial B2 = 12500000, C2 = 4300000 -> D2 = 16800000
        assert_eq!(app.table_data[1][3], "16800000");

        // Mutate B2 to 20000000
        app.table_data[1][1] = "20000000".to_string();
        app.recalculate_table_sums();

        // D2 should now be 20000000 + 4300000 = 24300000
        assert_eq!(app.table_data[1][3], "24300000");
        // B4 (total of B) should be 20000000 + 18200000 = 38200000
        assert_eq!(app.table_data[3][1], "38200000");
    }

    #[test]
    fn test_pdf_signature_selection_and_deletion() {
        let mut widget = PdfViewerWidget::new();
        let contract_pdf = oneoffice_pdf::generate_sample_korean_contract_pdf()
            .expect("generate sample contract failed");
        let session = oneoffice_pdf::PdfDocumentSession::load(&contract_pdf)
            .expect("load pdf session failed");
        widget.session = Some(session);

        widget.stamp_seal_at(100.0, 200.0).expect("stamp failed");
        assert_eq!(widget.layer_stack.signatures().len(), 1);
        let sig_id = widget.layer_stack.signatures()[0].id.clone();

        widget.selected_sig_id = Some(sig_id.clone());
        widget.layer_stack.remove_signature(&sig_id).expect("remove failed");
        assert_eq!(widget.layer_stack.signatures().len(), 0);

        // Undo restores it
        assert!(widget.layer_stack.undo());
        assert_eq!(widget.layer_stack.signatures().len(), 1);
    }

    #[test]
    fn test_korean_font_installation_and_glyph_coverage() {
        let probe_chars = &['한', '글', '제', '안', '서', '문', '서', '표', '실', '측', '비', '교'];
        let sys_font = wordcraft_fonts::system_cjk_ui_font_for(false, true, probe_chars);
        println!(">>> sys_font: {:?}", sys_font.as_ref().map(|f| (f.bytes.len(), f.index)));
        assert!(sys_font.is_some(), "system_cjk_ui_font_for must find a font");

        let ctx = egui::Context::default();
        theme::install_korean_fonts(&ctx);

        // Run one empty frame to compile fonts atlas
        let mut output = ctx.run_ui(egui::RawInput::default(), |_| {});
        output.textures_delta.clear();

        // Verify Korean glyphs are covered in proportional font family without tofu
        let font_id = egui::FontId::proportional(14.0);
        let sample = "차세대 통합 오피스 플랫폼 기술 제안서 한글 수식 직인";
        let covered = ctx.fonts_mut(|fonts| fonts.has_glyphs(&font_id, sample));
        assert!(covered, "Korean system font must cover all sample Hangul glyphs (no tofu/boxes)");

        let text_w = ctx.fonts_mut(|fonts| fonts.layout_no_wrap(sample.to_string(), font_id, egui::Color32::WHITE)).size().x;
        println!(">>> text_w: {}", text_w);
        assert!(text_w > 100.0, "Korean text layout must have non-zero measured width");
    }
}
