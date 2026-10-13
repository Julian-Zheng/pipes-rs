use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::error::{DoraError, Result};
use crate::queue::QueueConfig;

/// Root configuration schema for a declarative dataflow pipeline YAML.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataflowDescriptor {
    pub id: String,
    #[serde(default)]
    pub simulation: SimulationConfig,
    pub nodes: Vec<NodeConfig>,
}

impl DataflowDescriptor {
    /// Loads and parses a dataflow YAML configuration from a file path.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_yaml_str(&content)
    }

    /// Parses a dataflow YAML configuration string.
    pub fn from_yaml_str(yaml_str: &str) -> Result<Self> {
        let descriptor: DataflowDescriptor = serde_yaml::from_str(yaml_str)?;
        descriptor.validate()?;
        Ok(descriptor)
    }

    /// Validates topology and mappings.
    pub fn validate(&self) -> Result<()> {
        let mut node_ids = std::collections::HashSet::new();
        let mut output_registry = std::collections::HashSet::new();

        for node in &self.nodes {
            if !node_ids.insert(&node.id) {
                return Err(DoraError::Config(format!(
                    "Duplicate node ID detected: {}",
                    node.id
                )));
            }
            for out in &node.outputs {
                output_registry.insert(format!("{}/{}", node.id, out));
            }
        }

        // Validate that all node inputs point to existing outputs
        for node in &self.nodes {
            for (port, mapping) in &node.inputs {
                if !output_registry.contains(&mapping.source) {
                    return Err(DoraError::Config(format!(
                        "Node '{}' input '{}' points to unknown source '{}'",
                        node.id, port, mapping.source
                    )));
                }
            }
        }

        Ok(())
    }
}

/// Simulation-specific global clock and execution parameters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimulationConfig {
    /// Target simulation frequency in Hz (e.g., 10.0 Hz for Waymo Open Dataset sensors).
    #[serde(default = "default_clock_rate")]
    pub clock_rate_hz: f64,
    /// When true, processes advance synchronously via frame_timestamp_micros.
    #[serde(default = "default_deterministic")]
    pub deterministic: bool,
    /// Maximum number of frames to run before auto-stopping (useful for tests/benchmarks).
    pub max_frames: Option<u64>,
}

fn default_clock_rate() -> f64 {
    10.0
}

fn default_deterministic() -> bool {
    true
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            clock_rate_hz: default_clock_rate(),
            deterministic: default_deterministic(),
            max_frames: None,
        }
    }
}

/// Configuration for an individual node in the pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeConfig {
    pub id: String,
    pub name: Option<String>,
    pub path: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub inputs: HashMap<String, InputMapping>,
    #[serde(default)]
    pub outputs: Vec<String>,
    pub simulation_jitter: Option<JitterConfig>,
}

/// Input port configuration supporting both shorthand string (`node/port`)
/// and detailed object specification.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct InputMapping {
    pub source: String,
    pub queue: QueueConfig,
}

impl InputMapping {
    pub fn parse_source(&self) -> Result<(String, String)> {
        let parts: Vec<&str> = self.source.split('/').collect();
        if parts.len() != 2 {
            return Err(DoraError::Config(format!(
                "Invalid input source identifier '{}'. Expected 'node_id/output_port'",
                self.source
            )));
        }
        Ok((parts[0].to_string(), parts[1].to_string()))
    }
}

impl<'de> Deserialize<'de> for InputMapping {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Helper {
            Short(String),
            Detailed {
                source: String,
                #[serde(default)]
                queue: QueueConfig,
            },
        }

        match Helper::deserialize(deserializer)? {
            Helper::Short(source) => Ok(InputMapping {
                source,
                queue: QueueConfig::default(),
            }),
            Helper::Detailed { source, queue } => Ok(InputMapping { source, queue }),
        }
    }
}

/// Artificial latency and compute jitter injection for benchmarking downstream robustness.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JitterConfig {
    #[serde(default)]
    pub mean_delay_ms: u64,
    #[serde(default)]
    pub std_dev_ms: f64,
    pub fixed_delay_ms: Option<u64>,
}

impl JitterConfig {
    pub fn fixed(ms: u64) -> Self {
        Self {
            mean_delay_ms: ms,
            std_dev_ms: 0.0,
            fixed_delay_ms: Some(ms),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_declarative_pipeline_yaml() {
        let yaml = r#"
id: waymo-test-pipeline
simulation:
  clock_rate_hz: 10.0
  deterministic: true
  max_frames: 100

nodes:
  - id: waymo-source
    path: target/debug/mock_waymo_source
    outputs:
      - lidar_top
      - camera_front

  - id: perception-filter
    path: target/debug/perception_filter
    inputs:
      points:
        source: waymo-source/lidar_top
        queue:
          policy: drop_oldest
          capacity: 2
      camera: waymo-source/camera_front
    outputs:
      - clusters
    simulation_jitter:
      mean_delay_ms: 30
      std_dev_ms: 5.0
"#;

        let descriptor = DataflowDescriptor::from_yaml_str(yaml).expect("Failed to parse YAML");
        assert_eq!(descriptor.id, "waymo-test-pipeline");
        assert_eq!(descriptor.nodes.len(), 2);

        let filter_node = descriptor
            .nodes
            .iter()
            .find(|n| n.id == "perception-filter")
            .unwrap();
        assert_eq!(
            filter_node.inputs.get("camera").unwrap().source,
            "waymo-source/camera_front"
        );
        assert_eq!(
            filter_node.inputs.get("points").unwrap().queue.capacity,
            2
        );
    }
}
