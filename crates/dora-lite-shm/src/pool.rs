use dora_lite_core::message::ShmDescriptor;

use crate::error::{Result, ShmError};
use crate::region::{MmapMutRegion, MmapViewRegion};

const HEADER_SIZE: usize = 64;
const POOL_MAGIC: &[u8; 4] = b"DPOL";
const CURRENT_VERSION: u32 = 1;

/// Fixed-slot shared memory pool writer for deterministic frame allocation.
pub struct ShmPoolWriter {
    shm_id: String,
    region: MmapMutRegion,
    slot_size: usize,
    num_slots: usize,
    seq: u64,
}

impl ShmPoolWriter {
    pub fn new(shm_id: impl Into<String>, num_slots: usize, slot_size: usize) -> Result<Self> {
        let shm_id = shm_id.into();
        let total_size = HEADER_SIZE + (num_slots * slot_size);
        let mut region = MmapMutRegion::create(&shm_id, total_size, true)?;

        let slice = region.as_mut_slice();
        slice[0..4].copy_from_slice(POOL_MAGIC);
        slice[4..8].copy_from_slice(&CURRENT_VERSION.to_le_bytes());
        slice[8..16].copy_from_slice(&(total_size as u64).to_le_bytes());
        slice[16..20].copy_from_slice(&(slot_size as u32).to_le_bytes());
        slice[20..24].copy_from_slice(&(num_slots as u32).to_le_bytes());

        Ok(Self {
            shm_id,
            region,
            slot_size,
            num_slots,
            seq: 0,
        })
    }

    /// Allocates the next sequential slot and writes the payload zero-copy.
    pub fn write_frame(&mut self, data: &[u8], timestamp_micros: u64) -> Result<ShmDescriptor> {
        let len = data.len();
        if len > self.slot_size {
            return Err(ShmError::BufferOverflow {
                requested: len,
                available: self.slot_size,
            });
        }

        let slot_id = (self.seq % self.num_slots as u64) as u32;
        let offset = HEADER_SIZE + (slot_id as usize * self.slot_size);

        let slice = self.region.as_mut_slice();
        slice[offset..offset + len].copy_from_slice(data);

        let seq = self.seq;
        self.seq += 1;

        Ok(ShmDescriptor::new(
            &self.shm_id,
            offset as u64,
            len as u64,
            slot_id,
            seq,
            timestamp_micros,
        ))
    }

    pub fn slot_size(&self) -> usize {
        self.slot_size
    }

    pub fn num_slots(&self) -> usize {
        self.num_slots
    }

    pub fn shm_id(&self) -> &str {
        &self.shm_id
    }
}

/// Fixed-slot shared memory pool reader providing zero-copy view of slot payloads.
pub struct ShmPoolReader {
    shm_id: String,
    region: MmapViewRegion,
    slot_size: usize,
    num_slots: usize,
}

impl ShmPoolReader {
    pub fn open(shm_id: impl Into<String>) -> Result<Self> {
        let shm_id = shm_id.into();
        let region = MmapViewRegion::open(&shm_id)?;

        let slice = region.as_slice();
        if slice.len() < HEADER_SIZE || &slice[0..4] != POOL_MAGIC {
            return Err(ShmError::InvalidLayout(
                "Invalid or missing DPOL magic header".to_string(),
            ));
        }

        let slot_size = u32::from_le_bytes(slice[16..20].try_into().unwrap()) as usize;
        let num_slots = u32::from_le_bytes(slice[20..24].try_into().unwrap()) as usize;

        Ok(Self {
            shm_id,
            region,
            slot_size,
            num_slots,
        })
    }

    /// Reads payload slice zero-copy from shared memory descriptor.
    pub fn read<'a>(&'a self, descriptor: &ShmDescriptor) -> Result<&'a [u8]> {
        self.region.slice(descriptor.offset, descriptor.len)
    }

    pub fn slot_size(&self) -> usize {
        self.slot_size
    }

    pub fn num_slots(&self) -> usize {
        self.num_slots
    }

    pub fn shm_id(&self) -> &str {
        &self.shm_id
    }
}
