use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use std::net::TcpStream as StdTcpStream;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tokio_lite::net::TcpListener;
use tokio_lite::Runtime;

// Helper: Start echo server in background
fn start_echo_server(port: u16) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut runtime = Runtime::new().expect("Failed to create runtime");
        
        runtime.block_on(async move {
            let mut listener = TcpListener::bind(
                format!("127.0.0.1:{}", port).parse().unwrap()
            ).expect("Failed to bind");
            
            // Handle one connection then exit
            let mut stream = listener.accept().await.expect("Failed to accept");
            
            let mut buf = vec![0u8; 65536];
            loop {
                match stream.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        stream.write(&buf[..n]).await.expect("Failed to write");
                    }
                    Err(_) => break,
                }
            }
        });
    })
}

fn benchmark_task_spawning(c: &mut Criterion) {
    let mut group = c.benchmark_group("task_spawning");
    
    group.bench_function("spawn_empty_task", |b| {
        b.iter(|| {
            let mut runtime = Runtime::new().expect("Failed to create runtime");
            runtime.block_on(async {
                // Empty task
            });
        });
    });
    
    group.bench_function("spawn_simple_task", |b| {
        b.iter(|| {
            let mut runtime = Runtime::new().expect("Failed to create runtime");
            runtime.block_on(async {
                let x = black_box(42);
                let y = black_box(x + 1);
                black_box(y);
            });
        });
    });
    
    group.finish();
}

