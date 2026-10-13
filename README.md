# pipes-rs

Pipes is a Rust project for running sensor processing and fusion
in front of ROS 2.

## The architectural bet

Pipes owns the path from sensor drivers through processing and fusion. Sensor
drivers write directly into Pipes-owned Apache Arrow buffers. Pipeline stages
share those buffers without repeatedly copying or serializing the sensor data.
After fusion, Pipes converts the much smaller results into standard ROS 2
messages.

```text
camera / lidar / radar / IMU
              |
              v
     Rust sensor drivers
              |
              v
 Pipes metadata + Arrow buffers
              |
              v
 processing, synchronization, and fusion
              |
              v
 detections / tracks / state / health
              |
              v
       standard ROS 2 topics
```

Significant performance gains are expected because the largest data stays inside
one controlled pipeline:

- a sensor can write into its pipeline buffer once;
- processing, recording, visualization, and fusion can share that data;
- large images and point clouds do not need to be serialized and reconstructed
  between every stage; and
- ROS conversion happens once, after the data has been reduced to the result the
  rest of the robot needs.

Pipes should interoperate cleanly with ROS rather than replace it. ROS can keep
handling control, planning, mission logic, user interfaces, and communication
across the robot. Existing ROS nodes consume normal topics and do not need to
know that Pipes is running the sensor pipeline.

The same pipeline can run from live sensors, simulation, or recorded MCAP data.
Pipes records sensor timing, input order, synchronization decisions, delays, and
drops. This lets developers isolate the sensor and fusion subsystem, replay the
same workload, tune it quickly, and determine whether a bad result came from the
fusion algorithm or from data arriving late or going missing.

## Fusion in Motion

[Fusion in Motion](https://github.com/EthanMBoos/fusion-in-motion) is a separate
simulation playground. It can generate repeatable sensor workloads for Pipes,
but neither project depends on the other.


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
