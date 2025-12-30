//! Runtime metrics using the `metrics` crate
//!
//! This module emits metrics that can be collected by any metrics backend
//! (Prometheus, OpenTelemetry, StatsD, etc.)

/// Emit task-related metrics
#[cfg(feature = "metrics")]
pub(crate) mod emit {
    use metrics::{counter, gauge};
    
    #[inline]
    pub fn task_spawned() {
        counter!("tokio_lite.tasks.spawned").increment(1);
    }
    
    #[inline]
    pub fn task_completed() {
        counter!("tokio_lite.tasks.completed").increment(1);
    }
    
    #[inline]
    pub fn task_panicked() {
        counter!("tokio_lite.tasks.panicked").increment(1);
    }
    
    #[inline]
    pub fn io_event_processed() {
        counter!("tokio_lite.io.events_processed").increment(1);
    }
    
    #[inline]
    pub fn bytes_read(n: u64) {
        counter!("tokio_lite.io.bytes_read").increment(n);
    }
    
    #[inline]
    pub fn bytes_written(n: u64) {
        counter!("tokio_lite.io.bytes_written").increment(n);
    }
    
    #[inline]
    pub fn reactor_poll() {
        counter!("tokio_lite.reactor.polls").increment(1);
    }
    
    #[inline]
    pub fn tasks_in_progress(n: usize) {
        gauge!("tokio_lite.tasks.in_progress").set(n as f64);
    }
}

/// No-op implementations when metrics are disabled
#[cfg(not(feature = "metrics"))]
pub(crate) mod emit {
    #[inline]
    pub fn task_spawned() {}
    
    #[inline]
    pub fn task_completed() {}
    
    #[inline]
    pub fn task_panicked() {}
    
    #[inline]
    pub fn io_event_processed() {}
    
    #[inline]
    pub fn bytes_read(_n: u64) {}
    
    #[inline]
    pub fn bytes_written(_n: u64) {}
    
    #[inline]
    pub fn reactor_poll() {}
    
    #[inline]
    pub fn tasks_in_progress(_n: usize) {}
}


