mod reason_code;
pub use reason_code::PubCompReasonCode;

mod properties;
pub use properties::PubCompProperties;

use super::{
    error::DeserializeError,
    mqtt_trait::{MqttAsyncRead, MqttRead, MqttWrite, PacketAsyncRead, PacketRead, PacketWrite, WireLength},
};
use bytes::BufMut;
use tokio::io::AsyncReadExt;

/// The PUBCOMP Packet is the response to a PUBLISH Packet with QoS 2.
/// It is the fourth and final packet of the QoS 2 protocol exchange.
/// The user of the client application does not have to send this packet, it is handled internally by the client.
///
/// Both the client and server can send this packet.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct PubComp {
    pub packet_identifier: u16,
    pub reason_code: PubCompReasonCode,
    pub properties: PubCompProperties,
}

impl PubComp {
    pub fn new(packet_identifier: u16) -> Self {
        Self {
            packet_identifier,
            reason_code: PubCompReasonCode::Success,
            properties: PubCompProperties::default(),
        }
    }
}

impl PacketRead for PubComp {
    fn read(_: u8, remaining_length: usize, mut buf: bytes::Bytes) -> Result<Self, DeserializeError> {
        // reason code and properties are optional if reasoncode is success and properties empty.
        if remaining_length == 2 {
            return Ok(Self {
                packet_identifier: u16::read(&mut buf)?,
                reason_code: PubCompReasonCode::Success,
                properties: PubCompProperties::default(),
            });
        }
        // Requires u16, u8 and at leasy 1 byte of variable integer prop length so at least 4 bytes
        else if remaining_length < 4 {
            return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), buf.len(), 4));
        }

        let packet_identifier = u16::read(&mut buf)?;
        let reason_code = PubCompReasonCode::read(&mut buf)?;
        let properties = PubCompProperties::read(&mut buf)?;

        Ok(Self {
            packet_identifier,
            reason_code,
            properties,
        })
    }
}

impl<S> PacketAsyncRead<S> for PubComp
where
    S: tokio::io::AsyncRead + Unpin,
{
    fn async_read(_: u8, remaining_length: usize, stream: &mut S) -> impl std::future::Future<Output = Result<(Self, usize), crate::packets::error::ReadError>> {
        async move {
            let packet_identifier = stream.read_u16().await?;
            if remaining_length == 2 {
                return Ok((
                    Self {
                        packet_identifier,
                        reason_code: PubCompReasonCode::Success,
                        properties: PubCompProperties::default(),
                    },
                    2,
                ));
            }
            // Requires u16, u8 and at leasy 1 byte of variable integer prop length so at least 4 bytes
            else if remaining_length < 4 {
                return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), 0, 4).into());
            }

            let (reason_code, reason_code_read_bytes) = PubCompReasonCode::async_read(stream).await?;
            let (properties, properties_read_bytes) = PubCompProperties::async_read(stream).await?;

            assert_eq!(2 + reason_code_read_bytes + properties_read_bytes, remaining_length);

            Ok((
                Self {
                    packet_identifier,
                    reason_code,
                    properties,
                },
                2 + reason_code_read_bytes + properties_read_bytes,
            ))
        }
    }
}

impl PacketWrite for PubComp {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
        buf.put_u16(self.packet_identifier);

        if self.reason_code == PubCompReasonCode::Success && self.properties.reason_string.is_none() && self.properties.user_properties.is_empty() {
            // nothing here
        } else if self.properties.reason_string.is_none() && self.properties.user_properties.is_empty() {
            self.reason_code.write(buf)?;
        } else {
            self.reason_code.write(buf)?;
            self.properties.write(buf)?;
        }
        Ok(())
    }
}

impl WireLength for PubComp {
    fn wire_len(&self) -> usize {
        if self.reason_code == PubCompReasonCode::Success && self.properties.reason_string.is_none() && self.properties.user_properties.is_empty() {
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
        mqtt_trait::{MqttRead, MqttWrite, PacketRead, PacketWrite, WireLength},
        pubcomp::{PubComp, PubCompProperties},
        PropertyType, PubCompReasonCode, VariableInteger,
    };
    use bytes::{BufMut, Bytes, BytesMut};

    #[test]
    fn test_wire_len() {
        let mut pubcomp = PubComp {
            packet_identifier: 12,
            reason_code: PubCompReasonCode::Success,
            properties: PubCompProperties::default(),
        };

        let mut buf = BytesMut::new();

        pubcomp.write(&mut buf).unwrap();

        assert_eq!(2, pubcomp.wire_len());
        assert_eq!(2, buf.len());

        pubcomp.reason_code = PubCompReasonCode::PacketIdentifierNotFound;
        buf.clear();
        pubcomp.write(&mut buf).unwrap();

        assert_eq!(3, pubcomp.wire_len());
        assert_eq!(3, buf.len());
    }

    #[test]
    fn test_read_simple_pub_comp() {
        let stream = &[
            0x00, 0x0C, // Packet identifier = 12
            0x00, // Reason code success
            0x00, // no properties
        ];
        let buf = Bytes::from(&stream[..]);
        let p_ack = PubComp::read(0, 4, buf).unwrap();

        let expected = PubComp {
            packet_identifier: 12,
            reason_code: PubCompReasonCode::Success,
            properties: PubCompProperties::default(),
        };

        assert_eq!(expected, p_ack);
    }

    #[test]
    fn test_read_write_pub_comp_with_properties() {
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

        let p_ack = PubComp::read(0, buf.len(), buf.clone().into()).unwrap();

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

        let properties = PubCompProperties::read(&mut buf.clone().into()).unwrap();
        let mut result = BytesMut::new();
        properties.write(&mut result).unwrap();

        assert_eq!(buf.to_vec(), result.to_vec());
    }

    #[test]
    fn no_reason_code_or_props() {
        let mut buf = BytesMut::new();

        buf.put_u16(65_535u16);
        let p_ack = PubComp::read(0, buf.len(), buf.clone().into()).unwrap();

        let mut result = BytesMut::new();
        p_ack.write(&mut result).unwrap();

        let expected = PubComp {
            packet_identifier: 65535,
            reason_code: PubCompReasonCode::Success,
            properties: PubCompProperties::default(),
        };
        let mut result = BytesMut::new();
        expected.write(&mut result).unwrap();

        assert_eq!(expected, p_ack);
        assert_eq!(buf.to_vec(), result.to_vec());
    }
}
