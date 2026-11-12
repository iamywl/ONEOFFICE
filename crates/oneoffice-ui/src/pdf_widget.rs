//! Pure-Rust PDF viewer widget with electronic signature & chroma-key stamp overlay.

use egui::{Color32, Pos2, Rect, Stroke, TextureHandle, TextureOptions, Ui, Vec2};
use oneoffice_pdf::{
    FreehandBezierSignature, FreehandStroke, PdfDocumentSession, PlacedSignature,
    SignatureLayerStack, SignaturePayload, SignaturePoint,
    generate_cleanroom_seal_on_white_paper_png, remove_white_background, save_pdf_with_signatures,
};

/// High-DPI PDF viewer and electronic signing widget.
#[derive(Default)]
pub struct PdfViewerWidget {
    pub session: Option<PdfDocumentSession>,
    pub current_page: usize,
    pub zoom_scale: f32,
    pub rotation_deg: f32,
    pub page_texture: Option<TextureHandle>,
    pub layer_stack: SignatureLayerStack,
    pub seal_label: String,
    pub status_message: String,
    pub selected_sig_id: Option<String>,
    pub freehand_pad_open: bool,
    pub freehand_strokes: Vec<Vec<[f32; 2]>>,
    pub freehand_current_stroke: Vec<[f32; 2]>,
}

impl PdfViewerWidget {
    /// Constructs a new PDF viewer widget initialized with a clean-room Korean seal.
    #[must_use]
    pub fn new() -> Self {
        Self {
            session: None,
            current_page: 0,
            zoom_scale: 1.0,
            rotation_deg: 0.0,
            page_texture: None,
            layer_stack: SignatureLayerStack::default(),
            seal_label: "대표이사\n원오피스".to_string(),
            status_message: "PDF 문서를 불러와 주십시오.".to_string(),
            selected_sig_id: None,
            freehand_pad_open: false,
            freehand_strokes: Vec::new(),
            freehand_current_stroke: Vec::new(),
        }
    }

