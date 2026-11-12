#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use oneoffice_mcp::{call_tool, McpError};
use serde_json::json;

#[test]
fn test_mcp_adversarial_zero_panic_all_tools() {
    // ------------------------------------------------------------------------
    // RED-MCP-01: Unknown Tool Name Dispatch
    // ------------------------------------------------------------------------
    let res = call_tool("oneoffice.ghost.nonexistent", &json!({}));
    assert!(matches!(res, Err(McpError::ToolNotFound(_))));

    // ------------------------------------------------------------------------
    // RED-MCP-02: doc.create Hostile Inputs
    // ------------------------------------------------------------------------
    // 2a. Missing format
    let err_res = call_tool("oneoffice.doc.create", &json!({}));
    assert!(matches!(err_res, Err(McpError::InvalidArguments { .. })));

    // 2b. Unsupported format
    let err_res = call_tool("oneoffice.doc.create", &json!({ "format": "exe" }));
    assert!(matches!(err_res, Err(McpError::InvalidArguments { .. })));

    // 2c. Invalid base64 in image_base64
    let err_res = call_tool("oneoffice.doc.create", &json!({
        "format": "hwpx",
        "image_base64": "!!!NOT_VALID_BASE64!!!"
    }));
    assert!(matches!(err_res, Err(McpError::Base64(_))));

    // ------------------------------------------------------------------------
    // RED-MCP-03: doc.inspect Hostile Inputs
    // ------------------------------------------------------------------------
    // 3a. Missing data_base64
    let err_res = call_tool("oneoffice.doc.inspect", &json!({}));
    assert!(matches!(err_res, Err(McpError::InvalidArguments { .. })));

    // 3b. Corrupted Base64
    let err_res = call_tool("oneoffice.doc.inspect", &json!({ "data_base64": "corrupted!#" }));
    assert!(matches!(err_res, Err(McpError::Base64(_))));

    // 3c. Arbitrary binary junk (valid base64 of random noise)
    let junk_b64 = BASE64.encode(b"\xDE\xAD\xBE\xEF\x00\xFF\xAA\x55");
    let err_res = call_tool("oneoffice.doc.inspect", &json!({ "data_base64": junk_b64 }));
    assert!(matches!(err_res, Err(McpError::InvalidArguments { .. })));

    // ------------------------------------------------------------------------
    // RED-MCP-04: doc.convert Hostile Inputs
    // ------------------------------------------------------------------------
    // 4a. Missing arguments
    let err_res = call_tool("oneoffice.doc.convert", &json!({}));
    assert!(matches!(err_res, Err(McpError::InvalidArguments { .. })));

    // 4b. Unsupported conversion pair
    let dummy_b64 = BASE64.encode(b"dummy");
    let err_res = call_tool("oneoffice.doc.convert", &json!({
        "data_base64": dummy_b64,
        "from_format": "pdf",
        "to_format": "docx"
    }));
    assert!(matches!(err_res, Err(McpError::InvalidArguments { .. })));

    // 4c. Verify newly added conversions: md -> docx and hwpx -> docx
    let md_text = "# 제목\n\n첫번째 문단 내용입니다.\n\n두번째 문단 내용입니다.";
    let md_b64 = BASE64.encode(md_text.as_bytes());
    let md_to_docx_res = call_tool("oneoffice.doc.convert", &json!({
        "data_base64": md_b64,
        "from_format": "md",
        "to_format": "docx"
    })).expect("md to docx must succeed");
    let parsed: serde_json::Value = serde_json::from_str(&md_to_docx_res.content[0].text).unwrap();
    assert_eq!(parsed["success"], true);
    assert_eq!(parsed["to_format"], "docx");
    let docx_b64 = parsed["output_base64"].as_str().unwrap();

    // Verify generated DOCX via doc.inspect
    let inspect_res = call_tool("oneoffice.doc.inspect", &json!({
        "data_base64": docx_b64
    })).expect("inspect docx must succeed");
    let insp_parsed: serde_json::Value = serde_json::from_str(&inspect_res.content[0].text).unwrap();
    assert_eq!(insp_parsed["format"], "docx");
    assert!(insp_parsed["paragraphs_count"].as_u64().unwrap() >= 2);

    // ------------------------------------------------------------------------
    // RED-MCP-05: doc.fill_fields Hostile Inputs
    // ------------------------------------------------------------------------
    // 5a. Missing fields
    let err_res = call_tool("oneoffice.doc.fill_fields", &json!({
        "data_base64": dummy_b64
    }));
    assert!(matches!(err_res, Err(McpError::InvalidArguments { .. })));

    // 5b. Unsupported format
    let err_res = call_tool("oneoffice.doc.fill_fields", &json!({
        "data_base64": dummy_b64,
        "format": "pdf",
        "fields": { "NAME": "테스트" }
    }));
    assert!(matches!(err_res, Err(McpError::InvalidArguments { .. })));

    // 5c. Broken package bytes for hwpx
    let err_res = call_tool("oneoffice.doc.fill_fields", &json!({
        "data_base64": dummy_b64,
        "format": "hwpx",
        "fields": { "NAME": "테스트" }
    }));
    assert!(matches!(err_res, Err(McpError::Engine(_))));

    // ------------------------------------------------------------------------
    // RED-MCP-06: diagram.list_templates Hostile Inputs
    // ------------------------------------------------------------------------
    // Category filter that doesn't exist: should return 0 count without panic
    let res = call_tool("oneoffice.diagram.list_templates", &json!({
        "category": "non_existent_category_xyz"
    })).expect("should handle smoothly");
    let parsed: serde_json::Value = serde_json::from_str(&res.content[0].text).unwrap();
    assert_eq!(parsed["total_count"], 0);

    // ------------------------------------------------------------------------
    // RED-MCP-07: diagram.compose Hostile Inputs
    // ------------------------------------------------------------------------
    // Invalid template id
    let err_res = call_tool("oneoffice.diagram.compose", &json!({
        "template_id": "invalid_template_id_9999"
    }));
    assert!(matches!(err_res, Err(McpError::Engine(_))));

    // ------------------------------------------------------------------------
    // RED-MCP-08: diagram.patch Hostile Inputs
    // ------------------------------------------------------------------------
    // Missing spec
    let err_res = call_tool("oneoffice.diagram.patch", &json!({ "patch": {} }));
    assert!(matches!(err_res, Err(McpError::InvalidArguments { .. })));

    // ------------------------------------------------------------------------
    // RED-MCP-09: table.upsert Hostile Inputs
    // ------------------------------------------------------------------------
    // Missing spec
    let err_res = call_tool("oneoffice.table.upsert", &json!({}));
    assert!(matches!(err_res, Err(McpError::InvalidArguments { .. })));

    // Formula with Division by zero
    let div_zero_res = call_tool("oneoffice.table.upsert", &json!({
        "spec": {
            "sheet_name": "DivZero",
            "cells": [[{ "value": "=100/0" }]]
        }
    })).expect("div zero should evaluate to error string without engine crash");
    let parsed: serde_json::Value = serde_json::from_str(&div_zero_res.content[0].text).unwrap();
    assert_eq!(parsed["display_cells"][0][0], "#DIV/0!");

    // ------------------------------------------------------------------------
    // RED-MCP-10: chart.upsert Hostile Inputs
    // ------------------------------------------------------------------------
    // Missing spec
    let err_res = call_tool("oneoffice.chart.upsert", &json!({}));
    assert!(matches!(err_res, Err(McpError::InvalidArguments { .. })));

    // ------------------------------------------------------------------------
    // RED-MCP-11: pdf.find_anchors Hostile Inputs
    // ------------------------------------------------------------------------
    // Missing pdf_base64
    let err_res = call_tool("oneoffice.pdf.find_anchors", &json!({}));
    assert!(matches!(err_res, Err(McpError::InvalidArguments { .. })));

    // Out-of-bounds page index on valid PDF: should return 0 matches gracefully
    let contract_pdf = oneoffice_pdf::generate_sample_korean_contract_pdf().unwrap();
    let pdf_b64 = BASE64.encode(&contract_pdf);
    let oob_res = call_tool("oneoffice.pdf.find_anchors", &json!({
        "pdf_base64": pdf_b64,
        "page_index": 99999
    })).expect("out-of-bounds page should return empty results without panic");
    let parsed: serde_json::Value = serde_json::from_str(&oob_res.content[0].text).unwrap();
    assert_eq!(parsed["total_matches"], 0);

    // ------------------------------------------------------------------------
    // RED-MCP-12: pdf.sign Hostile Inputs
    // ------------------------------------------------------------------------
    // Missing pdf_base64
    let err_res = call_tool("oneoffice.pdf.sign", &json!({}));
    assert!(matches!(err_res, Err(McpError::InvalidArguments { .. })));

    // ------------------------------------------------------------------------
    // RED-MCP-13: calc.eval Hostile Inputs
    // ------------------------------------------------------------------------
    // 13a. Missing formula
    let err_res = call_tool("oneoffice.calc.eval", &json!({}));
    assert!(matches!(err_res, Err(McpError::InvalidArguments { .. })));

    // 13b. Division by zero in formula
    let res = call_tool("oneoffice.calc.eval", &json!({
        "formula": "=500/0"
    })).expect("div by zero must not panic");
    let parsed: serde_json::Value = serde_json::from_str(&res.content[0].text).unwrap();
    assert_eq!(parsed["evaluated_result"], "#DIV/0!");

    // 13c. Unknown function in formula
    let res = call_tool("oneoffice.calc.eval", &json!({
        "formula": "=UNRECOGNIZED_FUNCTION_CALL(123)"
    })).expect("unknown function must evaluate to #NAME? without panic");
    let parsed: serde_json::Value = serde_json::from_str(&res.content[0].text).unwrap();
    assert_eq!(parsed["evaluated_result"], "#NAME?");

    // ------------------------------------------------------------------------
    // RED-MCP-14: visual.verify_page Hostile Inputs
    // ------------------------------------------------------------------------
    // Missing format
    let err_res = call_tool("oneoffice.visual.verify_page", &json!({}));
    assert!(matches!(err_res, Err(McpError::InvalidArguments { .. })));

    // Unsupported format
    let err_res = call_tool("oneoffice.visual.verify_page", &json!({
        "format": "xlsx",
        "data_base64": "dummy"
    }));
    assert!(matches!(err_res, Err(McpError::InvalidArguments { .. })));
}
