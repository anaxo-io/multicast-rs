# Multicast for Rust

A Rust library for IPv4 and IPv6 multicast networking with a clean, easy-to-use API.

## Features

- Create publishers for both IPv4 and IPv6 multicast addresses
- Send individual messages to multicast groups
- Send batches of messages with configurable delays between them
- Publish a message and wait for a response

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
multicast-rs = "0.1.0"
```

## Quick Start

Publish to a multicast group is straightforward. Here's a simple example:

```rust
use multicast::publisher::MulticastPublisher;

// Create a publisher with the default IPv4 multicast address
let publisher = MulticastPublisher::new_ipv4(None).unwrap();

// Publish a message
publisher.publish("Hello, multicast world!").unwrap();
```

Subscribe messages is also easy. Here's how to set up a receiver:

```rust
use multicast::receiver::MulticastSubscriber;

// Create a subscriber for the default IPv4 multicast address
let subscriber = MulticastSubscriber::new_ipv4(None).unwrap();
// Subscribe to messages
subscriber.subscribe("Hello, multicast world!").unwrap();
```

## License
This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Contributing
Contributions are welcome! Please read the [CONTRIBUTING.md](CONTRIBUTING.md) file for details on how to get started.

## Acknowledgements
This project is inspired by the need for a simple and efficient way to handle multicast networking in Rust. Thanks to the Rust community for their support and contributions.
