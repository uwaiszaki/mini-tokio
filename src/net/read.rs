use mio::Token;
use std::future::Future;
use std::sync::Arc;
use std::io::Read;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::io::ErrorKind;

use crate::runtime::IoState;

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
            Ok(n) => Poll::Ready(Ok(n)),
            Err(err) => match err.kind() {
                ErrorKind::WouldBlock => {
                    future.io_state.read_waker.register(cx.waker());
                    Poll::Pending
                },
                _ => Poll::Ready(Err(err))
            }
        }
    }
}
