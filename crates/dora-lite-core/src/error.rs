use thiserror::Error;

#[derive(Error, Debug)]
pub enum DoraError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization/YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Configuration validation error: {0}")]
    Config(String),

    #[error("Shared memory error: {0}")]
    Shm(String),

    #[error("Node error [{node_id}]: {message}")]
    Node { node_id: String, message: String },

    #[error("Channel error: {0}")]
    Channel(String),

    #[error("Queue full error for port {0}")]
    QueueFull(String),

    #[error("Unknown error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, DoraError>;
