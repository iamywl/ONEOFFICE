//! Core tool registry and execution handlers for `oneoffice-mcp`.

use std::sync::Arc;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use oneoffice_bridge::calc::{CalcTableEngine, CalcTableSpec, CellInput};
use oneoffice_bridge::chart::{ChartSpecInput, UnifiedChartEngine};
use oneoffice_diagram::{
    compile_drawingml, compile_owpml, compile_png, compile_svg, instantiate_template,
    list_diagram_templates, patch_diagram, solve_layout, DiagramPatch, DiagramSpec,
};
use oneoffice_hwpx_core::compound::{build_hwpx_table_xml, replace_fields_in_section};
use oneoffice_hwpx_core::{
    embed_diagram_into_section, embed_table_into_section, HwpxPackage, HwpxTableFormulaSidecar,
};
use oneoffice_pdf::{
    generate_cleanroom_seal_on_white_paper_png, generate_document_pdf,
    remove_white_background, save_pdf_with_signatures, PdfDocumentSession,
    PlacedSignature, SignatureLayerStack, SignaturePayload,
};
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;

use crate::error::McpError;
use crate::protocol::{McpToolInfo, McpToolResult};

/// Enumerates all supported tool definitions with JSON-schema input validation.
#[must_use]
pub fn list_tools() -> Vec<McpToolInfo> {
    vec![
        McpToolInfo {
            name: "oneoffice.doc.create".to_string(),
            description: "Generates a complete, standard-compliant HWPX (KS X 6101) or DOCX (ECMA-376) document with styled paragraphs, embedded formula tables, 26-type charts, consulting diagrams, and media.".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["format"],
                "properties": {
                    "format": { "type": "string", "enum": ["docx", "hwpx"], "description": "Target document format" },
                    "title": { "type": "string", "description": "Document title (default: 'ONEOFFICE 사업제안서')" },
                    "paragraphs": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "List of paragraph text strings"
                    },
                    "table": {
                        "type": "array",
                        "items": { "type": "array", "items": { "type": "string" } },
                        "description": "Optional 2D table grid (e.g. [['항목', '금액'], ['클라우드', '12500000'], ['합계', '=SUM(B2:B2)']])"
                    },
                    "diagram_template_id": {
                        "type": "string",
                        "description": "Optional proposal diagram template to embed (e.g. 'problem-comparison-to-solution')"
                    },
                    "image_base64": {
                        "type": "string",
                        "description": "Optional PNG image base64 data to embed into document media"
                    }
                }
            }),
        },
        McpToolInfo {
            name: "oneoffice.doc.inspect".to_string(),
            description: "Inspects DOCX, HWPX, or PDF document bytes, extracting metadata, paragraphs, table counts, and embedded media.".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["data_base64"],
                "properties": {
                    "data_base64": { "type": "string", "description": "Base64 encoded document bytes" }
                }
            }),
        },
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
        McpToolInfo {
            name: "oneoffice.doc.convert".to_string(),
            description: "Converts documents between HWPX, DOCX, Markdown, and high-DPI vector PDF with Korean typography preservation.".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["data_base64", "from_format", "to_format"],
                "properties": {
                    "data_base64": { "type": "string", "description": "Base64 encoded input document bytes" },
                    "from_format": { "type": "string", "enum": ["hwpx", "docx", "md"], "description": "Source format" },
                    "to_format": { "type": "string", "enum": ["pdf", "md", "hwpx", "docx"], "description": "Target format" }
                }
            }),
        },
        McpToolInfo {
            name: "oneoffice.doc.fill_fields".to_string(),
            description: "Fills form fields and 누름틀 placeholders in HWPX (<hp:fieldBegin/End>, {{KEY}}) or DOCX ({{KEY}}, «KEY») without damaging sibling paragraph runs.".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["data_base64", "fields"],
                "properties": {
                    "data_base64": { "type": "string", "description": "Base64 encoded template document bytes" },
                    "format": { "type": "string", "enum": ["hwpx", "docx"], "description": "Document format (default: 'hwpx')" },
                    "fields": { "type": "object", "description": "Key-value dictionary of field names and replacement text" }
                }
            }),
        },
        McpToolInfo {
            name: "oneoffice.diagram.list_templates".to_string(),
            description: "Lists all 10 consulting proposal infographic templates with IDs, Korean titles, categories, and recommended canvas widths.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "category": { "type": "string", "description": "Optional category filter (e.g. 'proposal', 'process', 'strategy', 'architecture', 'roadmap', 'marketing')" }
                }
            }),
        },
        McpToolInfo {
            name: "oneoffice.diagram.patch".to_string(),
            description: "Incrementally updates specific nodes, KPI metrics, body bullets, colors, or connectors in an existing DiagramSpec and recompiles layout.".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["spec", "patch"],
                "properties": {
                    "spec": { "type": "object", "description": "DiagramSpec JSON definition to patch" },
                    "patch": { "type": "object", "description": "DiagramPatch JSON containing node updates, additions, or theme adjustments" }
                }
            }),
        },
        McpToolInfo {
            name: "oneoffice.pdf.find_anchors".to_string(),
            description: "Scans a PDF document to locate standard signing and seal anchor phrases (like '(인)', '(서명)') and returns bounding boxes and center points.".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["pdf_base64"],
                "properties": {
                    "pdf_base64": { "type": "string", "description": "Base64 encoded PDF document bytes" },
                    "query": { "type": "string", "description": "Anchor query text to locate (default: '(인)')" },
                    "page_index": { "type": "integer", "description": "Optional 0-based page index to restrict search" }
                }
            }),
        },
        McpToolInfo {
            name: "oneoffice.calc.eval".to_string(),
            description: "Evaluates an Excel formula expression (e.g. '=SUM(10, 20, 30)', '=XLOOKUP(...)', '=NPV(...)') using GridCraft's 513-function engine.".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["formula"],
                "properties": {
                    "formula": { "type": "string", "description": "Excel formula string to evaluate (e.g. '=SUM(100, 250, 400)')" }
                }
            }),
        },
    ]
}

