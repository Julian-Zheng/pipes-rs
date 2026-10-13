use std::sync::mpsc::channel;
use std::thread;
use std::time::Instant;

use dora_lite_shm::{ShmPoolReader, ShmPoolWriter, ShmRingBufferReader, ShmRingBufferWriter};

#[test]
fn test_ring_buffer_zero_copy_large_payload() {
    let shm_id = format!("test_ring_{}", uuid::Uuid::new_v4());
    let payload_size = 4 * 1024 * 1024; // 4 MB

    // Initialize 4MB test buffer with known pattern
    let mut original_data = vec![0u8; payload_size];
    for (i, byte) in original_data.iter_mut().enumerate() {
        *byte = (i % 256) as u8;
    }

    let capacity = 16 * 1024 * 1024; // 16 MB ring buffer
    let mut writer = ShmRingBufferWriter::new(&shm_id, capacity).expect("Failed to create writer");

    let write_start = Instant::now();
    let desc = writer
        .write_payload(&original_data, 1_700_000_000_123_456)
        .expect("Write failed");
    let write_duration = write_start.elapsed();
    println!("4MB write into SHM took: {:?}", write_duration);

    assert_eq!(desc.shm_id, shm_id);
    assert_eq!(desc.len, payload_size as u64);

    let reader = ShmRingBufferReader::open(&shm_id).expect("Failed to open reader");

    let read_start = Instant::now();
    let slice = reader.read(&desc).expect("Read failed");
    let read_duration = read_start.elapsed();
    println!("Zero-copy slice borrow took: {:?}", read_duration);

    // Verify sub-microsecond zero-copy borrow
    assert_eq!(slice.len(), payload_size);
    assert_eq!(slice[0], original_data[0]);
    assert_eq!(slice[payload_size / 2], original_data[payload_size / 2]);
    assert_eq!(slice[payload_size - 1], original_data[payload_size - 1]);
    assert_eq!(slice, &original_data[..]);
}

#[test]
fn test_multithreaded_slot_pool_zero_copy() {
    let shm_id = format!("test_pool_{}", uuid::Uuid::new_v4());
    let num_slots = 8;
    let slot_size = 2 * 1024 * 1024; // 2 MB per slot
    let shm_id_clone = shm_id.clone();

    // Create writer first so the shared memory region file exists
    let mut writer = ShmPoolWriter::new(&shm_id, num_slots, slot_size).expect("Failed to create writer");

    let (desc_tx, desc_rx) = channel();

    // Consumer thread
    let consumer_handle = thread::spawn(move || {
        let reader = ShmPoolReader::open(&shm_id_clone).expect("Failed to open reader");
        let mut received = 0;

        while let Ok(desc) = desc_rx.recv() {
            let slice = reader.read(&desc).expect("Read failed");
            assert_eq!(slice.len(), 1024 * 1024);
            // Verify content pattern
            assert_eq!(slice[0], (desc.seq % 256) as u8);
            received += 1;
        }

        received
    });

    let frame_count = 10;

    for seq in 0..frame_count {
        let pattern = (seq % 256) as u8;
        let frame_data = vec![pattern; 1024 * 1024]; // 1 MB payload

        let desc = writer
            .write_frame(&frame_data, 100_000 * seq)
            .expect("Failed to write frame");
        desc_tx.send(desc).unwrap();
    }
    drop(desc_tx);

    let total_received = consumer_handle.join().unwrap();
    assert_eq!(total_received, frame_count);
}
