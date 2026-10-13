use dora_lite_core::message::{FrameMetadata, SensorType};
use dora_lite_node_api::{DoraNode, Event};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use waymo_sim::BoundingBox3D;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .init();

    info!("Initializing perception filter node...");
    let mut node = DoraNode::init_from_env().await?;
    info!("Perception filter connected to coordinator. Processing sensor streams...");

    let mut processed_frames = 0;

    while let Some(event) = node.next_event().await {
        match event {
            Event::Input { id, metadata, data } => {
                info!(
                    "Perception filter received input on port '{}' for frame #{} (size: {} bytes)",
                    id,
                    metadata.frame_id,
                    data.len()
                );

                // Simulate perception compute workload (e.g. PointPillars / CenterPoint 3D detection)
                let compute_ms = 40 + (metadata.frame_id % 30);
                tokio::time::sleep(tokio::time::Duration::from_millis(compute_ms)).await;

                // Produce mock 3D bounding box predictions
                let detections = vec![
                    BoundingBox3D {
                        center_x: 12.5,
                        center_y: 3.2,
                        center_z: -0.8,
                        size_x: 4.8,
                        size_y: 2.0,
                        size_z: 1.6,
                        heading: 0.12,
                        confidence: 0.94,
                        class_name: "VEHICLE".to_string(),
                    },
                    BoundingBox3D {
                        center_x: 8.2,
                        center_y: -4.5,
                        center_z: -0.7,
                        size_x: 0.8,
                        size_y: 0.8,
                        size_z: 1.7,
                        heading: -0.5,
                        confidence: 0.88,
                        class_name: "PEDESTRIAN".to_string(),
                    },
                ];

                let output_bytes = serde_json::to_vec(&detections)?;
                let out_meta = FrameMetadata::new(
                    metadata.frame_id,
                    metadata.timestamp_micros,
                    SensorType::BoundingBoxes3D,
                )
                .with_attr("compute_time_ms", compute_ms.to_string());

                node.send_output("clusters_3d", &output_bytes, &out_meta).await?;
                processed_frames += 1;
            }
            Event::Stop => {
                info!("Perception filter received Stop event. Total frames processed: {}", processed_frames);
                break;
            }
            _ => {}
        }
    }

    Ok(())
}
