use bytes::BytesMut;
use tokio::io::{AsyncWriteExt, WriteHalf};

use crate::{error::ConnectionError, packets::Packet};

#[cfg(feature = "logs")]
use tracing::trace;

#[derive(Debug)]
pub struct WriteStream<S> {
    stream: WriteHalf<S>,
}

impl<S> WriteStream<S> {
    pub fn new(stream: WriteHalf<S>) -> Self {
        Self { stream }
    }
}

impl<S> WriteStream<S>
where
    S: tokio::io::AsyncWrite + Sized + Unpin,
{
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

        #[cfg(feature = "logs")]
        trace!("Sending packet {}", packet);

        self.stream.flush().await?;
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
