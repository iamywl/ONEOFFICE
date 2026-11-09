//! Unified 513-function Excel calculation engine, `.xlsx` lossless sidecar serializer, and
//! external literal-edit reconciliation (`CalcTableEngine`).

use std::collections::BTreeSet;
use std::sync::Arc;

use gridcraft_calc::Calc;
use gridcraft_core::{CellRef, DateSystem, MAX_COLS, MAX_ROWS, RangeRef, Value};
use gridcraft_model::{
    Cell, Color, Fill, Formula, HAlign, NumFmt, Sheet, Style, StyleId, Workbook,
};
use gridcraft_numfmt::{NumberFormat, format_value};
use serde::{Deserialize, Serialize};

use crate::error::BridgeError;
use crate::sha256::hash_display_grid;

/// Maximum dense rows materialized into a 2D `CalcTableOutput` grid (`65,536`).
const MAX_DENSE_ROWS: usize = 65_536;
/// Maximum dense columns materialized into a 2D `CalcTableOutput` grid (`1,024`).
const MAX_DENSE_COLS: usize = 1_024;
/// Maximum total cells materialized into a 2D `CalcTableOutput` grid (`1,000,000`) to guard OOM.
const MAX_DENSE_CELLS: usize = 1_000_000;

/// Single cell specification for [`CalcTableSpec`].
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct CellInput {
    /// Cell literal text/number or Excel formula starting with `'='`
    /// (e.g. `"=SUM(B2:B5)"`, `"=AVERAGE(B2:E2)"`, `"=IF(C2>100,\"달성\",\"미달\")"`).
    #[serde(default, alias = "raw", alias = "input")]
    pub value: String,
    /// Optional Excel number format code (e.g. `"#,##0"`, `"0.0%"`, `"₩#,##0"`).
    #[serde(default)]
    pub number_format: Option<String>,
    /// Whether the cell text is bold.
    #[serde(default)]
    pub bold: bool,
    /// Optional cell background hex color (e.g. `"#F1F5F9"` or `"F1F5F9"`).
    #[serde(default)]
    pub bg_hex: Option<String>,
    /// Optional horizontal alignment (`"left"`, `"center"`, `"right"`, `"justify"`).
    #[serde(default)]
    pub align: Option<String>,
}

impl CellInput {
    /// Creates an empty, unstyled cell input.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Creates a text/literal cell input. If `text` begins with `'='`, a leading apostrophe is
    /// prepended so Excel/GridCraft keeps it as literal text rather than a formula.
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        let s = text.into();
        let value = if s.starts_with('=') {
            format!("'{s}")
        } else {
            s
        };
        Self {
            value,
            ..Self::default()
        }
    }

    /// Creates a numeric cell input. Non-finite floats (`NaN` / `±Infinity`) map cleanly to
    /// Excel's `#NUM!` error value without panicking.
    #[must_use]
    pub fn number(n: f64) -> Self {
        let value = if n.is_finite() {
            gridcraft_core::number_to_text(if n == 0.0 { 0.0 } else { n })
        } else {
            "#NUM!".to_string()
        };
        Self {
            value,
            ..Self::default()
        }
    }

    /// Creates a formula cell input, automatically ensuring a leading `'='`.
    #[must_use]
    pub fn formula(formula: impl Into<String>) -> Self {
        let raw = formula.into();
        let trimmed = raw.trim();
        let value = if trimmed.starts_with('=') {
            trimmed.to_string()
        } else {
            format!("={trimmed}")
        };
        Self {
            value,
            ..Self::default()
        }
    }

    /// Attaches an Excel number format code (e.g. `"#,##0"`, `"0.0%"`, `"₩#,##0"`).
    #[must_use]
    pub fn with_number_format(mut self, fmt: impl Into<String>) -> Self {
        self.number_format = Some(fmt.into());
        self
    }

    /// Alias for [`Self::with_number_format`].
    #[must_use]
    pub fn with_format(self, fmt: impl Into<String>) -> Self {
        self.with_number_format(fmt)
    }

    /// Sets bold font styling on this cell.
    #[must_use]
    pub fn with_bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }

    /// Sets solid background fill color from a hex code (`"#RRGGBB"`).
    #[must_use]
    pub fn with_bg_hex(mut self, bg_hex: impl Into<String>) -> Self {
        self.bg_hex = Some(bg_hex.into());
        self
    }

    /// Sets horizontal alignment (`"left"`, `"center"`, `"right"`, `"justify"`).
    #[must_use]
    pub fn with_align(mut self, align: impl Into<String>) -> Self {
        self.align = Some(align.into());
        self
    }
}

