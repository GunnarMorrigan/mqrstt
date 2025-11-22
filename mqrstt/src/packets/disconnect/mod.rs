mod properties;
pub use properties::DisconnectProperties;

mod reason_code;
pub use reason_code::DisconnectReasonCode;

use super::{
    VariableInteger,
    mqtt_trait::{MqttAsyncRead, MqttRead, MqttWrite, PacketAsyncRead, PacketRead, PacketWrite, WireLength},
};

/// The DISCONNECT Packet is the final packet.
///  The client sends this packet to the server to disconnect for example on calling [`crate::MqttClient::disconnect`].
/// The server can send a disconnect packet to the client to indicate that the connection is being closed.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Disconnect {
    pub reason_code: DisconnectReasonCode,
    pub properties: DisconnectProperties,
}

impl<S> PacketAsyncRead<S> for Disconnect
where
    S: tokio::io::AsyncRead + Unpin,
{
    async fn async_read(_: u8, remaining_length: usize, stream: &mut S) -> Result<(Self, usize), crate::packets::error::ReadError> {
        if remaining_length == 0 {
            Ok((
                Self {
                    reason_code: DisconnectReasonCode::NormalDisconnection,
                    properties: DisconnectProperties::default(),
                },
                0,
            ))
        } else {
            let (reason_code, reason_code_read_bytes) = DisconnectReasonCode::async_read(stream).await?;
            let (properties, properties_read_bytes) = DisconnectProperties::async_read(stream).await?;

            Ok((Self { reason_code, properties }, reason_code_read_bytes + properties_read_bytes))
        }
    }
}

impl PacketRead for Disconnect {
    fn read(_: u8, remaining_length: usize, mut buf: bytes::Bytes) -> Result<Self, super::error::DeserializeError> {
        let reason_code;
        let properties;
        if remaining_length == 0 {
            reason_code = DisconnectReasonCode::NormalDisconnection;
            properties = DisconnectProperties::default();
        } else {
            reason_code = DisconnectReasonCode::read(&mut buf)?;
            properties = DisconnectProperties::read(&mut buf)?;
        }

        Ok(Self { reason_code, properties })
    }
}
impl PacketWrite for Disconnect {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
        if self.reason_code != DisconnectReasonCode::NormalDisconnection || self.properties.wire_len() != 0 {
            self.reason_code.write(buf)?;
            self.properties.write(buf)?;
        }
        Ok(())
    }
}

impl<S> crate::packets::mqtt_trait::PacketAsyncWrite<S> for Disconnect
where
    S: tokio::io::AsyncWrite + Unpin,
{
    fn async_write(&self, stream: &mut S) -> impl std::future::Future<Output = Result<usize, crate::packets::error::WriteError>> {
        use crate::packets::mqtt_trait::MqttAsyncWrite;
        async move {
            let mut total_written_bytes = 0;
            if self.reason_code != DisconnectReasonCode::NormalDisconnection || self.properties.wire_len() != 0 {
                total_written_bytes += self.reason_code.async_write(stream).await?;
                total_written_bytes += self.properties.async_write(stream).await?;
            }
            Ok(total_written_bytes)
        }
    }
}

impl WireLength for Disconnect {
    fn wire_len(&self) -> usize {
        if self.reason_code != DisconnectReasonCode::NormalDisconnection || self.properties.wire_len() != 0 {
            let property_len = self.properties.wire_len();
            // reasoncode, length of property length, property length
            1 + property_len.variable_integer_len() + property_len
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_write_and_async_read_disconnect() {
        let mut buf = bytes::BytesMut::new();
        let packet = Disconnect {
            properties: DisconnectProperties {
                session_expiry_interval: Some(123),
                reason_string: Some(Box::from("Some reason")),
                user_properties: vec![(Box::from("key1"), Box::from("value1")), (Box::from("key2"), Box::from("value2"))],
                server_reference: Some(Box::from("Server reference")),
            },
            reason_code: DisconnectReasonCode::NormalDisconnection,
        };

        packet.write(&mut buf).unwrap();

        let mut stream = &*buf;

        let (read_packet, read_bytes) = Disconnect::async_read(0, buf.len(), &mut stream).await.unwrap();

        assert_eq!(buf.len(), read_bytes);
        assert_eq!(read_packet.properties.session_expiry_interval, Some(123));
        assert_eq!(read_packet.properties.reason_string, Some(Box::from("Some reason")));
        assert_eq!(
            read_packet.properties.user_properties,
            vec![(Box::from("key1"), Box::from("value1")), (Box::from("key2"), Box::from("value2")),]
        );
        assert_eq!(read_packet.properties.server_reference, Some(Box::from("Server reference")));
    }

    #[test]
    fn test_write_and_read_disconnect() {
        let mut buf = bytes::BytesMut::new();
        let packet = Disconnect {
            properties: DisconnectProperties {
                session_expiry_interval: Some(123),
                reason_string: Some(Box::from("Some reason")),
                user_properties: vec![(Box::from("key1"), Box::from("value1")), (Box::from("key2"), Box::from("value2"))],
                server_reference: Some(Box::from("Server reference")),
            },
            reason_code: DisconnectReasonCode::NormalDisconnection,
        };

        packet.write(&mut buf).unwrap();

        let read_packet = Disconnect::read(0, buf.len(), buf.into()).unwrap();

        assert_eq!(read_packet.properties.session_expiry_interval, Some(123));
        assert_eq!(read_packet.properties.reason_string, Some(Box::from("Some reason")));
        assert_eq!(
            read_packet.properties.user_properties,
            vec![(Box::from("key1"), Box::from("value1")), (Box::from("key2"), Box::from("value2")),]
        );
        assert_eq!(read_packet.properties.server_reference, Some(Box::from("Server reference")));
    }

    #[test]
    fn test_write_and_read_disconnect_properties() {
        let mut buf = bytes::BytesMut::new();
        let properties = DisconnectProperties {
            session_expiry_interval: Some(123),
            reason_string: Some(Box::from("Some reason")),
            user_properties: vec![(Box::from("key1"), Box::from("value1")), (Box::from("key2"), Box::from("value2"))],
            server_reference: Some(Box::from("Server reference")),
        };

        properties.write(&mut buf).unwrap();

        let read_properties = DisconnectProperties::read(&mut buf.into()).unwrap();

        assert_eq!(read_properties.session_expiry_interval, Some(123));
        assert_eq!(read_properties.reason_string, Some(Box::from("Some reason")));
        assert_eq!(
            read_properties.user_properties,
            vec![(Box::from("key1"), Box::from("value1")), (Box::from("key2"), Box::from("value2")),]
        );
        assert_eq!(read_properties.server_reference, Some(Box::from("Server reference")));
    }
}
