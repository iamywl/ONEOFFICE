//! Unified 26-type vector chart engine (`UnifiedChartEngine`) producing:
//! - Layout primitives (`Vec<gridcraft_chart::Prim>`) measured with Korean HarfRust metrics,
//! - Clean vector `<svg>` (`Rect`, `Line`, `Polygon`, `Wedge`, `Circle`, `Text`),
//! - `2.0x` HiDPI RGBA PNG (`gridcraft_chart::rasterize` + `image`),
//! - ECMA-376 `<c:chartSpace>` XML with `<c:externalData r:id="rId1"/>` and cached values,
//! - Embedded `.xlsx` sidecar workbook (`gridcraft-xlsx`, resolving `GAP-PPTX-01` / `#335`).

use std::f32::consts::{PI, TAU};
use std::fmt::Write as _;
use std::sync::Arc;

use gridcraft_chart::{HAlign, Prim, Rgba, VAlign};
use gridcraft_core::{CellRef, Value, col_to_letters};
use gridcraft_model::{
    Anchor, AnchorMode, Chart, ChartKind, Color, LegendPos, Series, Sheet, Trendline,
    TrendlineKind, Workbook,
};
use image::ImageEncoder as _;
use serde::{Deserialize, Serialize};

use crate::error::BridgeError;
use crate::fonts::KoreanTextMeasurer;

/// Minimum chart dimension in points (`pt`).
const MIN_CHART_PT: f32 = 4.0;
/// Maximum chart dimension in points (`pt`) so `2.0x` HiDPI rasterization stays within `64 MP`.
const MAX_CHART_PT: f32 = 4096.0;
/// HiDPI rasterization scale factor (`2.0x`).
const HIDPI_SCALE: f32 = 2.0;
/// Maximum series count per chart (`Excel` cap).
const MAX_SERIES_CAP: usize = 255;
/// Maximum data points per series materialized into the backing sheet.
const MAX_POINTS_CAP: usize = 10_000;

/// Korean-first CSS/SVG font stack used for all `<text>` elements in generated chart SVGs.
pub const KOREAN_CHART_FONT_STACK: &str =
    "'Pretendard', 'Apple SD Gothic Neo', 'Malgun Gothic', 'Noto Sans KR', sans-serif";

/// Canonical names of all 26 supported [`ChartKind`] variants.
const CANONICAL_CHART_KINDS: [&str; 26] = [
    "column_clustered",
    "column_stacked",
    "column_stacked_100",
    "bar_clustered",
    "bar_stacked",
    "bar_stacked_100",
    "line",
    "line_markers",
    "line_stacked",
    "pie",
    "doughnut",
    "area",
    "area_stacked",
    "area_stacked_100",
    "scatter",
    "scatter_lines",
    "bubble",
    "radar",
    "histogram",
    "waterfall",
    "funnel",
    "treemap",
    "sunburst",
    "box_whisker",
    "stock",
    "combo",
];

/// Canonical names of all 6 supported [`TrendlineKind`] regression families.
const CANONICAL_TRENDLINE_KINDS: [&str; 6] = [
    "linear",
    "exponential",
    "logarithmic",
    "polynomial",
    "power",
    "moving_average",
];

/// Returns the canonical list of all 26 supported chart type identifiers.
#[must_use]
pub fn list_chart_kinds() -> Vec<&'static str> {
    CANONICAL_CHART_KINDS.to_vec()
}

/// Parses a chart type identifier case-insensitively (`snake_case`, `PascalCase`, `kebab-case`,
/// and common Excel/OOXML aliases) into one of the 26 [`ChartKind`] variants.
pub fn parse_chart_kind(raw: &str) -> Result<ChartKind, BridgeError> {
    let norm: String = raw
        .trim()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect();

    match norm.as_str() {
        "columnclustered" | "clusteredcolumn" | "column" | "col" => Ok(ChartKind::ColumnClustered),
        "columnstacked" | "stackedcolumn" => Ok(ChartKind::ColumnStacked),
        "columnstacked100" | "100stackedcolumn" | "columnpercentstacked" => {
            Ok(ChartKind::ColumnStacked100)
        }
        "barclustered" | "clusteredbar" | "bar" => Ok(ChartKind::BarClustered),
        "barstacked" | "stackedbar" => Ok(ChartKind::BarStacked),
        "barstacked100" | "100stackedbar" | "barpercentstacked" => Ok(ChartKind::BarStacked100),
        "line" => Ok(ChartKind::Line),
        "linemarkers" | "linewithmarkers" | "markedline" => Ok(ChartKind::LineMarkers),
        "linestacked" | "stackedline" => Ok(ChartKind::LineStacked),
        "pie" => Ok(ChartKind::Pie),
        "doughnut" | "donut" => Ok(ChartKind::Doughnut),
        "area" => Ok(ChartKind::Area),
        "areastacked" | "stackedarea" => Ok(ChartKind::AreaStacked),
        "areastacked100" | "100stackedarea" | "areapercentstacked" => {
            Ok(ChartKind::AreaStacked100)
        }
        "scatter" | "xy" | "xyscatter" => Ok(ChartKind::Scatter),
        "scatterlines" | "scatterwithlines" | "xyscatterlines" => Ok(ChartKind::ScatterLines),
        "bubble" => Ok(ChartKind::Bubble),
        "radar" => Ok(ChartKind::Radar),
        "histogram" => Ok(ChartKind::Histogram),
        "waterfall" => Ok(ChartKind::Waterfall),
        "funnel" => Ok(ChartKind::Funnel),
        "treemap" => Ok(ChartKind::Treemap),
        "sunburst" => Ok(ChartKind::Sunburst),
        "boxwhisker" | "boxandwhisker" | "boxplot" => Ok(ChartKind::BoxWhisker),
        "stock" | "ohlc" | "hlc" => Ok(ChartKind::Stock),
        "combo" | "combination" => Ok(ChartKind::Combo),
        _ => Err(BridgeError::InvalidChartKind(raw.to_string())),
    }
}

/// Returns the canonical list of all 6 supported trendline regression identifiers.
#[must_use]
pub fn list_trendline_kinds() -> Vec<&'static str> {
    CANONICAL_TRENDLINE_KINDS.to_vec()
}

/// Parses a trendline specification case-insensitively into one of the 6 [`TrendlineKind`]
/// variants (e.g. `"linear"`, `"exponential"`, `"logarithmic"`, `"polynomial"` / `"poly3"`,
/// `"power"`, `"moving_average"` / `"ma4"`).
pub fn parse_trendline_kind(raw: &str) -> Result<TrendlineKind, BridgeError> {
    let trimmed = raw.trim();
    let letters: String = trimmed
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    let digits: String = trimmed.chars().filter(|c| c.is_ascii_digit()).collect();

    match letters.as_str() {
        "linear" | "lin" => Ok(TrendlineKind::Linear),
        "exponential" | "exp" => Ok(TrendlineKind::Exponential),
        "logarithmic" | "log" => Ok(TrendlineKind::Logarithmic),
        "polynomial" | "poly" => {
            let order = digits
                .parse::<u8>()
                .ok()
                .map_or(2, |n| n.clamp(2, 6));
            Ok(TrendlineKind::Polynomial(order))
        }
        "power" | "pow" => Ok(TrendlineKind::Power),
        "movingaverage" | "movingavg" | "ma" => {
            let period = digits
                .parse::<u32>()
                .ok()
                .map_or(2, |p| p.clamp(2, 255));
            Ok(TrendlineKind::MovingAverage(period))
        }
        _ => Err(BridgeError::InvalidTrendlineKind(raw.to_string())),
    }
}

/// Single series input for [`ChartSpecInput`].
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct ChartSeriesInput {
    /// Display name of the series (e.g. `"매출액"`, `"영업이익"`).
    pub name: String,
    /// Numeric data points for this series.
    pub values: Vec<f64>,
}

impl ChartSeriesInput {
    /// Creates a new [`ChartSeriesInput`].
    #[must_use]
    pub fn new(name: impl Into<String>, values: Vec<f64>) -> Self {
        Self {
            name: name.into(),
            values,
        }
    }
}

