//! Live TCP JSON Lines control server for `oneoffice-desktop`.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

use serde_json::{json, Value};

/// Encapsulates a control request received over the TCP socket.
pub struct ControlCommand {
    pub method: String,
    pub params: Value,
    pub responder: Sender<Value>,
}

/// Control server instance listening for live commands.
pub struct ControlServer {
    pub port: u16,
    pub cmd_rx: Receiver<ControlCommand>,
}

impl ControlServer {
    /// Starts the control TCP listener on `127.0.0.1:port`.
    pub fn start(port: u16) -> Result<Self, String> {
        let listener = TcpListener::bind(format!("127.0.0.1:{port}"))
            .map_err(|e| format!("Failed to bind control TCP socket to 127.0.0.1:{port}: {e}"))?;

        let (cmd_tx, cmd_rx) = channel::<ControlCommand>();

        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let cmd_tx_clone = cmd_tx.clone();
                thread::spawn(move || {
                    handle_client(stream, cmd_tx_clone);
                });
            }
        });

        Ok(Self { port, cmd_rx })
    }
}

fn handle_client(mut stream: TcpStream, cmd_tx: Sender<ControlCommand>) {
    let reader = BufReader::new(match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    });

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let req: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                let err_resp = json!({ "ok": false, "error": format!("Invalid JSON: {e}") });
                let _ = writeln!(stream, "{err_resp}");
                continue;
            }
        };

        let method = req
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let params = req.get("params").cloned().unwrap_or(Value::Null);

        let (reply_tx, reply_rx) = channel();
        let cmd = ControlCommand {
            method,
            params,
            responder: reply_tx,
        };

        if cmd_tx.send(cmd).is_err() {
            break;
        }

        let resp = match reply_rx.recv_timeout(std::time::Duration::from_secs(5)) {
            Ok(v) => v,
            Err(_) => json!({ "ok": false, "error": "Request timed out waiting for UI frame" }),
        };

        if writeln!(stream, "{resp}").is_err() {
            break;
        }
    }
}
