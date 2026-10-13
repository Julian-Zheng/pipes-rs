use std::collections::HashMap;
use std::sync::Arc;
use dora_lite_core::config::DataflowDescriptor;
use dora_lite_core::message::DataflowMessage;

use crate::queue::PortQueue;

pub type RouteKey = (String, String); // (source_node, output_port)
pub type TargetPort = (String, String); // (target_node, target_input_port)

/// Routing table that maps (source_node, output_port) -> [ (target_node, target_port) ].
#[derive(Default)]
pub struct Router {
    routes: HashMap<RouteKey, Vec<TargetPort>>,
    node_queues: HashMap<TargetPort, Arc<PortQueue>>,
}

impl Router {
    pub fn build(descriptor: &DataflowDescriptor) -> Self {
        let mut routes: HashMap<RouteKey, Vec<TargetPort>> = HashMap::new();
        let mut node_queues: HashMap<TargetPort, Arc<PortQueue>> = HashMap::new();

        for node in &descriptor.nodes {
            for (in_port, mapping) in &node.inputs {
                if let Ok((src_node, src_port)) = mapping.parse_source() {
                    let route_key = (src_node, src_port);
                    let target = (node.id.clone(), in_port.clone());

                    routes.entry(route_key).or_default().push(target.clone());

                    let port_queue = Arc::new(PortQueue::new(
                        format!("{}/{}", node.id, in_port),
                        mapping.queue.clone(),
                    ));
                    node_queues.insert(target, port_queue);
                }
            }
        }

        Self {
            routes,
            node_queues,
        }
    }

    /// Dispatches an output message to all subscribing target queues.
    pub async fn dispatch(&self, msg: DataflowMessage) {
        let key = (msg.source_node.clone(), msg.output_port.clone());
        if let Some(targets) = self.routes.get(&key) {
            for target in targets {
                if let Some(queue) = self.node_queues.get(target) {
                    let mut forwarded = msg.clone();
                    forwarded.output_port = target.1.clone(); // rebind to the target's input port name
                    queue.push(forwarded).await;
                }
            }
        }
    }

    pub fn get_queues_for_node(&self, node_id: &str) -> Vec<(String, Arc<PortQueue>)> {
        self.node_queues
            .iter()
            .filter(|((nid, _), _)| nid == node_id)
            .map(|((_, port), q)| (port.clone(), q.clone()))
            .collect()
    }
}
