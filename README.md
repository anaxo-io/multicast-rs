# Multicast for Rust

A Rust library for IPv4 and IPv6 multicast networking with a clean, easy-to-use API.

## Features

- Create publishers for both IPv4 and IPv6 multicast addresses
- Send individual messages to multicast groups
- Send batches of messages with configurable delays between them
- Publish a message and wait for a response
- Optional statistics tracking for publishers and subscribers (enabled via feature flag)

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
multicast-rs = "0.1.0"
```

To enable statistics tracking, include the optional "stats" feature:

```toml
[dependencies]
multicast-rs = { version = "0.1.0", features = ["stats"] }
```

## Quick Start

Publishing to a multicast group is straightforward. Here's a simple example:

```rust
use multicast_rs::publisher::MulticastPublisher;

// Create a publisher with the default IPv4 multicast address
let publisher = MulticastPublisher::new_ipv4(None).unwrap();

// Publish a message
publisher.publish("Hello, multicast world!").unwrap();
```

Subscribing to messages is also easy. Here's how to set up a receiver:

```rust
use multicast_rs::subscriber::MulticastSubscriber;
use std::time::Duration;

// Create a subscriber for the default IPv4 multicast address
let subscriber = MulticastSubscriber::new_ipv4(None, None).unwrap();

// Receive a message with a timeout
match subscriber.receive(Some(Duration::from_secs(5))) {
    Ok((data, addr)) => {
        println!("Received from {}: {:?}", addr, data);
    },
    Err(e) => eprintln!("Error receiving message: {}", e),
}
```

## Statistics Tracking

When compiled with the `stats` feature, the library provides detailed statistics tracking for both publishers and subscribers:

### Publisher Statistics

```rust
use multicast_rs::publisher::MulticastPublisher;

let mut publisher = MulticastPublisher::new_ipv4(None).unwrap();

// Publish some messages
publisher.publish("Message 1").unwrap();
publisher.publish("Message 2").unwrap();

// Get statistics
let stats = publisher.get_stats();
println!("Messages published: {}", stats.messages_published);
println!("Bytes published: {}", stats.bytes_published);
println!("Max message size: {}", stats.max_message_size);

// Reset statistics if needed
publisher.reset_stats();
```

### Subscriber Statistics

```rust
use multicast_rs::subscriber::MulticastSubscriber;
use std::time::Duration;

let mut subscriber = MulticastSubscriber::new_ipv4(None, None).unwrap();

// Receive some messages...

// Get statistics
let stats = subscriber.get_stats();
println!("Messages received: {}", stats.messages_received);
println!("Bytes received: {}", stats.bytes_received);
println!("Deserialization errors: {}", stats.deserialization_errors);
println!("Average processing time (ns): {}", stats.avg_processing_time_ns);

// Reset statistics
subscriber.reset_stats();
```

## License
This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Contributing
Contributions are welcome! Please read the [CONTRIBUTING.md](CONTRIBUTING.md) file for details on how to get started.

## Acknowledgements
This project is inspired by the need for a simple and efficient way to handle multicast networking in Rust. Thanks to the Rust community for their support and contributions.
