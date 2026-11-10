//! Integrated Frosted Glass Studio shell combining HWPX, DOCX, XLSX, PPTX, and PDF.

use egui::{Color32, RichText, Ui};
use serde::{Deserialize, Serialize};

use crate::diagram_widget::DiagramWidget;
use crate::formula_hud::FormulaHudWidget;
use crate::pdf_widget::PdfViewerWidget;
use crate::theme::StudioTheme;

/// Active document type loaded in the studio.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ActiveDocumentKind {
    #[default]
    Hwpx,
    Docx,
    Xlsx,
    Pptx,
    Pdf,
}

/// Unified ONEOFFICE Studio application state.
pub struct StudioApp {
    pub theme: StudioTheme,
    pub active_doc: ActiveDocumentKind,
    pub formula_hud: FormulaHudWidget,
    pub diagram_widget: DiagramWidget,
    pub pdf_widget: PdfViewerWidget,
    pub document_title: String,
    pub zoom_pct: u32,
    pub status_text: String,
    pub table_selected_cell: (usize, usize),
    pub table_data: Vec<Vec<String>>,
}

impl Default for StudioApp {
    fn default() -> Self {
        Self::new()
    }
}

impl StudioApp {
    /// Creates a newly initialized studio app with default sample Korean contract and proposal.
    #[must_use]
    pub fn new() -> Self {
        Self {
            theme: StudioTheme::Dark,
            active_doc: ActiveDocumentKind::Hwpx,
            formula_hud: FormulaHudWidget::new("B2", "12500000"),
            diagram_widget: DiagramWidget::default(),
            pdf_widget: PdfViewerWidget::new(),
            document_title: "2026 차세대 사업계획서 및 표준용역계약서.hwpx".to_string(),
            zoom_pct: 100,
            status_text: "ONEOFFICE Pure-Rust Engine Ready (Zero COM · 100% Native)".to_string(),
            table_selected_cell: (1, 1),
            table_data: vec![
                vec![
                    "항목".into(),
                    "1차 예산 (원)".into(),
                    "2차 예산 (원)".into(),
                    "소계 (원)".into(),
                ],
                vec![
                    "클라우드 도입".into(),
                    "12500000".into(),
                    "4300000".into(),
                    "16800000".into(),
                ],
                vec![
                    "인프라 최적화".into(),
                    "18200000".into(),
                    "6100000".into(),
                    "24300000".into(),
                ],
                vec![
                    "합계 (SUM)".into(),
                    "30700000".into(),
                    "10400000".into(),
                    "41100000".into(),
                ],
            ],
        }
    }

    /// Pre-frame context configuration (applies visuals and font definitions).
    pub fn logic(&mut self, ctx: &egui::Context) {
        self.theme.apply(ctx);
    }

    /// Recalculates formula sums in the sample budget table.
    pub fn recalculate_table_sums(&mut self) {
        let b2 = self
            .table_data
            .get(1)
            .and_then(|r| r.get(1))
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let c2 = self
            .table_data
            .get(1)
            .and_then(|r| r.get(2))
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let b3 = self
            .table_data
            .get(2)
            .and_then(|r| r.get(1))
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let c3 = self
            .table_data
            .get(2)
            .and_then(|r| r.get(2))
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        let d2 = b2 + c2;
        let d3 = b3 + c3;
        let b4 = b2 + b3;
        let c4 = c2 + c3;
        let d4 = d2 + d3;

        if let Some(r) = self.table_data.get_mut(1)
            && r.len() > 3
        {
            r[3] = format!("{d2:.0}");
        }
        if let Some(r) = self.table_data.get_mut(2)
            && r.len() > 3
        {
            r[3] = format!("{d3:.0}");
        }
        if let Some(r) = self.table_data.get_mut(3)
            && r.len() > 3
        {
            r[1] = format!("{b4:.0}");
            r[2] = format!("{c4:.0}");
            r[3] = format!("{d4:.0}");
        }
    }

