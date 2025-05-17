// Copyright 2023
//! Example of using the MulticastSubscriber

extern crate multicast_rs;

use std::env;
use std::io;
use std::time::Duration;

use multicast_rs::subscriber::MulticastSubscriber;

fn main() {
    let args: Vec<String> = env::args().collect();

    // Optional response message
    let response = args
        .get(1)
        .map(|s| s.as_str())
        .unwrap_or("ACK from subscriber");

    println!("Creating IPv4 multicast subscriber...");

    // Create a subscriber for the default IPv4 multicast address
    // Using the new string-based API instead of requiring IpAddr
    let subscriber = match MulticastSubscriber::new_str("224.0.0.123", None, None) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error creating subscriber: {}", e);
            return;
        }
    };

    println!("Listening on {} for messages...", subscriber.address());
    println!("Will respond with: \"{}\"", response);

    // Continuously listen for messages until ctrl+c
    loop {
        println!("Waiting for message (press Ctrl+C to exit)...");

        // Wait for a message and respond
        match subscriber.receive_and_respond(None, response) {
            Ok((data, addr)) => {
                let data_str = String::from_utf8_lossy(&data);
                println!("Received message from {}: \"{}\"", addr, data_str);
                println!("Sent response: \"{}\"", response);
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                // Timeout occurred, just continue
                println!("Timeout waiting for message, continuing...");
            }
            Err(e) => {
                eprintln!("Error receiving message: {}", e);
                // Sleep a bit to avoid tight loop on errors
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
}
