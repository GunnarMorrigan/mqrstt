mod reason_code;
pub use reason_code::PubRelReasonCode;

mod properties;
pub use properties::PubRelProperties;

use bytes::BufMut;
use tokio::io::AsyncReadExt;

use super::{
    error::{DeserializeError, ReadError},
    mqtt_trait::{MqttAsyncRead, MqttRead, MqttWrite, PacketAsyncRead, PacketRead, PacketWrite, WireLength},
};

/// The [`PubRel`] (Publish Release) packet acknowledges the reception of a [`crate::packets::PubRec`] Packet.
///
/// This user does not need to send this message, it is handled internally by the client.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct PubRel {
    pub packet_identifier: u16,
    pub reason_code: PubRelReasonCode,
    pub properties: PubRelProperties,
}

impl PubRel {
    pub fn new(packet_identifier: u16) -> Self {
        Self {
            packet_identifier,
            reason_code: PubRelReasonCode::Success,
            properties: PubRelProperties::default(),
        }
    }
}

impl PacketRead for PubRel {
    fn read(_: u8, remaining_length: usize, mut buf: bytes::Bytes) -> Result<Self, DeserializeError> {
        // reason code and properties are optional if reasoncode is success and properties empty.
        if remaining_length == 2 {
            Ok(Self {
                packet_identifier: u16::read(&mut buf)?,
                reason_code: PubRelReasonCode::Success,
                properties: PubRelProperties::default(),
            })
        } else if remaining_length == 3 {
            Ok(Self {
                packet_identifier: u16::read(&mut buf)?,
                reason_code: PubRelReasonCode::read(&mut buf)?,
                properties: PubRelProperties::default(),
            })
        } else {
            Ok(Self {
                packet_identifier: u16::read(&mut buf)?,
                reason_code: PubRelReasonCode::read(&mut buf)?,
                properties: PubRelProperties::read(&mut buf)?,
            })
        }
    }
}

impl<S> PacketAsyncRead<S> for PubRel
where
    S: tokio::io::AsyncRead + Unpin,
{
    async fn async_read(_: u8, remaining_length: usize, stream: &mut S) -> Result<(Self, usize), ReadError> {
        let mut total_read_bytes = 0;
        let packet_identifier = stream.read_u16().await?;
        total_read_bytes += 2;
        let res = if remaining_length == 2 {
            Self {
                packet_identifier,
                reason_code: PubRelReasonCode::Success,
                properties: PubRelProperties::default(),
            }
        } else {
            let (reason_code, read_bytes) = PubRelReasonCode::async_read(stream).await?;
            total_read_bytes += read_bytes;
            if remaining_length == 3 {
                Self {
                    packet_identifier,
                    reason_code,
                    properties: PubRelProperties::default(),
                }
            } else {
                let (properties, read_bytes) = PubRelProperties::async_read(stream).await?;
                total_read_bytes += read_bytes;
                Self {
                    packet_identifier,
                    reason_code,
                    properties,
                }
            }
        };
        Ok((res, total_read_bytes))
    }
}

impl PacketWrite for PubRel {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
        buf.put_u16(self.packet_identifier);

        if self.reason_code == PubRelReasonCode::Success && self.properties.reason_string.is_none() && self.properties.user_properties.is_empty() {
            // Nothing here
        } else if self.properties.reason_string.is_none() && self.properties.user_properties.is_empty() {
            self.reason_code.write(buf)?;
        } else {
            self.reason_code.write(buf)?;
            self.properties.write(buf)?;
        }
        Ok(())
    }
}
impl<S> crate::packets::mqtt_trait::PacketAsyncWrite<S> for PubRel
where
    S: tokio::io::AsyncWrite + Unpin,
{
    fn async_write(&self, stream: &mut S) -> impl std::future::Future<Output = Result<usize, crate::packets::error::WriteError>> {
        use crate::packets::mqtt_trait::MqttAsyncWrite;
        async move {
            let mut total_writen_bytes = 2;
            self.packet_identifier.async_write(stream).await?;

            if self.reason_code == PubRelReasonCode::Success && self.properties.reason_string.is_none() && self.properties.user_properties.is_empty() {
                return Ok(total_writen_bytes);
            } else if self.properties.reason_string.is_none() && self.properties.user_properties.is_empty() {
                total_writen_bytes += self.reason_code.async_write(stream).await?;
            } else {
                total_writen_bytes += self.reason_code.async_write(stream).await?;
                total_writen_bytes += self.properties.async_write(stream).await?;
            }
            Ok(total_writen_bytes)
        }
    }
}

