use std::{future::Future, io::{ErrorKind, Write}, sync::Arc, task::Poll};

use mio::Token;

use crate::runtime::IoState;

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
            Ok(n) => Poll::Ready(Ok(n)),
            Err(e) => match e.kind() {
                ErrorKind::WouldBlock => {
                    future.io_state.write_waker.register(cx.waker());
                    Poll::Pending
                },
                _ => Poll::Ready(Err(e))
            }
        }
    }
}