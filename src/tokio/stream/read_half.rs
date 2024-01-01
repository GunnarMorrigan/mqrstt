use std::io;

use bytes::{Buf, BytesMut};
use tokio::io::{AsyncReadExt, ReadHalf};

use crate::{
    error::ConnectionError,
    packets::{error::ReadBytes, FixedHeader, Packet},
};

#[derive(Debug)]
pub struct ReadStream<S> {
    stream: ReadHalf<S>,

    /// Input buffer
    const_buffer: [u8; 4096],

    /// Write buffer
    read_buffer: BytesMut,
}

impl<S> ReadStream<S>
where
    S: tokio::io::AsyncRead + Sized + Unpin,
{
    pub fn new(stream: ReadHalf<S>, const_buffer: [u8; 4096], read_buffer: BytesMut) -> Self {
        Self { stream, const_buffer, read_buffer }
    }

    pub fn parse_message(&mut self) -> Result<Packet, ReadBytes<ConnectionError>> {
        let (header, header_length) = FixedHeader::read_fixed_header(self.read_buffer.iter())?;

        if header.remaining_length + header_length > self.read_buffer.len() {
            return Err(ReadBytes::InsufficientBytes(header.remaining_length - self.read_buffer.len()));
        }

        self.read_buffer.advance(header_length);

        let buf = self.read_buffer.split_to(header.remaining_length);
        let read_packet = Packet::read(header, buf.into())?;

        #[cfg(feature = "logs")]
        trace!("Read packet from network {}", read_packet);

        Ok(read_packet)
    }

    pub async fn read_bytes(&mut self) -> io::Result<usize> {
        let read = self.stream.read(&mut self.const_buffer).await?;
        if read == 0 {
            Err(io::Error::new(io::ErrorKind::ConnectionReset, "Connection reset by peer"))
        } else {
            self.read_buffer.extend_from_slice(&self.const_buffer[0..read]);
            Ok(read)
        }
    }
}
