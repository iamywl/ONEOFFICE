//! `DiagramSpec` v1 semantic schema, theme presets, connector anchors, and color utilities.

use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

/// Canonical schema version string for `DiagramSpec` v1.
pub const DIAGRAM_SCHEMA_V1: &str = "oneoffice-diagram/v1";

/// Default A4 portrait body content width in points (`42,520 HWPUNIT = 425.2 pt`).
pub const DEFAULT_A4_BODY_WIDTH_PT: f32 = 425.2;

/// Default 16:9 widescreen slide content width in points (`960.0 pt`).
pub const DEFAULT_SLIDE_WIDTH_PT: f32 = 960.0;

/// Errors returned by `oneoffice-diagram` operations.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum DiagramError {
    /// JSON parsing or serialization failed.
    #[error("invalid DiagramSpec JSON: {0}")]
    InvalidJson(String),
    /// Requested template ID does not exist.
    #[error("unknown diagram template: '{0}'")]
    UnknownTemplate(String),
    /// Requested theme preset ID is unrecognized.
    #[error("unknown diagram theme preset: '{0}'")]
    UnknownThemePreset(String),
    /// Target node ID was not found during `patch_diagram`.
    #[error("diagram node not found: '{0}'")]
    NodeNotFound(String),
    /// Patch field is not applicable to the target node variant.
    #[error("unsupported patch field '{field}' on node '{node_id}'")]
    UnsupportedPatchField { node_id: String, field: String },
    /// Canvas width is non-finite or non-positive.
    #[error("invalid canvas width: {0} pt")]
    InvalidCanvasWidth(f32),
    /// Raster scale factor is non-finite or non-positive.
    #[error("invalid PNG render scale: {0}")]
    InvalidRenderScale(f32),
    /// PNG rasterization or image encoding failed.
    #[error("PNG rasterization failed: {0}")]
    RasterError(String),
    /// XML compilation failed.
    #[error("XML compilation failed: {0}")]
    XmlError(String),
}

/// Parsed RGBA8 color value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl RgbColor {
    pub const WHITE: Self = Self { r: 255, g: 255, b: 255, a: 255 };
    pub const BLACK: Self = Self { r: 15, g: 23, b: 42, a: 255 };

    #[must_use]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    #[must_use]
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Formats as `#RRGGBB`.
    #[must_use]
    pub fn to_hex6(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    /// Formats as `#RRGGBB` (alias for [`to_hex6`]).
    #[must_use]
    pub fn to_hex(self) -> String {
        self.to_hex6()
    }

    /// Parses `#RRGGBB` or returns `fallback` if invalid.
    #[must_use]
    pub fn parse_hex_or(input: &str, fallback: Self) -> Self {
        parse_hex_color(input).unwrap_or(fallback)
    }

    /// Formats as bare `RRGGBB` (used by ECMA-376 DrawingML `<a:srgbClr val="..."/>`).
    #[must_use]
    pub fn to_hex6_bare(self) -> String {
        format!("{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    /// WCAG relative luminance (`0.0..=1.0`).
    #[must_use]
    pub fn relative_luminance(self) -> f32 {
        let lin = |v: u8| {
            let s = f32::from(v) / 255.0;
            if s <= 0.04045 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * lin(self.r) + 0.7152 * lin(self.g) + 0.0722 * lin(self.b)
    }

    /// WCAG 2.1 contrast ratio against `other` (`1.0..=21.0`).
    #[must_use]
    pub fn contrast_ratio(self, other: Self) -> f32 {
        let l1 = self.relative_luminance();
        let l2 = other.relative_luminance();
        (l1.max(l2) + 0.05) / (l1.min(l2) + 0.05)
    }
}

/// Parses `#RRGGBB`, `#RRGGBBAA`, `#RGB`, or bare hex strings without panicking.
#[must_use]
pub fn parse_hex_color(input: &str) -> Option<RgbColor> {
    let raw = input.trim().strip_prefix('#').unwrap_or_else(|| input.trim());
    match raw.len() {
        3 => {
            let r = u8::from_str_radix(raw.get(0..1)?, 16).ok()?;
            let g = u8::from_str_radix(raw.get(1..2)?, 16).ok()?;
            let b = u8::from_str_radix(raw.get(2..3)?, 16).ok()?;
            Some(RgbColor::rgb(r * 17, g * 17, b * 17))
        }
        6 => {
            let r = u8::from_str_radix(raw.get(0..2)?, 16).ok()?;
            let g = u8::from_str_radix(raw.get(2..4)?, 16).ok()?;
            let b = u8::from_str_radix(raw.get(4..6)?, 16).ok()?;
            Some(RgbColor::rgb(r, g, b))
        }
        8 => {
            let r = u8::from_str_radix(raw.get(0..2)?, 16).ok()?;
            let g = u8::from_str_radix(raw.get(2..4)?, 16).ok()?;
            let b = u8::from_str_radix(raw.get(4..6)?, 16).ok()?;
            let a = u8::from_str_radix(raw.get(6..8)?, 16).ok()?;
            Some(RgbColor::rgba(r, g, b, a))
        }
        _ => None,
    }
}

/// Parses a hex color string or returns `fallback` when invalid.
#[must_use]
pub fn parse_hex_or(input: &str, fallback: RgbColor) -> RgbColor {
    parse_hex_color(input).unwrap_or(fallback)
}

/// Built-in consulting theme preset identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemePreset {
    #[serde(rename = "consulting-navy-blue")]
    ConsultingNavyBlue,
    #[serde(rename = "public-rfp-emerald")]
    PublicRfpEmerald,
    #[serde(rename = "executive-slate-indigo")]
    ExecutiveSlateIndigo,
    #[serde(rename = "dark-boardroom")]
    DarkBoardroom,
}

impl ThemePreset {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ConsultingNavyBlue => "consulting-navy-blue",
            Self::PublicRfpEmerald => "public-rfp-emerald",
            Self::ExecutiveSlateIndigo => "executive-slate-indigo",
            Self::DarkBoardroom => "dark-boardroom",
        }
    }

    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        match id.trim() {
            "consulting-navy-blue" => Some(Self::ConsultingNavyBlue),
            "public-rfp-emerald" => Some(Self::PublicRfpEmerald),
            "executive-slate-indigo" => Some(Self::ExecutiveSlateIndigo),
            "dark-boardroom" => Some(Self::DarkBoardroom),
            _ => None,
        }
    }

    #[must_use]
    pub const fn all() -> [Self; 4] {
        [
            Self::ConsultingNavyBlue,
            Self::PublicRfpEmerald,
            Self::ExecutiveSlateIndigo,
            Self::DarkBoardroom,
        ]
    }
}

