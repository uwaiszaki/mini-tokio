use mio::Token;
use std::future::Future;
use std::sync::Arc;
use std::io::Read;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::io::ErrorKind;

use crate::runtime::IoState;
use crate::runtime::trace::*;
use crate::runtime::metrics::emit as metrics_emit;

pub struct TcpReadStreamFuture<'a, 'b> {
    pub token: Token,
    pub stream: &'a mut mio::net::TcpStream,
    pub buf: &'b mut [u8],
    pub io_state: Arc<IoState>,
}

impl<'a, 'b> Future for TcpReadStreamFuture<'a, 'b> {
    type Output = Result<usize, std::io::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let future = self.get_mut();

        let stream = &mut future.stream;
        match stream.read(future.buf) {
            Ok(n) => {
                if n > 0 {
                    trace!("Read {} bytes from stream (token {})", n, future.token.0);
                    metrics_emit::bytes_read(n as u64);
                }
                Poll::Ready(Ok(n))
            },
            Err(err) => match err.kind() {
                ErrorKind::WouldBlock => {
                    trace!("Read would block, registering waker");
                    future.io_state.read_waker.register(cx.waker());
                    Poll::Pending
                },
                _ => {
                    error!("Read error: {}", err);
                    Poll::Ready(Err(err))
                }
            }
        }
    }
}