/// Input specification for [`UnifiedChartEngine::build_chart`].
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ChartSpecInput {
    /// Optional chart title shown at the top of the plot.
    #[serde(default)]
    pub title: Option<String>,
    /// Chart type string (any of the 26 [`ChartKind`] names, case-insensitive).
    pub kind: String,
    /// Category / X-axis labels (e.g. `["1분기", "2분기", "3분기", "4분기"]`).
    #[serde(default)]
    pub categories: Vec<String>,
    /// Data series plotted on the chart.
    #[serde(default)]
    pub series: Vec<ChartSeriesInput>,
    /// Canvas width in points (`pt`).
    pub width_pt: f32,
    /// Canvas height in points (`pt`).
    pub height_pt: f32,
    /// Optional legend position (`"none"`, `"bottom"`, `"top"`, `"left"`, `"right"`).
    #[serde(default)]
    pub legend_pos: Option<String>,
    /// Optional trendline kind (`"linear"`, `"exponential"`, `"logarithmic"`, `"polynomial"`,
    /// `"power"`, `"moving_average"`).
    #[serde(default)]
    pub trendline: Option<String>,
    /// Whether to style canvas background, gridlines, and neutral labels for Dark Mode HUDs.
    #[serde(default)]
    pub dark_mode: bool,
}

/// Multi-format compiled chart bundle ready for HWPX, DOCX, PPTX, XLSX, PDF, and Webview HUDs.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ChartBundle {
    /// Vector drawing primitives laid out with [`KoreanTextMeasurer`].
    pub primitives: Vec<Prim>,
    /// Clean standalone vector `<svg>` rendering all [`Prim`] variants with Korean font stack.
    pub svg: String,
    /// `2.0x` HiDPI RGBA PNG bytes rasterized via `gridcraft_chart::rasterize` + `image`.
    pub png_bytes: Vec<u8>,
    /// Valid ECMA-376 `<c:chartSpace>` XML containing `<c:externalData r:id="rId1"/>` and cached
    /// `<c:strCache>` / `<c:numCache>` values.
    pub ooxml_chart_xml: String,
    /// Complete `.xlsx` sidecar workbook holding the source data and embedded `Chart` object.
    pub embedded_xlsx_bytes: Vec<u8>,
}

/// Unified 26-type vector chart compiler and sidecar generator.
#[derive(Clone, Copy, Debug, Default)]
pub struct UnifiedChartEngine;

impl UnifiedChartEngine {
    /// Lists all 26 supported chart kind strings.
    #[must_use]
    pub fn list_chart_kinds() -> Vec<&'static str> {
        list_chart_kinds()
    }

    /// Parses a chart kind string case-insensitively.
    pub fn parse_chart_kind(kind: &str) -> Result<ChartKind, BridgeError> {
        parse_chart_kind(kind)
    }

    /// Lists all 6 supported trendline kind strings.
    #[must_use]
    pub fn list_trendline_kinds() -> Vec<&'static str> {
        list_trendline_kinds()
    }

    /// Parses a trendline kind string case-insensitively.
    pub fn parse_trendline_kind(kind: &str) -> Result<TrendlineKind, BridgeError> {
        parse_trendline_kind(kind)
    }

    /// Compiles `spec` into a complete [`ChartBundle`] (primitives, SVG, `2.0x` HiDPI PNG,
    /// ECMA-376 `<c:chartSpace>` XML, and embedded `.xlsx` sidecar).
    pub fn build_chart(spec: &ChartSpecInput) -> Result<ChartBundle, BridgeError> {
        if !spec.width_pt.is_finite()
            || !spec.height_pt.is_finite()
            || !(MIN_CHART_PT..=MAX_CHART_PT).contains(&spec.width_pt)
            || !(MIN_CHART_PT..=MAX_CHART_PT).contains(&spec.height_pt)
        {
            return Err(BridgeError::InvalidChartDimensions {
                width_pt: spec.width_pt,
                height_pt: spec.height_pt,
            });
        }

        let chart_kind = parse_chart_kind(&spec.kind)?;
        let legend = parse_legend_pos(spec.legend_pos.as_deref());
        let trendline_opt = match spec
            .trendline
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("none"))
        {
            Some(tl_str) => Some(Trendline {
                kind: parse_trendline_kind(tl_str)?,
                ..Trendline::default()
            }),
            None => None,
        };

        let (wb, chart, resolved_cats, resolved_series) =
            build_backing_workbook(spec, chart_kind, legend, trendline_opt.as_ref());

        let embedded_xlsx_bytes = gridcraft_xlsx::write_xlsx(&wb)?;

        let chart_data = gridcraft_chart::resolve(&wb, 0, &chart);
        let measurer = KoreanTextMeasurer::new();
        let mut primitives = gridcraft_chart::render(
            &chart,
            &chart_data,
            spec.width_pt,
            spec.height_pt,
            &measurer,
        );
        if spec.dark_mode {
            apply_dark_mode_palette(&mut primitives);
        }

        let svg = render_primitives_svg(&primitives, spec.width_pt, spec.height_pt);
        let png_bytes =
            rasterize_hidpi_png(&primitives, spec.width_pt, spec.height_pt, spec.dark_mode)?;
        let ooxml_chart_xml = build_ooxml_chart_space_xml(
            &wb,
            &chart,
            &resolved_cats,
            &resolved_series,
        );

        Ok(ChartBundle {
            primitives,
            svg,
            png_bytes,
            ooxml_chart_xml,
            embedded_xlsx_bytes,
        })
    }
}

#[derive(Clone, Debug)]
struct ResolvedSeriesCache {
    name: String,
    name_ref: String,
    cat_ref: String,
    val_ref: String,
    values: Vec<Option<f64>>,
    color_hex: Option<String>,
    kind: Option<ChartKind>,
    trendlines: Vec<Trendline>,
}

fn parse_legend_pos(raw: Option<&str>) -> LegendPos {
    match raw.map(|s| s.trim().to_ascii_lowercase()).as_deref() {
        Some("none" | "off" | "hidden" | "false") => LegendPos::None,
        Some("top" | "t") => LegendPos::Top,
        Some("left" | "l") => LegendPos::Left,
        Some("right" | "r") => LegendPos::Right,
        _ => LegendPos::Bottom,
    }
}