/// Semantic color and typography palette for a `DiagramSpec`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagramTheme {
    pub preset: String,
    pub primary: String,
    pub secondary: String,
    pub hero_bg: String,
    pub hero_text: String,
    pub card_bg: String,
    pub card_border: String,
    pub subtle_fill: String,
    pub implication_fill: String,
    pub text_primary: String,
    pub text_muted: String,
    pub accent_badge_bg: String,
    pub accent_badge_text: String,
    pub font_family: String,
}

impl Default for DiagramTheme {
    fn default() -> Self {
        Self::from_preset_enum(ThemePreset::ConsultingNavyBlue)
    }
}

impl DiagramTheme {
    /// Instantiates one of the 4 standard consulting presets.
    #[must_use]
    pub fn from_preset_enum(preset: ThemePreset) -> Self {
        match preset {
            ThemePreset::ConsultingNavyBlue => Self {
                preset: preset.as_str().to_owned(),
                primary: "#0052CC".to_owned(),
                secondary: "#0284C7".to_owned(),
                hero_bg: "#0F2942".to_owned(),
                hero_text: "#FFFFFF".to_owned(),
                card_bg: "#FFFFFF".to_owned(),
                card_border: "#CBD5E1".to_owned(),
                subtle_fill: "#F1F5F9".to_owned(),
                implication_fill: "#E0F2FE".to_owned(),
                text_primary: "#0F172A".to_owned(),
                text_muted: "#475569".to_owned(),
                accent_badge_bg: "#DBEAFE".to_owned(),
                accent_badge_text: "#1D4ED8".to_owned(),
                font_family: "Pretendard".to_owned(),
            },
            ThemePreset::PublicRfpEmerald => Self {
                preset: preset.as_str().to_owned(),
                primary: "#047857".to_owned(),
                secondary: "#0D9488".to_owned(),
                hero_bg: "#064E3B".to_owned(),
                hero_text: "#FFFFFF".to_owned(),
                card_bg: "#FFFFFF".to_owned(),
                card_border: "#CBD5E1".to_owned(),
                subtle_fill: "#F0FDF4".to_owned(),
                implication_fill: "#D1FAE5".to_owned(),
                text_primary: "#0F172A".to_owned(),
                text_muted: "#475569".to_owned(),
                accent_badge_bg: "#DCFCE7".to_owned(),
                accent_badge_text: "#047857".to_owned(),
                font_family: "Pretendard".to_owned(),
            },
            ThemePreset::ExecutiveSlateIndigo => Self {
                preset: preset.as_str().to_owned(),
                primary: "#4338CA".to_owned(),
                secondary: "#6366F1".to_owned(),
                hero_bg: "#1E1B4B".to_owned(),
                hero_text: "#FFFFFF".to_owned(),
                card_bg: "#FFFFFF".to_owned(),
                card_border: "#CBD5E1".to_owned(),
                subtle_fill: "#F8FAFC".to_owned(),
                implication_fill: "#E0E7FF".to_owned(),
                text_primary: "#0F172A".to_owned(),
                text_muted: "#475569".to_owned(),
                accent_badge_bg: "#E0E7FF".to_owned(),
                accent_badge_text: "#3730A3".to_owned(),
                font_family: "Pretendard".to_owned(),
            },
            ThemePreset::DarkBoardroom => Self {
                preset: preset.as_str().to_owned(),
                primary: "#38BDF8".to_owned(),
                secondary: "#818CF8".to_owned(),
                hero_bg: "#020617".to_owned(),
                hero_text: "#F8FAFC".to_owned(),
                card_bg: "#0F172A".to_owned(),
                card_border: "#334155".to_owned(),
                subtle_fill: "#1E293B".to_owned(),
                implication_fill: "#172554".to_owned(),
                text_primary: "#F8FAFC".to_owned(),
                text_muted: "#94A3B8".to_owned(),
                accent_badge_bg: "#1E3A8A".to_owned(),
                accent_badge_text: "#93C5FD".to_owned(),
                font_family: "Pretendard".to_owned(),
            },
        }
    }