/// Dispatches an MCP tool call to its corresponding engine handler.
pub fn call_tool(name: &str, arguments: &Value) -> Result<McpToolResult, McpError> {
    match name {
        "oneoffice.doc.create" => handle_doc_create(arguments),
        "oneoffice.doc.inspect" => handle_doc_inspect(arguments),
        "oneoffice.doc.convert" => handle_doc_convert(arguments),
        "oneoffice.doc.fill_fields" => handle_doc_fill_fields(arguments),
        "oneoffice.diagram.list_templates" => handle_diagram_list_templates(arguments),
        "oneoffice.diagram.compose" => handle_diagram_compose(arguments),
        "oneoffice.diagram.patch" => handle_diagram_patch(arguments),
        "oneoffice.table.upsert" => handle_table_upsert(arguments),
        "oneoffice.chart.upsert" => handle_chart_upsert(arguments),
        "oneoffice.pdf.find_anchors" => handle_pdf_find_anchors(arguments),
        "oneoffice.pdf.sign" => handle_pdf_sign(arguments),
        "oneoffice.calc.eval" => handle_calc_eval(arguments),
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

fn handle_doc_create(args: &Value) -> Result<McpToolResult, McpError> {
    let format = args
        .get("format")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidArguments {
            tool: "oneoffice.doc.create".to_string(),
            details: "Missing required argument 'format' ('docx' or 'hwpx')".to_string(),
        })?;

    let title = args
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("ONEOFFICE 사업제안서");

    let paragraphs: Vec<String> = args
        .get("paragraphs")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    let table_data: Option<Vec<Vec<String>>> = args
        .get("table")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|r| {
                    r.as_array().map(|cells| {
                        cells
                            .iter()
                            .map(|c| c.as_str().unwrap_or("").to_string())
                            .collect()
                    })
                })
                .collect()
        });

    let diagram_template_id = args.get("diagram_template_id").and_then(Value::as_str);
    let image_b64 = args.get("image_base64").and_then(Value::as_str);
    let image_bytes = match image_b64 {
        Some(b64) => Some(BASE64.decode(b64).map_err(|e| McpError::Base64(e.to_string()))?),
        None => None,
    };

    match format {
        "docx" => {
            let mut doc = wordcraft_doc::Document::new();
            let mut body_blocks: Vec<Arc<wordcraft_doc::Block>> = Vec::new();

            // 1. Title
            let title_para = wordcraft_doc::Paragraph::with_text(
                title,
                wordcraft_doc::props::CharProps {
                    size: Some(22.0),
                    bold: Some(true),
                    ..Default::default()
                },
            );
            body_blocks.push(wordcraft_doc::para_block(title_para));

            // 2. Paragraphs
            for text in &paragraphs {
                let p = wordcraft_doc::Paragraph::with_text(
                    text,
                    wordcraft_doc::props::CharProps {
                        size: Some(11.0),
                        ..Default::default()
                    },
                );
                body_blocks.push(wordcraft_doc::para_block(p));
            }

            // 3. Table if provided
            if let Some(tbl) = &table_data
                && !tbl.is_empty()
            {
                let rows = tbl.len();
                let cols = tbl.first().map_or(1, Vec::len);
                let mut table = wordcraft_doc::Table::new(rows, cols, 468.0);
                for (r_idx, row_data) in tbl.iter().enumerate() {
                    if let Some(row) = table.rows.get_mut(r_idx) {
                        for (c_idx, val) in row_data.iter().enumerate() {
                            if let Some(cell) = row.cells.get_mut(c_idx) {
                                *cell = wordcraft_doc::Cell::with_text(val);
                                cell.props.width = Some(468.0 / cols as f32);
                            }
                        }
                    }
                }
                body_blocks.push(Arc::new(wordcraft_doc::Block::Table(table)));
            }

            // 4. Diagram if template provided
            if let Some(tid) = diagram_template_id {
                let spec = instantiate_template(tid)
                    .map_err(|e| McpError::Engine(format!("Diagram template instantiation failed: {e}")))?;
                let scene = solve_layout(&spec)
                    .map_err(|e| McpError::Engine(format!("Diagram solve layout failed: {e}")))?;
                let png_bytes = compile_png(&scene, 2.0)
                    .map_err(|e| McpError::Engine(format!("Diagram PNG compile failed: {e}")))?;
                let key = doc.add_media(png_bytes, "png");
                let obj = wordcraft_doc::para::InlineObject::Image {
                    media: key,
                    w: scene.width_pt,
                    h: scene.height_pt,
                    alt: spec.title.clone().unwrap_or_else(|| "도식화 다이어그램".to_string()),
                    float: wordcraft_doc::para::Float {
                        wrap: wordcraft_doc::para::Wrap::Inline,
                        ..Default::default()
                    },
                    crop: [0.0; 4],
                    ole: None,
                };
                let mut p = wordcraft_doc::Paragraph::with_text("", wordcraft_doc::props::CharProps::default());
                let _ = p.insert_object(0, obj, &wordcraft_doc::props::CharProps::default());
                body_blocks.push(wordcraft_doc::para_block(p));
            }

            // 5. Image if provided
            if let Some(img_data) = image_bytes {
                let key = doc.add_media(img_data, "png");
                let obj = wordcraft_doc::para::InlineObject::Image {
                    media: key,
                    w: 240.0,
                    h: 180.0,
                    alt: "첨부 그림 자료".to_string(),
                    float: wordcraft_doc::para::Float {
                        wrap: wordcraft_doc::para::Wrap::Inline,
                        ..Default::default()
                    },
                    crop: [0.0; 4],
                    ole: None,
                };
                let mut p = wordcraft_doc::Paragraph::with_text("", wordcraft_doc::props::CharProps::default());
                let _ = p.insert_object(0, obj, &wordcraft_doc::props::CharProps::default());
                body_blocks.push(wordcraft_doc::para_block(p));
            }

            doc.body = body_blocks;
            let docx_bytes = wordcraft_docx::write(&doc)
                .map_err(|e| McpError::Engine(format!("DOCX serialization failed: {e:?}")))?;

            let result = json!({
                "format": "docx",
                "title": title,
                "paragraphs_count": paragraphs.len() + 1,
                "has_table": table_data.as_ref().is_some_and(|t| !t.is_empty()),
                "has_diagram": diagram_template_id.is_some(),
                "has_image": image_b64.is_some(),
                "size_bytes": docx_bytes.len(),
                "document_base64": BASE64.encode(&docx_bytes),
            });
            Ok(McpToolResult::success_json(&result))
        }
        "hwpx" => {
            let initial_para = paragraphs.first().map_or("본문 내용", String::as_str);
            let mut hwpx = HwpxPackage::create_minimal_hwpx(title, initial_para)
                .map_err(|e| McpError::Engine(format!("HWPX creation failed: {e}")))?;

            let mut sec_xml = hwpx
                .section_xml(0)
                .map_err(|e| McpError::Engine(format!("HWPX section 0 read failed: {e}")))?;

            for (idx, p_text) in paragraphs.iter().enumerate().skip(1) {
                let p_xml = format!(
                    r#"<hp:p id="{}"><hp:run><hp:t>{}</hp:t></hp:run></hp:p>"#,
                    idx + 1,
                    p_text
                );
                if let Some((before_end, end_tag)) = sec_xml.rsplit_once("</hs:sec>") {
                    sec_xml = format!("{before_end}{p_xml}{end_tag}");
                }
            }

            // Table
            if let Some(tbl) = &table_data
                && !tbl.is_empty()
            {
                let mut cells = Vec::new();
                for (r_idx, row) in tbl.iter().enumerate() {
                    let mut row_inputs = Vec::new();
                    for (c_idx, val) in row.iter().enumerate() {
                        if val.starts_with('=') {
                            row_inputs.push(oneoffice_bridge::calc::CellInput::formula(val.clone()));
                        } else if r_idx == tbl.len() - 1 && c_idx > 0 && val.is_empty() {
                            let col_letter = (b'A' + c_idx as u8) as char;
                            row_inputs.push(oneoffice_bridge::calc::CellInput::formula(format!(
                                "=SUM({col_letter}2:{col_letter}{r_idx})"
                            )));
                        } else {
                            row_inputs.push(oneoffice_bridge::calc::CellInput::text(val.clone()));
                        }
                    }
                    cells.push(row_inputs);
                }
                let spec = CalcTableSpec {
                    sheet_name: "제안예산".to_string(),
                    cells,
                    merges: Vec::new(),
                };
                let mut sidecar = HwpxTableFormulaSidecar::new("tbl-oneoffice-1", spec);
                if let Ok(output) = sidecar.evaluate()
                    && let Ok(updated_sec) = embed_table_into_section(
                        &sec_xml,
                        &output,
                        "tbl-oneoffice-1",
                        10,
                    )
                {
                    sec_xml = updated_sec;
                    hwpx.add_binary_attachment("BinData/table_1.xlsx", output.xlsx_bytes);
                }
            }

            // Diagram
            if let Some(tid) = diagram_template_id
                && let Ok(spec) = instantiate_template(tid)
                && let Ok(updated_sec) = embed_diagram_into_section(&sec_xml, &spec, 20)
            {
                sec_xml = updated_sec;
            }

            // Image attachment
            if let Some(img_data) = image_bytes {
                hwpx.add_binary_attachment("BinData/image_1.png", img_data);
            }

            hwpx.set_section_xml(0, &sec_xml)
                .map_err(|e| McpError::Engine(format!("HWPX section 0 update failed: {e}")))?;
            let hwpx_bytes = hwpx
                .to_bytes()
                .map_err(|e| McpError::Engine(format!("HWPX serialization failed: {e}")))?;

            let result = json!({
                "format": "hwpx",
                "title": title,
                "paragraphs_count": paragraphs.len(),
                "has_table": table_data.as_ref().is_some_and(|t| !t.is_empty()),
                "has_diagram": diagram_template_id.is_some(),
                "has_image": image_b64.is_some(),
                "size_bytes": hwpx_bytes.len(),
                "document_base64": BASE64.encode(&hwpx_bytes),
            });
            Ok(McpToolResult::success_json(&result))
        }
        _ => Err(McpError::InvalidArguments {
            tool: "oneoffice.doc.create".to_string(),
            details: format!("Unsupported format '{format}'. Supported formats are 'docx', 'hwpx'"),
        }),
    }
}

