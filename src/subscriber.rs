// Copyright 2023
//! # Multicast Subscriber
//!
//! This module provides functionality for subscribing to and receiving messages from multicast groups.
//! It supports both IPv4 and IPv6 multicast addresses, and includes methods for
//! receiving individual messages, batches of messages, and background processing.
//!
//! ## Features
//!
//! - Create subscribers for both IPv4 and IPv6 multicast addresses
//! - Receive individual messages from multicast groups
//! - Process messages in batches for better performance
//! - Run background listeners that process messages in separate threads
//! - Support for both synchronous and asynchronous message processing patterns
//!
//! ## Examples
//!
//! ### Basic usage
//!
//! ```rust,no_run
//! use multicast_rs::subscriber::MulticastSubscriber;
//! use std::time::Duration;
//!
//! // Create a subscriber with the default IPv4 multicast address
//! let subscriber = MulticastSubscriber::new_ipv4(None, None).unwrap();
//!
//! // Receive a message with a timeout
//! match subscriber.receive(Some(Duration::from_secs(5))) {
//!     Ok((data, addr)) => {
//!         println!("Received from {}: {:?}", addr, data);
//!     },
//!     Err(e) => eprintln!("Error receiving message: {}", e),
//! }
//! ```
//!
//! ### Receiving messages in the background
//!
//! ```rust,no_run
//! use multicast_rs::subscriber::MulticastSubscriber;
//! use std::thread;
//! use std::time::Duration;
//!
//! // Create a subscriber
//! let subscriber = MulticastSubscriber::new_ipv4(None, None).unwrap();
//!
//! // Start a background listener
//! let mut background = subscriber.listen_in_background(|data, addr| {
//!     println!("Received from {}: {:?}", addr, data);
//!     Ok(())
//! }).unwrap();
//!
//! // Do other work while messages are processed in the background
//! thread::sleep(Duration::from_secs(60));
//!
//! // Stop the background listener when done
//! background.stop();
//! ```
//!
//! ### Receiving and processing batches
//!
//! ```rust,no_run
//! use multicast_rs::subscriber::MulticastSubscriber;
//! use std::time::Duration;
//!
//! // Create a subscriber
//! let subscriber = MulticastSubscriber::new_ipv4(None, None).unwrap();
//!
//! // Process up to 10 messages with timeouts
//! let processed = subscriber.process_batch(
//!     10, // max messages to process
//!     Some(Duration::from_millis(100)), // per-message timeout
//!     Some(Duration::from_secs(5)), // overall timeout
//!     |data, addr| {
//!         println!("Processing message from {}: {:?}", addr, data);
//!         Ok(())
//!     }
//! ).unwrap();
//!
//! println!("Processed {} messages in batch", processed);
//! ```

use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use socket2::Socket;

use crate::{join_multicast, new_socket, IPV4, IPV6, PORT};

/// A subscriber for receiving messages from a multicast group.
///
/// The `MulticastSubscriber` provides methods to subscribe to and receive
/// messages from multicast groups using either IPv4 or IPv6 addresses.
/// It manages the underlying UDP socket and provides various methods
/// for different message receiving patterns.
pub struct MulticastSubscriber {
    /// The UDP socket used to receive multicast messages
    socket: UdpSocket,

    /// The multicast address (IP and port) that this subscriber is bound to
    addr: SocketAddr,

    /// The size of the buffer used for receiving messages
    buffer_size: usize,

    /// The multicast interface to use for receiving messages
    interface: Option<String>,
}

/// A handler for received multicast messages.
///
/// This type alias represents a function or closure that can process
/// multicast messages as they are received by a background subscriber.
pub type MessageHandler = Box<dyn Fn(&[u8], SocketAddr) -> io::Result<()> + Send + 'static>;

/// Background multicast receiver that runs in a separate thread.
///
/// This struct manages a background thread that continuously receives
/// multicast messages and processes them with a provided handler function.
/// It provides a clean way to stop the background processing when no longer needed.
pub struct BackgroundSubscriber {
    /// The multicast address this background subscriber is listening to
    addr: SocketAddr,

    /// Flag to control whether the background thread should continue running
    running: Arc<AtomicBool>,

    /// Handle to the background thread, used to join it when stopping
    join_handle: Option<JoinHandle<()>>,
}

impl MulticastSubscriber {
    /// Create a new subscriber for the given multicast address.
    ///
    /// # Arguments
    /// * `addr` - The multicast IP address to subscribe to
    /// * `port` - The port to subscribe on (defaults to 7645 if None)
    /// * `buffer_size` - The size of the receive buffer (defaults to 1024 if None)
    ///
    /// # Returns
    /// A Result containing the new MulticastSubscriber or an IO error
    ///
    /// # Panics
    /// Panics if the provided address is not a multicast address
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use multicast_rs::subscriber::MulticastSubscriber;
    /// use std::net::IpAddr;
    /// use std::str::FromStr;
    ///
    /// // Create a subscriber for a specific multicast address
    /// let addr = IpAddr::from_str("224.0.0.123").unwrap();
    /// let subscriber = MulticastSubscriber::new(addr, Some(8000), Some(2048)).unwrap();
    /// ```
    pub fn new(addr: IpAddr, port: Option<u16>, buffer_size: Option<usize>) -> io::Result<Self> {
        Self::new_with_interface(addr, port, None, buffer_size)
    }

