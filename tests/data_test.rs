use reudp::{ReUDP, Mode, ReUDPError};
use std::thread;
use std::time::Duration;
use std::sync::{Arc, Mutex};

fn run_server(server_addr: &str, received_data: Arc<Mutex<Option<Vec<u8>>>>) -> Result<(), ReUDPError> {
    let mut reudp = ReUDP::new(server_addr, Mode::Server, Duration::from_secs(1), 1024)?;

    for _ in 0..10 { // Run for a limited number of iterations
        match reudp.recv() {
            Ok(Some((addr, data))) => {
                println!("Server received from {}: {:?}", addr, String::from_utf8(data.clone()));
                *received_data.lock().unwrap() = Some(data.clone());
                reudp.send(b"Hello from server!".to_vec(), true)?;
            },
            Ok(None) => (),
            Err(ReUDPError::ConnectionLost) => {
                println!("Server: Connection lost.");
                return Err(ReUDPError::ConnectionLost);
            },
            Err(ReUDPError::NoResponseFromServer) => {
                println!("Server: No response from server.");
                return Err(ReUDPError::NoResponseFromServer);
            },
            Err(e) => return Err(e.into()),
        }

        if let Some(ping) = reudp.get_current_ping() {
            println!("Server current ping: {} ms", ping.as_millis());
        }

        thread::sleep(Duration::from_millis(100)); // Sleep to simulate periodic checking
    }
    Ok(())
}

fn run_client(client_addr: &str, server_addr: &str, data_to_send: Vec<u8>, received_data: Arc<Mutex<Option<Vec<u8>>>>) -> Result<(), ReUDPError> {
    let server_addr = server_addr.parse().unwrap();
    let mut reudp = ReUDP::new(client_addr, Mode::Client(server_addr), Duration::from_secs(1), 1024)?;

    for _ in 0..10 { // Run for a limited number of iterations
        reudp.send(data_to_send.clone(), true)?;

        match reudp.recv() {
            Ok(Some((addr, data))) => {
                println!("Client received from {}: {:?}", addr, String::from_utf8(data.clone()));
                *received_data.lock().unwrap() = Some(data.clone());
            },
            Ok(None) => (),
            Err(ReUDPError::ConnectionLost) => {
                println!("Client: Connection lost.");
                return Err(ReUDPError::ConnectionLost);
            },
            Err(ReUDPError::NoResponseFromServer) => {
                println!("Client: No response from server.");
                return Err(ReUDPError::NoResponseFromServer);
            },
            Err(e) => return Err(e.into()),
        }

        if let Some(ping) = reudp.get_current_ping() {
            println!("Client current ping: {} ms", ping.as_millis());
        }

        thread::sleep(Duration::from_millis(100)); // Sleep to simulate periodic sending and checking
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn test_reudp_communication() {
        let server_addr = "127.0.0.1:8080";
        let client_addr = "127.0.0.1:8081";
        let data_to_send = b"Test message from client".to_vec();
        let server_received_data = Arc::new(Mutex::new(None));
        let client_received_data = Arc::new(Mutex::new(None));
        let server_running = Arc::new(AtomicBool::new(true));
        let client_running = Arc::new(AtomicBool::new(true));

        // Clone the Arc pointers for passing to the threads
        let server_received_data_clone = Arc::clone(&server_received_data);
        let client_received_data_clone = Arc::clone(&client_received_data);
        let server_running_clone = Arc::clone(&server_running);
        let client_running_clone = Arc::clone(&client_running);

        // Start the server in a separate thread
        let server_thread = thread::spawn(move || {
            let result = run_server(server_addr, server_received_data_clone);
            server_running_clone.store(false, Ordering::SeqCst);
            result.unwrap();
        });

        // Start the client in a separate thread
        let data_to_send_clone = data_to_send.clone();
        let client_thread = thread::spawn(move || {
            let result = run_client(client_addr, server_addr, data_to_send_clone, client_received_data_clone);
            client_running_clone.store(false, Ordering::SeqCst);
            result.unwrap();
        });

        // Wait for both threads to finish
        while server_running.load(Ordering::SeqCst) || client_running.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(100));
        }

        server_thread.join().unwrap();
        client_thread.join().unwrap();

        // Verify that the server received the correct data
        let server_data = server_received_data.lock().unwrap();
        assert_eq!(*server_data, Some(data_to_send));

        // Verify that the client received the correct response
        let client_data = client_received_data.lock().unwrap();
        assert_eq!(*client_data, Some(b"Hello from server!".to_vec()));
    }
}