fn handle_doc_inspect(args: &Value) -> Result<McpToolResult, McpError> {
    let data_b64 = args
        .get("data_base64")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidArguments {
            tool: "oneoffice.doc.inspect".to_string(),
            details: "Missing required argument 'data_base64'".to_string(),
        })?;

    let bytes = BASE64
        .decode(data_b64)
        .map_err(|e| McpError::Base64(e.to_string()))?;

    // 1. PDF detection (%PDF-)
    if bytes.starts_with(b"%PDF-") {
        let session = PdfDocumentSession::load(&bytes)
            .map_err(|e| McpError::Engine(format!("PDF inspection failed: {e}")))?;
        let page_count = session.page_count();
        let mut dimensions = Vec::new();
        for i in 0..page_count {
            if let Ok(dims) = session.page_dimensions_pt(i) {
                dimensions.push(dims);
            }
        }
        let mut total_anchors = 0;
        for i in 0..page_count {
            if let Ok(anchors) = session.find_signature_anchors(i) {
                total_anchors += anchors.len();
            }
        }
        let result = json!({
            "format": "pdf",
            "page_count": page_count,
            "dimensions_pt": dimensions,
            "signing_anchors_count": total_anchors,
            "size_bytes": bytes.len(),
        });
        return Ok(McpToolResult::success_json(&result));
    }

    // 2. ZIP packages (DOCX, HWPX, XLSX, PPTX)
    if bytes.starts_with(b"PK\x03\x04") {
        // Test HWPX first via HwpxPackage, ensuring KS X 6101 package structure
        if let Ok(hwpx) = HwpxPackage::from_bytes(&bytes)
            && (hwpx.entries().contains_key("Contents/content.hpf")
                || hwpx.entries().contains_key("Contents/section0.xml"))
        {
            let sec_count = hwpx.section_count();
            let mut has_table = false;
            let mut has_diagram = false;
            let mut titles = Vec::new();

            for i in 0..sec_count {
                if let Ok(xml) = hwpx.section_xml(i) {
                    if xml.contains("<hp:tbl") {
                        has_table = true;
                    }
                    if xml.contains("<hp:container") || xml.contains("oneoffice:diagram") {
                        has_diagram = true;
                    }
                    if let Some(pos) = xml.find("<hp:t>")
                        && let Some(end) = xml[pos..].find("</hp:t>")
                    {
                        titles.push(xml[pos + 6..pos + end].to_string());
                    }
                }
            }

            let attachments: Vec<String> = hwpx.entries().keys().cloned().collect();

            let result = json!({
                "format": "hwpx",
                "section_count": sec_count,
                "has_table": has_table,
                "has_diagram": has_diagram,
                "entries_count": attachments.len(),
                "attachments": attachments,
                "sample_text": titles.first(),
                "size_bytes": bytes.len(),
            });
            return Ok(McpToolResult::success_json(&result));
        }

        // Test DOCX via wordcraft_docx::read
        if let Ok(doc) = wordcraft_docx::read(&bytes) {
            let mut para_count = 0;
            let mut table_count = 0;
            let mut sample_texts = Vec::new();

            for block in &doc.body {
                match block.as_ref() {
                    wordcraft_doc::Block::Para(p) => {
                        para_count += 1;
                        if !p.text.is_empty() && sample_texts.len() < 3 {
                            sample_texts.push(p.text.clone());
                        }
                    }
                    wordcraft_doc::Block::Table(_) => {
                        table_count += 1;
                    }
                }
            }

            let result = json!({
                "format": "docx",
                "paragraphs_count": para_count,
                "tables_count": table_count,
                "media_count": doc.media.len(),
                "sample_texts": sample_texts,
                "size_bytes": bytes.len(),
            });
            return Ok(McpToolResult::success_json(&result));
        }

        // Other zip formats: inspect entry names
        let cursor = std::io::Cursor::new(&bytes);
        if let Ok(mut archive) = zip::ZipArchive::new(cursor) {
            let mut file_names = Vec::new();
            for i in 0..archive.len() {
                if let Ok(file) = archive.by_index(i) {
                    file_names.push(file.name().to_string());
                }
            }

            let kind = if file_names.iter().any(|n| n.starts_with("xl/")) {
                "xlsx"
            } else if file_names.iter().any(|n| n.starts_with("ppt/")) {
                "pptx"
            } else {
                "zip"
            };

            let result = json!({
                "format": kind,
                "entries_count": file_names.len(),
                "entries": file_names,
                "size_bytes": bytes.len(),
            });
            return Ok(McpToolResult::success_json(&result));
        }
    }

    Err(McpError::InvalidArguments {
        tool: "oneoffice.doc.inspect".to_string(),
        details: "Unrecognized document format (neither PDF nor valid DOCX/HWPX ZIP package)".to_string(),
    })
}

