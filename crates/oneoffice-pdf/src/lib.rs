//! Pure-Rust cross-platform PDF viewer and non-destructive signature/stamp engine.
//!
//! Built on clean-room components:
//! - Parser & Interpreter: `craft-pdf` (`hayro`)
//! - Synthesis & Vector PDF Writer: `craft-pdf` (`krilla`)
//! - Bezier & Geometry: `kurbo`
//! - Raster & Image: `vello_cpu` + `image` + `skrifa`
//! - Korean Typography & Shaper: `harfrust`

#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable
)]
#![allow(
    clippy::chunks_exact_to_as_chunks,
    clippy::field_reassign_with_default,
    clippy::vec_init_then_push
)]

pub mod error;
pub mod font_fallback;
pub mod signature;
pub mod viewer;
pub mod writer;

pub use error::{PdfError, PdfResult};
pub use signature::{
    ChromaKeyAlphaMatting, FreehandBezierSignature, FreehandStroke, PlacedSignature, QuickSignText,
    RgbaStamp, SignatureLayerStack, SignaturePayload, SignaturePoint, composite_signatures_onto_rgba,
    decode_png_stamp, generate_cleanroom_seal_on_white_paper_png, remove_white_background,
    unpremultiply_rgba_bytes,
};
pub use viewer::{
    A4_HEIGHT_PT, A4_WIDTH_PT, PdfDocumentSession, TextAnchor,
    generate_document_pdf, generate_sample_korean_contract_pdf,
};
pub use writer::save_pdf_with_signatures;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_sample_contract_and_render_hidpi() {
        let pdf_bytes = generate_sample_korean_contract_pdf().expect("contract generation failed");
        assert!(!pdf_bytes.is_empty(), "PDF bytes must not be empty");

        let session = PdfDocumentSession::load(&pdf_bytes).expect("session load failed");
        assert_eq!(session.page_count(), 1, "Expected 1 contract page");

        let [pw, ph] = session
            .page_dimensions_pt(0)
            .expect("page dimensions failed");
        assert!((pw - A4_WIDTH_PT).abs() < 1.0);
        assert!((ph - A4_HEIGHT_PT).abs() < 1.0);

        let raster_1x = session
            .render_page_rgba(0, 1.0)
            .expect("1x rasterization failed");
        assert!(raster_1x.width > 500 && raster_1x.width < 650);
        assert!(raster_1x.height > 800 && raster_1x.height < 900);

        let png_bytes = session
            .render_page_png(0, 2.0)
            .expect("2x png render failed");
        assert!(!png_bytes.is_empty());
        assert_eq!(&png_bytes[1..4], b"PNG");
    }

    #[test]
    fn test_anchor_detection_and_chroma_key_stamping() {
        let pdf_bytes = generate_sample_korean_contract_pdf().expect("contract generation failed");
        let session = PdfDocumentSession::load(&pdf_bytes).expect("session load failed");

        let anchors = session
            .find_signature_anchors(0)
            .expect("anchor search failed");
        assert!(!anchors.is_empty(), "Must detect at least one signing anchor");
        let anchor = &anchors[0];
        let [cx, cy] = anchor.center();
        assert!(cx > 0.0 && cy > 0.0);

        // Generate synthetic Korean red seal on white paper
        let seal_raw_png =
            generate_cleanroom_seal_on_white_paper_png("대표이사\n원오피스")
                .expect("seal generator failed");
        assert!(!seal_raw_png.is_empty());

        // Chroma-key remove white background
        let transparent_seal =
            remove_white_background(&seal_raw_png)
                .expect("chroma key failed");
        assert!(transparent_seal.width > 0 && transparent_seal.height > 0);

        // Corner texel (0, 0) must be fully transparent
        let corner = transparent_seal.pixel(0, 0).expect("corner pixel");
        assert!(
            corner[3] < 15,
            "Corner alpha must be transparent, got {}",
            corner[3]
        );

        // Center texel should have vermilion red ink
        let mid_x = transparent_seal.width / 2;
        let mid_y = transparent_seal.height / 2;
        let center = transparent_seal.pixel(mid_x, mid_y).expect("center pixel");
        assert!(center[0] > 100, "Red channel must be high, got {}", center[0]);
        assert!(
            center[3] > 100,
            "Alpha channel must be opaque, got {}",
            center[3]
        );
    }

    #[test]
    fn test_undo_redo_layer_stack() {
        let mut stack = SignatureLayerStack::default();
        assert!(!stack.can_undo());
        assert!(!stack.can_redo());

        let sig1 = PlacedSignature {
            id: "sig-test-1".to_string(),
            page_index: 0,
            rect_pt: [100.0, 100.0, 50.0, 50.0],
            rotation_deg: 0.0,
            opacity: 1.0,
            payload: SignaturePayload::QuickTextStamp(QuickSignText {
                text: "홍길동".to_string(),
                font_size_pt: 18.0,
                text_color: [180, 20, 20, 255],
                border_color: Some([180, 20, 20, 255]),
            }),
        };

        stack.add_signature(sig1);
        assert_eq!(stack.signatures().len(), 1);
        assert!(stack.can_undo());

        stack.move_signature("sig-test-1", 120.0, 140.0).unwrap();
        assert_eq!(stack.get("sig-test-1").unwrap().rect_pt[0], 120.0);

        // Undo move
        assert!(stack.undo());
        assert_eq!(stack.get("sig-test-1").unwrap().rect_pt[0], 100.0);
        assert!(stack.can_redo());

        // Redo move
        assert!(stack.redo());
        assert_eq!(stack.get("sig-test-1").unwrap().rect_pt[0], 120.0);
    }

    #[test]
    fn test_save_pdf_with_signatures_roundtrip() {
        let original_pdf =
            generate_sample_korean_contract_pdf().expect("contract generation failed");
        let mut stack = SignatureLayerStack::default();

        // 1. With empty stack: should be identical bytes
        let unchanged = save_pdf_with_signatures(&original_pdf, &stack).expect("save failed");
        assert_eq!(unchanged.len(), original_pdf.len());

        // 2. Add stamp and save
        let seal_raw_png =
            generate_cleanroom_seal_on_white_paper_png("대표이사\n원오피스").unwrap();
        let transparent_seal =
            remove_white_background(&seal_raw_png).unwrap();

        stack.add_signature(PlacedSignature {
            id: "seal-1".to_string(),
            page_index: 0,
            rect_pt: [420.0, 725.0, 48.0, 48.0],
            rotation_deg: -3.5,
            opacity: 0.95,
            payload: SignaturePayload::RgbaStamp(transparent_seal),
        });

        let signed_pdf = save_pdf_with_signatures(&original_pdf, &stack).expect("save signed failed");
        assert!(!signed_pdf.is_empty());

        // Reload signed PDF and verify page count & rendering
        let signed_session =
            PdfDocumentSession::load(&signed_pdf).expect("signed session load failed");
        assert_eq!(signed_session.page_count(), 1);

        let signed_png = signed_session
            .render_page_png(0, 1.5)
            .expect("render signed page failed");
        assert!(!signed_png.is_empty());
    }

    #[test]
    fn test_generate_document_pdf_export() {
        let paragraphs = vec![
            "1. 차세대 올인원 오피스 플랫폼 기술 개요 및 도입 목적 기술.".to_string(),
            "2. 순수 Rust 기반 HWPX, DOCX, XLSX, PPTX, PDF 통합 지원 체계 구축.".to_string(),
            "3. 전자서명 및 직인 날인 검증 앵커가 포함된 표준 규격 보고서.".to_string(),
        ];
        let pdf_bytes = generate_document_pdf(
            "ONEOFFICE 사업수행계획서",
            "문서번호: ONEOFFICE-2026-PLAN | 최종 승인본",
            &paragraphs,
        )
        .expect("generate document pdf must succeed");

        assert!(pdf_bytes.len() > 1000);
        let session = PdfDocumentSession::load(&pdf_bytes).expect("session load must succeed");
        assert_eq!(session.page_count(), 1);

        let anchors = session.find_signature_anchors(0).expect("anchors search");
        assert!(!anchors.is_empty(), "Exported PDF must have signature anchor (인)");
    }
}
