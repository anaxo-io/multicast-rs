#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

#[macro_use]
extern crate lazy_static;
extern crate socket2;

// Add the modules
pub mod publisher;
pub mod subscriber;

use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};
use std::sync::{Arc, Barrier};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use socket2::{Domain, Protocol, SockAddr, Socket, Type};

pub const PORT: u16 = 7645;
lazy_static! {
    pub static ref IPV4: IpAddr = Ipv4Addr::new(224, 0, 0, 123).into();
    pub static ref IPV6: IpAddr = Ipv6Addr::new(0xFF02, 0, 0, 0, 0, 0, 0, 0x0123).into();
}

/// Create a new socket for multicast
fn new_socket(addr: &SocketAddr) -> io::Result<Socket> {
    let domain = if addr.is_ipv4() {
        Domain::IPV4
    } else {
        Domain::IPV6
    };

    let socket = Socket::new(domain, Type::DGRAM, Some(Protocol::UDP))?;

    // we're going to use read timeouts so that we don't hang waiting for packets
    socket.set_read_timeout(Some(Duration::from_millis(100)))?;

    Ok(socket)
}

/// Get interface address from interface name or return the provided fallback address
fn get_interface_addr(interface: Option<&str>, fallback: &IpAddr) -> io::Result<IpAddr> {
    match interface {
        None => Ok(*fallback),
        Some(iface) => {
            // Try to find the interface by name and get its address
            if let Some(iface_addr) = get_interface_ip(iface, fallback.is_ipv4())? {
                Ok(iface_addr)
            } else {
                // If we couldn't find it, return the fallback
                Ok(*fallback)
            }
        }
    }
}

/// Get IP address for a named interface
fn get_interface_ip(interface_name: &str, want_ipv4: bool) -> io::Result<Option<IpAddr>> {
    // For now, we'll use a simple approach to get interface IPs
    // A more robust implementation could use platform-specific methods
    
    // First, check if interface_name is directly an IP address
    if let Ok(addr) = interface_name.parse::<IpAddr>() {
        return Ok(Some(addr));
    }

    #[cfg(target_family = "unix")]
    {
        use std::process::Command;
        
        // This is a simplistic approach - we run 'ip addr' command and parse it
        // In production code, you'd want to use a proper network interface library
        let output = Command::new("ip")
            .args(&["addr", "show", "dev", interface_name])
            .output()?;
            
        if !output.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound, 
                format!("Interface {} not found", interface_name)
            ));
        }
        
        let output_str = String::from_utf8_lossy(&output.stdout);
        
        // Extract IP addresses from the output
        let mut ipv4_addr: Option<IpAddr> = None;
        let mut ipv6_addr: Option<IpAddr> = None;
        
        for line in output_str.lines() {
            let line = line.trim();
            
            // Look for IPv4 addresses
            if line.starts_with("inet ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let addr_with_prefix = parts[1];
                    if let Some(ip_str) = addr_with_prefix.split('/').next() {
                        if let Ok(addr) = ip_str.parse::<Ipv4Addr>() {
                            ipv4_addr = Some(IpAddr::V4(addr));
                        }
                    }
                }
            }
            
            // Look for IPv6 addresses
            if line.starts_with("inet6 ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let addr_with_prefix = parts[1];
                    if let Some(ip_str) = addr_with_prefix.split('/').next() {
                        if let Ok(addr) = ip_str.parse::<Ipv6Addr>() {
                            // Skip link-local addresses unless they're explicitly requested
                            if !addr.is_unicast_link_local() || interface_name.contains("link-local") {
                                ipv6_addr = Some(IpAddr::V6(addr));
                            }
                        }
                    }
                }
            }
        }
        
        // Return the appropriate IP address type based on what was requested
        return Ok(if want_ipv4 { ipv4_addr } else { ipv6_addr.or(ipv4_addr) });
    }
    
    #[cfg(not(target_family = "unix"))]
    {
        // On non-Unix platforms, we'll return None for now
        // A robust implementation would use platform-specific APIs
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Interface lookup not implemented on this platform"
        ));
    }
}

/// On Windows, unlike all Unix variants, it is improper to bind to the multicast address
///
/// see https://msdn.microsoft.com/en-us/library/windows/desktop/ms737550(v=vs.85).aspx
#[cfg(windows)]
fn bind_multicast(socket: &Socket, addr: &SocketAddr) -> io::Result<()> {
    let addr = match *addr {
        SocketAddr::V4(addr) => SocketAddr::new(Ipv4Addr::new(0, 0, 0, 0).into(), addr.port()),
        SocketAddr::V6(addr) => {
            SocketAddr::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0).into(), addr.port())
        }
    };
    socket.bind(&socket2::SockAddr::from(addr))
}

