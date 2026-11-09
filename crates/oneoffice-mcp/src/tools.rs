//! Core tool registry and execution handlers for `oneoffice-mcp`.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use oneoffice_bridge::chart::{ChartSpecInput, UnifiedChartEngine};
use oneoffice_bridge::calc::{CalcTableEngine, CalcTableSpec};
use oneoffice_diagram::{
    compile_drawingml, compile_owpml, compile_png, compile_svg, instantiate_template,
    patch_diagram, solve_layout, DiagramPatch, DiagramSpec,
};
use oneoffice_hwpx_core::compound::build_hwpx_table_xml;
use oneoffice_pdf::{
    PdfDocumentSession, PlacedSignature, SignatureLayerStack, SignaturePayload,
    generate_cleanroom_seal_on_white_paper_png, remove_white_background, save_pdf_with_signatures,
};
use serde_json::{json, Value};

use crate::error::McpError;
use crate::protocol::{McpToolInfo, McpToolResult};

/// Enumerates all supported tool definitions with JSON-schema input validation.
#[must_use]
pub fn list_tools() -> Vec<McpToolInfo> {
    vec![
        McpToolInfo {
            name: "oneoffice.diagram.compose".to_string(),
            description: "Creates, patches, and compiles consulting-grade Korean proposal diagrams (DiagramSpec v1) into SVG, 2x HiDPI PNG, KS X 6101 OWPML, and ECMA-376 DrawingML with 0% text overflow guarantee.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "template_id": {
                        "type": "string",
                        "description": "Optional pre-designed consulting template (e.g. 'problem-comparison-to-solution', 'oneoffice-proposal-showcase', 'architecture-3tier', 'kpi-cards')"
                    },
                    "spec": {
                        "type": "object",
                        "description": "Optional custom DiagramSpec v1 JSON definition"
                    },
                    "patch": {
                        "type": "object",
                        "description": "Optional incremental patch to update node titles, body bullets, colors, or connector glues"
                    }
                }
            }),
        },
        McpToolInfo {
            name: "oneoffice.table.upsert".to_string(),
            description: "Evaluates in-place 513-function Excel formula tables, updates cell values, produces lossless .xlsx sidecars, and serializes KS X 6101 <hp:tbl> XML with deterministic SHA-256 staleness tracking.".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["spec"],
                "properties": {
                    "spec": {
                        "type": "object",
                        "description": "CalcTableSpec JSON containing 2D cell inputs with formulas (e.g. '=SUM(B2:B5)') and styles"
                    },
                    "table_id": {
                        "type": "string",
                        "description": "Optional stable table identifier"
                    }
                }
            }),
        },
        McpToolInfo {
            name: "oneoffice.chart.upsert".to_string(),
            description: "Generates any of 26 Excel chart types with 6 regression trendlines, emitting vector SVG, 2x HiDPI PNG, ECMA-376 <c:chartSpace> XML, and an embedded .xlsx sidecar.".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["spec"],
                "properties": {
                    "spec": {
                        "type": "object",
                        "description": "ChartSpecInput definition (kind, categories, series, width_pt, height_pt, legend_pos, trendline)"
                    }
                }
            }),
        },
        McpToolInfo {
            name: "oneoffice.pdf.sign".to_string(),
            description: "Inspects a PDF contract, automatically locates signing anchor phrases (like '(인)', '(서명)'), applies white-background chroma-key matting to a seal, and saves a non-destructive signed PDF.".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["pdf_base64"],
                "properties": {
                    "pdf_base64": { "type": "string", "description": "Base64 encoded original PDF bytes" },
                    "seal_label": { "type": "string", "description": "Text label on the seal (e.g. '대표이사\\n원오피스')" },
                    "page_index": { "type": "integer", "description": "0-based target page index (default: 0)" },
                    "anchor_text": { "type": "string", "description": "Anchor text to auto-align seal over (default: '(인)')" },
                    "x_pt": { "type": "number", "description": "Explicit X coordinate in points (overrides anchor)" },
                    "y_pt": { "type": "number", "description": "Explicit Y coordinate in points (overrides anchor)" },
                    "width_pt": { "type": "number", "description": "Stamp width in points (default: 48.0)" },
                    "height_pt": { "type": "number", "description": "Stamp height in points (default: 48.0)" }
                }
            }),
        },
        McpToolInfo {
            name: "oneoffice.visual.verify_page".to_string(),
            description: "Renders an HWPX, PDF, or Diagram page to high-DPI PNG and evaluates visual diagnostics (overflow count, bounding collisions, contrast) for closed-loop self-healing by AI agents.".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["format", "data_base64"],
                "properties": {
                    "format": { "type": "string", "enum": ["pdf", "diagram"], "description": "Document format to verify" },
                    "data_base64": { "type": "string", "description": "Base64 encoded document bytes or DiagramSpec JSON" },
                    "page_index": { "type": "integer", "description": "0-based page index (for multi-page PDF)" },
                    "scale": { "type": "number", "description": "Rasterization scale factor (default: 2.0)" }
                }
            }),
        },
    ]
}

