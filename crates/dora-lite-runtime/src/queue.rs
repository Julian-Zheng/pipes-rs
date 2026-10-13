use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use dora_lite_core::message::DataflowMessage;
use dora_lite_core::queue::{QueueConfig, QueuePolicyKind};
use tokio::sync::{Mutex, Notify};
use tracing::warn;

/// Statistics for a single input port queue.
#[derive(Debug, Default)]
pub struct QueueStats {
    pub pushed: AtomicU64,
    pub popped: AtomicU64,
    pub dropped: AtomicU64,
}

/// An asynchronous queue enforcing `DropOldest` or `Backpressure` semantics.
pub struct PortQueue {
    port_name: String,
    config: QueueConfig,
    buffer: Arc<Mutex<VecDeque<DataflowMessage>>>,
    not_empty: Arc<Notify>,
    not_full: Arc<Notify>,
    stats: Arc<QueueStats>,
}

impl PortQueue {
    pub fn new(port_name: impl Into<String>, config: QueueConfig) -> Self {
        Self {
            port_name: port_name.into(),
            config,
            buffer: Arc::new(Mutex::new(VecDeque::new())),
            not_empty: Arc::new(Notify::new()),
            not_full: Arc::new(Notify::new()),
            stats: Arc::new(QueueStats::default()),
        }
    }

    /// Pushes a message into the queue respecting the configured queue policy.
    pub async fn push(&self, msg: DataflowMessage) {
        match self.config.policy {
            QueuePolicyKind::DropOldest => {
                let mut queue = self.buffer.lock().await;
                if queue.len() >= self.config.capacity
                    && let Some(dropped) = queue.pop_front()
                {
                    self.stats.dropped.fetch_add(1, Ordering::Relaxed);
                    warn!(
                        "Port '{}' [drop_oldest]: dropped stale frame seq {} (capacity {})",
                        self.port_name,
                        dropped.metadata.frame_id,
                        self.config.capacity
                    );
                }
                queue.push_back(msg);
                self.stats.pushed.fetch_add(1, Ordering::Relaxed);
                self.not_empty.notify_one();
            }
            QueuePolicyKind::Backpressure => {
                loop {
                    let mut queue = self.buffer.lock().await;
                    if queue.len() < self.config.capacity {
                        queue.push_back(msg);
                        self.stats.pushed.fetch_add(1, Ordering::Relaxed);
                        self.not_empty.notify_one();
                        break;
                    }
                    drop(queue);
                    // Wait until consumer pops an item
                    self.not_full.notified().await;
                }
            }
        }
    }

    /// Pops the next message from the queue, blocking until one is available.
    pub async fn pop(&self) -> DataflowMessage {
        loop {
            let mut queue = self.buffer.lock().await;
            if let Some(msg) = queue.pop_front() {
                self.stats.popped.fetch_add(1, Ordering::Relaxed);
                self.not_full.notify_one();
                return msg;
            }
            drop(queue);
            self.not_empty.notified().await;
        }
    }

    /// Attempts to pop immediately without waiting.
    pub async fn try_pop(&self) -> Option<DataflowMessage> {
        let mut queue = self.buffer.lock().await;
        let msg = queue.pop_front();
        if msg.is_some() {
            self.stats.popped.fetch_add(1, Ordering::Relaxed);
            self.not_full.notify_one();
        }
        msg
    }

    pub fn stats(&self) -> Arc<QueueStats> {
        self.stats.clone()
    }

    pub fn config(&self) -> &QueueConfig {
        &self.config
    }
}
