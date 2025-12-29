use mio::{Events, Token};

use crate::runtime::context;

pub fn run_reactor() {
    println!("[Reactor] Starting reactor loop");
    let mut events = Events::with_capacity(128);
    let state = context::state();

    // Wait on something and then wake the waker
    loop {
        // Lock poll only for the duration of poll() call
        let poll_result = {
            let mut poll = state.poll.lock().unwrap();
            poll.poll(&mut events, None)
        };
        
        if let Err(err) = poll_result {
            match err.kind() {
                std::io::ErrorKind::Interrupted => continue,
                _ => panic!("Poll error: {}", err),
            }
        }

        for event in &events {
            let Token(key) = event.token();
            if let Some(io_state) = state.slabs.read().unwrap().get(key).cloned() {
                println!("[Reactor] Event received for token {}: readable={} writable={}", 
                         key, event.is_readable(), event.is_writable());
                if event.is_readable() {
                    io_state.read_waker.wake();
                }
                if event.is_writable() {
                    io_state.write_waker.wake();
                }
            }
        }
    }
}
