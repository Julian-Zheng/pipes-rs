use thiserror::Error;

#[derive(Error, Debug)]
pub enum ShmError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Shared memory file already exists: {0}")]
    AlreadyExists(String),

    #[error("Shared memory file not found: {0}")]
    NotFound(String),

    #[error("Invalid shared memory layout: {0}")]
    InvalidLayout(String),

    #[error("Buffer overflow: requested {requested} bytes, available {available} bytes")]
    BufferOverflow { requested: usize, available: usize },

    #[error("Invalid descriptor: offset {offset} + len {len} exceeds total size {total_size}")]
    OutOfBounds {
        offset: u64,
        len: u64,
        total_size: u64,
    },

    #[error("Pool exhausted: no free slots available")]
    PoolExhausted,

    #[error("Slot {slot_id} invalid or corrupted")]
    InvalidSlot { slot_id: u32 },
}

pub type Result<T> = std::result::Result<T, ShmError>;
