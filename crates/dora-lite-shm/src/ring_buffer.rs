use dora_lite_core::message::ShmDescriptor;

use crate::error::{Result, ShmError};
use crate::region::{MmapMutRegion, MmapViewRegion};

const HEADER_SIZE: usize = 64;
const MAGIC: &[u8; 4] = b"DSHM";
const CURRENT_VERSION: u32 = 1;

/// Circular shared memory ring buffer writer.
pub struct ShmRingBufferWriter {
    shm_id: String,
    region: MmapMutRegion,
    data_start: usize,
    capacity: usize,
    write_offset: usize,
    seq: u64,
}

impl ShmRingBufferWriter {
    pub fn new(shm_id: impl Into<String>, capacity: usize) -> Result<Self> {
        let shm_id = shm_id.into();
        let total_size = HEADER_SIZE + capacity;
        let mut region = MmapMutRegion::create(&shm_id, total_size, true)?;

        // Initialize header
        let slice = region.as_mut_slice();
        slice[0..4].copy_from_slice(MAGIC);
        slice[4..8].copy_from_slice(&CURRENT_VERSION.to_le_bytes());
        slice[8..16].copy_from_slice(&(total_size as u64).to_le_bytes());
        slice[16..24].copy_from_slice(&(HEADER_SIZE as u64).to_le_bytes());

        Ok(Self {
            shm_id,
            region,
            data_start: HEADER_SIZE,
            capacity,
            write_offset: HEADER_SIZE,
            seq: 0,
        })
    }

    /// Writes raw payload into shared memory and generates a lightweight ShmDescriptor.
    pub fn write_payload(&mut self, data: &[u8], timestamp_micros: u64) -> Result<ShmDescriptor> {
        let len = data.len();
        if len > self.capacity {
            return Err(ShmError::BufferOverflow {
                requested: len,
                available: self.capacity,
            });
        }

        let total_size = self.region.size();

        // Check if data fits between current write_offset and the end of the buffer
        let target_offset = if self.write_offset + len <= total_size {
            self.write_offset
        } else {
            // Wrap around to the start of the data ring
            self.data_start
        };

        // Copy directly into memory mapped file
        let slice = self.region.as_mut_slice();
        slice[target_offset..target_offset + len].copy_from_slice(data);

        let seq = self.seq;
        self.seq += 1;
        self.write_offset = target_offset + len;

        // Align next write offset to 64-byte boundary for SIMD / cacheline efficiency
        let rem = self.write_offset % 64;
        if rem != 0 {
            self.write_offset += 64 - rem;
        }

        Ok(ShmDescriptor::new(
            &self.shm_id,
            target_offset as u64,
            len as u64,
            0,
            seq,
            timestamp_micros,
        ))
    }

    pub fn shm_id(&self) -> &str {
        &self.shm_id
    }
}

/// Circular shared memory ring buffer reader providing zero-copy view of payloads.
pub struct ShmRingBufferReader {
    shm_id: String,
    region: MmapViewRegion,
}

impl ShmRingBufferReader {
    pub fn open(shm_id: impl Into<String>) -> Result<Self> {
        let shm_id = shm_id.into();
        let region = MmapViewRegion::open(&shm_id)?;

        // Validate magic header
        let slice = region.as_slice();
        if slice.len() < HEADER_SIZE || &slice[0..4] != MAGIC {
            return Err(ShmError::InvalidLayout(
                "Invalid or missing DSHM magic header".to_string(),
            ));
        }

        Ok(Self { shm_id, region })
    }

    /// Reads payload slice zero-copy from shared memory descriptor.
    pub fn read<'a>(&'a self, descriptor: &ShmDescriptor) -> Result<&'a [u8]> {
        self.region.slice(descriptor.offset, descriptor.len)
    }

    pub fn shm_id(&self) -> &str {
        &self.shm_id
    }
}
