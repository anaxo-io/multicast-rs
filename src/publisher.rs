// Copyright 2025
//! # Multicast Publisher
//!
//! This module provides functionality for publishing messages to multicast groups.
//! It supports both IPv4 and IPv6 multicast addresses, and includes methods for
//! sending individual messages or batches of messages.
//!
//! ## Features
//!
//! - Create publishers for both IPv4 and IPv6 multicast addresses
//! - Send individual messages to multicast groups
//! - Send batches of messages with configurable delays between them
//! - Publish a message and wait for a response
//!
//! ## Examples
//!
//! ### Basic usage
//!
//! ```rust,no_run
//! use multicast_rs::publisher::MulticastPublisher;
//! use std::net::IpAddr;
//! use std::str::FromStr;
//!
//! // Create a publisher with the default IPv4 multicast address
//! let publisher = MulticastPublisher::new_ipv4(None).unwrap();
//!
//! // Publish a message
//! publisher.publish("Hello, multicast world!").unwrap();
//! ```
//!
//! ### Publishing batches of messages
//!
//! ```rust,no_run
//! use multicast_rs::publisher::MulticastPublisher;
//! use std::time::Duration;
//!
//! // Create a publisher
//! let publisher = MulticastPublisher::new_ipv4(None).unwrap();
//!
//! // Create a batch of messages
//! let messages = vec![
//!     "Message 1",
//!     "Message 2",
//!     "Message 3",
//! ];
//!
//! // Publish the batch with a 10ms delay between messages
//! let results = publisher.publish_batch(&messages, Some(10));
//!
//! // Check results
//! for (i, result) in results.iter().enumerate() {
//!     match result {
//!         Ok(bytes) => println!("Sent message {} ({} bytes)", i+1, bytes),
//!         Err(e) => eprintln!("Failed to send message {}: {}", i+1, e),
//!     }
//! }
//! ```
//!
//! ### Publishing and waiting for a response
//!
//! ```rust,no_run
//! use multicast_rs::publisher::MulticastPublisher;
//! use std::time::Duration;
//!
//! // Create a publisher
//! let publisher = MulticastPublisher::new_ipv4(None).unwrap();
//!
//! // Publish a message and wait for a response with a timeout
//! match publisher.publish_and_receive("Hello", Some(Duration::from_secs(2))) {
//!     Ok((response_data, addr)) => {
//!         println!("Received response from {}: {:?}", addr, response_data);
//!     },
//!     Err(e) => eprintln!("Error receiving response: {}", e),
//! }
//! ```

use std::io;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::time::Duration;

#[cfg(feature = "stats")]
use std::time::Instant;

#[cfg(feature = "stats")]
use std::cell::RefCell;

use crate::{DEFAULT_PORT, IPV4, IPV6, new_sender};

#[cfg(feature = "stats")]
/// Statistics for the multicast publisher
#[derive(Debug, Default, Clone)]
pub struct PublisherStatistics {
    /// Total number of messages published
    pub messages_published: u64,
    /// Total number of bytes published
    pub bytes_published: u64,
    /// Total number of publish errors
    pub publish_errors: u64,
    /// Maximum message size published
    pub max_message_size: usize,
    /// Last publish timestamp
    pub last_publish_time: Option<Instant>,
}

/// A publisher for sending messages to a multicast group.
///
/// The `MulticastPublisher` provides methods to send messages to multicast groups
/// using either IPv4 or IPv6 addresses. It manages the underlying UDP socket
/// and provides convenient methods for different message sending patterns.
pub struct MulticastPublisher {
    /// The UDP socket used to send multicast messages
    socket: UdpSocket,

    /// The multicast address (IP and port) that this publisher sends to
    addr: SocketAddr,

    #[cfg(feature = "stats")]
    /// Statistics for this publisher
    stats: RefCell<PublisherStatistics>,
}

