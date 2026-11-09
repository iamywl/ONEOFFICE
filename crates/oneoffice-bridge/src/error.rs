//! Error types for the ONEOFFICE unified calculation, chart, and Korean typography bridge.

use thiserror::Error;

/// Unified error type returned by [`crate::calc::CalcTableEngine`],
/// [`crate::chart::UnifiedChartEngine`], and [`crate::fonts::KoreanTextMeasurer`].
#[derive(Debug, Error)]
pub enum BridgeError {
    /// Requested worksheet index does not exist in the workbook.
    #[error("sheet index {requested} out of bounds (workbook has {sheet_count} sheets)")]
    SheetOutOfBounds {
        /// Zero-based index requested by the caller.
        requested: usize,
        /// Number of sheets available in the workbook.
        sheet_count: usize,
    },

    /// A cell merge span has inverted coordinates, exceeds Excel limits, or overlaps another span.
    #[error("invalid merge span: {0}")]
    InvalidMergeSpan(String),

    /// Input table dimensions exceed Excel's sheet grid (`1,048,576 x 16,384`) or safety caps.
    #[error("grid dimensions ({rows} rows x {cols} cols) exceed supported spreadsheet limits")]
    GridDimensionsExceeded {
        /// Row count requested.
        rows: usize,
        /// Column count requested.
        cols: usize,
    },

    /// Spreadsheet `.xlsx` package read/write error from `gridcraft-xlsx`.
    #[error("xlsx I/O error: {0}")]
    XlsxIo(#[from] gridcraft_xlsx::IoError),

    /// Unrecognized chart type string.
    #[error("unsupported chart kind '{0}'")]
    InvalidChartKind(String),

    /// Unrecognized trendline regression type string.
    #[error("unsupported trendline kind '{0}'")]
    InvalidTrendlineKind(String),

    /// Non-finite, zero/negative, or oversized chart canvas dimensions.
    #[error("invalid chart dimensions ({width_pt} pt x {height_pt} pt); must be finite within 4..=4096 pt")]
    InvalidChartDimensions {
        /// Requested width in points.
        width_pt: f32,
        /// Requested height in points.
        height_pt: f32,
    },

    /// Failed to encode RGBA buffer into PNG format.
    #[error("PNG encoding failed: {0}")]
    PngEncode(String),

    /// Failed to install a runtime font file into the global font database.
    #[error("font installation failed: {0}")]
    FontInstall(String),
}
