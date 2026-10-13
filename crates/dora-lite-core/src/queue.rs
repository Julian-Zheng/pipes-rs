use serde::{Deserialize, Serialize};

/// Queue policy for input ports in the dataflow pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum QueuePolicyKind {
    /// Drops oldest incoming frames when downstream perception nodes lag behind.
    #[default]
    DropOldest,
    /// Blocks or throttles upstream readers when downstream capacity is exhausted.
    Backpressure,
}

/// Detailed configuration for a port's queue policy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QueueConfig {
    #[serde(default)]
    pub policy: QueuePolicyKind,
    #[serde(default = "default_capacity")]
    pub capacity: usize,
}

fn default_capacity() -> usize {
    10
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            policy: QueuePolicyKind::DropOldest,
            capacity: default_capacity(),
        }
    }
}

impl QueueConfig {
    pub fn drop_oldest(capacity: usize) -> Self {
        Self {
            policy: QueuePolicyKind::DropOldest,
            capacity,
        }
    }

    pub fn backpressure(capacity: usize) -> Self {
        Self {
            policy: QueuePolicyKind::Backpressure,
            capacity,
        }
    }
}