    /// Instantiates a theme by preset identifier string (`None` if unknown).
    #[must_use]
    pub fn from_preset(id: &str) -> Option<Self> {
        ThemePreset::from_id(id).map(Self::from_preset_enum)
    }

    /// Whether this theme uses a dark canvas background.
    #[must_use]
    pub fn is_dark(&self) -> bool {
        self.preset == ThemePreset::DarkBoardroom.as_str()
            || parse_hex_color(&self.card_bg)
                .is_some_and(|c| c.relative_luminance() < 0.2)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawDiagramTheme {
    #[serde(default)]
    preset: Option<String>,
    #[serde(default)]
    primary: Option<String>,
    #[serde(default)]
    secondary: Option<String>,
    #[serde(default, alias = "hero_bg")]
    hero_bg: Option<String>,
    #[serde(default, alias = "hero_text")]
    hero_text: Option<String>,
    #[serde(default, alias = "card_bg")]
    card_bg: Option<String>,
    #[serde(default, alias = "card_border")]
    card_border: Option<String>,
    #[serde(default, alias = "subtle_fill")]
    subtle_fill: Option<String>,
    #[serde(default, alias = "implication_fill")]
    implication_fill: Option<String>,
    #[serde(default, alias = "text_primary")]
    text_primary: Option<String>,
    #[serde(default, alias = "text_muted")]
    text_muted: Option<String>,
    #[serde(default, alias = "accent_badge_bg")]
    accent_badge_bg: Option<String>,
    #[serde(default, alias = "accent_badge_text")]
    accent_badge_text: Option<String>,
    #[serde(default, alias = "font_family")]
    font_family: Option<String>,
}

impl<'de> Deserialize<'de> for DiagramTheme {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawDiagramTheme::deserialize(deserializer)?;
        let preset_name = raw
            .preset
            .clone()
            .unwrap_or_else(|| ThemePreset::ConsultingNavyBlue.as_str().to_owned());
        let mut base = Self::from_preset(&preset_name).unwrap_or_default();
        base.preset = preset_name;
        if let Some(v) = raw.primary {
            base.primary = v;
        }
        if let Some(v) = raw.secondary {
            base.secondary = v;
        }
        if let Some(v) = raw.hero_bg {
            base.hero_bg = v;
        }
        if let Some(v) = raw.hero_text {
            base.hero_text = v;
        }
        if let Some(v) = raw.card_bg {
            base.card_bg = v;
        }
        if let Some(v) = raw.card_border {
            base.card_border = v;
        }
        if let Some(v) = raw.subtle_fill {
            base.subtle_fill = v;
        }
        if let Some(v) = raw.implication_fill {
            base.implication_fill = v;
        }
        if let Some(v) = raw.text_primary {
            base.text_primary = v;
        }
        if let Some(v) = raw.text_muted {
            base.text_muted = v;
        }
        if let Some(v) = raw.accent_badge_bg {
            base.accent_badge_bg = v;
        }
        if let Some(v) = raw.accent_badge_text {
            base.accent_badge_text = v;
        }
        if let Some(v) = raw.font_family {
            base.font_family = v;
        }
        Ok(base)
    }
}

const fn default_canvas_width() -> f32 {
    DEFAULT_A4_BODY_WIDTH_PT
}

fn default_schema_version() -> String {
    DIAGRAM_SCHEMA_V1.to_owned()
}

fn default_diagram_id() -> String {
    "diag_01".to_owned()
}

const fn default_vstack_gap() -> f32 {
    12.0
}

const fn default_hstack_gap() -> f32 {
    12.0
}

const fn default_grid_cols() -> usize {
    2
}

const fn default_true() -> bool {
    true
}

const fn default_weight() -> f32 {
    1.0
}

/// Target canvas geometry for layout solving.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagramCanvas {
    #[serde(default = "default_canvas_width", alias = "target_width_pt")]
    pub target_width_pt: f32,
    #[serde(default, alias = "padding_pt")]
    pub padding_pt: f32,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "bg_color")]
    pub bg_color: Option<String>,
}

