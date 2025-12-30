pub mod runtime;
pub mod net;
pub mod timer;

// Re-export commonly used items for convenience
pub use runtime::Runtime;
pub use net::TcpListener;

/// Initialize tracing with default settings (stdout, INFO level)
/// # Default Level
/// If `RUST_LOG` is not set, defaults to `tokio_lite=debug,info`
#[cfg(feature = "tracing")]
pub fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};
    
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("tokio_lite=debug,info"));
    
    fmt()
        .with_env_filter(filter)
        .try_init()
        .ok(); // Ignore if already initialized
}

#[cfg(not(feature = "tracing"))]
pub fn init_tracing() {}
