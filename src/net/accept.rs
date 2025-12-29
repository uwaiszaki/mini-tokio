use std::{future::Future, io::{ErrorKind, Result}, pin::Pin, sync::Arc, task::{Context, Poll}};

use mio::{Interest, Token};

use crate::runtime::{context, state::IoState};

use super::tcp_listener::TcpListener;
use super::tcp_stream::TcpStream;

pub struct TcpAcceptFuture<'a> {
    pub listener: &'a mut TcpListener,
    pub io_state: Arc<IoState>
}

impl<'a> Future for TcpAcceptFuture<'a> {
    type Output = Result<TcpStream>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let state = context::state();

        match self.listener.listener.accept() {
            Ok((mut stream, _)) => {
                let io_state = Arc::new(IoState::new());
                let token_state = io_state.clone();
                let token = {
                    let mut slabs = state.slabs.write().unwrap();
                    let entry = slabs.vacant_entry();
                    let key = entry.key();
                    entry.insert(token_state);
                    Token(key)
                };
                let registry = &state.poll_registry;
                // Registered the stream for fd polling
                match registry.register(&mut stream, token, Interest::READABLE | Interest::WRITABLE) {
                    Ok(_) => Poll::Ready(Ok(TcpStream { stream, token, io_state })),
                    Err(e) => Poll::Ready(Err(e))
                }
            },
            Err(e) => match e.kind() {
                ErrorKind::WouldBlock => {
                    self.io_state.read_waker.register(cx.waker());
                    Poll::Pending
                },
                _ => Poll::Ready(Err(e))
            }
        }
    }
}