impl Default for DiagramCanvas {
    fn default() -> Self {
        Self {
            target_width_pt: DEFAULT_A4_BODY_WIDTH_PT,
            padding_pt: 0.0,
            bg_color: None,
        }
    }
}

/// Top-level `DiagramSpec` v1 document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagramSpec {
    #[serde(default = "default_schema_version", alias = "schema_version")]
    pub schema_version: String,
    #[serde(default = "default_diagram_id")]
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default)]
    pub theme: DiagramTheme,
    #[serde(default)]
    pub canvas: DiagramCanvas,
    pub root: DiagramNode,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub connectors: Vec<GluedConnector>,
}

impl DiagramSpec {
    /// Parses a `DiagramSpec` from JSON text.
    pub fn from_json_str(json: &str) -> Result<Self, DiagramError> {
        serde_json::from_str(json).map_err(|e| DiagramError::InvalidJson(e.to_string()))
    }

    /// Serializes this `DiagramSpec` to pretty JSON text.
    pub fn to_json_pretty(&self) -> Result<String, DiagramError> {
        serde_json::to_string_pretty(self).map_err(|e| DiagramError::InvalidJson(e.to_string()))
    }
}

/// Port anchor positions on a diagram node's bounding box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortAnchor {
    BottomCenter,
    TopCenter,
    TopQuarterLeft,
    TopQuarterRight,
    BottomQuarterLeft,
    BottomQuarterRight,
    LeftCenter,
    RightCenter,
}

/// Endpoint of a [`GluedConnector`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectorEndpoint {
    #[serde(alias = "node_id")]
    pub node_id: String,
    pub port: PortAnchor,
}

/// Routing and stroke style of a [`GluedConnector`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ConnectorStyle {
    #[default]
    #[serde(rename = "straightArrow", alias = "straight_arrow", alias = "StraightArrow")]
    StraightArrow,
    #[serde(rename = "elbowArrow", alias = "elbow_arrow", alias = "ElbowArrow")]
    ElbowArrow,
    #[serde(rename = "dashedArrow", alias = "dashed_arrow", alias = "DashedArrow")]
    DashedArrow,
    #[serde(rename = "lineOnly", alias = "line_only", alias = "LineOnly")]
    LineOnly,
}

pub type ConnectorPort = PortAnchor;
pub type ConnectorAnchor = PortAnchor;
pub type PortRef = ConnectorEndpoint;

/// Visual variant for comparison and transformation elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ComparisonVariant {
    #[default]
    Neutral,
    Problem,
    Solution,
    Positive,
    Negative,
}

/// Port-glued arrow connector linking two semantic nodes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GluedConnector {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub from: ConnectorEndpoint,
    pub to: ConnectorEndpoint,
    #[serde(default)]
    pub style: ConnectorStyle,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

/// Big KPI metric callout inside a [`DiagramNode::ComparisonCard`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KpiCallout {
    pub metric: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sublabel: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cite: Option<String>,
}

/// Bullet item supporting inline `**bold**` spans and right-aligned `[N]` citation tags.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceBullet {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cite: Option<String>,
}

