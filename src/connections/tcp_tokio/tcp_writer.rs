use async_channel::Receiver;
use bytes::BytesMut;
use tokio::net::tcp::{OwnedWriteHalf};
#[cfg(feature = "tokio")]
use tokio::{io::{AsyncWriteExt,}};
#[cfg(feature = "smol")]
use smol::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream};
use tracing::trace;

use crate::{error::ConnectionError, packets::packets::Packet};

use super::AsyncMqttNetworkWrite;


pub struct TcpWriter{
    writehalf: OwnedWriteHalf,

    buffer: BytesMut,
}

impl TcpWriter{
    pub fn new(writehalf: OwnedWriteHalf) -> Self{
        Self{
            writehalf,
            buffer: BytesMut::with_capacity(20 * 1024),
        }
    }
}

impl AsyncMqttNetworkWrite for TcpWriter{
    async fn write_buffer(&mut self, buffer: &mut BytesMut) -> Result<(), ConnectionError> {
        if buffer.is_empty(){
            return Ok(());
        }

        self.writehalf.write_all(&buffer[..]).await?;
        buffer.clear();
        Ok(())
    }

    async fn write(&mut self, outgoing: &Receiver<Packet>) -> Result<(), ConnectionError> {
        let packet = outgoing.recv().await?;

        trace!("Writing Packet to network: {:?}", packet);

        packet.write(&mut self.buffer)?;

        self.writehalf.write_all(&self.buffer[..]).await?;
        self.buffer.clear();
        Ok(())
    }   
}