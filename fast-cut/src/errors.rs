use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FastCutError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("CSV parsing error: {0}")]
    Csv(#[from] csv::Error),

    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("File not found: {path}")]
    FileNotFound { path: PathBuf },

    #[error("Permission denied accessing file: {path}")]
    PermissionDenied { path: PathBuf },

    #[error("Invalid field selector: {message}")]
    InvalidFieldSelector { message: String },

    #[error("Field not found: {field} (available: {available:?})")]
    FieldNotFound {
        field: String,
        available: Vec<String>,
    },

    #[error("Invalid field index: {index} (line has {field_count} fields)")]
    InvalidFieldIndex { index: usize, field_count: usize },

    #[error("No header found but field names specified")]
    NoHeaderFound,

    #[error("Empty input data")]
    EmptyInput,

    #[error("Invalid configuration: {message}")]
    InvalidConfig { message: String },

    #[error("Buffer overflow: line too long ({length} bytes)")]
    BufferOverflow { length: usize },

    #[error("Encoding error: {message}")]
    EncodingError { message: String },

    #[error("Task join error: {0}")]
    TaskJoin(String),

    #[error("Operation cancelled")]
    Cancelled,
}

impl FastCutError {
    pub fn file_not_found(path: PathBuf) -> Self {
        Self::FileNotFound { path }
    }

    pub fn permission_denied(path: PathBuf) -> Self {
        Self::PermissionDenied { path }
    }

    pub fn invalid_field_selector(message: impl Into<String>) -> Self {
        Self::InvalidFieldSelector {
            message: message.into(),
        }
    }

    pub fn field_not_found(field: impl Into<String>, available: Vec<String>) -> Self {
        Self::FieldNotFound {
            field: field.into(),
            available,
        }
    }

    pub fn invalid_field_index(index: usize, field_count: usize) -> Self {
        Self::InvalidFieldIndex { index, field_count }
    }

    pub fn invalid_config(message: impl Into<String>) -> Self {
        Self::InvalidConfig {
            message: message.into(),
        }
    }

    pub fn buffer_overflow(length: usize) -> Self {
        Self::BufferOverflow { length }
    }

    pub fn encoding_error(message: impl Into<String>) -> Self {
        Self::EncodingError {
            message: message.into(),
        }
    }
}

pub type Result<T> = std::result::Result<T, FastCutError>;