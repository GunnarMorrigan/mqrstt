use std::io::{self, ErrorKind, Error};

use bytes::{BytesMut, Buf};
use tokio::{net::TcpStream, io::{AsyncWriteExt, AsyncReadExt}};
use tracing::debug;

use crate::{state::State, error::ConnectionError, connect_options::ConnectOptions, network::Incoming};
use crate::packets::{connect::Connect, error::ReadBytes, packets::{Packet, FixedHeader}};

use super::AsyncMqttNetwork;

pub struct Tcp{
    connection: TcpStream,
    /// Buffered reads
    buffer: BytesMut,
    /// Maximum packet size
    max_incoming_size: usize,
}

impl Tcp{
    pub async fn new_tcp(options: &ConnectOptions) -> Self{
        let connection = TcpStream::connect((options.address.clone(), options.port)).await.unwrap();
        Tcp{
            connection,
            buffer: BytesMut::with_capacity(u32::MAX as usize),
            max_incoming_size: u32::MAX as usize,
        }
    }

    pub async fn read(&mut self) -> io::Result<Packet>{
        loop {
            let (header, header_length) = match FixedHeader::read_fixed_header(self.buffer.iter()) {
                Ok(header) => header,
                Err(ReadBytes::InsufficientBytes(required_len)) => {
                    self.read_bytes(required_len).await?; 
                    continue;
                },
                Err(ReadBytes::Err(err)) => return Err(Error::new(ErrorKind::InvalidData, err)),
            };

            self.buffer.advance(header_length);

            if header.remaining_length > self.buffer.len(){
                self.read_bytes(header.remaining_length - self.buffer.len()).await?;
            }

            let buf = self.buffer.split_to(header.remaining_length);

            return Packet::read(header, buf.into()).map_err(|err| Error::new(ErrorKind::InvalidData, err));
        }
    }
    
    /// Reads more than 'required' bytes to frame a packet into self.read buffer
    async fn read_bytes(&mut self, required: usize) -> io::Result<usize> {
        let mut total_read = 0;
        loop {
            let read = self.connection.read_buf(&mut self.buffer).await?;
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

    pub async fn connect(&mut self, options: &ConnectOptions){
        let mut connect = Connect::default();

        connect.client_id = options.client_id.clone();
        connect.clean_session = options.clean_session;
        connect.keep_alive = options.keep_alive_interval_s as u16;
        connect.connect_properties.request_problem_information = Some(1u8);
        connect.connect_properties.request_response_information = Some(1u8);

        let packet = Packet::Connect(connect);

        let mut buf_out = BytesMut::new();

        packet.write(&mut buf_out).unwrap();

        self.connection.write_buf(&mut buf_out).await.unwrap();
    }
}


impl AsyncMqttNetwork for Tcp{
    fn connect(options: &crate::connect_options::ConnectOptions) -> impl std::future::Future<Output = Result<(Self, Packet), ConnectionError>> + Send + '_ {
        async{
            
            let mut tcp = Tcp::new_tcp(options).await;
            debug!("Created TCP connection");
            tcp.connect(options).await;
            debug!("Send MQTT Connect packet");
            let packet = tcp.read().await?;
            if packet.is_connack(){
                debug!("Received ConnAck");
                Ok((tcp, packet))
            }
            else{
                Err(ConnectionError::NotConnAck(packet))
            }
        }
    }

    async fn read(&mut self) -> Result<Packet, ConnectionError> {
        Ok(self.read().await?)
    }

    async fn read_many(&mut self, incoming_packet_sender: &mut async_channel::Sender<Incoming>) -> Result<(), ConnectionError> {
        let mut read_packets = 0;
        loop {
            let (header, header_length) = match FixedHeader::read_fixed_header(self.buffer.iter()) {
                Ok(header) => header,
                Err(ReadBytes::InsufficientBytes(required_len)) => {
                    self.read_bytes(required_len).await?;
                    continue;
                },
                Err(ReadBytes::Err(err)) => return Err(ConnectionError::DeserializationError(err)),
            };

            self.buffer.advance(header_length);

            if header.remaining_length > self.buffer.len(){
                self.read_bytes(header.remaining_length - self.buffer.len()).await?;
            }

            let buf = self.buffer.split_to(header.remaining_length);
            let read_packet = Packet::read(header, buf.into())?;
            tracing::trace!("Read packet from network {:?}", read_packet);
            incoming_packet_sender.send(read_packet).await?;
            read_packets += 1;
            if read_packets >= 10{
                return Ok(());
            }
        }
    }

    async fn write(&mut self, write_buf: &mut BytesMut) -> Result<(), ConnectionError> {
        if write_buf.is_empty(){
            return Ok(());
        }

        self.connection.write_all(&write_buf[..]).await?;
        write_buf.clear();
        Ok(())
    }
}






#[cfg(test)]
mod tests{
    use bytes::{BytesMut, Buf, Bytes};

