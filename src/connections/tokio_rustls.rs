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

use super::transport::TlsConfig;
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
            match tls_config{
                TlsConfig::Simple { ca, alpn, client_auth } => {
                    let connector = TlsConnector::from(simple_rust_tls(ca.clone(), alpn.clone(), client_auth.clone())?);
                    let domain = ServerName::try_from(options.address.as_str())?;
                    let stream = TcpStream::connect((options.address.as_str(), options.port)).await?;

                    trace!("Connecting TLS");
                    let connection = connector.connect(domain, stream).await?;
                    trace!("Connected TLS");
            
                    let (readhalf, writehalf) = tokio::io::split(connection);
            
                    let reader = Self {
                        readhalf,
                        buffer: BytesMut::with_capacity(20 * 1024),
                    };
                    let writer = TlsWriter::new(writehalf);
            
                    Ok((reader, writer))
                },
                _ => unreachable!(),
            }
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
        let mut counter = 0;
        loop {
            let read = self.readhalf.read(&mut self.buffer).await?;
            counter += 1;

            if counter == 10 {
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

    // async fn write(&mut self, outgoing: &Receiver<Packet>) -> Result<bool, ConnectionError> {
    //     let mut disconnect = false;

    //     let packet = outgoing.recv().await?;
    //     packet.write(&mut self.buffer)?;
    //     if packet.packet_type() == PacketType::Disconnect {
    //         disconnect = true;
    //     }

    //     while !outgoing.is_empty() && !disconnect {
    //         let packet = outgoing.recv().await?;
    //         packet.write(&mut self.buffer)?;
    //         if packet.packet_type() == PacketType::Disconnect {
    //             disconnect = true;
    //             break;
    //         }
    //         trace!("Going to write packet to network: {:?}", packet);
    //     }

    //     self.writehalf.write_all(&self.buffer[..]).await?;
    //     self.writehalf.flush().await?;
    //     self.buffer.clear();
    //     Ok(disconnect)
    // }
}


#[cfg(test)]
mod test{
    use tokio::{net::TcpStream, io::{AsyncWriteExt, AsyncReadExt}};
    use tokio_rustls::TlsConnector;
    use bytes::BytesMut;
    use rustls::ServerName;
    use tracing::{trace, Level};
    use tracing_subscriber::FmtSubscriber;

    use crate::{connections::{util::simple_rust_tls, create_connect_from_options, AsyncMqttNetworkRead, transport::TlsConfig, AsyncMqttNetworkWrite}, tests::resources::{google, EMQX}, connect_options::ConnectOptions, packets::packets::Packet};

    use super::TlsReader;

    #[test]
    fn connect_google_test(){
        let fut = async {

            let addr = "google.com";

            let config = simple_rust_tls(google.to_vec(), None, None).unwrap();
            let connector = TlsConnector::from(config);
            let domain = ServerName::try_from(addr).unwrap();
            let stream = TcpStream::connect((addr, 443)).await.unwrap();

            trace!("Connecting TLS");
            let connection = connector.connect(domain, stream).await.unwrap();
            trace!("Connected TLS");
        };

        smol::block_on(fut)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn connect_emqx_custom_test(){
        let subscriber = FmtSubscriber::builder()
        // .with_env_filter(filter)
            .with_max_level(Level::TRACE)
            .with_line_number(true)
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");


        let fut = async {
            trace!("Starting test");

            let addr = "broker.emqx.io";

            let config = simple_rust_tls(EMQX.to_vec(), None, None).unwrap();
            let connector = TlsConnector::from(config);
            let domain = ServerName::try_from(addr).unwrap();
            let stream = TcpStream::connect((addr, 8883)).await.unwrap();

            trace!("Connecting TLS");
            let connection = connector.connect(domain, stream).await.unwrap();
            trace!("Connected TLS");

            let opt = ConnectOptions::new("broker.emqx.io".to_string(), 8883, "test123123".to_string(), None);

            let mut buf_out = BytesMut::new();
            let mut buf_in = BytesMut::with_capacity(20*1024); // Using buf_in for reading also breaks everything

            let mut vec_in = Vec::<u8>::with_capacity(20*1024);

            create_connect_from_options(&opt).write(&mut buf_out).unwrap();

            let (mut read, mut write) = tokio::io::split(connection);

            write.write_all(&buf_out).await.unwrap();
            write.flush().await.unwrap();

            // buf_out.clear(); Brakes everything

            println!("{:?}", buf_out);

            let mut len = 0;
            loop {
                // len += read.read_to_end(&mut vec_in).await.unwrap();
                len += read.read(&mut buf_in).await.unwrap();
                
                if len != 0{
                    println!("Len: {}", len);
                    break;
                }
            }
            println!("{:?}", buf_in);
            
            println!("{:?}", Packet::read_from_buffer(&mut buf_in));
        };

        fut.await;
    }


    #[test]
    fn connect_emqx_less_custom_test(){
        let subscriber = FmtSubscriber::builder()
        // .with_env_filter(filter)
            .with_max_level(Level::TRACE)
            .with_line_number(true)
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");


        let fut = async {
            trace!("Starting test");

            let config = TlsConfig::Simple{
                ca: EMQX.to_vec(),
                alpn: None,
                client_auth: None,
            };

            let opt = ConnectOptions::new("broker.emqx.io".to_string(), 8883, "test123123".to_string(), Some(config));

            let mut buf_out = BytesMut::new();


            let (mut reader, mut writer) = TlsReader::new(&opt).await.unwrap();

            create_connect_from_options(&opt).write(&mut buf_out).unwrap();

            writer.write_buffer(&mut buf_out).await.unwrap();

            writer.writehalf.write_all(&buf_out).await.unwrap();
            buf_out.clear();
            
            println!("{:?}", buf_out);

            let packet = reader.read().await.unwrap();

            println!("{}", packet);
        };

        smol::block_on(fut)
    }



    #[test]
    fn connect_emqx_test(){

        let config = TlsConfig::Simple{
            ca: EMQX.to_vec(),
            alpn: None,
            client_auth: None,
        };

        let opt = ConnectOptions::new("broker.emqx.io".to_string(), 8883, "test123123".to_string(), Some(config));

        smol::block_on(TlsReader::connect(&opt)).unwrap();
    }

}