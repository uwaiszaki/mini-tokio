use std::{future::Future, io::{ErrorKind, Write}, sync::Arc, task::Poll};

use mio::Token;

use crate::runtime::IoState;
use crate::runtime::trace::*;
use crate::runtime::metrics::emit as metrics_emit;

pub struct TcpStreamWriteFuture<'a> {
    pub stream: &'a mut mio::net::TcpStream,
    pub io_state: Arc<IoState>,
    pub token: Token,
    pub buf: &'a [u8]
}

impl<'a> Future for TcpStreamWriteFuture<'a> {
    type Output = std::io::Result<usize>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let future = self.get_mut();
        let stream = &mut future.stream;

        match stream.write(future.buf) {
            Ok(n) => {
                if n > 0 {
                    trace!("Wrote {} bytes to stream (token {})", n, future.token.0);
                    metrics_emit::bytes_written(n as u64);
                }
                Poll::Ready(Ok(n))
            },
            Err(e) => match e.kind() {
                ErrorKind::WouldBlock => {
                    trace!("Write would block, registering waker");
                    future.io_state.write_waker.register(cx.waker());
                    Poll::Pending
                },
                _ => {
                    error!("Write error: {}", e);
                    Poll::Ready(Err(e))
                }
            }
        }
    }
}