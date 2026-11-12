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

mod control;

use control::{ControlCommand, ControlServer};
use eframe::egui;
use oneoffice_ui::{ActiveDocumentKind, StudioApp, StudioTheme};
use serde_json::{json, Value};

struct DesktopApp {
    studio: StudioApp,
    initial_path: Option<String>,
    control_server: Option<ControlServer>,
}

impl DesktopApp {
    fn new(initial_path: Option<String>, control_server: Option<ControlServer>) -> Self {
        Self {
            studio: StudioApp::new(),
            initial_path,
            control_server,
        }
    }

    fn handle_control_command(&mut self, cmd: &ControlCommand, ctx: &egui::Context) -> Value {
        match cmd.method.as_str() {
            "ping" => json!({ "ok": true, "pong": true }),
            "app.open" => {
                if let Some(p) = cmd.params.get("path").and_then(Value::as_str) {
                    self.studio.open_file(ctx, p);
                    json!({ "ok": true, "opened": p, "title": self.studio.document_title })
                } else {
                    json!({ "ok": false, "error": "Missing 'path' parameter" })
                }
            }
            "app.switch_tab" => {
                let kind = cmd.params.get("kind").and_then(Value::as_str);
                match kind {
                    Some("hwpx") => {
                        self.studio.active_doc = ActiveDocumentKind::Hwpx;
                        json!({ "ok": true, "active_doc": "hwpx" })
                    }
                    Some("docx") => {
                        self.studio.active_doc = ActiveDocumentKind::Docx;
                        json!({ "ok": true, "active_doc": "docx" })
                    }
                    Some("xlsx") => {
                        self.studio.active_doc = ActiveDocumentKind::Xlsx;
                        json!({ "ok": true, "active_doc": "xlsx" })
                    }
                    Some("pptx") => {
                        self.studio.active_doc = ActiveDocumentKind::Pptx;
                        json!({ "ok": true, "active_doc": "pptx" })
                    }
                    Some("pdf") => {
                        self.studio.active_doc = ActiveDocumentKind::Pdf;
                        json!({ "ok": true, "active_doc": "pdf" })
                    }
                    _ => json!({
                        "ok": false,
                        "error": "Invalid document kind. Expected hwpx, docx, xlsx, pptx, or pdf"
                    }),
                }
            }
            "document.inspect" => json!({
                "ok": true,
                "title": self.studio.document_title,
                "active_doc": format!("{:?}", self.studio.active_doc).to_lowercase(),
                "is_dirty": self.studio.is_dirty,
                "file_path": self.studio.current_file_path,
                "zoom_pct": self.studio.zoom_pct,
            }),
            "ui.inspect" => json!({
                "ok": true,
                "theme": match self.studio.theme {
                    StudioTheme::Dark => "dark",
                    StudioTheme::Light => "light",
                },
                "zoom_pct": self.studio.zoom_pct,
                "status_text": self.studio.status_text,
                "table_selected_cell": self.studio.table_selected_cell,
            }),
            "ui.set_theme" => {
                let theme_str = cmd.params.get("theme").and_then(Value::as_str);
                match theme_str {
                    Some("light") => {
                        self.studio.theme = StudioTheme::Light;
                        json!({ "ok": true, "theme": "light" })
                    }
                    Some("dark") => {
                        self.studio.theme = StudioTheme::Dark;
                        json!({ "ok": true, "theme": "dark" })
                    }
                    _ => json!({ "ok": false, "error": "Invalid theme. Expected 'light' or 'dark'" }),
                }
            }
            "ui.set_zoom" => {
                if let Some(zoom) = cmd.params.get("zoom_pct").and_then(Value::as_u64) {
                    self.studio.zoom_pct = zoom.clamp(25, 400) as u32;
                    json!({ "ok": true, "zoom_pct": self.studio.zoom_pct })
                } else {
                    json!({ "ok": false, "error": "Missing 'zoom_pct' integer parameter" })
                }
            }
            "ui.undo" => {
                self.studio.undo();
                json!({ "ok": true })
            }
            "ui.redo" => {
                self.studio.redo();
                json!({ "ok": true })
            }
            "table.set_cell" => {
                let row = cmd.params.get("row").and_then(Value::as_u64).map(|v| v as usize);
                let col = cmd.params.get("col").and_then(Value::as_u64).map(|v| v as usize);
                let val = cmd.params.get("value").and_then(Value::as_str);
                if let (Some(r), Some(c), Some(v)) = (row, col, val) {
                    if r < self.studio.table_data.len() && c < self.studio.table_data[r].len() {
                        self.studio.table_data[r][c] = v.to_string();
                        self.studio.is_dirty = true;
                        json!({ "ok": true, "row": r, "col": c, "value": v })
                    } else {
                        json!({ "ok": false, "error": "Cell coordinate out of bounds" })
                    }
                } else {
                    json!({ "ok": false, "error": "Missing row, col, or value" })
                }
            }
            "calc.eval" => {
                let formula = cmd.params.get("formula").and_then(Value::as_str);
                if let Some(f) = formula {
                    let f_eq = if f.starts_with('=') { f.to_string() } else { format!("={f}") };
                    let spec = oneoffice_bridge::calc::CalcTableSpec {
                        sheet_name: "LiveCalc".to_string(),
                        cells: vec![vec![oneoffice_bridge::calc::CellInput::formula(f_eq)]],
                        merges: Vec::new(),
                    };
                    match oneoffice_bridge::calc::CalcTableEngine::evaluate_spec(&spec) {
                        Ok(out) => {
                            let res = out.display_cells.first().and_then(|r| r.first()).cloned().unwrap_or_default();
                            json!({ "ok": true, "result": res })
                        }
                        Err(e) => json!({ "ok": false, "error": e.to_string() }),
                    }
                } else {
                    json!({ "ok": false, "error": "Missing formula parameter" })
                }
            }
            "app.save" => match self.studio.save_current_document() {
                Ok(bytes) => json!({
                    "ok": true,
                    "size_bytes": bytes.len(),
                    "title": self.studio.document_title,
                }),
                Err(e) => json!({ "ok": false, "error": e }),
            },
            other => json!({ "ok": false, "error": format!("Unknown method '{other}'") }),
        }
    }
}

