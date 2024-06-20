use std::collections::{HashMap, HashSet};
use std::net::{UdpSocket, SocketAddr};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::error::ReUDPError;
use crate::message::{Message, MessageType};
use crate::mode::Mode;

/// ReUDP provides a reliable layer over UDP, ensuring reliable message delivery
/// and supporting client-server communication patterns.
pub struct ReUDP {
    /// Buffer for received messages that are out of sequence
    pub recv_buffer: HashMap<u64, Vec<u8>>,
    /// Sequence number for the next message to send
    pub send_sequence: u64,
    /// Sequence number for the next message to receive
    pub recv_sequence: u64,
    /// Unacknowledged packets waiting for acknowledgment
    pub unacked_packets: HashMap<u64, Vec<u8>>,
    /// Operating mode (Client or Server)
    pub mode: Mode,
    /// List of clients (for server mode)
    pub clients: HashSet<SocketAddr>,
    /// Timestamp of the last heartbeat sent
    pub last_heartbeat_time: Instant,
    /// Interval between heartbeats
    pub heartbeat_interval: Duration,
    /// Timestamp of the last heartbeat response received
    pub last_heartbeat_response_time: Option<Instant>,
    /// Timestamp of the last ping sent
    pub last_ping_time: Option<Instant>,
    /// Current ping duration
    pub current_ping: Option<Duration>,
    /// UDP socket for communication
    socket: Arc<UdpSocket>,
    /// Buffer size for received messages
    buffer_size: usize,
    /// Flag indicating whether the ReUDP instance is running
    running: Arc<Mutex<bool>>,
}

impl ReUDP {
    /// Creates a new ReUDP instance.
    ///
    /// # Arguments
    ///
    /// * `local_addr` - Local address to bind the UDP socket.
    /// * `mode` - Operating mode (Client or Server).
    /// * `heartbeat_interval` - Interval between heartbeats.
    /// * `buffer_size` - Size of the buffer for received messages.
    ///
    /// # Returns
    ///
    /// * `Result<Self, std::io::Error>` - The created ReUDP instance or an error.
    pub fn new(
        local_addr: &str,
        mode: Mode,
        heartbeat_interval: Duration,
        buffer_size: usize,
    ) -> Result<Self, std::io::Error> {
        let socket = UdpSocket::bind(local_addr)?;
        socket.set_nonblocking(true)?;
        let reudp = Self {
            recv_buffer: HashMap::new(),
            send_sequence: 0,
            recv_sequence: 0,
            unacked_packets: HashMap::new(),
            mode,
            clients: HashSet::new(),
            last_heartbeat_time: Instant::now(),
            heartbeat_interval,
            last_heartbeat_response_time: None,
            last_ping_time: None,
            current_ping: None,
            socket: Arc::new(socket),
            buffer_size,
            running: Arc::new(Mutex::new(true)),
        };

        reudp.start_heartbeat();
        Ok(reudp)
    }

    /// Starts the heartbeat mechanism in a separate thread.
    fn start_heartbeat(&self) {
        let socket = Arc::clone(&self.socket);
        let heartbeat_interval = self.heartbeat_interval;
        let mode = self.mode.clone();
        let clients = self.clients.clone();
        let unacked_packets = Arc::new(Mutex::new(self.unacked_packets.clone()));
        let last_heartbeat_time = Arc::new(Mutex::new(self.last_heartbeat_time));
        let last_heartbeat_response_time = Arc::new(Mutex::new(self.last_heartbeat_response_time));
        let last_ping_time = Arc::new(Mutex::new(self.last_ping_time));
        let current_ping = Arc::new(Mutex::new(self.current_ping));
        let running = Arc::clone(&self.running);

        thread::spawn(move || {
            while *running.lock().unwrap() {
                let packets = unacked_packets.lock().unwrap();
                let mut last_heartbeat = last_heartbeat_time.lock().unwrap();
                let last_response_time = last_heartbeat_response_time.lock().unwrap();
                let mut ping_time = last_ping_time.lock().unwrap();
                let mut ping = current_ping.lock().unwrap();

                // Resend unacknowledged packets
                for packet in packets.values() {
                    match mode {
                        Mode::Client(ref remote_addr) => {
                            socket.send_to(packet, remote_addr).unwrap();
                        }
                        Mode::Server => {
                            for client in &clients {
                                socket.send_to(packet, client).unwrap();
                            }
                        }
                    }
                }

                // Send heartbeat
                if Instant::now().duration_since(*last_heartbeat) > heartbeat_interval {
                    let heartbeat_message = Message::new(0, MessageType::Heartbeat, vec![]);
                    let serialized_heartbeat = heartbeat_message.to_bytes();
                    match mode {
                        Mode::Client(ref remote_addr) => {
                            socket.send_to(&serialized_heartbeat, remote_addr).unwrap();
                        }
                        Mode::Server => {
                            for client in &clients {
                                socket.send_to(&serialized_heartbeat, client).unwrap();
                            }
                        }
                    }
                    *ping_time = Some(Instant::now());
                    *last_heartbeat = Instant::now();
                }

                // Check heartbeat response
                if let Some(response_time) = *last_response_time {
                    if Instant::now().duration_since(response_time) > heartbeat_interval * 2 {
                        println!("Connection lost");
                        // Connection lost
                        *running.lock().unwrap() = false;
                    } else if let Some(sent_time) = *ping_time {
                        *ping = Some(Instant::now().duration_since(sent_time));
                    }
                } else if Instant::now().duration_since(*last_heartbeat) > heartbeat_interval * 2 {
                    println!("No response from server");
                    // No response from server
                    *running.lock().unwrap() = false;
                }

                thread::sleep(Duration::from_secs(1));
            }
        });
    }