fn build_backing_workbook(
    spec: &ChartSpecInput,
    chart_kind: ChartKind,
    legend: LegendPos,
    trendline: Option<&Trendline>,
) -> (Workbook, Chart, Vec<String>, Vec<ResolvedSeriesCache>) {
    let mut wb = Workbook::new();
    let mut sheet = Sheet::new("Sheet1");

    let capped_series: Vec<&ChartSeriesInput> = spec.series.iter().take(MAX_SERIES_CAP).collect();
    let max_val_len = capped_series
        .iter()
        .map(|s| s.values.len().min(MAX_POINTS_CAP))
        .max()
        .unwrap_or(0);
    let point_count = spec.categories.len().min(MAX_POINTS_CAP).max(max_val_len);

    let categories: Vec<String> = (0..point_count)
        .map(|idx| {
            spec.categories
                .get(idx)
                .cloned()
                .unwrap_or_else(|| (idx + 1).to_string())
        })
        .collect();

    sheet.set_value(CellRef::new(0, 0), Value::text("Category"));
    let numeric_x = matches!(
        chart_kind,
        ChartKind::Scatter | ChartKind::ScatterLines | ChartKind::Bubble
    );

    for (r_idx, cat) in categories.iter().enumerate() {
        let cell_val = if numeric_x
            && let Some(n) = gridcraft_core::parse::parse_number_text(cat)
            && n.is_finite()
        {
            Value::number(n)
        } else {
            Value::text(clean_xml_chars(cat))
        };
        sheet.set_value(CellRef::new((r_idx + 1) as u32, 0), cell_val);
    }

    let cat_range_str = if point_count > 0 {
        format!("Sheet1!$A$2:$A${}", point_count + 1)
    } else {
        String::new()
    };

    let mut model_series = Vec::with_capacity(capped_series.len());
    let mut caches = Vec::with_capacity(capped_series.len());

    for (s_idx, s_in) in capped_series.iter().enumerate() {
        let col_idx = (s_idx + 1) as u32;
        let col_let = col_to_letters(col_idx);
        let clean_name = if s_in.name.trim().is_empty() {
            format!("Series{}", s_idx + 1)
        } else {
            clean_xml_chars(&s_in.name)
        };
        sheet.set_value(CellRef::new(0, col_idx), Value::text(clean_name.clone()));

        let mut cached_vals = Vec::with_capacity(point_count);
        for r_idx in 0..point_count {
            let finite_opt = s_in
                .values
                .get(r_idx)
                .copied()
                .filter(|v| v.is_finite());
            if let Some(v) = finite_opt {
                sheet.set_value(CellRef::new((r_idx + 1) as u32, col_idx), Value::number(v));
            }
            cached_vals.push(finite_opt);
        }

        let name_ref = format!("Sheet1!${col_let}$1");
        let val_ref = if point_count > 0 {
            format!("Sheet1!${col_let}$2:${col_let}${}", point_count + 1)
        } else {
            String::new()
        };

        let per_series_kind = if chart_kind == ChartKind::Combo {
            Some(if s_idx == 0 {
                ChartKind::ColumnClustered
            } else {
                ChartKind::Line
            })
        } else {
            None
        };

        let effective_kind = per_series_kind.unwrap_or(chart_kind);
        let series_trendlines = match trendline {
            Some(tl) if gridcraft_chart::supports_trendline(effective_kind) => vec![tl.clone()],
            Some(tl) if s_idx == 0 => vec![tl.clone()],
            _ => Vec::new(),
        };

        let rgba = gridcraft_chart::series_color(&wb.theme, s_idx);
        let color_hex = Some(format!("{:02X}{:02X}{:02X}", rgba[0], rgba[1], rgba[2]));

        model_series.push(Series {
            name: Some(name_ref.clone()),
            categories: (!cat_range_str.is_empty()).then(|| cat_range_str.clone()),
            values: val_ref.clone(),
            bubble_sizes: (chart_kind == ChartKind::Bubble && !val_ref.is_empty())
                .then(|| val_ref.clone()),
            color: Some(Color::rgb(rgba[0], rgba[1], rgba[2])),
            secondary: false,
            kind: per_series_kind,
            smooth: false,
            quartiles: Default::default(),
            trendlines: series_trendlines.clone(),
        });

        caches.push(ResolvedSeriesCache {
            name: clean_name,
            name_ref,
            cat_ref: cat_range_str.clone(),
            val_ref,
            values: cached_vals,
            color_hex,
            kind: per_series_kind,
            trendlines: series_trendlines,
        });
    }

    let chart = Chart {
        id: 1,
        kind: chart_kind,
        anchor: Anchor {
            cell: CellRef::new(0, (capped_series.len() as u32).saturating_add(2)),
            dx: 0.0,
            dy: 0.0,
            width: spec.width_pt,
            height: spec.height_pt,
            mode: AnchorMode::MoveAndSize,
        },
        title: spec
            .title
            .as_deref()
            .map(clean_xml_chars)
            .filter(|t| !t.trim().is_empty()),
        series: model_series,
        legend,
        data_labels: false,
        gridlines: true,
        style: 2,
        x_title: None,
        y_title: None,
        source: None,
        by_rows: false,
        pivot: None,
    };

    sheet.charts.push(chart.clone());
    wb.sheets = vec![Arc::new(sheet)];

    (wb, chart, categories, caches)
}

fn apply_dark_mode_palette(primitives: &mut [Prim]) {
    const WHITE: Rgba = [0xFF, 0xFF, 0xFF, 0xFF];
    const GRID_LIGHT: Rgba = [0xD9, 0xD9, 0xD9, 0xFF];
    const AXIS_LIGHT: Rgba = [0xBF, 0xBF, 0xBF, 0xFF];
    const TEXT_MID: Rgba = [0x59, 0x59, 0x59, 0xFF];
    const TEXT_DARK: Rgba = [0x26, 0x26, 0x26, 0xFF];
    const STOCK_DARK: Rgba = [0x40, 0x40, 0x40, 0xFF];

    const BG_DARK: Rgba = [0x18, 0x1C, 0x24, 0xFF];
    const BORDER_DARK: Rgba = [0x33, 0x3C, 0x4D, 0xFF];
    const GRID_DARK: Rgba = [0x2D, 0x35, 0x45, 0xFF];
    const AXIS_DARK: Rgba = [0x52, 0x5E, 0x73, 0xFF];
    const TEXT_LIGHT: Rgba = [0xE6, 0xEC, 0xF5, 0xFF];

    let remap_stroke = |c: Rgba| -> Rgba {
        match c {
            WHITE => BG_DARK,
            GRID_LIGHT => BORDER_DARK,
            AXIS_LIGHT => AXIS_DARK,
            STOCK_DARK => TEXT_LIGHT,
            other => other,
        }
    };

    for prim in primitives.iter_mut() {
        match prim {
            Prim::Rect { fill, stroke, .. } => {
                if *fill == Some(WHITE) {
                    *fill = Some(BG_DARK);
                } else if *fill == Some(STOCK_DARK) {
                    *fill = Some(TEXT_LIGHT);
                }
                if let Some((sc, _)) = stroke {
                    *sc = remap_stroke(*sc);
                }
            }
            Prim::Line { color, .. } => {
                *color = match *color {
                    GRID_LIGHT => GRID_DARK,
                    AXIS_LIGHT => AXIS_DARK,
                    WHITE => BG_DARK,
                    STOCK_DARK => TEXT_LIGHT,
                    other => other,
                };
            }
            Prim::Polygon { stroke, .. } => {
                if let Some((sc, _)) = stroke {
                    *sc = remap_stroke(*sc);
                }
            }
            Prim::Wedge { stroke, .. } | Prim::Circle { stroke, .. } => {
                if let Some((sc, _)) = stroke {
                    *sc = remap_stroke(*sc);
                }
            }
            Prim::Text { color, .. } => {
                if matches!(*color, TEXT_MID | TEXT_DARK | STOCK_DARK) {
                    *color = TEXT_LIGHT;
                }
            }
        }
    }
}

/// Renders a slice of [`Prim`] primitives into a standalone vector `<svg>` document.
#[must_use]
pub fn render_primitives_svg(primitives: &[Prim], width_pt: f32, height_pt: f32) -> String {
    let w = if width_pt.is_finite() && width_pt > 0.0 {
        width_pt
    } else {
        1.0
    };
    let h = if height_pt.is_finite() && height_pt > 0.0 {
        height_pt
    } else {
        1.0
    };

    let mut out = String::with_capacity(primitives.len() * 128 + 160);
    let _ = write!(
        out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w:.2}\" height=\"{h:.2}\" viewBox=\"0 0 {w:.2} {h:.2}\">"
    );

    for prim in primitives {
        match prim {
            Prim::Rect {
                x,
                y,
                w: rw,
                h: rh,
                fill,
                stroke,
                radius,
            } => {
                let _ = write!(
                    out,
                    "<rect x=\"{x:.2}\" y=\"{y:.2}\" width=\"{rw:.2}\" height=\"{rh:.2}\""
                );
                if *radius > 0.0 {
                    let _ = write!(out, " rx=\"{radius:.2}\" ry=\"{radius:.2}\"");
                }
                write_svg_paint(&mut out, *fill, *stroke);
                out.push_str("/>");
            }
            Prim::Line {
                pts,
                color,
                width,
                dash,
            } => {
                if pts.len() < 2 {
                    continue;
                }
                out.push_str("<polyline points=\"");
                for (i, p) in pts.iter().enumerate() {
                    if i > 0 {
                        out.push(' ');
                    }
                    let _ = write!(out, "{:.2},{:.2}", p[0], p[1]);
                }
                let stroke_css = rgba_css(*color);
                let _ = write!(
                    out,
                    "\" fill=\"none\" stroke=\"{stroke_css}\" stroke-width=\"{width:.2}\" stroke-linecap=\"round\" stroke-linejoin=\"round\""
                );
                if *dash {
                    let (on, off) = (width * 4.0, width * 3.0);
                    let _ = write!(out, " stroke-dasharray=\"{on:.2},{off:.2}\"");
                }
                out.push_str("/>");
            }
            Prim::Polygon { pts, fill, stroke } => {
                if pts.len() < 3 {
                    continue;
                }
                out.push_str("<polygon points=\"");
                for (i, p) in pts.iter().enumerate() {
                    if i > 0 {
                        out.push(' ');
                    }
                    let _ = write!(out, "{:.2},{:.2}", p[0], p[1]);
                }
                out.push('"');
                write_svg_paint(&mut out, Some(*fill), *stroke);
                out.push_str("/>");
            }
            Prim::Wedge {
                cx,
                cy,
                r_outer,
                r_inner,
                a0,
                a1,
                fill,
                stroke,
            } => {
                let Some(d) = wedge_svg_path(*cx, *cy, *r_outer, *r_inner, *a0, *a1) else {
                    continue;
                };
                let _ = write!(out, "<path d=\"{d}\" fill-rule=\"evenodd\"");
                write_svg_paint(&mut out, Some(*fill), *stroke);
                out.push_str("/>");
            }
            Prim::Circle {
                cx,
                cy,
                r,
                fill,
                stroke,
            } => {
                let _ = write!(out, "<circle cx=\"{cx:.2}\" cy=\"{cy:.2}\" r=\"{r:.2}\"");
                write_svg_paint(&mut out, *fill, *stroke);
                out.push_str("/>");
            }
            Prim::Text {
                x,
                y,
                text,
                size,
                color,
                bold,
                align,
                valign,
                rotation,
            } => {
                let cleaned = clean_xml_chars(text);
                if cleaned.is_empty() {
                    continue;
                }
                let anchor = match align {
                    HAlign::Left => "start",
                    HAlign::Center => "middle",
                    HAlign::Right => "end",
                };
                let baseline = match valign {
                    VAlign::Top => "hanging",
                    VAlign::Middle => "central",
                    VAlign::Bottom => "alphabetic",
                };
                let weight = if *bold { "700" } else { "400" };
                let fill_css = rgba_css(*color);
                let _ = write!(
                    out,
                    "<text x=\"{x:.2}\" y=\"{y:.2}\" font-family=\"{KOREAN_CHART_FONT_STACK}\" font-size=\"{size:.2}\" font-weight=\"{weight}\" fill=\"{fill_css}\" text-anchor=\"{anchor}\" dominant-baseline=\"{baseline}\""
                );
                if rotation.abs() > 1e-4 {
                    let deg = rotation.to_degrees();
                    let _ = write!(out, " transform=\"rotate({deg:.2} {x:.2} {y:.2})\"");
                }
                let _ = write!(out, ">{}</text>", xml_escape(&cleaned));
            }
        }
    }

    out.push_str("</svg>");
    out
}