/// On unixes we bind to the multicast address, which causes multicast packets to be filtered
#[cfg(unix)]
fn bind_multicast(socket: &Socket, addr: &SocketAddr) -> io::Result<()> {
    socket.bind(&socket2::SockAddr::from(*addr))
}

fn join_multicast(addr: SocketAddr, interface: Option<&str>) -> io::Result<UdpSocket> {
    let ip_addr = addr.ip();

    let socket = new_socket(&addr)?;
    
    // Get the interface address
    let iface_addr = get_interface_addr(interface, &IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)))?;

    // depending on the IP protocol we have slightly different work
    match ip_addr {
        IpAddr::V4(ref mdns_v4) => {
            // Get the interface IPv4 address if specified
            let iface_v4 = match iface_addr {
                IpAddr::V4(v4) => v4,
                _ => Ipv4Addr::new(0, 0, 0, 0), // Default to all interfaces
            };
            
            // join to the multicast address, with the specified interface
            socket.join_multicast_v4(mdns_v4, &iface_v4)?;
        }
        IpAddr::V6(ref mdns_v6) => {
            // For IPv6, figure out interface index if possible
            let iface_idx = match interface {
                Some(iface) => {
                    #[cfg(target_family = "unix")]
                    {
                        // Try to get interface index
                        let output = std::process::Command::new("ip")
                            .args(&["link", "show", "dev", iface])
                            .output()?;
                        
                        if output.status.success() {
                            // Very naive parsing - in production code use proper APIs
                            let output_str = String::from_utf8_lossy(&output.stdout);
                            if let Some(first_line) = output_str.lines().next() {
                                if let Some(idx_str) = first_line.split(':').next() {
                                    if let Ok(idx) = idx_str.trim().parse::<u32>() {
                                        idx
                                    } else {
                                        0 // Default to all interfaces
                                    }
                                } else {
                                    0
                                }
                            } else {
                                0
                            }
                        } else {
                            0 // Default to all interfaces
                        }
                    }
                    
                    #[cfg(not(target_family = "unix"))]
                    {
                        0 // Default to all interfaces on non-Unix
                    }
                }
                None => 0, // Default to all interfaces
            };
            
            // join to the multicast address, with the specified interface
            socket.join_multicast_v6(mdns_v6, iface_idx)?;
            socket.set_only_v6(true)?;
        }
    };

    // bind us to the socket address.
    bind_multicast(&socket, &addr)?;

    // convert to standard sockets
    Ok(socket.into())
}

fn new_sender(addr: &SocketAddr, interface: Option<&str>) -> io::Result<UdpSocket> {
    let socket = new_socket(addr)?;

    if addr.is_ipv4() {
        // Get interface address if specified
        let iface_v4 = match get_interface_addr(interface, &IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)))? {
            IpAddr::V4(v4) => v4,
            _ => Ipv4Addr::new(0, 0, 0, 0), // Default to all interfaces
        };
        
        socket.set_multicast_if_v4(&iface_v4)?;

        socket.bind(&SockAddr::from(SocketAddr::new(
            iface_v4.into(),
            0,
        )))?;
    } else {
        // For IPv6 interfaces
        let iface_idx = match interface {
            Some(iface) => {
                #[cfg(target_family = "unix")]
                {
                    // Try to get interface index - same naive approach as above
                    let output = std::process::Command::new("ip")
                        .args(&["link", "show", "dev", iface])
                        .output()?;
                    
                    if output.status.success() {
                        let output_str = String::from_utf8_lossy(&output.stdout);
                        if let Some(first_line) = output_str.lines().next() {
                            if let Some(idx_str) = first_line.split(':').next() {
                                if let Ok(idx) = idx_str.trim().parse::<u32>() {
                                    idx
                                } else {
                                    0 // Default
                                }
                            } else {
                                0
                            }
                        } else {
                            0
                        }
                    } else {
                        0
                    }
                }
                
                #[cfg(not(target_family = "unix"))]
                {
                    0 // Default on non-Unix
                }
            }
            None => 0,
        };
        
        socket.set_multicast_if_v6(iface_idx)?;

        socket.bind(&SockAddr::from(SocketAddr::new(
            Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0).into(),
            0,
        )))?;
    }

    // convert to standard sockets...
    Ok(socket.into())
}