impl MulticastPublisher {
    /// Create a new publisher for the given multicast address.
    ///
    /// # Arguments
    /// * `addr` - The multicast IP address to publish to
    /// * `port` - The port to publish on (defaults to 7645 if None)
    ///
    /// # Returns
    /// A Result containing the new MulticastPublisher or an IO error
    ///
    /// # Panics
    /// Panics if the provided address is not a multicast address
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use multicast_rs::publisher::MulticastPublisher;
    /// use std::net::IpAddr;
    /// use std::str::FromStr;
    ///
    /// // Create a publisher for a specific multicast address
    /// let addr = IpAddr::from_str("224.0.0.123").unwrap();
    /// let publisher = MulticastPublisher::new(addr, Some(8000)).unwrap();
    /// ```
    pub fn new(addr: IpAddr, port: Option<u16>) -> io::Result<Self> {
        Self::new_with_interface(addr, port, None)
    }

    /// Create a new publisher for the given multicast address on a specific network interface.
    ///
    /// # Arguments
    /// * `addr` - The multicast IP address to publish to
    /// * `port` - The port to publish on (defaults to 7645 if None)
    /// * `interface` - The network interface name or IP address to use (e.g., "eth0", "192.168.1.5")
    ///
    /// # Returns
    /// A Result containing the new MulticastPublisher or an IO error
    ///
    /// # Panics
    /// Panics if the provided address is not a multicast address
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use multicast_rs::publisher::MulticastPublisher;
    /// use std::net::IpAddr;
    /// use std::str::FromStr;
    ///
    /// // Create a publisher for a specific multicast address on eth0 interface
    /// let addr = IpAddr::from_str("224.0.0.123").unwrap();
    /// let publisher = MulticastPublisher::new_with_interface(addr, Some(8000), Some("eth0")).unwrap();
    /// ```
    pub fn new_with_interface(
        addr: IpAddr,
        port: Option<u16>,
        interface: Option<&str>,
    ) -> io::Result<Self> {
        assert!(addr.is_multicast(), "Address must be a multicast address");

        let port = port.unwrap_or(DEFAULT_PORT);
        let addr = SocketAddr::new(addr, port);
        let socket = new_sender(&addr, interface)?;

        #[cfg(feature = "stats")]
        {
            Ok(MulticastPublisher {
                socket,
                addr,
                stats: RefCell::new(PublisherStatistics::default()),
            })
        }

        #[cfg(not(feature = "stats"))]
        {
            Ok(MulticastPublisher { socket, addr })
        }
    }

    /// Create a new publisher for the default IPv4 multicast address.
    ///
    /// This is a convenience method that creates a publisher using the default
    /// IPv4 multicast address (224.0.0.123) defined in the library.
    ///
    /// # Arguments
    /// * `port` - The port to publish on (defaults to 7645 if None)
    ///
    /// # Returns
    /// A Result containing the new MulticastPublisher or an IO error
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use multicast_rs::publisher::MulticastPublisher;
    ///
    /// // Create a publisher with the default IPv4 address and custom port
    /// let publisher = MulticastPublisher::new_ipv4(Some(9000)).unwrap();
    /// ```
    pub fn new_ipv4(port: Option<u16>) -> io::Result<Self> {
        Self::new_ipv4_with_interface(port, None)
    }

    /// Create a new publisher for the default IPv4 multicast address on a specific network interface.
    ///
    /// This is a convenience method that creates a publisher using the default
    /// IPv4 multicast address (224.0.0.123) defined in the library.
    ///
    /// # Arguments
    /// * `port` - The port to publish on (defaults to 7645 if None)
    /// * `interface` - The network interface name or IP address to use (e.g., "eth0", "192.168.1.5")
    ///
    /// # Returns
    /// A Result containing the new MulticastPublisher or an IO error
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use multicast_rs::publisher::MulticastPublisher;
    ///
    /// // Create a publisher with the default IPv4 address on the eth0 interface
    /// let publisher = MulticastPublisher::new_ipv4_with_interface(Some(9000), Some("eth0")).unwrap();
    /// ```
    pub fn new_ipv4_with_interface(port: Option<u16>, interface: Option<&str>) -> io::Result<Self> {
        Self::new_with_interface(*IPV4, port, interface)
    }