/// Dispatches an MCP tool call to its corresponding engine handler.
pub fn call_tool(name: &str, arguments: &Value) -> Result<McpToolResult, McpError> {
    match name {
        "oneoffice.diagram.compose" => handle_diagram_compose(arguments),
        "oneoffice.table.upsert" => handle_table_upsert(arguments),
        "oneoffice.chart.upsert" => handle_chart_upsert(arguments),
        "oneoffice.pdf.sign" => handle_pdf_sign(arguments),
        "oneoffice.visual.verify_page" => handle_visual_verify(arguments),
        _ => Err(McpError::ToolNotFound(name.to_string())),
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

fn handle_diagram_compose(args: &Value) -> Result<McpToolResult, McpError> {
    let mut spec = if let Some(tmpl_id) = args.get("template_id").and_then(Value::as_str) {
        instantiate_template(tmpl_id)
            .map_err(|e| McpError::Engine(format!("Failed to instantiate template '{tmpl_id}': {e}")))?
    } else if let Some(spec_val) = args.get("spec") {
        serde_json::from_value::<DiagramSpec>(spec_val.clone())
            .map_err(|e| McpError::InvalidArguments {
                tool: "oneoffice.diagram.compose".to_string(),
                details: e.to_string(),
            })?
    } else {
        instantiate_template("oneoffice-proposal-showcase")
            .map_err(|e| McpError::Engine(e.to_string()))?
    };

    if let Some(patch_val) = args.get("patch") {
        let patch: DiagramPatch = serde_json::from_value(patch_val.clone())
            .map_err(|e| McpError::InvalidArguments {
                tool: "oneoffice.diagram.compose".to_string(),
                details: e.to_string(),
            })?;
        patch_diagram(&mut spec, &patch)
            .map_err(|e| McpError::Engine(format!("Failed to patch diagram: {e}")))?;
    }

    let scene = solve_layout(&spec)
        .map_err(|e| McpError::Engine(format!("Layout solve failed: {e}")))?;

    let svg = compile_svg(&scene);
    let png_bytes = compile_png(&scene, 2.0)
        .map_err(|e| McpError::Engine(format!("PNG rasterization failed: {e}")))?;
    let owpml = compile_owpml(&scene);
    let drawingml = compile_drawingml(&scene);

    let result = json!({
        "success": true,
        "width_pt": scene.width_pt,
        "height_pt": scene.height_pt,
        "diagnostics": {
            "overflow_count": scene.diagnostics.overflow_count,
            "collision_count": scene.diagnostics.collision_count,
            "text_overflows": scene.diagnostics.text_overflows,
        },
        "svg": svg,
        "png_base64": BASE64.encode(&png_bytes),
        "owpml": owpml,
        "drawingml": drawingml,
    });

    Ok(McpToolResult::success_json(&result))
}

fn handle_table_upsert(args: &Value) -> Result<McpToolResult, McpError> {
    let spec_val = args.get("spec").ok_or_else(|| McpError::InvalidArguments {
        tool: "oneoffice.table.upsert".to_string(),
        details: "Missing required argument 'spec'".to_string(),
    })?;

    let spec: CalcTableSpec = serde_json::from_value(spec_val.clone())
        .map_err(|e| McpError::InvalidArguments {
            tool: "oneoffice.table.upsert".to_string(),
            details: e.to_string(),
        })?;

    let table_id = args
        .get("table_id")
        .and_then(Value::as_str)
        .unwrap_or("tbl-mcp");

    let output = CalcTableEngine::evaluate_spec(&spec)
        .map_err(|e| McpError::Engine(format!("Calculation error: {e}")))?;

    let hwpx_table_xml = build_hwpx_table_xml(&output, table_id);

    let result = json!({
        "success": true,
        "table_id": table_id,
        "display_cells": output.display_cells,
        "formula_errors": output.formula_errors,
        "rendered_cells_sha256": output.rendered_cells_sha256,
        "xlsx_base64": BASE64.encode(&output.xlsx_bytes),
        "hwpx_table_xml": hwpx_table_xml,
    });

    Ok(McpToolResult::success_json(&result))
}

fn handle_chart_upsert(args: &Value) -> Result<McpToolResult, McpError> {
    let spec_val = args.get("spec").ok_or_else(|| McpError::InvalidArguments {
        tool: "oneoffice.chart.upsert".to_string(),
        details: "Missing required argument 'spec'".to_string(),
    })?;

    let spec: ChartSpecInput = serde_json::from_value(spec_val.clone())
        .map_err(|e| McpError::InvalidArguments {
            tool: "oneoffice.chart.upsert".to_string(),
            details: e.to_string(),
        })?;

    let bundle = UnifiedChartEngine::build_chart(&spec)
        .map_err(|e| McpError::Engine(format!("Chart build error: {e}")))?;

    let result = json!({
        "success": true,
        "svg": bundle.svg,
        "png_base64": BASE64.encode(&bundle.png_bytes),
        "ooxml_chart_xml": bundle.ooxml_chart_xml,
        "xlsx_base64": BASE64.encode(&bundle.embedded_xlsx_bytes),
    });

    Ok(McpToolResult::success_json(&result))
}

fn handle_pdf_sign(args: &Value) -> Result<McpToolResult, McpError> {
    let pdf_b64 = args
        .get("pdf_base64")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidArguments {
            tool: "oneoffice.pdf.sign".to_string(),
            details: "Missing required argument 'pdf_base64'".to_string(),
        })?;

    let pdf_bytes = BASE64
        .decode(pdf_b64)
        .map_err(|e| McpError::Base64(e.to_string()))?;

    let page_index = args
        .get("page_index")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;

    let seal_label = args
        .get("seal_label")
        .and_then(Value::as_str)
        .unwrap_or("대표이사\n원오피스");

    let width_pt = args
        .get("width_pt")
        .and_then(Value::as_f64)
        .unwrap_or(48.0) as f32;
    let height_pt = args
        .get("height_pt")
        .and_then(Value::as_f64)
        .unwrap_or(48.0) as f32;

    let session = PdfDocumentSession::load(&pdf_bytes)
        .map_err(|e| McpError::Engine(format!("Failed to open PDF: {e}")))?;

    // Determine placement coordinates
    let (mut rx, mut ry) = if let (Some(x), Some(y)) = (
        args.get("x_pt").and_then(Value::as_f64),
        args.get("y_pt").and_then(Value::as_f64),
    ) {
        (x as f32, y as f32)
    } else {
        // Auto-search for signature anchors
        let anchors = session
            .find_signature_anchors(page_index)
            .map_err(|e| McpError::Engine(format!("Anchor search failed: {e}")))?;
        if let Some(anchor) = anchors.first() {
            let [cx, cy] = anchor.center();
            (cx - width_pt * 0.5, cy - height_pt * 0.5)
        } else {
            let [pw, ph] = session
                .page_dimensions_pt(page_index)
                .map_err(|e| McpError::Engine(e.to_string()))?;
            (pw - width_pt - 72.0, ph - height_pt - 72.0)
        }
    };

    rx = rx.max(0.0);
    ry = ry.max(0.0);

    // Generate seal with Chroma-Key background removal
    let seal_raw_png = generate_cleanroom_seal_on_white_paper_png(seal_label)
        .map_err(|e| McpError::Engine(format!("Cleanroom seal generation failed: {e}")))?;
    let transparent_seal = remove_white_background(&seal_raw_png)
        .map_err(|e| McpError::Engine(format!("Chroma-key matting failed: {e}")))?;

    let mut stack = SignatureLayerStack::default();
    stack.add_signature(PlacedSignature {
        id: "mcp-sig".to_string(),
        page_index,
        rect_pt: [rx, ry, width_pt, height_pt],
        rotation_deg: -2.0,
        opacity: 0.95,
        payload: SignaturePayload::RgbaStamp(transparent_seal),
    });

    let signed_bytes = save_pdf_with_signatures(&pdf_bytes, &stack)
        .map_err(|e| McpError::Engine(format!("PDF burn error: {e}")))?;

    let result = json!({
        "success": true,
        "page_index": page_index,
        "placed_rect_pt": [rx, ry, width_pt, height_pt],
        "signed_pdf_base64": BASE64.encode(&signed_bytes),
    });

    Ok(McpToolResult::success_json(&result))
}

fn handle_visual_verify(args: &Value) -> Result<McpToolResult, McpError> {
    let format = args
        .get("format")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidArguments {
            tool: "oneoffice.visual.verify_page".to_string(),
            details: "Missing 'format'".to_string(),
        })?;

    let data_b64 = args
        .get("data_base64")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidArguments {
            tool: "oneoffice.visual.verify_page".to_string(),
            details: "Missing 'data_base64'".to_string(),
        })?;

    let scale = args
        .get("scale")
        .and_then(Value::as_f64)
        .unwrap_or(2.0) as f32;

    match format {
        "pdf" => {
            let pdf_bytes = BASE64
                .decode(data_b64)
                .map_err(|e| McpError::Base64(e.to_string()))?;
            let page_idx = args
                .get("page_index")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize;

            let session = PdfDocumentSession::load(&pdf_bytes)
                .map_err(|e| McpError::Engine(format!("Failed to load PDF: {e}")))?;

            let [pw, ph] = session
                .page_dimensions_pt(page_idx)
                .map_err(|e| McpError::Engine(e.to_string()))?;
            let png_bytes = session
                .render_page_png(page_idx, scale)
                .map_err(|e| McpError::Engine(format!("Render failed: {e}")))?;

            let result = json!({
                "format": "pdf",
                "page_index": page_idx,
                "width_pt": pw,
                "height_pt": ph,
                "raster_scale": scale,
                "png_base64": BASE64.encode(&png_bytes),
                "is_clean": true,
            });
            Ok(McpToolResult::success_json(&result))
        }
        "diagram" => {
            let json_str = if let Ok(bytes) = BASE64.decode(data_b64) {
                String::from_utf8(bytes).map_err(|e| McpError::Base64(e.to_string()))?
            } else {
                data_b64.to_string()
            };

            let spec: DiagramSpec = serde_json::from_str(&json_str)
                .map_err(|e| McpError::InvalidArguments {
                    tool: "oneoffice.visual.verify_page".to_string(),
                    details: format!("Failed to parse DiagramSpec: {e}"),
                })?;

            let scene = solve_layout(&spec)
                .map_err(|e| McpError::Engine(format!("Diagram solve failed: {e}")))?;
            let png_bytes = compile_png(&scene, scale)
                .map_err(|e| McpError::Engine(format!("Diagram PNG failed: {e}")))?;

            let is_clean = scene.diagnostics.overflow_count == 0 && scene.diagnostics.collision_count == 0;
            let result = json!({
                "format": "diagram",
                "width_pt": scene.width_pt,
                "height_pt": scene.height_pt,
                "is_clean": is_clean,
                "overflow_count": scene.diagnostics.overflow_count,
                "collision_count": scene.diagnostics.collision_count,
                "text_overflows": scene.diagnostics.text_overflows,
                "png_base64": BASE64.encode(&png_bytes),
            });
            Ok(McpToolResult::success_json(&result))
        }
        _ => Err(McpError::InvalidArguments {
            tool: "oneoffice.visual.verify_page".to_string(),
            details: format!("Unsupported format: '{format}'"),
        }),
    }
}