/// Rectangular inclusive 0-based cell merge span (`start_row..=end_row`, `start_col..=end_col`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MergeSpan {
    /// Zero-based top row index.
    pub start_row: u32,
    /// Zero-based left column index.
    pub start_col: u32,
    /// Zero-based bottom row index (inclusive).
    pub end_row: u32,
    /// Zero-based right column index (inclusive).
    pub end_col: u32,
}

impl MergeSpan {
    /// Creates a new 0-based inclusive merge span.
    #[must_use]
    pub const fn new(start_row: u32, start_col: u32, end_row: u32, end_col: u32) -> Self {
        Self {
            start_row,
            start_col,
            end_row,
            end_col,
        }
    }
}

/// Input specification for constructing and evaluating an in-place Excel calculation table.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct CalcTableSpec {
    /// Worksheet tab name (sanitized to Excel's 1..=31 char rules when empty/invalid).
    pub sheet_name: String,
    /// Row-major 2D grid of cell inputs.
    pub cells: Vec<Vec<CellInput>>,
    /// Rectangular cell merge spans.
    #[serde(default)]
    pub merges: Vec<MergeSpan>,
}

/// Diagnostic entry for any cell whose formula/evaluation yielded an Excel error (`#DIV/0!`, etc.).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct FormulaDiagnostic {
    /// Zero-based row index.
    pub row: u32,
    /// Zero-based column index.
    pub col: u32,
    /// `A1`-style cell address (e.g. `"C4"`).
    pub address: String,
    /// Excel error code string (e.g. `"#DIV/0!"`, `"#VALUE!"`, `"#NAME?"`, `"#CIRC!"`).
    pub error_code: String,
}

/// Evaluated calculation table output with formatted display strings, raw numbers, formulas,
/// error diagnostics, lossless `.xlsx` sidecar bytes, and deterministic SHA-256 digest.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CalcTableOutput {
    /// Display strings formatted through `gridcraft-numfmt`.
    pub display_cells: Vec<Vec<String>>,
    /// Raw finite `f64` numeric results (`None` for text, booleans, blanks, or errors).
    pub raw_numbers: Vec<Vec<Option<f64>>>,
    /// Canonical Excel formula strings with leading `'='` (`None` for literal cells).
    pub formulas: Vec<Vec<Option<String>>>,
    /// Diagnostics for cells holding an Excel error value.
    pub formula_errors: Vec<FormulaDiagnostic>,
    /// Valid `.xlsx` workbook bytes preserving formulas, styles, number formats, and merges.
    pub xlsx_bytes: Vec<u8>,
    /// Deterministic FIPS 180-4 SHA-256 hex digest over `display_cells`.
    pub rendered_cells_sha256: String,
}

/// Result of reconciling external literal edits (from Hancom HWPX / MS Word) against an `.xlsx`
/// sidecar while preserving all formula cells.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ReconcileResult {
    /// Recalculated table output and updated `.xlsx` sidecar bytes.
    pub output: CalcTableOutput,
    /// Count of non-formula cells whose literal values were updated externally.
    pub modified_literal_count: usize,
}

/// Stateless entry point for 513-function Excel table evaluation, `.xlsx` round-tripping, and
/// external literal reconciliation.
#[derive(Clone, Copy, Debug, Default)]
pub struct CalcTableEngine;