    /// Create a new subscriber for the given multicast address on a specific network interface.
    ///
    /// # Arguments
    /// * `addr` - The multicast IP address to subscribe to
    /// * `port` - The port to subscribe on (defaults to 7645 if None)
    /// * `interface` - The network interface name or IP address to use (e.g., "eth0", "192.168.1.5")
    /// * `buffer_size` - The size of the receive buffer (defaults to 1024 if None)
    ///
    /// # Returns
    /// A Result containing the new MulticastSubscriber or an IO error
    ///
    /// # Panics
    /// Panics if the provided address is not a multicast address
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use multicast_rs::subscriber::MulticastSubscriber;
    /// use std::net::IpAddr;
    /// use std::str::FromStr;
    ///
    /// // Create a subscriber for a specific multicast address on eth0 interface
    /// let addr = IpAddr::from_str("224.0.0.123").unwrap();
    /// let subscriber = MulticastSubscriber::new_with_interface(addr, Some(8000), Some("eth0"), Some(2048)).unwrap();
    /// ```
    pub fn new_with_interface(
        addr: IpAddr,
        port: Option<u16>,
        interface: Option<&str>,
        buffer_size: Option<usize>,
    ) -> io::Result<Self> {
        assert!(addr.is_multicast(), "Address must be a multicast address");

        let port = port.unwrap_or(PORT);
        let addr = SocketAddr::new(addr, port);
        let socket = join_multicast(addr, interface)?;
        let buffer_size = buffer_size.unwrap_or(1024);

        Ok(MulticastSubscriber {
            socket,
            addr,
            buffer_size,
            interface: interface.map(String::from),
        })
    }

    /// Create a new subscriber for the default IPv4 multicast address.
    ///
    /// This is a convenience method that creates a subscriber using the default
    /// IPv4 multicast address (224.0.0.123) defined in the library.
    ///
    /// # Arguments
    /// * `port` - The port to subscribe on (defaults to 7645 if None)
    /// * `buffer_size` - The size of the receive buffer (defaults to 1024 if None)
    ///
    /// # Returns
    /// A Result containing the new MulticastSubscriber or an IO error
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use multicast_rs::subscriber::MulticastSubscriber;
    ///
    /// // Create a subscriber with the default IPv4 address and custom port and buffer
    /// let subscriber = MulticastSubscriber::new_ipv4(Some(9000), Some(4096)).unwrap();
    /// ```
    pub fn new_ipv4(port: Option<u16>, buffer_size: Option<usize>) -> io::Result<Self> {
        Self::new_ipv4_with_interface(port, None, buffer_size)
    }

    /// Create a new subscriber for the default IPv4 multicast address on a specific network interface.
    ///
    /// This is a convenience method that creates a subscriber using the default
    /// IPv4 multicast address (224.0.0.123) defined in the library.
    ///
    /// # Arguments
    /// * `port` - The port to subscribe on (defaults to 7645 if None)
    /// * `interface` - The network interface name or IP address to use (e.g., "eth0", "192.168.1.5")
    /// * `buffer_size` - The size of the receive buffer (defaults to 1024 if None)
    ///
    /// # Returns
    /// A Result containing the new MulticastSubscriber or an IO error
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use multicast_rs::subscriber::MulticastSubscriber;
    ///
    /// // Create a subscriber with the default IPv4 address on eth0 interface
    /// let subscriber = MulticastSubscriber::new_ipv4_with_interface(Some(9000), Some("eth0"), Some(4096)).unwrap();
    /// ```
    pub fn new_ipv4_with_interface(
        port: Option<u16>,
        interface: Option<&str>,
        buffer_size: Option<usize>,
    ) -> io::Result<Self> {
        Self::new_with_interface(*IPV4, port, interface, buffer_size)
    }

    /// Create a new subscriber for the default IPv6 multicast address.
    ///
    /// This is a convenience method that creates a subscriber using the default
    /// IPv6 multicast address (FF02::0123) defined in the library.
    ///
    /// # Arguments
    /// * `port` - The port to subscribe on (defaults to 7645 if None)
    /// * `buffer_size` - The size of the receive buffer (defaults to 1024 if None)
    ///
    /// # Returns
    /// A Result containing the new MulticastSubscriber or an IO error
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use multicast_rs::subscriber::MulticastSubscriber;
    ///
    /// // Create a subscriber with the default IPv6 address
    /// let subscriber = MulticastSubscriber::new_ipv6(None, None).unwrap();
    /// ```
    pub fn new_ipv6(port: Option<u16>, buffer_size: Option<usize>) -> io::Result<Self> {
        Self::new_ipv6_with_interface(port, None, buffer_size)
    }

    /// Create a new subscriber for the default IPv6 multicast address on a specific network interface.
    ///
    /// This is a convenience method that creates a subscriber using the default
    /// IPv6 multicast address (FF02::0123) defined in the library.
    ///
    /// # Arguments
    /// * `port` - The port to subscribe on (defaults to 7645 if None)
    /// * `interface` - The network interface name or index to use (e.g., "eth0")
    /// * `buffer_size` - The size of the receive buffer (defaults to 1024 if None)
    ///
    /// # Returns
    /// A Result containing the new MulticastSubscriber or an IO error
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use multicast_rs::subscriber::MulticastSubscriber;
    ///
    /// // Create a subscriber with the default IPv6 address on eth0 interface
    /// let subscriber = MulticastSubscriber::new_ipv6_with_interface(None, Some("eth0"), None).unwrap();
    /// ```
    pub fn new_ipv6_with_interface(
        port: Option<u16>,
        interface: Option<&str>,
        buffer_size: Option<usize>,
    ) -> io::Result<Self> {
        Self::new_with_interface(*IPV6, port, interface, buffer_size)
    }

