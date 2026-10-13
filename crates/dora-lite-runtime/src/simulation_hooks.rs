use std::time::Duration;
use dora_lite_core::config::JitterConfig;
use tracing::trace;

/// Simulation hook for injecting compute jitter and artificial processing latency.
#[derive(Debug, Clone)]
pub struct JitterInjector {
    mean_delay_ms: f64,
    std_dev_ms: f64,
    fixed_delay_ms: Option<u64>,
}

impl JitterInjector {
    pub fn new(config: &JitterConfig) -> Self {
        Self {
            mean_delay_ms: config.mean_delay_ms as f64,
            std_dev_ms: config.std_dev_ms,
            fixed_delay_ms: config.fixed_delay_ms,
        }
    }

    /// Computes the sampled jitter delay duration.
    pub fn sample_delay(&self) -> Duration {
        if let Some(fixed) = self.fixed_delay_ms {
            return Duration::from_millis(fixed);
        }

        if self.std_dev_ms <= 0.0 {
            return Duration::from_millis(self.mean_delay_ms as u64);
        }

        // Box-Muller transform to sample from Normal(mean, std_dev)
        let u1: f64 = rand::random::<f64>().max(1e-7);
        let u2: f64 = rand::random::<f64>();
        let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();

        let sampled_ms = (self.mean_delay_ms + (z0 * self.std_dev_ms)).max(0.0);
        Duration::from_micros((sampled_ms * 1000.0) as u64)
    }

    /// Asynchronously injects the artificial latency.
    pub async fn inject(&self) {
        let delay = self.sample_delay();
        trace!("Injecting simulation jitter delay: {:?}", delay);
        tokio::time::sleep(delay).await;
    }
}