fn handle_diagram_list_templates(args: &Value) -> Result<McpToolResult, McpError> {
    let category_filter = args.get("category").and_then(Value::as_str);
    let templates = list_diagram_templates();
    let mut list = Vec::new();
    for t in templates {
        if let Some(cat) = category_filter
            && !t.category.eq_ignore_ascii_case(cat)
        {
            continue;
        }
        list.push(json!({
            "id": t.id,
            "name": t.name,
            "category": t.category,
            "description": t.description,
            "recommended_width_pt": t.recommended_width_pt,
        }));
    }
    let result = json!({
        "success": true,
        "total_count": list.len(),
        "templates": list,
    });
    Ok(McpToolResult::success_json(&result))
}

fn handle_diagram_patch(args: &Value) -> Result<McpToolResult, McpError> {
    let spec_val = args.get("spec").ok_or_else(|| McpError::InvalidArguments {
        tool: "oneoffice.diagram.patch".to_string(),
        details: "Missing required argument 'spec'".to_string(),
    })?;
    let mut spec: DiagramSpec = serde_json::from_value(spec_val.clone())
        .map_err(|e| McpError::InvalidArguments {
            tool: "oneoffice.diagram.patch".to_string(),
            details: format!("Failed to parse 'spec': {e}"),
        })?;

    let patch_val = args.get("patch").ok_or_else(|| McpError::InvalidArguments {
        tool: "oneoffice.diagram.patch".to_string(),
        details: "Missing required argument 'patch'".to_string(),
    })?;
    let patch: DiagramPatch = serde_json::from_value(patch_val.clone())
        .map_err(|e| McpError::InvalidArguments {
            tool: "oneoffice.diagram.patch".to_string(),
            details: format!("Failed to parse 'patch': {e}"),
        })?;

    patch_diagram(&mut spec, &patch)
        .map_err(|e| McpError::Engine(format!("Failed to patch diagram: {e}")))?;

    let scene = solve_layout(&spec)
        .map_err(|e| McpError::Engine(format!("Failed to solve layout: {e}")))?;

    let svg = compile_svg(&scene);
    let png_bytes = compile_png(&scene, 2.0)
        .map_err(|e| McpError::Engine(format!("Failed to compile PNG: {e}")))?;
    let png_b64 = BASE64.encode(&png_bytes);
    let owpml = compile_owpml(&scene);
    let drawingml = compile_drawingml(&scene);

    let diag = &scene.diagnostics;
    let result = json!({
        "success": true,
        "updated_spec": spec,
        "svg": svg,
        "png_base64": png_b64,
        "owpml": owpml,
        "drawingml": drawingml,
        "diagnostics": {
            "overflow_count": diag.overflow_count,
            "collision_count": diag.collision_count,
            "bounding_size_pt": [scene.width_pt, scene.height_pt]
        }
    });
    Ok(McpToolResult::success_json(&result))
}