impl WireLength for PubRel {
    fn wire_len(&self) -> usize {
        if self.reason_code == PubRelReasonCode::Success && self.properties.reason_string.is_none() && self.properties.user_properties.is_empty() {
            2
        } else if self.properties.reason_string.is_none() && self.properties.user_properties.is_empty() {
            3
        } else {
            2 + 1 + self.properties.wire_len()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::packets::{
        mqtt_trait::{MqttAsyncRead, MqttRead, MqttWrite, PacketAsyncRead, PacketRead, PacketWrite, WireLength},
        pubrel::{PubRel, PubRelProperties},
        PropertyType, PubRelReasonCode, VariableInteger,
    };
    use bytes::{BufMut, Bytes, BytesMut};

    #[test]
    fn test_wire_len() {
        let mut pubrel = PubRel {
            packet_identifier: 12,
            reason_code: PubRelReasonCode::Success,
            properties: PubRelProperties::default(),
        };

        let mut buf = BytesMut::new();

        pubrel.write(&mut buf).unwrap();

        assert_eq!(2, pubrel.wire_len());
        assert_eq!(2, buf.len());

        pubrel.reason_code = PubRelReasonCode::PacketIdentifierNotFound;
        buf.clear();
        pubrel.write(&mut buf).unwrap();

        assert_eq!(3, pubrel.wire_len());
        assert_eq!(3, buf.len());
    }

    #[test]
    fn test_wire_len2() {
        let mut buf = BytesMut::new();

        let prop = PubRelProperties {
            reason_string: Some("reason string, test 1-2-3.".into()), // 26 + 1 + 2
            user_properties: vec![
                ("This is the key".into(), "This is the value".into()), // 32 + 1 + 2 + 2
                ("Another thingy".into(), "The thingy".into()),         // 24 + 1 + 2 + 2
            ],
        };

        let len = prop.wire_len();
        // determine length of variable integer
        let len_of_wire_len = len.write_variable_integer(&mut buf).unwrap();
        // clear buffer before writing actual properties
        buf.clear();
        prop.write(&mut buf).unwrap();

        assert_eq!(len + len_of_wire_len, buf.len());
    }

    #[test]
    fn test_read_short() {
        let mut expected_pubrel = PubRel {
            packet_identifier: 12,
            reason_code: PubRelReasonCode::Success,
            properties: PubRelProperties::default(),
        };

        let mut buf = BytesMut::new();

        expected_pubrel.write(&mut buf).unwrap();

        assert_eq!(2, buf.len());

        let pubrel = PubRel::read(0, 2, buf.into()).unwrap();

        assert_eq!(expected_pubrel, pubrel);

        let mut buf = BytesMut::new();
        expected_pubrel.reason_code = PubRelReasonCode::PacketIdentifierNotFound;
        expected_pubrel.write(&mut buf).unwrap();

        assert_eq!(3, buf.len());

        let pubrel = PubRel::read(0, 3, buf.into()).unwrap();
        assert_eq!(expected_pubrel, pubrel);
    }

    #[tokio::test]
    async fn test_async_read_short() {
        let mut expected_pubrel = PubRel {
            packet_identifier: 12,
            reason_code: PubRelReasonCode::Success,
            properties: PubRelProperties::default(),
        };

        let mut buf = BytesMut::new();

        expected_pubrel.write(&mut buf).unwrap();

        assert_eq!(2, buf.len());
        let mut stream: &[u8] = &*buf;

        let (pubrel, read_bytes) = PubRel::async_read(0, 2, &mut stream).await.unwrap();

        assert_eq!(expected_pubrel, pubrel);
        assert_eq!(read_bytes, 2);

        let mut buf = BytesMut::new();
        expected_pubrel.reason_code = PubRelReasonCode::PacketIdentifierNotFound;
        expected_pubrel.write(&mut buf).unwrap();

        assert_eq!(3, buf.len());
        let mut stream: &[u8] = &*buf;

        let (pubrel, read_bytes) = PubRel::async_read(0, 3, &mut stream).await.unwrap();
        assert_eq!(read_bytes, 3);
        assert_eq!(expected_pubrel, pubrel);
    }

    #[test]
    fn test_read_simple_pub_rel() {
        let stream = &[
            0x00, 0x0C, // Packet identifier = 12
            0x00, // Reason code success
            0x00, // no properties
        ];
        let buf = Bytes::from(&stream[..]);
        let p_ack = PubRel::read(0, 4, buf).unwrap();

        let expected = PubRel {
            packet_identifier: 12,
            reason_code: PubRelReasonCode::Success,
            properties: PubRelProperties::default(),
        };

        assert_eq!(expected, p_ack);
    }
    #[tokio::test]
    async fn test_async_read_simple_pub_rel() {
        let stream = &[
            0x00, 0x0C, // Packet identifier = 12
            0x00, // Reason code success
            0x00, // no properties
        ];

        let mut stream = stream.as_ref();

        let (p_ack, read_bytes) = PubRel::async_read(0, 4, &mut stream).await.unwrap();

        let expected = PubRel {
            packet_identifier: 12,
            reason_code: PubRelReasonCode::Success,
            properties: PubRelProperties::default(),
        };

        assert_eq!(expected, p_ack);
        assert_eq!(read_bytes, 4);
    }

    #[test]
    fn test_read_write_pubrel_with_properties() {
        let mut buf = BytesMut::new();

        buf.put_u16(65_535u16);
        buf.put_u8(0x92);

        let mut properties = BytesMut::new();
        PropertyType::ReasonString.write(&mut properties).unwrap();
        "reason string, test 1-2-3.".write(&mut properties).unwrap();
        PropertyType::UserProperty.write(&mut properties).unwrap();
        "This is the key".write(&mut properties).unwrap();
        "This is the value".write(&mut properties).unwrap();
        PropertyType::UserProperty.write(&mut properties).unwrap();
        "Another thingy".write(&mut properties).unwrap();
        "The thingy".write(&mut properties).unwrap();

        properties.len().write_variable_integer(&mut buf).unwrap();

        buf.extend(properties);

        // flags can be 0 because not used.
        // remaining_length must be at least 4
        let p_ack = PubRel::read(0, buf.len(), buf.clone().into()).unwrap();

        let mut result = BytesMut::new();
        p_ack.write(&mut result).unwrap();

        assert_eq!(buf.to_vec(), result.to_vec());
    }

    #[tokio::test]
    async fn test_async_read_write_pubrel_with_properties() {
        let mut buf = BytesMut::new();

        buf.put_u16(65_535u16);
        buf.put_u8(0x92);

        let mut properties = BytesMut::new();
        PropertyType::ReasonString.write(&mut properties).unwrap();
        "reason string, test 1-2-3.".write(&mut properties).unwrap();
        PropertyType::UserProperty.write(&mut properties).unwrap();
        "This is the key".write(&mut properties).unwrap();
        "This is the value".write(&mut properties).unwrap();
        PropertyType::UserProperty.write(&mut properties).unwrap();
        "Another thingy".write(&mut properties).unwrap();
        "The thingy".write(&mut properties).unwrap();

        properties.len().write_variable_integer(&mut buf).unwrap();

        buf.extend(properties);

        let mut stream = &*buf;
        // flags can be 0 because not used.
        // remaining_length must be at least 4
        let (p_ack, _) = PubRel::async_read(0, buf.len(), &mut stream).await.unwrap();

        let mut result = BytesMut::new();
        p_ack.write(&mut result).unwrap();

        assert_eq!(buf.to_vec(), result.to_vec());
    }

    #[test]
    fn test_properties() {
        let mut properties_data = BytesMut::new();
        PropertyType::ReasonString.write(&mut properties_data).unwrap();
        "reason string, test 1-2-3.".write(&mut properties_data).unwrap();
        PropertyType::UserProperty.write(&mut properties_data).unwrap();
        "This is the key".write(&mut properties_data).unwrap();
        "This is the value".write(&mut properties_data).unwrap();
        PropertyType::UserProperty.write(&mut properties_data).unwrap();
        "Another thingy".write(&mut properties_data).unwrap();
        "The thingy".write(&mut properties_data).unwrap();

        let mut buf = BytesMut::new();
        properties_data.len().write_variable_integer(&mut buf).unwrap();
        buf.extend(properties_data);

        let properties = PubRelProperties::read(&mut buf.clone().into()).unwrap();
        let mut result = BytesMut::new();
        properties.write(&mut result).unwrap();

        assert_eq!(buf.to_vec(), result.to_vec());
    }

    #[tokio::test]
    async fn test_async_read_properties() {
        let mut properties_data = BytesMut::new();
        PropertyType::ReasonString.write(&mut properties_data).unwrap();
        "reason string, test 1-2-3.".write(&mut properties_data).unwrap();
        PropertyType::UserProperty.write(&mut properties_data).unwrap();
        "This is the key".write(&mut properties_data).unwrap();
        "This is the value".write(&mut properties_data).unwrap();
        PropertyType::UserProperty.write(&mut properties_data).unwrap();
        "Another thingy".write(&mut properties_data).unwrap();
        "The thingy".write(&mut properties_data).unwrap();

        let mut buf = BytesMut::new();
        properties_data.len().write_variable_integer(&mut buf).unwrap();
        buf.extend(properties_data);

        let (properties, read_bytes) = PubRelProperties::async_read(&mut &*buf).await.unwrap();
        let mut result = BytesMut::new();
        properties.write(&mut result).unwrap();

        assert_eq!(buf.to_vec(), result.to_vec());
        assert_eq!(buf.len(), read_bytes);
    }

    #[test]
    fn no_reason_code_or_props() {
        let mut buf = BytesMut::new();

        buf.put_u16(65_535u16);
        let p_ack = PubRel::read(0, buf.len(), buf.clone().into()).unwrap();

        let mut result = BytesMut::new();
        p_ack.write(&mut result).unwrap();

        let expected = PubRel {
            packet_identifier: 65535,
            reason_code: PubRelReasonCode::Success,
            properties: PubRelProperties::default(),
        };
        let mut result = BytesMut::new();
        expected.write(&mut result).unwrap();

        assert_eq!(expected, p_ack);
        assert_eq!(buf.to_vec(), result.to_vec());
    }
}
