pub mod read_half;
pub mod write_half;

use std::io::{self, Error, ErrorKind};

use bytes::{Buf, BytesMut};

use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[cfg(feature = "logs")]
use tracing::trace;

use crate::packets::error::WriteError;
use crate::packets::ConnAck;
use crate::packets::{
    error::ReadBytes,
    ConnAckReasonCode, {FixedHeader, Packet},
};
use crate::{connect_options::ConnectOptions, error::ConnectionError};

use self::read_half::ReadStream;
use self::write_half::WriteStream;

#[derive(Debug)]
pub struct Stream<S> {
    stream: S,
}

impl<S> Stream<S>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Sized + Unpin,
{
    pub fn split(self) -> (ReadStream<S>, WriteStream<S>) {
        let Self { stream } = self;

        let (read_stream, write_stream) = tokio::io::split(stream);

        (ReadStream::new(read_stream), WriteStream::new(write_stream))
    }

    pub async fn connect(options: &ConnectOptions, stream: S) -> Result<(Self, ConnAck), ConnectionError> {
        let mut s = Self { stream };

        let connect = options.create_connect_from_options();

        s.write(&connect).await?;

        let packet = Packet::async_read(&mut s.stream).await?;
        if let Packet::ConnAck(con) = packet {
            if con.reason_code == ConnAckReasonCode::Success {
                #[cfg(feature = "logs")]
                trace!("Connected to server");
                Ok((s, con))
            } else {
                Err(ConnectionError::ConnectionRefused(con.reason_code))
            }
        } else {
            Err(ConnectionError::NotConnAck(packet))
        }
    }

    pub async fn read(&mut self) -> Result<Packet, ConnectionError> {
        Ok(Packet::async_read(&mut self.stream).await?)
    }

    // pub async fn read_bytes(&mut self) -> io::Result<usize> {
    //     let read = self.stream.read(&mut self.const_buffer).await?;
    //     if read == 0 {
    //         Err(io::Error::new(io::ErrorKind::ConnectionReset, "Connection reset by peer"))
    //     } else {
    //         self.read_buffer.extend_from_slice(&self.const_buffer[0..read]);
    //         Ok(read)
    //     }
    // }

    // /// Reads more than 'required' bytes to frame a packet into self.read buffer
    // pub async fn read_required_bytes(&mut self, required: usize) -> io::Result<usize> {
    //     let mut total_read = 0;

    //     loop {
    //         let read = self.read_bytes().await?;
    //         total_read += read;
    //         if total_read >= required {
    //             return Ok(total_read);
    //         }
    //     }
    // }

    pub async fn write(&mut self, packet: &Packet) -> Result<(), ConnectionError> {
        match packet.async_write(&mut self.stream).await {
            Ok(_) => (),
            Err(err) => {
                return match err {
                    crate::packets::error::WriteError::SerializeError(serialize_error) => Err(ConnectionError::SerializationError(serialize_error)),
                    crate::packets::error::WriteError::IoError(error) => Err(ConnectionError::Io(error)),
                }
            }
        }
        self.stream.flush().await?;

        #[cfg(feature = "logs")]
        trace!("Sending packet {}", packet);

        Ok(())
    }

    pub async fn write_all(&mut self, packets: &mut Vec<Packet>) -> Result<(), ConnectionError> {
        for packet in packets {
            let _ = packet.async_write(&mut self.stream).await;
            #[cfg(feature = "logs")]
            trace!("Sending packet {}", packet);
        }
        self.stream.flush().await?;
        Ok(())
    }
}
