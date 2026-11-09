//! JSON-RPC 2.0 stdio MCP server loop and message dispatcher.

use std::io::{BufRead, Write};

use serde_json::{json, Value};

use crate::error::McpError;
use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::tools::{call_tool, list_tools};

/// Unified ONEOFFICE MCP server instance.
#[derive(Clone, Debug, Default)]
pub struct McpServer;

impl McpServer {
    /// Creates a new MCP server.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Handles a single incoming JSON-RPC 2.0 request and produces the response.
    #[must_use]
    pub fn handle_request(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        let req_id = req.id.clone();
        match req.method.as_str() {
            "initialize" => {
                let result = json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {
                            "listChanged": false
                        }
                    },
                    "serverInfo": {
                        "name": "oneoffice-mcp",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                });
                JsonRpcResponse::success(req_id, result)
            }
            "ping" => JsonRpcResponse::success(req_id, json!({})),
            "notifications/initialized" => {
                // Client acknowledgment notification: no response needed for notifications in standard JSON-RPC
                JsonRpcResponse::success(req_id, json!({}))
            }
            "tools/list" => {
                let tools = list_tools();
                JsonRpcResponse::success(req_id, json!({ "tools": tools }))
            }
            "tools/call" => {
                let params = req.params.unwrap_or(Value::Null);
                let tool_name = params.get("name").and_then(Value::as_str);
                let arguments = params.get("arguments").unwrap_or(&Value::Null);

                match tool_name {
                    Some(name) => match call_tool(name, arguments) {
                        Ok(result) => JsonRpcResponse::success(
                            req_id,
                            serde_json::to_value(result).unwrap_or(Value::Null),
                        ),
                        Err(e) => JsonRpcResponse::error(req_id, -32000, e.to_string()),
                    },
                    None => JsonRpcResponse::error(
                        req_id,
                        -32602,
                        "Missing required param 'name' in tools/call",
                    ),
                }
            }
            _ => JsonRpcResponse::error(
                req_id,
                -32601,
                format!("Method not found: '{}'", req.method),
            ),
        }
    }

    /// Dispatches a raw JSON-RPC string line and returns the serialized JSON response string.
    pub fn handle_line(&self, line: &str) -> Result<String, McpError> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Ok(String::new());
        }

        let req: JsonRpcRequest = serde_json::from_str(trimmed)?;
        let resp = self.handle_request(req);
        let resp_str = serde_json::to_string(&resp)?;
        Ok(resp_str)
    }

    /// Runs the stdio line-delimited JSON-RPC loop until EOF.
    pub fn run_stdio<R: BufRead, W: Write>(&self, mut reader: R, mut writer: W) -> Result<(), McpError> {
        let mut line = String::new();
        while reader
            .read_line(&mut line)
            .map_err(|e| McpError::Io(e.to_string()))?
            > 0
        {
            if let Ok(resp_line) = self.handle_line(&line)
                && !resp_line.is_empty()
            {
                writeln!(writer, "{resp_line}").map_err(|e| McpError::Io(e.to_string()))?;
                writer.flush().map_err(|e| McpError::Io(e.to_string()))?;
            }
            line.clear();
        }
        Ok(())
    }
}