    /// Automatically loads a clean-room sample contract if no PDF is currently loaded.
    pub fn ensure_sample_loaded(&mut self, ctx: &egui::Context) {
        if self.session.is_none()
            && let Ok(sample_bytes) = oneoffice_pdf::generate_sample_korean_contract_pdf()
        {
            let _ = self.load_pdf_bytes(ctx, &sample_bytes);
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

    /// Stamps the buffered freehand drawing as a Bezier fountain-pen signature at the specified point.
    pub fn stamp_freehand_signature_at(&mut self, x_pt: f32, y_pt: f32) -> Result<(), String> {
        if self.freehand_strokes.is_empty() && self.freehand_current_stroke.is_empty() {
            return Err("서명 패드에 입력된 자필 서명이 없습니다.".to_string());
        }

        let mut strokes_data = self.freehand_strokes.clone();
        if !self.freehand_current_stroke.is_empty() {
            strokes_data.push(self.freehand_current_stroke.clone());
        }

        let mut freehand_strokes = Vec::new();
        for stroke in strokes_data {
            if stroke.is_empty() {
                continue;
            }
            let pts: Vec<SignaturePoint> = stroke
                .iter()
                .enumerate()
                .map(|(idx, pt)| SignaturePoint::new(pt[0], pt[1], idx as f64 * 16.0))
                .collect();
            freehand_strokes.push(FreehandStroke::new(pts, 1.8));
        }

        if freehand_strokes.is_empty() {
            return Err("유효한 서명 획이 없습니다.".to_string());
        }

        let bezier = FreehandBezierSignature::new(
            freehand_strokes,
            [15, 23, 42, 255],
            [120.0, 48.0],
        );

        self.layer_stack.add_signature(PlacedSignature {
            id: format!("freehand-{}", self.layer_stack.signatures().len() + 1),
            page_index: self.current_page,
            rect_pt: [x_pt, y_pt, 120.0, 48.0],
            rotation_deg: 0.0,
            opacity: 1.0,
            payload: SignaturePayload::BezierStrokes(bezier),
        });

        self.freehand_strokes.clear();
        self.freehand_current_stroke.clear();
        self.freehand_pad_open = false;
        self.status_message = format!("페이지 {}에 자필 서명을 날인했습니다.", self.current_page + 1);
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
        self.ensure_sample_loaded(ctx);

        // Keyboard shortcuts for page navigation and zoom
        let ctrl_or_cmd = ui.input(|i| i.modifiers.command || i.modifiers.ctrl);
        let total_pages = self.session.as_ref().map_or(1, |s| s.page_count());
        if !ctrl_or_cmd {
            if (ui.input(|i| i.key_pressed(egui::Key::PageDown)) || ui.input(|i| i.key_pressed(egui::Key::ArrowDown))) && self.current_page + 1 < total_pages {
                self.current_page += 1;
                self.selected_sig_id = None;
                self.refresh_page_texture(ctx);
            }
            if (ui.input(|i| i.key_pressed(egui::Key::PageUp)) || ui.input(|i| i.key_pressed(egui::Key::ArrowUp))) && self.current_page > 0 {
                self.current_page -= 1;
                self.selected_sig_id = None;
                self.refresh_page_texture(ctx);
            }
            if ui.input(|i| i.key_pressed(egui::Key::Home)) && self.current_page != 0 {
                self.current_page = 0;
                self.selected_sig_id = None;
                self.refresh_page_texture(ctx);
            }
            if ui.input(|i| i.key_pressed(egui::Key::End)) && self.current_page + 1 < total_pages {
                self.current_page = total_pages.saturating_sub(1);
                self.selected_sig_id = None;
                self.refresh_page_texture(ctx);
            }
        } else {
            if (ui.input(|i| i.key_pressed(egui::Key::Plus)) || ui.input(|i| i.key_pressed(egui::Key::Equals))) && self.zoom_scale < 2.5 {
                self.zoom_scale += 0.25;
                self.refresh_page_texture(ctx);
            }
            if ui.input(|i| i.key_pressed(egui::Key::Minus)) && self.zoom_scale > 0.5 {
                self.zoom_scale -= 0.25;
                self.refresh_page_texture(ctx);
            }
            if ui.input(|i| i.key_pressed(egui::Key::Num0)) {
                self.zoom_scale = 1.0;
                self.refresh_page_texture(ctx);
            }
        }

        // Toolbar
        ui.horizontal(|ui| {
            if let Some(session) = &self.session {
                let total = session.page_count();
                if ui.button("< 이전").clicked() && self.current_page > 0 {
                    self.current_page -= 1;
                    self.selected_sig_id = None;
                    self.refresh_page_texture(ctx);
                }

                ui.label(format!("페이지 {} / {}", self.current_page + 1, total));

                if ui.button("다음 >").clicked() && self.current_page + 1 < total {
                    self.current_page += 1;
                    self.selected_sig_id = None;
                    self.refresh_page_texture(ctx);
                }
            }

            ui.separator();

            // Zoom controls
            if ui.button("- 축소").clicked() && self.zoom_scale > 0.5 {
                self.zoom_scale -= 0.25;
                self.refresh_page_texture(ctx);
            }
            ui.label(format!("{:.0}%", self.zoom_scale * 100.0));
            if ui.button("+ 확대").clicked() && self.zoom_scale < 2.5 {
                self.zoom_scale += 0.25;
                self.refresh_page_texture(ctx);
            }

            ui.separator();

            if ui.button("⟳ 회전").on_hover_text("페이지 90° 시계방향 회전").clicked() {
                self.rotation_deg = (self.rotation_deg + 90.0) % 360.0;
            }

            let sig_pad_btn = ui.selectable_label(self.freehand_pad_open, "✍ 자필 서명");
            if sig_pad_btn.on_hover_text("마우스/스타일러스 자필 베지어 서명 패드 토글").clicked() {
                self.freehand_pad_open = !self.freehand_pad_open;
            }

            ui.separator();

            // Seal Label Customizer & Presets
            ui.label("직인 문구:");
            ui.add(egui::TextEdit::singleline(&mut self.seal_label).desired_width(120.0));

            egui::ComboBox::from_id_salt("pdf_seal_preset_dropdown")
                .selected_text("프리셋")
                .width(75.0)
                .show_ui(ui, |ui| {
                    if ui.selectable_label(self.seal_label.contains("대표이사"), "대표이사 직인").clicked() {
                        self.seal_label = "대표이사\n원오피스".to_string();
                    }
                    if ui.selectable_label(self.seal_label.contains("홍길동"), "개인 인영").clicked() {
                        self.seal_label = "홍길동\n(인)".to_string();
                    }
                    if ui.selectable_label(self.seal_label.contains("APPROVED"), "승인/결재인").clicked() {
                        self.seal_label = "결재승인\nAPPROVED".to_string();
                    }
                    if ui.selectable_label(self.seal_label.contains("간인"), "문서 간인").clicked() {
                        self.seal_label = "간인\nONEOFFICE".to_string();
                    }
                });

            // Seal stamping actions
            if ui.button("(인) 앵커 자동 날인").clicked()
                && let Err(e) = self.auto_stamp_anchor()
            {
                self.status_message = e;
            }

            // Delete selected signature button
            if let Some(sel_id) = &self.selected_sig_id.clone() {
                if ui.button("선택 서명 삭제").clicked() {
                    let _ = self.layer_stack.remove_signature(sel_id);
                    self.selected_sig_id = None;
                    self.status_message = "선택된 서명을 삭제했습니다.".to_string();
                }
            } else if self.layer_stack.can_undo() && ui.button("서명 취소").clicked() {
                self.layer_stack.undo();
            }

            ui.label(
                egui::RichText::new(&self.status_message)
                    .size(11.0)
                    .color(Color32::from_rgb(16, 185, 129)),
            );
        });

        // Freehand Signature Drawing Pad Panel
        if self.freehand_pad_open {
            ui.add_space(4.0);
            egui::Frame::new()
                .fill(Color32::from_rgb(248, 250, 252))
                .stroke(Stroke::new(1.0, Color32::from_rgb(203, 213, 225)))
                .corner_radius(egui::CornerRadius::same(6))
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("✍ 자필 서명 패드 (Freehand Signature Pad)")
                                .strong()
                                .color(Color32::from_rgb(30, 41, 59)),
                        );
                        if ui.button("지우기").on_hover_text("서명 내용 초기화").clicked() {
                            self.freehand_strokes.clear();
                            self.freehand_current_stroke.clear();
                        }
                        if ui.button("날인 적용").on_hover_text("현재 서명을 페이지에 날인").clicked() {
                            let _ = self.stamp_freehand_signature_at(200.0, 700.0);
                        }
                        if ui.button("✕ 닫기").clicked() {
                            self.freehand_pad_open = false;
                        }
                    });

                    ui.add_space(4.0);
                    let (resp, painter) = ui.allocate_painter(Vec2::new(320.0, 90.0), egui::Sense::drag());
                    painter.rect_filled(resp.rect, 4.0, Color32::WHITE);
                    painter.rect_stroke(resp.rect, 4.0, Stroke::new(1.0, Color32::from_rgb(226, 232, 240)), egui::StrokeKind::Outside);

                    // Baseline guide line
                    painter.line_segment(
                        [
                            Pos2::new(resp.rect.min.x + 16.0, resp.rect.min.y + 68.0),
                            Pos2::new(resp.rect.max.x - 16.0, resp.rect.min.y + 68.0),
                        ],
                        Stroke::new(1.0, Color32::from_rgb(226, 232, 240)),
                    );
                    painter.text(
                        Pos2::new(resp.rect.min.x + 20.0, resp.rect.min.y + 64.0),
                        egui::Align2::LEFT_BOTTOM,
                        "서명란 (Sign Here)",
                        egui::FontId::proportional(11.0),
                        Color32::from_rgb(203, 213, 225),
                    );

                    // Handle pointer dragging for freehand input
                    if resp.dragged() {
                        if let Some(pos) = resp.interact_pointer_pos()
                            && resp.rect.contains(pos)
                        {
                            let rel_x = pos.x - resp.rect.min.x;
                            let rel_y = pos.y - resp.rect.min.y;
                            self.freehand_current_stroke.push([rel_x, rel_y]);
                        }
                    } else if resp.drag_stopped() && !self.freehand_current_stroke.is_empty() {
                        let stroke = std::mem::take(&mut self.freehand_current_stroke);
                        self.freehand_strokes.push(stroke);
                    }

                    // Paint finished strokes
                    let ink_stroke = Stroke::new(2.5, Color32::from_rgb(15, 23, 42));
                    for stroke in &self.freehand_strokes {
                        for window in stroke.windows(2) {
                            painter.line_segment(
                                [
                                    Pos2::new(resp.rect.min.x + window[0][0], resp.rect.min.y + window[0][1]),
                                    Pos2::new(resp.rect.min.x + window[1][0], resp.rect.min.y + window[1][1]),
                                ],
                                ink_stroke,
                            );
                        }
                    }
                    // Paint current in-progress stroke
                    for window in self.freehand_current_stroke.windows(2) {
                        painter.line_segment(
                            [
                                Pos2::new(resp.rect.min.x + window[0][0], resp.rect.min.y + window[0][1]),
                                Pos2::new(resp.rect.min.x + window[1][0], resp.rect.min.y + window[1][1]),
                            ],
                            ink_stroke,
                        );
                    }
                });
        }

        ui.add_space(8.0);

        // PDF Page view area with signature overlay (centered paper with crisp border)
        if let Some(session) = &self.session {
            let [page_w_pt, page_h_pt] = session
                .page_dimensions_pt(self.current_page)
                .unwrap_or([595.28, 841.89]);
            let display_w = page_w_pt * self.zoom_scale;
            let display_h = page_h_pt * self.zoom_scale;

            ui.vertical_centered(|ui| {
                egui::Frame::canvas(ui.style())
                    .fill(Color32::WHITE)
                    .stroke(Stroke::new(1.0, Color32::from_rgb(203, 213, 225)))
                    .inner_margin(0.0)
                    .show(ui, |ui| {
                        let (response, painter) =
                            ui.allocate_painter(Vec2::new(display_w, display_h), egui::Sense::click());
                        let origin = response.rect.min;

                        // Draw base PDF page
                        if let Some(tex) = &self.page_texture {
                            painter.image(
                                tex.id(),
                                response.rect,
                                Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                                Color32::WHITE,
                            );
                        }

                        // Draw placed signatures on this page
                        let mut clicked_sig_id = None;
                        for sig in self.layer_stack.signatures() {
                            if sig.page_index == self.current_page {
                                let [rx, ry, rw, rh] = sig.rect_pt;
                                let sig_rect = Rect::from_min_size(
                                    Pos2::new(origin.x + rx * self.zoom_scale, origin.y + ry * self.zoom_scale),
                                    Vec2::new(rw * self.zoom_scale, rh * self.zoom_scale),
                                );

                                let is_selected = self.selected_sig_id.as_deref() == Some(&sig.id);

                                match &sig.payload {
                                    SignaturePayload::BezierStrokes(bezier) => {
                                        let ink = Color32::from_rgba_premultiplied(
                                            bezier.ink_color[0],
                                            bezier.ink_color[1],
                                            bezier.ink_color[2],
                                            bezier.ink_color[3],
                                        );
                                        for stroke in &bezier.strokes {
                                            let pts: Vec<Pos2> = stroke
                                                .points
                                                .iter()
                                                .map(|p| Pos2::new(sig_rect.min.x + p.x * self.zoom_scale, sig_rect.min.y + p.y * self.zoom_scale))
                                                .collect();
                                            if pts.len() >= 2 {
                                                for window in pts.windows(2) {
                                                    painter.line_segment(
                                                        [window[0], window[1]],
                                                        Stroke::new((stroke.base_width_pt * self.zoom_scale).max(1.5), ink),
                                                    );
                                                }
                                            }
                                        }
                                    }
                                    SignaturePayload::RgbaStamp(_) | SignaturePayload::QuickTextStamp(_) => {
                                        // Red seal indicator circle
                                        let center = sig_rect.center();
                                        let radius = (rw.min(rh) * self.zoom_scale) * 0.45;
                                        painter.circle_stroke(
                                            center,
                                            radius,
                                            Stroke::new(2.0, Color32::from_rgb(220, 38, 38)),
                                        );
                                        painter.text(
                                            center,
                                            egui::Align2::CENTER_CENTER,
                                            "印",
                                            egui::FontId::proportional(14.0 * self.zoom_scale),
                                            Color32::from_rgb(220, 38, 38),
                                        );
                                    }
                                }

                                // Selection highlight box
                                if is_selected {
                                    painter.rect_stroke(
                                        sig_rect.expand(4.0),
                                        4.0,
                                        Stroke::new(2.0, Color32::from_rgb(37, 99, 235)),
                                        egui::StrokeKind::Outside,
                                    );
                                }

                                if response.clicked()
                                    && let Some(pos) = response.interact_pointer_pos()
                                    && sig_rect.contains(pos)
                                {
                                    clicked_sig_id = Some(sig.id.clone());
                                }
                            }
                        }

                        // Handle click
                        if response.clicked()
                            && let Some(pos) = response.interact_pointer_pos()
                        {
                            if let Some(hit_id) = clicked_sig_id {
                                self.selected_sig_id = Some(hit_id);
                                self.status_message = "서명이 선택되었습니다. (삭제 가능)".to_string();
                            } else {
                                self.selected_sig_id = None;
                                let click_x = (pos.x - origin.x) / self.zoom_scale - 24.0;
                                let click_y = (pos.y - origin.y) / self.zoom_scale - 24.0;
                                let _ = self.stamp_seal_at(click_x, click_y);
                            }
                        }
                    });
            });
        }
    }
}
