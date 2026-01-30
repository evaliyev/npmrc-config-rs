//! Error types for npmrc-config-rs.

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur when working with npmrc configuration.
#[derive(Error, Debug)]
pub enum Error {
    /// Config file not found.
    #[error("config file not found: {0}")]
    FileNotFound(PathBuf),

    /// Failed to read a config file.
    #[error("failed to read config file {path}: {source}")]
    ReadFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to parse INI content.
    #[error("failed to parse INI content from {path}: {message}")]
    ParseIni { path: PathBuf, message: String },

    /// Invalid URL in configuration.
    #[error("invalid URL '{url}': {message}")]
    InvalidUrl { url: String, message: String },

    /// Invalid base64 encoding in password field.
    #[error("invalid base64 encoding in _password field")]
    InvalidBase64(#[from] base64::DecodeError),

    /// UTF-8 decoding error.
    #[error("invalid UTF-8 in decoded password")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),
}

/// Result type alias for npmrc-config-rs operations.
pub type Result<T> = std::result::Result<T, Error>;
