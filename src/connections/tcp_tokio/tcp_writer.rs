use async_channel::Receiver;
use bytes::BytesMut;
#[cfg(feature = "smol")]
use smol::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
#[cfg(feature = "tokio")]
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::OwnedWriteHalf;
use tracing::trace;

use crate::{
    error::ConnectionError,
    packets::packets::{Packet, PacketType},
};

use super::AsyncMqttNetworkWrite;

pub struct TcpWriter {
    writehalf: OwnedWriteHalf,

    buffer: BytesMut,
}

impl TcpWriter {
    pub fn new(writehalf: OwnedWriteHalf) -> Self {
        Self {
            writehalf,
            buffer: BytesMut::with_capacity(20 * 1024),
        }
    }
}

impl AsyncMqttNetworkWrite for TcpWriter {
    async fn write_buffer(&mut self, buffer: &mut BytesMut) -> Result<(), ConnectionError> {
        if buffer.is_empty() {
            return Ok(());
        }

        self.writehalf.write_all(&buffer[..]).await?;
        buffer.clear();
        Ok(())
    }

    async fn write(&mut self, outgoing: &Receiver<Packet>) -> Result<bool, ConnectionError> {
        let mut disconnect = false;

        let packet = outgoing.recv().await?;
        packet.write(&mut self.buffer)?;
        if packet.packet_type() == PacketType::Disconnect {
            disconnect = true;
        }

        while !outgoing.is_empty() && !disconnect {
            let packet = outgoing.recv().await?;
            packet.write(&mut self.buffer)?;
            if packet.packet_type() == PacketType::Disconnect {
                disconnect = true;
                break;
            }
            trace!("Going to write packet to network: {:?}", packet);
        }

        self.writehalf.write_all(&self.buffer[..]).await?;
        self.writehalf.flush().await?;
        self.buffer.clear();
        Ok(disconnect)
    }
}
