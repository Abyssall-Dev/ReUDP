# ReUDP

[![Crate](https://img.shields.io/crates/v/reudp.svg?label=crate)](https://crates.io/crates/reudp)
[![Docs](https://docs.rs/reudp/badge.svg)](https://docs.rs/reudp/0.0.1/repath/)
[![Rust](https://github.com/Abyssall-Dev/ReUDP/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/Abyssall-Dev/ReUDP/actions/workflows/rust.yml)

ReUDP is a reliability layer on top of unreliable UDP. It provides packet acknowledgment, heartbeats, and supports both client and server modes, ensuring reliable communication over UDP.

ReUDP was developed for [Respark](https://playrespark.com/), an upcoming open world MMO shooter. Respark combines intense combat, strategic gameplay, and a vast, dynamic world to explore. Join our community on [Discord](https://discord.gg/8qzSGyekVJ) to stay updated with the latest news and development progress.

## Description

ReUDP was developed to address the need for reliable communication in multiplayer video games without the use of slow TCP. Given the nature of UDP, packet loss, and out-of-order delivery are common issues. ReUDP adds a reliability layer over UDP, ensuring that critical game data is delivered correctly and in order.

### Why It's Reliable

ReUDP's reliability comes from its combination of packet acknowledgment, heartbeats to detect lost connections, and support for both client and server modes. By acknowledging received packets and resending unacknowledged ones, ReUDP ensures that all critical data reaches its destination.

## Features

- **Packet Acknowledgment**: Ensures reliable data delivery.
- **Heartbeat Mechanism**: Detects and handles lost connections.
- **Client and Server Modes**: Supports both client-server communication patterns.
- **Concurrent Handling**: Utilizes multiple threads for efficient processing.

## Usage

### Adding ReUDP to Your Project

Add ReUDP to your `Cargo.toml`:

```toml
[dependencies]
reudp = "0.0.1"
```

Then use it in your project:

```rust
use reudp::{ReUDP, Mode, ReUDPError};
use std::time::Duration;
use std::net::SocketAddr;

fn main() -> Result<(), ReUDPError> {
    // Create a new server instance
    let mut server = ReUDP::new("127.0.0.1:8080", Mode::Server, Duration::from_secs(1), 1024)?;

    // Parse the client address and handle potential errors
    let server_addr: SocketAddr = "127.0.0.1:8080".parse().map_err(|e| ReUDPError::IoError(std::io::Error::new(std::io::ErrorKind::InvalidInput, e)))?;
    
    // Create a new client instance
    let mut client = ReUDP::new("127.0.0.1:8081", Mode::Client(server_addr), Duration::from_secs(1), 1024)?;

    // Client sends a message to the server
    client.send(b"Hello, server!".to_vec(), true)?;

    loop {
        // Server receives a message
        if let Some((_addr, data)) = server.recv()? {
            println!("Server received: {:?}", String::from_utf8(data).unwrap());
            // Server sends a response back to the client
            server.send(b"Hello, client!".to_vec(), true)?;
        }
        
        // Client receives a response from the server
        if let Some((_addr, data)) = client.recv()? {
            println!("Client received: {:?}", String::from_utf8(data).unwrap());
            break;
        }
    }

    Ok(())
}
```

### Packet Loss vs Retransmissions

ReUDP ensures reliable data delivery by retransmitting lost packets and acknowledging received ones. The heartbeat mechanism helps detect and handle lost connections, making it suitable for real-time games and other latency-sensitive applications.

### License

ReUDP is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
