#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid: {0}")]
    Invalid(String),
    #[error("the game is running")]
    GameRunning,
}

pub type Result<T> = std::result::Result<T, EngineError>;
