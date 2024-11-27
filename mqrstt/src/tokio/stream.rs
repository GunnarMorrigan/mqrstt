use tokio::io::AsyncWriteExt;

#[cfg(feature = "logs")]
use tracing::trace;

use crate::packets::ConnAck;
use crate::packets::{ConnAckReasonCode, Packet};
use crate::{connect_options::ConnectOptions, error::ConnectionError};

pub(crate) trait StreamExt {
    fn connect(&mut self, options: &ConnectOptions) -> impl std::future::Future<Output = Result<ConnAck, ConnectionError>>;
    fn read_packet(&mut self) -> impl std::future::Future<Output = Result<Packet, ConnectionError>>;
    fn write_packet(&mut self, packet: &Packet) -> impl std::future::Future<Output = Result<(), ConnectionError>>;
    fn write_packets(&mut self, packets: &[Packet]) -> impl std::future::Future<Output = Result<(), ConnectionError>>;
    fn flush_packets(&mut self) -> impl std::future::Future<Output = std::io::Result<()>>;
}

impl<S> StreamExt for S
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Sized + Unpin,
{
    fn connect(&mut self, options: &ConnectOptions) -> impl std::future::Future<Output = Result<ConnAck, ConnectionError>> {
        async move {
            let connect = options.create_connect_from_options();

            self.write_packet(&connect).await?;

            let packet = Packet::async_read(self).await?;
            if let Packet::ConnAck(con) = packet {
                if con.reason_code == ConnAckReasonCode::Success {
                    #[cfg(feature = "logs")]
                    trace!("Connected to server");
                    Ok(con)
                } else {
                    Err(ConnectionError::ConnectionRefused(con.reason_code))
                }
            } else {
                Err(ConnectionError::NotConnAck(packet))
            }
        }
    }

    fn read_packet(&mut self) -> impl std::future::Future<Output = Result<Packet, ConnectionError>> {
        async move { Ok(Packet::async_read(self).await?) }
    }

    fn write_packet(&mut self, packet: &Packet) -> impl std::future::Future<Output = Result<(), ConnectionError>> {
        async move {
            match packet.async_write(self).await {
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

            self.flush().await?;
            // self.flush_packets().await?;

            Ok(())
        }
    }

    async fn write_packets(&mut self, packets: &[Packet]) -> Result<(), ConnectionError> {
        for packet in packets {
            let _ = packet.async_write(self).await;
            #[cfg(feature = "logs")]
            trace!("Sending packet {}", packet);
        }
        self.flush_packets().await?;
        Ok(())
    }

    fn flush_packets(&mut self) -> impl std::future::Future<Output = std::io::Result<()>> {
        tokio::io::AsyncWriteExt::flush(self)
    }
}
