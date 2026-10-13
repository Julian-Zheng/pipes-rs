# dora-lite

A high-performance, 100% Rust-based dataflow robotics runtime inspired by [dora-rs](https://github.com/dora-rs/dora), designed specifically for multi-sensor autonomous vehicle simulation (such as the Waymo Open Dataset).

## Key Highlights

- **Zero-Copy IPC via Shared Memory:** Uses POSIX shared memory / memory-mapped files (`memmap2`) for streaming multi-megabyte payloads (LiDAR point clouds, camera streams) passing only 48-byte `ShmDescriptor`s over node queues.
- **Configurable Queue Backpressure & Drop Policies:**
  - `drop_oldest`: Discards stale frames when downstream perception nodes lag behind real-time simulation speeds.
  - `backpressure`: Throttles upstream data generation.
- **Declarative YAML Pipelines:** Define nodes, ports, binaries, queue policies, and simulation jitter directly in YAML.
- **Deterministic Simulation Clock:** Sensors step synchronously according to `frame_timestamp_micros`.
- **Latency & Compute Jitter Injection:** Built-in simulation hooks to model realistic GPU/CPU execution jitter.

## Workspace Architecture

- [`crates/dora-lite-core`](crates/dora-lite-core): Core dataflow descriptors, YAML configuration schemas, metadata definitions, and queue policies.
- [`crates/dora-lite-shm`](crates/dora-lite-shm): High-performance shared memory pools and circular ring buffers with zero-copy readers/writers.
- [`crates/dora-lite-node-api`](crates/dora-lite-node-api): Ergonomic Rust SDK for authoring pipeline nodes.
- [`crates/dora-lite-runtime`](crates/dora-lite-runtime): Process supervisor, Unix domain socket control plane, message router, and daemon CLI (`dora-daemon`).
- [`examples/waymo-sim`](examples/waymo-sim): End-to-end multi-sensor pipeline running a mock Waymo source, perception filter with simulated GPU jitter, and latency sink.

## Quick Start

### 1. Build Workspace
```bash
cargo build --all
```

### 2. Run Tests
```bash
cargo test --all
```

### 3. Run Waymo Simulation Example
```bash
cargo run -p dora-lite-runtime --bin dora-daemon -- run --dataflow examples/waymo-sim/dataflow.yaml
```
