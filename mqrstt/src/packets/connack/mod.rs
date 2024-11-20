mod properties;
pub use properties::ConnAckProperties;

mod reason_code;
pub use reason_code::ConnAckReasonCode;


use super::{
    error::{DeserializeError, SerializeError},
    mqtt_trait::{MqttAsyncRead, MqttRead, MqttWrite, PacketAsyncRead, PacketRead, PacketWrite, WireLength}, VariableInteger,
};
use bytes::{Buf, BufMut};


#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ConnAck {
    /// 3.2.2.1 Connect Acknowledge Flags
    pub connack_flags: ConnAckFlags,

    /// 3.2.2.2 Connect Reason Code
    /// Byte 2 in the Variable Header is the Connect Reason Code.
    pub reason_code: ConnAckReasonCode,

    /// 3.2.2.3 CONNACK Properties
    pub connack_properties: ConnAckProperties,
}

impl PacketRead for ConnAck {
    fn read(_: u8, header_len: usize, mut buf: bytes::Bytes) -> Result<Self, DeserializeError> {
        if header_len > buf.len() {
            return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), buf.len(), header_len));
        }

        let connack_flags = ConnAckFlags::read(&mut buf)?;
        let reason_code = ConnAckReasonCode::read(&mut buf)?;
        let connack_properties = ConnAckProperties::read(&mut buf)?;

        Ok(Self {
            connack_flags,
            reason_code,
            connack_properties,
        })
    }
}

impl<S> PacketAsyncRead<S> for ConnAck where S: tokio::io::AsyncReadExt + Unpin {
    fn async_read(_: u8, remaining_length: usize, stream: &mut S) -> impl std::future::Future<Output = Result<(Self, usize), super::error::ReadError>> {
        async move {
            let (connack_flags, read_bytes) = ConnAckFlags::async_read(stream).await?;
            let (reason_code, reason_code_read_bytes) = ConnAckReasonCode::async_read(stream).await?;
            let (connack_properties, connack_properties_read_bytes) = ConnAckProperties::async_read(stream).await?;
    
            Ok((
                Self {
                    connack_flags,
                    reason_code,
                    connack_properties,
                },
                read_bytes + reason_code_read_bytes + connack_properties_read_bytes
            ))
    
        }
    }
}

impl PacketWrite for ConnAck {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), SerializeError> {
        self.connack_flags.write(buf)?;
        self.reason_code.write(buf)?;
        self.connack_properties.write(buf)?;

        Ok(())
    }
}

impl WireLength for ConnAck {
    fn wire_len(&self) -> usize {
        2 + // 1 for connack_flags and 1 for reason_code 
        self.connack_properties.wire_len().variable_integer_len() +
        self.connack_properties.wire_len()
    }
}


#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub struct ConnAckFlags {
    pub session_present: bool,
}

impl<S> MqttAsyncRead<S> for ConnAckFlags where S: tokio::io::AsyncReadExt + Unpin {
    fn async_read(stream: &mut S) -> impl std::future::Future<Output = Result<(Self, usize), super::error::ReadError>> {
        async move {
            let byte = stream.read_u8().await?;
            Ok((Self {
                session_present: (byte & 0b00000001) == 0b00000001,
            }, 1))
        }
    }
}

impl MqttRead for ConnAckFlags {
    fn read(buf: &mut bytes::Bytes) -> Result<Self, DeserializeError> {
        if buf.is_empty() {
            return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), 0, 1));
        }

        let byte = buf.get_u8();

        Ok(Self {
            session_present: (byte & 0b00000001) == 0b00000001,
        })
    }
}

impl MqttWrite for ConnAckFlags {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
        let byte = self.session_present as u8;

        buf.put_u8(byte);
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use crate::packets::{
        connack::{ConnAck, ConnAckProperties}, mqtt_trait::{MqttRead, MqttWrite, PacketRead, PacketWrite, WireLength}, ConnAckReasonCode, Packet, VariableInteger
    };