impl CalcTableEngine {
    /// Returns all supported Excel worksheet function names (combining `gridcraft-functions`
    /// and `gridcraft-calc::SPECIAL_FUNCTIONS`, sorted and deduplicated).
    #[must_use]
    pub fn supported_function_names() -> Vec<&'static str> {
        let mut set = BTreeSet::new();
        for spec in gridcraft_functions::all() {
            set.insert(spec.name);
        }
        for &special in gridcraft_calc::SPECIAL_FUNCTIONS {
            set.insert(special);
        }
        set.into_iter().collect()
    }

    /// Returns the total count of supported Excel worksheet functions (`>= 513`).
    #[must_use]
    pub fn supported_function_count() -> usize {
        Self::supported_function_names().len()
    }

    /// Builds a `Workbook` from `spec`, evaluates all formulas via `gridcraft-calc`, formats cells
    /// via `gridcraft-numfmt`, and serializes lossless `.xlsx` sidecar bytes via `gridcraft-xlsx`.
    pub fn evaluate_spec(spec: &CalcTableSpec) -> Result<CalcTableOutput, BridgeError> {
        let spec_rows = spec.cells.len();
        let spec_cols = spec.cells.iter().map(Vec::len).max().unwrap_or(0);
        check_dense_bounds(spec_rows, spec_cols)?;

        let mut wb = Workbook::new();
        let sheet_name = sanitize_sheet_name(&spec.sheet_name);
        wb.sheets = vec![Arc::new(Sheet::new(sheet_name))];

        // Validate and convert merge spans first.
        let mut merge_ranges: Vec<RangeRef> = Vec::with_capacity(spec.merges.len());
        for m in &spec.merges {
            if m.start_row > m.end_row || m.start_col > m.end_col {
                return Err(BridgeError::InvalidMergeSpan(format!(
                    "start ({}, {}) exceeds end ({}, {})",
                    m.start_row, m.start_col, m.end_row, m.end_col
                )));
            }
            if m.end_row >= MAX_ROWS || m.end_col >= MAX_COLS {
                return Err(BridgeError::InvalidMergeSpan(format!(
                    "span ({}, {})..=({}, {}) exceeds Excel sheet limits",
                    m.start_row, m.start_col, m.end_row, m.end_col
                )));
            }
            if m.start_row == m.end_row && m.start_col == m.end_col {
                continue;
            }
            let r = RangeRef::new(
                CellRef::new(m.start_row, m.start_col),
                CellRef::new(m.end_row, m.end_col),
            );
            if merge_ranges.iter().any(|existing| existing.intersects(&r)) {
                return Err(BridgeError::InvalidMergeSpan(format!(
                    "overlapping merge span at {}",
                    r.a1()
                )));
            }
            merge_ranges.push(r);
        }

        // Populate cells and intern styles.
        let date_sys = wb.date_system;
        let mut staged_cells: Vec<(CellRef, Cell)> = Vec::new();
        for (r_idx, row) in spec.cells.iter().enumerate() {
            for (c_idx, input) in row.iter().enumerate() {
                let cell_ref = CellRef::new(r_idx as u32, c_idx as u32);
                let (cell_val, formula_opt, inferred_fmt) =
                    parse_cell_input_value(&input.value, date_sys);

                let style_id = build_style_id(&mut wb, input, inferred_fmt);
                let cell = match formula_opt {
                    Some(f) => Cell {
                        value: Value::Empty,
                        formula: Some(Arc::new(f)),
                        style: style_id,
                    },
                    None => Cell {
                        value: cell_val,
                        formula: None,
                        style: style_id,
                    },
                };
                if !cell.is_blank() {
                    staged_cells.push((cell_ref, cell));
                }
            }
        }

        if let Some(sheet) = wb.sheet_mut(0) {
            sheet.merges = merge_ranges;
            for (c_ref, cell) in staged_cells {
                sheet.set_cell(c_ref, cell);
            }
        }

        let mut calc = Calc::new();
        calc.recalc_all(&mut wb);

        extract_workbook_output(&wb, 0, spec_rows, spec_cols)
    }

    /// Reads `.xlsx` bytes via `gridcraft-xlsx`, recalculates all formulas via `gridcraft-calc`,
    /// and returns the evaluated [`CalcTableOutput`] for `sheet_index`.
    pub fn evaluate_xlsx_bytes(
        xlsx_bytes: &[u8],
        sheet_index: usize,
    ) -> Result<CalcTableOutput, BridgeError> {
        let (mut wb, _report) = gridcraft_xlsx::read_xlsx(xlsx_bytes)?;
        if sheet_index >= wb.sheets.len() {
            return Err(BridgeError::SheetOutOfBounds {
                requested: sheet_index,
                sheet_count: wb.sheets.len(),
            });
        }
        let mut calc = Calc::new();
        calc.recalc_all(&mut wb);

        extract_workbook_output(&wb, sheet_index, 0, 0)
    }

    /// Reconciles external display-grid edits (e.g. literal edits made inside Hancom HWPX or Word
    /// tables) back into the active sheet (index `0`) of `xlsx_bytes`:
    /// - Preserves every formula cell (and dynamic-array spill children) intact.
    /// - Detects literal cells whose formatted display text changed, parses numeric/currency/percent
    ///   strings (`"1,250"`, `"1250"`, `"₩1,500"`, `"15.5%"`) into numbers, updates the `Workbook`,
    ///   recalculates dependents via `gridcraft-calc`, and emits updated `.xlsx` bytes.
    pub fn reconcile_external_literals(
        xlsx_bytes: &[u8],
        updated_display_grid: &[Vec<String>],
    ) -> Result<ReconcileResult, BridgeError> {
        let ext_rows = updated_display_grid.len();
        let ext_cols = updated_display_grid.iter().map(Vec::len).max().unwrap_or(0);
        check_dense_bounds(ext_rows, ext_cols)?;

        let (mut wb, _report) = gridcraft_xlsx::read_xlsx(xlsx_bytes)?;
        if wb.sheets.is_empty() {
            return Err(BridgeError::SheetOutOfBounds {
                requested: 0,
                sheet_count: 0,
            });
        }

        // First calculate current display strings so untouched formatted numbers (e.g. "₩1,250")
        // compare equal against `updated_display_grid` without false-positive edits.
        let mut calc = Calc::new();
        calc.recalc_all(&mut wb);

        let date_sys = wb.date_system;
        let mut candidates = Vec::new();

        if let Some(sheet) = wb.sheet(0) {
            for (r_idx, row) in updated_display_grid.iter().enumerate() {
                for (c_idx, new_display) in row.iter().enumerate() {
                    let c_ref = CellRef::new(r_idx as u32, c_idx as u32);
                    // Preserve formula cells and dynamic-array spill targets untouched!
                    let is_formula = sheet
                        .cell(c_ref)
                        .and_then(|cell| cell.formula.as_ref())
                        .is_some();
                    if is_formula || sheet.spill_anchor(c_ref).is_some() {
                        continue;
                    }

                    let style_id = sheet.style_id(c_ref);
                    let cur_code = wb.styles.get(style_id).num_fmt.as_str().to_string();
                    let cur_val = sheet.value(c_ref);
                    candidates.push((c_ref, style_id, cur_code, cur_val, (*new_display).clone()));
                }
            }
        }

        let mut pending_updates: Vec<(CellRef, Value, StyleId)> = Vec::new();
        for (c_ref, style_id, cur_code, cur_val, new_display) in candidates {
            let cur_display =
                format_value(&cur_val, &NumberFormat::parse_cached(&cur_code), date_sys).text;

            if new_display == cur_display {
                continue;
            }

            let (parsed_val, inferred_fmt) = parse_literal_string(&new_display, date_sys);
            let next_style = if cur_code.eq_ignore_ascii_case("general")
                && let Some(fmt_code) = inferred_fmt
            {
                wb.styles
                    .derive(style_id, |s| s.num_fmt = NumFmt::new(fmt_code))
            } else {
                style_id
            };
            pending_updates.push((c_ref, parsed_val, next_style));
        }

        let modified_literal_count = pending_updates.len();
        if modified_literal_count > 0 {
            if let Some(sheet) = wb.sheet_mut(0) {
                for (c_ref, val, style_id) in pending_updates {
                    sheet.set_cell(
                        c_ref,
                        Cell {
                            value: val,
                            formula: None,
                            style: style_id,
                        },
                    );
                }
            }
            calc.recalc_all(&mut wb);
        }

        let output = extract_workbook_output(&wb, 0, ext_rows, ext_cols)?;
        Ok(ReconcileResult {
            output,
            modified_literal_count,
        })
    }
}

