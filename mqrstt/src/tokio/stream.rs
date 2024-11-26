use tokio::io::AsyncWriteExt;

#[cfg(feature = "logs")]
use tracing::trace;

use crate::packets::ConnAck;
use crate::packets::{ConnAckReasonCode, Packet};
use crate::{connect_options::ConnectOptions, error::ConnectionError};

#[derive(Debug)]
pub struct Stream<S> {
    stream: S,
}

impl<S> Stream<S>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Sized + Unpin,
{
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
