// Copyright 2023
//! Example of using the MulticastSubscriber's receive_batch method with deserialization

extern crate multicast_rs;
extern crate serde;
extern crate serde_json;

use std::env;
use std::io;
use std::time::Duration;

use multicast_rs::subscriber::MulticastSubscriber;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct TradeMessage {
    symbol: String,
    price: f64,
    quantity: u32,
    timestamp: u64,
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // Parse the batch size from command line or use default
    let batch_size = args
        .get(1)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(10);

    // Parse max processing time in ms from command line or use default
    let max_time_ms = args
        .get(2)
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(1000); // Default 1000ms = 1s

    println!("Creating IPv4 multicast subscriber...");

    // Create a subscriber for the default IPv4 multicast address
    let mut subscriber = match MulticastSubscriber::new_str("224.0.0.123", None, None) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error creating subscriber: {}", e);
            return;
        }
    };

    println!("Listening on {} for messages...", subscriber.address());
    println!(
        "Will process batches of up to {} messages with {}ms max processing time",
        batch_size, max_time_ms
    );

    // Create a buffer to receive deserialized TradeMessage objects
    let mut trade_buffer = vec![TradeMessage::default(); batch_size];

    // Convert milliseconds to nanoseconds
    let max_time_ns = max_time_ms * 1_000_000;

    // Continuously listen for message batches until ctrl+c
    loop {
        println!("Waiting for batch of messages (press Ctrl+C to exit)...");

        // Wait for messages and deserialize them into the buffer
        match subscriber.receive_batch(&mut trade_buffer, Some(max_time_ns)) {
            Ok(count) => {
                println!("Received and deserialized {} messages:", count);

                // Print the deserialized messages
                for i in 0..count {
                    let trade = &trade_buffer[i];
                    println!(
                        "  {}: {} @ ${:.2} x {} (timestamp: {})",
                        i + 1,
                        trade.symbol,
                        trade.price,
                        trade.quantity,
                        trade.timestamp
                    );
                }
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                // Timeout occurred, just continue
                println!("Timeout waiting for messages, continuing...");
            }
            Err(e) if e.kind() == io::ErrorKind::InvalidData => {
                // Deserialization error
                eprintln!("Error deserializing message: {}", e);
                // Sleep a bit to avoid tight loop on errors
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                eprintln!("Error receiving messages: {}", e);
                // Sleep a bit to avoid tight loop on errors
                std::thread::sleep(Duration::from_millis(100));
            }
        }

        // Brief pause between batches
        std::thread::sleep(Duration::from_millis(500));
    }
}