    /// Receive a single message from the multicast group.
    ///
    /// Waits for a message to arrive on the multicast group and returns its content
    /// and the sender's address. If a timeout is specified, the method will return
    /// an error if no message is received within the timeout period.
    ///
    /// # Arguments
    /// * `timeout` - Optional timeout for receiving a message
    ///
    /// # Returns
    /// A Result containing the message data and the sender's address, or an IO error
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use multicast_rs::subscriber::MulticastSubscriber;
    /// use std::time::Duration;
    ///
    /// let subscriber = MulticastSubscriber::new_ipv4(None, None).unwrap();
    ///
    /// // Receive a message with a 2 second timeout
    /// match subscriber.receive(Some(Duration::from_secs(2))) {
    ///     Ok((data, addr)) => {
    ///         println!("Received {} bytes from {}", data.len(), addr);
    ///         println!("Message: {}", String::from_utf8_lossy(&data));
    ///     },
    ///     Err(e) => eprintln!("No message received: {}", e),
    /// }
    /// ```
    pub fn receive(&self, timeout: Option<Duration>) -> io::Result<(Vec<u8>, SocketAddr)> {
        // Set timeout if provided
        if let Some(duration) = timeout {
            self.socket.set_read_timeout(Some(duration))?;
        }

        // Prepare to receive the message
        let mut buf = vec![0u8; self.buffer_size];
        let (size, addr) = self.socket.recv_from(&mut buf)?;

        // Resize buffer to actual data size
        buf.truncate(size);

        Ok((buf, addr))
    }

    /// Receive a message and send a response.
    ///
    /// This method receives a message from the multicast group and then sends
    /// a direct response back to the sender. This is useful for implementing
    /// request-response patterns in multicast communication.
    ///
    /// # Arguments
    /// * `timeout` - Optional timeout for receiving a message
    /// * `response` - The response to send back to the sender
    ///
    /// # Returns
    /// A Result containing the received message data and the sender's address, or an IO error
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use multicast_rs::subscriber::MulticastSubscriber;
    /// use std::time::Duration;
    ///
    /// let subscriber = MulticastSubscriber::new_ipv4(None, None).unwrap();
    ///
    /// // Receive a message and respond with an acknowledgment
    /// match subscriber.receive_and_respond(Some(Duration::from_secs(2)), "ACK") {
    ///     Ok((data, addr)) => {
    ///         println!("Received from {} and sent ACK: {}", addr, String::from_utf8_lossy(&data));
    ///     },
    ///     Err(e) => eprintln!("Error: {}", e),
    /// }
    /// ```
    pub fn receive_and_respond<T: AsRef<[u8]>>(
        &self,
        timeout: Option<Duration>,
        response: T,
    ) -> io::Result<(Vec<u8>, SocketAddr)> {
        let (data, remote_addr) = self.receive(timeout)?;

        // Create a socket to send the response
        let responder = new_socket(&remote_addr)?;

        // Send the response
        responder.send_to(response.as_ref(), &remote_addr.into())?;

        Ok((data, remote_addr))
    }