fn wedge_svg_path(
    cx: f32,
    cy: f32,
    r_outer: f32,
    r_inner: f32,
    a0: f32,
    a1: f32,
) -> Option<String> {
    if r_outer <= 0.0 {
        return None;
    }
    let sweep = (a1 - a0).clamp(0.0, TAU);
    if sweep <= 1e-5 {
        return None;
    }
    let ri = r_inner.clamp(0.0, r_outer);
    let pt = |r: f32, a: f32| -> (f32, f32) { (cx + r * a.sin(), cy - r * a.cos()) };

    // Full-circle 360-degree wedge: split into two 180-degree `A` arcs so SVG endpoints differ.
    if sweep >= TAU - 1e-4 {
        let amid = a0 + PI;
        let (ox0, oy0) = pt(r_outer, a0);
        let (oxm, oym) = pt(r_outer, amid);
        let mut d = format!(
            "M {ox0:.2} {oy0:.2} A {r_outer:.2} {r_outer:.2} 0 1 1 {oxm:.2} {oym:.2} A {r_outer:.2} {r_outer:.2} 0 1 1 {ox0:.2} {oy0:.2} Z"
        );
        if ri > 0.0 {
            let (ix0, iy0) = pt(ri, a0);
            let (ixm, iym) = pt(ri, amid);
            let _ = write!(
                d,
                " M {ix0:.2} {iy0:.2} A {ri:.2} {ri:.2} 0 1 0 {ixm:.2} {iym:.2} A {ri:.2} {ri:.2} 0 1 0 {ix0:.2} {iy0:.2} Z"
            );
        }
        return Some(d);
    }

    let large_arc = u8::from(sweep > PI);
    let (ox0, oy0) = pt(r_outer, a0);
    let (ox1, oy1) = pt(r_outer, a1);
    if ri <= 0.0 {
        Some(format!(
            "M {cx:.2} {cy:.2} L {ox0:.2} {oy0:.2} A {r_outer:.2} {r_outer:.2} 0 {large_arc} 1 {ox1:.2} {oy1:.2} Z"
        ))
    } else {
        let (ix1, iy1) = pt(ri, a1);
        let (ix0, iy0) = pt(ri, a0);
        Some(format!(
            "M {ox0:.2} {oy0:.2} A {r_outer:.2} {r_outer:.2} 0 {large_arc} 1 {ox1:.2} {oy1:.2} L {ix1:.2} {iy1:.2} A {ri:.2} {ri:.2} 0 {large_arc} 0 {ix0:.2} {iy0:.2} Z"
        ))
    }
}

fn write_svg_paint(out: &mut String, fill: Option<Rgba>, stroke: Option<(Rgba, f32)>) {
    match fill {
        Some(c) => {
            let css = rgba_css(c);
            let _ = write!(out, " fill=\"{css}\"");
        }
        None => out.push_str(" fill=\"none\""),
    }
    if let Some((sc, sw)) = stroke
        && sw > 0.0
    {
        let css = rgba_css(sc);
        let _ = write!(out, " stroke=\"{css}\" stroke-width=\"{sw:.2}\"");
    }
}

fn rgba_css(c: Rgba) -> String {
    if c[3] == 0xFF {
        format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])
    } else {
        let a = f32::from(c[3]) / 255.0;
        format!("rgba({},{},{},{a:.3})", c[0], c[1], c[2])
    }
}

fn scale_prim(prim: &Prim, s: f32) -> Prim {
    match prim {
        Prim::Rect {
            x,
            y,
            w,
            h,
            fill,
            stroke,
            radius,
        } => Prim::Rect {
            x: x * s,
            y: y * s,
            w: w * s,
            h: h * s,
            fill: *fill,
            stroke: stroke.map(|(c, sw)| (c, sw * s)),
            radius: radius * s,
        },
        Prim::Line {
            pts,
            color,
            width,
            dash,
        } => Prim::Line {
            pts: pts.iter().map(|p| [p[0] * s, p[1] * s]).collect(),
            color: *color,
            width: width * s,
            dash: *dash,
        },
        Prim::Polygon { pts, fill, stroke } => Prim::Polygon {
            pts: pts.iter().map(|p| [p[0] * s, p[1] * s]).collect(),
            fill: *fill,
            stroke: stroke.map(|(c, sw)| (c, sw * s)),
        },
        Prim::Wedge {
            cx,
            cy,
            r_outer,
            r_inner,
            a0,
            a1,
            fill,
            stroke,
        } => Prim::Wedge {
            cx: cx * s,
            cy: cy * s,
            r_outer: r_outer * s,
            r_inner: r_inner * s,
            a0: *a0,
            a1: *a1,
            fill: *fill,
            stroke: stroke.map(|(c, sw)| (c, sw * s)),
        },
        Prim::Circle {
            cx,
            cy,
            r,
            fill,
            stroke,
        } => Prim::Circle {
            cx: cx * s,
            cy: cy * s,
            r: r * s,
            fill: *fill,
            stroke: stroke.map(|(c, sw)| (c, sw * s)),
        },
        Prim::Text {
            x,
            y,
            text,
            size,
            color,
            bold,
            align,
            valign,
            rotation,
        } => Prim::Text {
            x: x * s,
            y: y * s,
            text: text.clone(),
            size: size * s,
            color: *color,
            bold: *bold,
            align: *align,
            valign: *valign,
            rotation: *rotation,
        },
    }
}

