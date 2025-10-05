use thiserror::Error;

#[derive(Error, Debug)]
pub enum JailError {
    #[error("Backend error: {0}")]
    Backend(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Jail already exists: {0}")]
    AlreadyExists(String),

    #[error("Command execution failed: {0}")]
    ExecutionFailed(String),
}

pub type Result<T> = std::result::Result<T, JailError>;
