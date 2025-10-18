// Copyright 2023
//! Example benchmark comparing JSON vs Bincode serialization for low latency applications

extern crate bincode;
extern crate multicast_rs;
extern crate serde;
extern crate serde_json;

use std::env;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use multicast_rs::publisher::MulticastPublisher;
use multicast_rs::subscriber::{DeserializeFormat, MulticastSubscriber};
use serde::{Deserialize, Serialize};

// Define a realistic trading message struct
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
struct MarketDataUpdate {
    symbol: String,
    bid: f64,
    ask: f64,
    bid_size: u32,
    ask_size: u32,
    timestamp: u64,
    sequence: u64,
    exchange_id: u16,
    flags: u8,
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // How many messages to benchmark
    let message_count = args
        .get(1)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(10_000);

    // IP and port for the benchmark
    let benchmark_ip = "224.0.0.123";
    let benchmark_port = 7646; // Using separate port to avoid conflicts

    println!("======= Multicast Serialization Format Benchmark =======");
    println!("Comparing JSON vs Bincode for {message_count} messages");
    println!("Using multicast address: {benchmark_ip}:{benchmark_port}");
    println!();

    // Create sample market data
    let sample_data = create_sample_data(message_count);
    println!("Generated {} sample market data records", sample_data.len());

    // Benchmark serialization formats
    benchmark_serialization(&sample_data);

    // Benchmark end-to-end multicast performance with each format
    benchmark_multicast(benchmark_ip, benchmark_port, &sample_data);
}

fn create_sample_data(count: usize) -> Vec<MarketDataUpdate> {
    let symbols = [
        "AAPL", "MSFT", "GOOGL", "AMZN", "TSLA", "META", "NFLX", "NVDA",
    ];
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let mut result = Vec::with_capacity(count);

    for i in 0..count {
        let symbol = symbols[i % symbols.len()];
        let base_price = match symbol {
            "AAPL" => 180.0,
            "MSFT" => 350.0,
            "GOOGL" => 140.0,
            "AMZN" => 130.0,
            "TSLA" => 240.0,
            "META" => 310.0,
            "NFLX" => 560.0,
            "NVDA" => 430.0,
            _ => 100.0,
        };

        // Create realistic price variations
        let bid = base_price * (1.0 - (i as f64 * 0.0001).sin() * 0.001);
        let ask = bid + 0.01 + (i as f64 * 0.0001).cos() * 0.005;

        result.push(MarketDataUpdate {
            symbol: symbol.to_string(),
            bid,
            ask,
            bid_size: (100 + (i % 20) * 10) as u32,
            ask_size: (100 + ((i + 5) % 20) * 10) as u32,
            timestamp: now + i as u64,
            sequence: i as u64,
            exchange_id: (i % 5) as u16,
            flags: (i % 8) as u8,
        });
    }

    result
}

fn benchmark_serialization(data: &[MarketDataUpdate]) {
    println!("=== Serialization Performance ===");

    let single_record = &data[0];

    // Benchmark JSON serialization for a single record
    let start = Instant::now();
    let json = serde_json::to_string(single_record).unwrap();
    let json_single_time = start.elapsed();
    let json_size = json.len();

    // Benchmark Bincode serialization for a single record
    let start = Instant::now();
    let bincode =
        bincode::serde::encode_to_vec(single_record, bincode::config::standard()).unwrap();
    let bincode_single_time = start.elapsed();
    let bincode_size = bincode.len();

    println!("Single record serialization:");
    println!("  JSON:    {json_single_time:?} ({json_size} bytes)");
    println!("  Bincode: {bincode_single_time:?} ({bincode_size} bytes)");
    println!(
        "  Size reduction with Bincode: {:.1}%",
        100.0 * (json_size - bincode_size) as f64 / json_size as f64
    );
    println!(
        "  Speed improvement with Bincode: {:.1}x",
        json_single_time.as_nanos() as f64 / bincode_single_time.as_nanos() as f64
    );
    println!();

    // Benchmark batch serialization
    println!("Batch serialization ({} records):", data.len());

    // JSON batch
    let start = Instant::now();
    let mut json_total_size = 0;
    for record in data {
        let json = serde_json::to_string(record).unwrap();
        json_total_size += json.len();
    }
    let json_batch_time = start.elapsed();

    // Bincode batch
    let start = Instant::now();
    let mut bincode_total_size = 0;
    for record in data {
        let bincode = bincode::serde::encode_to_vec(record, bincode::config::standard()).unwrap();
        bincode_total_size += bincode.len();
    }
    let bincode_batch_time = start.elapsed();

    println!(
        "  JSON:    {:?} (avg {} bytes/record)",
        json_batch_time,
        json_total_size / data.len()
    );
    println!(
        "  Bincode: {:?} (avg {} bytes/record)",
        bincode_batch_time,
        bincode_total_size / data.len()
    );
    println!(
        "  Throughput improvement with Bincode: {:.1}x",
        json_batch_time.as_nanos() as f64 / bincode_batch_time.as_nanos() as f64
    );
    println!();
}