fn rasterize_hidpi_png(
    primitives: &[Prim],
    width_pt: f32,
    height_pt: f32,
    dark_mode: bool,
) -> Result<Vec<u8>, BridgeError> {
    let px_w = (width_pt * HIDPI_SCALE).round().clamp(1.0, 8192.0) as u32;
    let px_h = (height_pt * HIDPI_SCALE).round().clamp(1.0, 8192.0) as u32;
    let scaled: Vec<Prim> = primitives
        .iter()
        .map(|p| scale_prim(p, HIDPI_SCALE))
        .collect();
    let bg: Rgba = if dark_mode {
        [0x18, 0x1C, 0x24, 0xFF]
    } else {
        [0xFF, 0xFF, 0xFF, 0xFF]
    };
    let mut rgba = gridcraft_chart::rasterize(&scaled, px_w, px_h, bg);
    let expected_len = (px_w as usize)
        .saturating_mul(px_h as usize)
        .saturating_mul(4);
    if rgba.len() != expected_len {
        return Err(BridgeError::PngEncode(format!(
            "unexpected RGBA buffer length {} (expected {expected_len})",
            rgba.len()
        )));
    }

    render_chart_text_overlay(&mut rgba, px_w, px_h, &scaled);

    let mut png_bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png_bytes)
        .write_image(&rgba, px_w, px_h, image::ExtendedColorType::Rgba8)
        .map_err(|e| BridgeError::PngEncode(e.to_string()))?;
    Ok(png_bytes)
}

fn render_chart_text_overlay(
    rgba: &mut [u8],
    px_w: u32,
    px_h: u32,
    primitives: &[Prim],
) {
    if px_w > u16::MAX as u32 || px_h > u16::MAX as u32 || px_w == 0 || px_h == 0 {
        return;
    }
    let u16_w = px_w as u16;
    let u16_h = px_h as u16;

    let total_pixels = (px_w as usize).saturating_mul(px_h as usize);
    let mut overlay = vec![0u8; total_pixels.saturating_mul(4)];

    let mut ctx = vello_cpu::RenderContext::new_with(
        u16_w,
        u16_h,
        vello_cpu::RenderSettings {
            num_threads: 0,
            ..Default::default()
        },
    );

    let fam = crate::fonts::default_korean_family().unwrap_or("Pretendard");
    let regular_face = wordcraft_fonts::word::resolve(fam, false, false).face;
    let bold_face = wordcraft_fonts::word::resolve(fam, true, false).face;

    let mut any_text = false;

    for prim in primitives {
        if let Prim::Text {
            x,
            y,
            text,
            size,
            color,
            bold,
            align,
            valign,
            rotation,
        } = prim
        {
            if text.is_empty() || *size <= 0.0 {
                continue;
            }
            any_text = true;

            let face = if *bold { &bold_face } else { &regular_face };
            let upem = if face.upem.is_finite() && face.upem > 0.0 {
                face.upem
            } else {
                1000.0
            };
            let k = (*size as f64) / upem;

            let glyphs = wordcraft_fonts::shape(face, text, &[], |c| c);
            let total_width: f64 = glyphs.iter().map(|g| g.x_advance as f64 * k).sum();

            let dx = match align {
                HAlign::Left => 0.0,
                HAlign::Center => -total_width * 0.5,
                HAlign::Right => -total_width,
            };

            let cap = if face.cap_height.is_finite() && face.cap_height > 0.0 {
                face.cap_height.clamp(upem * 0.3, upem)
            } else {
                upem * 0.7
            };
            let cap_pt = cap * k;

            let dy = match valign {
                VAlign::Top => cap_pt,
                VAlign::Middle => cap_pt * 0.5,
                VAlign::Bottom => 0.0,
            };

            let peniko_c = vello_cpu::peniko::Color::from_rgba8(color[0], color[1], color[2], color[3]);
            ctx.set_paint(peniko_c);

            let rot = *rotation as f64;
            let anchor_x = *x as f64;
            let anchor_y = *y as f64;

            let base_tf = kurbo::Affine::translate((anchor_x, anchor_y))
                * kurbo::Affine::rotate(rot)
                * kurbo::Affine::translate((dx, dy));

            let mut cur_x = 0.0;
            for g in glyphs {
                let gx = cur_x + g.x_offset as f64 * k;
                let gy = -g.y_offset as f64 * k;
                let tf = base_tf * kurbo::Affine::translate((gx, gy)) * kurbo::Affine::scale(k);
                ctx.set_transform(tf);
                ctx.fill_path(&face.glyph_outline(g.gid));
                cur_x += g.x_advance as f64 * k;
            }
        }
    }

    if !any_text {
        return;
    }

    ctx.flush();
    let mut res = vello_cpu::Resources::new();
    if let Some(pm) = vello_cpu::PixmapMut::new(u16_w, u16_h, &mut overlay) {
        ctx.render(pm, &mut res);
    }

    // Alpha-blend overlay over original chart canvas
    #[allow(clippy::chunks_exact_to_as_chunks, clippy::manual_checked_ops)]
    for (dst, src) in rgba.chunks_exact_mut(4).zip(overlay.chunks_exact(4)) {
        let sa = src[3] as u32;
        if sa == 0 {
            continue;
        }
        if sa == 255 {
            dst.copy_from_slice(src);
            continue;
        }
        let da = dst[3] as u32;
        let inv_sa = 255 - sa;
        let out_a = sa + (da * inv_sa + 127) / 255;
        if out_a > 0 {
            for c in 0..3 {
                let s_col = src[c] as u32;
                let d_col = dst[c] as u32;
                let num = s_col * sa + (d_col * da * inv_sa + 127) / 255;
                dst[c] = (num / out_a).min(255) as u8;
            }
            dst[3] = out_a.min(255) as u8;
        }
    }
}

// ---------------------------------------------------------------------------
// ECMA-376 `<c:chartSpace>` XML Compiler (with `<c:externalData r:id="rId1"/>`
// and cached `<c:strCache>` / `<c:numCache>` values)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum OoxmlGroup {
    Bar {
        horizontal: bool,
        grouping: &'static str,
    },
    Line {
        stacked: bool,
        markers: bool,
    },
    Pie,
    Doughnut,
    Area {
        grouping: &'static str,
    },
    Scatter {
        lines: bool,
    },
    Bubble,
    Radar,
    Stock,
}

fn ooxml_group_for(kind: ChartKind) -> OoxmlGroup {
    match kind {
        ChartKind::ColumnClustered
        | ChartKind::Histogram
        | ChartKind::Waterfall
        | ChartKind::BoxWhisker
        | ChartKind::Treemap => OoxmlGroup::Bar {
            horizontal: false,
            grouping: "clustered",
        },
        ChartKind::ColumnStacked => OoxmlGroup::Bar {
            horizontal: false,
            grouping: "stacked",
        },
        ChartKind::ColumnStacked100 => OoxmlGroup::Bar {
            horizontal: false,
            grouping: "percentStacked",
        },
        ChartKind::BarClustered | ChartKind::Funnel => OoxmlGroup::Bar {
            horizontal: true,
            grouping: "clustered",
        },
        ChartKind::BarStacked => OoxmlGroup::Bar {
            horizontal: true,
            grouping: "stacked",
        },
        ChartKind::BarStacked100 => OoxmlGroup::Bar {
            horizontal: true,
            grouping: "percentStacked",
        },
        ChartKind::Line => OoxmlGroup::Line {
            stacked: false,
            markers: false,
        },
        ChartKind::LineMarkers => OoxmlGroup::Line {
            stacked: false,
            markers: true,
        },
        ChartKind::LineStacked => OoxmlGroup::Line {
            stacked: true,
            markers: false,
        },
        ChartKind::Pie => OoxmlGroup::Pie,
        ChartKind::Doughnut | ChartKind::Sunburst => OoxmlGroup::Doughnut,
        ChartKind::Area => OoxmlGroup::Area {
            grouping: "standard",
        },
        ChartKind::AreaStacked => OoxmlGroup::Area {
            grouping: "stacked",
        },
        ChartKind::AreaStacked100 => OoxmlGroup::Area {
            grouping: "percentStacked",
        },
        ChartKind::Scatter => OoxmlGroup::Scatter { lines: false },
        ChartKind::ScatterLines => OoxmlGroup::Scatter { lines: true },
        ChartKind::Bubble => OoxmlGroup::Bubble,
        ChartKind::Radar => OoxmlGroup::Radar,
        ChartKind::Stock => OoxmlGroup::Stock,
        ChartKind::Combo => OoxmlGroup::Bar {
            horizontal: false,
            grouping: "clustered",
        },
    }
}

