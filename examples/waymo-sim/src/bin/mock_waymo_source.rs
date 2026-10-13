use std::time::Duration;
use dora_lite_core::clock::SimulationClock;
use dora_lite_core::message::{FrameMetadata, SensorType};
use dora_lite_node_api::DoraNode;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use waymo_sim::{EgoPose, PointCloudFormat};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .init();

    info!("Initializing mock Waymo Open Dataset source node...");
    let mut node = DoraNode::init_from_env().await?;
    info!("Waymo source node successfully connected to coordinator");

    // 10 Hz simulation clock matching Waymo dataset (100ms per frame)
    let clock = SimulationClock::new(10.0, 1_700_000_000_000_000);
    let mut ticker = tokio::time::interval(Duration::from_millis(100));

    let num_points = 50_000; // 50,000 points * 16 bytes = 800 KB per LiDAR frame
    let camera_bytes_len = 512 * 1024; // 512 KB per camera frame

    loop {
        ticker.tick().await;

        let (frame_id, timestamp_micros) = clock.tick();

        // 1. Generate & publish Top LiDAR frame (800KB)
        let lidar_data = PointCloudFormat::generate_synthetic_points(num_points, frame_id);
        let lidar_meta = FrameMetadata::new(frame_id, timestamp_micros, SensorType::Lidar)
            .with_dimensions(num_points as u32, 4, 1)
            .with_attr("beam_count", "64");

        node.send_output("lidar_top", &lidar_data, &lidar_meta).await?;

        // --- ADDED: 1.5. Generate & publish Rear/Second LiDAR frame ---
        let lidar_rear_data = PointCloudFormat::generate_synthetic_points(num_points, frame_id);
        let lidar_rear_meta = FrameMetadata::new(frame_id, timestamp_micros, SensorType::Lidar)
            .with_dimensions(num_points as u32, 4, 1)
            .with_attr("beam_count", "32"); // e.g., lower resolution rear lidar

        node.send_output("lidar_rear", &lidar_rear_data, &lidar_rear_meta).await?;

        // 2. Generate & publish Front Camera frame (512KB)
        let mut camera_data = vec![(frame_id % 256) as u8; camera_bytes_len];
        // Embed frame timestamp into first 8 bytes for end-to-end verification
        camera_data[0..8].copy_from_slice(&timestamp_micros.to_le_bytes());
        let camera_meta = FrameMetadata::new(frame_id, timestamp_micros, SensorType::Camera)
            .with_dimensions(1920, 1080, 3)
            .with_attr("camera_id", "FRONT");

        node.send_output("camera_front", &camera_data, &camera_meta).await?;

        // 3. Generate & publish Ego Pose (<1KB)
        let ego_pose = EgoPose {
            x: 100.0 + (frame_id as f64) * 1.5,
            y: 200.0 + (frame_id as f64) * 0.2,
            z: 10.0,
            qw: 1.0,
            qx: 0.0,
            qy: 0.0,
            qz: 0.0,
            velocity_mps: 15.0,
        };
        let pose_bytes = serde_json::to_vec(&ego_pose)?;
        let pose_meta = FrameMetadata::new(frame_id, timestamp_micros, SensorType::Pose);

        node.send_output("ego_pose", &pose_bytes, &pose_meta).await?;

        info!(
            "Published Waymo frame #{:04} | timestamp: {} µs | LiDAR: {:.1} KB | Camera: {:.1} KB",
            frame_id,
            timestamp_micros,
            lidar_data.len() as f64 / 1024.0,
            camera_data.len() as f64 / 1024.0
        );

        if frame_id >= 200 {
            info!("Reached simulation limit (200 frames). Stopping mock source.");
            break;
        }
    }

    Ok(())
}
