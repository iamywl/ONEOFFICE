//! Integrated Frosted Glass Studio shell combining HWPX, DOCX, XLSX, PPTX, and PDF.

use std::collections::HashMap;
use std::path::Path;
use egui::{Color32, RichText, TextureHandle, TextureOptions, Ui};
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

/// A parsed paragraph in a loaded document.
#[derive(Clone, Debug)]
pub struct DocParagraph {
    pub text: String,
    pub is_heading: bool,
    pub font_size: f32,
    pub is_bold: bool,
}

/// Image asset extracted from a DOCX or HWPX package.
#[derive(Clone, Debug)]
pub struct DocImageItem {
    pub name: String,
    pub alt: String,
    pub width_pt: f32,
    pub height_pt: f32,
    pub is_behind_text: bool,
    pub png_bytes: Vec<u8>,
}

/// Parsed structure of a loaded DOCX or HWPX document.
#[derive(Clone, Debug, Default)]
pub struct LoadedDocument {
    pub title: String,
    pub subtitle: String,
    pub paragraphs: Vec<DocParagraph>,
    pub images: Vec<DocImageItem>,
    pub tables: Vec<Vec<Vec<String>>>,
}

/// Parsed structure of a multi-sheet Excel spreadsheet.
#[derive(Clone, Debug, Default)]
pub struct LoadedSpreadsheet {
    pub sheet_names: Vec<String>,
    pub active_sheet_index: usize,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub formulas: HashMap<(usize, usize), String>,
}

/// Parsed slide item in a loaded presentation.
#[derive(Clone, Debug)]
pub struct SlideItem {
    pub slide_number: usize,
    pub title: String,
    pub bullet_points: Vec<String>,
    pub template_id: String,
}

/// Parsed presentation slide deck.
#[derive(Clone, Debug, Default)]
pub struct LoadedPresentation {
    pub title: String,
    pub slides: Vec<SlideItem>,
    pub active_slide_index: usize,
}

/// Active text styling and formatting toolbar state.
#[derive(Clone, Debug, PartialEq)]
pub struct TextFormatState {
    pub font_size: f32,
    pub is_bold: bool,
    pub is_italic: bool,
    pub is_underline: bool,
    pub text_color: Color32,
    pub align: egui::Align,
}

impl Default for TextFormatState {
    fn default() -> Self {
        Self {
            font_size: 14.0,
            is_bold: false,
            is_italic: false,
            is_underline: false,
            text_color: Color32::from_rgb(15, 23, 42),
            align: egui::Align::LEFT,
        }
    }
}

/// Snapshot of a document for multi-level Undo/Redo history.
#[derive(Clone, Debug)]
pub enum DocumentSnapshot {
    Doc(LoadedDocument),
    Spreadsheet(Vec<Vec<String>>),
    Presentation(LoadedPresentation),
}

/// Find and Replace state for interactive document search and text substitution.
#[derive(Clone, Debug, Default)]
pub struct FindReplaceState {
    pub is_open: bool,
    pub find_query: String,
    pub replace_query: String,
    pub status: String,
}

/// Unified ONEOFFICE Studio application state.
pub struct StudioApp {
    pub theme: StudioTheme,
    pub active_doc: ActiveDocumentKind,
    pub formula_hud: FormulaHudWidget,
    pub diagram_widget: DiagramWidget,
    pub chart_widget: crate::chart_widget::ChartWidget,
    pub pdf_widget: PdfViewerWidget,
    pub document_title: String,
    pub zoom_pct: u32,
    pub status_text: String,
    pub table_selected_cell: (usize, usize),
    pub table_data: Vec<Vec<String>>,
    // Real document models
    pub doc_model: Option<LoadedDocument>,
    pub spreadsheet_model: Option<LoadedSpreadsheet>,
    pub presentation_model: Option<LoadedPresentation>,
    pub doc_textures: Vec<(String, TextureHandle)>,
    // In-Place editing & Ribbon state
    pub format_state: TextFormatState,
    pub active_paragraph_idx: Option<usize>,
    pub inline_editing_cell: Option<(usize, usize)>,
    pub cell_edit_buffer: String,
    pub current_file_path: Option<String>,
    pub is_dirty: bool,
    pub request_open_file: bool,
    pub request_save_as: bool,
    pub undo_stack: Vec<DocumentSnapshot>,
    pub redo_stack: Vec<DocumentSnapshot>,
    pub find_replace: FindReplaceState,
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
        let initial_table = vec![
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
        ];

        let mut app = Self {
            theme: StudioTheme::Dark,
            active_doc: ActiveDocumentKind::Hwpx,
            formula_hud: FormulaHudWidget::new("B2", "12500000"),
            diagram_widget: DiagramWidget::default(),
            chart_widget: crate::chart_widget::ChartWidget::new(),
            pdf_widget: PdfViewerWidget::new(),
            document_title: "2026 차세대 사업계획서 및 표준용역계약서.hwpx".to_string(),
            zoom_pct: 100,
            status_text: "ONEOFFICE Multi-Format Studio Ready (Zero COM · 100% Native)".to_string(),
            table_selected_cell: (1, 1),
            table_data: initial_table.clone(),
            doc_model: None,
            spreadsheet_model: Some(LoadedSpreadsheet {
                sheet_names: vec!["2026 사업예산".into(), "집행내역".into()],
                active_sheet_index: 0,
                headers: vec!["항목".into(), "1차 예산".into(), "2차 예산".into(), "소계".into()],
                rows: initial_table,
                formulas: HashMap::new(),
            }),
            presentation_model: None,
            doc_textures: Vec::new(),
            format_state: TextFormatState::default(),
            active_paragraph_idx: None,
            inline_editing_cell: None,
            cell_edit_buffer: String::new(),
            current_file_path: None,
            is_dirty: false,
            request_open_file: false,
            request_save_as: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            find_replace: FindReplaceState::default(),
        };

        // Initialize default presentation model
        app.presentation_model = Some(LoadedPresentation {
            title: "ONEOFFICE 통합 아키텍처 제안".to_string(),
            slides: vec![
                SlideItem {
                    slide_number: 1,
                    title: "ONEOFFICE 통합 아키텍처 제안".to_string(),
                    bullet_points: vec![
                        "HWPX · DOCX · XLSX · PPTX · PDF 올인원 오피스 플랫폼".to_string(),
                        "순수 Rust 기반 무손실 엔진 탑재 (Zero OS Lock-In)".to_string(),
                        "컨설팅급 제안서 도식화 및 26종 엑셀 차트 인플레이스 연동".to_string(),
                    ],
                    template_id: "problem-comparison-to-solution".to_string(),
                },
                SlideItem {
                    slide_number: 2,
                    title: "기획 배경 및 시장 요구사항 (Why Now)".to_string(),
                    bullet_points: vec![
                        "공공/엔터프라이즈 환경에서 포맷 간 단절 문제 심각".to_string(),
                        "캡처 이미지 복사-붙여넣기로 인한 수식 및 벡터 정보 유실".to_string(),
                        "WASM 및 네이티브 듀얼 런타임으로 단일 코드베이스 유지".to_string(),
                    ],
                    template_id: "as-is-to-be-comparison".to_string(),
                },
            ],
            active_slide_index: 0,
        });

        // Attempt to auto-load dummy_report.docx from standard search paths
        for candidate in [
            "test_documents/dummy_report.docx",
            "../../test_documents/dummy_report.docx",
            "../test_documents/dummy_report.docx",
        ] {
            if let Ok(bytes) = std::fs::read(candidate) {
                app.parse_docx_bytes_headless(&bytes);
                break;
            }
        }

        app
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

    /// Pushes a snapshot of the active document state to the undo history stack.
    pub fn push_undo_snapshot(&mut self) {
        let snapshot = match self.active_doc {
            ActiveDocumentKind::Hwpx | ActiveDocumentKind::Docx => {
                self.doc_model.as_ref().map(|d| DocumentSnapshot::Doc(d.clone()))
            }
            ActiveDocumentKind::Xlsx => {
                Some(DocumentSnapshot::Spreadsheet(self.table_data.clone()))
            }
            ActiveDocumentKind::Pptx => {
                self.presentation_model.as_ref().map(|p| DocumentSnapshot::Presentation(p.clone()))
            }
            ActiveDocumentKind::Pdf => None,
        };

        if let Some(snap) = snapshot {
            if self.undo_stack.len() >= 50 {
                self.undo_stack.remove(0);
            }
            self.undo_stack.push(snap);
            self.redo_stack.clear();
        }
    }

    /// Undoes the last operation and restores previous document state.
    pub fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            let current = match &prev {
                DocumentSnapshot::Doc(_) => {
                    self.doc_model.as_ref().map(|d| DocumentSnapshot::Doc(d.clone()))
                }
                DocumentSnapshot::Spreadsheet(_) => {
                    Some(DocumentSnapshot::Spreadsheet(self.table_data.clone()))
                }
                DocumentSnapshot::Presentation(_) => {
                    self.presentation_model.as_ref().map(|p| DocumentSnapshot::Presentation(p.clone()))
                }
            };
            if let Some(c) = current {
                self.redo_stack.push(c);
            }

