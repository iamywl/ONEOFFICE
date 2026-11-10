//! In-place consulting proposal diagramming sub-canvas widget for HWPX and DOCX.

use egui::{Color32, Pos2, Rect, Stroke, StrokeKind, Ui, Vec2};
use oneoffice_diagram::layout::{PositionedScene, SceneElement, solve_layout};
use oneoffice_diagram::spec::{DiagramSpec, RgbColor};
use oneoffice_diagram::templates::{instantiate_template, list_diagram_templates};

/// Sub-canvas widget for interactive proposal infographic layout and editing.
#[derive(Clone, Debug)]
pub struct DiagramWidget {
    pub spec: DiagramSpec,
    pub selected_template_id: String,
    pub last_scene: Option<PositionedScene>,
}

impl Default for DiagramWidget {
    fn default() -> Self {
        let template_id = "problem-comparison-to-solution";
        let initial_spec = instantiate_template(template_id).unwrap_or_else(|_| DiagramSpec {
            schema_version: oneoffice_diagram::spec::DIAGRAM_SCHEMA_V1.to_string(),
            id: "default_diagram".to_string(),
            title: Some("ONEOFFICE Proposal".to_string()),
            canvas: oneoffice_diagram::spec::DiagramCanvas::default(),
            theme: oneoffice_diagram::spec::DiagramTheme::default(),
            root: oneoffice_diagram::spec::DiagramNode::VStack {
                id: Some("root".to_string()),
                gap_pt: 8.0,
                padding_pt: 0.0,
                children: Vec::new(),
                connectors: Vec::new(),
            },
            connectors: Vec::new(),
        });
        let scene = solve_layout(&initial_spec).ok();
        Self {
            spec: initial_spec,
            selected_template_id: template_id.to_string(),
            last_scene: scene,
        }
    }
}

impl DiagramWidget {
    /// Renders the diagram template bar and interactive preview canvas.
    pub fn show(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("제안서 도식 템플릿:").strong());

            let templates = list_diagram_templates();
            egui::ComboBox::from_id_salt("diagram_template_picker")
                .selected_text(&self.selected_template_id)
                .show_ui(ui, |ui| {
                    for info in templates {
                        let is_selected = self.selected_template_id == info.id;
                        if ui.selectable_label(is_selected, info.name).clicked() {
                            self.selected_template_id = info.id.to_string();
                            if let Ok(new_spec) = instantiate_template(info.id) {
                                self.spec = new_spec;
                                self.last_scene = solve_layout(&self.spec).ok();
                            }
                        }
                    }
                });

            if ui.button("레이아웃 재해석").clicked() {
                self.last_scene = solve_layout(&self.spec).ok();
            }

            if let Some(scene) = &self.last_scene {
                let diag = &scene.diagnostics;
                if diag.overflow_count == 0 && diag.collision_count == 0 {
                    ui.label(
                        egui::RichText::new("글자 잘림 0건 (완벽 정렬)")
                            .color(Color32::from_rgb(16, 185, 129))
                            .size(11.0),
                    );
                } else {
                    ui.label(
                        egui::RichText::new(format!(
                            "! 오버플로우: {}건, 충돌: {}건",
                            diag.overflow_count, diag.collision_count
                        ))
                        .color(Color32::from_rgb(245, 158, 11))
                        .size(11.0),
                    );
                }
            }
        });

        ui.add_space(8.0);

        // Canvas viewport
        if let Some(scene) = &self.last_scene {
            let canvas_w = scene.width_pt;
            let canvas_h = scene.height_pt;
            let (response, painter) =
                ui.allocate_painter(Vec2::new(canvas_w, canvas_h), egui::Sense::hover());

            let origin = response.rect.min;

            // Background canvas rect
            painter.rect_filled(response.rect, 4.0, Color32::from_rgb(248, 250, 252));
            painter.rect_stroke(
                response.rect,
                4.0,
                Stroke::new(1.0, Color32::from_rgb(226, 232, 240)),
                StrokeKind::Inside,
            );

            // Render positioned elements
            for elem in &scene.elements {
                match elem {
                    SceneElement::Rect {
                        rect,
                        fill,
                        stroke,
                        corner_radius,
                    } => {
                        let screen_rect = Rect::from_min_size(
                            Pos2::new(origin.x + rect.x, origin.y + rect.y),
                            Vec2::new(rect.w, rect.h),
                        );
                        let fill_c = fill.map_or(Color32::TRANSPARENT, to_egui_color);
                        let stroke_s = stroke.map_or(Stroke::NONE, |(c, w)| {
                            Stroke::new(w, to_egui_color(c))
                        });
                        painter.rect(
                            screen_rect,
                            *corner_radius,
                            fill_c,
                            stroke_s,
                            StrokeKind::Inside,
                        );
                    }
                    SceneElement::Text {
                        x,
                        y,
                        text,
                        font_size_pt,
                        color,
                        ..
                    } => {
                        let text_pos = Pos2::new(origin.x + x, origin.y + y);
                        painter.text(
                            text_pos,
                            egui::Align2::LEFT_TOP,
                            text,
                            egui::FontId::proportional(*font_size_pt),
                            to_egui_color(*color),
                        );
                    }
                    SceneElement::Line {
                        x1,
                        y1,
                        x2,
                        y2,
                        color,
                        width,
                        ..
                    } => {
                        painter.line_segment(
                            [
                                Pos2::new(origin.x + x1, origin.y + y1),
                                Pos2::new(origin.x + x2, origin.y + y2),
                            ],
                            Stroke::new(*width, to_egui_color(*color)),
                        );
                    }
                    SceneElement::Arrow {
                        x1,
                        y1,
                        x2,
                        y2,
                        color,
                        width,
                    } => {
                        painter.line_segment(
                            [
                                Pos2::new(origin.x + x1, origin.y + y1),
                                Pos2::new(origin.x + x2, origin.y + y2),
                            ],
                            Stroke::new(*width, to_egui_color(*color)),
                        );
                    }
                    SceneElement::PillBadge {
                        rect,
                        bg_color,
                        text,
                        text_color,
                        font_size_pt,
                    } => {
                        let screen_rect = Rect::from_min_size(
                            Pos2::new(origin.x + rect.x, origin.y + rect.y),
                            Vec2::new(rect.w, rect.h),
                        );
                        painter.rect_filled(screen_rect, 12.0, to_egui_color(*bg_color));
                        painter.text(
                            screen_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            text,
                            egui::FontId::proportional(*font_size_pt),
                            to_egui_color(*text_color),
                        );
                    }
                    SceneElement::Chevron { x, y, size, color } => {
                        let p1 = Pos2::new(origin.x + x, origin.y + y);
                        let p2 = Pos2::new(origin.x + x + size * 0.5, origin.y + y + size * 0.5);
                        let p3 = Pos2::new(origin.x + x, origin.y + y + size);
                        painter.add(egui::Shape::line(
                            vec![p1, p2, p3],
                            Stroke::new(2.0, to_egui_color(*color)),
                        ));
                    }
                }
            }
        }
    }
}

fn to_egui_color(c: RgbColor) -> Color32 {
    Color32::from_rgb(c.r, c.g, c.b)
}
