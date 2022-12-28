use std::io::{self, Error, ErrorKind};

use async_channel::Receiver;
use bytes::{Buf, BytesMut};
// #[cfg(feature = "smol")]
// use smol::{
//     io::{AsyncReadExt, AsyncWriteExt},
//     net::TcpStream,
// };
use tokio::{net::tcp::{OwnedReadHalf, OwnedWriteHalf}, io::AsyncWriteExt};
#[cfg(feature = "tokio")]
use tokio::{io::AsyncReadExt, net::TcpStream};
use tracing::trace;

use crate::packets::{
    error::ReadBytes,
    packets::{FixedHeader, Packet, PacketType},
    reason_codes::ConnAckReasonCode,
};
use crate::{
    connect_options::ConnectOptions,
    connections::{create_connect_from_options, AsyncMqttNetworkWrite},
    error::ConnectionError,
    network::Incoming,
};

use super::{AsyncMqttNetworkRead};

#[derive(Debug)]
pub struct TcpReader {
    readhalf: OwnedReadHalf,

    /// Buffered reads
    buffer: BytesMut,
}

impl TcpReader {
    pub async fn new_tcp(
        options: &ConnectOptions,
    ) -> Result<(TcpReader, TcpWriter), ConnectionError> {
        let (readhalf, writehalf) = TcpStream::connect((options.address.clone(), options.port))
            .await?
            .into_split();
        let reader = TcpReader {
            readhalf,
            buffer: BytesMut::with_capacity(20 * 1024),
            // max_incoming_size: u32::MAX as usize,
        };
        let writer = TcpWriter::new(writehalf);
        Ok((reader, writer))
    }

    pub async fn read(&mut self) -> io::Result<Packet> {
        loop {
            let (header, header_length) = match FixedHeader::read_fixed_header(self.buffer.iter()) {
                Ok(header) => header,
                Err(ReadBytes::InsufficientBytes(required_len)) => {
                    self.read_bytes(required_len).await?;
                    continue;
                }
                Err(ReadBytes::Err(err)) => return Err(Error::new(ErrorKind::InvalidData, err)),
            };

            self.buffer.advance(header_length);

            if header.remaining_length > self.buffer.len() {
                self.read_bytes(header.remaining_length - self.buffer.len())
                    .await?;
            }

            let buf = self.buffer.split_to(header.remaining_length);

            return Packet::read(header, buf.into())
                .map_err(|err| Error::new(ErrorKind::InvalidData, err));
        }
    }

    /// Reads more than 'required' bytes to frame a packet into self.read buffer
    async fn read_bytes(&mut self, required: usize) -> io::Result<usize> {
        let mut total_read = 0;

        loop {
            #[cfg(feature = "tokio")]
            let read = self.readhalf.read_buf(&mut self.buffer).await?;
            // #[cfg(feature = "smol")]
            // let read = self.connection.read(&mut self.buffer).await?;
            if 0 == read {
                return if self.buffer.is_empty() {
                    Err(io::Error::new(
                        io::ErrorKind::ConnectionAborted,
                        "Connection closed by peer",
                    ))
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::ConnectionReset,
                        "Connection reset by peer",
                    ))
                };
            }

            total_read += read;
            if total_read >= required {
                return Ok(total_read);
            }
        }
    }
}

impl AsyncMqttNetworkRead for TcpReader {
    type W = TcpWriter;

    fn connect(
        options: &ConnectOptions,
    ) -> impl std::future::Future<Output = Result<(Self, Self::W, Packet), ConnectionError>> + Send + '_
    {
        async {
            let (mut reader, mut writer) = TcpReader::new_tcp(options).await?;

            let mut buf_out = BytesMut::new();

            create_connect_from_options(options).write(&mut buf_out)?;

            writer.write_buffer(&mut buf_out).await?;

            let packet = reader.read().await?;
            if let Packet::ConnAck(con) = packet {
                if con.reason_code == ConnAckReasonCode::Success {
                    Ok((reader, writer, Packet::ConnAck(con)))
                } else {
                    Err(ConnectionError::ConnectionRefused(con.reason_code))
                }
            } else {
                Err(ConnectionError::NotConnAck(packet))
            }
        }
    }

    async fn read(&mut self) -> Result<Packet, ConnectionError> {
        Ok(self.read().await?)
    }

    async fn read_direct(
        &mut self,
        incoming_packet_sender: &async_channel::Sender<Incoming>,
    ) -> Result<bool, ConnectionError> {
        let mut read_packets = 0;
        loop {
            let (header, header_length) = match FixedHeader::read_fixed_header(self.buffer.iter()) {
                Ok(header) => header,
                Err(ReadBytes::InsufficientBytes(required_len)) => {
                    self.read_bytes(required_len).await?;
                    continue;
                }
                Err(ReadBytes::Err(err)) => return Err(ConnectionError::DeserializationError(err)),
            };

            self.buffer.advance(header_length);

            if header.remaining_length > self.buffer.len() {
                self.read_bytes(header.remaining_length - self.buffer.len())
                    .await?;
            }

            let buf = self.buffer.split_to(header.remaining_length);
            let read_packet = Packet::read(header, buf.into())?;
            tracing::trace!("Read packet from network {}", read_packet);
            let disconnect = read_packet.packet_type() == PacketType::Disconnect;
            incoming_packet_sender.send(read_packet).await?;
            if disconnect {
                return Ok(true);
            }
            read_packets += 1;
            if read_packets >= 10 {
                return Ok(false);
            }
        }
    }
}


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