impl EvidenceBullet {
    #[must_use]
    pub fn new(text: impl Into<String>, cite: Option<&str>) -> Self {
        Self {
            text: text.into(),
            cite: cite.map(str::to_owned),
        }
    }
}

/// Bottom-aligned implication / takeaway box (`시사점`) inside a [`DiagramNode::ComparisonCard`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImplicationBox {
    pub title: String,
    pub body: String,
}

/// Span header grouping consecutive steps inside a [`DiagramNode::HeroContainer`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepGroupLabel {
    #[serde(alias = "start_step")]
    pub start_step: usize,
    #[serde(alias = "end_step")]
    pub end_step: usize,
    pub label: String,
}

/// One step card inside a [`DiagramNode::HeroContainer`]'s `step_chain`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessStep {
    pub step: usize,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub badge: Option<String>,
    #[serde(default = "default_weight")]
    pub weight: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bullets: Vec<String>,
}

/// Metric card inside a [`DiagramNode::KpiRow`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KpiMetricCard {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub label: String,
    pub metric: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cite: Option<String>,
}

/// Bullet point within an As-Is or To-Be column.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AsIsToBeItem {
    pub headline: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Horizontal tier inside a [`DiagramNode::ArchitectureStack`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchitectureLayer {
    #[serde(default)]
    pub level: usize,
    #[serde(alias = "title")]
    pub name: String,
    #[serde(default, alias = "modules")]
    pub components: Vec<String>,
}
pub type ArchLayer = ArchitectureLayer;

/// Item within a Gantt track.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GanttItem {
    pub label: String,
    pub start_idx: usize,
    pub span_count: usize,
    #[serde(default)]
    pub completed: bool,
}

/// Workstream bar inside a [`DiagramNode::GanttRoadmap`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GanttTrack {
    pub name: String,
    #[serde(default)]
    pub items: Vec<GanttItem>,
}

/// Source reference item inside a [`DiagramNode::CitationFooter`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationEntry {
    pub index: String,
    pub source: String,
}

/// Stage row inside a [`DiagramNode::FunnelStages`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunnelStage {
    #[serde(alias = "stage")]
    pub stage_name: String,
    #[serde(alias = "description")]
    pub count_label: String,
    #[serde(alias = "rate")]
    pub conversion_pct: String,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "action")]
    pub strategic_action: Option<String>,
}

/// Team/unit box inside a [`DiagramNode::OrgTree`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrgUnit {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub role: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headcount: Option<String>,
    #[serde(default)]
    pub responsibilities: Vec<String>,
}

#[allow(dead_code)]
fn default_as_is_badge() -> String {
    "AS-IS · 현행 한계".to_owned()
}

#[allow(dead_code)]
fn default_bridge_label() -> String {
    "핵심 전환 과제".to_owned()
}

#[allow(dead_code)]
fn default_to_be_badge() -> String {
    "TO-BE · 목표 모델".to_owned()
}

