//! Error types for `oneoffice-mcp`.

use thiserror::Error;

/// Result type alias for operations in `oneoffice-mcp`.
pub type McpResult<T> = Result<T, McpError>;

/// Errors encountered during MCP request parsing and tool execution.
#[derive(Debug, Error)]
pub enum McpError {
    /// JSON serialization or deserialization failure.
    #[error("JSON RPC error: {0}")]
    Json(#[from] serde_json::Error),

    /// Base64 decode error.
    #[error("Base64 error: {0}")]
    Base64(String),

    /// Tool not found.
    #[error("Tool not found: '{0}'")]
    ToolNotFound(String),

    /// Invalid tool arguments.
    #[error("Invalid arguments for '{tool}': {details}")]
    InvalidArguments { tool: String, details: String },

    /// Underlying engine error.
    #[error("Engine error: {0}")]
    Engine(String),

    /// General I/O failure.
    #[error("I/O error: {0}")]
    Io(String),
}