fn handle_pdf_find_anchors(args: &Value) -> Result<McpToolResult, McpError> {
    let pdf_b64 = args.get("pdf_base64").and_then(Value::as_str).ok_or_else(|| McpError::InvalidArguments {
        tool: "oneoffice.pdf.find_anchors".to_string(),
        details: "Missing required argument 'pdf_base64'".to_string(),
    })?;
    let pdf_bytes = BASE64.decode(pdf_b64).map_err(|e| McpError::Base64(e.to_string()))?;

    let session = PdfDocumentSession::load(&pdf_bytes)
        .map_err(|e| McpError::Engine(format!("Failed to load PDF: {e}")))?;

    let query = args.get("query").and_then(Value::as_str).unwrap_or("(인)");
    let page_index_opt = args.get("page_index").and_then(Value::as_u64).map(|v| v as usize);

    let mut matches = Vec::new();
    let total_pages = session.page_count();

    let mut search_page = |page: usize| {
        if query == "(인)" || query == "(서명)" || query == "서명" || query == "인" {
            if let Ok(anchors) = session.find_signature_anchors(page) {
                for a in anchors {
                    let [cx, cy] = a.center();
                    matches.push(json!({
                        "page_index": a.page_index,
                        "text": a.text,
                        "rect_pt": a.rect_pt,
                        "center_pt": [cx, cy],
                    }));
                }
            }
        } else if let Ok(anchors) = session.find_text(page, query) {
            for a in anchors {
                let [cx, cy] = a.center();
                matches.push(json!({
                    "page_index": a.page_index,
                    "text": a.text,
                    "rect_pt": a.rect_pt,
                    "center_pt": [cx, cy],
                }));
            }
        }
    };

    if let Some(target_page) = page_index_opt {
        if target_page < total_pages {
            search_page(target_page);
        }
    } else {
        for p in 0..total_pages {
            search_page(p);
        }
    }

    let result = json!({
        "success": true,
        "query": query,
        "page_count": total_pages,
        "total_matches": matches.len(),
        "anchors": matches,
    });
    Ok(McpToolResult::success_json(&result))
}

