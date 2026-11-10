//! Interactive 26-type Excel vector chart widget for ONEOFFICE Studio.

use egui::{Color32, TextureHandle, TextureOptions, Ui};
use oneoffice_bridge::chart::{ChartSeriesInput, ChartSpecInput, UnifiedChartEngine, list_chart_kinds};

/// State for an interactive chart embedded inside a document or spreadsheet.
#[derive(Clone)]
pub struct ChartWidget {
    pub is_visible: bool,
    pub title: String,
    pub selected_kind: String,
    pub width_pt: f32,
    pub height_pt: f32,
    pub last_png_bytes: Option<Vec<u8>>,
    pub texture: Option<TextureHandle>,
}

impl std::fmt::Debug for ChartWidget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChartWidget")
            .field("is_visible", &self.is_visible)
            .field("title", &self.title)
            .field("selected_kind", &self.selected_kind)
            .field("width_pt", &self.width_pt)
            .field("height_pt", &self.height_pt)
            .field("has_png_bytes", &self.last_png_bytes.is_some())
            .field("has_texture", &self.texture.is_some())
            .finish()
    }
}

impl Default for ChartWidget {
    fn default() -> Self {
        Self {
            is_visible: false,
            title: "2026 사업 예산 배분 및 집행 분석".to_string(),
            selected_kind: "column_clustered".to_string(),
            width_pt: 520.0,
            height_pt: 260.0,
            last_png_bytes: None,
            texture: None,
        }
    }
}

impl ChartWidget {
    /// Creates a newly initialized chart widget.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Rebuilds the chart from the given table data.
    pub fn rebuild_from_table(&mut self, ctx: &egui::Context, table: &[Vec<String>], dark_mode: bool) {
        if table.len() < 2 {
            return;
        }

        // Row 0: Headers (e.g. ["항목", "1차 예산", "2차 예산", "소계"])
        // Rows 1..N-1: Data categories and series values
        let header_row = &table[0];
        let mut categories = Vec::new();
        let mut series_1_vals = Vec::new();
        let mut series_2_vals = Vec::new();

        let s1_name = header_row.get(1).cloned().unwrap_or_else(|| "1차 예산".into());
        let s2_name = header_row.get(2).cloned().unwrap_or_else(|| "2차 예산".into());

        for row in table.iter().skip(1) {
            // Skip summary rows like "합계"
            if let Some(cat) = row.first() {
                if cat.contains("합계") || cat.contains("SUM") {
                    continue;
                }
                categories.push(cat.clone());
            }
            if let Some(v1_str) = row.get(1) {
                let v = v1_str.replace(',', "").parse::<f64>().unwrap_or(0.0);
                series_1_vals.push(v);
            }
            if let Some(v2_str) = row.get(2) {
                let v = v2_str.replace(',', "").parse::<f64>().unwrap_or(0.0);
                series_2_vals.push(v);
            }
        }

        if categories.is_empty() {
            categories = vec!["클라우드".into(), "인프라".into(), "소프트웨어".into()];
            series_1_vals = vec![12500000.0, 18200000.0, 6000000.0];
            series_2_vals = vec![4300000.0, 6100000.0, 2000000.0];
        }

        let mut series = vec![ChartSeriesInput::new(s1_name, series_1_vals)];
        if !series_2_vals.is_empty() {
            series.push(ChartSeriesInput::new(s2_name, series_2_vals));
        }

        let spec = ChartSpecInput {
            title: Some(self.title.clone()),
            kind: self.selected_kind.clone(),
            categories,
            series,
            width_pt: self.width_pt,
            height_pt: self.height_pt,
            legend_pos: Some("bottom".to_string()),
            trendline: None,
            dark_mode,
        };

        if let Ok(bundle) = UnifiedChartEngine::build_chart(&spec)
            && let Ok(dyn_img) = image::load_from_memory(&bundle.png_bytes)
        {
            let rgba = dyn_img.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            let pixels = rgba.into_raw();
            let color_img = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
            let tex = ctx.load_texture("embedded_chart_texture", color_img, TextureOptions::LINEAR);
            self.texture = Some(tex);
            self.last_png_bytes = Some(bundle.png_bytes);
        }
    }

    /// Renders the chart controls bar and the chart canvas inside the document.
    pub fn show(&mut self, ui: &mut Ui, ctx: &egui::Context, table: &[Vec<String>], dark_mode: bool) {
        if !self.is_visible {
            return;
        }

        ui.vertical(|ui| {
            // Chart Control Toolbar
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("26종 엑셀 차트 엔진:").strong());

                let chart_kinds = list_chart_kinds();
                let mut kind_changed = false;
                egui::ComboBox::from_id_salt("chart_kind_picker")
                    .selected_text(&self.selected_kind)
                    .width(160.0)
                    .show_ui(ui, |ui| {
                        for kind in chart_kinds {
                            let label = match kind {
                                "column_clustered" => "세로 막대형 (기본)",
                                "column_stacked" => "누적 세로 막대",
                                "bar_clustered" => "가로 막대형",
                                "bar_stacked" => "누적 가로 막대",
                                "line" => "꺾은선형",
                                "line_markers" => "표식 있는 꺾은선형",
                                "pie_2d" => "2D 원형 차트",
                                "doughnut" => "도넛형 차트",
                                "area" => "영역형 차트",
                                "scatter" => "분산형 (XY)",
                                "waterfall" => "폭포수형 (Waterfall)",
                                "radar" => "방사형 차트",
                                other => other,
                            };
                            let is_selected = self.selected_kind == kind;
                            if ui.selectable_label(is_selected, label).clicked() {
                                self.selected_kind = kind.to_string();
                                kind_changed = true;
                            }
                        }
                    });

                if kind_changed || self.texture.is_none() {
                    self.rebuild_from_table(ctx, table, dark_mode);
                }

                if ui.button("새로고침").clicked() {
                    self.rebuild_from_table(ctx, table, dark_mode);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("X").on_hover_text("차트 제거").clicked() {
                        self.is_visible = false;
                    }
                });
            });

            ui.add_space(4.0);

            // Chart Render Canvas
            if let Some(tex) = &self.texture {
                let w = self.width_pt.min(ui.available_width());
                let aspect = tex.size()[0] as f32 / (tex.size()[1].max(1) as f32);
                let h = w / aspect.max(0.1);
                ui.image((tex.id(), egui::vec2(w, h)));
            } else {
                ui.label(
                    egui::RichText::new("차트 렌더링 중...")
                        .color(Color32::GRAY)
                        .size(11.0),
                );
            }
        });
    }
}
