//! Compound objects in KS X 6101 HWPX: Excel calculation table sidecars,
//! OWPML `<hp:container>` proposal infographics, and template field replacement.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use oneoffice_bridge::calc::{CalcTableEngine, CalcTableOutput, CalcTableSpec};
use oneoffice_diagram::spec::DiagramSpec;
use oneoffice_diagram::{compile_owpml, solve_layout};

use crate::error::HwpxError;

/// An embedded Excel calculation table sidecar linked to an HWPX `<hp:tbl>`.
#[derive(Clone, Debug, PartialEq)]
pub struct HwpxTableFormulaSidecar {
    /// Stable table identifier (e.g. `"tbl-budget-2026"`).
    pub table_id: String,
    /// 513-function Excel formula table specification.
    pub spec: CalcTableSpec,
    /// FIPS 180-4 SHA-256 digest of evaluated display cells for staleness reconciliation.
    pub rendered_cells_sha256: String,
}

impl HwpxTableFormulaSidecar {
    /// Creates a new formula table sidecar specification.
    #[must_use]
    pub fn new(table_id: impl Into<String>, spec: CalcTableSpec) -> Self {
        Self {
            table_id: table_id.into(),
            spec,
            rendered_cells_sha256: String::new(),
        }
    }

    /// Evaluates all formulas, produces lossless `.xlsx` bytes, and updates `rendered_cells_sha256`.
    pub fn evaluate(&mut self) -> Result<CalcTableOutput, HwpxError> {
        let output = CalcTableEngine::evaluate_spec(&self.spec)
            .map_err(|e| HwpxError::Compound(format!("Table calculation failed: {e}")))?;
        self.rendered_cells_sha256 = output.rendered_cells_sha256.clone();
        Ok(output)
    }

    /// Checks if external edits in the HWP document have caused the displayed table cells to diverge
    /// from the evaluated formula results.
    #[must_use]
    pub fn is_stale(&self, current_cells_sha256: &str) -> bool {
        !self.rendered_cells_sha256.is_empty()
            && self.rendered_cells_sha256 != current_cells_sha256
    }
}

