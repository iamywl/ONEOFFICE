//! Unified ONEOFFICE Model Context Protocol (MCP) server library.

#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable
)]

pub mod error;
pub mod protocol;
pub mod server;
pub mod tools;

pub use error::{McpError, McpResult};
pub use protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, McpToolInfo, McpToolResult};
pub use server::McpServer;
pub use tools::{call_tool, list_tools};

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use base64::Engine as _;
    use base64::engine::general_purpose::STANDARD as BASE64;
    use serde_json::json;

    use super::*;

    #[test]
    fn test_server_initialize_and_ping() {
        let server = McpServer::new();

        let init_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: Some(json!({})),
        };
        let init_resp = server.handle_request(init_req);
        assert!(init_resp.error.is_none());
        let res = init_resp.result.unwrap();
        assert_eq!(res["serverInfo"]["name"], "oneoffice-mcp");

        let ping_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(2)),
            method: "ping".to_string(),
            params: None,
        };
        let ping_resp = server.handle_request(ping_req);
        assert!(ping_resp.error.is_none());
    }

    #[test]
    fn test_tools_list() {
        let server = McpServer::new();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(10)),
            method: "tools/list".to_string(),
            params: None,
        };
        let resp = server.handle_request(req);
        assert!(resp.error.is_none());
        let res = resp.result.unwrap();
        let tools = res["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 5);

        let names: Vec<&str> = tools
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"oneoffice.diagram.compose"));
        assert!(names.contains(&"oneoffice.table.upsert"));
        assert!(names.contains(&"oneoffice.chart.upsert"));
        assert!(names.contains(&"oneoffice.pdf.sign"));
        assert!(names.contains(&"oneoffice.visual.verify_page"));
    }

    #[test]
    fn test_tool_diagram_compose() {
        let args = json!({
            "template_id": "problem-comparison-to-solution"
        });
        let result = call_tool("oneoffice.diagram.compose", &args).expect("compose failed");
        assert!(!result.is_error);
        let parsed: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
        assert_eq!(parsed["success"], true);
        assert!(parsed["svg"].as_str().unwrap().contains("<svg"));
        assert!(!parsed["png_base64"].as_str().unwrap().is_empty());
        assert!(parsed["owpml"].as_str().unwrap().contains("<hp:container"));
        assert_eq!(parsed["diagnostics"]["overflow_count"], 0);
    }

    #[test]
    fn test_tool_table_upsert() {
        let args = json!({
            "spec": {
                "sheet_name": "MCP예산",
                "cells": [
                    [{"value": "항목"}, {"value": "금액"}],
                    [{"value": "클라우드"}, {"value": "200000"}],
                    [{"value": "네트워크"}, {"value": "100000"}],
                    [{"value": "합계"}, {"value": "=SUM(B2:B3)"}]
                ]
            }
        });
        let result = call_tool("oneoffice.table.upsert", &args).expect("table upsert failed");
        assert!(!result.is_error);
        let parsed: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
        assert_eq!(parsed["success"], true);
        assert_eq!(parsed["display_cells"][3][1], "300000");
        assert!(parsed["hwpx_table_xml"].as_str().unwrap().contains("<hp:tbl"));
        assert!(!parsed["rendered_cells_sha256"].as_str().unwrap().is_empty());
    }

    #[test]
    fn test_tool_pdf_sign_and_visual_verify() {
        let contract_pdf = oneoffice_pdf::generate_sample_korean_contract_pdf()
            .expect("contract generation failed");
        let pdf_b64 = BASE64.encode(&contract_pdf);

        let sign_args = json!({
            "pdf_base64": pdf_b64,
            "anchor_text": "(인)",
            "seal_label": "대표이사\n원오피스"
        });
        let sign_res = call_tool("oneoffice.pdf.sign", &sign_args).expect("pdf sign failed");
        assert!(!sign_res.is_error);
        let sign_parsed: serde_json::Value = serde_json::from_str(&sign_res.content[0].text).unwrap();
        assert_eq!(sign_parsed["success"], true);
        let signed_pdf_b64 = sign_parsed["signed_pdf_base64"].as_str().unwrap();

        // Closed-loop verification with oneoffice.visual.verify_page
        let verify_args = json!({
            "format": "pdf",
            "data_base64": signed_pdf_b64,
            "page_index": 0,
            "scale": 1.5
        });
        let verify_res = call_tool("oneoffice.visual.verify_page", &verify_args)
            .expect("visual verify failed");
        assert!(!verify_res.is_error);
        let verify_parsed: serde_json::Value = serde_json::from_str(&verify_res.content[0].text).unwrap();
        assert_eq!(verify_parsed["is_clean"], true);
        assert!(!verify_parsed["png_base64"].as_str().unwrap().is_empty());
    }
}
