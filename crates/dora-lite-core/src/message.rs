use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Sensor type categorization for Waymo Open Dataset simulation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SensorType {
    Camera,
    Lidar,
    Pose,
    BoundingBoxes3D,
    #[default]
    Generic,
}

/// Metadata describing an autonomous vehicle sensor frame.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FrameMetadata {
    /// Synchronized simulation frame index.
    pub frame_id: u64,
    /// Deterministic Waymo simulation timestamp in microseconds.
    pub timestamp_micros: u64,
    /// The sensor classification.
    #[serde(default)]
    pub sensor_type: SensorType,
    /// Optional dimensional metadata (e.g., width, height, channels for camera, or point count for LiDAR).
    pub dimensions: Option<(u32, u32, u32)>,
    /// Additional custom key-value attributes (e.g., camera intrinsic matrix, frame rate).
    #[serde(default)]
    pub custom_attrs: HashMap<String, String>,
}

impl FrameMetadata {
    pub fn new(frame_id: u64, timestamp_micros: u64, sensor_type: SensorType) -> Self {
        Self {
            frame_id,
            timestamp_micros,
            sensor_type,
            dimensions: None,
            custom_attrs: HashMap::new(),
        }
    }

    pub fn with_dimensions(mut self, width: u32, height: u32, channels: u32) -> Self {
        self.dimensions = Some((width, height, channels));
        self
    }

    pub fn with_attr(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom_attrs.insert(key.into(), value.into());
        self
    }
}

/// Descriptor pointing to zero-copy data stored in shared memory.
/// Passed over Unix domain sockets / node queues instead of the large payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShmDescriptor {
    /// Identifier / filename of the shared memory pool/file.
    pub shm_id: String,
    /// Byte offset within the shared memory region.
    pub offset: u64,
    /// Byte length of the valid payload slice.
    pub len: u64,
    /// Slot index (if using slot-based ring pools).
    pub slot_id: u32,
    /// Monotonically increasing sequence number.
    pub seq: u64,
    /// Waymo simulation timestamp in microseconds.
    pub timestamp_micros: u64,
}

impl ShmDescriptor {
    pub fn new(
        shm_id: impl Into<String>,
        offset: u64,
        len: u64,
        slot_id: u32,
        seq: u64,
        timestamp_micros: u64,
    ) -> Self {
        Self {
            shm_id: shm_id.into(),
            offset,
            len,
            slot_id,
            seq,
            timestamp_micros,
        }
    }
}

/// Payload format across dataflow channels:
/// - Payloads <= 4KB can be transmitted inline.
/// - Payloads > 4KB are passed via `ShmDescriptor` for zero-copy IPC.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Payload {
    Inline(Vec<u8>),
    SharedMemory(ShmDescriptor),
}

impl Payload {
    pub fn len(&self) -> usize {
        match self {
            Payload::Inline(bytes) => bytes.len(),
            Payload::SharedMemory(desc) => desc.len as usize,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A complete message routed through the dataflow runtime.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataflowMessage {
    pub source_node: String,
    pub output_port: String,
    pub metadata: FrameMetadata,
    pub payload: Payload,
}

impl DataflowMessage {
    pub fn new_shm(
        source_node: impl Into<String>,
        output_port: impl Into<String>,
        metadata: FrameMetadata,
        descriptor: ShmDescriptor,
    ) -> Self {
        Self {
            source_node: source_node.into(),
            output_port: output_port.into(),
            metadata,
            payload: Payload::SharedMemory(descriptor),
        }
    }

    pub fn new_inline(
        source_node: impl Into<String>,
        output_port: impl Into<String>,
        metadata: FrameMetadata,
        data: Vec<u8>,
    ) -> Self {
        Self {
            source_node: source_node.into(),
            output_port: output_port.into(),
            metadata,
            payload: Payload::Inline(data),
        }
    }
}
