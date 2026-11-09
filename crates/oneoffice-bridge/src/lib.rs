//! Unified Excel 513-formula calc, 26-type vector chart, and Korean HarfRust font bridge.
#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable
)]

pub mod calc;
pub mod chart;
pub mod error;
pub mod fonts;
pub mod sha256;

pub use calc::{
    CalcTableEngine, CalcTableOutput, CalcTableSpec, CellInput, FormulaDiagnostic, MergeSpan,
    ReconcileResult,
};
pub use chart::{
    ChartBundle, ChartSeriesInput, ChartSpecInput, UnifiedChartEngine, list_chart_kinds,
    parse_chart_kind,
};
pub use error::BridgeError;
pub use fonts::KoreanTextMeasurer;
pub use sha256::sha256_hex;
