use std::io::{self, Error, ErrorKind};

use async_channel::Receiver;

use bytes::{Buf, BytesMut};
use rustls::ServerName;

use tokio::io::{ReadHalf, AsyncReadExt, WriteHalf, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;
use tracing::trace;

use crate::error::TlsError;
use crate::{packets::{
    error::ReadBytes,
    packets::{FixedHeader, Packet, PacketType},
    reason_codes::ConnAckReasonCode,
}, connections::{AsyncMqttNetworkRead, AsyncMqttNetworkWrite}};
use crate::{
    connect_options::ConnectOptions,
    connections::{create_connect_from_options},
    error::ConnectionError,
    network::Incoming,
};

use super::transport::{TlsConfig, RustlsConfig};
use super::util::simple_rust_tls;

#[derive(Debug)]
pub struct TlsReader {
    readhalf: ReadHalf<TlsStream<TcpStream>>,

    /// Buffered reads
    buffer: BytesMut,
}

impl TlsReader {
    pub async fn new(
        options: &ConnectOptions,
    ) -> Result<(TlsReader, TlsWriter), TlsError> {
        if let Some(tls_config) = &options.tls_config{
            let arc_tls_config = match tls_config{
                TlsConfig::Rustls(RustlsConfig::Simple { ca, alpn, client_auth }) => simple_rust_tls(ca.clone(), alpn.clone(), client_auth.clone())?,
                TlsConfig::Rustls(RustlsConfig::Rustls(config)) => config.clone(),
            };

            let domain = ServerName::try_from(options.address.as_str())?;
            let connector = TlsConnector::from(arc_tls_config);
                        
            let stream = TcpStream::connect((options.address.as_str(), options.port)).await?;
            let connection = connector.connect(domain, stream).await?;

            let (readhalf, writehalf) = tokio::io::split(connection);

            let reader = Self {
                readhalf,
                buffer: BytesMut::with_capacity(20 * 1024),
            };
            let writer = TlsWriter::new(writehalf);

        Ok((reader, writer))

        }
        else{
            Err(TlsError::NoTlsConfig)
        }
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
            let read = self.readhalf.read_buf(&mut self.buffer).await?;

            if read == 0 {
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

impl AsyncMqttNetworkRead for TlsReader {
    type W = TlsWriter;

    fn connect(
        options: &ConnectOptions,
    ) -> impl std::future::Future<Output = Result<(Self, Self::W, Packet), ConnectionError>> + Send + '_
    {
        async {
            let (mut reader, mut writer) = TlsReader::new(options).await?;

            let mut buf_out = BytesMut::new();

            create_connect_from_options(options).write(&mut buf_out)?;

            writer.write_buffer(&mut buf_out).await?;

            if !buf_out.is_empty(){
                panic!("Should be empty");
            }

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

#[derive(Debug)]
pub struct TlsWriter {
    writehalf: WriteHalf<TlsStream<TcpStream>>,
    buffer: BytesMut,
}

impl TlsWriter {
    pub fn new(writehalf: WriteHalf<TlsStream<TcpStream>>) -> Self {
        Self {
            writehalf,
            buffer: BytesMut::with_capacity(20 * 1024),
        }
    }
}

impl AsyncMqttNetworkWrite for TlsWriter {
    async fn write_buffer(&mut self, buffer: &mut BytesMut) -> Result<(), ConnectionError> {
        if buffer.is_empty() {
            return Ok(());
        }

        self.writehalf.write_all(&buffer[..]).await?;
        self.writehalf.flush().await?;
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


#[cfg(test)]
mod test{
    use crate::{connections::{AsyncMqttNetworkRead, transport::{TlsConfig, RustlsConfig}}, tests::resources::{EMQX_CERT}, connect_options::ConnectOptions, packets::{packets::{PacketType, Packet}, reason_codes::ConnAckReasonCode}};

    use super::TlsReader;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn connect_emqx_test(){

        let config = TlsConfig::Rustls(RustlsConfig::Simple {
            ca: EMQX_CERT.to_vec(),
            alpn: None,
            client_auth: None,
        });
        
        let opt = ConnectOptions::new_with_tls_config("broker.emqx.io".to_string(), 8883, "test123123".to_string(), Some(config));

        let (_, _, packet) = TlsReader::connect(&opt).await.unwrap();

        assert_eq!(PacketType::ConnAck, packet.packet_type());
        if let Packet::ConnAck(conn) = packet{
            assert_eq!(ConnAckReasonCode::Success, conn.reason_code);
        }
    }

}