use std::io;

use bytes::{Buf, BytesMut};
use tokio::io::{AsyncReadExt, ReadHalf};

use crate::{
    error::ConnectionError,
    packets::{error::ReadBytes, FixedHeader, Packet},
};

#[cfg(feature = "logs")]
use tracing::trace;

#[derive(Debug)]
pub struct ReadStream<S> {
    stream: ReadHalf<S>,
}

impl<S> ReadStream<S>
where
    S: tokio::io::AsyncRead + Sized + Unpin,
{
    pub fn new(stream: ReadHalf<S>) -> Self {
        Self { stream }
    }

    pub async fn read(&mut self) -> Result<Packet, ConnectionError> {
        Ok(Packet::async_read(&mut self.stream).await?)
    }
}