    #[test]
    fn test_wire_len() {
        let mut buf = bytes::BytesMut::new();

        let connack_properties = ConnAckProperties {
            session_expiry_interval: Some(60),  // Session expiry interval in seconds
            receive_maximum: Some(20),  // Maximum number of QoS 1 and QoS 2 publications that the client is willing to process concurrently
            maximum_qos: Some(crate::packets::QoS::AtMostOnce),  // Maximum QoS level supported by the server
            retain_available: Some(true),  // Whether the server supports retained messages
            maximum_packet_size: Some(1024),  // Maximum packet size the server is willing to accept
            assigned_client_id: Some(Box::from("client-12345")),  // Client identifier assigned by the server
            topic_alias_maximum: Some(10),  // Maximum number of topic aliases supported by the server
            reason_string: Some(Box::from("Connection accepted")),  // Reason string for the connection acknowledgment
            user_properties: vec![(Box::from("key1"), Box::from("value1"))],  // User property key-value pair
            wildcards_available: Some(true),  // Whether wildcard subscriptions are available
            subscription_ids_available: Some(true),  // Whether subscription identifiers are available
            shared_subscription_available: Some(true),  // Whether shared subscriptions are available
            server_keep_alive: Some(120),  // Server keep alive time in seconds
            response_info: Some(Box::from("Response info")),  // Response information
            server_reference: Some(Box::from("server-reference")),  // Server reference
            authentication_method: Some(Box::from("auth-method")),  // Authentication method
            authentication_data: Some(vec![1, 2, 3, 4]),  // Authentication data
        };

        let len = connack_properties.wire_len();
        // determine length of variable integer
        let len_of_wire_len = len.write_variable_integer(&mut buf).unwrap();
        // clear buffer before writing actual properties
        buf.clear();
        connack_properties.write(&mut buf).unwrap();

        assert_eq!(len + len_of_wire_len, buf.len());
    
    }

    #[test]
    fn read_write_connack_packet() {
        let c = ConnAck { ..Default::default() };

        let p1 = Packet::ConnAck(c);
        let mut buf = bytes::BytesMut::new();

        p1.write(&mut buf).unwrap();

        let p2 = Packet::read_from_buffer(&mut buf).unwrap();

        assert_eq!(p1, p2);
    }

    #[test]
    fn read_write_connack() {
        let mut buf = bytes::BytesMut::new();
        let packet = &[
            0x01, // Connack flags
            0x00, // Reason code,
            0x00, // empty properties
        ];

        buf.extend_from_slice(packet);
        let c1 = ConnAck::read(0, packet.len(), buf.into()).unwrap();

        assert_eq!(ConnAckReasonCode::Success, c1.reason_code);
        assert_eq!(ConnAckProperties::default(), c1.connack_properties);

        let mut buf = bytes::BytesMut::new();

        c1.write(&mut buf).unwrap();

        let c2 = ConnAck::read(0, packet.len(), buf.into()).unwrap();

        assert_eq!(c1, c2)
    }

    #[test]
    fn read_write_connack_properties() {
        let mut buf = bytes::BytesMut::new();
        let packet = &[
            56, // ConnAckProperties variable length
            17, // session_expiry_interval
            0xff, 0xff, 37,  // retain_available
            0x1, // true
            18,  // Assigned Client Id
            0, 11, // 11 bytes "KeanuReeves" without space
            b'K', b'e', b'a', b'n', b'u', b'R', b'e', b'e', b'v', b'e', b's', 36, // Max QoS
            2,  // QoS 2 Exactly Once
            34, // Topic Alias Max = 255
            0, 255, 31, // Reason String = 'Houston we have got a problem'
            0, 29, b'H', b'o', b'u', b's', b't', b'o', b'n', b' ', b'w', b'e', b' ', b'h', b'a', b'v', b'e', b' ', b'g', b'o', b't', b' ', b'a', b' ', b'p', b'r', b'o', b'b', b'l', b'e', b'm',
        ];

        buf.extend_from_slice(packet);
        let c1 = ConnAckProperties::read(&mut buf.into()).unwrap();

        let mut buf = bytes::BytesMut::new();

        let variable_length = c1.wire_len();
        assert_eq!(variable_length, 56);

        c1.write(&mut buf).unwrap();

        let _buf_clone = buf.to_vec();

        let c2 = ConnAckProperties::read(&mut buf.into()).unwrap();

        assert_eq!(c1, c2);
    }
}