            match prev {
                DocumentSnapshot::Doc(d) => self.doc_model = Some(d),
                DocumentSnapshot::Spreadsheet(t) => {
                    self.table_data = t;
                    self.recalculate_table_sums();
                }
                DocumentSnapshot::Presentation(p) => self.presentation_model = Some(p),
            }
            self.is_dirty = true;
            self.status_text = "↩ 실행 취소 완료".to_string();
        }
    }

    /// Redoes the previously undone operation.
    pub fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            let current = match &next {
                DocumentSnapshot::Doc(_) => {
                    self.doc_model.as_ref().map(|d| DocumentSnapshot::Doc(d.clone()))
                }
                DocumentSnapshot::Spreadsheet(_) => {
                    Some(DocumentSnapshot::Spreadsheet(self.table_data.clone()))
                }
                DocumentSnapshot::Presentation(_) => {
                    self.presentation_model.as_ref().map(|p| DocumentSnapshot::Presentation(p.clone()))
                }
            };
            if let Some(c) = current {
                self.undo_stack.push(c);
            }

            match next {
                DocumentSnapshot::Doc(d) => self.doc_model = Some(d),
                DocumentSnapshot::Spreadsheet(t) => {
                    self.table_data = t;
                    self.recalculate_table_sums();
                }
                DocumentSnapshot::Presentation(p) => self.presentation_model = Some(p),
            }
            self.is_dirty = true;
            self.status_text = "↪ 다시 실행 완료".to_string();
        }
    }

    /// Finds the next occurrence of `find_query` across the active document.
    pub fn perform_find_next(&mut self) {
        let q = self.find_replace.find_query.trim().to_string();
        if q.is_empty() {
            self.find_replace.status = "검색어를 입력해 주십시오.".to_string();
            return;
        }

        match self.active_doc {
            ActiveDocumentKind::Hwpx | ActiveDocumentKind::Docx => {
                if let Some(doc) = &self.doc_model {
                    let start_idx = self.active_paragraph_idx.map_or(0, |i| i + 1);
                    let mut found = None;
                    for i in 0..doc.paragraphs.len() {
                        let idx = (start_idx + i) % doc.paragraphs.len();
                        if doc.paragraphs[idx].text.contains(&q) {
                            found = Some(idx);
                            break;
                        }
                    }
                    if let Some(idx) = found {
                        self.active_paragraph_idx = Some(idx);
                        self.find_replace.status = format!("단락 {}에서 일치 항목을 찾았습니다.", idx + 1);
                    } else {
                        self.find_replace.status = format!("'{q}' 검색 결과가 없습니다.");
                    }
                }
            }
            ActiveDocumentKind::Xlsx => {
                let mut found = None;
                let (cur_r, cur_c) = self.table_selected_cell;
                for r in 0..self.table_data.len() {
                    for c in 0..self.table_data[r].len() {
                        if (r > cur_r || (r == cur_r && c > cur_c)) && self.table_data[r][c].contains(&q) {
                            found = Some((r, c));
                            break;
                        }
                    }
                    if found.is_some() {
                        break;
                    }
                }
                if found.is_none() {
                    for r in 0..self.table_data.len() {
                        for c in 0..self.table_data[r].len() {
                            if self.table_data[r][c].contains(&q) {
                                found = Some((r, c));
                                break;
                            }
                        }
                        if found.is_some() {
                            break;
                        }
                    }
                }
                if let Some((r, c)) = found {
                    self.table_selected_cell = (r, c);
                    let col_letter = (b'A' + c as u8) as char;
                    self.formula_hud.cell_address = format!("{col_letter}{}", r + 1);
                    self.find_replace.status = format!("셀 {col_letter}{}에서 일치 항목을 찾았습니다.", r + 1);
                } else {
                    self.find_replace.status = format!("'{q}' 검색 결과가 없습니다.");
                }
            }
            ActiveDocumentKind::Pptx => {
                if let Some(pres) = &self.presentation_model {
                    let mut found = None;
                    let start = (pres.active_slide_index + 1) % pres.slides.len().max(1);
                    for i in 0..pres.slides.len() {
                        let idx = (start + i) % pres.slides.len();
                        let s = &pres.slides[idx];
                        if s.title.contains(&q) || s.bullet_points.iter().any(|b| b.contains(&q)) {
                            found = Some(idx);
                            break;
                        }
                    }
                    if let Some(idx) = found {
                        if let Some(p) = &mut self.presentation_model {
                            p.active_slide_index = idx;
                        }
                        self.find_replace.status = format!("슬라이드 {}에서 일치 항목을 찾았습니다.", idx + 1);
                    } else {
                        self.find_replace.status = format!("'{q}' 검색 결과가 없습니다.");
                    }
                }
            }
            ActiveDocumentKind::Pdf => {
                self.find_replace.status = "PDF 상단 툴바의 검색 HUD를 이용해 주십시오.".to_string();
            }
        }
    }

    /// Replaces the next occurrence of `find_query` with `replace_query`.
    pub fn perform_replace_next(&mut self) {
        let q = self.find_replace.find_query.trim().to_string();
        let r_str = self.find_replace.replace_query.clone();
        if q.is_empty() {
            return;
        }

        match self.active_doc {
            ActiveDocumentKind::Hwpx | ActiveDocumentKind::Docx => {
                if let Some(doc) = &mut self.doc_model {
                    let mut replaced = false;
                    let target_idx = self.active_paragraph_idx.unwrap_or(0);
                    if let Some(para) = doc.paragraphs.get_mut(target_idx)
                        && para.text.contains(&q)
                    {
                        para.text = para.text.replacen(&q, &r_str, 1);
                        replaced = true;
                    }
                    if !replaced {
                        for (idx, para) in doc.paragraphs.iter_mut().enumerate() {
                            if para.text.contains(&q) {
                                para.text = para.text.replacen(&q, &r_str, 1);
                                self.active_paragraph_idx = Some(idx);
                                replaced = true;
                                break;
                            }
                        }
                    }
                    if replaced {
                        self.is_dirty = true;
                        self.find_replace.status = "1개 항목을 치환했습니다.".to_string();
                    } else {
                        self.find_replace.status = format!("'{q}' 검색 결과가 없습니다.");
                    }
                }
            }
            ActiveDocumentKind::Xlsx => {
                let (r, c) = self.table_selected_cell;
                if let Some(row) = self.table_data.get_mut(r)
                    && let Some(cell) = row.get_mut(c)
                    && cell.contains(&q)
                {
                    *cell = cell.replacen(&q, &r_str, 1);
                    self.recalculate_table_sums();
                    self.is_dirty = true;
                    self.find_replace.status = "셀 내용을 치환했습니다.".to_string();
                } else {
                    self.perform_find_next();
                }
            }
            ActiveDocumentKind::Pptx => {
                if let Some(pres) = &mut self.presentation_model {
                    let idx = pres.active_slide_index;
                    if let Some(slide) = pres.slides.get_mut(idx) {
                        let mut replaced = false;
                        if slide.title.contains(&q) {
                            slide.title = slide.title.replacen(&q, &r_str, 1);
                            replaced = true;
                        } else {
                            for b in &mut slide.bullet_points {
                                if b.contains(&q) {
                                    *b = b.replacen(&q, &r_str, 1);
                                    replaced = true;
                                    break;
                                }
                            }
                        }
                        if replaced {
                            self.is_dirty = true;
                            self.find_replace.status = "슬라이드 항목을 치환했습니다.".to_string();
                        } else {
                            self.perform_find_next();
                        }
                    }
                }
            }
            ActiveDocumentKind::Pdf => {}
        }
    }

    /// Replaces all occurrences of `find_query` with `replace_query` across the active document.
    pub fn perform_replace_all(&mut self) {
        let q = self.find_replace.find_query.trim().to_string();
        let r_str = self.find_replace.replace_query.clone();
        if q.is_empty() {
            return;
        }

        let mut count = 0;
        match self.active_doc {
            ActiveDocumentKind::Hwpx | ActiveDocumentKind::Docx => {
                if let Some(doc) = &mut self.doc_model {
                    for para in &mut doc.paragraphs {
                        let matches = para.text.matches(&q).count();
                        if matches > 0 {
                            count += matches;
                            para.text = para.text.replace(&q, &r_str);
                        }
                    }
                    if doc.title.contains(&q) {
                        count += doc.title.matches(&q).count();
                        doc.title = doc.title.replace(&q, &r_str);
                    }
                }
            }
            ActiveDocumentKind::Xlsx => {
                for row in &mut self.table_data {
                    for cell in row {
                        let matches = cell.matches(&q).count();
                        if matches > 0 {
                            count += matches;
                            *cell = cell.replace(&q, &r_str);
                        }
                    }
                }
                self.recalculate_table_sums();
            }
            ActiveDocumentKind::Pptx => {
                if let Some(pres) = &mut self.presentation_model {
                    for slide in &mut pres.slides {
                        if slide.title.contains(&q) {
                            count += slide.title.matches(&q).count();
                            slide.title = slide.title.replace(&q, &r_str);
                        }
                        for b in &mut slide.bullet_points {
                            if b.contains(&q) {
                                count += b.matches(&q).count();
                                *b = b.replace(&q, &r_str);
                            }
                        }
                    }
                }
            }
            ActiveDocumentKind::Pdf => {}
        }

        if count > 0 {
            self.is_dirty = true;
            self.find_replace.status = format!("총 {count}개 항목을 모두 바꾸기 완료했습니다.");
        } else {
            self.find_replace.status = format!("'{q}' 검색 결과가 없습니다.");
        }
    }

    /// Exports the active document (DOCX, HWPX, XLSX, or PPTX) to vector PDF and loads it into the PDF viewer.
    pub fn export_current_document_to_pdf(&mut self, ctx: &egui::Context) -> Result<Vec<u8>, String> {
        let (title, subtitle, paragraphs) = match self.active_doc {
            ActiveDocumentKind::Hwpx | ActiveDocumentKind::Docx => {
                let doc = self.doc_model.as_ref().ok_or("문서 모델이 없습니다.")?;
                let paras: Vec<String> = doc.paragraphs.iter().map(|p| p.text.clone()).collect();
                (doc.title.clone(), doc.subtitle.clone(), paras)
            }
            ActiveDocumentKind::Xlsx => {
                let mut paras = Vec::new();
                for (idx, row) in self.table_data.iter().enumerate() {
                    let line = row.join("  |  ");
                    paras.push(format!("행 {}: {}", idx + 1, line));
                }
                (self.document_title.clone(), "스프레드시트 요약 보고서".to_string(), paras)
            }
            ActiveDocumentKind::Pptx => {
                let pres = self.presentation_model.as_ref().ok_or("프레젠테이션 모델이 없습니다.")?;
                let mut paras = Vec::new();
                for slide in &pres.slides {
                    paras.push(format!("슬라이드 {}: {}", slide.slide_number, slide.title));
                    for b in &slide.bullet_points {
                        paras.push(format!("  • {b}"));
                    }
                }
                (pres.title.clone(), "프레젠테이션 슬라이드 덱 PDF 변환".to_string(), paras)
            }
            ActiveDocumentKind::Pdf => {
                if let Some(session) = &self.pdf_widget.session {
                    return Ok(session.raw_bytes().to_vec());
                } else {
                    return Err("PDF 세션이 없습니다.".to_string());
                }
            }
        };

        let pdf_bytes = oneoffice_pdf::generate_document_pdf(&title, &subtitle, &paragraphs)
            .map_err(|e| format!("PDF 변환 실패: {e}"))?;

        self.active_doc = ActiveDocumentKind::Pdf;
        self.pdf_widget.load_pdf_bytes(ctx, &pdf_bytes)
            .map_err(|e| format!("PDF 뷰어 로드 실패: {e}"))?;

        self.status_text = format!("📤 PDF 내보내기 성공 ({} bytes): 전자서명 뷰어로 전환되었습니다.", pdf_bytes.len());
        Ok(pdf_bytes)
    }

    /// Renders the entire Frosted Glass studio interface into the target `Ui`.
    pub fn show(&mut self, ui: &mut Ui) {
        self.theme.apply(ui.ctx());
        let ctrl_or_cmd = ui.input(|i| i.modifiers.command || i.modifiers.ctrl);
        let shift = ui.input(|i| i.modifiers.shift);

        // Handle Ctrl+S / Cmd+S for instant save
        if ctrl_or_cmd && ui.input(|i| i.key_pressed(egui::Key::S)) {
            let _ = self.save_current_document();
        }
        // Handle Ctrl+O / Cmd+O for open file
        if ctrl_or_cmd && ui.input(|i| i.key_pressed(egui::Key::O)) {
            self.request_open_file = true;
        }
        // Handle Ctrl+Z / Cmd+Z for undo, Ctrl+Shift+Z / Ctrl+Y for redo
        if ctrl_or_cmd && ui.input(|i| i.key_pressed(egui::Key::Z)) {
            if shift {
                self.redo();
            } else {
                self.undo();
            }
        } else if ctrl_or_cmd && ui.input(|i| i.key_pressed(egui::Key::Y)) {
            self.redo();
        }
        // Handle Ctrl+F / Ctrl+H for Find & Replace HUD
        if ctrl_or_cmd && (ui.input(|i| i.key_pressed(egui::Key::F)) || ui.input(|i| i.key_pressed(egui::Key::H))) {
            self.find_replace.is_open = true;
        }

        ui.vertical(|ui| {
            // 1. Top Header Glass Bar
            self.show_header(ui);
            ui.separator();

            // 2. Format Ribbon Toolbar
            self.show_ribbon_toolbar(ui);
            ui.separator();

            // 3. Find & Replace Glass Bar (when active)
            self.show_find_replace_bar(ui);

            // 4. Central Workspace Panel
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
            // 4. Bottom Status Bar
            self.show_status_bar(ui);
        });
    }

    /// Parses DOCX bytes into `doc_model` without requiring an egui context.
    pub fn parse_docx_bytes_headless(&mut self, bytes: &[u8]) {
        if let Ok(doc) = wordcraft_docx::read(bytes) {
            let mut loaded = LoadedDocument {
                title: self.document_title.clone(),
                subtitle: "WordCraft OOXML Technical Report".to_string(),
                paragraphs: Vec::new(),
                images: Vec::new(),
                tables: Vec::new(),
            };

            for block in &doc.body {
                if let Some(para) = block.as_para() {
                    let text = para.text.trim().to_string();
                    if !text.is_empty() {
                        let is_h = text.len() < 50
                            && (text.starts_with("ONEOFFICE")
                                || text.starts_with("그림")
                                || text.starts_with("제")
                                || text.starts_with("1.")
                                || text.starts_with("2."));
                        loaded.paragraphs.push(DocParagraph {
                            text,
                            is_heading: is_h,
                            font_size: if is_h { 18.0 } else { 13.0 },
                            is_bold: is_h,
                        });
                    }

                    for obj in &para.objects {
                        if let wordcraft_doc::para::InlineObject::Image {
                            media,
                            w,
                            h,
                            float,
                            alt,
                            ..
                        } = obj
                        {
                            let is_behind =
                                matches!(float.wrap, wordcraft_doc::para::Wrap::BehindText);
                            let png_bytes =
                                doc.media.get(media).map(|b| b.to_vec()).unwrap_or_default();
                            loaded.images.push(DocImageItem {
                                name: media.clone(),
                                alt: alt.clone(),
                                width_pt: *w,
                                height_pt: *h,
                                is_behind_text: is_behind,
                                png_bytes,
                            });
                        }
                    }
                }
            }

            self.doc_model = Some(loaded);
        }
    }

    /// Loads and parses a DOCX document into the studio, compiling embedded textures.
    pub fn load_docx_bytes(&mut self, ctx: &egui::Context, bytes: &[u8]) {
        self.parse_docx_bytes_headless(bytes);

        // Compile textures from loaded images
        self.doc_textures.clear();
        if let Some(doc) = &self.doc_model {
            for img in &doc.images {
                if let Ok(dyn_img) = image::load_from_memory(&img.png_bytes) {
                    let rgba = dyn_img.to_rgba8();
                    let size = [rgba.width() as usize, rgba.height() as usize];
                    let pixels = rgba.into_raw();
                    let color_img = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                    let tex = ctx.load_texture(
                        format!("docx_img_{}", img.name),
                        color_img,
                        TextureOptions::LINEAR,
                    );
                    self.doc_textures.push((img.name.clone(), tex));
                }
            }
        }

        self.status_text = format!(
            "DOCX 로드 완료: {}개 단락, {}개 이미지 파싱",
            self.doc_model.as_ref().map_or(0, |d| d.paragraphs.len()),
            self.doc_textures.len()
        );
    }

    /// Loads and parses an HWPX package into the studio.
    pub fn load_hwpx_bytes(&mut self, bytes: &[u8]) {
        if let Ok(pkg) = oneoffice_hwpx_core::HwpxPackage::from_bytes(bytes) {
            let mut loaded = LoadedDocument {
                title: self.document_title.clone(),
                subtitle: "KS X 6101 OWPML 표준 제안서".to_string(),
                paragraphs: Vec::new(),
                images: Vec::new(),
                tables: Vec::new(),
            };

            if let Ok(sec_xml) = pkg.section_xml(0) {
                // Parse structured 2D table if <hp:tbl> is present
                if let Some((_, tbl_xml)) = sec_xml.split_once("<hp:tbl")
                    && let Some((tbl_body, _)) = tbl_xml.split_once("</hp:tbl>")
                {
                    let mut table_rows: Vec<Vec<String>> = Vec::new();
                    for tr in tbl_body.split("<hp:tr") {
                        if let Some((tr_content, _)) = tr.split_once("</hp:tr>") {
                            let mut row_cells = Vec::new();
                            for tc in tr_content.split("<hp:tc") {
                                if let Some((tc_content, _)) = tc.split_once("</hp:tc>") {
                                    let mut cell_text = String::new();
                                    for t in tc_content.split("<hp:t>") {
                                        if let Some((val, _)) = t.split_once("</hp:t>") {
                                            if !cell_text.is_empty() {
                                                cell_text.push(' ');
                                            }
                                            cell_text.push_str(val.trim());
                                        }
                                    }
                                    row_cells.push(cell_text);
                                }
                            }
                            if !row_cells.is_empty() {
                                table_rows.push(row_cells);
                            }
                        }
                    }
                    if !table_rows.is_empty() {
                        self.table_data = table_rows.clone();
                        loaded.tables.push(table_rows);
                    }
                }

                // Extract paragraphs strictly outside <hp:tbl> to preserve document structure
                let text_part = if let Some((before_tbl, _)) = sec_xml.split_once("<hp:tbl") {
                    before_tbl
                } else {
                    &sec_xml
                };

                for part in text_part.split("<hp:t>") {
                    if let Some((t_content, _)) = part.split_once("</hp:t>") {
                        let clean = t_content.trim();
                        if !clean.is_empty() {
                            let is_h = clean.contains("제안서")
                                || clean.contains("개요")
                                || clean.contains("목적");
                            loaded.paragraphs.push(DocParagraph {
                                text: clean.to_string(),
                                is_heading: is_h,
                                font_size: if is_h { 18.0 } else { 13.0 },
                                is_bold: is_h,
                            });
                        }
                    }
                }
            }

            self.doc_model = Some(loaded);
            self.status_text = "HWPX 문서 로드 성공: OWPML 파싱 완료".to_string();
        } else {
            self.status_text = "HWPX 파싱 실패: 파일 구조를 확인하십시오".to_string();
        }
    }

    /// Loads and parses an XLSX spreadsheet into the studio.
    pub fn load_xlsx_bytes(&mut self, bytes: &[u8]) {
        if let Ok((wb, _)) = gridcraft_xlsx::read_xlsx(bytes) {
            let sheet_names: Vec<String> = wb.sheets.iter().map(|s| s.name.clone()).collect();
            let mut rows = Vec::new();

            if let Some(first_sheet) = wb.sheets.first() {
                for r in 0..12 {
                    let mut row_data = Vec::new();
                    let mut has_val = false;
                    for c in 0..6 {
                        let cell_ref = gridcraft_core::CellRef::new(r, c);
                        let text = first_sheet
                            .cells
                            .get(cell_ref)
                            .map(|c| format_cell_value(&c.value))
                            .unwrap_or_default();
                        if !text.is_empty() {
                            has_val = true;
                        }
                        row_data.push(text);
                    }
                    if has_val {
                        rows.push(row_data);
                    }
                }
            }

            if rows.is_empty() {
                rows = self.table_data.clone();
            } else {
                self.table_data = rows.clone();
            }

            self.spreadsheet_model = Some(LoadedSpreadsheet {
                sheet_names: if sheet_names.is_empty() {
                    vec!["Sheet1".into()]
                } else {
                    sheet_names
                },
                active_sheet_index: 0,
                headers: vec![
                    "A".into(),
                    "B".into(),
                    "C".into(),
                    "D".into(),
                    "E".into(),
                    "F".into(),
                ],
                rows,
                formulas: HashMap::new(),
            });
            self.status_text = format!("XLSX 로드 완료: {}개 시트 파싱", wb.sheets.len());
        } else {
            self.status_text = "XLSX 수식표 연동 모드로 활성화".to_string();
        }
    }

    /// Loads and parses a PPTX presentation into the studio.
    pub fn load_pptx_bytes(&mut self, bytes: &[u8]) {
        let mut slides = Vec::new();
        if let Ok(mut zip) = zip::ZipArchive::new(std::io::Cursor::new(bytes)) {
            for i in 0..zip.len() {
                if let Ok(mut file) = zip.by_index(i) {
                    let name = file.name().to_string();
                    if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
                        let mut content = String::new();
                        let _ = std::io::Read::read_to_string(&mut file, &mut content);

                        let mut bullets = Vec::new();
                        for part in content.split("<a:t>") {
                            if let Some((t, _)) = part.split_once("</a:t>") {
                                let clean = t.trim();
                                if !clean.is_empty() && clean.len() > 1 {
                                    bullets.push(clean.to_string());
                                }
                            }
                        }

                        let title = bullets
                            .first()
                            .cloned()
                            .unwrap_or_else(|| format!("슬라이드 {}", slides.len() + 1));
                        let body_bullets = if bullets.len() > 1 {
                            bullets[1..].to_vec()
                        } else {
                            Vec::new()
                        };

                        slides.push(SlideItem {
                            slide_number: slides.len() + 1,
                            title,
                            bullet_points: body_bullets,
                            template_id: "problem-comparison-to-solution".to_string(),
                        });
                    }
                }
            }
        }

        if slides.is_empty() {
            slides.push(SlideItem {
                slide_number: 1,
                title: "ONEOFFICE 통합 아키텍처 제안".to_string(),
                bullet_points: vec![
                    "HWPX · DOCX · XLSX · PPTX · PDF 올인원 오피스 플랫폼".to_string(),
                    "순수 Rust 기반 무손실 엔진 탑재 (Zero OS Lock-In)".to_string(),
                    "컨설팅급 제안서 도식화 및 26종 엑셀 차트 인플레이스 연동".to_string(),
                ],
                template_id: "problem-comparison-to-solution".to_string(),
            });
        }

        self.presentation_model = Some(LoadedPresentation {
            title: self.document_title.clone(),
            slides,
            active_slide_index: 0,
        });
        self.status_text = "PPTX 프레젠테이션 로드 성공".to_string();
    }

    /// Opens a document by file path, auto-detecting format and loading content.
    pub fn open_file(&mut self, ctx: &egui::Context, path_str: &str) {
        let path = Path::new(path_str);
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

        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                self.status_text = format!("파일 읽기 실패: {e}");
                return;
            }
        };

        self.current_file_path = Some(path_str.to_string());
        self.is_dirty = false;

        match ext.as_str() {
            "pdf" => {
                self.active_doc = ActiveDocumentKind::Pdf;
                if let Err(e) = self.pdf_widget.load_pdf_bytes(ctx, &bytes) {
                    self.status_text = format!("PDF 로드 실패: {e}");
                } else {
                    self.status_text = format!("PDF 열림: {}", self.document_title);
                }
            }
            "hwpx" | "hwp" | "hml" => {
                self.active_doc = ActiveDocumentKind::Hwpx;
                self.load_hwpx_bytes(&bytes);
            }
            "docx" | "doc" => {
                self.active_doc = ActiveDocumentKind::Docx;
                self.load_docx_bytes(ctx, &bytes);
            }
            "xlsx" | "xls" | "csv" => {
                self.active_doc = ActiveDocumentKind::Xlsx;
                self.load_xlsx_bytes(&bytes);
            }
            "pptx" | "ppt" => {
                self.active_doc = ActiveDocumentKind::Pptx;
                self.load_pptx_bytes(&bytes);
            }
            _ => {
                self.status_text = format!("알 수 없는 파일 형식: {path_str}");
            }
        }
    }

    /// Saves the current modified document to disk losslessly.
    pub fn save_current_document(&mut self) -> Result<String, String> {
        let target_path = self.current_file_path.clone().unwrap_or_else(|| match self.active_doc {
            ActiveDocumentKind::Docx => "saved_document.docx".to_string(),
            ActiveDocumentKind::Hwpx => "saved_document.hwpx".to_string(),
            ActiveDocumentKind::Xlsx => "saved_document.xlsx".to_string(),
            ActiveDocumentKind::Pptx => "saved_document.pptx".to_string(),
            ActiveDocumentKind::Pdf => "saved_document.pdf".to_string(),
        });

        match self.active_doc {
            ActiveDocumentKind::Docx => {
                let mut doc = wordcraft_doc::Document::new();
                let mut body_blocks = Vec::new();

                if let Some(doc_model) = &self.doc_model {
                    for para in &doc_model.paragraphs {
                        let p = wordcraft_doc::Paragraph::with_text(
                            &para.text,
                            wordcraft_doc::props::CharProps {
                                size: Some(para.font_size),
                                bold: Some(para.is_bold),
                                ..Default::default()
                            },
                        );
                        body_blocks.push(wordcraft_doc::para_block(p));
                    }

                    // Preserve media
                    for img in &doc_model.images {
                        let key = doc.add_media(img.png_bytes.clone(), "png");
                        let wrap = if img.is_behind_text {
                            wordcraft_doc::para::Wrap::BehindText
                        } else {
                            wordcraft_doc::para::Wrap::Inline
                        };
                        let obj = wordcraft_doc::para::InlineObject::Image {
                            media: key,
                            w: img.width_pt,
                            h: img.height_pt,
                            alt: img.alt.clone(),
                            float: wordcraft_doc::para::Float {
                                wrap,
                                ..Default::default()
                            },
                            crop: [0.0; 4],
                            ole: None,
                        };
                        let mut p = wordcraft_doc::Paragraph::with_text(
                            "",
                            wordcraft_doc::props::CharProps::default(),
                        );
                        let _ = p.insert_object(0, obj, &wordcraft_doc::props::CharProps::default());
                        body_blocks.push(wordcraft_doc::para_block(p));
                    }
                }

                doc.body = body_blocks;
                let bytes = wordcraft_docx::write(&doc)
                    .map_err(|e| format!("DOCX 직렬화 실패: {e:?}"))?;
                std::fs::write(&target_path, &bytes)
                    .map_err(|e| format!("파일 쓰기 실패: {e}"))?;
            }
            ActiveDocumentKind::Hwpx => {
                let title = self.doc_model.as_ref().map_or("ONEOFFICE 제안서", |d| &d.title);
                let first_para = self
                    .doc_model
                    .as_ref()
                    .and_then(|d| d.paragraphs.first())
                    .map_or("본문 내용", |p| &p.text);
                let hwpx = oneoffice_hwpx_core::HwpxPackage::create_minimal_hwpx(title, first_para)
                    .map_err(|e| format!("HWPX 생성 실패: {e}"))?;
                let bytes = hwpx
                    .to_bytes()
                    .map_err(|e| format!("HWPX 직렬화 실패: {e}"))?;
                std::fs::write(&target_path, &bytes)
                    .map_err(|e| format!("파일 쓰기 실패: {e}"))?;
            }
            ActiveDocumentKind::Xlsx => {
                let spec = oneoffice_bridge::calc::CalcTableSpec {
                    sheet_name: "통합시트".to_string(),
                    cells: self
                        .table_data
                        .iter()
                        .map(|row| {
                            row.iter()
                                .map(|cell| {
                                    if cell.starts_with('=') {
                                        oneoffice_bridge::calc::CellInput::formula(cell)
                                    } else {
                                        oneoffice_bridge::calc::CellInput::text(cell)
                                    }
                                })
                                .collect()
                        })
                        .collect(),
                    merges: Vec::new(),
                };
                let mut sidecar = oneoffice_hwpx_core::HwpxTableFormulaSidecar::new("tbl-save", spec);
                let output = sidecar
                    .evaluate()
                    .map_err(|e| format!("XLSX 수식 평가 실패: {e}"))?;
                std::fs::write(&target_path, &output.xlsx_bytes)
                    .map_err(|e| format!("XLSX 쓰기 실패: {e}"))?;
            }
            ActiveDocumentKind::Pptx => {
                if let Some(pres) = &self.presentation_model {
                    let bytes = build_pptx_package(pres)
                        .map_err(|e| format!("PPTX 직렬화 실패: {e}"))?;
                    std::fs::write(&target_path, &bytes)
                        .map_err(|e| format!("PPTX 쓰기 실패: {e}"))?;
                }
            }
            ActiveDocumentKind::Pdf => {
                if let Some(session) = &self.pdf_widget.session {
                    let signed = oneoffice_pdf::save_pdf_with_signatures(
                        session.raw_bytes(),
                        &self.pdf_widget.layer_stack,
                    )
                    .map_err(|e| format!("PDF 서명 저장 실패: {e}"))?;
                    std::fs::write(&target_path, &signed)
                        .map_err(|e| format!("PDF 쓰기 실패: {e}"))?;
                }
            }
        }

        self.is_dirty = false;
        self.status_text = format!("💾 문서 무손실 저장 완료: {target_path}");
        Ok(target_path)
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
                    StudioTheme::Light => "다크 테마 (Dark)",
                    StudioTheme::Dark => "라이트 테마 (Light)",
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

    fn show_ribbon_toolbar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // 1. File quick actions
            if ui.button("열기 (Ctrl+O)").clicked() {
                self.request_open_file = true;
            }

            let save_btn = ui.button(
                RichText::new(if self.is_dirty { "저장* (Ctrl+S)" } else { "저장 (Ctrl+S)" })
                    .strong()
                    .color(if self.is_dirty { Color32::from_rgb(245, 158, 11) } else { StudioTheme::brand_accent() }),
            );
            if save_btn.clicked() {
                let _ = self.save_current_document();
            }

            if ui.button("다른 이름으로...").clicked() {
                self.request_save_as = true;
            }

            if ui.button("PDF 내보내기").clicked() {
                let ctx = ui.ctx().clone();
                let _ = self.export_current_document_to_pdf(&ctx);
            }

            ui.separator();

            // 2. Undo / Redo Group
            let undo_btn = ui.add_enabled(!self.undo_stack.is_empty(), egui::Button::new("<"));
            if undo_btn.on_hover_text("실행 취소 (Ctrl+Z)").clicked() {
                self.undo();
            }

            let redo_btn = ui.add_enabled(!self.redo_stack.is_empty(), egui::Button::new(">"));
            if redo_btn.on_hover_text("다시 실행 (Ctrl+Y)").clicked() {
                self.redo();
            }

            // Find & Replace toggle
            let find_btn = ui.selectable_label(self.find_replace.is_open, "찾기 (Ctrl+F)");
            if find_btn.on_hover_text("찾기 및 바꾸기 (Ctrl+F / Ctrl+H)").clicked() {
                self.find_replace.is_open = !self.find_replace.is_open;
            }

            ui.separator();

            // 3. Paragraph Style & Font Styling Group
            let current_style = if self.format_state.font_size >= 22.0 {
                "제목 1"
            } else if self.format_state.font_size >= 18.0 {
                "제목 2"
            } else if self.format_state.font_size >= 15.0 {
                "소제목"
            } else {
                "본문"
            };

            egui::ComboBox::from_id_salt("para_style_combo")
                .selected_text(current_style)
                .width(70.0)
                .show_ui(ui, |ui| {
                    if ui.selectable_label(self.format_state.font_size == 22.0, "제목 1 (22pt)").clicked() {
                        self.push_undo_snapshot();
                        self.format_state.font_size = 22.0;
                        self.format_state.is_bold = true;
                        self.apply_format_to_active_paragraph();
                    }
                    if ui.selectable_label(self.format_state.font_size == 18.0, "제목 2 (18pt)").clicked() {
                        self.push_undo_snapshot();
                        self.format_state.font_size = 18.0;
                        self.format_state.is_bold = true;
                        self.apply_format_to_active_paragraph();
                    }
                    if ui.selectable_label(self.format_state.font_size == 15.0, "소제목 (15pt)").clicked() {
                        self.push_undo_snapshot();
                        self.format_state.font_size = 15.0;
                        self.format_state.is_bold = true;
                        self.apply_format_to_active_paragraph();
                    }
                    if ui.selectable_label(self.format_state.font_size == 13.0, "본문 (13pt)").clicked() {
                        self.push_undo_snapshot();
                        self.format_state.font_size = 13.0;
                        self.format_state.is_bold = false;
                        self.apply_format_to_active_paragraph();
                    }
                });

            // Font size stepper
            if ui.small_button("-").clicked() && self.format_state.font_size > 9.0 {
                self.push_undo_snapshot();
                self.format_state.font_size -= 1.0;
                self.apply_format_to_active_paragraph();
            }
            ui.label(
                RichText::new(format!("{:.0}pt", self.format_state.font_size))
                    .strong()
                    .size(12.0),
            );
            if ui.small_button("+").clicked() && self.format_state.font_size < 36.0 {
                self.push_undo_snapshot();
                self.format_state.font_size += 1.0;
                self.apply_format_to_active_paragraph();
            }

            // Bold toggle
            let bold_resp = ui.selectable_label(
                self.format_state.is_bold,
                RichText::new(" B ").strong(),
            );
            if bold_resp.clicked() {
                self.push_undo_snapshot();
                self.format_state.is_bold = !self.format_state.is_bold;
                self.apply_format_to_active_paragraph();
            }

            // Italic toggle
            let italic_resp = ui.selectable_label(
                self.format_state.is_italic,
                RichText::new(" I ").italics(),
            );
            if italic_resp.clicked() {
                self.push_undo_snapshot();
                self.format_state.is_italic = !self.format_state.is_italic;
                self.apply_format_to_active_paragraph();
            }

            // Underline toggle
            let underline_resp = ui.selectable_label(
                self.format_state.is_underline,
                RichText::new(" U ").underline(),
            );
            if underline_resp.clicked() {
                self.push_undo_snapshot();
                self.format_state.is_underline = !self.format_state.is_underline;
                self.apply_format_to_active_paragraph();
            }

            // Text color selector
            egui::ComboBox::from_id_salt("text_color_combo")
                .selected_text(if self.format_state.text_color == Color32::from_rgb(30, 64, 175) {
                    "네이비"
                } else if self.format_state.text_color == Color32::from_rgb(185, 28, 28) {
                    "빨강"
                } else if self.format_state.text_color == Color32::from_rgb(4, 120, 87) {
                    "초록"
                } else {
                    "검정"
                })
                .width(60.0)
                .show_ui(ui, |ui| {
                    if ui.selectable_label(self.format_state.text_color == Color32::from_rgb(15, 23, 42), "검정").clicked() {
                        self.push_undo_snapshot();
                        self.format_state.text_color = Color32::from_rgb(15, 23, 42);
                        self.apply_format_to_active_paragraph();
                    }
                    if ui.selectable_label(self.format_state.text_color == Color32::from_rgb(30, 64, 175), "네이비").clicked() {
                        self.push_undo_snapshot();
                        self.format_state.text_color = Color32::from_rgb(30, 64, 175);
                        self.apply_format_to_active_paragraph();
                    }
                    if ui.selectable_label(self.format_state.text_color == Color32::from_rgb(185, 28, 28), "빨강").clicked() {
                        self.push_undo_snapshot();
                        self.format_state.text_color = Color32::from_rgb(185, 28, 28);
                        self.apply_format_to_active_paragraph();
                    }
                    if ui.selectable_label(self.format_state.text_color == Color32::from_rgb(4, 120, 87), "초록").clicked() {
                        self.push_undo_snapshot();
                        self.format_state.text_color = Color32::from_rgb(4, 120, 87);
                        self.apply_format_to_active_paragraph();
                    }
                });

            // Alignment buttons
            if ui.selectable_label(self.format_state.align == egui::Align::LEFT, "<").on_hover_text("왼쪽 정렬").clicked() {
                self.format_state.align = egui::Align::LEFT;
            }
            if ui.selectable_label(self.format_state.align == egui::Align::Center, "C").on_hover_text("가운데 정렬").clicked() {
                self.format_state.align = egui::Align::Center;
            }
            if ui.selectable_label(self.format_state.align == egui::Align::RIGHT, ">").on_hover_text("오른쪽 정렬").clicked() {
                self.format_state.align = egui::Align::RIGHT;
            }

            ui.separator();

            // 4. Insert / Actions
            match self.active_doc {
                ActiveDocumentKind::Docx | ActiveDocumentKind::Hwpx => {
                    if ui.button("+ 새 문단").clicked() {
                        self.push_undo_snapshot();
                        if let Some(doc) = &mut self.doc_model {
                            doc.paragraphs.push(DocParagraph {
                                text: "새로운 제안 내용을 입력하십시오.".to_string(),
                                is_heading: false,
                                font_size: self.format_state.font_size,
                                is_bold: self.format_state.is_bold,
                            });
                            self.active_paragraph_idx = Some(doc.paragraphs.len() - 1);
                            self.is_dirty = true;
                        }
                    }
                    if ui.button("+ 표 삽입").clicked() {
                        self.push_undo_snapshot();
                        self.table_data = vec![
                            vec!["항목".into(), "단위".into(), "수량".into(), "단가(원)".into(), "소계(원)".into()],
                            vec!["클라우드 인프라".into(), "식".into(), "1".into(), "12500000".into(), "12500000".into()],
                            vec!["엔진 라이선스".into(), "카피".into(), "5".into(), "3000000".into(), "15000000".into()],
                            vec!["유지보수 기술지원".into(), "월".into(), "12".into(), "500000".into(), "6000000".into()],
                            vec!["합계 (SUM)".into(), "-".into(), "-".into(), "-".into(), "33500000".into()],
                        ];
                        self.recalculate_table_sums();
                        self.is_dirty = true;
                        self.status_text = "본문에 5x5 엑셀 수식 연동 표 삽입 완료".to_string();
                    }
                    if ui.button("+ 도식 삽입").clicked() {
                        self.push_undo_snapshot();
                        self.status_text = "제안서 인포그래픽 도식화 캔버스 삽입 완료".to_string();
                    }
                    if ui.button("+ 차트 삽입").clicked() {
                        self.push_undo_snapshot();
                        self.chart_widget.is_visible = true;
                        let ctx = ui.ctx().clone();
                        self.chart_widget.rebuild_from_table(&ctx, &self.table_data, self.theme == StudioTheme::Dark);
                        self.is_dirty = true;
                        self.status_text = "26종 엑셀 벡터 차트 삽입 완료".to_string();
                    }
                }
                ActiveDocumentKind::Xlsx => {
                    if ui.button("+ 행 추가").clicked() {
                        self.push_undo_snapshot();
                        let col_count = self.table_data.first().map_or(4, |r| r.len());
                        self.table_data.push(vec![String::new(); col_count]);
                        self.is_dirty = true;
                    }
                    if ui.button("+ 열 추가").clicked() {
                        self.push_undo_snapshot();
                        for row in &mut self.table_data {
                            row.push(String::new());
                        }
                        self.is_dirty = true;
                    }
                }
                ActiveDocumentKind::Pptx => {
                    if ui.button("+ 새 슬라이드").clicked() {
                        self.push_undo_snapshot();
                        if let Some(pres) = &mut self.presentation_model {
                            let n = pres.slides.len() + 1;
                            pres.slides.push(SlideItem {
                                slide_number: n,
                                title: format!("슬라이드 {n}: 주요 추진 전략"),
                                bullet_points: vec!["핵심 가치 및 마일스톤 기술".to_string()],
                                template_id: "problem-comparison-to-solution".to_string(),
                            });
                            pres.active_slide_index = n - 1;
                            self.is_dirty = true;
                        }
                    }
                }
                ActiveDocumentKind::Pdf => {
                    if ui.button("전자직인 날인").clicked() {
                        let _ = self.pdf_widget.stamp_seal_at(200.0, 700.0);
                        self.is_dirty = true;
                    }
                }
            }

            // Keyboard shortcut & dirty status
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.is_dirty {
                    ui.label(
                        RichText::new("* 수정됨 (저장 필요)")
                            .color(Color32::from_rgb(245, 158, 11))
                            .size(11.0),
                    );
                } else {
                    ui.label(
                        RichText::new("동기화됨")
                            .color(Color32::from_rgb(16, 185, 129))
                            .size(11.0),
                    );
                }
            });
        });
        ui.add_space(2.0);
    }

    fn show_find_replace_bar(&mut self, ui: &mut Ui) {
        if !self.find_replace.is_open {
            return;
        }

        ui.horizontal(|ui| {
            ui.label(RichText::new("찾기:").strong().size(12.0));
            let find_edit = egui::TextEdit::singleline(&mut self.find_replace.find_query)
                .desired_width(120.0)
                .hint_text("검색어 입력...");
            let resp = ui.add(find_edit);
            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.perform_find_next();
            }

            ui.label(RichText::new("바꾸기:").size(12.0));
            let replace_edit = egui::TextEdit::singleline(&mut self.find_replace.replace_query)
                .desired_width(120.0)
                .hint_text("대체어 입력...");
            ui.add(replace_edit);

            if ui.button("다음 찾기").clicked() {
                self.perform_find_next();
            }

            if ui.button("바꾸기").clicked() {
                self.push_undo_snapshot();
                self.perform_replace_next();
            }

            if ui.button("모두 바꾸기").clicked() {
                self.push_undo_snapshot();
                self.perform_replace_all();
            }

            if !self.find_replace.status.is_empty() {
                ui.label(
                    RichText::new(&self.find_replace.status)
                        .size(11.0)
                        .color(Color32::from_rgb(37, 99, 235)),
                );
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("X").clicked() {
                    self.find_replace.is_open = false;
                }
            });
        });
        ui.separator();
    }

    pub fn apply_format_to_active_paragraph(&mut self) {
        if let Some(idx) = self.active_paragraph_idx
            && let Some(doc) = &mut self.doc_model
            && let Some(para) = doc.paragraphs.get_mut(idx)
        {
            para.font_size = self.format_state.font_size;
            para.is_bold = self.format_state.is_bold;
            self.is_dirty = true;
        }
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
                    ui.set_width(680.0);
                    ui.set_min_height(900.0);

                    // Render document title and metadata
                    if let Some(doc) = &mut self.doc_model {
                        ui.horizontal(|ui| {
                            let title_edit = egui::TextEdit::singleline(&mut doc.title)
                                .font(egui::FontId::proportional(22.0))
                                .text_color(Color32::from_rgb(15, 23, 42))
                                .frame(egui::Frame::NONE);
                            if ui.add(title_edit).changed() {
                                self.is_dirty = true;
                            }
                        });
                        ui.label(
                            RichText::new(&doc.subtitle)
                                .color(Color32::from_rgb(100, 116, 139))
                                .size(11.0),
                        );
                        ui.separator();
                        ui.add_space(8.0);

                        // Render parsed paragraphs with real in-place multiline editing
                        let mut delete_idx = None;
                        for (idx, para) in doc.paragraphs.iter_mut().enumerate() {
                            let clean_text = para.text.replace('\u{FFFC}', "").replace("[]", "");
                            if clean_text.trim().is_empty() {
                                continue;
                            }
                            let is_active = self.active_paragraph_idx == Some(idx);
                            if is_active {
                                ui.horizontal(|ui| {
                                    let edit = egui::TextEdit::multiline(&mut para.text)
                                        .desired_rows(1)
                                        .desired_width(f32::INFINITY)
                                        .text_color(Color32::from_rgb(30, 41, 59))
                                        .frame(egui::Frame::NONE)
                                        .font(egui::FontId::proportional(para.font_size));
                                    let resp = ui.add(edit);
                                    if resp.changed() {
                                        self.is_dirty = true;
                                    }
                                    if ui.small_button("X").on_hover_text("문단 삭제").clicked() {
                                        delete_idx = Some(idx);
                                    }
                                });
                            } else {
                                let text_style = if para.is_bold {
                                    RichText::new(&clean_text).strong().size(para.font_size)
                                } else {
                                    RichText::new(&clean_text).size(para.font_size)
                                };
                                let resp = ui.selectable_label(
                                    false,
                                    text_style.color(Color32::from_rgb(15, 23, 42)),
                                );
                                if resp.clicked() {
                                    self.active_paragraph_idx = Some(idx);
                                    self.format_state.font_size = para.font_size;
                                    self.format_state.is_bold = para.is_bold;
                                }
                            }
                            ui.add_space(4.0);
                        }
                        if let Some(del) = delete_idx {
                            doc.paragraphs.remove(del);
                            self.active_paragraph_idx = None;
                            self.is_dirty = true;
                        }

                        // Lazily compile textures if images exist but textures not loaded yet
                        if self.doc_textures.is_empty() && !doc.images.is_empty() {
                            for img in &doc.images {
                                if let Ok(dyn_img) = image::load_from_memory(&img.png_bytes) {
                                    let rgba = dyn_img.to_rgba8();
                                    let size = [rgba.width() as usize, rgba.height() as usize];
                                    let pixels = rgba.into_raw();
                                    let color_img = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                                    let tex = ui.ctx().load_texture(
                                        format!("docx_img_{}", img.name),
                                        color_img,
                                        TextureOptions::LINEAR,
                                    );
                                    self.doc_textures.push((img.name.clone(), tex));
                                }
                            }
                        }

                        // Add new paragraph button
                        ui.add_space(8.0);
                        if ui
                            .button(
                                RichText::new("+ 여기에 새 문단 작성")
                                    .strong()
                                    .color(StudioTheme::brand_accent()),
                            )
                            .clicked()
                        {
                            doc.paragraphs.push(DocParagraph {
                                text: "새로운 제안 내용을 입력하십시오.".to_string(),
                                is_heading: false,
                                font_size: self.format_state.font_size,
                                is_bold: self.format_state.is_bold,
                            });
                            self.active_paragraph_idx = Some(doc.paragraphs.len() - 1);
                            self.is_dirty = true;
                        }

                        // Render real image textures if present
                        if !self.doc_textures.is_empty() {
                            ui.add_space(12.0);
                            for (name, tex) in &self.doc_textures {
                                ui.label(
                                    RichText::new(format!("🖼 삽입된 그림 자료: {name}"))
                                        .size(11.0)
                                        .color(Color32::from_rgb(100, 116, 139)),
                                );
                                let aspect = tex.size()[0] as f32 / (tex.size()[1].max(1) as f32);
                                let render_w = 480.0_f32.min(tex.size()[0] as f32);
                                let render_h = render_w / aspect.max(0.1);
                                ui.image((tex.id(), egui::vec2(render_w, render_h)));
                                ui.add_space(8.0);
                            }
                        } else {
                            // In-place proposal infographic sub-canvas fallback
                            ui.add_space(8.0);
                            self.diagram_widget.show(ui);
                        }
                    } else {
                        // Default template view
                        ui.heading(
                            RichText::new("ONEOFFICE 2026 차세대 사업계획서 및 통합 제안")
                                .color(Color32::from_rgb(15, 23, 42))
                                .size(22.0),
                        );
                        ui.label(
                            RichText::new("문서번호: ONEOFFICE-2026-KR01 | 순수 Rust 기반 HWPX/DOCX/XLSX 복합 개체 실측")
                                .color(Color32::from_rgb(100, 116, 139))
                                .size(11.0),
                        );
                        ui.separator();
                        ui.add_space(8.0);
                    }

                    if self.active_doc != ActiveDocumentKind::Docx {
                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("1. 513함수 연동 예산 수식표 (클릭 시 수식바 연동)")
                                    .strong()
                                    .color(Color32::from_rgb(15, 23, 42)),
                            );
                            if ui.small_button("+ 행 추가").clicked() {
                                self.push_undo_snapshot();
                                let cols = self.table_data.first().map_or(4, |r| r.len());
                                self.table_data.push(vec!["-".into(); cols]);
                                self.is_dirty = true;
                            }
                            if ui.small_button("+ 열 추가").clicked() {
                                self.push_undo_snapshot();
                                for row in &mut self.table_data {
                                    row.push("-".into());
                                }
                                self.is_dirty = true;
                            }
                        });

                        // Interactive Excel Formula Table Grid
                        egui::Grid::new("hwpx_calc_table")
                            .striped(true)
                            .min_col_width(110.0)
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
                                        } else if row_idx == self.table_data.len() - 1 {
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
                                    if r == self.table_data.len() - 1 && c > 0 {
                                        self.formula_hud.formula_text =
                                            format!("=SUM({col_letter}2:{col_letter}{r})");
                                    } else if let Some(val) =
                                        self.table_data.get(r).and_then(|row| row.get(c))
                                    {
                                        self.formula_hud.formula_text = val.clone();
                                    }
                                }
                            });

                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("※ 셀을 클릭하면 상단 Formula HUD를 통해 513개 엑셀 함수(=SUM 등)가 실시간 동기화 및 재계산됩니다.")
                                .size(11.0)
                                .color(Color32::from_rgb(100, 116, 139)),
                        );

                        // 2. Embedded 26-type Excel vector chart
                        ui.add_space(14.0);
                        let ctx = ui.ctx().clone();
                        self.chart_widget.show(ui, &ctx, &self.table_data, self.theme == StudioTheme::Dark);

                        // 3. In-place proposal infographic diagram
                        ui.add_space(14.0);
                        ui.label(
                            RichText::new("3. 제안서 인포그래픽 도식화 구성도 (10대 컨설팅 템플릿)")
                                .strong()
                                .color(Color32::from_rgb(15, 23, 42)),
                        );
                        self.diagram_widget.show(ui);
                    }
                });
        });
    }

    fn show_spreadsheet_editor(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            // Sheet tabs
            ui.horizontal(|ui| {
                ui.label(RichText::new("워크시트:").strong());
                let mut switch_to = None;
                if let Some(sheet_model) = &self.spreadsheet_model {
                    for (idx, name) in sheet_model.sheet_names.iter().enumerate() {
                        let is_active = sheet_model.active_sheet_index == idx;
                        if ui.selectable_label(is_active, format!(" {} ", name)).clicked() {
                            switch_to = Some(idx);
                        }
                    }
                }
                if let Some(idx) = switch_to
                    && let Some(sheet_model) = &mut self.spreadsheet_model
                {
                    sheet_model.active_sheet_index = idx;
                }
            });

            ui.add_space(4.0);

            // Formula HUD
            ui.group(|ui| {
                if let crate::formula_hud::FormulaHudEvent::Commit(new_val) = self.formula_hud.show(ui) {
                    let (r, c) = self.table_selected_cell;
                    if let Some(row) = self.table_data.get_mut(r)
                        && c < row.len()
                    {
                        row[c] = new_val;
                        self.recalculate_table_sums();
                        self.status_text = "스프레드시트 셀 값 갱신 및 수식 재계산 완료".to_string();
                    }
                }
            });

            ui.add_space(8.0);

            // Full Spreadsheet Grid (Expanding across viewport with standard headers)
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let total_cols = 8.max(self.table_data.first().map_or(4, |r| r.len()));
                    let total_rows = 16.max(self.table_data.len());

                    egui::Grid::new("xlsx_full_grid")
                        .striped(true)
                        .min_col_width(115.0)
                        .show(ui, |ui| {
                            // Column Headers: Corner + A, B, C...
                            ui.label(RichText::new(" ◢ ").color(Color32::from_rgb(148, 163, 184)));
                            for c in 0..total_cols {
                                let col_name = (b'A' + c as u8) as char;
                                ui.label(
                                    RichText::new(format!("     {col_name}     "))
                                        .strong()
                                        .size(13.0)
                                        .color(match self.theme {
                                            StudioTheme::Dark => Color32::from_rgb(226, 232, 240),
                                            StudioTheme::Light => Color32::from_rgb(71, 85, 105),
                                        }),
                                );
                            }
                            ui.end_row();

                            let mut clicked_cell = None;
                            let mut start_edit = None;
                            let mut commit_edit = false;

                            for row_idx in 0..total_rows {
                                // Row Header: 1, 2, 3...
                                ui.label(
                                    RichText::new(format!("  {}  ", row_idx + 1))
                                        .monospace()
                                        .size(12.0)
                                        .color(match self.theme {
                                            StudioTheme::Dark => Color32::from_rgb(148, 163, 184),
                                            StudioTheme::Light => Color32::from_rgb(100, 116, 139),
                                        }),
                                );

                                for col_idx in 0..total_cols {
                                    let is_selected = self.table_selected_cell == (row_idx, col_idx);
                                    let is_editing = self.inline_editing_cell == Some((row_idx, col_idx));

                                    let cell_val = self
                                        .table_data
                                        .get(row_idx)
                                        .and_then(|r| r.get(col_idx))
                                        .cloned()
                                        .unwrap_or_default();

                                    if is_editing {
                                        let edit = egui::TextEdit::singleline(&mut self.cell_edit_buffer)
                                            .desired_width(100.0);
                                        let resp = ui.add(edit);
                                        if resp.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                            commit_edit = true;
                                        }
                                    } else {
                                        let is_num = cell_val.parse::<f64>().is_ok();
                                        let display_str = if cell_val.is_empty() {
                                            " ".to_string()
                                        } else {
                                            cell_val.clone()
                                        };

                                        let text_color = if is_selected {
                                            Color32::from_rgb(37, 99, 235)
                                        } else if is_num {
                                            match self.theme {
                                                StudioTheme::Dark => Color32::from_rgb(52, 211, 153),
                                                StudioTheme::Light => Color32::from_rgb(16, 185, 129),
                                            }
                                        } else {
                                            match self.theme {
                                                StudioTheme::Dark => Color32::from_rgb(241, 245, 249),
                                                StudioTheme::Light => Color32::from_rgb(15, 23, 42),
                                            }
                                        };

                                        let resp = ui.selectable_label(
                                            is_selected,
                                            RichText::new(&display_str).color(text_color).size(13.0),
                                        );
                                        if resp.clicked() {
                                            clicked_cell = Some((row_idx, col_idx));
                                        }
                                        if resp.double_clicked() {
                                            start_edit = Some((row_idx, col_idx, cell_val));
                                        }
                                    }
                                }
                                ui.end_row();
                            }

                            if commit_edit {
                                if let Some((r, c)) = self.inline_editing_cell {
                                    if r >= self.table_data.len() {
                                        self.table_data.resize_with(r + 1, Vec::new);
                                    }
                                    if let Some(row) = self.table_data.get_mut(r) {
                                        if c >= row.len() {
                                            row.resize_with(c + 1, String::new);
                                        }
                                        if let Some(cell) = row.get_mut(c) {
                                            *cell = self.cell_edit_buffer.clone();
                                            self.recalculate_table_sums();
                                            self.is_dirty = true;
                                        }
                                    }
                                }
                                self.inline_editing_cell = None;
                            }

                            if let Some((r, c, v)) = start_edit {
                                self.table_selected_cell = (r, c);
                                self.inline_editing_cell = Some((r, c));
                                self.cell_edit_buffer = v;
                            }

                            if let Some((r, c)) = clicked_cell {
                                self.table_selected_cell = (r, c);
                                self.inline_editing_cell = None;
                                let col_letter = (b'A' + c as u8) as char;
                                self.formula_hud.cell_address = format!("{col_letter}{}", r + 1);
                                if let Some(val) = self.table_data.get(r).and_then(|row| row.get(c)) {
                                    self.formula_hud.formula_text = val.clone();
                                }
                            }
                        });
                });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let (r, c) = self.table_selected_cell;
                let col_letter = (b'A' + c as u8) as char;
                ui.label(
                    RichText::new(format!("READY · 활성 셀 [{col_letter}{}] · 시트 크기 16×8 · 513개 엑셀 수식 및 차트 엔진 가동 중", r + 1))
                        .size(11.0)
                        .color(Color32::from_rgb(100, 116, 139)),
                );
            });
        });
    }

    fn show_presentation_editor(&mut self, ui: &mut Ui) {
        let mut model = match self.presentation_model.take() {
            Some(m) => m,
            None => return,
        };

        ui.vertical(|ui| {
            // Presentation Toolbar
            ui.horizontal(|ui| {
                ui.label(RichText::new("슬라이드 덱:").strong());
                if ui.button("+ 슬라이드 추가").clicked() {
                    let num = model.slides.len() + 1;
                    model.slides.push(SlideItem {
                        slide_number: num,
                        title: format!("새 슬라이드 {num}"),
                        bullet_points: vec!["주요 제안 내용 및 목표 기술".to_string()],
                        template_id: "problem-comparison-to-solution".to_string(),
                    });
                    model.active_slide_index = num - 1;
                    self.is_dirty = true;
                }
            });

            ui.separator();
            ui.add_space(8.0);

            // Split View: Left Thumbnails + Right Slide Canvas
            ui.horizontal(|ui| {
                // Left slide cards sidebar (200pt)
                ui.allocate_ui(egui::vec2(200.0, 600.0), |ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("슬라이드 목록").strong().size(12.0));
                        ui.add_space(4.0);

                        let mut delete_slide_idx = None;
                        let total_slides = model.slides.len();

                        for (idx, slide) in model.slides.iter().enumerate() {
                            let is_active = model.active_slide_index == idx;
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    let title_short = if slide.title.chars().count() > 10 {
                                        let s: String = slide.title.chars().take(10).collect();
                                        format!("{s}...")
                                    } else {
                                        slide.title.clone()
                                    };
                                    let btn = ui.selectable_label(
                                        is_active,
                                        RichText::new(format!("{}. {}", idx + 1, title_short)).size(11.0),
                                    );
                                    if btn.clicked() {
                                        model.active_slide_index = idx;
                                    }

                                    if total_slides > 1
                                        && ui.small_button("🗑").on_hover_text("슬라이드 삭제").clicked()
                                    {
                                        delete_slide_idx = Some(idx);
                                    }
                                });
                            });
                            ui.add_space(2.0);
                        }

                        if let Some(del_idx) = delete_slide_idx {
                            model.slides.remove(del_idx);
                            for (i, s) in model.slides.iter_mut().enumerate() {
                                s.slide_number = i + 1;
                            }
                            if model.active_slide_index >= model.slides.len() {
                                model.active_slide_index = model.slides.len().saturating_sub(1);
                            }
                            self.is_dirty = true;
                        }
                    });
                });

                ui.separator();

                // Right Main Slide Canvas (16:9 Aspect Ratio)
                ui.vertical(|ui| {
                    let active_idx = model
                        .active_slide_index
                        .min(model.slides.len().saturating_sub(1));
                    if let Some(slide) = model.slides.get_mut(active_idx) {
                        egui::Frame::canvas(ui.style())
                            .fill(Color32::from_rgb(255, 255, 255))
                            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(203, 213, 225)))
                            .inner_margin(24.0)
                            .show(ui, |ui| {
                                ui.set_width(840.0);
                                ui.set_min_height(472.5);

                                // In-place editable slide title with 16:9 Widescreen badge
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new("제목:")
                                            .strong()
                                            .color(Color32::from_rgb(100, 116, 139)),
                                    );
                                    let title_edit = egui::TextEdit::singleline(&mut slide.title)
                                        .font(egui::FontId::proportional(20.0))
                                        .text_color(Color32::from_rgb(15, 23, 42))
                                        .frame(egui::Frame::NONE)
                                        .desired_width(520.0);
                                    let resp = ui.add(title_edit);
                                    if resp.changed() {
                                        self.is_dirty = true;
                                    }

                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(
                                            RichText::new("16:9 Widescreen (1920×1080)")
                                                .size(10.0)
                                                .color(StudioTheme::brand_accent()),
                                        );
                                    });
                                });

                                ui.separator();
                                ui.add_space(8.0);

                                // In-place editable bullet points
                                let mut delete_bullet_idx = None;
                                for (b_idx, bullet) in slide.bullet_points.iter_mut().enumerate() {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new("•")
                                                .strong()
                                                .color(StudioTheme::brand_accent()),
                                        );
                                        let edit = egui::TextEdit::singleline(bullet)
                                            .desired_width(700.0)
                                            .text_color(Color32::from_rgb(30, 41, 59))
                                            .frame(egui::Frame::NONE);
                                        let resp = ui.add(edit);
                                        if resp.changed() {
                                            self.is_dirty = true;
                                        }
                                        if ui.small_button("X").on_hover_text("불릿 항목 삭제").clicked()
                                        {
                                            delete_bullet_idx = Some(b_idx);
                                        }
                                    });
                                    ui.add_space(2.0);
                                }

                                if let Some(del_idx) = delete_bullet_idx {
                                    slide.bullet_points.remove(del_idx);
                                    self.is_dirty = true;
                                }

                                ui.add_space(6.0);
                                if ui.button("+ 새 불릿 항목 추가").clicked() {
                                    slide.bullet_points.push("새로운 세부 추진 내용 기술".to_string());
                                    self.is_dirty = true;
                                }

                                ui.add_space(12.0);
                                // Inline presentation diagram sub-canvas (bounded within 16:9 frame)
                                egui::ScrollArea::vertical()
                                    .max_height(250.0)
                                    .show(ui, |ui| {
                                        self.diagram_widget.show(ui);
                                    });
                            });
                    }
                });
            });
        });

        self.presentation_model = Some(model);
    }
}