fn check_dense_bounds(rows: usize, cols: usize) -> Result<(), BridgeError> {
    if rows > MAX_DENSE_ROWS
        || cols > MAX_DENSE_COLS
        || rows.saturating_mul(cols) > MAX_DENSE_CELLS
    {
        return Err(BridgeError::GridDimensionsExceeded { rows, cols });
    }
    Ok(())
}

fn sanitize_sheet_name(raw: &str) -> String {
    let cleaned: String = raw
        .trim()
        .chars()
        .map(|c| {
            if matches!(c, ':' | '\\' | '/' | '?' | '*' | '[' | ']') || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('\'').trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("history") {
        return "Sheet1".to_string();
    }
    trimmed.chars().take(31).collect()
}

fn parse_hex_color(hex: &str) -> Option<Color> {
    let trimmed = hex.trim().trim_start_matches('#');
    if trimmed.len() == 3 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        let mut expanded = String::with_capacity(6);
        for ch in trimmed.chars() {
            expanded.push(ch);
            expanded.push(ch);
        }
        return Color::from_hex(&expanded);
    }
    Color::from_hex(trimmed)
}

fn build_style_id(wb: &mut Workbook, input: &CellInput, inferred_fmt: Option<&str>) -> StyleId {
    let mut style = Style::default();
    if input.bold {
        style.font.bold = true;
    }
    if let Some(bg) = input.bg_hex.as_deref()
        && let Some(color) = parse_hex_color(bg)
    {
        style.fill = Fill::solid(color);
    }
    if let Some(align) = input.align.as_deref() {
        style.align.h = match align.trim().to_ascii_lowercase().as_str() {
            "left" | "l" => HAlign::Left,
            "center" | "centre" | "c" | "middle" => HAlign::Center,
            "right" | "r" => HAlign::Right,
            "justify" => HAlign::Justify,
            "distributed" => HAlign::Distributed,
            _ => HAlign::General,
        };
    }
    if let Some(code) = input
        .number_format
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        style.num_fmt = NumFmt::new(code);
    } else if let Some(auto_code) = inferred_fmt {
        style.num_fmt = NumFmt::new(auto_code);
    }
    wb.styles.intern(style)
}