impl eframe::App for DesktopApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut pending_commands = Vec::new();
        if let Some(ref server) = self.control_server {
            while let Ok(cmd) = server.cmd_rx.try_recv() {
                pending_commands.push(cmd);
            }
        }
        for cmd in pending_commands {
            let reply = self.handle_control_command(&cmd, ctx);
            let _ = cmd.responder.send(reply);
        }

        if let Some(path) = self.initial_path.take() {
            self.studio.open_file(ctx, &path);
        }

        if self.studio.request_open_file {
            self.studio.request_open_file = false;
            let dialog = rfd::FileDialog::new()
                .add_filter(
                    "모든 지원 문서",
                    &["hwpx", "hwp", "docx", "doc", "xlsx", "xls", "pptx", "ppt", "pdf"],
                )
                .add_filter("한글 문서 (*.hwpx, *.hwp)", &["hwpx", "hwp"])
                .add_filter("워드 문서 (*.docx, *.doc)", &["docx", "doc"])
                .add_filter("엑셀 문서 (*.xlsx, *.xls)", &["xlsx", "xls"])
                .add_filter("프레젠테이션 (*.pptx, *.ppt)", &["pptx", "ppt"])
                .add_filter("PDF 문서 (*.pdf)", &["pdf"]);

            if let Some(path) = dialog.pick_file()
                && let Some(path_str) = path.to_str()
            {
                self.studio.open_file(ctx, path_str);
            }
        }

        if self.studio.request_save_as {
            self.studio.request_save_as = false;
            let (filter_name, filter_ext) = match self.studio.active_doc {
                ActiveDocumentKind::Hwpx => ("HWPX Document (*.hwpx)", "hwpx"),
                ActiveDocumentKind::Docx => ("Word Document (*.docx)", "docx"),
                ActiveDocumentKind::Xlsx => ("Excel Workbook (*.xlsx)", "xlsx"),
                ActiveDocumentKind::Pptx => ("PowerPoint Presentation (*.pptx)", "pptx"),
                ActiveDocumentKind::Pdf => ("PDF Document (*.pdf)", "pdf"),
            };

            let dialog = rfd::FileDialog::new()
                .add_filter(filter_name, &[filter_ext])
                .set_file_name(&self.studio.document_title);

            if let Some(path) = dialog.save_file()
                && let Some(path_str) = path.to_str()
            {
                self.studio.current_file_path = Some(path_str.to_string());
                let _ = self.studio.save_current_document();
            }
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
    let mut control_port = None;

    let mut args = std::env::args().skip(1).peekable();
    while let Some(arg) = args.next() {
        if arg == "--control" {
            if let Some(port_str) = args.next()
                && let Ok(p) = port_str.parse::<u16>()
            {
                control_port = Some(p);
            }
        } else if !arg.starts_with("--") {
            initial_path = Some(arg);
        }
    }

    let control_server = match control_port {
        Some(port) => match ControlServer::start(port) {
            Ok(server) => {
                println!("[ONEOFFICE] Control server listening on 127.0.0.1:{}", server.port);
                Some(server)
            }
            Err(e) => {
                eprintln!("[ONEOFFICE] Control server startup warning: {e}");
                None
            }
        },
        None => None,
    };

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
        Box::new(move |_cc| Ok(Box::new(DesktopApp::new(initial_path, control_server)))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_desktop_app_init() {
        let app = DesktopApp::new(Some("test_document.hwpx".to_string()), None);
        assert_eq!(app.initial_path.as_deref(), Some("test_document.hwpx"));
        assert_eq!(app.studio.document_title, "2026 차세대 사업계획서 및 표준용역계약서.hwpx");
    }

    #[test]
    fn test_desktop_app_file_requests() {
        let mut app = DesktopApp::new(None, None);
        assert!(!app.studio.request_open_file);
        assert!(!app.studio.request_save_as);

        app.studio.request_open_file = true;
        assert!(app.studio.request_open_file);

        app.studio.request_save_as = true;
        assert!(app.studio.request_save_as);
    }

    #[test]
    fn test_desktop_app_control_commands() {
        let mut app = DesktopApp::new(None, None);
        let ctx = egui::Context::default();

        // 1. Ping
        let ping_cmd = ControlCommand {
            method: "ping".to_string(),
            params: Value::Null,
            responder: std::sync::mpsc::channel().0,
        };
        let res = app.handle_control_command(&ping_cmd, &ctx);
        assert_eq!(res["ok"], true);
        assert_eq!(res["pong"], true);

        // 2. Switch Tab
        let switch_cmd = ControlCommand {
            method: "app.switch_tab".to_string(),
            params: json!({ "kind": "pdf" }),
            responder: std::sync::mpsc::channel().0,
        };
        let res = app.handle_control_command(&switch_cmd, &ctx);
        assert_eq!(res["ok"], true);
        assert_eq!(app.studio.active_doc, ActiveDocumentKind::Pdf);

        // 3. Document Inspect
        let inspect_cmd = ControlCommand {
            method: "document.inspect".to_string(),
            params: Value::Null,
            responder: std::sync::mpsc::channel().0,
        };
        let res = app.handle_control_command(&inspect_cmd, &ctx);
        assert_eq!(res["ok"], true);
        assert_eq!(res["active_doc"], "pdf");

        // 4. UI Set Theme
        let theme_cmd = ControlCommand {
            method: "ui.set_theme".to_string(),
            params: json!({ "theme": "light" }),
            responder: std::sync::mpsc::channel().0,
        };
        let res = app.handle_control_command(&theme_cmd, &ctx);
        assert_eq!(res["ok"], true);
        assert_eq!(app.studio.theme, StudioTheme::Light);

        // 5. Calc Eval
        let calc_cmd = ControlCommand {
            method: "calc.eval".to_string(),
            params: json!({ "formula": "=SUM(500, 250)" }),
            responder: std::sync::mpsc::channel().0,
        };
        let res = app.handle_control_command(&calc_cmd, &ctx);
        assert_eq!(res["ok"], true);
        assert_eq!(res["result"], "750");

        // 6. Table Set Cell
        let cell_cmd = ControlCommand {
            method: "table.set_cell".to_string(),
            params: json!({ "row": 1, "col": 1, "value": "9999999" }),
            responder: std::sync::mpsc::channel().0,
        };
        let res = app.handle_control_command(&cell_cmd, &ctx);
        assert_eq!(res["ok"], true);
        assert_eq!(app.studio.table_data[1][1], "9999999");
        assert!(app.studio.is_dirty);
    }
}
