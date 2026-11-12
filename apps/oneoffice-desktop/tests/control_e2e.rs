#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

use serde_json::{json, Value};

#[test]
fn test_live_desktop_control_socket_e2e() {
    let port = 17988;
    let listener = std::net::TcpListener::bind(format!("127.0.0.1:{port}")).expect("bind port");

    let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<AppsCmd>();

    struct AppsCmd {
        method: String,
        params: Value,
        responder: std::sync::mpsc::Sender<Value>,
    }

    // Mock UI thread listening on channel
    let ui_thread = thread::spawn(move || {
        let mut active_doc = "hwpx".to_string();
        let mut zoom_pct = 100;
        let mut theme = "dark".to_string();

        while let Ok(cmd) = cmd_rx.recv() {
            let is_quit = cmd.method == "quit";
            let reply = match cmd.method.as_str() {
                "ping" => json!({ "ok": true, "pong": true }),
                "app.switch_tab" => {
                    if let Some(k) = cmd.params.get("kind").and_then(Value::as_str) {
                        active_doc = k.to_string();
                        json!({ "ok": true, "active_doc": active_doc })
                    } else {
                        json!({ "ok": false })
                    }
                }
                "ui.inspect" => json!({
                    "ok": true,
                    "theme": theme,
                    "zoom_pct": zoom_pct,
                    "active_doc": active_doc,
                }),
                "ui.set_theme" => {
                    if let Some(t) = cmd.params.get("theme").and_then(Value::as_str) {
                        theme = t.to_string();
                        json!({ "ok": true, "theme": theme })
                    } else {
                        json!({ "ok": false })
                    }
                }
                "ui.set_zoom" => {
                    if let Some(z) = cmd.params.get("zoom_pct").and_then(Value::as_u64) {
                        zoom_pct = z as u32;
                        json!({ "ok": true, "zoom_pct": zoom_pct })
                    } else {
                        json!({ "ok": false })
                    }
                }
                "calc.eval" => {
                    if let Some(f) = cmd.params.get("formula").and_then(Value::as_str) {
                        let f_eq = if f.starts_with('=') { f.to_string() } else { format!("={f}") };
                        let spec = oneoffice_bridge::calc::CalcTableSpec {
                            sheet_name: "Test".to_string(),
                            cells: vec![vec![oneoffice_bridge::calc::CellInput::formula(f_eq)]],
                            merges: Vec::new(),
                        };
                        let out = oneoffice_bridge::calc::CalcTableEngine::evaluate_spec(&spec).unwrap();
                        let res = out.display_cells[0][0].clone();
                        json!({ "ok": true, "result": res })
                    } else {
                        json!({ "ok": false })
                    }
                }
                "quit" => json!({ "ok": true }),
                _ => json!({ "ok": false, "error": "unknown method" }),
            };
            let _ = cmd.responder.send(reply);
            if is_quit {
                break;
            }
        }
    });

    // Accept thread
    let server_thread = thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            while reader.read_line(&mut line).unwrap() > 0 {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    let req: Value = serde_json::from_str(trimmed).unwrap();
                    let (tx, rx) = std::sync::mpsc::channel();
                    let cmd = AppsCmd {
                        method: req["method"].as_str().unwrap().to_string(),
                        params: req["params"].clone(),
                        responder: tx,
                    };
                    cmd_tx.send(cmd).unwrap();
                    let resp = rx.recv_timeout(Duration::from_secs(2)).unwrap();
                    writeln!(stream, "{resp}").unwrap();
                    if req["method"] == "quit" {
                        break;
                    }
                }
                line.clear();
            }
        }
    });

    // Client connection
    thread::sleep(Duration::from_millis(50));
    let mut client = TcpStream::connect(format!("127.0.0.1:{port}")).expect("client connect");
    let mut reader = BufReader::new(client.try_clone().unwrap());

    // 1. Send Ping
    writeln!(client, "{}", json!({ "method": "ping", "params": {} })).unwrap();
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    let r1: Value = serde_json::from_str(&line).unwrap();
    assert_eq!(r1["ok"], true);
    assert_eq!(r1["pong"], true);

    // 2. Switch Tab to PDF
    line.clear();
    writeln!(client, "{}", json!({ "method": "app.switch_tab", "params": { "kind": "pdf" } })).unwrap();
    reader.read_line(&mut line).unwrap();
    let r2: Value = serde_json::from_str(&line).unwrap();
    assert_eq!(r2["ok"], true);
    assert_eq!(r2["active_doc"], "pdf");

    // 3. Set Theme to Light
    line.clear();
    writeln!(client, "{}", json!({ "method": "ui.set_theme", "params": { "theme": "light" } })).unwrap();
    reader.read_line(&mut line).unwrap();
    let r3: Value = serde_json::from_str(&line).unwrap();
    assert_eq!(r3["ok"], true);
    assert_eq!(r3["theme"], "light");

    // 4. Calc Eval formula over TCP
    line.clear();
    writeln!(client, "{}", json!({ "method": "calc.eval", "params": { "formula": "=SUM(1234, 5678)" } })).unwrap();
    reader.read_line(&mut line).unwrap();
    let r4: Value = serde_json::from_str(&line).unwrap();
    assert_eq!(r4["ok"], true);
    assert_eq!(r4["result"], "6912");

    // 5. Quit
    line.clear();
    writeln!(client, "{}", json!({ "method": "quit", "params": {} })).unwrap();

    server_thread.join().unwrap();
    ui_thread.join().unwrap();
}