/// Generates valid KS X 6101 `<hp:tbl>` XML for an evaluated [`CalcTableOutput`].
#[must_use]
pub fn build_hwpx_table_xml(output: &CalcTableOutput, table_id: &str) -> String {
    let row_count = output.display_cells.len();
    let col_count = output
        .display_cells
        .first()
        .map_or(0, |row| row.len());

    let mut xml = String::with_capacity(4096);
    // 1 pt = 100 HWPUNITs. Default cell width ~ 100 pt (10,000 HWPUNITs), height ~ 24 pt (2,400 HWPUNITs)
    let col_width_hwp = 9000i32;
    let row_height_hwp = 2400i32;
    let total_width_hwp = (col_count as i32) * col_width_hwp;

    let _ = writeln!(
        xml,
        r#"<hp:tbl id="{table_id}" zOrder="0" treatAsChar="1" rowCnt="{row_count}" colCnt="{col_count}">"#
    );
    let _ = writeln!(xml, r#"  <hp:sz width="{total_width_hwp}" height="0"/>"#);
    let _ = writeln!(xml, r#"  <hp:pos treatAsChar="1"><hp:point x="0" y="0"/></hp:pos>"#);

    for (r_idx, row) in output.display_cells.iter().enumerate() {
        let _ = writeln!(xml, r#"  <hp:tr>"#);
        for (c_idx, val) in row.iter().enumerate() {
            let escaped = escape_xml(val);
            let _ = writeln!(
                xml,
                r#"    <hp:tc hasMargin="1">"#
            );
            let _ = writeln!(
                xml,
                r#"      <hp:cellAddr rowAddr="{r_idx}" colAddr="{c_idx}"/>"#
            );
            let _ = writeln!(
                xml,
                r#"      <hp:cellSpan colSpan="1" rowSpan="1"/>"#
            );
            let _ = writeln!(
                xml,
                r#"      <hp:cellSz width="{col_width_hwp}" height="{row_height_hwp}"/>"#
            );
            let _ = writeln!(
                xml,
                r#"      <hp:subList>"#
            );
            let _ = writeln!(
                xml,
                r#"        <hp:p id="0" paraPrIDRef="0" styleIDRef="0" pageBreak="0" columnBreak="0">"#
            );
            let _ = writeln!(
                xml,
                r#"          <hp:run charPrIDRef="0"><hp:t>{escaped}</hp:t></hp:run>"#
            );
            let _ = writeln!(xml, r#"        </hp:p>"#);
            let _ = writeln!(xml, r#"      </hp:subList>"#);
            let _ = writeln!(xml, r#"    </hp:tc>"#);
        }
        let _ = writeln!(xml, r#"  </hp:tr>"#);
    }
    let _ = writeln!(xml, r#"</hp:tbl>"#);
    xml
}

/// Solves diagram constraints and embeds the KS X 6101 `<hp:container>` into the target paragraph
/// of a section XML string.
pub fn embed_diagram_into_section(
    section_xml: &str,
    diagram: &DiagramSpec,
    target_p_idx: usize,
) -> Result<String, HwpxError> {
    let scene = solve_layout(diagram)
        .map_err(|e| HwpxError::Compound(format!("Diagram layout error: {e}")))?;
    let container_xml = compile_owpml(&scene);

    embed_xml_object_into_section(section_xml, &container_xml, target_p_idx)
}

/// Embeds an evaluated Excel calculation table `<hp:tbl>` into the target paragraph of a section XML string.
pub fn embed_table_into_section(
    section_xml: &str,
    output: &CalcTableOutput,
    table_id: &str,
    target_p_idx: usize,
) -> Result<String, HwpxError> {
    let tbl_xml = build_hwpx_table_xml(output, table_id);
    embed_xml_object_into_section(section_xml, &tbl_xml, target_p_idx)
}

fn embed_xml_object_into_section(
    section_xml: &str,
    object_xml: &str,
    target_p_idx: usize,
) -> Result<String, HwpxError> {
    let p_tag_target = format!(r#"<hp:p id="{target_p_idx}""#);
    let p_pos = section_xml.find(&p_tag_target).or_else(|| section_xml.find("<hp:p "));

    if let Some(pos) = p_pos {
        // Find closing </hp:p> of this paragraph
        if let Some(close_pos) = section_xml[pos..].find("</hp:p>") {
            let insert_point = pos + close_pos;
            let wrapped_run = format!("\n    <hp:run charPrIDRef=\"0\">\n      {object_xml}\n    </hp:run>\n  ");
            let mut result = String::with_capacity(section_xml.len() + wrapped_run.len() + 64);
            result.push_str(&section_xml[..insert_point]);
            result.push_str(&wrapped_run);
            result.push_str(&section_xml[insert_point..]);
            return Ok(result);
        }
    }

    // Fallback: append paragraph before </hs:sec>
    if let Some(sec_close) = section_xml.rfind("</hs:sec>") {
        let p_xml = format!(
            "\n  <hp:p id=\"{target_p_idx}\" paraPrIDRef=\"0\" styleIDRef=\"0\" pageBreak=\"0\" columnBreak=\"0\">\n    <hp:run charPrIDRef=\"0\">\n      {object_xml}\n    </hp:run>\n  </hp:p>\n"
        );
        let mut result = String::with_capacity(section_xml.len() + p_xml.len() + 64);
        result.push_str(&section_xml[..sec_close]);
        result.push_str(&p_xml);
        result.push_str(&section_xml[sec_close..]);
        return Ok(result);
    }

    Err(HwpxError::Xml("Could not locate paragraph or section close in section XML".to_string()))
}

/// Replaces template field placeholders (`{{KEY}}` or `<hp:fieldBegin name="KEY"/>...<hp:fieldEnd/>`)
/// in an HWPX section XML string without invalidating XML tags or paragraph structure.
#[must_use]
pub fn replace_fields_in_section(
    section_xml: &str,
    replacements: &BTreeMap<String, String>,
) -> String {
    let mut out = section_xml.to_string();

    for (key, val) in replacements {
        let escaped_val = escape_xml(val);

        // 1. Double curly brace placeholder replacement: {{KEY}}
        let mustache = format!("{{{{{key}}}}}");
        out = out.replace(&mustache, &escaped_val);

        // 2. FieldBegin/End pattern replacement: `<hp:fieldBegin ... name="KEY" .../>...<hp:fieldEnd/>`
        let search_name = format!(r#"name="{key}""#);
        if let Some(name_idx) = out.find(&search_name) {
            // Find fieldBegin tag start and end
            if let Some(_tag_start) = out[..name_idx].rfind("<hp:fieldBegin")
                && let Some(tag_end_rel) = out[name_idx..].find("/>")
            {
                let tag_end = name_idx + tag_end_rel + 2;
                if let Some(end_tag_rel) = out[tag_end..].find("<hp:fieldEnd") {
                    let inner_start = tag_end;
                    let inner_end = tag_end + end_tag_rel;

                    let replacement_run = format!(r#"<hp:run charPrIDRef="0"><hp:t>{escaped_val}</hp:t></hp:run>"#);
                    out.replace_range(inner_start..inner_end, &replacement_run);
                }
            }
        }
    }

    out
}

fn escape_xml(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}