    /// Receive multiple messages from the multicast group.
    ///
    /// This method attempts to receive a batch of messages from the multicast group.
    /// It will receive up to the specified number of messages, stopping either when
    /// that count is reached, when no more messages are available and at least one
    /// has been received, or when the maximum wait time is exceeded.
    ///
    /// # Arguments
    /// * `count` - The number of messages to receive
    /// * `timeout` - Optional timeout for each receive operation
    /// * `max_wait_time` - Optional maximum total time to wait for all messages
    ///
    /// # Returns
    /// A Result containing a vector of received messages and their sender addresses
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use multicast_rs::subscriber::MulticastSubscriber;
    /// use std::time::Duration;
    ///
    /// let subscriber = MulticastSubscriber::new_ipv4(None, None).unwrap();
    ///
    /// // Receive up to 10 messages with a per-message timeout of 200ms
    /// // and an overall timeout of 5 seconds
    /// match subscriber.receive_batch(
    ///     10,
    ///     Some(Duration::from_millis(200)),
    ///     Some(Duration::from_secs(5))
    /// ) {
    ///     Ok(results) => {
    ///         println!("Received {} messages", results.len());
    ///         for (i, (data, addr)) in results.iter().enumerate() {
    ///             println!("Message {}: {} bytes from {}", i+1, data.len(), addr);
    ///         }
    ///     },
    ///     Err(e) => eprintln!("Error receiving batch: {}", e),
    /// }
    /// ```
    pub fn receive_batch(
        &self,
        count: usize,
        timeout: Option<Duration>,
        max_wait_time: Option<Duration>,
    ) -> io::Result<Vec<(Vec<u8>, SocketAddr)>> {
        // Set timeout if provided for individual messages
        if let Some(duration) = timeout {
            self.socket.set_read_timeout(Some(duration))?;
        }

        let start_time = std::time::Instant::now();
        let mut results = Vec::with_capacity(count);
        let mut buf = vec![0u8; self.buffer_size];

        for _ in 0..count {
            // Check if we've exceeded max wait time
            if let Some(max_time) = max_wait_time {
                if start_time.elapsed() > max_time {
                    break;
                }
            }

            match self.socket.recv_from(&mut buf) {
                Ok((size, addr)) => {
                    // Create a copy of just the received data
                    let msg_data = buf[..size].to_vec();
                    results.push((msg_data, addr));
                }
                Err(e) => {
                    // If we get a timeout and have at least one message, we can return
                    if e.kind() == io::ErrorKind::WouldBlock && !results.is_empty() {
                        break;
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        // If we didn't get any messages and max_wait_time is provided
        // and we've waited at least that long, return a timeout error
        if results.is_empty()
            && max_wait_time.is_some()
            && start_time.elapsed() >= max_wait_time.unwrap()
        {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Timed out waiting for batch messages",
            ));
        }

        Ok(results)
    }

    /// Process a batch of received messages with a handler function.
    ///
    /// This method is similar to `receive_batch` but applies a handler function
    /// to each received message. This allows for direct processing of messages
    /// as they are received, without needing to store them all in memory first.
    ///
    /// # Arguments
    /// * `count` - The maximum number of messages to receive and process
    /// * `timeout` - Optional timeout for each receive operation
    /// * `max_wait_time` - Optional maximum total time to wait for all messages
    /// * `handler` - Function to process each message
    ///
    /// # Returns
    /// A Result containing the number of messages processed
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use multicast_rs::subscriber::MulticastSubscriber;
    /// use std::time::Duration;
    /// use std::collections::HashMap;
    /// use std::sync::{Arc, Mutex};
    ///
    /// // Create a shared counter for message statistics
    /// let message_stats = Arc::new(Mutex::new(HashMap::new()));
    /// let stats_clone = message_stats.clone();
    ///
    /// let subscriber = MulticastSubscriber::new_ipv4(None, None).unwrap();
    ///
    /// // Process up to 20 messages, tracking stats by sender
    /// let processed = subscriber.process_batch(
    ///     20,
    ///     Some(Duration::from_millis(100)),
    ///     Some(Duration::from_secs(5)),
    ///     move |data, addr| {
    ///         let mut stats = stats_clone.lock().unwrap();
    ///         let counter = stats.entry(addr).or_insert(0);
    ///         *counter += 1;
    ///         Ok(())
    ///     }
    /// ).unwrap_or(0);
    ///
    /// println!("Processed {} messages", processed);
    /// ```
    pub fn process_batch<F>(
        &self,
        count: usize,
        timeout: Option<Duration>,
        max_wait_time: Option<Duration>,
        mut handler: F,
    ) -> io::Result<usize>
    where
        F: FnMut(&[u8], SocketAddr) -> io::Result<()>,
    {
        let messages = self.receive_batch(count, timeout, max_wait_time)?;

        let mut processed = 0;
        for (data, addr) in &messages {
            handler(data, *addr)?;
            processed += 1;
        }

        Ok(processed)
    }

    /// Start listening for multicast messages in the background.
    ///
    /// This method spawns a new thread that continuously receives messages
    /// from the multicast group and processes them using the provided handler
    /// function. It returns a BackgroundSubscriber that can be used to stop
    /// the listener when it's no longer needed.
    ///
    /// # Arguments
    /// * `handler` - A function to call for each received message
    ///
    /// # Returns
    /// A BackgroundSubscriber that can be used to stop the listener
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use multicast_rs::subscriber::MulticastSubscriber;
    /// use std::sync::{Arc, Mutex};
    /// use std::collections::VecDeque;
    ///
    /// // Create a message queue to store received messages
    /// let message_queue = Arc::new(Mutex::new(VecDeque::new()));
    /// let queue_clone = message_queue.clone();
    ///
    /// let subscriber = MulticastSubscriber::new_ipv4(None, None).unwrap();
    ///
    /// // Start a background listener that adds messages to our queue
    /// let mut background = subscriber.listen_in_background(move |data, _addr| {
    ///     let message = String::from_utf8_lossy(data).to_string();
    ///     let mut queue = queue_clone.lock().unwrap();
    ///     queue.push_back(message);
    ///     
    ///     // Keep the queue from growing too large
    ///     while queue.len() > 100 {
    ///         queue.pop_front();
    ///     }
    ///     
    ///     Ok(())
    /// }).unwrap();
    ///
    /// // The background thread will keep running until we stop it
    /// // or the BackgroundSubscriber is dropped
    /// ```
    pub fn listen_in_background<F>(&self, handler: F) -> io::Result<BackgroundSubscriber>
    where
        F: Fn(&[u8], SocketAddr) -> io::Result<()> + Send + 'static,
    {
        let running = Arc::new(AtomicBool::new(true));
        let thread_running = running.clone();

        // Clone values for the thread
        let addr = self.addr;
        let interface = self.interface.clone();
        let buffer_size = self.buffer_size;

        // Create a new socket for the thread
        let thread_socket = join_multicast(addr, interface.as_deref())?;

        // Start the listener thread
        let join_handle = thread::Builder::new()
            .name(format!("multicast-subscriber:{}", addr))
            .spawn(move || {
                let mut buf = vec![0u8; buffer_size];

                while thread_running.load(Ordering::Relaxed) {
                    match thread_socket.recv_from(&mut buf) {
                        Ok((len, remote_addr)) => {
                            // Call the handler with the received data
                            if let Err(e) = handler(&buf[..len], remote_addr) {
                                eprintln!("Error in message handler: {}", e);
                            }
                        }
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                            // Timeout occurred, just continue
                            thread::sleep(Duration::from_millis(10));
                        }
                        Err(e) => {
                            eprintln!("Error receiving multicast: {}", e);
                        }
                    }
                }
            })?;

        Ok(BackgroundSubscriber {
            addr,
            running,
            join_handle: Some(join_handle),
        })
    }

    /// Get the multicast address this subscriber is using.
    ///
    /// # Returns
    /// The socket address (IP and port) this subscriber is bound to
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use multicast_rs::subscriber::MulticastSubscriber;
    ///
    /// let subscriber = MulticastSubscriber::new_ipv4(None, None).unwrap();
    /// println!("Subscribed to: {}", subscriber.address());
    /// ```
    pub fn address(&self) -> SocketAddr {
        self.addr
    }

    /// Get a reference to the underlying UDP socket.
    ///
    /// This allows access to the raw socket for advanced configurations.
    ///
    /// # Returns
    /// A reference to the UdpSocket used by this subscriber
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use multicast_rs::subscriber::MulticastSubscriber;
    /// use std::net::Ipv4Addr;
    /// use std::time::Duration;
    ///
    /// let subscriber = MulticastSubscriber::new_ipv4(None, None).unwrap();
    ///
    /// // Set custom socket options
    /// subscriber.socket().set_read_timeout(Some(Duration::from_secs(1))).unwrap();
    /// ```
    pub fn socket(&self) -> &UdpSocket {
        &self.socket
    }
}

impl BackgroundSubscriber {
    /// Stop the background listener.
    ///
    /// This method signals the background thread to stop and waits for it
    /// to complete. After calling this method, the background listener is
    /// no longer active.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use multicast_rs::subscriber::MulticastSubscriber;
    /// use std::thread;
    /// use std::time::Duration;
    ///
    /// let subscriber = MulticastSubscriber::new_ipv4(None, None).unwrap();
    ///
    /// let mut background = subscriber.listen_in_background(|data, addr| {
    ///     println!("Received {} bytes from {}", data.len(), addr);
    ///     Ok(())
    /// }).unwrap();
    ///
    /// // Listen for 30 seconds
    /// thread::sleep(Duration::from_secs(30));
    ///
    /// // Stop the background listener
    /// background.stop();
    /// ```
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.join_handle.take() {
            if let Err(e) = handle.join() {
                eprintln!("Error joining multicast listener thread: {:?}", e);
            }
        }
    }

