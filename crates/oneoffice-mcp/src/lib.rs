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
        assert_eq!(tools.len(), 7);

        let names: Vec<&str> = tools
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"oneoffice.doc.create"));
        assert!(names.contains(&"oneoffice.doc.inspect"));
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

    #[test]
    fn test_tool_doc_create_and_inspect_docx() {
        let args = json!({
            "format": "docx",
            "title": "DOCX 생성 테스트 제안서",
            "paragraphs": [
                "이 문서는 ONEOFFICE MCP 서버의 표준 문서 생성 도구를 통해 자동 생성되었습니다.",
                "두 번째 문단: 도식화 및 수식표와 함께 패키징됩니다."
            ],
            "table": [
                ["품명", "단가", "수량"],
                ["엔진 라이선스", "5000000", "2"],
                ["유지보수", "1000000", "1"]
            ],
            "diagram_template_id": "problem-comparison-to-solution"
        });

        let create_res = call_tool("oneoffice.doc.create", &args).expect("doc create failed");
        assert!(!create_res.is_error);
        let create_parsed: serde_json::Value = serde_json::from_str(&create_res.content[0].text).unwrap();
        assert_eq!(create_parsed["format"], "docx");
        assert_eq!(create_parsed["has_table"], true);
        assert_eq!(create_parsed["has_diagram"], true);
        let doc_b64 = create_parsed["document_base64"].as_str().unwrap();
        assert!(!doc_b64.is_empty());

        // Inspect the generated DOCX
        let inspect_args = json!({
            "data_base64": doc_b64
        });
        let inspect_res = call_tool("oneoffice.doc.inspect", &inspect_args).expect("doc inspect failed");
        assert!(!inspect_res.is_error);
        let inspect_parsed: serde_json::Value = serde_json::from_str(&inspect_res.content[0].text).unwrap();
        assert_eq!(inspect_parsed["format"], "docx");
        assert!(inspect_parsed["paragraphs_count"].as_u64().unwrap() >= 2);
        assert_eq!(inspect_parsed["tables_count"], 1);
        assert_eq!(inspect_parsed["media_count"], 1);
    }

    #[test]
    fn test_tool_doc_create_and_inspect_hwpx() {
        let args = json!({
            "format": "hwpx",
            "title": "HWPX KS X 6101 제안서",
            "paragraphs": [
                "한국 공공기관 표준 HWPX 문서입니다.",
                "513함수 엑셀 수식표와 제안서 도식이 무손실 결합됩니다."
            ],
            "table": [
                ["구분", "공급가"],
                ["서버", "12000000"],
                ["합계", "=SUM(B2:B2)"]
            ],
            "diagram_template_id": "problem-comparison-to-solution"
        });

        let create_res = call_tool("oneoffice.doc.create", &args).expect("hwpx create failed");
        assert!(!create_res.is_error);
        let create_parsed: serde_json::Value = serde_json::from_str(&create_res.content[0].text).unwrap();
        assert_eq!(create_parsed["format"], "hwpx");
        assert_eq!(create_parsed["has_table"], true);
        assert_eq!(create_parsed["has_diagram"], true);
        let doc_b64 = create_parsed["document_base64"].as_str().unwrap();

        // Inspect the generated HWPX
        let inspect_args = json!({
            "data_base64": doc_b64
        });
        let inspect_res = call_tool("oneoffice.doc.inspect", &inspect_args).expect("hwpx inspect failed");
        assert!(!inspect_res.is_error);
        let inspect_parsed: serde_json::Value = serde_json::from_str(&inspect_res.content[0].text).unwrap();
        assert_eq!(inspect_parsed["format"], "hwpx");
        assert_eq!(inspect_parsed["has_table"], true);
        assert_eq!(inspect_parsed["has_diagram"], true);
        let attachments = inspect_parsed["attachments"].as_array().unwrap();
        let att_str: Vec<&str> = attachments.iter().map(|v| v.as_str().unwrap()).collect();
        assert!(att_str.contains(&"BinData/table_1.xlsx"));
    }

    #[test]
    fn test_tool_doc_inspect_pdf() {
        let contract_pdf = oneoffice_pdf::generate_sample_korean_contract_pdf()
            .expect("contract generation failed");
        let pdf_b64 = BASE64.encode(&contract_pdf);

        let inspect_args = json!({
            "data_base64": pdf_b64
        });
        let inspect_res = call_tool("oneoffice.doc.inspect", &inspect_args).expect("pdf inspect failed");
        assert!(!inspect_res.is_error);
        let inspect_parsed: serde_json::Value = serde_json::from_str(&inspect_res.content[0].text).unwrap();
        assert_eq!(inspect_parsed["format"], "pdf");
        assert_eq!(inspect_parsed["page_count"], 1);
        assert!(inspect_parsed["signing_anchors_count"].as_u64().unwrap() >= 1);
    }

    #[test]
    fn test_tool_chart_upsert() {
        let args = json!({
            "spec": {
                "kind": "ColumnClustered",
                "categories": ["2023", "2024", "2025", "2026"],
                "series": [
                    {
                        "name": "연간매출액",
                        "values": [120.0, 250.0, 480.0, 950.0]
                    }
                ],
                "width_pt": 480.0,
                "height_pt": 260.0
            }
        });
        let result = call_tool("oneoffice.chart.upsert", &args).expect("chart upsert failed");
        assert!(!result.is_error);
        let parsed: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
        assert_eq!(parsed["success"], true);
        assert!(parsed["svg"].as_str().unwrap().contains("<svg"));
        assert!(!parsed["png_base64"].as_str().unwrap().is_empty());
        assert!(parsed["ooxml_chart_xml"].as_str().unwrap().contains("<c:chartSpace"));
        assert!(!parsed["xlsx_base64"].as_str().unwrap().is_empty());
    }
}
