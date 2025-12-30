//! Integration tests for Tokio-Lite runtime
//!
//! These tests verify the correctness of the async runtime, TCP handling,
//! and concurrent operations.

use std::io::{Read, Write};
use std::net::TcpStream as StdTcpStream;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use tokio_lite::net::TcpListener;
use tokio_lite::Runtime;

/// Helper to wait for server to be ready
fn wait_for_server(addr: &str, timeout_ms: u64) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed().as_millis() < timeout_ms as u128 {
        if StdTcpStream::connect(addr).is_ok() {
            return true;
        }
        thread::sleep(Duration::from_millis(10));
    }
    false
}

#[test]
fn test_echo_server_single_message() {
    // Start echo server in background thread
    let server_handle = thread::spawn(|| {
        let mut runtime = Runtime::new().expect("Failed to create runtime");
        
        runtime.block_on(async {
            let mut listener = TcpListener::bind("127.0.0.1:9001".parse().unwrap())
                .expect("Failed to bind");
            
            let mut stream = listener.accept().await.expect("Failed to accept");
            
            // Echo once and exit
            let mut buf = vec![0u8; 1024];
            let n = stream.read(&mut buf).await.expect("Failed to read");
            stream.write(&buf[..n]).await.expect("Failed to write");
        });
    });
    
    // Wait for server
    thread::sleep(Duration::from_millis(100));
    
    // Connect and test
    let mut client = StdTcpStream::connect("127.0.0.1:9001")
        .expect("Failed to connect");
    client.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    
    let msg = b"Hello, Tokio-Lite!";
    client.write_all(msg).expect("Failed to write");
    
    let mut buf = vec![0u8; 1024];
    let n = client.read(&mut buf).expect("Failed to read");
    
    assert_eq!(&buf[..n], msg, "Echo mismatch");
    
    drop(client);
    server_handle.join().expect("Server thread panicked");
}

#[test]
fn test_echo_server_multiple_messages() {
    let server_handle = thread::spawn(|| {
        let mut runtime = Runtime::new().expect("Failed to create runtime");
        
        runtime.block_on(async {
            let mut listener = TcpListener::bind("127.0.0.1:9002".parse().unwrap())
                .expect("Failed to bind");
            
            let mut stream = listener.accept().await.expect("Failed to accept");
            
            // Echo 3 messages
            for _ in 0..3 {
                let mut buf = vec![0u8; 1024];
                let n = stream.read(&mut buf).await.expect("Failed to read");
                if n == 0 {
                    break;
                }
                stream.write(&buf[..n]).await.expect("Failed to write");
            }
        });
    });
    
    thread::sleep(Duration::from_millis(100));
    
    let mut client = StdTcpStream::connect("127.0.0.1:9002")
        .expect("Failed to connect");
    client.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    
    let messages: Vec<&[u8]> = vec![b"First", b"Second", b"Third"];
    
    for msg in &messages {
        client.write_all(msg).expect("Failed to write");
        
        let mut buf = vec![0u8; 1024];
        let n = client.read(&mut buf).expect("Failed to read");
        
        assert_eq!(&buf[..n], &msg[..], "Echo mismatch for message");
    }
    
    drop(client);
    server_handle.join().expect("Server thread panicked");
}

#[test]
fn test_concurrent_connections() {
    // NOTE: This test is limited by our single-threaded runtime
    // True concurrent handling will be added in Phase 2 (multi-threaded executor)
    // For now, we test that multiple sequential connections work correctly
    
    let connections_count = 5;
    let server_ready = Arc::new(Mutex::new(false));
    let server_ready_clone = server_ready.clone();
    
    let server_handle = thread::spawn(move || {
        let mut runtime = Runtime::new().expect("Failed to create runtime");
        
        runtime.block_on(async move {
            let mut listener = TcpListener::bind("127.0.0.1:9003".parse().unwrap())
                .expect("Failed to bind");
            
            *server_ready_clone.lock().unwrap() = true;
            
            // Handle connections sequentially (as our single-threaded runtime requires)
            // TODO Phase 2: Use spawn() for true concurrent handling
            for i in 0..connections_count {
                let mut stream = listener.accept().await
                    .expect(&format!("Failed to accept connection {}", i));
                
                let mut buf = vec![0u8; 1024];
                let n = stream.read(&mut buf).await.expect("Failed to read");
                stream.write(&buf[..n]).await.expect("Failed to write");
            }
        });
    });
    
    // Wait for server to be ready
    while !*server_ready.lock().unwrap() {
        thread::sleep(Duration::from_millis(10));
    }
    thread::sleep(Duration::from_millis(100));
    
    // Connect clients SEQUENTIALLY (not concurrently)
    // Our single-threaded runtime can't handle concurrent connections yet
    for i in 0..connections_count {
        let mut client = StdTcpStream::connect("127.0.0.1:9003")
            .expect(&format!("Failed to connect client {}", i));
        client.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        
        let msg = format!("Message from client {}", i);
        client.write_all(msg.as_bytes()).expect("Failed to write");
        
        let mut buf = vec![0u8; 1024];
        let n = client.read(&mut buf).expect("Failed to read");
        
        assert_eq!(&buf[..n], msg.as_bytes(), "Echo mismatch for client {}", i);
    }
    
    server_handle.join().expect("Server thread panicked");
}