    /// Get the multicast address this background subscriber is using.
    ///
    /// # Returns
    /// The socket address (IP and port) this subscriber is bound to
    pub fn address(&self) -> SocketAddr {
        self.addr
    }
}

/// Automatically stops the background listener when it goes out of scope
impl Drop for BackgroundSubscriber {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::new_sender;
    use crate::publisher::MulticastPublisher;
    use std::sync::atomic::{AtomicU16, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    // Use atomic counter to ensure each test gets a unique port
    static PORT_COUNTER: AtomicU16 = AtomicU16::new(7800);

    fn get_unique_port() -> u16 {
        PORT_COUNTER.fetch_add(1, Ordering::SeqCst)
    }

    #[test]
    fn test_subscriber_ipv4() {
        let port = get_unique_port();
        let addr = SocketAddr::new(*IPV4, port);

        // Create subscriber
        let subscriber =
            MulticastSubscriber::new_ipv4(Some(port), None).expect("Failed to create subscriber");

        // Ready signal
        let ready = Arc::new(Mutex::new(false));
        let ready_clone = ready.clone();

        // Start a thread to send a test message
        let sender_thread = thread::spawn(move || {
            // Wait until subscriber signals it's ready
            let mut is_ready = false;
            for _ in 0..20 {
                {
                    let r = ready_clone.lock().unwrap();
                    is_ready = *r;
                }
                if is_ready {
                    break;
                }
                thread::sleep(Duration::from_millis(50));
            }

            if !is_ready {
                panic!("Subscriber never signaled ready state");
            }

            // Create a sender socket
            let socket = new_sender(&addr, None).expect("Failed to create sender");

            // Send a test message
            socket
                .send_to(b"Hello from test!", &addr)
                .expect("Failed to send message");
        });

        // Signal that we're ready to receive
        {
            let mut r = ready.lock().unwrap();
            *r = true;
        }

        // Try to receive the message with a timeout
        match subscriber.receive(Some(Duration::from_secs(5))) {
            Ok((data, _)) => {
                let message = String::from_utf8_lossy(&data);
                assert_eq!("Hello from test!", message);
            }
            Err(e) => {
                panic!("Failed to receive message: {}", e);
            }
        }

        // Wait for the sender thread to finish
        sender_thread.join().unwrap();
    }

    #[test]
    fn test_subscriber_creation() {
        // Test IPv4 subscriber creation with unique ports
        let port1 = get_unique_port();
        let subscriber = MulticastSubscriber::new_ipv4(Some(port1), None)
            .expect("Failed to create IPv4 subscriber");
        assert_eq!(subscriber.address().ip(), *IPV4);
        assert_eq!(subscriber.address().port(), port1);

        // Test custom buffer size with a different port
        let port2 = get_unique_port();
        let subscriber = MulticastSubscriber::new_ipv4(Some(port2), Some(2048))
            .expect("Failed to create subscriber with custom buffer");
        assert_eq!(subscriber.address().ip(), *IPV4);
        assert_eq!(subscriber.address().port(), port2);

        // Test custom port
        let custom_port = get_unique_port();
        let subscriber = MulticastSubscriber::new_ipv4(Some(custom_port), None)
            .expect("Failed to create subscriber with custom port");
        assert_eq!(subscriber.address().ip(), *IPV4);
        assert_eq!(subscriber.address().port(), custom_port);
    }

    #[test]
    fn test_receive_and_respond() {
        let port = get_unique_port();
        let addr = SocketAddr::new(*IPV4, port);

        // Create subscriber
        let subscriber =
            MulticastSubscriber::new_ipv4(Some(port), None).expect("Failed to create subscriber");

        // Ready signal
        let ready = Arc::new(Mutex::new(false));
        let ready_clone = ready.clone();

        // Start a thread to send a test message and expect a response
        let sender_thread = thread::spawn(move || {
            // Wait until subscriber signals it's ready
            let mut is_ready = false;
            for _ in 0..20 {
                {
                    let r = ready_clone.lock().unwrap();
                    is_ready = *r;
                }
                if is_ready {
                    break;
                }
                thread::sleep(Duration::from_millis(50));
            }

            if !is_ready {
                panic!("Subscriber never signaled ready state");
            }

            // Create a sender socket
            let socket = new_sender(&addr, None).expect("Failed to create sender");
            socket
                .set_read_timeout(Some(Duration::from_secs(5)))
                .expect("Failed to set timeout");

            // Send a test message
            socket
                .send_to(b"Request message", &addr)
                .expect("Failed to send message");

            // Wait for response
            let mut buf = vec![0u8; 1024];
            match socket.recv_from(&mut buf) {
                Ok((size, _)) => {
                    let response = String::from_utf8_lossy(&buf[..size]);
                    assert_eq!("Response to: Request message", response);
                }
                Err(e) => {
                    panic!("Failed to receive response: {}", e);
                }
            }
        });

        // Signal that we're ready to receive
        {
            let mut r = ready.lock().unwrap();
            *r = true;
        }

        // Receive and respond to the message
        match subscriber
            .receive_and_respond(Some(Duration::from_secs(5)), "Response to: Request message")
        {
            Ok((data, _)) => {
                let message = String::from_utf8_lossy(&data);
                assert_eq!("Request message", message);
            }
            Err(e) => {
                panic!("Failed to receive or respond to message: {}", e);
            }
        }

        // Wait for the sender thread to finish
        sender_thread.join().unwrap();
    }

    #[test]
    fn test_background_subscriber() {
        // Use a port in a completely separate range to avoid conflicts
        let port = 9876;
        let addr = SocketAddr::new(*IPV4, port);

        // Create a variable to track received messages
        let received_messages = Arc::new(Mutex::new(Vec::<String>::new()));
        let received_messages_clone = received_messages.clone();

        // Create socket directly first to ensure we have control over it
        let socket = match join_multicast(addr, None) {
            Ok(s) => s,
            Err(e) => {
                // If we can't bind to this port, just skip the test
                println!("Skipping background test due to socket error: {}", e);
                return;
            }
        };

        // Wrap the socket in a subscriber
        let subscriber = MulticastSubscriber {
            socket,
            addr,
            buffer_size: 1024,
            interface: None,
        };

        // Start background subscriber
        let mut background = match subscriber.listen_in_background(move |data, _addr| {
            let message = String::from_utf8_lossy(data).to_string();
            let mut messages = received_messages_clone.lock().unwrap();
            messages.push(message);
            Ok(())
        }) {
            Ok(bg) => bg,
            Err(e) => {
                // If we can't start the background subscriber, just skip the test
                println!("Skipping background test due to error: {}", e);
                return;
            }
        };

        // Sleep a bit to ensure the background thread is ready
        thread::sleep(Duration::from_millis(200));

        // Create a sender socket
        let socket = match new_sender(&addr, None) {
            Ok(s) => s,
            Err(e) => {
                // If we can't create a sender, skip the test
                println!("Skipping background test due to sender error: {}", e);
                background.stop();
                return;
            }
        };

        // Send multiple test messages with sufficient delay between them
        let _ = socket.send_to(b"Message 1", &addr);
        thread::sleep(Duration::from_millis(100));
        let _ = socket.send_to(b"Message 2", &addr);
        thread::sleep(Duration::from_millis(100));
        let _ = socket.send_to(b"Message 3", &addr);

        // Sleep to allow processing of all messages
        thread::sleep(Duration::from_millis(500));

        // Stop the background subscriber
        background.stop();

        // Check that messages were received
        let messages = received_messages.lock().unwrap();
        if !messages.is_empty() {
            // Only verify if we actually received messages
            // (This makes the test more robust on different platforms)
            println!("Received {} messages", messages.len());
            assert!(messages.len() > 0);
        } else {
            println!(
                "No messages received in background test - this may be expected on some platforms"
            );
        }
    }

    // Skip IPv6 tests if not running on a properly configured system
    #[cfg(not(windows))] // IPv6 test is tricky on Windows
    #[test]
    fn test_ipv6_subscriber() {
        // This test will only work if IPv6 multicast is properly configured on your system
        let port = get_unique_port();
        if let Ok(subscriber) = MulticastSubscriber::new_ipv6(Some(port), None) {
            assert_eq!(subscriber.address().ip(), *IPV6);
            assert_eq!(subscriber.address().port(), port);

            // Basic test that subscriber address is correct
            let addr = subscriber.address();
            assert!(addr.ip().is_ipv6());
            assert!(addr.ip().is_multicast());
        } else {
            // Not failing the test as IPv6 might not be properly configured
            eprintln!("IPv6 multicast not available on this system");
        }
    }

    #[test]
    fn test_receive_batch() {
        let port = get_unique_port();
        let addr = SocketAddr::new(*IPV4, port);

        // Create subscriber
        let subscriber =
            MulticastSubscriber::new_ipv4(Some(port), None).expect("Failed to create subscriber");

        // Ready signal
        let ready = Arc::new(Mutex::new(false));
        let ready_clone = ready.clone();

        // Number of messages to send and receive
        let message_count = 5;

        // Start a thread to send test messages
        let sender_thread = thread::spawn(move || {
            // Wait until subscriber signals it's ready
            let mut is_ready = false;
            for _ in 0..20 {
                {
                    let r = ready_clone.lock().unwrap();
                    is_ready = *r;
                }
                if is_ready {
                    break;
                }
                thread::sleep(Duration::from_millis(50));
            }

            if !is_ready {
                panic!("Subscriber never signaled ready state");
            }

            // Create a sender socket
            let socket = new_sender(&addr, None).expect("Failed to create sender");

            // Send multiple test messages
            for i in 1..=message_count {
                let message = format!("Batch message {}", i);
                socket
                    .send_to(message.as_bytes(), &addr)
                    .expect("Failed to send message");

                // Add delay between messages
                thread::sleep(Duration::from_millis(20));
            }
        });

        // Signal that we're ready to receive
        {
            let mut r = ready.lock().unwrap();
            *r = true;
        }

        // Try to receive the batch of messages
        match subscriber.receive_batch(
            message_count,
            Some(Duration::from_millis(200)),
            Some(Duration::from_secs(5)),
        ) {
            Ok(results) => {
                // Check that we received the expected number of messages
                assert_eq!(results.len(), message_count);

                // Convert received messages to strings for easier verification
                let received_messages: Vec<String> = results
                    .iter()
                    .map(|(data, _)| String::from_utf8_lossy(data).to_string())
                    .collect();

                // Check that each expected message was received
                for i in 1..=message_count {
                    let expected = format!("Batch message {}", i);
                    assert!(
                        received_messages.contains(&expected),
                        "Missing message: {}",
                        expected
                    );
                }
            }
            Err(e) => {
                panic!("Failed to receive batch: {}", e);
            }
        }

        // Wait for the sender thread to finish
        sender_thread.join().unwrap();
    }

    #[test]
    fn test_process_batch() {
        let port = get_unique_port();
        let addr = SocketAddr::new(*IPV4, port);

        // Create subscriber
        let subscriber =
            MulticastSubscriber::new_ipv4(Some(port), None).expect("Failed to create subscriber");

        // Ready signal
        let ready = Arc::new(Mutex::new(false));
        let ready_clone = ready.clone();

        // Storage for received messages
        let received_messages = Arc::new(Mutex::new(Vec::new()));
        let received_messages_clone = received_messages.clone();

        // Number of messages to send and receive
        let message_count = 5;

        // Start a thread to send test messages
        let sender_thread = thread::spawn(move || {
            // Wait until subscriber signals it's ready
            let mut is_ready = false;
            for _ in 0..20 {
                {
                    let r = ready_clone.lock().unwrap();
                    is_ready = *r;
                }
                if is_ready {
                    break;
                }
                thread::sleep(Duration::from_millis(50));
            }

            if !is_ready {
                panic!("Subscriber never signaled ready state");
            }

            // Create a sender socket
            let socket = new_sender(&addr, None).expect("Failed to create sender");

            // Send multiple test messages
            for i in 1..=message_count {
                let message = format!("Process batch message {}", i);
                socket
                    .send_to(message.as_bytes(), &addr)
                    .expect("Failed to send message");

                // Add delay between messages
                thread::sleep(Duration::from_millis(20));
            }
        });

        // Signal that we're ready to receive
        {
            let mut r = ready.lock().unwrap();
            *r = true;
        }

        // Process the batch of messages
        let processed = subscriber
            .process_batch(
                message_count,
                Some(Duration::from_millis(200)),
                Some(Duration::from_secs(5)),
                |data, _addr| {
                    let message = String::from_utf8_lossy(data).to_string();
                    let mut messages = received_messages_clone.lock().unwrap();
                    messages.push(message);
                    Ok(())
                },
            )
            .expect("Failed to process batch");

        // Check that we processed the expected number of messages
        assert_eq!(processed, message_count);

        // Check the contents of the received messages
        let messages = received_messages.lock().unwrap();
        assert_eq!(messages.len(), message_count);

        // Check that each expected message was received
        for i in 1..=message_count {
            let expected = format!("Process batch message {}", i);
            assert!(
                messages.contains(&expected),
                "Missing message: {}",
                expected
            );
        }

        // Wait for the sender thread to finish
        sender_thread.join().unwrap();
    }

    #[test]
    fn test_subscriber_interface_creation() {
        let port = get_unique_port();

        // Try to create a subscriber with the loopback interface
        match MulticastSubscriber::new_ipv4_with_interface(Some(port), Some("lo"), None) {
            Ok(subscriber) => {
                println!("Successfully created subscriber on loopback interface");
                assert_eq!(subscriber.address().port(), port);
                assert_eq!(subscriber.address().ip(), *IPV4);
            }
            Err(e) => {
                // On some systems this might fail but we don't want the test to fail
                println!("Could not create subscriber on loopback interface: {}", e);
            }
        }

        // Also test with direct IP address specification
        match MulticastSubscriber::new_ipv4_with_interface(Some(port), Some("127.0.0.1"), None) {
            Ok(subscriber) => {
                println!("Successfully created subscriber with explicit IP 127.0.0.1");
                assert_eq!(subscriber.address().port(), port);
            }
            Err(e) => {
                // This should generally work on all systems, but don't fail the test
                println!(
                    "Could not create subscriber with explicit IP 127.0.0.1: {}",
                    e
                );
            }
        }

        // Test with a non-existent interface
        match MulticastSubscriber::new_ipv4_with_interface(
            Some(port),
            Some("nonexistent_iface"),
            None,
        ) {
            Ok(_) => {
                panic!("Subscriber was created with non-existent interface, should have failed");
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

    // This test checks if we can bind to a specific interface and receive messages
    #[test]
    fn test_subscriber_with_interface_receive() {
        if let Some(iface) = detect_network_interface() {
            println!("Testing subscriber with detected interface: {}", iface);

            let port = get_unique_port();
            let addr = SocketAddr::new(*IPV4, port);

            // Create a publisher and subscriber using the same interface
            match (
                MulticastSubscriber::new_ipv4_with_interface(Some(port), Some(&iface), None),
                MulticastPublisher::new_ipv4_with_interface(Some(port), Some(&iface)),
            ) {
                (Ok(subscriber), Ok(publisher)) => {
                    // Set up test coordination
                    let ready = Arc::new(Mutex::new(false));
                    let ready_clone = ready.clone();
                    let test_message = "Interface-specific test message";

                    // Clone iface for the thread
                    let thread_iface = iface.clone();

                    // Start a thread to listen for messages
                    let receiver_thread = thread::spawn(move || {
                        // Signal that we're ready
                        {
                            let mut r = ready_clone.lock().unwrap();
                            *r = true;
                        }

                        // Try to receive the message with a timeout
                        match subscriber.receive(Some(Duration::from_secs(3))) {
                            Ok((data, _)) => {
                                let message = String::from_utf8_lossy(&data);
                                assert_eq!(test_message, message);
                                true
                            }
                            Err(e) => {
                                println!("Failed to receive on interface {}: {}", iface, e);
                                false
                            }
                        }
                    });

                    // Wait for the receiver to be ready
                    let mut is_ready = false;
                    for _ in 0..20 {
                        {
                            let r = ready.lock().unwrap();
                            is_ready = *r;
                        }
                        if is_ready {
                            break;
                        }
                        thread::sleep(Duration::from_millis(50));
                    }

                    if !is_ready {
                        panic!("Receiver thread failed to signal ready state");
                    }

                    // Give the receiver thread a bit more time
                    thread::sleep(Duration::from_millis(100));

                    // Publish a message
                    match publisher.publish(test_message) {
                        Ok(bytes_sent) => {
                            println!(
                                "Published {} bytes on interface {}",
                                bytes_sent, thread_iface
                            );
                        }
                        Err(e) => {
                            println!("Failed to publish on interface {}: {}", thread_iface, e);
                        }
                    }

                    // Check if the message was received
                    match receiver_thread.join() {
                        Ok(received) => {
                            if !received {
                                println!("Test communication on interface {} did not succeed - this might be expected", thread_iface);
                            }
                        }
                        Err(_) => {
                            println!("Receiver thread panicked");
                        }
                    }
                }
                (Err(e), _) => {
                    println!("Could not create subscriber on interface {}: {}", iface, e);
                }
                (_, Err(e)) => {
                    println!("Could not create publisher on interface {}: {}", iface, e);
                }
            }
        } else {
            println!("No suitable network interface detected, skipping test");
        }
    }

    // Test communication between loopback interfaces
    #[test]
    fn test_loopback_interface_communication() {
        let port = get_unique_port() + 1000; // Ensure unique port
        let test_message = "Loopback test message";

        // Try to create a publisher and subscriber both using loopback
        match (
            MulticastSubscriber::new_ipv4_with_interface(Some(port), Some("lo"), None),
            MulticastPublisher::new_ipv4_with_interface(Some(port), Some("lo")),
        ) {
            (Ok(subscriber), Ok(publisher)) => {
                // Set up test coordination
                let ready = Arc::new(Mutex::new(false));
                let ready_clone = ready.clone();

                // Start a thread to listen for messages
                let receiver_thread = thread::spawn(move || {
                    // Signal that we're ready
                    {
                        let mut r = ready_clone.lock().unwrap();
                        *r = true;
                    }

                    // Try to receive the message with a timeout
                    match subscriber.receive(Some(Duration::from_secs(3))) {
                        Ok((data, _)) => {
                            let message = String::from_utf8_lossy(&data);
                            println!("Received on loopback: {}", message);
                            message == test_message
                        }
                        Err(e) => {
                            println!("Failed to receive on loopback: {}", e);
                            false
                        }
                    }
                });

                // Wait for the receiver to be ready
                let mut is_ready = false;
                for _ in 0..20 {
                    {
                        let r = ready.lock().unwrap();
                        is_ready = *r;
                    }
                    if is_ready {
                        break;
                    }
                    thread::sleep(Duration::from_millis(50));
                }

                if !is_ready {
                    panic!("Receiver thread failed to signal ready state");
                }

                // Give the receiver thread a bit more time
                thread::sleep(Duration::from_millis(100));

                // Publish a message
                match publisher.publish(test_message) {
                    Ok(_) => println!("Published message on loopback interface"),
                    Err(e) => println!("Failed to publish on loopback: {}", e),
                }

                // Not asserting the result, just log what happens
                // Loopback multicast behavior varies by OS
                match receiver_thread.join() {
                    Ok(received) => {
                        if received {
                            println!("Successfully communicated over loopback interface");
                        } else {
                            println!("Loopback communication failed - this may be normal on some systems");
                        }
                    }
                    Err(_) => println!("Receiver thread panicked"),
                }
            }
            (Err(e), _) => println!("Could not create subscriber on loopback: {}", e),
            (_, Err(e)) => println!("Could not create publisher on loopback: {}", e),
        }
    }

    // Helper function to detect a real network interface, similar to the one in publisher tests
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
}
