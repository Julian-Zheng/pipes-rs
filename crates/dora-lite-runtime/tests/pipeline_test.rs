use std::time::{Duration, Instant};
use dora_lite_core::config::JitterConfig;
use dora_lite_core::message::{DataflowMessage, FrameMetadata, SensorType};
use dora_lite_core::queue::QueueConfig;
use dora_lite_runtime::queue::PortQueue;
use dora_lite_runtime::simulation_hooks::JitterInjector;

#[tokio::test]
async fn test_drop_oldest_queue_policy() {
    let capacity = 3;
    let queue = PortQueue::new("lidar_in", QueueConfig::drop_oldest(capacity));

    // Push 5 frames into capacity-3 queue
    for i in 1..=5 {
        let meta = FrameMetadata::new(i, 100_000 * i, SensorType::Lidar);
        let msg = DataflowMessage::new_inline("source", "lidar", meta, vec![i as u8; 32]);
        queue.push(msg).await;
    }

    // Since capacity is 3 and policy is drop_oldest, frames 1 and 2 should have been dropped!
    let stats = queue.stats();
    assert_eq!(stats.dropped.load(std::sync::atomic::Ordering::Relaxed), 2);
    assert_eq!(stats.pushed.load(std::sync::atomic::Ordering::Relaxed), 5);

    // Remaining items should be frames 3, 4, 5
    let msg1 = queue.pop().await;
    assert_eq!(msg1.metadata.frame_id, 3);

    let msg2 = queue.pop().await;
    assert_eq!(msg2.metadata.frame_id, 4);

    let msg3 = queue.pop().await;
    assert_eq!(msg3.metadata.frame_id, 5);
}

#[tokio::test]
async fn test_backpressure_queue_policy() {
    let capacity = 2;
    let queue = std::sync::Arc::new(PortQueue::new(
        "camera_in",
        QueueConfig::backpressure(capacity),
    ));

    // Fill queue to capacity
    for i in 1..=2 {
        let meta = FrameMetadata::new(i, 100_000 * i, SensorType::Camera);
        let msg = DataflowMessage::new_inline("source", "cam", meta, vec![i as u8; 64]);
        queue.push(msg).await;
    }

    let q_clone = queue.clone();

    // Spawn task that tries to push a 3rd item (should block until popped)
    let push_task = tokio::spawn(async move {
        let meta = FrameMetadata::new(3, 300_000, SensorType::Camera);
        let msg = DataflowMessage::new_inline("source", "cam", meta, vec![3u8; 64]);
        q_clone.push(msg).await;
    });

    // Wait a brief moment to ensure push_task is blocked
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(!push_task.is_finished());

    // Pop an item to free space
    let popped = queue.pop().await;
    assert_eq!(popped.metadata.frame_id, 1);

    // Now push_task should complete
    push_task.await.unwrap();

    let stats = queue.stats();
    assert_eq!(stats.dropped.load(std::sync::atomic::Ordering::Relaxed), 0);
    assert_eq!(stats.pushed.load(std::sync::atomic::Ordering::Relaxed), 3);
}

#[tokio::test]
async fn test_simulation_jitter_injection() {
    let config = JitterConfig::fixed(40);
    let injector = JitterInjector::new(&config);

    let start = Instant::now();
    injector.inject().await;
    let elapsed = start.elapsed();

    assert!(elapsed >= Duration::from_millis(38));
}
