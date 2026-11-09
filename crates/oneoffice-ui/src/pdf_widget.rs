//! Pure-Rust PDF viewer widget with electronic signature & chroma-key stamp overlay.

use egui::{Color32, Pos2, Rect, Stroke, TextureHandle, TextureOptions, Ui, Vec2};
use oneoffice_pdf::{
    PdfDocumentSession, PlacedSignature, SignatureLayerStack, SignaturePayload,
    generate_cleanroom_seal_on_white_paper_png, remove_white_background, save_pdf_with_signatures,
};

/// High-DPI PDF viewer and electronic signing widget.
#[derive(Default)]
pub struct PdfViewerWidget {
    pub session: Option<PdfDocumentSession>,
    pub current_page: usize,
    pub zoom_scale: f32,
    pub page_texture: Option<TextureHandle>,
    pub layer_stack: SignatureLayerStack,
    pub seal_label: String,
    pub status_message: String,
}

impl PdfViewerWidget {
    /// Constructs a new PDF viewer widget initialized with a clean-room Korean seal.
    #[must_use]
    pub fn new() -> Self {
        Self {
            session: None,
            current_page: 0,
            zoom_scale: 1.5,
            page_texture: None,
            layer_stack: SignatureLayerStack::default(),
            seal_label: "대표이사\n원오피스".to_string(),
            status_message: "PDF 문서를 불러와 주십시오.".to_string(),
        }
    }

    /// Loads a PDF from raw bytes and rasterizes the first page.
    pub fn load_pdf_bytes(&mut self, ctx: &egui::Context, pdf_bytes: &[u8]) -> Result<(), String> {
        let session = PdfDocumentSession::load(pdf_bytes).map_err(|e| e.to_string())?;
        self.session = Some(session);
        self.current_page = 0;
        self.layer_stack.clear();
        self.refresh_page_texture(ctx);
        self.status_message = "PDF 문서를 성공적으로 로드했습니다.".to_string();
        Ok(())
    }

    /// Refreshes the active page texture using high-DPI rasterization.
    pub fn refresh_page_texture(&mut self, ctx: &egui::Context) {
        if let Some(session) = &self.session
            && let Ok(stamp) = session.render_page_rgba(self.current_page, self.zoom_scale)
        {
            let color_img = egui::ColorImage::from_rgba_unmultiplied(
                [stamp.width as usize, stamp.height as usize],
                &stamp.rgba,
            );
            self.page_texture = Some(ctx.load_texture(
                format!("pdf_page_{}", self.current_page),
                color_img,
                TextureOptions::LINEAR,
            ));
        }
    }

    /// Stamps a transparent red seal on the active page at the specified location.
    pub fn stamp_seal_at(&mut self, x_pt: f32, y_pt: f32) -> Result<(), String> {
        let seal_white_png = generate_cleanroom_seal_on_white_paper_png(&self.seal_label)
            .map_err(|e| e.to_string())?;
        let transparent_seal = remove_white_background(&seal_white_png)
            .map_err(|e| e.to_string())?;

        self.layer_stack.add_signature(PlacedSignature {
            id: format!("seal-{}", self.layer_stack.signatures().len() + 1),
            page_index: self.current_page,
            rect_pt: [x_pt, y_pt, 48.0, 48.0],
            rotation_deg: -2.0,
            opacity: 0.95,
            payload: SignaturePayload::RgbaStamp(transparent_seal),
        });

        self.status_message = format!("페이지 {}에 직인을 날인했습니다.", self.current_page + 1);
        Ok(())
    }

    /// Automatically searches for `(인)` or `(서명)` anchors and stamps the seal.
    pub fn auto_stamp_anchor(&mut self) -> Result<(), String> {
        if let Some(session) = &self.session {
            let anchors = session
                .find_signature_anchors(self.current_page)
                .map_err(|e| e.to_string())?;
            if let Some(anchor) = anchors.first() {
                let [cx, cy] = anchor.center();
                self.stamp_seal_at(cx - 24.0, cy - 24.0)?;
                return Ok(());
            }
        }
        Err("서명/날인 앵커 (인)을 찾을 수 없습니다.".to_string())
    }