    /// Sends a message with optional acknowledgment requirement.
    ///
    /// # Arguments
    ///
    /// * `data` - The data to be sent.
    /// * `require_ack` - Whether the message requires an acknowledgment.
    ///
    /// # Returns
    ///
    /// * `Result<(), ReUDPError>` - Ok if successful, or an error.
    pub fn send(&mut self, data: Vec<u8>, require_ack: bool) -> Result<(), ReUDPError> {
        let message = Message::new(self.send_sequence, MessageType::Data, data.clone());
        let serialized = message.to_bytes();

        match self.mode {
            Mode::Client(ref remote_addr) => {
                self.socket.send_to(&serialized, remote_addr)?;
            }
            Mode::Server => {
                for client in &self.clients {
                    self.socket.send_to(&serialized, client)?;
                }
            }
        }

        if require_ack {
            self.unacked_packets.insert(self.send_sequence, serialized);
        }
        self.send_sequence += 1;
        Ok(())
    }

    /// Receives a message, handling acknowledgment and heartbeats.
    ///
    /// # Returns
    ///
    /// * `Result<Option<(SocketAddr, Vec<u8>)>, ReUDPError>` - The address and data of the received message, or an error.
    pub fn recv(&mut self) -> Result<Option<(SocketAddr, Vec<u8>)>, ReUDPError> {
        let mut buf = vec![0; self.buffer_size];
        match self.socket.recv_from(&mut buf) {
            Ok((len, addr)) => {
                let message = Message::from_bytes(&buf[..len]);

                if let Mode::Server = self.mode {
                    self.clients.insert(addr);
                }

                match message.message_type {
                    MessageType::Data => {
                        let ack = Message::new(message.sequence, MessageType::Ack, vec![]);
                        let serialized_ack = ack.to_bytes();
                        self.socket.send_to(&serialized_ack, &addr)?;

                        if message.sequence == self.recv_sequence {
                            self.recv_sequence += 1;
                            Ok(Some((addr, message.payload)))
                        } else {
                            self.recv_buffer.insert(message.sequence, message.payload);
                            Ok(None)
                        }
                    }
                    MessageType::Ack => {
                        self.unacked_packets.remove(&message.sequence);
                        Ok(None)
                    }
                    MessageType::Heartbeat => {
                        let response = Message::new(0, MessageType::Heartbeat, vec![]);
                        let serialized_response = response.to_bytes();
                        self.socket.send_to(&serialized_response, &addr)?;

                        self.last_heartbeat_response_time = Some(Instant::now());
                        self.current_ping = self.last_heartbeat_response_time.map(|resp_time| resp_time.elapsed());

                        Ok(None)
                    }
                    MessageType::Unknown(t) => {
                        eprintln!("Received unknown message type: {}", t);
                        Ok(None)
                    }
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(ReUDPError::IoError(e)),
        }
    }

    /// Returns the current ping duration.
    ///
    /// # Returns
    ///
    /// * `Option<Duration>` - The current ping duration, if available.
    pub fn get_current_ping(&self) -> Option<Duration> {
        self.current_ping
    }

    /// Returns a reference to the underlying UDP socket.
    ///
    /// # Returns
    ///
    /// * `&UdpSocket` - Reference to the UDP socket.
    pub fn socket(&self) -> &UdpSocket {
        &self.socket
    }
}
