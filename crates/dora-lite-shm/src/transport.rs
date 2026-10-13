use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use dora_lite_core::message::ShmDescriptor;

use crate::error::Result;
use crate::region::MmapViewRegion;
use crate::ring_buffer::ShmRingBufferWriter;

/// Producer-side shared memory transport manager.
pub struct ShmTransportWriter {
    ring_writers: HashMap<String, ShmRingBufferWriter>,
    default_capacity: usize,
}

impl ShmTransportWriter {
    pub fn new(default_capacity: usize) -> Self {
        Self {
            ring_writers: HashMap::new(),
            default_capacity,
        }
    }
}

impl Default for ShmTransportWriter {
    fn default() -> Self {
        Self::new(64 * 1024 * 1024) // 64 MB default capacity
    }
}

impl ShmTransportWriter {

    /// Writes data into the specified shared memory buffer (creating it if needed)
    /// and returns the lightweight descriptor.
    pub fn write(
        &mut self,
        shm_id: &str,
        data: &[u8],
        timestamp_micros: u64,
    ) -> Result<ShmDescriptor> {
        let capacity = self.default_capacity.max(data.len() * 4);
        let writer = self
            .ring_writers
            .entry(shm_id.to_string())
            .or_insert_with(|| ShmRingBufferWriter::new(shm_id, capacity).unwrap());

        writer.write_payload(data, timestamp_micros)
    }
}

/// Consumer-side shared memory transport manager with lazy reader caching.
#[derive(Default)]
pub struct ShmTransportReader {
    readers: Mutex<HashMap<String, Arc<MmapViewRegion>>>,
}

impl ShmTransportReader {
    pub fn new() -> Self {
        Self {
            readers: Mutex::new(HashMap::new()),
        }
    }

    /// Reads payload zero-copy from the region identified in descriptor.
    pub fn read_bytes(&self, descriptor: &ShmDescriptor) -> Result<Vec<u8>> {
        let mut readers = self.readers.lock().unwrap();
        let region = match readers.get(&descriptor.shm_id) {
            Some(r) => r.clone(),
            None => {
                let r = Arc::new(MmapViewRegion::open(&descriptor.shm_id)?);
                readers.insert(descriptor.shm_id.clone(), r.clone());
                r
            }
        };

        let slice = region.slice(descriptor.offset, descriptor.len)?;
        Ok(slice.to_vec())
    }

    /// Directly borrows payload slice from the mapped region.
    pub fn with_slice<F, R>(&self, descriptor: &ShmDescriptor, f: F) -> Result<R>
    where
        F: FnOnce(&[u8]) -> R,
    {
        let mut readers = self.readers.lock().unwrap();
        let region = match readers.get(&descriptor.shm_id) {
            Some(r) => r.clone(),
            None => {
                let r = Arc::new(MmapViewRegion::open(&descriptor.shm_id)?);
                readers.insert(descriptor.shm_id.clone(), r.clone());
                r
            }
        };

        let slice = region.slice(descriptor.offset, descriptor.len)?;
        Ok(f(slice))
    }
}
