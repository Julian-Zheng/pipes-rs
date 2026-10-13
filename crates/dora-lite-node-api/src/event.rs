use std::ops::Deref;
use dora_lite_core::message::FrameMetadata;

/// A zero-copy buffer view wrapping either inline bytes or shared memory slices.
#[derive(Debug, Clone)]
pub enum ZeroCopyBuffer {
    Inline(Vec<u8>),
    SharedMemory {
        data: Vec<u8>,
        shm_id: String,
        offset: u64,
        len: u64,
    },
}

impl ZeroCopyBuffer {
    pub fn from_inline(data: Vec<u8>) -> Self {
        Self::Inline(data)
    }

    pub fn from_shm(data: Vec<u8>, shm_id: String, offset: u64, len: u64) -> Self {
        Self::SharedMemory {
            data,
            shm_id,
            offset,
            len,
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        match self {
            Self::Inline(b) => b.as_slice(),
            Self::SharedMemory { data, .. } => data.as_slice(),
        }
    }
}

impl Deref for ZeroCopyBuffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl AsRef<[u8]> for ZeroCopyBuffer {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

/// Events dispatched to node worker loops.
#[derive(Debug, Clone)]
pub enum Event {
    /// Sensor frame or intermediate perception message received on an input port.
    Input {
        id: String,
        metadata: FrameMetadata,
        data: ZeroCopyBuffer,
    },
    /// Simulation clock tick event for time-driven source nodes.
    Tick {
        frame_id: u64,
        timestamp_micros: u64,
    },
    /// Pipeline termination signal.
    Stop,
}