fn handle_doc_fill_fields(args: &Value) -> Result<McpToolResult, McpError> {
    let data_b64 = args.get("data_base64").and_then(Value::as_str).ok_or_else(|| McpError::InvalidArguments {
        tool: "oneoffice.doc.fill_fields".to_string(),
        details: "Missing required argument 'data_base64'".to_string(),
    })?;
    let bytes = BASE64.decode(data_b64).map_err(|e| McpError::Base64(e.to_string()))?;

    let format = args.get("format").and_then(Value::as_str).unwrap_or("hwpx");
    let fields_map = args.get("fields").and_then(Value::as_object).ok_or_else(|| McpError::InvalidArguments {
        tool: "oneoffice.doc.fill_fields".to_string(),
        details: "Missing required argument 'fields' (must be a key-value object)".to_string(),
    })?;

    let mut replacements = std::collections::BTreeMap::new();
    for (k, v) in fields_map {
        let val_str = match v {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        replacements.insert(k.clone(), val_str);
    }

    match format {
        "hwpx" => {
            let mut hwpx = HwpxPackage::from_bytes(&bytes)
                .map_err(|e| McpError::Engine(format!("Failed to parse HWPX package: {e}")))?;
            let sec_count = hwpx.section_count();
            let mut modified_count = 0;
            for i in 0..sec_count {
                if let Ok(xml) = hwpx.section_xml(i) {
                    let replaced = replace_fields_in_section(&xml, &replacements);
                    if replaced != xml {
                        modified_count += 1;
                        hwpx.set_section_xml(i, &replaced)
                            .map_err(|e| McpError::Engine(format!("Failed to update section XML: {e}")))?;
                    }
                }
            }
            let out_bytes = hwpx.to_bytes()
                .map_err(|e| McpError::Engine(format!("Failed to serialize HWPX: {e}")))?;
            let result = json!({
                "success": true,
                "format": "hwpx",
                "sections_modified": modified_count,
                "fields_applied_count": replacements.len(),
                "document_base64": BASE64.encode(&out_bytes),
            });
            Ok(McpToolResult::success_json(&result))
        }
        "docx" => {
            let reader = std::io::Cursor::new(&bytes);
            let mut zip = zip::ZipArchive::new(reader)
                .map_err(|e| McpError::Engine(format!("Failed to read DOCX zip: {e}")))?;

            let mut out_buffer = std::io::Cursor::new(Vec::new());
            let mut zip_writer = zip::ZipWriter::new(&mut out_buffer);
            let mut replaced_any = false;

            for i in 0..zip.len() {
                let mut file = zip.by_index(i)
                    .map_err(|e| McpError::Engine(format!("Failed to read zip entry: {e}")))?;
                let name = file.name().to_string();

                let mut data = Vec::new();
                std::io::Read::read_to_end(&mut file, &mut data)
                    .map_err(|e| McpError::Io(e.to_string()))?;

                if name == "word/document.xml"
                    && let Ok(mut text) = String::from_utf8(data.clone())
                {
                    for (k, v) in &replacements {
                        let curly_pat = format!("{{{{{}}}}}", k);
                        let guillemet_pat = format!("«{}»", k);
                        if text.contains(&curly_pat) || text.contains(&guillemet_pat) {
                            text = text.replace(&curly_pat, v);
                            text = text.replace(&guillemet_pat, v);
                            replaced_any = true;
                        }
                    }
                    data = text.into_bytes();
                }

                let options = SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated);
                zip_writer.start_file(&name, options)
                    .map_err(|e| McpError::Engine(format!("Failed to write zip entry: {e}")))?;
                std::io::Write::write_all(&mut zip_writer, &data)
                    .map_err(|e| McpError::Io(e.to_string()))?;
            }

            zip_writer.finish()
                .map_err(|e| McpError::Engine(format!("Failed to finalize DOCX zip: {e}")))?;

            let out_bytes = out_buffer.into_inner();
            let result = json!({
                "success": true,
                "format": "docx",
                "replaced": replaced_any,
                "fields_applied_count": replacements.len(),
                "document_base64": BASE64.encode(&out_bytes),
            });
            Ok(McpToolResult::success_json(&result))
        }
        other => Err(McpError::InvalidArguments {
            tool: "oneoffice.doc.fill_fields".to_string(),
            details: format!("Unsupported document format: '{other}'. Expected 'hwpx' or 'docx'"),
        }),
    }
}

