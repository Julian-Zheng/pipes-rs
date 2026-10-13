use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use dora_lite_core::config::DataflowDescriptor;
use dora_lite_core::message::DataflowMessage;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, info, warn};

use crate::channel::Router;
use crate::error::{Result, RuntimeError};
use crate::simulation_hooks::JitterInjector;

/// Active node session connected via Unix Domain Socket.
struct NodeSession {
    _node_id: String,
    tx: mpsc::Sender<DataflowMessage>,
}

/// The runtime coordinator supervising the dataflow execution graph.
pub struct Coordinator {
    descriptor: DataflowDescriptor,
    socket_path: PathBuf,
    router: Arc<Router>,
    sessions: Arc<RwLock<HashMap<String, NodeSession>>>,
    children: Arc<Mutex<Vec<(String, Child)>>>,
}

impl Coordinator {
    pub fn new(descriptor: DataflowDescriptor) -> Self {
        let sock_id = format!("dora_coord_{}.sock", uuid::Uuid::new_v4());
        let socket_path = std::env::temp_dir().join(sock_id);
        let router = Arc::new(Router::build(&descriptor));

        Self {
            descriptor,
            socket_path,
            router,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            children: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Starts the coordinator listener, spawns all configured nodes, and coordinates messages.
    pub async fn run(&self) -> Result<()> {
        info!("Starting dora-lite coordinator for pipeline '{}'", self.descriptor.id);

        if self.socket_path.exists() {
            let _ = std::fs::remove_file(&self.socket_path);
        }

        let listener = UnixListener::bind(&self.socket_path)?;
        info!("IPC control plane listening on {:?}", self.socket_path);

        // Spawn child node processes
        self.spawn_nodes().await?;

        // Handle incoming node connections
        let router = self.router.clone();
        let sessions = self.sessions.clone();
        let _listener_task = tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let (reader_half, mut writer_half) = stream.into_split();
                let router_clone = router.clone();
                let sessions_clone = sessions.clone();

                tokio::spawn(async move {
                    let mut lines = BufReader::new(reader_half).lines();
                    let mut registered_node_id: Option<String> = None;

                    // Read registration
                    if let Ok(Some(first_line)) = lines.next_line().await
                        && let Ok(val) = serde_json::from_str::<serde_json::Value>(&first_line)
                        && val["action"] == "register"
                        && let Some(id) = val["node_id"].as_str()
                    {
                        registered_node_id = Some(id.to_string());
                    }

                    let node_id = match registered_node_id {
                        Some(id) => {
                            info!("Node '{}' registered successfully with coordinator", id);
                            id
                        }
                        None => {
                            warn!("Client connected but failed to register properly");
                            return;
                        }
                    };

                    let (node_tx, mut node_rx) = mpsc::channel::<DataflowMessage>(128);

                    // Register session
                    {
                        let mut sess = sessions_clone.write().await;
                        sess.insert(node_id.clone(), NodeSession {
                            _node_id: node_id.clone(),
                            tx: node_tx,
                        });
                    }

                    // Task to write outgoing messages from coordinator to the node's socket
                    tokio::spawn(async move {
                        while let Some(msg) = node_rx.recv().await {
                            if let Ok(mut json) = serde_json::to_string(&msg) {
                                json.push('\n');
                                if writer_half.write_all(json.as_bytes()).await.is_err() {
                                    break;
                                }
                                let _ = writer_half.flush().await;
                            }
                        }
                    });

                    // Read incoming output messages from the node
                    while let Ok(Some(line)) = lines.next_line().await {
                        if line.trim().is_empty() {
                            continue;
                        }
                        if let Ok(msg) = serde_json::from_str::<DataflowMessage>(&line) {
                            debug!(
                                "Routing message from '{}/{}' (seq {})",
                                msg.source_node, msg.output_port, msg.metadata.frame_id
                            );
                            router_clone.dispatch(msg).await;
                        }
                    }

                    // Cleanup session on disconnect
                    let mut sess = sessions_clone.write().await;
                    sess.remove(&node_id);
                    info!("Node '{}' disconnected", node_id);
                });
            }
        });

        // Spawn queue forwarders to route from PortQueue to connected sockets
        self.spawn_queue_forwarders().await;

        info!("All nodes initialized. Dataflow running.");
        Ok(())
    }

    async fn spawn_queue_forwarders(&self) {
        for node in &self.descriptor.nodes {
            let queues = self.router.get_queues_for_node(&node.id);
            let sessions = self.sessions.clone();
            let jitter_hook = node.simulation_jitter.as_ref().map(JitterInjector::new);
            let node_id = node.id.clone();

            for (_, queue) in queues {
                let q = queue.clone();
                let sess_map = sessions.clone();
                let hook = jitter_hook.clone();
                let nid = node_id.clone();

                tokio::spawn(async move {
                    loop {
                        let msg = q.pop().await;

                        // Inject simulation jitter if configured
                        if let Some(ref j) = hook {
                            j.inject().await;
                        }

                        // Send to the node's session
                        let mut sent = false;
                        for _ in 0..50 {
                            let sess = sess_map.read().await;
                            if let Some(session) = sess.get(&nid)
                                && session.tx.send(msg.clone()).await.is_ok()
                            {
                                sent = true;
                                break;
                            }
                            drop(sess);
                            tokio::time::sleep(Duration::from_millis(20)).await;
                        }

                        if !sent {
                            warn!("Timed out delivering message to node '{}'", nid);
                        }
                    }
                });
            }
        }
    }

    async fn spawn_nodes(&self) -> Result<()> {
        let mut children = self.children.lock().await;

        for node in &self.descriptor.nodes {
            if let Some(ref bin_path) = node.path {
                info!("Spawning node '{}' at executable '{}'", node.id, bin_path);

                let mut cmd = Command::new(bin_path);
                cmd.args(&node.args)
                    .env("DORA_NODE_ID", &node.id)
                    .env("DORA_CONTROL_SOCK", &self.socket_path)
                    .stdin(Stdio::null())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit());

                for (k, v) in &node.env {
                    cmd.env(k, v);
                }

                let child = cmd.spawn().map_err(|e| RuntimeError::ProcessSpawn {
                    node_id: node.id.clone(),
                    message: format!("Command spawn failed for '{}': {}", bin_path, e),
                })?;

                children.push((node.id.clone(), child));
            } else {
                debug!("Node '{}' has no executable path (embedded or mock)", node.id);
            }
        }

        Ok(())
    }

    /// Stops all spawned child processes and cleans up temporary resources.
    pub async fn shutdown(&self) {
        info!("Shutting down dora-lite coordinator...");
        let mut children = self.children.lock().await;
        for (id, mut child) in children.drain(..) {
            debug!("Terminating node '{}'", id);
            let _ = child.kill().await;
        }

        if self.socket_path.exists() {
            let _ = std::fs::remove_file(&self.socket_path);
        }
        info!("Shutdown complete.");
    }
}

impl Drop for Coordinator {
    fn drop(&mut self) {
        if self.socket_path.exists() {
            let _ = std::fs::remove_file(&self.socket_path);
        }
    }
}
