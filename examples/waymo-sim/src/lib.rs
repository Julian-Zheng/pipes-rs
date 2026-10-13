use serde::{Deserialize, Serialize};

/// Lightweight 3D bounding box prediction produced by perception nodes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BoundingBox3D {
    pub center_x: f32,
    pub center_y: f32,
    pub center_z: f32,
    pub size_x: f32,
    pub size_y: f32,
    pub size_z: f32,
    pub heading: f32,
    pub confidence: f32,
    pub class_name: String,
}

/// Autonomous vehicle ego pose state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EgoPose {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub qw: f64,
    pub qx: f64,
    pub qy: f64,
    pub qz: f64,
    pub velocity_mps: f32,
}

/// Helper to serialize / deserialize raw LiDAR point cloud frames in memory.
pub struct PointCloudFormat;

impl PointCloudFormat {
    /// Generates synthetic Waymo LiDAR point cloud (4 floats per point: x, y, z, intensity = 16 bytes).
    pub fn generate_synthetic_points(num_points: usize, frame_id: u64) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(num_points * 16);
        let base_angle = (frame_id as f32) * 0.05;

        for i in 0..num_points {
            let angle = base_angle + (i as f32) * 0.001;
            let distance = 5.0 + ((i % 100) as f32) * 0.5;
            let x = distance * angle.cos();
            let y = distance * angle.sin();
            let z = -1.5 + ((i % 20) as f32) * 0.1;
            let intensity = ((i % 256) as f32) / 255.0;

            buffer.extend_from_slice(&x.to_le_bytes());
            buffer.extend_from_slice(&y.to_le_bytes());
            buffer.extend_from_slice(&z.to_le_bytes());
            buffer.extend_from_slice(&intensity.to_le_bytes());
        }

        buffer
    }
}
