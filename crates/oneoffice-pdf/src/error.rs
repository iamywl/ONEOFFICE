//! Error types for `oneoffice-pdf`.

use thiserror::Error;

/// Result type alias for operations in `oneoffice-pdf`.
pub type PdfResult<T> = Result<T, PdfError>;

/// Errors returned by `oneoffice-pdf` operations.
#[derive(Debug, Error)]
pub enum PdfError {
    /// PDF opening or reading error from `craft-pdf`.
    #[error("failed to open PDF: {0}")]
    Open(#[from] craft_pdf::OpenError),

    /// PDF writing or serialization error from `craft-pdf`.
    #[error("failed to write PDF: {0}")]
    Write(#[from] craft_pdf::write::WriteError),

    /// Page rasterization error from `craft-pdf`.
    #[error("failed to rasterize PDF page: {0}")]
    Raster(#[from] craft_pdf::raster::Error),

    /// Page rendering or surface compositing error.
    #[error("rendering error: {0}")]
    Render(String),

    /// The requested page index is out of bounds.
    #[error("page index {page_index} is out of bounds (document has {page_count} pages)")]
    PageOutOfBounds { page_index: usize, page_count: usize },

    /// Invalid image or page dimensions.
    #[error("invalid dimensions: {width}x{height}")]
    InvalidDimensions { width: u32, height: u32 },

    /// Image pixel buffer length does not match `width * height * 4`.
    #[error("buffer length mismatch: expected {expected} bytes, got {actual} bytes")]
    BufferLengthMismatch { expected: usize, actual: usize },

    /// Image decoding failed.
    #[error("image decoding failed: {0}")]
    ImageDecode(String),

    /// Image encoding failed.
    #[error("image encoding failed: {0}")]
    ImageEncode(String),

    /// Signature with the specified ID was not found in the layer stack.
    #[error("signature not found: '{0}'")]
    SignatureNotFound(String),

    /// Search or text parsing error.
    #[error("text search error: {0}")]
    Search(String),

    /// General I/O or serialization error.
    #[error("I/O error: {0}")]
    Io(String),
}