fn benchmark_multicast(ip: &str, port: u16, data: &[MarketDataUpdate]) {
    println!("=== Multicast Performance ===");
    println!("Benchmarking end-to-end multicast performance with 1000 messages");
    println!("Note: This test uses loopback interface so actual network performance may differ");

    let sample_data = &data[0..1000]; // Use first 1000 records for this test

    // JSON round-trip test
    if let Ok(publisher) = MulticastPublisher::new_str(ip, Some(port)) {
        if let Ok(mut subscriber) = MulticastSubscriber::new_str(ip, Some(port), Some(8192)) {
            println!("\nTesting JSON format round-trip:");

            // Create buffer for received messages
            let mut received = vec![MarketDataUpdate::default(); 1000];

            // Time to send all messages with JSON
            let send_start = Instant::now();
            for record in sample_data {
                if let Ok(json) = serde_json::to_string(record) {
                    let _ = publisher.publish(&json);
                }
            }
            let send_time = send_start.elapsed();
            println!("  Sent 1000 JSON messages in: {send_time:?}");

            // Time to receive and deserialize
            let receive_start = Instant::now();
            let max_wait_ns = 3_000_000_000; // 3 seconds max wait

            match subscriber.receive_batch_with_format(
                &mut received,
                Some(max_wait_ns),
                DeserializeFormat::Json,
            ) {
                Ok(count) => {
                    let receive_time = receive_start.elapsed();
                    println!("  Received and deserialized {count} messages in: {receive_time:?}");
                    if count > 0 {
                        println!(
                            "  Average JSON round-trip time per message: {:?}",
                            receive_time / count as u32
                        );
                    }
                }
                Err(e) => {
                    println!("  Error receiving JSON messages: {e}");
                }
            }
        }
    }

    // Brief pause between tests
    std::thread::sleep(Duration::from_millis(1000));

    // Bincode round-trip test
    if let Ok(publisher) = MulticastPublisher::new_str(ip, Some(port + 1)) {
        if let Ok(mut subscriber) = MulticastSubscriber::new_str(ip, Some(port + 1), Some(8192)) {
            println!("\nTesting Bincode format round-trip:");

            // Create buffer for received messages
            let mut received = vec![MarketDataUpdate::default(); 1000];

            // Time to send all messages with Bincode
            let send_start = Instant::now();
            for record in sample_data {
                if let Ok(binary) =
                    bincode::serde::encode_to_vec(record, bincode::config::standard())
                {
                    let _ = publisher.publish(&binary);
                }
            }
            let send_time = send_start.elapsed();
            println!("  Sent 1000 Bincode messages in: {send_time:?}");

            // Time to receive and deserialize
            let receive_start = Instant::now();
            let max_wait_ns = 3_000_000_000; // 3 seconds max wait

            match subscriber.receive_batch_with_format(
                &mut received,
                Some(max_wait_ns),
                DeserializeFormat::Bincode,
            ) {
                Ok(count) => {
                    let receive_time = receive_start.elapsed();
                    println!("  Received and deserialized {count} messages in: {receive_time:?}");
                    if count > 0 {
                        println!(
                            "  Average Bincode round-trip time per message: {:?}",
                            receive_time / count as u32
                        );
                    }
                }
                Err(e) => {
                    println!("  Error receiving Bincode messages: {e}");
                }
            }
        }
    }

    println!("\n=== Serialization Format Recommendations ===");
    println!("For low-latency trading applications:");
    println!("1. Use Bincode when communicating between Rust applications");
    println!("2. Consider alternatives like FlatBuffers or Cap'n Proto for even lower latency");
    println!("3. Use JSON only when interoperability with other systems is required");
}
