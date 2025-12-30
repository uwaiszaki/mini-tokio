use mio::{Events, Token};

use crate::runtime::context;
use crate::runtime::trace::*;
use crate::runtime::metrics::emit as metrics_emit;

pub fn run_reactor() {
    info!("Reactor started");
    let mut events = Events::with_capacity(128);
    let state = context::state();

    loop {
        // Lock poll only for the duration of poll() call
        let poll_result = {
            let mut poll = state.poll.lock().unwrap();
            poll.poll(&mut events, None)
        };
        
        if let Err(err) = poll_result {
            match err.kind() {
                std::io::ErrorKind::Interrupted => {
                    trace!("Poll interrupted, continuing");
                    continue;
                }
                _ => {
                    error!("Fatal poll error: {}", err);
                    panic!("Poll error: {}", err);
                }
            }
        }

        metrics_emit::reactor_poll();
        
        let event_count = events.iter().count();
        if event_count > 0 {
            debug!("Processing {} I/O events", event_count);
        }

        for event in &events {
            let Token(key) = event.token();
            if let Some(io_state) = state.slabs.read().unwrap().get(key).cloned() {
                trace!(
                    "I/O event for token {}: readable={}, writable={}", 
                    key, 
                    event.is_readable(), 
                    event.is_writable()
                );
                
                metrics_emit::io_event_processed();
                
                if event.is_readable() {
                    io_state.read_waker.wake();
                }
                if event.is_writable() {
                    io_state.write_waker.wake();
                }
            } else {
                warn!("Received event for unknown token: {}", key);
            }
        }
    }
}