    /// Burns all active signatures non-destructively and returns the signed PDF bytes.
    #[must_use]
    pub fn save_signed_pdf(&self) -> Option<Vec<u8>> {
        if let Some(session) = &self.session {
            save_pdf_with_signatures(session.raw_bytes(), &self.layer_stack).ok()
        } else {
            None
        }
    }

    /// Renders the PDF viewer UI and signature toolbar.
    pub fn show(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        // Toolbar
        ui.horizontal(|ui| {
            if let Some(session) = &self.session {
                let total = session.page_count();
                if ui.button("◀ 이전").clicked() && self.current_page > 0 {
                    self.current_page -= 1;
                    self.refresh_page_texture(ctx);
                }

                ui.label(format!("페이지 {} / {}", self.current_page + 1, total));

                if ui.button("다음 ▶").clicked() && self.current_page + 1 < total {
                    self.current_page += 1;
                    self.refresh_page_texture(ctx);
                }
            }

            ui.separator();

            // Zoom controls
            if ui.button("➖ 축소").clicked() && self.zoom_scale > 0.75 {
                self.zoom_scale -= 0.25;
                self.refresh_page_texture(ctx);
            }
            ui.label(format!("{:.0}%", self.zoom_scale * 100.0 / 1.5));
            if ui.button("➕ 확대").clicked() && self.zoom_scale < 3.0 {
                self.zoom_scale += 0.25;
                self.refresh_page_texture(ctx);
            }

            ui.separator();

            // Seal stamping actions
            if ui.button("🔴 (인) 앵커 자동 날인").clicked()
                && let Err(e) = self.auto_stamp_anchor()
            {
                self.status_message = e;
            }

            if self.layer_stack.can_undo() && ui.button("↩ 서명 취소").clicked() {
                self.layer_stack.undo();
            }

            ui.label(
                egui::RichText::new(&self.status_message)
                    .size(11.0)
                    .color(Color32::from_rgb(16, 185, 129)),
            );
        });

        ui.add_space(8.0);

        // PDF Page view area with signature overlay
        if let Some(tex) = &self.page_texture {
            let display_w = tex.size()[0] as f32 / (self.zoom_scale / 1.0);
            let display_h = tex.size()[1] as f32 / (self.zoom_scale / 1.0);

            let (response, painter) =
                ui.allocate_painter(Vec2::new(display_w, display_h), egui::Sense::click());
            let origin = response.rect.min;

            // Draw base PDF page
            painter.image(
                tex.id(),
                response.rect,
                Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );

            // Draw placed signatures on this page
            for sig in self.layer_stack.signatures() {
                if sig.page_index == self.current_page {
                    let [rx, ry, rw, rh] = sig.rect_pt;
                    let sig_rect = Rect::from_min_size(
                        Pos2::new(origin.x + rx, origin.y + ry),
                        Vec2::new(rw, rh),
                    );

                    // Red seal indicator circle
                    let center = sig_rect.center();
                    let radius = rw.min(rh) * 0.45;
                    painter.circle_stroke(
                        center,
                        radius,
                        Stroke::new(2.0, Color32::from_rgb(220, 38, 38)),
                    );
                    painter.text(
                        center,
                        egui::Align2::CENTER_CENTER,
                        "印",
                        egui::FontId::proportional(14.0),
                        Color32::from_rgb(220, 38, 38),
                    );
                }
            }

            // Click to place seal manually
            if response.clicked()
                && let Some(pos) = response.interact_pointer_pos()
            {
                let click_x = pos.x - origin.x - 24.0;
                let click_y = pos.y - origin.y - 24.0;
                let _ = self.stamp_seal_at(click_x, click_y);
            }
        }
    }
}
