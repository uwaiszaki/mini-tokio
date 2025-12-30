use tokio_lite::runtime::Runtime;
use tokio_lite::net::TcpListener;

fn main() -> std::io::Result<()> {
    println!("Starting Tokio-Lite runtime...");
    tokio_lite::init_tracing();
 
    let mut runtime = Runtime::new()?;

    runtime.block_on(async {
        println!("[Main] Starting TCP echo server...");

        let mut listener = match TcpListener::bind("127.0.0.1:8080".parse().unwrap()) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[Main] Failed to bind: {}", e);
                return;
            }
        };
        println!("[Main] Listening on 127.0.0.1:8080");
        println!("[Main] Connect with: telnet 127.0.0.1 8080");
        
        loop {
            println!("\n[Main] Waiting for new connection...");
            let mut stream = match listener.accept().await {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[Main] Accept error: {}", e);
                    continue;
                }
            };
            println!("[Main] Accepted connection!");
            
            // Send welcome message
            let welcome_msg = b"Welcome to Tokio-Lite Echo Server!\nType something and press Enter:\n";
            match stream.write(welcome_msg).await {
                Ok(n) => println!("[Main] Sent welcome message ({} bytes)", n),
                Err(e) => {
                    println!("[Main] Failed to send welcome: {}", e);
                    continue;
                }
            }
            
            // Read and echo loop
            let mut buffer = [0u8; 1024];
            loop {
                println!("[Main] Waiting for data...");
                match stream.read(&mut buffer).await {
                    Ok(0) => {
                        println!("[Main] Connection closed by client");
                        break;
                    }
                    Ok(n) => {
                        println!("[Main] Received {} bytes: {:?}", n, 
                                 String::from_utf8_lossy(&buffer[..n]));
                        
                        // Echo back to client
                        let echo_msg = format!("Echo: {}", String::from_utf8_lossy(&buffer[..n]));
                        match stream.write(echo_msg.as_bytes()).await {
                            Ok(written) => println!("[Main] Echoed {} bytes back", written),
                            Err(e) => {
                                println!("[Main] Failed to echo: {}", e);
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        println!("[Main] Read error: {}", e);
                        break;
                    }
                }
            }
            
            println!("[Main] Connection handler finished");
        }
    });
    
    Ok(())
}