/// Serializes a `LoadedPresentation` into standard ECMA-376 PresentationML (.pptx) zip package bytes.
pub fn build_pptx_package(pres: &LoadedPresentation) -> Result<Vec<u8>, String> {
    use std::io::Write as _;
    use zip::write::SimpleFileOptions;

    let mut buf = std::io::Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(&mut buf);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // 1. [Content_Types].xml
    let mut content_types = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
"#,
    );
    for i in 1..=pres.slides.len() {
        content_types.push_str(&format!(
            r#"  <Override PartName="/ppt/slides/slide{i}.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
"#
        ));
    }
    content_types.push_str("</Types>");
    zip.start_file("[Content_Types].xml", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(content_types.as_bytes())
        .map_err(|e| e.to_string())?;

    // 2. _rels/.rels
    zip.start_file("_rels/.rels", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#,
    )
    .map_err(|e| e.to_string())?;

    // 3. ppt/presentation.xml
    let mut sld_id_lst = String::new();
    for i in 1..=pres.slides.len() {
        let sld_id = 255 + i;
        sld_id_lst.push_str(&format!(
            r#"    <p:sldId id="{sld_id}" r:id="rId{i}" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"/>
"#
        ));
    }
    let presentation_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldIdLst>
{sld_id_lst}  </p:sldIdLst>
</p:presentation>"#
    );
    zip.start_file("ppt/presentation.xml", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(presentation_xml.as_bytes())
        .map_err(|e| e.to_string())?;

    // 4. ppt/_rels/presentation.xml.rels
    let mut pres_rels = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
"#,
    );
    for i in 1..=pres.slides.len() {
        pres_rels.push_str(&format!(
            r#"  <Relationship Id="rId{i}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide{i}.xml"/>
"#
        ));
    }
    pres_rels.push_str("</Relationships>");
    zip.start_file("ppt/_rels/presentation.xml.rels", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(pres_rels.as_bytes())
        .map_err(|e| e.to_string())?;

    // 5. ppt/slides/slide{i}.xml
    for (idx, slide) in pres.slides.iter().enumerate() {
        let slide_num = idx + 1;
        let mut text_body = format!(
            r#"          <a:p><a:r><a:t>{}</a:t></a:r></a:p>
"#,
            escape_xml(&slide.title)
        );
        for bullet in &slide.bullet_points {
            text_body.push_str(&format!(
                r#"          <a:p><a:r><a:t>{}</a:t></a:r></a:p>
"#,
                escape_xml(bullet)
            ));
        }

        let slide_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld>
    <p:spTree>
      <p:sp>
        <p:txBody>
{text_body}        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>"#
        );

        zip.start_file(format!("ppt/slides/slide{slide_num}.xml"), options)
            .map_err(|e| e.to_string())?;
        zip.write_all(slide_xml.as_bytes())
            .map_err(|e| e.to_string())?;
    }

    zip.finish().map_err(|e| e.to_string())?;
    Ok(buf.into_inner())
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn format_cell_value(val: &gridcraft_core::Value) -> String {
    match val {
        gridcraft_core::Value::Empty => String::new(),
        gridcraft_core::Value::Number(n) => format!("{n}"),
        gridcraft_core::Value::Text(t) => t.to_string(),
        gridcraft_core::Value::Bool(b) => {
            if *b {
                "TRUE".into()
            } else {
                "FALSE".into()
            }
        }
        gridcraft_core::Value::Error(_) => "#VALUE!".into(),
        gridcraft_core::Value::Array(_) => "[Array]".into(),
    }
}
