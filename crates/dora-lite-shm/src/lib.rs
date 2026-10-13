pub mod error;
pub mod pool;
pub mod region;
pub mod ring_buffer;
pub mod transport;

pub use error::{Result, ShmError};
pub use pool::{ShmPoolReader, ShmPoolWriter};
pub use region::{default_shm_dir, resolve_shm_path, MmapMutRegion, MmapViewRegion};
pub use ring_buffer::{ShmRingBufferReader, ShmRingBufferWriter};
pub use transport::{ShmTransportReader, ShmTransportWriter};