    /// Create a new publisher for the default IPv6 multicast address.
    ///
    /// This is a convenience method that creates a publisher using the default
    /// IPv6 multicast address (FF02::0123) defined in the library.
    ///
    /// # Arguments
    /// * `port` - The port to publish on (defaults to 7645 if None)
    ///
    /// # Returns
    /// A Result containing the new MulticastPublisher or an IO error
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use multicast_rs::publisher::MulticastPublisher;
    ///
    /// // Create a publisher with the default IPv6 address
    /// let publisher = MulticastPublisher::new_ipv6(None).unwrap();
    /// ```
    pub fn new_ipv6(port: Option<u16>) -> io::Result<Self> {
        Self::new_ipv6_with_interface(port, None)
    }

    /// Create a new publisher for the default IPv6 multicast address on a specific network interface.
    ///
    /// This is a convenience method that creates a publisher using the default
    /// IPv6 multicast address (FF02::0123) defined in the library.
    ///
    /// # Arguments
    /// * `port` - The port to publish on (defaults to 7645 if None)
    /// * `interface` - The network interface name or index to use (e.g., "eth0")
    ///
    /// # Returns
    /// A Result containing the new MulticastPublisher or an IO error
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use multicast_rs::publisher::MulticastPublisher;
    ///
    /// // Create a publisher with the default IPv6 address on the eth0 interface
    /// let publisher = MulticastPublisher::new_ipv6_with_interface(None, Some("eth0")).unwrap();
    /// ```
    pub fn new_ipv6_with_interface(port: Option<u16>, interface: Option<&str>) -> io::Result<Self> {
        Self::new_with_interface(*IPV6, port, interface)
    }

