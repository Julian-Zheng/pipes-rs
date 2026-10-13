use std::sync::Arc;
use dora_lite_core::error::{DoraError, Result};
use dora_lite_core::message::{DataflowMessage, FrameMetadata, Payload};
use dora_lite_shm::{ShmTransportReader, ShmTransportWriter};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::mpsc;
use tracing::error;

use crate::event::{Event, ZeroCopyBuffer};

const SHM_THRESHOLD: usize = 4096; // 4KB threshold as per specification

/// Client handle for a node executing in the dora-lite dataflow runtime.
pub struct DoraNode {
    node_id: String,
    event_rx: mpsc::Receiver<Event>,
    out_tx: mpsc::Sender<DataflowMessage>,
    shm_writer: ShmTransportWriter,
}

impl DoraNode {
    /// Initializes a node by inspecting environment variables set by the coordinator.
    pub async fn init_from_env() -> Result<Self> {
        let node_id = std::env::var("DORA_NODE_ID").map_err(|_| {
            DoraError::Node {
                node_id: "unknown".into(),
                message: "Missing DORA_NODE_ID environment variable".into(),
            }
        })?;

        let sock_path = std::env::var("DORA_CONTROL_SOCK").map_err(|_| {
            DoraError::Node {
                node_id: node_id.clone(),
                message: "Missing DORA_CONTROL_SOCK environment variable".into(),
            }
        })?;

        Self::connect(&node_id, &sock_path).await
    }

    /// Connects to the runtime coordinator via Unix Domain Socket.
    pub async fn connect(node_id: &str, socket_path: &str) -> Result<Self> {
        let stream = UnixStream::connect(socket_path).await?;
        let (reader_half, mut writer_half) = stream.into_split();

        // Send registration message
        let reg = serde_json::json!({
            "action": "register",
            "node_id": node_id
        });
        let mut reg_str = serde_json::to_string(&reg)?;
        reg_str.push('\n');
        writer_half.write_all(reg_str.as_bytes()).await?;
        writer_half.flush().await?;

        let (event_tx, event_rx) = mpsc::channel(128);
        let (out_tx, mut out_rx) = mpsc::channel::<DataflowMessage>(128);

        let shm_reader = Arc::new(ShmTransportReader::new());

        // Spawn incoming reader task
        let reader_shm = shm_reader.clone();
        let ev_tx = event_tx.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(reader_half).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }

                if let Ok(msg) = serde_json::from_str::<DataflowMessage>(&line) {
                    let zero_copy_data = match msg.payload {
                        Payload::Inline(bytes) => ZeroCopyBuffer::from_inline(bytes),
                        Payload::SharedMemory(desc) => {
                            match reader_shm.read_bytes(&desc) {
                                Ok(bytes) => ZeroCopyBuffer::from_shm(
                                    bytes,
                                    desc.shm_id,
                                    desc.offset,
                                    desc.len,
                                ),
                                Err(err) => {
                                    error!("Failed to read shared memory slice: {:?}", err);
                                    continue;
                                }
                            }
                        }
                    };

                    let event = Event::Input {
                        id: msg.output_port,
                        metadata: msg.metadata,
                        data: zero_copy_data,
                    };

                    if ev_tx.send(event).await.is_err() {
                        break;
                    }
                } else if line.contains("\"action\":\"stop\"") {
                    let _ = ev_tx.send(Event::Stop).await;
                    break;
                }
            }
        });

        // Spawn outgoing writer task
        tokio::spawn(async move {
            while let Some(msg) = out_rx.recv().await {
                if let Ok(mut json) = serde_json::to_string(&msg) {
                    json.push('\n');
                    if writer_half.write_all(json.as_bytes()).await.is_err() {
                        break;
                    }
                    if writer_half.flush().await.is_err() {
                        break;
                    }
                }
            }
        });

        // Initialize shared memory writer
        let shm_writer = ShmTransportWriter::default();

        Ok(Self {
            node_id: node_id.to_string(),
            event_rx,
            out_tx,
            shm_writer,
        })
    }

    /// Creates an in-memory mock node for tests.
    pub fn mock(node_id: &str) -> (Self, mpsc::Sender<Event>, mpsc::Receiver<DataflowMessage>) {
        let (event_tx, event_rx) = mpsc::channel(128);
        let (out_tx, out_rx) = mpsc::channel(128);
        let shm_writer = ShmTransportWriter::new(16 * 1024 * 1024);

        (
            Self {
                node_id: node_id.to_string(),
                event_rx,
                out_tx,
                shm_writer,
            },
            event_tx,
            out_rx,
        )
    }

    /// Asynchronously receives the next event from input queues or clock.
    pub async fn next_event(&mut self) -> Option<Event> {
        self.event_rx.recv().await
    }

    /// Sends an output payload to the specified port. Automatically selects zero-copy
    /// shared memory if payload size > 4KB, or inline if <= 4KB.
    pub async fn send_output(
        &mut self,
        port: &str,
        data: &[u8],
        metadata: &FrameMetadata,
    ) -> Result<()> {
        let message = if data.len() > SHM_THRESHOLD {
            let shm_id = format!("dora_{}_{}", self.node_id, port);
            let desc = self
                .shm_writer
                .write(&shm_id, data, metadata.timestamp_micros)
                .map_err(|e| DoraError::Shm(e.to_string()))?;
            DataflowMessage::new_shm(&self.node_id, port, metadata.clone(), desc)
        } else {
            DataflowMessage::new_inline(&self.node_id, port, metadata.clone(), data.to_vec())
        };

        self.out_tx
            .send(message)
            .await
            .map_err(|_| DoraError::Channel("Failed to dispatch output message".to_string()))?;

        Ok(())
    }

    pub fn node_id(&self) -> &str {
        &self.node_id
    }
}