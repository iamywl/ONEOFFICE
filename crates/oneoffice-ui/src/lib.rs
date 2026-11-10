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
}
