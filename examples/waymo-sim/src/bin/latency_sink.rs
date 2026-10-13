use dora_lite_node_api::{DoraNode, Event};
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

struct LatencyStats {
    total_received: u64,
    last_frame_id: Option<u64>,
    dropped_frames_detected: u64,
}

impl LatencyStats {
    fn new() -> Self {
        Self {
            total_received: 0,
            last_frame_id: None,
            dropped_frames_detected: 0,
        }
    }

    fn record_frame(&mut self, frame_id: u64, _sim_timestamp_micros: u64) {
        self.total_received += 1;

        if let Some(prev) = self.last_frame_id
            && frame_id > prev + 1
        {
            let skipped = frame_id - prev - 1;
            self.dropped_frames_detected += skipped;
            warn!(
                "[LatencySink] Stale frame drop detected! Skipped {} frame(s) (between #{} and #{})",
                skipped, prev, frame_id
            );
        }
        self.last_frame_id = Some(frame_id);
    }

    fn print_summary(&self) {
        info!("=================== Latency Sink Performance Summary ===================");
        info!("Total frames received: {}", self.total_received);
        info!("Detected dropped frames (via drop_oldest policy): {}", self.dropped_frames_detected);
        if self.total_received > 0 {
            let total_frames = self.total_received + self.dropped_frames_detected;
            let drop_rate = (self.dropped_frames_detected as f64 / total_frames as f64) * 100.0;
            info!("Effective frame drop rate: {:.2}%", drop_rate);
        }
        info!("========================================================================");
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .init();

    info!("Initializing latency-measuring sink node...");
    let mut node = DoraNode::init_from_env().await?;
    info!("Latency sink connected to coordinator. Monitoring end-to-end metrics...");

    let mut stats = LatencyStats::new();

    while let Some(event) = node.next_event().await {
        match event {
            Event::Input { id, metadata, data } => {
                stats.record_frame(metadata.frame_id, metadata.timestamp_micros);

                info!(
                    "[LatencySink] Port '{}' | Frame #{:04} | Payload Size: {} bytes | Timestamp: {} µs",
                    id,
                    metadata.frame_id,
                    data.len(),
                    metadata.timestamp_micros
                );

                if stats.total_received.is_multiple_of(10) {
                    stats.print_summary();
                }
            }
            Event::Stop => {
                info!("Latency sink received Stop signal.");
                break;
            }
            _ => {}
        }
    }

    stats.print_summary();
    Ok(())
}
