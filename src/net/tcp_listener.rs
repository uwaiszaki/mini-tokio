use std::net::SocketAddr;
use std::sync::Arc;
use mio::{Interest, Token, net::TcpListener as Listener};

use crate::{net::accept::TcpAcceptFuture, runtime::{IoState, context}};

pub struct TcpListener {
    pub listener: Listener,
    pub token: Token,
    pub io_state: Arc<IoState>
}


impl TcpListener {
    pub fn bind(addr: SocketAddr) -> std::io::Result<Self> {
        let mut listener = Listener::bind(addr)?;
        let state = context::state();
        let io_state = Arc::new(IoState::new());
        let slab_state = io_state.clone();
        let token = {
            let mut slabs = state.slabs.write().unwrap();
            let entry = slabs.vacant_entry();
            let token = entry.key();
            entry.insert(slab_state);
            Token(token)
        };

        // Register the listener for fd polling
        state.poll_registry.register(&mut listener, token, Interest::READABLE)?;

        Ok(TcpListener { listener, token, io_state })
    }

    pub fn accept(&mut self) -> TcpAcceptFuture {
        let io_state = self.io_state.clone();
        TcpAcceptFuture {
            listener: self,
            io_state
        }
    }
}