fn parse_cell_input_value(
    raw: &str,
    sys: DateSystem,
) -> (Value, Option<Formula>, Option<&'static str>) {
    if raw.is_empty() {
        return (Value::Empty, None, None);
    }
    if raw.starts_with('=') {
        return (Value::Empty, Some(Formula::new(raw)), None);
    }
    let (val, fmt) = parse_literal_string(raw, sys);
    (val, None, fmt)
}

/// Parses a literal cell string (non-formula), supporting Korean Won (`₩` / `￦`), thousands
/// separators (`"1,250"`), percentages (`"12.5%"`), booleans, dates, and forced-text apostrophes.
fn parse_literal_string(raw: &str, sys: DateSystem) -> (Value, Option<&'static str>) {
    if raw.is_empty() {
        return (Value::Empty, None);
    }
    if let Some(forced_text) = raw.strip_prefix('\'') {
        return (Value::text(forced_text), None);
    }
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return (Value::Empty, None);
    }
    if let Some((won_num, won_fmt)) = parse_korean_won(trimmed, sys) {
        return (Value::number(won_num), Some(won_fmt));
    }
    let parsed = gridcraft_core::parse::parse_input(raw, sys);
    (parsed.value, parsed.format)
}

fn parse_korean_won(trimmed: &str, sys: DateSystem) -> Option<(f64, &'static str)> {
    let (neg, rest) = if let Some(r) = trimmed.strip_prefix('-') {
        (true, r.trim_start())
    } else if let Some(r) = trimmed.strip_prefix('+') {
        (false, r.trim_start())
    } else {
        (false, trimmed)
    };
    let after_symbol = rest
        .strip_prefix('₩')
        .or_else(|| rest.strip_prefix('￦'))?
        .trim_start();
    if after_symbol.is_empty() || after_symbol.ends_with('%') {
        return None;
    }
    let num = gridcraft_core::parse::parse_number_text_in(after_symbol, sys)?;
    if !num.is_finite() {
        return None;
    }
    let signed = if neg { -num } else { num };
    let fmt = if after_symbol.contains('.') {
        "₩#,##0.00"
    } else {
        "₩#,##0"
    };
    Some((signed, fmt))
}