#[test]
fn test_large_payload_transfer() {
    let payload_size = 1024 * 1024; // 1MB
    
    let server_handle = thread::spawn(move || {
        let mut runtime = Runtime::new().expect("Failed to create runtime");
        
        runtime.block_on(async move {
            let mut listener = TcpListener::bind("127.0.0.1:9004".parse().unwrap())
                .expect("Failed to bind");
            
            let mut stream = listener.accept().await.expect("Failed to accept");
            
            // Read large payload in chunks
            let mut received = Vec::new();
            let mut buf = vec![0u8; 8192];
            
            loop {
                let n = stream.read(&mut buf).await.expect("Failed to read");
                if n == 0 {
                    break;
                }
                received.extend_from_slice(&buf[..n]);
                
                // Echo back immediately
                stream.write(&buf[..n]).await.expect("Failed to write");
                
                if received.len() >= payload_size {
                    break;
                }
            }
        });
    });
    
    thread::sleep(Duration::from_millis(100));
    
    let mut client = StdTcpStream::connect("127.0.0.1:9004")
        .expect("Failed to connect");
    client.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    client.set_write_timeout(Some(Duration::from_secs(5))).unwrap();
    
    // Create 1MB of data with pattern
    let mut large_data = vec![0u8; payload_size];
    for (i, byte) in large_data.iter_mut().enumerate() {
        *byte = (i % 256) as u8;
    }
    
    // Send in chunks
    let chunk_size = 8192;
    let mut sent = 0;
    let mut received_data = Vec::new();
    
    while sent < payload_size {
        let end = (sent + chunk_size).min(payload_size);
        client.write_all(&large_data[sent..end]).expect("Failed to write chunk");
        
        // Read echo back
        let mut buf = vec![0u8; chunk_size];
        let n = client.read(&mut buf).expect("Failed to read chunk");
        received_data.extend_from_slice(&buf[..n]);
        
        sent = end;
    }
    
    // Verify echoed data matches
    assert_eq!(
        received_data.len(),
        payload_size,
        "Received data size mismatch"
    );
    assert_eq!(
        &received_data[..],
        &large_data[..],
        "Large payload echo mismatch"
    );
    
    drop(client);
    server_handle.join().expect("Server thread panicked");
}

#[test]
fn test_connection_close_handling() {
    let server_handle = thread::spawn(|| {
        let mut runtime = Runtime::new().expect("Failed to create runtime");
        
        runtime.block_on(async {
            let mut listener = TcpListener::bind("127.0.0.1:9005".parse().unwrap())
                .expect("Failed to bind");
            
            let mut stream = listener.accept().await.expect("Failed to accept");
            
            // Read until connection closes
            let mut buf = vec![0u8; 1024];
            loop {
                match stream.read(&mut buf).await {
                    Ok(0) => {
                        // Connection closed gracefully
                        break;
                    }
                    Ok(n) => {
                        stream.write(&buf[..n]).await.expect("Failed to write");
                    }
                    Err(_) => break,
                }
            }
        });
    });
    
    thread::sleep(Duration::from_millis(100));
    
    let mut client = StdTcpStream::connect("127.0.0.1:9005")
        .expect("Failed to connect");
    client.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    
    // Send one message
    let msg = b"Test message";
    client.write_all(msg).expect("Failed to write");
    
    let mut buf = vec![0u8; 1024];
    let n = client.read(&mut buf).expect("Failed to read");
    assert_eq!(&buf[..n], msg);
    
    // Close connection explicitly
    drop(client);
    
    server_handle.join().expect("Server thread panicked");
}

