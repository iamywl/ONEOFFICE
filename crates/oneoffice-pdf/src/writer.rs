//! Lossless and high-DPI signature PDF exporter.
//!
//! When no signatures are placed, original PDF bytes are returned unchanged.
//! When signatures are placed, pages are rendered at high DPI, signatures are
//! composited with alpha blending and sub-pixel accuracy, and clean-room PDF
//! pages are synthesized via `krilla` and `craft-pdf`.

use craft_pdf::raster::{Options as RasterOptions, RenderCache};
use craft_pdf::write::{Metadata, Writer};
use krilla::geom::Size as KrillaSize;
use krilla::page::PageSettings;

use crate::error::PdfError;
use crate::font_fallback::build_interpreter_settings;
use crate::signature::{RgbaStamp, SignatureLayerStack, composite_signatures_onto_rgba, unpremultiply_rgba_bytes};

/// Saves a PDF document with all active signatures burned into their target pages.
///
/// If `layers` contains no active signatures, `original_pdf_bytes` is returned
/// verbatim, guaranteeing byte-level lossless preservation of existing fonts,
/// hyperlinks, bookmarks, and structural tags.
pub fn save_pdf_with_signatures(
    original_pdf_bytes: &[u8],
    layers: &SignatureLayerStack,
) -> Result<Vec<u8>, PdfError> {
    let active_signatures = layers.signatures();
    if active_signatures.is_empty() {
        return Ok(original_pdf_bytes.to_vec());
    }

    let pdf = craft_pdf::open_slice(original_pdf_bytes, None)?;
    let page_count = pdf.pages().len();
    if page_count == 0 {
        return Ok(original_pdf_bytes.to_vec());
    }

    let meta = Metadata {
        title: Some("ONEOFFICE Signed Document".to_string()),
        authors: vec!["ONEOFFICE Pure-Rust Engine".to_string()],
        creator: "ONEOFFICE (krilla + hayro)".to_string(),
        ..Default::default()
    };
    let mut writer = Writer::new(krilla::SerializeSettings::default(), meta);

    let doc = writer.document_mut();
    let cache = RenderCache::new();
    let settings = build_interpreter_settings();
    let scale = 2.0f32; // 144 DPI HiDPI rendering

    for page_idx in 0..page_count {
        let page = pdf.pages().get(page_idx).ok_or(PdfError::PageOutOfBounds {
            page_index: page_idx,
            page_count,
        })?;

        let (pw, ph) = page.render_dimensions();
        let page_settings =
            PageSettings::from_wh(pw, ph).ok_or(PdfError::InvalidDimensions {
                width: pw as u32,
                height: ph as u32,
            })?;
        let mut k_page = doc.start_page_with(page_settings);
        let mut surface = k_page.surface();

        // 1. Render base page raster
        let raster = craft_pdf::raster::render(
            page,
            &cache,
            &settings,
            &RasterOptions {
                scale: [scale, scale],
                size: None,
                offset: [0.0, 0.0],
                background: [255, 255, 255, 255],
                color_replacement: None,
            },
        )?;

        // 2. Unpremultiply to straight RGBA8
        let mut straight = unpremultiply_rgba_bytes(raster.bytes());

        // 3. Composite active signatures on this page
        composite_signatures_onto_rgba(
            &mut straight,
            raster.width,
            raster.height,
            page_idx,
            scale,
            [0.0, 0.0],
            active_signatures,
        )?;

        // 4. Encode to PNG and draw onto Krilla surface
        let stamp = RgbaStamp::new(raster.width, raster.height, straight)?;
        let png_bytes = stamp.to_png_bytes()?;
        let k_data = krilla::Data::from(png_bytes);
        let k_img = krilla::image::Image::from_png(k_data, true)
            .map_err(|e| PdfError::Render(format!("Failed to embed signed page image: {e}")))?;

        let page_size = KrillaSize::from_wh(pw, ph).ok_or(PdfError::InvalidDimensions {
            width: pw as u32,
            height: ph as u32,
        })?;
        surface.draw_image(k_img, page_size);

        surface.finish();
        k_page.finish();
    }

    let out_pdf = writer.finish()?;
    Ok(out_pdf)
}
