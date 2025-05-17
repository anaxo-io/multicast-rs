// Copyright 2023
//! Example of using the MulticastPublisher

extern crate multicast_rs;

use std::env;
use std::time::Duration;

use multicast_rs::publisher::MulticastPublisher;

fn main() {
    let args: Vec<String> = env::args().collect();

    // Default message if not provided
    let message = args
        .get(1)
        .map(|s| s.as_str())
        .unwrap_or("Hello, multicast world!");

    println!("Creating IPv4 multicast publisher...");

    // Create a publisher for the default IPv4 multicast address
    let publisher = match MulticastPublisher::new_ipv4(None) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error creating publisher: {}", e);
            return;
        }
    };

    println!("Publishing to {}: \"{}\"", publisher.address(), message);

    // Publish the message and wait for any response with a 2-second timeout
    match publisher.publish_and_receive(message, Some(Duration::from_secs(2))) {
        Ok((data, addr)) => {
            let data_str = String::from_utf8_lossy(&data);
            println!("Received response from {}: {}", addr, data_str);
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::WouldBlock {
                println!("No response received within timeout period.");
            } else {
                eprintln!("Error receiving response: {}", e);
            }
        }
    }
}