fn benchmark_tcp_echo(c: &mut Criterion) {
    let mut group = c.benchmark_group("tcp_echo");
    
    // Test different payload sizes
    for size in [64, 512, 4096, 32768].iter() {
        group.throughput(Throughput::Bytes(*size as u64));
        
        group.bench_with_input(
            BenchmarkId::new("echo", size),
            size,
            |b, &size| {
                let port = 8100 + (size / 64) as u16; // Different port per size
                let _server = start_echo_server(port);
                thread::sleep(Duration::from_millis(100));
                
                b.iter(|| {
                    let mut client = StdTcpStream::connect(format!("127.0.0.1:{}", port))
                        .expect("Failed to connect");
                    client.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
                    
                    let data = vec![0u8; size];
                    client.write_all(&data).expect("Failed to write");
                    
                    let mut buf = vec![0u8; size];
                    client.read_exact(&mut buf).expect("Failed to read");
                    
                    black_box(buf);
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_concurrent_connections(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_connections");
    group.sample_size(20); // Fewer samples for longer benchmarks
    
    for conn_count in [1, 5, 10].iter() {
        group.bench_with_input(
            BenchmarkId::new("connections", conn_count),
            conn_count,
            |b, &conn_count| {
                b.iter(|| {
                    let port = 8200;
                    let server_handle = thread::spawn(move || {
                        let mut runtime = Runtime::new().expect("Failed to create runtime");
                        
                        runtime.block_on(async move {
                            let mut listener = TcpListener::bind(
                                format!("127.0.0.1:{}", port).parse().unwrap()
                            ).expect("Failed to bind");
                            
                            for _ in 0..conn_count {
                                let mut stream = listener.accept().await.expect("Failed to accept");
                                let mut buf = vec![0u8; 1024];
                                let n = stream.read(&mut buf).await.expect("Failed to read");
                                stream.write(&buf[..n]).await.expect("Failed to write");
                            }
                        });
                    });
                    
                    thread::sleep(Duration::from_millis(100));
                    
                    let mut handles = vec![];
                    for i in 0..conn_count {
                        let handle = thread::spawn(move || {
                            let mut client = StdTcpStream::connect(format!("127.0.0.1:{}", port))
                                .expect("Failed to connect");
                            client.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
                            
                            let msg = format!("Message {}", i);
                            client.write_all(msg.as_bytes()).expect("Failed to write");
                            
                            let mut buf = vec![0u8; 1024];
                            let n = client.read(&mut buf).expect("Failed to read");
                            black_box(&buf[..n]);
                        });
                        handles.push(handle);
                    }
                    
                    for handle in handles {
                        handle.join().unwrap();
                    }
                    
                    server_handle.join().unwrap();
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_large_transfers(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_transfers");
    group.sample_size(10); // Fewer samples for large transfers
    
    for size in [65536, 262144, 1048576].iter() { // 64KB, 256KB, 1MB
        group.throughput(Throughput::Bytes(*size as u64));
        
        group.bench_with_input(
            BenchmarkId::new("transfer", size),
            size,
            |b, &size| {
                let port = 8300;
                
                b.iter(|| {
                    let server_handle = thread::spawn(move || {
                        let mut runtime = Runtime::new().expect("Failed to create runtime");
                        
                        runtime.block_on(async move {
                            let mut listener = TcpListener::bind(
                                format!("127.0.0.1:{}", port).parse().unwrap()
                            ).expect("Failed to bind");
                            
                            let mut stream = listener.accept().await.expect("Failed to accept");
                            
                            let mut buf = vec![0u8; 8192];
                            let mut received = 0;
                            
                            while received < size {
                                let n = stream.read(&mut buf).await.expect("Failed to read");
                                if n == 0 { break; }
                                stream.write(&buf[..n]).await.expect("Failed to write");
                                received += n;
                            }
                        });
                    });
                    
                    thread::sleep(Duration::from_millis(100));
                    
                    let mut client = StdTcpStream::connect(format!("127.0.0.1:{}", port))
                        .expect("Failed to connect");
                    client.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
                    
                    let data = vec![0u8; size];
                    let chunk_size = 8192;
                    let mut sent = 0;
                    let mut received_data = Vec::new();
                    
                    while sent < size {
                        let end = (sent + chunk_size).min(size);
                        client.write_all(&data[sent..end]).expect("Failed to write");
                        
                        let mut buf = vec![0u8; chunk_size];
                        let n = client.read(&mut buf).expect("Failed to read");
                        received_data.extend_from_slice(&buf[..n]);
                        
                        sent = end;
                    }
                    
                    black_box(received_data);
                    drop(client);
                    server_handle.join().unwrap();
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_runtime_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("runtime_overhead");
    
    group.bench_function("runtime_creation", |b| {
        b.iter(|| {
            let runtime = Runtime::new().expect("Failed to create runtime");
            black_box(runtime);
        });
    });
    
    group.bench_function("runtime_with_noop_task", |b| {
        b.iter(|| {
            let mut runtime = Runtime::new().expect("Failed to create runtime");
            runtime.block_on(async {
                // No-op
            });
        });
    });
    
    group.bench_function("listener_bind", |b| {
        b.iter(|| {
            let mut runtime = Runtime::new().expect("Failed to create runtime");
            runtime.block_on(async {
                let listener = TcpListener::bind("127.0.0.1:0".parse().unwrap())
                    .expect("Failed to bind");
                black_box(listener);
            });
        });
    });
    
    group.finish();
}

fn benchmark_async_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("async_operations");
    
    group.bench_function("single_read_write", |b| {
        let port = 8400;
        let _server = start_echo_server(port);
        thread::sleep(Duration::from_millis(100));
        
        b.iter(|| {
            let mut client = StdTcpStream::connect(format!("127.0.0.1:{}", port))
                .expect("Failed to connect");
            client.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
            
            let data = b"Hello, World!";
            client.write_all(data).expect("Failed to write");
            
            let mut buf = vec![0u8; 1024];
            let n = client.read(&mut buf).expect("Failed to read");
            black_box(&buf[..n]);
        });
    });
    
    group.bench_function("multiple_small_writes", |b| {
        let port = 8401;
        let _server = start_echo_server(port);
        thread::sleep(Duration::from_millis(100));
        
        b.iter(|| {
            let mut client = StdTcpStream::connect(format!("127.0.0.1:{}", port))
                .expect("Failed to connect");
            client.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
            
            for _ in 0..10 {
                let data = b"Test";
                client.write_all(data).expect("Failed to write");
                
                let mut buf = vec![0u8; 1024];
                let n = client.read(&mut buf).expect("Failed to read");
                black_box(&buf[..n]);
            }
        });
    });
    
    group.finish();
}

fn benchmark_concurrent_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_load");
    group.sample_size(10);
    
    // Real concurrent benchmark - spawns N threads that ALL connect simultaneously
    // Shows TOTAL time AND average time per connection (like Go's ns/op)
    for conn_count in [5, 10, 25, 50, 100, 500, 1000].iter() {
        group.throughput(Throughput::Elements(*conn_count as u64)); // Enable per-connection metrics!
        
        group.bench_with_input(
            BenchmarkId::new("parallel_clients", conn_count),
            conn_count,
            |b, &conn_count| {
                b.iter(|| {
                    let port = 8800;
                    let completed = Arc::new(Mutex::new(0));
                    let completed_clone = completed.clone();
                    
                    let server_handle = thread::spawn(move || {
                        let mut runtime = Runtime::new().expect("Failed to create runtime");
                        
                        runtime.block_on(async move {
                            let mut listener = TcpListener::bind(
                                format!("127.0.0.1:{}", port).parse().unwrap()
                            ).expect("Failed to bind");
                            
                            // Accept all connections
                            for _ in 0..conn_count {
                                let mut stream = listener.accept().await.expect("Failed to accept");
                                let mut buf = vec![0u8; 64];
                                let n = stream.read(&mut buf).await.expect("Failed to read");
                                stream.write(&buf[..n]).await.expect("Failed to write");
                                
                                *completed_clone.lock().unwrap() += 1;
                            }
                        });
                    });
                    
                    thread::sleep(Duration::from_millis(100));
                    
                    let start = std::time::Instant::now();
                    
                    // Spawn N client threads SIMULTANEOUSLY (like goroutines!)
                    let mut client_handles = vec![];
                    for i in 0..conn_count {
                        let handle = thread::spawn(move || {
                            let mut client = StdTcpStream::connect(format!("127.0.0.1:{}", port))
                                .expect(&format!("Client {} failed to connect", i));
                            client.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
                            
                            let msg = format!("req{}", i);
                            client.write_all(msg.as_bytes()).expect("Failed to write");
                            
                            let mut buf = vec![0u8; 64];
                            client.read(&mut buf).expect("Failed to read");
                        });
                        client_handles.push(handle);
                    }
                    
                    // Wait for ALL clients to finish
                    for handle in client_handles {
                        handle.join().expect("Client thread panicked");
                    }
                    
                    let elapsed = start.elapsed();
                    server_handle.join().unwrap();
                    
                    // Verify all were handled
                    assert_eq!(*completed.lock().unwrap(), conn_count);
                    
                    black_box(elapsed);
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_task_spawning,
    benchmark_runtime_overhead,
    benchmark_tcp_echo,
    benchmark_async_operations,
    benchmark_concurrent_connections,
    benchmark_large_transfers,
    benchmark_concurrent_load,
);

criterion_main!(benches);