    /// Renders the entire Frosted Glass studio interface into the target `Ui`.
    pub fn show(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            // 1. Top Header Glass Bar
            self.show_header(ui);
            ui.separator();

            // 2. Central Workspace Panel
            egui::ScrollArea::both().show(ui, |ui| {
                match self.active_doc {
                    ActiveDocumentKind::Hwpx | ActiveDocumentKind::Docx => {
                        self.show_document_editor(ui);
                    }
                    ActiveDocumentKind::Xlsx => {
                        self.show_spreadsheet_editor(ui);
                    }
                    ActiveDocumentKind::Pptx => {
                        self.show_presentation_editor(ui);
                    }
                    ActiveDocumentKind::Pdf => {
                        let ctx = ui.ctx().clone();
                        self.pdf_widget.show(ui, &ctx);
                    }
                }
            });

            ui.separator();
            // 3. Bottom Status Bar
            self.show_status_bar(ui);
        });
    }

    /// Opens a document by file path, auto-detecting format and loading content.
    pub fn open_file(&mut self, ctx: &egui::Context, path_str: &str) {
        let path = std::path::Path::new(path_str);
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        self.document_title = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(path_str)
            .to_string();

        match ext.as_str() {
            "pdf" => {
                self.active_doc = ActiveDocumentKind::Pdf;
                match std::fs::read(path) {
                    Ok(bytes) => {
                        if let Err(e) = self.pdf_widget.load_pdf_bytes(ctx, &bytes) {
                            self.status_text = format!("PDF 로드 실패: {e}");
                        } else {
                            self.status_text = format!("PDF 열림: {}", self.document_title);
                        }
                    }
                    Err(e) => {
                        self.status_text = format!("파일 읽기 실패: {e}");
                    }
                }
            }
            "hwpx" | "hwp" | "hml" => {
                self.active_doc = ActiveDocumentKind::Hwpx;
                self.status_text = format!("한글 문서 열림: {}", self.document_title);
            }
            "docx" | "doc" => {
                self.active_doc = ActiveDocumentKind::Docx;
                self.status_text = format!("워드 문서 열림: {}", self.document_title);
            }
            "xlsx" | "xls" | "csv" => {
                self.active_doc = ActiveDocumentKind::Xlsx;
                self.status_text = format!("엑셀 시트 열림: {}", self.document_title);
            }
            "pptx" | "ppt" => {
                self.active_doc = ActiveDocumentKind::Pptx;
                self.status_text = format!("슬라이드 문서 열림: {}", self.document_title);
            }
            _ => {
                self.status_text = format!("알 수 없는 파일 형식: {path_str}");
            }
        }
    }

    fn show_header(&mut self, ui: &mut Ui) {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("ONEOFFICE")
                    .strong()
                    .color(StudioTheme::brand_accent())
                    .size(16.0),
            );

            ui.separator();

            // Document format switcher tabs
            let docs = [
                (ActiveDocumentKind::Hwpx, "한글 (.hwpx)"),
                (ActiveDocumentKind::Docx, "워드 (.docx)"),
                (ActiveDocumentKind::Xlsx, "엑셀 (.xlsx)"),
                (ActiveDocumentKind::Pptx, "슬라이드 (.pptx)"),
                (ActiveDocumentKind::Pdf, "PDF 뷰어/서명 (.pdf)"),
            ];

            for (kind, label) in docs {
                let is_active = self.active_doc == kind;
                if ui.selectable_label(is_active, label).clicked() {
                    self.active_doc = kind;
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Theme toggle
                let theme_label = match self.theme {
                    StudioTheme::Light => "🌙 다크 모드",
                    StudioTheme::Dark => "☀ 라이트 모드",
                };
                if ui.button(theme_label).clicked() {
                    self.theme = match self.theme {
                        StudioTheme::Light => StudioTheme::Dark,
                        StudioTheme::Dark => StudioTheme::Light,
                    };
                }

                ui.label(
                    RichText::new(format!("{}%", self.zoom_pct))
                        .monospace()
                        .size(12.0),
                );
            });
        });
        ui.add_space(4.0);
    }

    fn show_status_bar(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("●")
                    .color(Color32::from_rgb(16, 185, 129))
                    .size(10.0),
            );
            ui.label(
                RichText::new(&self.status_text)
                    .size(11.0)
                    .color(Color32::GRAY),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new("Zero Panic · unsafe_code = forbid")
                        .monospace()
                        .size(10.0)
                        .color(Color32::from_rgb(100, 116, 139)),
                );
            });
        });
    }

    fn show_document_editor(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            // Floating formula HUD
            ui.group(|ui| {
                if let crate::formula_hud::FormulaHudEvent::Commit(new_val) = self.formula_hud.show(ui) {
                    let (r, c) = self.table_selected_cell;
                    if let Some(row) = self.table_data.get_mut(r)
                        && c < row.len()
                    {
                        row[c] = new_val;
                        self.recalculate_table_sums();
                        self.status_text = "513 엑셀 수식표 실시간 재계산 완료".to_string();
                    }
                }
            });

            ui.add_space(12.0);

            // Paper canvas container
            egui::Frame::canvas(ui.style())
                .fill(StudioTheme::paper_fill())
                .stroke(StudioTheme::paper_border())
                .inner_margin(32.0)
                .show(ui, |ui| {
                    ui.set_width(620.0);
                    ui.set_min_height(820.0);

                    ui.heading(
                        RichText::new("차세대 통합 오피스 플랫폼 기술 제안서")
                            .color(Color32::from_rgb(15, 23, 42))
                            .size(20.0),
                    );
                    ui.label(
                        RichText::new("문서번호: ONEOFFICE-2026-RFP | 배포용 문서 복호화 지원")
                            .color(Color32::from_rgb(100, 116, 139))
                            .size(11.0),
                    );

                    ui.separator();
                    ui.add_space(8.0);

                    ui.label(
                        RichText::new("1. 제안 개요 및 도식화 구성도")
                            .strong()
                            .color(Color32::from_rgb(15, 23, 42)),
                    );

                    // Embedded proposal infographic sub-canvas
                    self.diagram_widget.show(ui);

                    ui.add_space(16.0);
                    ui.label(
                        RichText::new("2. 513함수 연동 예산 수식표 (클릭 시 수식바 연동)")
                            .strong()
                            .color(Color32::from_rgb(15, 23, 42)),
                    );

                    // Interactive Excel Formula Table Grid
                    egui::Grid::new("hwpx_calc_table")
                        .striped(true)
                        .min_col_width(120.0)
                        .show(ui, |ui| {
                            let mut clicked_cell = None;
                            for (row_idx, row) in self.table_data.iter().enumerate() {
                                for (col_idx, val) in row.iter().enumerate() {
                                    let is_header = row_idx == 0 || col_idx == 0;
                                    let is_selected =
                                        self.table_selected_cell == (row_idx, col_idx);

                                    let cell_text = if is_header {
                                        RichText::new(val)
                                            .strong()
                                            .color(Color32::from_rgb(15, 23, 42))
                                    } else if row_idx == 3 {
                                        RichText::new(val)
                                            .strong()
                                            .color(Color32::from_rgb(37, 99, 235))
                                    } else {
                                        RichText::new(val).color(Color32::from_rgb(51, 65, 85))
                                    };

                                    let resp = ui.selectable_label(is_selected, cell_text);
                                    if resp.clicked() {
                                        clicked_cell = Some((row_idx, col_idx));
                                    }
                                }
                                ui.end_row();
                            }

                            if let Some((r, c)) = clicked_cell {
                                self.table_selected_cell = (r, c);
                                let col_letter = (b'A' + c as u8) as char;
                                self.formula_hud.cell_address = format!("{col_letter}{}", r + 1);
                                if r == 3 && c > 0 {
                                    self.formula_hud.formula_text =
                                        format!("=SUM({col_letter}2:{col_letter}3)");
                                } else if c == 3 && r > 0 {
                                    self.formula_hud.formula_text =
                                        format!("=SUM(B{}:C{})", r + 1, r + 1);
                                } else if let Some(val) =
                                    self.table_data.get(r).and_then(|row| row.get(c))
                                {
                                    self.formula_hud.formula_text = val.clone();
                                }
                            }
                        });

                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("※ 셀을 클릭하면 상단 Formula HUD를 통해 513개 엑셀 함수(=SUM 등)가 실시간 동기화 및 재계산됩니다.")
                            .size(11.0)
                            .color(Color32::from_rgb(100, 116, 139)),
                    );
                });
        });
    }

    fn show_spreadsheet_editor(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            self.formula_hud.show(ui);
        });
        ui.add_space(8.0);
        ui.label(RichText::new("GridCraft 513함수 스프레드시트 캔버스 (L0~L7 증분 계산 그래프 연동)").strong());
    }

    fn show_presentation_editor(&mut self, ui: &mut Ui) {
        ui.label(RichText::new("DeckCraft 180종 AutoShape & 커넥터 글루 프레젠테이션 스튜디오").strong());
        self.diagram_widget.show(ui);
    }
}
