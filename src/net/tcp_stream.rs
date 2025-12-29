// use super::tcp_stream_read::TcpStreamRead;
use std::sync::Arc;
use mio::{Token, net::TcpStream as Stream};

use crate::runtime::IoState;

use super::{
    read::TcpReadStreamFuture,
    write::TcpStreamWriteFuture
};

pub struct TcpStream {
    pub stream: Stream,
    pub io_state: Arc<IoState>,
    pub token: Token
}

impl TcpStream {
    pub fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> TcpReadStreamFuture<'a, 'a> {
        TcpReadStreamFuture {
            token: self.token,
            stream: &mut self.stream,
            io_state: self.io_state.clone(),
            buf
        }
    }

    pub fn write<'a>(&'a mut self, buf: &'a [u8]) -> TcpStreamWriteFuture<'a> {
        TcpStreamWriteFuture {
            token: self.token,
            stream: &mut self.stream,
            io_state: self.io_state.clone(),
            buf
        }
    }
}