fn build_ooxml_chart_space_xml(
    _wb: &Workbook,
    chart: &Chart,
    categories: &[String],
    series: &[ResolvedSeriesCache],
) -> String {
    let mut x = String::with_capacity(2048);
    x.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n");
    x.push_str("<c:chartSpace xmlns:c=\"http://schemas.openxmlformats.org/drawingml/2006/chart\" xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">");
    x.push_str("<c:roundedCorners val=\"0\"/>");
    let _ = write!(x, "<c:style val=\"{}\"/>", chart.style.clamp(1, 48));
    x.push_str("<c:chart>");

    match &chart.title {
        Some(title) if !title.is_empty() => {
            x.push_str(&ooxml_title_xml(title));
            x.push_str("<c:autoTitleDeleted val=\"0\"/>");
        }
        _ => x.push_str("<c:autoTitleDeleted val=\"1\"/>"),
    }

    x.push_str("<c:plotArea><c:layout/>");

    let indexed: Vec<(usize, &ResolvedSeriesCache)> = series.iter().enumerate().collect();
    let primary_group = if chart.kind == ChartKind::Combo {
        let bar_sers: Vec<(usize, &ResolvedSeriesCache)> = indexed
            .iter()
            .filter(|(i, s)| {
                let k = s.kind.unwrap_or(if *i == 0 {
                    ChartKind::ColumnClustered
                } else {
                    ChartKind::Line
                });
                !matches!(k, ChartKind::Line | ChartKind::LineMarkers | ChartKind::LineStacked)
            })
            .copied()
            .collect();
        let line_sers: Vec<(usize, &ResolvedSeriesCache)> = indexed
            .iter()
            .filter(|(i, s)| {
                let k = s.kind.unwrap_or(if *i == 0 {
                    ChartKind::ColumnClustered
                } else {
                    ChartKind::Line
                });
                matches!(k, ChartKind::Line | ChartKind::LineMarkers | ChartKind::LineStacked)
            })
            .copied()
            .collect();

        let g_bar = OoxmlGroup::Bar {
            horizontal: false,
            grouping: "clustered",
        };
        if !bar_sers.is_empty() || line_sers.is_empty() {
            x.push_str(&ooxml_group_xml(g_bar, &bar_sers, categories));
        }
        if !line_sers.is_empty() {
            x.push_str(&ooxml_group_xml(
                OoxmlGroup::Line {
                    stacked: false,
                    markers: false,
                },
                &line_sers,
                categories,
            ));
        }
        g_bar
    } else {
        let g = match ooxml_group_for(chart.kind) {
            OoxmlGroup::Stock if !(3..=4).contains(&series.len()) => OoxmlGroup::Line {
                stacked: false,
                markers: false,
            },
            other => other,
        };
        x.push_str(&ooxml_group_xml(g, &indexed, categories));
        g
    };

    // Emit axes where required by ECMA-376 plotArea schema.
    match primary_group {
        OoxmlGroup::Pie | OoxmlGroup::Doughnut => {}
        OoxmlGroup::Scatter { .. } | OoxmlGroup::Bubble => {
            x.push_str(&ooxml_axis_xml(false, 1, 2, "b", false));
            x.push_str(&ooxml_axis_xml(false, 2, 1, "l", chart.gridlines));
        }
        OoxmlGroup::Bar {
            horizontal: true, ..
        } => {
            x.push_str(&ooxml_axis_xml(true, 1, 2, "l", false));
            x.push_str(&ooxml_axis_xml(false, 2, 1, "b", chart.gridlines));
        }
        _ => {
            x.push_str(&ooxml_axis_xml(true, 1, 2, "b", false));
            x.push_str(&ooxml_axis_xml(false, 2, 1, "l", chart.gridlines));
        }
    }

    x.push_str("</c:plotArea>");

    let legend_code = match chart.legend {
        LegendPos::None => None,
        LegendPos::Bottom => Some("b"),
        LegendPos::Top => Some("t"),
        LegendPos::Left => Some("l"),
        LegendPos::Right => Some("r"),
    };
    if let Some(pos) = legend_code {
        let _ = write!(
            x,
            "<c:legend><c:legendPos val=\"{pos}\"/><c:overlay val=\"0\"/></c:legend>"
        );
    }

    x.push_str("<c:plotVisOnly val=\"1\"/><c:dispBlanksAs val=\"gap\"/></c:chart>");
    x.push_str("<c:externalData r:id=\"rId1\"/>");
    x.push_str("</c:chartSpace>");
    x
}

fn ooxml_group_xml(
    group: OoxmlGroup,
    series: &[(usize, &ResolvedSeriesCache)],
    categories: &[String],
) -> String {
    let mut sers = String::new();
    for &(idx, ser) in series {
        sers.push_str(&ooxml_series_xml(idx, ser, group, categories));
    }
    let dlbls = "<c:dLbls><c:showLegendKey val=\"0\"/><c:showVal val=\"0\"/><c:showCatName val=\"0\"/><c:showSerName val=\"0\"/><c:showPercent val=\"0\"/><c:showBubbleSize val=\"0\"/></c:dLbls>";
    let axes = "<c:axId val=\"1\"/><c:axId val=\"2\"/>";

    match group {
        OoxmlGroup::Bar {
            horizontal,
            grouping,
        } => {
            let dir = if horizontal { "bar" } else { "col" };
            let overlap = if grouping == "clustered" {
                ""
            } else {
                "<c:overlap val=\"100\"/>"
            };
            format!(
                "<c:barChart><c:barDir val=\"{dir}\"/><c:grouping val=\"{grouping}\"/><c:varyColors val=\"0\"/>{sers}{dlbls}<c:gapWidth val=\"150\"/>{overlap}{axes}</c:barChart>"
            )
        }
        OoxmlGroup::Line { stacked, .. } => {
            let grp = if stacked { "stacked" } else { "standard" };
            format!(
                "<c:lineChart><c:grouping val=\"{grp}\"/><c:varyColors val=\"0\"/>{sers}{dlbls}<c:marker val=\"1\"/>{axes}</c:lineChart>"
            )
        }
        OoxmlGroup::Pie => {
            format!(
                "<c:pieChart><c:varyColors val=\"1\"/>{sers}{dlbls}<c:firstSliceAng val=\"0\"/></c:pieChart>"
            )
        }
        OoxmlGroup::Doughnut => {
            format!(
                "<c:doughnutChart><c:varyColors val=\"1\"/>{sers}{dlbls}<c:firstSliceAng val=\"0\"/><c:holeSize val=\"50\"/></c:doughnutChart>"
            )
        }
        OoxmlGroup::Area { grouping } => {
            format!(
                "<c:areaChart><c:grouping val=\"{grouping}\"/><c:varyColors val=\"0\"/>{sers}{dlbls}{axes}</c:areaChart>"
            )
        }
        OoxmlGroup::Scatter { .. } => {
            format!(
                "<c:scatterChart><c:scatterStyle val=\"lineMarker\"/><c:varyColors val=\"0\"/>{sers}{dlbls}{axes}</c:scatterChart>"
            )
        }
        OoxmlGroup::Bubble => {
            format!(
                "<c:bubbleChart><c:varyColors val=\"0\"/>{sers}{dlbls}<c:bubbleScale val=\"100\"/><c:showNegBubbles val=\"0\"/>{axes}</c:bubbleChart>"
            )
        }
        OoxmlGroup::Radar => {
            format!(
                "<c:radarChart><c:radarStyle val=\"marker\"/><c:varyColors val=\"0\"/>{sers}{dlbls}{axes}</c:radarChart>"
            )
        }
        OoxmlGroup::Stock => {
            format!("<c:stockChart>{sers}{dlbls}<c:hiLowLines/>{axes}</c:stockChart>")
        }
    }
}

fn ooxml_series_xml(
    idx: usize,
    ser: &ResolvedSeriesCache,
    group: OoxmlGroup,
    categories: &[String],
) -> String {
    let mut x = format!("<c:ser><c:idx val=\"{idx}\"/><c:order val=\"{idx}\"/>");
    let _ = write!(
        x,
        "<c:tx><c:strRef><c:f>{}</c:f><c:strCache><c:ptCount val=\"1\"/><c:pt idx=\"0\"><c:v>{}</c:v></c:pt></c:strCache></c:strRef></c:tx>",
        xml_escape(&ser.name_ref),
        xml_escape(&ser.name)
    );

    if let Some(hex) = &ser.color_hex {
        let line_like = matches!(
            group,
            OoxmlGroup::Line { .. } | OoxmlGroup::Radar | OoxmlGroup::Scatter { lines: true }
        );
        if line_like {
            let _ = write!(
                x,
                "<c:spPr><a:ln w=\"28575\" cap=\"rnd\"><a:solidFill><a:srgbClr val=\"{hex}\"/></a:solidFill><a:round/></a:ln></c:spPr>"
            );
        } else if group == (OoxmlGroup::Scatter { lines: false }) {
            let _ = write!(
                x,
                "<c:spPr><a:ln w=\"19050\"><a:noFill/></a:ln></c:spPr><c:marker><c:symbol val=\"circle\"/><c:size val=\"5\"/><c:spPr><a:solidFill><a:srgbClr val=\"{hex}\"/></a:solidFill></c:spPr></c:marker>"
            );
        } else {
            let _ = write!(
                x,
                "<c:spPr><a:solidFill><a:srgbClr val=\"{hex}\"/></a:solidFill></c:spPr>"
            );
        }
    }

    match group {
        OoxmlGroup::Bar { .. } | OoxmlGroup::Bubble => {
            x.push_str("<c:invertIfNegative val=\"0\"/>");
        }
        OoxmlGroup::Line { markers: false, .. }
        | OoxmlGroup::Scatter { lines: true }
        | OoxmlGroup::Stock => {
            x.push_str("<c:marker><c:symbol val=\"none\"/></c:marker>");
        }
        _ => {}
    }

    if !matches!(group, OoxmlGroup::Pie | OoxmlGroup::Doughnut | OoxmlGroup::Radar) {
        for tl in &ser.trendlines {
            x.push_str(&ooxml_trendline_xml(tl));
        }
    }

    let cat_f = if ser.cat_ref.is_empty() {
        "Sheet1!$A$2:$A$2"
    } else {
        ser.cat_ref.as_str()
    };
    let val_f = if ser.val_ref.is_empty() {
        "Sheet1!$B$2:$B$2"
    } else {
        ser.val_ref.as_str()
    };

    match group {
        OoxmlGroup::Scatter { .. } | OoxmlGroup::Bubble => {
            x.push_str(&ooxml_str_cache_xml("xVal", cat_f, categories));
            x.push_str(&ooxml_num_cache_xml("yVal", val_f, &ser.values));
            if group == OoxmlGroup::Bubble {
                x.push_str(&ooxml_num_cache_xml("bubbleSize", val_f, &ser.values));
                x.push_str("<c:bubble3D val=\"0\"/>");
            } else {
                x.push_str("<c:smooth val=\"0\"/>");
            }
        }
        _ => {
            x.push_str(&ooxml_str_cache_xml("cat", cat_f, categories));
            x.push_str(&ooxml_num_cache_xml("val", val_f, &ser.values));
            if matches!(group, OoxmlGroup::Line { .. }) {
                x.push_str("<c:smooth val=\"0\"/>");
            }
        }
    }

    x.push_str("</c:ser>");
    x
}

fn ooxml_str_cache_xml(tag: &str, formula_ref: &str, items: &[String]) -> String {
    let mut s = format!(
        "<c:{tag}><c:strRef><c:f>{}</c:f><c:strCache><c:ptCount val=\"{}\"/>",
        xml_escape(formula_ref),
        items.len()
    );
    for (i, item) in items.iter().enumerate() {
        let cleaned = clean_xml_chars(item);
        let _ = write!(s, "<c:pt idx=\"{i}\"><c:v>{}</c:v></c:pt>", xml_escape(&cleaned));
    }
    let _ = write!(s, "</c:strCache></c:strRef></c:{tag}>");
    s
}

fn ooxml_num_cache_xml(tag: &str, formula_ref: &str, values: &[Option<f64>]) -> String {
    let mut s = format!(
        "<c:{tag}><c:numRef><c:f>{}</c:f><c:numCache><c:formatCode>General</c:formatCode><c:ptCount val=\"{}\"/>",
        xml_escape(formula_ref),
        values.len()
    );
    for (i, v) in values.iter().enumerate() {
        if let Some(n) = v.filter(|n| n.is_finite()) {
            let formatted = gridcraft_core::number_to_text(n);
            let _ = write!(s, "<c:pt idx=\"{i}\"><c:v>{formatted}</c:v></c:pt>");
        }
    }
    let _ = write!(s, "</c:numCache></c:numRef></c:{tag}>");
    s
}

fn ooxml_trendline_xml(tl: &Trendline) -> String {
    let mut s = String::from("<c:trendline>");
    if let Some(name) = &tl.name {
        let cleaned = clean_xml_chars(name);
        let _ = write!(s, "<c:name>{}</c:name>", xml_escape(&cleaned));
    }
    let ty = match tl.kind {
        TrendlineKind::Linear => "linear",
        TrendlineKind::Exponential => "exp",
        TrendlineKind::Logarithmic => "log",
        TrendlineKind::Polynomial(_) => "poly",
        TrendlineKind::Power => "power",
        TrendlineKind::MovingAverage(_) => "movingAvg",
    };
    let _ = write!(s, "<c:trendlineType val=\"{ty}\"/>");
    match tl.kind {
        TrendlineKind::Polynomial(order) => {
            let _ = write!(s, "<c:order val=\"{}\"/>", order.clamp(2, 6));
        }
        TrendlineKind::MovingAverage(period) => {
            let _ = write!(s, "<c:period val=\"{}\"/>", period.clamp(2, 255));
        }
        _ => {}
    }
    s.push_str("<c:dispRSqr val=\"0\"/><c:dispEq val=\"0\"/></c:trendline>");
    s
}

fn ooxml_title_xml(title: &str) -> String {
    let mut s = String::from("<c:title><c:tx><c:rich><a:bodyPr/><a:lstStyle/>");
    for line in title.split('\n') {
        let cleaned = clean_xml_chars(line);
        let _ = write!(
            s,
            "<a:p><a:pPr><a:defRPr/></a:pPr><a:r><a:t>{}</a:t></a:r></a:p>",
            xml_escape(&cleaned)
        );
    }
    s.push_str("</c:rich></c:tx><c:overlay val=\"0\"/></c:title>");
    s
}

fn ooxml_axis_xml(cat_axis: bool, id: u32, cross: u32, pos: &str, gridlines: bool) -> String {
    let tag = if cat_axis { "catAx" } else { "valAx" };
    let mut s = format!(
        "<c:{tag}><c:axId val=\"{id}\"/><c:scaling><c:orientation val=\"minMax\"/></c:scaling><c:delete val=\"0\"/><c:axPos val=\"{pos}\"/>"
    );
    if gridlines {
        s.push_str("<c:majorGridlines/>");
    }
    let _ = write!(
        s,
        "<c:numFmt formatCode=\"General\" sourceLinked=\"1\"/><c:majorTickMark val=\"out\"/><c:minorTickMark val=\"none\"/><c:tickLblPos val=\"nextTo\"/><c:crossAx val=\"{cross}\"/><c:crosses val=\"autoZero\"/>"
    );
    if cat_axis {
        s.push_str("<c:auto val=\"1\"/><c:lblAlgn val=\"ctr\"/><c:lblOffset val=\"100\"/><c:noMultiLvlLbl val=\"0\"/></c:catAx>");
    } else {
        s.push_str("<c:crossBetween val=\"between\"/></c:valAx>");
    }
    s
}

/// Strips XML 1.0 illegal control characters while keeping `\t`, `\n`, and all valid Unicode.
fn clean_xml_chars(s: &str) -> String {
    s.chars()
        .filter(|&c| {
            c == '\t'
                || c == '\n'
                || ('\u{0020}'..='\u{D7FF}').contains(&c)
                || ('\u{E000}'..='\u{FFFD}').contains(&c)
                || ('\u{10000}'..='\u{10FFFF}').contains(&c)
        })
        .collect()
}

fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn assert_well_formed_xml(xml: &str) {
        let mut reader = quick_xml::Reader::from_str(xml);
        let mut depth = 0i32;
        loop {
            match reader.read_event() {
                Ok(quick_xml::events::Event::Start(_)) => depth += 1,
                Ok(quick_xml::events::Event::End(_)) => depth -= 1,
                Ok(quick_xml::events::Event::Eof) => break,
                Ok(_) => {}
                Err(err) => assert!(err.to_string().is_empty(), "malformed XML: {err}\n{xml}"),
            }
        }
        assert_eq!(depth, 0, "unbalanced XML tags in:\n{xml}");
    }

    #[test]
    fn all_26_chart_kinds_and_6_trendlines_build_svg_png_ooxml_and_xlsx() -> Result<(), BridgeError>
    {
        let kinds = list_chart_kinds();
        assert_eq!(kinds.len(), 26);

        let mut distinct_kinds = HashSet::new();
        for kind_name in &kinds {
            let parsed = parse_chart_kind(kind_name)?;
            // Exhaustiveness guard: if a 27th variant is ever added to ChartKind, compiler flags it.
            match parsed {
                ChartKind::ColumnClustered
                | ChartKind::ColumnStacked
                | ChartKind::ColumnStacked100
                | ChartKind::BarClustered
                | ChartKind::BarStacked
                | ChartKind::BarStacked100
                | ChartKind::Line
                | ChartKind::LineMarkers
                | ChartKind::LineStacked
                | ChartKind::Pie
                | ChartKind::Doughnut
                | ChartKind::Area
                | ChartKind::AreaStacked
                | ChartKind::AreaStacked100
                | ChartKind::Scatter
                | ChartKind::ScatterLines
                | ChartKind::Bubble
                | ChartKind::Radar
                | ChartKind::Histogram
                | ChartKind::Waterfall
                | ChartKind::Funnel
                | ChartKind::Treemap
                | ChartKind::Sunburst
                | ChartKind::BoxWhisker
                | ChartKind::Stock
                | ChartKind::Combo => {}
            }
            distinct_kinds.insert(parsed);

            let spec = ChartSpecInput {
                title: Some(format!("2026 사업 성과 분석 ({kind_name}) & <목표>")),
                kind: kind_name.to_ascii_uppercase(),
                categories: vec![
                    "1분기".into(),
                    "2분기".into(),
                    "3분기".into(),
                    "4분기".into(),
                ],
                series: vec![
                    ChartSeriesInput::new("시가", vec![110.0, 125.0, 140.0, 165.0]),
                    ChartSeriesInput::new("고가", vec![130.0, 150.0, 160.0, 190.0]),
                    ChartSeriesInput::new("저가", vec![95.0, 115.0, 120.0, 150.0]),
                    ChartSeriesInput::new("종가", vec![120.0, 145.0, 155.0, 180.0]),
                ],
                width_pt: 360.0,
                height_pt: 240.0,
                legend_pos: Some("bottom".into()),
                trendline: Some("linear".into()),
                dark_mode: false,
            };

            let bundle = UnifiedChartEngine::build_chart(&spec)?;
            assert!(!bundle.primitives.is_empty(), "{kind_name} primitives empty");

            // SVG verification
            assert!(bundle.svg.starts_with("<svg ") && bundle.svg.ends_with("</svg>"));
            assert!(bundle.svg.contains(KOREAN_CHART_FONT_STACK));
            if matches!(parsed, ChartKind::Pie | ChartKind::Doughnut | ChartKind::Sunburst) {
                assert!(
                    bundle.svg.contains("<path d=\"M ") && bundle.svg.contains(" A "),
                    "{kind_name} missing SVG wedge arc path"
                );
            }
            assert_well_formed_xml(&bundle.svg);

            // HiDPI 2.0x PNG verification (360x240 pt -> 720x480 px)
            assert!(
                bundle.png_bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
                "{kind_name} invalid PNG signature"
            );
            let decoded = image::load_from_memory(&bundle.png_bytes);
            assert!(decoded.is_ok(), "{kind_name} PNG decode failed");
            if let Ok(img) = decoded {
                assert_eq!((img.width(), img.height()), (720, 480));
            }

            // OOXML c:chartSpace verification
            assert!(bundle.ooxml_chart_xml.contains("<c:chartSpace "));
            assert!(bundle.ooxml_chart_xml.contains("<c:externalData r:id=\"rId1\"/>"));
            assert!(bundle.ooxml_chart_xml.contains("<c:strCache>"));
            assert!(bundle.ooxml_chart_xml.contains("<c:numCache>"));
            assert_well_formed_xml(&bundle.ooxml_chart_xml);

            // Embedded .xlsx sidecar verification (GAP-PPTX-01 #335)
            let (wb, _) = gridcraft_xlsx::read_xlsx(&bundle.embedded_xlsx_bytes)?;
            let chart_count = wb.sheet(0).map(|s| s.charts.len()).unwrap_or(0);
            assert_eq!(chart_count, 1, "{kind_name} embedded xlsx missing Chart");
        }
        assert_eq!(distinct_kinds.len(), 26);

        // Verify all 6 trendline kinds
        let tl_kinds = list_trendline_kinds();
        assert_eq!(tl_kinds.len(), 6);
        for tl in tl_kinds {
            let spec = ChartSpecInput {
                title: Some("추세선 검증".into()),
                kind: "line_markers".into(),
                categories: vec!["1".into(), "2".into(), "3".into(), "4".into(), "5".into()],
                series: vec![ChartSeriesInput::new(
                    "매출",
                    vec![10.0, 22.0, 35.0, 51.0, 70.0],
                )],
                width_pt: 320.0,
                height_pt: 200.0,
                legend_pos: Some("right".into()),
                trendline: Some(tl.to_string()),
                dark_mode: true,
            };
            let bundle = UnifiedChartEngine::build_chart(&spec)?;
            assert!(bundle.ooxml_chart_xml.contains("<c:trendline>"));
            assert!(bundle.svg.contains("#181C24"));
            assert_well_formed_xml(&bundle.svg);
            assert_well_formed_xml(&bundle.ooxml_chart_xml);
        }

        Ok(())
    }

    #[test]
    fn single_slice_full_circle_pie_and_doughnut_render_valid_svg_arcs() -> Result<(), BridgeError>
    {
        for kind in ["pie", "doughnut"] {
            let spec = ChartSpecInput {
                title: Some("단일 항목 100%".into()),
                kind: kind.into(),
                categories: vec!["독점 점유율".into()],
                series: vec![ChartSeriesInput::new("점유율", vec![100.0])],
                width_pt: 240.0,
                height_pt: 200.0,
                legend_pos: Some("bottom".into()),
                trendline: None,
                dark_mode: false,
            };
            let bundle = UnifiedChartEngine::build_chart(&spec)?;
            assert!(bundle.svg.contains("<path d=\"M "));
            assert_well_formed_xml(&bundle.svg);
        }
        Ok(())
    }

    #[test]
    fn hostile_chart_inputs_never_panic() {
        assert!(parse_chart_kind("").is_err());
        assert!(parse_chart_kind("3d_hologram").is_err());
        assert!(parse_trendline_kind("fourier").is_err());

        for (w, h) in [
            (0.0, 200.0),
            (200.0, -10.0),
            (f32::NAN, 200.0),
            (200.0, f32::INFINITY),
            (50_000.0, 50_000.0),
        ] {
            let bad = ChartSpecInput {
                title: None,
                kind: "column_clustered".into(),
                categories: vec![],
                series: vec![],
                width_pt: w,
                height_pt: h,
                legend_pos: None,
                trendline: None,
                dark_mode: false,
            };
            assert!(UnifiedChartEngine::build_chart(&bad).is_err());
        }

        // Hostile NaN/Infinity values, empty series, XML injection & NUL control chars in strings
        let hostile_data = ChartSpecInput {
            title: Some("악성\u{0000}\u{0007}<script>&\"'제목".into()),
            kind: "column_clustered".into(),
            categories: vec!["A\u{0001}<x>".into(), "B&\"'".into()],
            series: vec![ChartSeriesInput::new(
                "시리즈\u{0002}</c:ser>",
                vec![f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.0],
            )],
            width_pt: 280.0,
            height_pt: 180.0,
            legend_pos: Some("unknown_pos".into()),
            trendline: Some("poly99".into()),
            dark_mode: true,
        };
        let res = UnifiedChartEngine::build_chart(&hostile_data);
        assert!(res.is_ok());
        if let Ok(bundle) = res {
            assert_well_formed_xml(&bundle.svg);
            assert_well_formed_xml(&bundle.ooxml_chart_xml);
        }
    }
}
