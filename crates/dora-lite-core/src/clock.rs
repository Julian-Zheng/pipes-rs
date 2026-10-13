use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Deterministic Waymo Open Dataset simulation clock.
/// Ensures sensors (LiDAR, cameras, poses) synchronize on identical frame indices
/// and microsecond timestamps.
#[derive(Debug, Clone)]
pub struct SimulationClock {
    clock_rate_hz: f64,
    step_micros: u64,
    current_frame: Arc<AtomicU64>,
    base_timestamp_micros: u64,
}

impl SimulationClock {
    pub fn new(clock_rate_hz: f64, base_timestamp_micros: u64) -> Self {
        let step_micros = if clock_rate_hz > 0.0 {
            (1_000_000.0 / clock_rate_hz) as u64
        } else {
            100_000 // default 10Hz = 100ms
        };

        Self {
            clock_rate_hz,
            step_micros,
            current_frame: Arc::new(AtomicU64::new(0)),
            base_timestamp_micros,
        }
    }

    /// Advances the clock to the next frame and returns (frame_id, timestamp_micros).
    pub fn tick(&self) -> (u64, u64) {
        let frame_id = self.current_frame.fetch_add(1, Ordering::SeqCst);
        let timestamp_micros = self.base_timestamp_micros + (frame_id * self.step_micros);
        (frame_id, timestamp_micros)
    }

    /// Retrieves current frame ID and microsecond timestamp without advancing.
    pub fn current(&self) -> (u64, u64) {
        let frame_id = self.current_frame.load(Ordering::SeqCst);
        let timestamp_micros = self.base_timestamp_micros + (frame_id * self.step_micros);
        (frame_id, timestamp_micros)
    }

    pub fn clock_rate_hz(&self) -> f64 {
        self.clock_rate_hz
    }

    pub fn step_micros(&self) -> u64 {
        self.step_micros
    }
}

impl Default for SimulationClock {
    fn default() -> Self {
        Self::new(10.0, 1_700_000_000_000_000)
    }
}
