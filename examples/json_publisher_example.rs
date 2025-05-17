// Copyright 2023
//! Example of publishing JSON serialized data for the batch_deserialize_example

extern crate multicast_rs;
extern crate serde;
extern crate serde_json;

use std::env;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use multicast_rs::publisher::MulticastPublisher;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct TradeMessage {
    symbol: String,
    price: f64,
    quantity: u32,
    timestamp: u64,
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // How many messages to send (default 20)
    let message_count = args
        .get(1)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(20);

    // Delay between messages in ms (default 100ms)
    let delay_ms = args
        .get(2)
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(100);

    println!("Creating IPv4 multicast publisher...");

    // Create a publisher for the default IPv4 multicast address
    let publisher = match MulticastPublisher::new_str("224.0.0.123", None) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error creating publisher: {}", e);
            return;
        }
    };

    println!("Publishing to {}...", publisher.address());
    println!(
        "Will send {} JSON messages with {}ms delay between them",
        message_count, delay_ms
    );

    // Sample stock symbols
    let symbols = ["AAPL", "MSFT", "GOOGL", "AMZN", "TSLA"];

    // Send sample trade messages
    for i in 1..=message_count {
        // Create a trade message with sample data
        let symbol = symbols[i % symbols.len()];
        let trade = TradeMessage {
            symbol: symbol.to_string(),
            price: 100.0 + (i as f64 * 0.25),
            quantity: 100 + (i * 10) as u32,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        };

        // Serialize to JSON
        let json = match serde_json::to_string(&trade) {
            Ok(j) => j,
            Err(e) => {
                eprintln!("Error serializing message: {}", e);
                continue;
            }
        };

        // Publish the JSON string
        match publisher.publish(&json) {
            Ok(bytes) => {
                println!(
                    "Sent {} bytes: {} @ ${:.2} x {} ({})",
                    bytes, trade.symbol, trade.price, trade.quantity, i
                );
            }
            Err(e) => {
                eprintln!("Error publishing message: {}", e);
            }
        }

        // Wait before sending the next message
        thread::sleep(Duration::from_millis(delay_ms));
    }

    println!("Finished sending {} messages", message_count);
}
