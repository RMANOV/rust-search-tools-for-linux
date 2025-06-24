use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FastGrepError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Pattern compilation failed: {pattern}")]
    PatternCompilation { 
        pattern: String, 
        #[source] source: regex::Error 
    },
    
    #[error("Invalid argument: {arg} - {reason}")]
    InvalidArgument { 
        arg: String, 
        reason: String 
    },
    
    #[error("File processing error: {path}")]
    FileProcessing { 
        path: PathBuf, 
        #[source] source: Box<dyn std::error::Error + Send + Sync> 
    },
    
    #[error("Binary file skipped: {path}")]
    BinaryFile { 
        path: PathBuf 
    },
    
    #[error("Permission denied: cannot access {path}")]
    PermissionDenied { 
        path: PathBuf 
    },
    
    #[error("Worker pool error: {0}")]
    WorkerPool(#[from] rayon::ThreadPoolBuildError),
    
    #[error("Search interrupted")]
    Interrupted,
    
    #[error("Memory mapping failed for file: {path}")]
    MemoryMapping { 
        path: PathBuf, 
        #[source] source: std::io::Error 
    },
    
    #[error("Content inspection failed for file: {path}")]
    ContentInspection { 
        path: PathBuf, 
        #[source] source: std::io::Error 
    },
}

impl FastGrepError {
    pub fn pattern_compilation(pattern: String, source: regex::Error) -> Self {
        Self::PatternCompilation { pattern, source }
    }
    
    pub fn file_processing<E>(path: PathBuf, source: E) -> Self 
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::FileProcessing { 
            path, 
            source: Box::new(source) 
        }
    }
    
    pub fn memory_mapping(path: PathBuf, source: std::io::Error) -> Self {
        Self::MemoryMapping { path, source }
    }
    
    pub fn content_inspection(path: PathBuf, source: std::io::Error) -> Self {
        Self::ContentInspection { path, source }
    }
}

// Type alias for convenience
pub type Result<T> = std::result::Result<T, FastGrepError>;