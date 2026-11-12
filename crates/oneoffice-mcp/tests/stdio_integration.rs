#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::io::Cursor;
use oneoffice_mcp::McpServer;
use serde_json::{json, Value};

#[test]
fn test_stdio_e2e_jsonrpc_pipeline() {
    let requests = [
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "oneoffice.diagram.compose",
                "arguments": {
                    "template_id": "problem-comparison-to-solution"
                }
            }
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "oneoffice.table.upsert",
                "arguments": {
                    "spec": {
                        "sheet_name": "연간예산",
                        "cells": [
                            [{"value": "항목"}, {"value": "금액"}],
                            [{"value": "클라우드"}, {"value": "10000000"}],
                            [{"value": "보안솔루션"}, {"value": "5000000"}],
                            [{"value": "합계"}, {"value": "=SUM(B2:B3)"}]
                        ]
                    }
                }
            }
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "oneoffice.chart.upsert",
                "arguments": {
                    "spec": {
                        "kind": "ColumnClustered",
                        "categories": ["Q1", "Q2", "Q3", "Q4"],
                        "series": [{
                            "name": "실적",
                            "values": [10.0, 25.0, 40.0, 75.0]
                        }],
                        "width_pt": 400.0,
                        "height_pt": 250.0
                    }
                }
            }
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "tools/call",
            "params": {
                "name": "oneoffice.doc.create",
                "arguments": {
                    "format": "hwpx",
                    "title": "전자정부 표준 HWPX 문서",
                    "paragraphs": ["공공기관 제출용 첨부 문서입니다."],
                    "table": [["구분", "금액"], ["서버", "10000000"]]
                }
            }
        }),
    ];

    let mut input_buffer = Vec::new();
    for req in &requests {
        let line = serde_json::to_string(req).unwrap();
        input_buffer.extend_from_slice(line.as_bytes());
        input_buffer.push(b'\n');
    }

    let reader = Cursor::new(input_buffer);
    let mut writer = Vec::new();

    let server = McpServer::new();
    server.run_stdio(reader, &mut writer).expect("stdio loop failed");

    let output_str = String::from_utf8(writer).expect("invalid utf8 output");
    let lines: Vec<&str> = output_str.trim().split('\n').filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), requests.len());

    // 1. Verify initialize response
    let r1: Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(r1["id"], 1);
    assert_eq!(r1["result"]["serverInfo"]["name"], "oneoffice-mcp");

    // 2. Verify tools/list response
    let r2: Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(r2["id"], 2);
    let tools = r2["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 7);

    // 3. Verify diagram.compose response
    let r3: Value = serde_json::from_str(lines[2]).unwrap();
    assert_eq!(r3["id"], 3);
    let c3: Value = serde_json::from_str(r3["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(c3["success"], true);
    assert_eq!(c3["diagnostics"]["overflow_count"], 0);

    // 4. Verify table.upsert response
    let r4: Value = serde_json::from_str(lines[3]).unwrap();
    assert_eq!(r4["id"], 4);
    let c4: Value = serde_json::from_str(r4["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(c4["display_cells"][3][1], "15000000");

    // 5. Verify chart.upsert response
    let r5: Value = serde_json::from_str(lines[4]).unwrap();
    assert_eq!(r5["id"], 5);
    let c5: Value = serde_json::from_str(r5["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(c5["success"], true);
    assert!(c5["svg"].as_str().unwrap().contains("<svg"));

    // 6. Verify doc.create response
    let r6: Value = serde_json::from_str(lines[5]).unwrap();
    assert_eq!(r6["id"], 6);
    let c6: Value = serde_json::from_str(r6["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(c6["format"], "hwpx");
    assert_eq!(c6["has_table"], true);
    assert!(c6["size_bytes"].as_u64().unwrap() > 4000);
}
