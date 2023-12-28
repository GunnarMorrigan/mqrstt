use bytes::BytesMut;
use tokio::io::{AsyncWriteExt, WriteHalf};

use crate::{error::ConnectionError, packets::Packet};

#[derive(Debug)]
pub struct WriteStream<S> {
    pub stream: WriteHalf<S>,

    /// Write buffer
    write_buffer: BytesMut,
}

impl<S> WriteStream<S> {
    pub fn new(stream: WriteHalf<S>, write_buffer: BytesMut) -> Self {
        Self { stream, write_buffer }
    }
}

impl<S> WriteStream<S>
where
    S: tokio::io::AsyncWrite + Sized + Unpin,
{
    pub async fn write(&mut self, packet: &Packet) -> Result<(), ConnectionError> {
        packet.write(&mut self.write_buffer)?;

        #[cfg(feature = "logs")]
        trace!("Sending packet {}", packet);

        self.stream.write_all(&self.write_buffer[..]).await?;
        self.stream.flush().await?;
        self.write_buffer.clear();
        Ok(())
    }

    pub async fn write_all<I>(&mut self, packets: &mut I) -> Result<(), ConnectionError>
    where
        I: Iterator<Item = Packet>,
    {
        let writes = packets.map(|packet| {
            packet.write(&mut self.write_buffer)?;

            #[cfg(feature = "logs")]
            trace!("Sending packet {}", packet);

            Ok::<(), ConnectionError>(())
        });

        for write in writes {
            write?;
        }

        self.stream.write_all(&self.write_buffer[..]).await?;
        self.stream.flush().await?;
        self.write_buffer.clear();
        Ok(())
    }
}
