use thiserror::Error;

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Core error: {0}")]
    Core(#[from] dora_lite_core::error::DoraError),

    #[error("Shared memory error: {0}")]
    Shm(#[from] dora_lite_shm::error::ShmError),

    #[error("Process spawning failed for node '{node_id}': {message}")]
    ProcessSpawn { node_id: String, message: String },

    #[error("Node '{0}' not registered within timeout")]
    NodeTimeout(String),

    #[error("IPC communication error: {0}")]
    Ipc(String),

    #[error("Simulation error: {0}")]
    Simulation(String),
}

pub type Result<T> = std::result::Result<T, RuntimeError>;
