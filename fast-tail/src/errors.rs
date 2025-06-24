use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FastTailError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("File not found: {path}")]
    FileNotFound { path: PathBuf },

    #[error("Permission denied accessing file: {path}")]
    PermissionDenied { path: PathBuf },

    #[error("File watching error: {0}")]
    WatchError(#[from] notify::Error),

    #[error("Pattern compilation error: {pattern} - {source}")]
    PatternCompilation {
        pattern: String,
        #[source]
        source: regex::Error,
    },

    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Task join error: {0}")]
    TaskJoin(#[from] tokio::task::JoinError),

    #[error("File rotation detected but follow-name not enabled: {path}")]
    FileRotationDetected { path: PathBuf },

    #[error("Maximum buffer size exceeded: {current} lines")]
    BufferOverflow { current: usize },

    #[error("Invalid configuration: {message}")]
    InvalidConfig { message: String },

    #[error("Operation cancelled")]
    Cancelled,
}

impl FastTailError {
    pub fn pattern_compilation(pattern: String, source: regex::Error) -> Self {
        Self::PatternCompilation { pattern, source }
    }

    pub fn file_not_found(path: PathBuf) -> Self {
        Self::FileNotFound { path }
    }

    pub fn permission_denied(path: PathBuf) -> Self {
        Self::PermissionDenied { path }
    }

    pub fn file_rotation_detected(path: PathBuf) -> Self {
        Self::FileRotationDetected { path }
    }

    pub fn buffer_overflow(current: usize) -> Self {
        Self::BufferOverflow { current }
    }

    pub fn invalid_config(message: impl Into<String>) -> Self {
        Self::InvalidConfig {
            message: message.into(),
        }
    }
}

pub type Result<T> = std::result::Result<T, FastTailError>;