fn extract_workbook_output(
    wb: &Workbook,
    sheet_index: usize,
    min_rows: usize,
    min_cols: usize,
) -> Result<CalcTableOutput, BridgeError> {
    let Some(sheet) = wb.sheet(sheet_index) else {
        return Err(BridgeError::SheetOutOfBounds {
            requested: sheet_index,
            sheet_count: wb.sheets.len(),
        });
    };

    let mut rows = min_rows;
    let mut cols = min_cols;
    if let Some(used) = sheet.used_range() {
        rows = rows.max(used.end.row as usize + 1);
        cols = cols.max(used.end.col as usize + 1);
    }
    for merge in &sheet.merges {
        rows = rows.max(merge.end.row as usize + 1);
        cols = cols.max(merge.end.col as usize + 1);
    }
    check_dense_bounds(rows, cols)?;

    let mut display_cells = Vec::with_capacity(rows);
    let mut raw_numbers = Vec::with_capacity(rows);
    let mut formulas = Vec::with_capacity(rows);
    let mut formula_errors = Vec::new();

    for r in 0..rows {
        let mut disp_row = Vec::with_capacity(cols);
        let mut num_row = Vec::with_capacity(cols);
        let mut fmla_row = Vec::with_capacity(cols);

        for c in 0..cols {
            let c_ref = CellRef::new(r as u32, c as u32);
            let val = sheet.value(c_ref);
            let style = wb.styles.get(sheet.style_id(c_ref));
            let fmt = NumberFormat::parse_cached(style.num_fmt.as_str());
            let formatted = format_value(&val, &fmt, wb.date_system);

            let raw_num = match &val {
                Value::Number(n) if n.is_finite() => Some(*n),
                Value::Array(arr) => arr.data.first().and_then(|v| match v {
                    Value::Number(n) if n.is_finite() => Some(*n),
                    _ => None,
                }),
                _ => None,
            };

            let formula_str = sheet
                .cell(c_ref)
                .and_then(|cell| cell.formula.as_ref())
                .map(|f| format!("={}", f.text));

            if let Some(err) = val.as_error() {
                formula_errors.push(FormulaDiagnostic {
                    row: r as u32,
                    col: c as u32,
                    address: c_ref.a1(),
                    error_code: err.as_str().to_string(),
                });
            }

            disp_row.push(formatted.text);
            num_row.push(raw_num);
            fmla_row.push(formula_str);
        }

        display_cells.push(disp_row);
        raw_numbers.push(num_row);
        formulas.push(fmla_row);
    }

    let xlsx_bytes = gridcraft_xlsx::write_xlsx(wb)?;
    let rendered_cells_sha256 = hash_display_grid(&display_cells);

    Ok(CalcTableOutput {
        display_cells,
        raw_numbers,
        formulas,
        formula_errors,
        xlsx_bytes,
        rendered_cells_sha256,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_exposes_all_513_excel_functions() {
        let count = CalcTableEngine::supported_function_count();
        assert!(
            count >= 513,
            "expected at least 513 Excel functions, found {count}"
        );
        let names = CalcTableEngine::supported_function_names();
        for required in [
            "SUM",
            "AVERAGE",
            "IF",
            "XLOOKUP",
            "VLOOKUP",
            "INDEX",
            "MATCH",
            "SUMIFS",
            "NPV",
            "IRR",
            "LET",
            "LAMBDA",
            "TEXT",
        ] {
            assert!(names.contains(&required), "missing {required}");
        }
    }

    #[test]
    fn korean_proposal_budget_table_evaluates_and_roundtrips_xlsx_losslessly()
    -> Result<(), BridgeError> {
        // Rows:
        // R1 (0): [구분, 2024년, 2025년, 평균, 증감률, 목표판정]
        // R2 (1): [클라우드 인프라, 120, 180, =AVERAGE(B2:C2), =(C2-B2)/B2, =IF(C2>=150,"달성","미달")]
        // R3 (2): [AI 모델 고도화,  80,  140, =AVERAGE(B3:C3), =(C3-B3)/B3, =IF(C3>=150,"달성","미달")]
        // R4 (3): [보안 관제 구축,  50,   80, =AVERAGE(B4:C4), =(C4-B4)/B4, =IF(C4>=150,"달성","미달")]
        // R5 (4): [합계, =SUM(B2:B4), =SUM(C2:C4), =AVERAGE(B5:C5), =(C5-B5)/B5, =IF(C5>=350,"달성","미달")]
        let spec = CalcTableSpec {
            sheet_name: "2026 제안 예산안".to_string(),
            cells: vec![
                vec![
                    CellInput::text("구분").with_bold(true).with_bg_hex("#E2E8F0"),
                    CellInput::text("2024년").with_bold(true).with_align("right"),
                    CellInput::text("2025년").with_bold(true).with_align("right"),
                    CellInput::text("평균").with_bold(true).with_align("right"),
                    CellInput::text("증감률").with_bold(true).with_align("right"),
                    CellInput::text("목표판정").with_bold(true).with_align("center"),
                ],
                vec![
                    CellInput::text("클라우드 인프라"),
                    CellInput::number(120.0).with_number_format("₩#,##0"),
                    CellInput::number(180.0).with_number_format("₩#,##0"),
                    CellInput::formula("=AVERAGE(B2:C2)").with_number_format("₩#,##0"),
                    CellInput::formula("=(C2-B2)/B2").with_number_format("0.0%"),
                    CellInput::formula("=IF(C2>=150,\"달성\",\"미달\")"),
                ],
                vec![
                    CellInput::text("AI 모델 고도화"),
                    CellInput::number(80.0).with_number_format("₩#,##0"),
                    CellInput::number(140.0).with_number_format("₩#,##0"),
                    CellInput::formula("=AVERAGE(B3:C3)").with_number_format("₩#,##0"),
                    CellInput::formula("=(C3-B3)/B3").with_number_format("0.0%"),
                    CellInput::formula("=IF(C3>=150,\"달성\",\"미달\")"),
                ],
                vec![
                    CellInput::text("보안 관제 구축"),
                    CellInput::number(50.0).with_number_format("₩#,##0"),
                    CellInput::number(80.0).with_number_format("₩#,##0"),
                    CellInput::formula("=AVERAGE(B4:C4)").with_number_format("₩#,##0"),
                    CellInput::formula("=(C4-B4)/B4").with_number_format("0.0%"),
                    CellInput::formula("=IF(C4>=150,\"달성\",\"미달\")"),
                ],
                vec![
                    CellInput::text("합계").with_bold(true),
                    CellInput::formula("=SUM(B2:B4)")
                        .with_bold(true)
                        .with_number_format("₩#,##0"),
                    CellInput::formula("=SUM(C2:C4)")
                        .with_bold(true)
                        .with_number_format("₩#,##0"),
                    CellInput::formula("=AVERAGE(B5:C5)")
                        .with_bold(true)
                        .with_number_format("₩#,##0"),
                    CellInput::formula("=(C5-B5)/B5")
                        .with_bold(true)
                        .with_number_format("0.0%"),
                    CellInput::formula("=IF(C5>=350,\"달성\",\"미달\")").with_bold(true),
                ],
            ],
            merges: vec![],
        };

        let out = CalcTableEngine::evaluate_spec(&spec)?;
        assert!(out.formula_errors.is_empty(), "{:?}", out.formula_errors);
        assert_eq!(out.display_cells[1][3], "₩150");
        assert_eq!(out.display_cells[1][4], "50.0%");
        assert_eq!(out.display_cells[1][5], "달성");
        assert_eq!(out.display_cells[2][5], "미달");
        assert_eq!(out.display_cells[4][1], "₩250");
        assert_eq!(out.display_cells[4][2], "₩400");
        assert_eq!(out.display_cells[4][3], "₩325");
        assert_eq!(out.display_cells[4][4], "60.0%");
        assert_eq!(out.display_cells[4][5], "달성");
        assert_eq!(out.raw_numbers[4][2], Some(400.0));
        assert_eq!(out.formulas[4][2].as_deref(), Some("=SUM(C2:C4)"));
        assert_eq!(out.rendered_cells_sha256.len(), 64);

        // Lossless .xlsx round-trip verification (including merges & number formats).
        let roundtrip = CalcTableEngine::evaluate_xlsx_bytes(&out.xlsx_bytes, 0)?;
        assert_eq!(roundtrip.display_cells, out.display_cells);
        assert_eq!(roundtrip.formulas, out.formulas);
        assert_eq!(roundtrip.rendered_cells_sha256, out.rendered_cells_sha256);

        Ok(())
    }

    #[test]
    fn external_literal_reconciliation_updates_literals_and_preserves_formulas()
    -> Result<(), BridgeError> {
        let spec = CalcTableSpec {
            sheet_name: "사업비".to_string(),
            cells: vec![
                vec![
                    CellInput::text("항목"),
                    CellInput::text("예산"),
                    CellInput::text("비중"),
                ],
                vec![
                    CellInput::text("인건비"),
                    CellInput::number(1000.0).with_number_format("#,##0"),
                    CellInput::formula("=B2/$B$4").with_number_format("0.0%"),
                ],
                vec![
                    CellInput::text("장비비"),
                    CellInput::number(1000.0).with_number_format("#,##0"),
                    CellInput::formula("=B3/$B$4").with_number_format("0.0%"),
                ],
                vec![
                    CellInput::text("총계"),
                    CellInput::formula("=SUM(B2:B3)").with_number_format("#,##0"),
                    CellInput::formula("=SUM(C2:C3)").with_number_format("0.0%"),
                ],
            ],
            merges: vec![],
        };

        let initial = CalcTableEngine::evaluate_spec(&spec)?;
        assert_eq!(initial.display_cells[3][1], "2,000");
        assert_eq!(initial.display_cells[1][2], "50.0%");

        // Simulate external table edit in Hancom/Word:
        // - Change B2 literal "1,000" -> "3,000"
        // - Even pass stale text on formula cell B4 ("2,000"): formula MUST remain intact and recalculate to "4,000"!
        let mut external_grid = initial.display_cells.clone();
        external_grid[1][1] = "3,000".to_string();

        let rec = CalcTableEngine::reconcile_external_literals(&initial.xlsx_bytes, &external_grid)?;
        assert_eq!(rec.modified_literal_count, 1);
        assert_eq!(rec.output.display_cells[1][1], "3,000");
        assert_eq!(rec.output.display_cells[3][1], "4,000");
        assert_eq!(rec.output.display_cells[1][2], "75.0%");
        assert_eq!(rec.output.display_cells[2][2], "25.0%");
        assert_eq!(rec.output.display_cells[3][2], "100.0%");
        assert_eq!(rec.output.formulas[3][1].as_deref(), Some("=SUM(B2:B3)"));
        assert_ne!(
            rec.output.rendered_cells_sha256,
            initial.rendered_cells_sha256
        );

        Ok(())
    }

    #[test]
    fn merges_and_formula_diagnostics_are_reported_and_roundtripped() -> Result<(), BridgeError> {
        let spec = CalcTableSpec {
            sheet_name: "진단표".to_string(),
            cells: vec![
                vec![
                    CellInput::text("통합 헤더").with_bold(true),
                    CellInput::empty(),
                    CellInput::empty(),
                ],
                vec![
                    CellInput::number(100.0),
                    CellInput::number(0.0),
                    CellInput::formula("=A2/B2"),
                ],
            ],
            merges: vec![MergeSpan::new(0, 0, 0, 2)],
        };

        let out = CalcTableEngine::evaluate_spec(&spec)?;
        assert_eq!(out.formula_errors.len(), 1);
        assert_eq!(out.formula_errors[0].address, "C2");
        assert_eq!(out.formula_errors[0].error_code, "#DIV/0!");

        let (wb, _) = gridcraft_xlsx::read_xlsx(&out.xlsx_bytes)?;
        let merges = wb.sheet(0).map(|s| s.merges.len()).unwrap_or(0);
        assert_eq!(merges, 1);

        Ok(())
    }

    #[test]
    fn hostile_calc_inputs_never_panic() {
        // Broken `.xlsx` bytes
        assert!(CalcTableEngine::evaluate_xlsx_bytes(b"", 0).is_err());
        assert!(CalcTableEngine::evaluate_xlsx_bytes(b"PK\x03\x04corrupt_zip_payload", 0).is_err());

        // Circular reference, unknown function, NaN literal, invalid sheet chars
        let spec = CalcTableSpec {
            sheet_name: "bad:[sheet]/name*with?chars_exceeding_thirty_one_characters_limit".into(),
            cells: vec![
                vec![
                    CellInput::formula("=B1+1"),
                    CellInput::formula("=A1+1"),
                    CellInput::formula("=NONEXISTENT_FN(1,2)"),
                    CellInput::number(f64::NAN),
                    CellInput::formula("=("),
                ],
            ],
            merges: vec![],
        };
        let res = CalcTableEngine::evaluate_spec(&spec);
        assert!(res.is_ok());
        if let Ok(out) = res {
            assert!(out.formula_errors.len() >= 4);
            assert!(CalcTableEngine::evaluate_xlsx_bytes(&out.xlsx_bytes, 99).is_err());
        }

        // Inverted merge span rejected cleanly
        let bad_merge = CalcTableSpec {
            sheet_name: "S".into(),
            cells: vec![vec![CellInput::number(1.0)]],
            merges: vec![MergeSpan::new(3, 3, 1, 1)],
        };
        assert!(CalcTableEngine::evaluate_spec(&bad_merge).is_err());
    }
}
