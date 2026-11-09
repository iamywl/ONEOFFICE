//! Floating Excel formula bar and function autocompletion HUD for in-place tables.

use egui::{Color32, RichText, Ui};
use oneoffice_bridge::calc::CalcTableEngine;

/// Event emitted by user actions on the formula HUD.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormulaHudEvent {
    None,
    Commit(String),
    Cancel,
}

/// In-place floating formula HUD widget.
#[derive(Clone, Debug)]
pub struct FormulaHudWidget {
    /// Active cell coordinate in `A1` notation (e.g. `"B4"`).
    pub cell_address: String,
    /// Raw cell text or formula (e.g. `"=SUM(B2:B3)"`).
    pub formula_text: String,
    /// Whether the autocompletion dropdown is visible.
    pub show_suggestions: bool,
    /// Cached supported function names for fast prefix filtering.
    functions: Vec<&'static str>,
}

impl Default for FormulaHudWidget {
    fn default() -> Self {
        Self {
            cell_address: "A1".to_string(),
            formula_text: String::new(),
            show_suggestions: false,
            functions: CalcTableEngine::supported_function_names(),
        }
    }
}

impl FormulaHudWidget {
    /// Constructs a new formula HUD for the given cell address and initial formula text.
    #[must_use]
    pub fn new(cell_address: impl Into<String>, initial_text: impl Into<String>) -> Self {
        Self {
            cell_address: cell_address.into(),
            formula_text: initial_text.into(),
            show_suggestions: false,
            functions: CalcTableEngine::supported_function_names(),
        }
    }

    /// Renders the formula bar HUD into the provided UI and returns any user commit/cancel event.
    pub fn show(&mut self, ui: &mut Ui) -> FormulaHudEvent {
        let mut event = FormulaHudEvent::None;

        ui.horizontal(|ui| {
            // Formula indicator glyph [fx]
            ui.label(
                RichText::new("fx")
                    .italics()
                    .strong()
                    .color(Color32::from_rgb(37, 99, 235)),
            );

            // Active cell address badge
            ui.label(
                RichText::new(&self.cell_address)
                    .monospace()
                    .strong(),
            );
            ui.separator();

            // Formula input field
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.formula_text)
                    .desired_width(320.0)
                    .hint_text("수식(=SUM) 또는 텍스트 입력..."),
            );

            if response.changed() {
                self.show_suggestions = self.formula_text.starts_with('=');
            }

            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                event = FormulaHudEvent::Commit(self.formula_text.clone());
                self.show_suggestions = false;
            }

            // Quick action buttons
            if ui.button("✔").on_hover_text("수식 적용 (Enter)").clicked() {
                event = FormulaHudEvent::Commit(self.formula_text.clone());
                self.show_suggestions = false;
            }

            if ui.button("✖").on_hover_text("취소 (Esc)").clicked() {
                event = FormulaHudEvent::Cancel;
                self.show_suggestions = false;
            }
        });

        // Autocompletion popup if typing an Excel formula
        if self.show_suggestions && self.formula_text.starts_with('=') {
            let prefix = self.formula_text[1..].trim().to_uppercase();
            if !prefix.is_empty() {
                let matches: Vec<&&str> = self
                    .functions
                    .iter()
                    .filter(|name| name.starts_with(&prefix))
                    .take(6)
                    .collect();

                if !matches.is_empty() {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("함수 제안:").size(11.0).color(Color32::GRAY));
                        for &&fn_name in &matches {
                            if ui
                                .button(RichText::new(fn_name).monospace().size(11.0))
                                .clicked()
                            {
                                self.formula_text = format!("={fn_name}(");
                                self.show_suggestions = false;
                            }
                        }
                    });
                }
            }
        }

        event
    }
}