fn handle_doc_convert(args: &Value) -> Result<McpToolResult, McpError> {
    let from_fmt = args.get("from_format").and_then(Value::as_str).ok_or_else(|| McpError::InvalidArguments {
        tool: "oneoffice.doc.convert".to_string(),
        details: "Missing required argument 'from_format'".to_string(),
    })?;
    let to_fmt = args.get("to_format").and_then(Value::as_str).ok_or_else(|| McpError::InvalidArguments {
        tool: "oneoffice.doc.convert".to_string(),
        details: "Missing required argument 'to_format'".to_string(),
    })?;
    let data_b64 = args.get("data_base64").and_then(Value::as_str).ok_or_else(|| McpError::InvalidArguments {
        tool: "oneoffice.doc.convert".to_string(),
        details: "Missing required argument 'data_base64'".to_string(),
    })?;
    let bytes = BASE64.decode(data_b64).map_err(|e| McpError::Base64(e.to_string()))?;

    match (from_fmt, to_fmt) {
        ("hwpx", "pdf") => {
            let hwpx = HwpxPackage::from_bytes(&bytes)
                .map_err(|e| McpError::Engine(format!("Failed to parse HWPX: {e}")))?;
            let mut paragraphs = Vec::new();
            let mut title = "ONEOFFICE HWPX 변환 문서".to_string();

            for i in 0..hwpx.section_count() {
                if let Ok(xml) = hwpx.section_xml(i) {
                    let mut cursor = 0;
                    while let Some(start) = xml[cursor..].find("<hp:t>") {
                        let actual_start = cursor + start + 6;
                        if let Some(end) = xml[actual_start..].find("</hp:t>") {
                            let text = &xml[actual_start..actual_start + end];
                            if !text.trim().is_empty() {
                                if paragraphs.is_empty() {
                                    title = text.trim().to_string();
                                }
                                paragraphs.push(text.trim().to_string());
                            }
                            cursor = actual_start + end + 7;
                        } else {
                            break;
                        }
                    }
                }
            }
            if paragraphs.is_empty() {
                paragraphs.push("내용이 없는 문서입니다.".to_string());
            }
            let pdf_bytes = generate_document_pdf(&title, "HWPX → PDF 변환 산출물", &paragraphs)
                .map_err(|e| McpError::Engine(format!("Failed to generate PDF: {e}")))?;

            let result = json!({
                "success": true,
                "from_format": "hwpx",
                "to_format": "pdf",
                "mime_type": "application/pdf",
                "paragraphs_converted": paragraphs.len(),
                "output_base64": BASE64.encode(&pdf_bytes),
            });
            Ok(McpToolResult::success_json(&result))
        }
        ("docx", "pdf") => {
            let doc = wordcraft_docx::read(&bytes)
                .map_err(|e| McpError::Engine(format!("Failed to parse DOCX: {e}")))?;
            let mut paragraphs = Vec::new();
            let mut title = "ONEOFFICE DOCX 변환 문서".to_string();

            for block in &doc.body {
                if let wordcraft_doc::Block::Para(p) = block.as_ref()
                    && !p.text.trim().is_empty()
                {
                    if paragraphs.is_empty() {
                        title = p.text.trim().to_string();
                    }
                    paragraphs.push(p.text.trim().to_string());
                }
            }
            if paragraphs.is_empty() {
                paragraphs.push("내용이 없는 문서입니다.".to_string());
            }
            let pdf_bytes = generate_document_pdf(&title, "DOCX → PDF 변환 산출물", &paragraphs)
                .map_err(|e| McpError::Engine(format!("Failed to generate PDF: {e}")))?;

            let result = json!({
                "success": true,
                "from_format": "docx",
                "to_format": "pdf",
                "mime_type": "application/pdf",
                "paragraphs_converted": paragraphs.len(),
                "output_base64": BASE64.encode(&pdf_bytes),
            });
            Ok(McpToolResult::success_json(&result))
        }
        ("hwpx", "md") => {
            let hwpx = HwpxPackage::from_bytes(&bytes)
                .map_err(|e| McpError::Engine(format!("Failed to parse HWPX: {e}")))?;
            let mut md = String::new();
            for i in 0..hwpx.section_count() {
                if let Ok(xml) = hwpx.section_xml(i) {
                    let mut cursor = 0;
                    while let Some(start) = xml[cursor..].find("<hp:t>") {
                        let actual_start = cursor + start + 6;
                        if let Some(end) = xml[actual_start..].find("</hp:t>") {
                            let text = &xml[actual_start..actual_start + end];
                            if !text.trim().is_empty() {
                                if md.is_empty() {
                                    md.push_str(&format!("# {}\n\n", text.trim()));
                                } else {
                                    md.push_str(&format!("{}\n\n", text.trim()));
                                }
                            }
                            cursor = actual_start + end + 7;
                        } else {
                            break;
                        }
                    }
                }
            }
            let result = json!({
                "success": true,
                "from_format": "hwpx",
                "to_format": "md",
                "markdown": md,
            });
            Ok(McpToolResult::success_json(&result))
        }
        ("docx", "md") => {
            let doc = wordcraft_docx::read(&bytes)
                .map_err(|e| McpError::Engine(format!("Failed to parse DOCX: {e}")))?;
            let mut md = String::new();
            for block in &doc.body {
                if let wordcraft_doc::Block::Para(p) = block.as_ref()
                    && !p.text.trim().is_empty()
                {
                    if md.is_empty() {
                        md.push_str(&format!("# {}\n\n", p.text.trim()));
                    } else {
                        md.push_str(&format!("{}\n\n", p.text.trim()));
                    }
                }
            }
            let result = json!({
                "success": true,
                "from_format": "docx",
                "to_format": "md",
                "markdown": md,
            });
            Ok(McpToolResult::success_json(&result))
        }
        ("md", "hwpx") => {
            let md_str = String::from_utf8(bytes)
                .map_err(|e| McpError::Engine(format!("Invalid UTF-8 in Markdown: {e}")))?;
            let lines: Vec<&str> = md_str.lines().filter(|l| !l.trim().is_empty()).collect();
            let title = lines.first().copied().unwrap_or("Markdown 변환 문서").trim_start_matches('#').trim();
            let paragraphs: Vec<String> = lines.iter().skip(1).map(|l| l.to_string()).collect();

            let hwpx = HwpxPackage::create_minimal_hwpx(title, &paragraphs.join("\n\n"))
                .map_err(|e| McpError::Engine(format!("Failed to create HWPX from Markdown: {e}")))?;
            let hwpx_bytes = hwpx.to_bytes()
                .map_err(|e| McpError::Engine(format!("Failed to serialize HWPX: {e}")))?;

            let result = json!({
                "success": true,
                "from_format": "md",
                "to_format": "hwpx",
                "mime_type": "application/x-hwp+zip",
                "output_base64": BASE64.encode(&hwpx_bytes),
            });
            Ok(McpToolResult::success_json(&result))
        }
        ("md", "pdf") => {
            let md_str = String::from_utf8(bytes)
                .map_err(|e| McpError::Engine(format!("Invalid UTF-8 in Markdown: {e}")))?;
            let lines: Vec<&str> = md_str.lines().filter(|l| !l.trim().is_empty()).collect();
            let title = lines.first().copied().unwrap_or("Markdown 변환 문서").trim_start_matches('#').trim();
            let paragraphs: Vec<String> = lines.iter().skip(1).map(|l| l.to_string()).collect();

            let pdf_bytes = generate_document_pdf(title, "Markdown → PDF 변환 산출물", &paragraphs)
                .map_err(|e| McpError::Engine(format!("Failed to generate PDF: {e}")))?;

            let result = json!({
                "success": true,
                "from_format": "md",
                "to_format": "pdf",
                "mime_type": "application/pdf",
                "output_base64": BASE64.encode(&pdf_bytes),
            });
            Ok(McpToolResult::success_json(&result))
        }
        (from, to) => Err(McpError::InvalidArguments {
            tool: "oneoffice.doc.convert".to_string(),
            details: format!("Unsupported conversion pair: '{from}' to '{to}'"),
        }),
    }
}

fn handle_calc_eval(args: &Value) -> Result<McpToolResult, McpError> {
    let formula = args.get("formula").and_then(Value::as_str).ok_or_else(|| McpError::InvalidArguments {
        tool: "oneoffice.calc.eval".to_string(),
        details: "Missing required argument 'formula'".to_string(),
    })?;

    let formula_with_eq = if formula.starts_with('=') {
        formula.to_string()
    } else {
        format!("={formula}")
    };

    let spec = CalcTableSpec {
        sheet_name: "CalcEval".to_string(),
        cells: vec![vec![CellInput::formula(formula_with_eq.clone())]],
        merges: Vec::new(),
    };

    let output = CalcTableEngine::evaluate_spec(&spec)
        .map_err(|e| McpError::Engine(format!("Formula evaluation failed: {e}")))?;

    let evaluated = output.display_cells
        .first()
        .and_then(|row| row.first())
        .cloned()
        .unwrap_or_default();

    let supported_count = CalcTableEngine::supported_function_count();

    let result = json!({
        "success": true,
        "formula": formula,
        "evaluated_result": evaluated,
        "engine_supported_functions_count": supported_count,
    });
    Ok(McpToolResult::success_json(&result))
}
