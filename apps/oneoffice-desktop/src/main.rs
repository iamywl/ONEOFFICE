//! Cross-platform Native Desktop App (`macOS` / `Windows` / `Linux`) for ONEOFFICE.
#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable
)]

use eframe::egui;
use oneoffice_ui::StudioApp;

struct DesktopApp {
    studio: StudioApp,
    initial_path: Option<String>,
}

impl DesktopApp {
    fn new(initial_path: Option<String>) -> Self {
        Self {
            studio: StudioApp::new(),
            initial_path,
        }
    }
}

impl eframe::App for DesktopApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(path) = self.initial_path.take() {
            self.studio.open_file(ctx, &path);
        }
        self.studio.logic(ctx);
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        for f in std::mem::take(&mut raw.dropped_files) {
            let path = f.path();
            if let Some(path_str) = path.to_str()
                && !path_str.is_empty()
            {
                self.studio.open_file(ctx, path_str);
            }
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.studio.show(ui);
    }
}

fn main() -> Result<(), eframe::Error> {
    let mut initial_path = None;
    let args = std::env::args().skip(1);
    for arg in args {
        if !arg.starts_with("--") {
            initial_path = Some(arg);
            break;
        }
    }

    let viewport = egui::ViewportBuilder::default()
        .with_title("ONEOFFICE — 통합 오피스 플랫폼")
        .with_inner_size([1440.0, 920.0])
        .with_min_inner_size([800.0, 600.0])
        .with_app_id("com.oneoffice.desktop")
        .with_drag_and_drop(true);

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "ONEOFFICE",
        options,
        Box::new(move |_cc| Ok(Box::new(DesktopApp::new(initial_path)))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_desktop_app_init() {
        let app = DesktopApp::new(Some("test_document.hwpx".to_string()));
        assert_eq!(app.initial_path.as_deref(), Some("test_document.hwpx"));
        assert_eq!(app.studio.document_title, "2026 차세대 사업계획서 및 표준용역계약서.hwpx");
    }
}