/// Composable semantic diagram node tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum DiagramNode {
    /// Vertical flow container.
    #[serde(rename_all = "camelCase")]
    VStack {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default = "default_vstack_gap", alias = "gap_pt")]
        gap_pt: f32,
        #[serde(default, alias = "padding_pt")]
        padding_pt: f32,
        #[serde(default)]
        children: Vec<DiagramNode>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        connectors: Vec<GluedConnector>,
    },

    /// Horizontal equal-height container.
    #[serde(rename_all = "camelCase")]
    HStack {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default = "default_hstack_gap", alias = "gap_pt")]
        gap_pt: f32,
        #[serde(default = "default_true", alias = "equalize_heights")]
        equalize_heights: bool,
        #[serde(default)]
        children: Vec<DiagramNode>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        connectors: Vec<GluedConnector>,
    },

    /// Multi-column consulting grid with automatic row height & implication baseline equalization.
    #[serde(rename_all = "camelCase")]
    Grid {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default = "default_grid_cols")]
        columns: usize,
        #[serde(default = "default_hstack_gap", alias = "gap_pt")]
        gap_pt: f32,
        #[serde(default = "default_true", alias = "equalize_card_heights")]
        equalize_card_heights: bool,
        #[serde(default)]
        children: Vec<DiagramNode>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        connectors: Vec<GluedConnector>,
    },

    /// Top proposal meta header bar (`ONEOFFICE × RFP 2026 | ...` + right capsule tag).
    #[serde(rename_all = "camelCase")]
    HeaderBar {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(alias = "left_text")]
        left_text: String,
        #[serde(default, skip_serializing_if = "Option::is_none", alias = "right_badge")]
        right_badge: Option<String>,
        #[serde(default = "default_true", alias = "show_divider")]
        show_divider: bool,
    },

    /// Consulting section title block (`00 / WHY NOW` + headline + 2-line lead copy).
    #[serde(rename_all = "camelCase")]
    SectionHeading {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        eyebrow: Option<String>,
        title: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        lead: Option<String>,
    },

    /// Equalized problem/option comparison card with badge, KPI callout, bold evidence bullets, and bottom `시사점` box.
    #[serde(rename_all = "camelCase")]
    ComparisonCard {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        badge: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        category: Option<String>,
        headline: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kpi: Option<KpiCallout>,
        #[serde(default, skip_serializing_if = "Option::is_none", alias = "bullets_title")]
        bullets_title: Option<String>,
        #[serde(default)]
        bullets: Vec<EvidenceBullet>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        implication: Option<ImplicationBox>,
        #[serde(default, skip_serializing_if = "Option::is_none", alias = "accent_color")]
        accent_color: Option<String>,
    },

    /// Deep-navy solution hero container with optional group labels and `①..④` chevron step chain.
    #[serde(rename_all = "camelCase")]
    HeroContainer {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        eyebrow: Option<String>,
        headline: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        subheadline: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty", alias = "group_labels")]
        group_labels: Vec<StepGroupLabel>,
        #[serde(default, alias = "step_chain")]
        step_chain: Vec<ProcessStep>,
    },

    /// Horizontal row of executive KPI callout cards.
    #[serde(rename_all = "camelCase")]
    KpiRow {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default = "default_hstack_gap", alias = "gap_pt")]
        gap_pt: f32,
        #[serde(default)]
        items: Vec<KpiMetricCard>,
    },

    /// 3-column `AS-IS -> Bridge Pillars -> TO-BE` transformation diagram.
    #[serde(rename_all = "camelCase")]
    AsIsToBe {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(alias = "as_is_title")]
        as_is_title: String,
        #[serde(default, alias = "as_is_items")]
        as_is_items: Vec<AsIsToBeItem>,
        #[serde(alias = "catalyst_label", alias = "bridge_label")]
        catalyst_label: String,
        #[serde(alias = "to_be_title")]
        to_be_title: String,
        #[serde(default, alias = "to_be_items")]
        to_be_items: Vec<AsIsToBeItem>,
    },

    /// Multi-layer technical/service architecture stack.
    #[serde(rename_all = "camelCase")]
    ArchitectureStack {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default)]
        layers: Vec<ArchitectureLayer>,
    },

    /// Consulting milestone & phase Gantt roadmap table.
    #[serde(rename_all = "camelCase")]
    GanttRoadmap {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default, alias = "time_periods", alias = "periods")]
        time_periods: Vec<String>,
        #[serde(default)]
        tracks: Vec<GanttTrack>,
    },

    /// 1- or 2-column source citation bar with optional right-aligned page number (`01 / 05`).
    #[serde(rename_all = "camelCase")]
    CitationFooter {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default)]
        citations: Vec<CitationEntry>,
        #[serde(default, skip_serializing_if = "Option::is_none", alias = "page_indicator")]
        page_indicator: Option<String>,
    },

    /// Tapered conversion funnel with stage rates and right-side strategic levers.
    #[serde(rename_all = "camelCase")]
    FunnelStages {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default)]
        stages: Vec<FunnelStage>,
    },

    /// Project PMO / governance hierarchy tree with orthogonal connectors.
    #[serde(rename_all = "camelCase")]
    OrgTree {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        lead: OrgUnit,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        advisors: Vec<OrgUnit>,
        #[serde(default)]
        teams: Vec<OrgUnit>,
    },
}

impl DiagramNode {
    /// Returns the explicit `id` of this node if set.
    #[must_use]
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::VStack { id, .. }
            | Self::HStack { id, .. }
            | Self::Grid { id, .. }
            | Self::HeaderBar { id, .. }
            | Self::SectionHeading { id, .. }
            | Self::ComparisonCard { id, .. }
            | Self::HeroContainer { id, .. }
            | Self::KpiRow { id, .. }
            | Self::AsIsToBe { id, .. }
            | Self::ArchitectureStack { id, .. }
            | Self::GanttRoadmap { id, .. }
            | Self::CitationFooter { id, .. }
            | Self::FunnelStages { id, .. }
            | Self::OrgTree { id, .. } => id.as_deref(),
        }
    }
}