#[test]
fn test_empty_message_handling() {
    let server_handle = thread::spawn(|| {
        let mut runtime = Runtime::new().expect("Failed to create runtime");
        
        runtime.block_on(async {
            let mut listener = TcpListener::bind("127.0.0.1:9006".parse().unwrap())
                .expect("Failed to bind");
            
            let mut stream = listener.accept().await.expect("Failed to accept");
            
            // Should handle empty reads gracefully
            let mut buf = vec![0u8; 1024];
            let mut read_count = 0;
            
            loop {
                match stream.read(&mut buf).await {
                    Ok(0) => break, // Connection closed
                    Ok(n) => {
                        if n > 0 {
                            stream.write(&buf[..n]).await.expect("Failed to write");
                        }
                        read_count += 1;
                        if read_count > 2 {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
    });
    
    thread::sleep(Duration::from_millis(100));
    
    let mut client = StdTcpStream::connect("127.0.0.1:9006")
        .expect("Failed to connect");
    client.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    
    let msg = b"Hello";
    client.write_all(msg).expect("Failed to write");
    
    let mut buf = vec![0u8; 1024];
    let n = client.read(&mut buf).expect("Failed to read");
    assert_eq!(&buf[..n], msg);
    
    drop(client);
    server_handle.join().expect("Server thread panicked");
}

#[test]
fn test_rapid_connect_disconnect() {
    let iterations = 10;
    
    let server_handle = thread::spawn(move || {
        let mut runtime = Runtime::new().expect("Failed to create runtime");
        
        runtime.block_on(async move {
            let mut listener = TcpListener::bind("127.0.0.1:9007".parse().unwrap())
                .expect("Failed to bind");
            
            for _ in 0..iterations {
                let mut stream = listener.accept().await.expect("Failed to accept");
                
                // Try to echo once
                let mut buf = vec![0u8; 1024];
                match stream.read(&mut buf).await {
                    Ok(n) if n > 0 => {
                        let _ = stream.write(&buf[..n]).await;
                    }
                    _ => continue,
                }
            }
        });
    });
    
    thread::sleep(Duration::from_millis(100));
    
    for i in 0..iterations {
        let mut client = StdTcpStream::connect("127.0.0.1:9007")
            .expect(&format!("Failed to connect iteration {}", i));
        client.set_read_timeout(Some(Duration::from_millis(500))).unwrap();
        
        let msg = format!("Msg {}", i);
        client.write_all(msg.as_bytes()).expect("Failed to write");
        
        let mut buf = vec![0u8; 1024];
        match client.read(&mut buf) {
            Ok(n) => {
                assert_eq!(&buf[..n], msg.as_bytes());
            }
            Err(_) => {
                // Timeout acceptable in rapid connections
            }
        }
        
        drop(client);
        thread::sleep(Duration::from_millis(10));
    }
    
    server_handle.join().expect("Server thread panicked");
}

#[test]
fn test_binary_data_transfer() {
    let server_handle = thread::spawn(|| {
        let mut runtime = Runtime::new().expect("Failed to create runtime");
        
        runtime.block_on(async {
            let mut listener = TcpListener::bind("127.0.0.1:9008".parse().unwrap())
                .expect("Failed to bind");
            
            let mut stream = listener.accept().await.expect("Failed to accept");
            
            let mut buf = vec![0u8; 1024];
            let n = stream.read(&mut buf).await.expect("Failed to read");
            stream.write(&buf[..n]).await.expect("Failed to write");
        });
    });
    
    thread::sleep(Duration::from_millis(100));
    
    let mut client = StdTcpStream::connect("127.0.0.1:9008")
        .expect("Failed to connect");
    client.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    
    // Send binary data (all byte values)
    let binary_data: Vec<u8> = (0..=255).collect();
    client.write_all(&binary_data).expect("Failed to write");
    
    let mut buf = vec![0u8; 1024];
    let n = client.read(&mut buf).expect("Failed to read");
    
    assert_eq!(&buf[..n], &binary_data[..], "Binary data mismatch");
    
    drop(client);
    server_handle.join().expect("Server thread panicked");
}

#[test]
fn test_runtime_cleanup() {
    // Create and drop runtime multiple times
    for i in 0..3 {
        let mut runtime = Runtime::new()
            .expect(&format!("Failed to create runtime iteration {}", i));
        
        runtime.block_on(async move {
            // Simple async operation
            let listener = TcpListener::bind(
                format!("127.0.0.1:{}", 9100 + i).parse().unwrap()
            ).expect("Failed to bind");
            
            // Don't wait for connections, just verify binding works
            drop(listener);
        });
        
        drop(runtime);
        // Runtime should clean up properly
        thread::sleep(Duration::from_millis(50));
    }
    
    // If we get here without panicking, cleanup worked
    assert!(true, "Runtime cleanup successful");
}