    use crate::packets::QoS;
    use crate::packets::disconnect::{DisconnectProperties, Disconnect};
    use crate::packets::connack::{ConnAck, ConnAckFlags, ConnAckProperties};

    use crate::packets::publish::{Publish, PublishProperties};
    use crate::packets::reason_codes::{ConAckReasonCode, DisconnectReasonCode};
    use crate::packets::error::{ReadBytes, DeserializeError};
    use crate::packets::packets::{FixedHeader, Packet};

    pub fn read(buffer: &mut BytesMut) -> Result<Packet, ReadBytes<DeserializeError>>{
        let (header, header_length) = FixedHeader::read_fixed_header(buffer.iter())?;
    
        if header.remaining_length > buffer.len(){
            return Err(ReadBytes::InsufficientBytes(header.remaining_length - buffer.len()));
        }
        buffer.advance(header_length);
        
        let buf = buffer.split_to(header.remaining_length);
    
        return Ok(Packet::read(header, buf.into())?);
    }

    #[test]
    fn test_connack_read(){
        let connack = [0x20, 0x13, 0x01, 0x00, 0x10, 0x27, 0x00, 0x10, 0x00, 0x00, 0x25, 0x01, 0x2a, 0x01, 0x29, 0x01,
        0x22, 0xff, 0xff, 0x28, 0x01];
        let mut buf = BytesMut::new();
        buf.extend(connack);
        
        let res = read(&mut buf);
        assert!(res.is_ok());
        let res = res.unwrap();

        let expected = ConnAck {
            connack_flags: ConnAckFlags::SESSION_PRESENT,
            reason_code: ConAckReasonCode::Success,
            connack_properties: ConnAckProperties {
                session_expiry_interval: None,
                receive_maximum: None,
                maximum_qos: None,
                retain_available: Some(
                    true,
                ),
                maximum_packet_size: Some(
                    1048576,
                ),
                assigned_client_id: None,
                topic_alias_maximum: Some(
                    65535,
                ),
                reason_string: None,
                user_properties: vec![],
                wildcards_available: Some(
                    true,
                ),
                subscription_ids_available: Some(
                    true,
                ),
                shared_subscription_available: Some(
                    true,
                ),
                server_keep_alive: None,
                response_info: None,
                server_reference: None,
                authentication_method: None,
                authentication_data: None,
            },
        };

        assert_eq!(Packet::ConnAck(expected), res);
    }

    #[test]
    fn test_disconnect_read(){
        let packet = [0xe0, 0x02, 0x8e, 0x00];
        let mut buf = BytesMut::new();
        buf.extend(packet);
        
        let res = read(&mut buf);
        assert!(res.is_ok());
        let res = res.unwrap();

        let expected = Disconnect {
            reason_code: DisconnectReasonCode::SessionTakenOver,
            properties: DisconnectProperties {
                session_expiry_interval: None,
                reason_string: None,
                user_properties: vec![],
                server_reference: None,
            },
        };

        assert_eq!(Packet::Disconnect(expected), res);
    }

    #[test]
    fn test_pingreq_read(){
        let packet = [0xc0, 0x00];
        let mut buf = BytesMut::new();
        buf.extend(packet);
        
        let res = read(&mut buf);
        assert!(res.is_ok());
        let res = res.unwrap();

        assert_eq!(Packet::PingReq, res);
    }

    #[test]
    fn test_pingresp_read(){
        let packet = [0xd0, 0x00];
        let mut buf = BytesMut::new();
        buf.extend(packet);
        
        let res = read(&mut buf);
        assert!(res.is_ok());
        let res = res.unwrap();

        assert_eq!(Packet::PingResp, res);
    }

    #[test]
    fn test_publish_read(){
        let packet = [0x35, 0x24, 0x00, 0x14, 0x74, 0x65, 0x73, 0x74, 0x2f, 0x31, 0x32, 0x33, 0x2f, 0x74, 0x65, 0x73,
        0x74, 0x2f, 0x62, 0x6c, 0x61, 0x62, 0x6c, 0x61, 0x35, 0xd3, 0x0b, 0x01, 0x01, 0x09, 0x00, 0x04,
        0x31, 0x32, 0x31, 0x32, 0x0b, 0x01];

        let mut buf = BytesMut::new();
        buf.extend(packet);
        
        let res = read(&mut buf);
        assert!(res.is_ok());
        let res = res.unwrap();

        let expected = Publish {
            dup: false,
            qos: QoS::ExactlyOnce,
            retain: true,
            topic: "test/123/test/blabla".to_string(),
            packet_identifier: Some(
                13779,
            ),
            publish_properties: PublishProperties {
                payload_format_indicator: Some(
                    1,
                ),
                message_expiry_interval: None,
                topic_alias: None,
                response_topic: None,
                correlation_data: Some(
                    Bytes::from_static(b"1212"),
                ),
                subscription_identifier: vec![1],
                user_properties: vec![],
                content_type: None,
            },
            payload: Bytes::from_static(b""),
        };

        assert_eq!(Packet::Publish(expected), res);
    }
    

}