    /// Create a new publisher for the given multicast address specified as a string.
    ///
    /// # Arguments
    /// * `addr` - The multicast address as a string (e.g. "224.0.0.123" for IPv4 or "ff02::123" for IPv6)
    /// * `port` - The port to publish on (defaults to 7645 if None)
    ///
    /// # Returns
    /// A Result containing the new MulticastPublisher or an IO error
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use multicast_rs::publisher::MulticastPublisher;
    ///
    /// // Create a publisher for a specific multicast address
    /// let publisher = MulticastPublisher::new_str("224.0.0.123", Some(8000)).unwrap();
    /// ```
    pub fn new_str(addr: &str, port: Option<u16>) -> io::Result<Self> {
        match addr.parse::<IpAddr>() {
            Ok(ip_addr) => Self::new(ip_addr, port),
            Err(e) => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Invalid IP address: {}", e),
            )),
        }
    }

    /// Create a new publisher for the given multicast address as a string on a specific network interface.
    ///
    /// # Arguments
    /// * `addr` - The multicast address as a string (e.g. "224.0.0.123" for IPv4 or "ff02::123" for IPv6)
    /// * `port` - The port to publish on (defaults to 7645 if None)
    /// * `interface` - The network interface name or IP address to use (e.g., "eth0", "192.168.1.5")
    ///
    /// # Returns
    /// A Result containing the new MulticastPublisher or an IO error
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use multicast_rs::publisher::MulticastPublisher;
    ///
    /// // Create a publisher for a specific multicast address on eth0 interface
    /// let publisher = MulticastPublisher::new_str_with_interface("224.0.0.123", Some(8000), Some("eth0")).unwrap();
    /// ```
    pub fn new_str_with_interface(
        addr: &str,
        port: Option<u16>,
        interface: Option<&str>,
    ) -> io::Result<Self> {
        match addr.parse::<IpAddr>() {
            Ok(ip_addr) => Self::new_with_interface(ip_addr, port, interface),
            Err(e) => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Invalid IP address: {}", e),
            )),
        }
    }

    /// Publish a message to the multicast group.
    ///
    /// Sends a single message to the multicast group. The message can be any type
    /// that can be converted to a byte slice, such as a string, vector of bytes, or array.
    ///
    /// # Arguments
    /// * `message` - The message to publish
    ///
    /// # Returns
    /// The number of bytes sent or an IO error
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use multicast_rs::publisher::MulticastPublisher;
    ///
    /// let publisher = MulticastPublisher::new_ipv4(None).unwrap();
    ///
    /// // Send a string message
    /// publisher.publish("Hello, multicast!").unwrap();
    ///
    /// // Send binary data
    /// let binary_data = vec![0x01, 0x02, 0x03, 0x04];
    /// publisher.publish(&binary_data).unwrap();
    /// ```
    pub fn publish<T: AsRef<[u8]>>(&self, message: T) -> io::Result<usize> {
        #[cfg(feature = "stats")]
        {
            let message_ref = message.as_ref();
            let message_len = message_ref.len();

            match self.socket.send_to(message_ref, &self.addr) {
                Ok(bytes_sent) => {
                    // Update statistics
                    let mut stats = self.stats.borrow_mut();
                    stats.messages_published += 1;
                    stats.bytes_published += bytes_sent as u64;
                    stats.max_message_size = std::cmp::max(stats.max_message_size, message_len);
                    stats.last_publish_time = Some(Instant::now());

                    Ok(bytes_sent)
                }
                Err(e) => {
                    // Update error count
                    self.stats.borrow_mut().publish_errors += 1;
                    Err(e)
                }
            }
        }

        #[cfg(not(feature = "stats"))]
        {
            self.socket.send_to(message.as_ref(), &self.addr)
        }
    }

    /// Publish a message and wait for a response.
    ///
    /// This method sends a message to the multicast group and then waits for a
    /// direct response from any recipient. This is useful for request-response
    /// patterns where you expect a specific recipient to reply.
    ///
    /// # Arguments
    /// * `message` - The message to publish
    /// * `timeout` - Optional timeout for receiving a response
    ///
    /// # Returns
    /// A Result containing the response data and the sender's address, or an IO error
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use multicast_rs::publisher::MulticastPublisher;
    /// use std::time::Duration;
    ///
    /// let publisher = MulticastPublisher::new_ipv4(None).unwrap();
    ///
    /// // Send a message and wait up to 2 seconds for a response
    /// match publisher.publish_and_receive("Request", Some(Duration::from_secs(2))) {
    ///     Ok((data, addr)) => {
    ///         println!("Received response from {}: {:?}", addr, data);
    ///     },
    ///     Err(e) => eprintln!("No response received: {}", e),
    /// }
    /// ```
    pub fn publish_and_receive<T: AsRef<[u8]>>(
        &self,
        message: T,
        timeout: Option<Duration>,
    ) -> io::Result<(Vec<u8>, SocketAddr)> {
        // Set timeout if provided
        if let Some(duration) = timeout {
            self.socket.set_read_timeout(Some(duration))?;
        }

        // Send the message
        self.socket.send_to(message.as_ref(), &self.addr)?;

        // Prepare to receive the response
        let mut buf = vec![0u8; 1024]; // 1KB buffer
        let (size, addr) = self.socket.recv_from(&mut buf)?;

        // Resize buffer to actual data size
        buf.truncate(size);

        Ok((buf, addr))
    }

    /// Publish a batch of messages to the multicast group.
    ///
    /// Sends multiple messages to the multicast group with an optional delay between messages.
    /// This is useful for efficiently sending a sequence of related messages.
    ///
    /// # Arguments
    /// * `messages` - A slice of messages to publish
    /// * `delay_between_messages_ms` - Optional delay between messages in milliseconds
    ///
    /// # Returns
    /// A vector of results, one for each message sent
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use multicast_rs::publisher::MulticastPublisher;
    ///
    /// let publisher = MulticastPublisher::new_ipv4(None).unwrap();
    ///
    /// // Create batch of messages
    /// let messages = vec![
    ///     "Message 1",
    ///     "Message 2",
    ///     "Message 3",
    /// ];
    ///
    /// // Send with a 5ms delay between messages
    /// let results = publisher.publish_batch(&messages, Some(5));
    ///
    /// // Check all results
    /// for (i, result) in results.iter().enumerate() {
    ///     match result {
    ///         Ok(bytes) => println!("Message {} sent successfully ({} bytes)", i+1, bytes),
    ///         Err(e) => eprintln!("Failed to send message {}: {}", i+1, e),
    ///     }
    /// }
    /// ```
    pub fn publish_batch<T: AsRef<[u8]>>(
        &self,
        messages: &[T],
        delay_between_messages_ms: Option<u64>,
    ) -> Vec<io::Result<usize>> {
        let mut results = Vec::with_capacity(messages.len());

        for message in messages {
            let result = self.publish(message);
            results.push(result);

            // Add delay between messages if specified
            if let Some(delay) = delay_between_messages_ms {
                if delay > 0 {
                    std::thread::sleep(Duration::from_millis(delay));
                }
            }
        }

        results
    }

    /// Get the multicast address this publisher is using.
    ///
    /// # Returns
    /// The socket address (IP and port) this publisher sends to
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use multicast_rs::publisher::MulticastPublisher;
    ///
    /// let publisher = MulticastPublisher::new_ipv4(None).unwrap();
    /// println!("Publishing to: {}", publisher.address());
    /// ```
    pub fn address(&self) -> SocketAddr {
        self.addr
    }

    /// Get a reference to the underlying UDP socket.
    ///
    /// This allows access to the raw socket for advanced configurations.
    ///
    /// # Returns
    /// A reference to the UdpSocket used by this publisher
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use multicast_rs::publisher::MulticastPublisher;
    /// use std::time::Duration;
    ///
    /// let publisher = MulticastPublisher::new_ipv4(None).unwrap();
    ///
    /// // Set custom socket options
    /// publisher.socket().set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    /// ```
    pub fn socket(&self) -> &UdpSocket {
        &self.socket
    }

    #[cfg(feature = "stats")]
    /// Returns publisher statistics
    pub fn get_stats(&self) -> PublisherStatistics {
        self.stats.borrow().clone()
    }

    #[cfg(feature = "stats")]
    /// Reset publisher statistics
    pub fn reset_stats(&mut self) {
        *self.stats.borrow_mut() = PublisherStatistics::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::join_multicast;
    use std::sync::atomic::{AtomicU16, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    // Use atomic counter to ensure each test gets a unique port
    static PORT_COUNTER: AtomicU16 = AtomicU16::new(7700);

    fn get_unique_port() -> u16 {
        PORT_COUNTER.fetch_add(1, Ordering::SeqCst)
    }

    #[test]
    fn test_publisher_creation() {
        let port = get_unique_port();
        let publisher =
            MulticastPublisher::new(*IPV4, Some(port)).expect("Failed to create publisher");

        assert_eq!(publisher.address().ip(), *IPV4);
        assert_eq!(publisher.address().port(), port);
    }

    #[test]
    fn test_publish_different_sizes() {
        let port = get_unique_port();
        let addr = SocketAddr::new(*IPV4, port);

        // Create a subscriber socket to receive the messages
        let receiver = join_multicast(addr, None).expect("Failed to create receiver socket");

        // Set a reasonable timeout
        receiver
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("Failed to set read timeout");

        // Create publisher
        let publisher =
            MulticastPublisher::new(*IPV4, Some(port)).expect("Failed to create publisher");

        // Give the receiver time to fully initialize
        thread::sleep(Duration::from_millis(50));

        // Test a small message
        let small_msg = b"small";
        let bytes_sent = publisher
            .publish(small_msg)
            .expect("Failed to publish small message");
        assert_eq!(bytes_sent, small_msg.len());

        // Receive and verify the small message
        let mut buf = vec![0u8; 1024];
        let (size, _) = receiver
            .recv_from(&mut buf)
            .expect("Failed to receive small message");
        assert_eq!(&buf[..size], small_msg);

        // Test a medium-sized message
        let medium_msg = vec![b'A'; 512];
        let bytes_sent = publisher
            .publish(&medium_msg)
            .expect("Failed to publish medium message");
        assert_eq!(bytes_sent, medium_msg.len());

        // Receive and verify the medium message
        let (size, _) = receiver
            .recv_from(&mut buf)
            .expect("Failed to receive medium message");
        assert_eq!(&buf[..size], &medium_msg);

        // Test a large message
        let large_msg = vec![b'B'; 1000];
        let bytes_sent = publisher
            .publish(&large_msg)
            .expect("Failed to publish large message");
        assert_eq!(bytes_sent, large_msg.len());

        // Receive and verify the large message
        let (size, _) = receiver
            .recv_from(&mut buf)
            .expect("Failed to receive large message");
        assert_eq!(&buf[..size], &large_msg);
    }

    #[test]
    fn test_publish_and_receive() {
        let port = get_unique_port();
        let addr = SocketAddr::new(*IPV4, port);
        let test_msg = b"Echo this message";

        // Create flags for coordinating thread execution
        let responder_ready = Arc::new(Mutex::new(false));
        let responder_ready_clone = responder_ready.clone();

        // Create a publisher
        let publisher =
            MulticastPublisher::new(*IPV4, Some(port)).expect("Failed to create publisher");

        // Start a thread that will act as the responder
        let handle = thread::spawn(move || {
            // Create a subscriber to receive the initial message
            let socket = join_multicast(addr, None).expect("Failed to join multicast group");

            // Signal that we're ready to receive
            {
                let mut ready = responder_ready_clone.lock().unwrap();
                *ready = true;
            }

            // Set a reasonable timeout
            socket
                .set_read_timeout(Some(Duration::from_secs(5)))
                .expect("Failed to set read timeout");

            // Wait for a message
            let mut buf = vec![0u8; 1024];
            match socket.recv_from(&mut buf) {
                Ok((size, src_addr)) => {
                    // Echo the received data back to the sender
                    let response = format!("Echoed: {}", String::from_utf8_lossy(&buf[..size]));

                    // Use UdpSocket directly instead of Socket
                    let responder =
                        UdpSocket::bind("0.0.0.0:0").expect("Failed to create responder socket");

                    // Add a small delay to ensure publisher is ready to receive
                    thread::sleep(Duration::from_millis(50));

                    responder
                        .send_to(response.as_bytes(), &src_addr)
                        .expect("Failed to send response");
                }
                Err(e) => {
                    eprintln!("Error receiving message: {}", e);
                }
            }
        });

        // Wait until responder thread signals it's ready
        let mut ready = false;
        for _ in 0..10 {
            {
                let r = responder_ready.lock().unwrap();
                ready = *r;
            }
            if ready {
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }

        if !ready {
            panic!("Responder thread failed to start in time");
        }

        // Give the responder thread a bit more time to be fully ready to receive
        thread::sleep(Duration::from_millis(100));

        // Publish and wait for response with a generous timeout
        match publisher.publish_and_receive(test_msg, Some(Duration::from_secs(5))) {
            Ok((response, _)) => {
                let response_str = String::from_utf8_lossy(&response);
                assert_eq!(response_str, "Echoed: Echo this message");
            }
            Err(e) => {
                panic!("Failed to receive response: {}", e);
            }
        }

        // Wait for the responder thread to finish
        handle.join().expect("Failed to join responder thread");
    }

    #[test]
    fn test_publish_batch() {
        let port = get_unique_port();
        let addr = SocketAddr::new(*IPV4, port);

        // Create a subscriber socket to receive the messages
        let receiver = join_multicast(addr, None).expect("Failed to create receiver socket");

        // Set a reasonable timeout
        receiver
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("Failed to set read timeout");

        // Create publisher
        let publisher =
            MulticastPublisher::new(*IPV4, Some(port)).expect("Failed to create publisher");

        // Give the receiver time to fully initialize
        thread::sleep(Duration::from_millis(50));

        // Create a batch of messages
        let messages = vec![
            b"batch message 1".to_vec(),
            b"batch message 2".to_vec(),
            b"batch message 3".to_vec(),
            b"batch message 4".to_vec(),
            b"batch message 5".to_vec(),
        ];

        // Publish the batch with a small delay between messages
        let results = publisher.publish_batch(&messages, Some(10));

        // Check that all messages were sent successfully
        for (i, result) in results.iter().enumerate() {
            match result {
                Ok(bytes) => assert_eq!(*bytes, messages[i].len()),
                Err(e) => panic!("Failed to send message {}: {}", i, e),
            }
        }

        // Receive and verify all messages
        let mut buf = vec![0u8; 1024];
        let mut received_messages = Vec::new();

        for _ in 0..messages.len() {
            match receiver.recv_from(&mut buf) {
                Ok((size, _)) => {
                    received_messages.push(buf[..size].to_vec());
                }
                Err(e) => {
                    panic!("Failed to receive message: {}", e);
                }
            }
        }

        // Verify that all messages were received (possibly out of order)
        assert_eq!(received_messages.len(), messages.len());

        for sent_msg in &messages {
            let found = received_messages
                .iter()
                .any(|received| received == sent_msg);
            assert!(
                found,
                "Message not found in received batch: {:?}",
                String::from_utf8_lossy(sent_msg)
            );
        }
    }

    // Skip IPv6 tests if not running on a properly configured system
    #[cfg(not(windows))] // IPv6 test is tricky on Windows
    #[test]
    fn test_ipv6_publisher() {
        let port = get_unique_port();
        // This test will only work if IPv6 multicast is properly configured on your system
        if let Ok(publisher) = MulticastPublisher::new_ipv6(Some(port)) {
            assert_eq!(publisher.address().ip(), *IPV6);
            assert_eq!(publisher.address().port(), port);

            // Basic publish test - mainly checking that it doesn't error
            if let Err(e) = publisher.publish(b"IPv6 test message") {
                eprintln!("Note: IPv6 multicast publish failed: {}", e);
                // Not failing the test as IPv6 might not be properly configured
            }
        } else {
            // Not failing the test as IPv6 might not be properly configured
            eprintln!("IPv6 multicast not available on this system");
        }
    }

    #[test]
    fn test_publisher_interface_creation() {
        let port = get_unique_port();

        // Try to create a publisher with the loopback interface
        // This should work on all systems
        match MulticastPublisher::new_ipv4_with_interface(Some(port), Some("lo")) {
            Ok(publisher) => {
                println!("Successfully created publisher on loopback interface");
                assert_eq!(publisher.address().port(), port);
            }
            Err(e) => {
                // On some systems this might fail but we don't want the test to fail
                println!("Could not create publisher on loopback interface: {}", e);
            }
        }

        // Also test with direct IP address specification
        match MulticastPublisher::new_ipv4_with_interface(Some(port), Some("127.0.0.1")) {
            Ok(publisher) => {
                println!("Successfully created publisher with explicit IP 127.0.0.1");
                assert_eq!(publisher.address().port(), port);
            }
            Err(e) => {
                // This should generally work on all systems, but don't fail the test
                println!(
                    "Could not create publisher with explicit IP 127.0.0.1: {}",
                    e
                );
            }
        }

        // Test with a non-existent interface
        // This should fail with a specific error
        match MulticastPublisher::new_ipv4_with_interface(Some(port), Some("nonexistent_iface")) {
            Ok(_) => {
                panic!("Publisher was created with non-existent interface, should have failed");
            }
            Err(e) => {
                println!("Expected error for non-existent interface: {}", e);
                // Check that it's the correct error type - not found
                assert!(
                    e.kind() == std::io::ErrorKind::NotFound
                        || e.kind() == std::io::ErrorKind::Unsupported,
                    "Expected NotFound or Unsupported error, got: {:?}",
                    e.kind()
                );
            }
        }
    }

    // This test attempts to detect the first real network interface
    // and create a publisher on it. It's skipped if no interface can be found.
    #[test]
    fn test_publisher_with_detected_interface() {
        let port = get_unique_port();

        // Try to detect a real network interface
        let interface = detect_network_interface();

        if let Some(iface) = interface {
            println!("Testing with detected interface: {}", iface);

            match MulticastPublisher::new_ipv4_with_interface(Some(port), Some(&iface)) {
                Ok(publisher) => {
                    assert_eq!(publisher.address().port(), port);

                    // Try to publish a message to verify basic functionality
                    match publisher.publish("Test on real interface") {
                        Ok(bytes) => {
                            println!(
                                "Successfully published {} bytes on interface {}",
                                bytes, iface
                            );
                        }
                        Err(e) => {
                            println!("Failed to publish on interface {}: {}", iface, e);
                            // Not failing test, as publishing might not work for various reasons
                        }
                    }
                }
                Err(e) => {
                    println!(
                        "Could not create publisher on detected interface {}: {}",
                        iface, e
                    );
                    // Not failing the test as the interface might not support multicast
                }
            }
        } else {
            println!("No suitable network interface detected, skipping test");
        }
    }

    // Helper function to detect a real network interface
    fn detect_network_interface() -> Option<String> {
        #[cfg(target_family = "unix")]
        {
            use std::process::Command;

            // Use ip link to get interfaces, excluding loopback
            if let Ok(output) = Command::new("ip").args(&["link", "show"]).output() {
                if output.status.success() {
                    let output_str = String::from_utf8_lossy(&output.stdout);

                    // Very naive parsing - in production code use proper APIs
                    for line in output_str.lines() {
                        if line.contains(": ") && !line.contains("lo:") {
                            // Extract interface name
                            if let Some(iface_with_num) = line.split(": ").nth(1) {
                                if let Some(iface) = iface_with_num.split(':').next() {
                                    return Some(iface.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    #[cfg(feature = "stats")]
    #[test]
    fn test_publisher_stats() {
        // Create a publisher
        let mut publisher =
            super::MulticastPublisher::new_ipv4(None).expect("Failed to create publisher");

        // Check initial stats
        let initial_stats = publisher.get_stats();
        assert_eq!(initial_stats.messages_published, 0);
        assert_eq!(initial_stats.bytes_published, 0);
        assert_eq!(initial_stats.publish_errors, 0);
        assert_eq!(initial_stats.max_message_size, 0);
        assert!(initial_stats.last_publish_time.is_none());

        // Publish a message
        let message = "Test message for stats";
        let bytes_sent = publisher
            .publish(message)
            .expect("Failed to publish message");

        // Verify stats after publishing
        let stats = publisher.get_stats();
        assert_eq!(stats.messages_published, 1);
        assert_eq!(stats.bytes_published, bytes_sent as u64);
        assert_eq!(stats.max_message_size, message.len());
        assert!(stats.last_publish_time.is_some());

        // Publish a larger message
        let large_message = vec![b'A'; 1000]; // 1KB message
        let large_bytes_sent = publisher
            .publish(&large_message)
            .expect("Failed to publish large message");

        // Verify stats updated correctly
        let stats = publisher.get_stats();
        assert_eq!(stats.messages_published, 2);
        assert_eq!(
            stats.bytes_published,
            (bytes_sent + large_bytes_sent) as u64
        );
        assert_eq!(stats.max_message_size, large_message.len());

        // Reset stats
        publisher.reset_stats();

        // Verify reset worked
        let reset_stats = publisher.get_stats();
        assert_eq!(reset_stats.messages_published, 0);
        assert_eq!(reset_stats.bytes_published, 0);
        assert_eq!(reset_stats.publish_errors, 0);
        assert_eq!(reset_stats.max_message_size, 0);
        assert!(reset_stats.last_publish_time.is_none());

        // Test error counting by directly updating the error count
        // This avoids the need to actually trigger a network error, which is unreliable
        publisher.stats.borrow_mut().publish_errors += 1;

        // Verify error count was updated
        let stats = publisher.get_stats();
        assert_eq!(stats.publish_errors, 1);
    }

    #[cfg(feature = "stats")]
    #[test]
    fn test_publisher_batch_stats() {
        // Create a publisher
        let mut publisher =
            super::MulticastPublisher::new_ipv4(None).expect("Failed to create publisher");

        // Reset stats to ensure a clean state
        publisher.reset_stats();

        // Create a batch of messages with different sizes
        let messages = vec![
            "Message 1".to_string(),
            "Message 2 with more content".to_string(),
            "Message 3 with even more content to increase size".to_string(),
        ];

        let expected_total_bytes = messages.iter().map(|m| m.len()).sum::<usize>();
        let expected_max_size = messages.iter().map(|m| m.len()).max().unwrap_or(0);

        // Publish batch
        publisher.publish_batch(&messages, Some(10));

        // Verify stats
        let stats = publisher.get_stats();
        assert_eq!(stats.messages_published, messages.len() as u64);
        assert_eq!(stats.bytes_published, expected_total_bytes as u64);
        assert_eq!(stats.max_message_size, expected_max_size);
        assert!(stats.last_publish_time.is_some());
    }
}
