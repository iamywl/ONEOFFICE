//! Error types for `oneoffice-hwpx-core`.

use thiserror::Error;

/// Result type alias for operations in `oneoffice-hwpx-core`.
pub type HwpxResult<T> = Result<T, HwpxError>;

/// Errors encountered during HWPX package parsing, XML manipulation, and compound serialization.
#[derive(Debug, Error)]
pub enum HwpxError {
    /// ZIP container read or write failure.
    #[error("ZIP package error: {0}")]
    Zip(String),

    /// XML parsing, validation, or serialization failure.
    #[error("XML parsing error: {0}")]
    Xml(String),

    /// General I/O or conversion error.
    #[error("I/O error: {0}")]
    Io(String),

    /// The specified section was not found in the HWPX package.
    #[error("HWPX section not found: Contents/section{0}.xml")]
    SectionNotFound(usize),

    /// The package does not conform to the KS X 6101 HWPX specification.
    #[error("Malformed HWPX package: {0}")]
    MalformedPackage(String),

    /// ZIP bomb or decompression limit exceeded.
    #[error("ZIP bomb detected or decompression quota exceeded: {0}")]
    ZipBomb(String),

    /// Staleness detected between rendered cell values and embedded calculation sidecar.
    #[error("Stale formula sidecar: expected hash {expected}, got {actual}")]
    StaleSidecar { expected: String, actual: String },

    /// Compound object (table formula, chart, diagram) embedding failure.
    #[error("Compound object error: {0}")]
    Compound(String),
}
