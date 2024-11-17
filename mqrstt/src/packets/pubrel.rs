use bytes::BufMut;

use super::{
    error::{DeserializeError, ReadError}, mqtt_traits::{MqttAsyncRead, MqttRead, MqttWrite, PacketAsyncRead, PacketRead, PacketWrite, WireLength}, read_async_variable_integer, read_variable_integer, reason_codes::PubRelReasonCode, write_variable_integer, PacketType, PropertyType
};

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

impl<S> PacketAsyncRead<S> for PubRel where S: tokio::io::AsyncReadExt + Unpin {
    async fn async_read(_: u8, remaining_length: usize, stream: &mut S) -> Result<(Self, usize), ReadError> {
        let mut total_read_bytes = 0;
        let (packet_identifier, read_bytes) = u16::async_read(stream).await?;
        total_read_bytes += read_bytes;
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

// #[derive(Debug, PartialEq, Eq, Clone, Hash, Default)]
// pub struct PubRelProperties {
//     pub reason_string: Option<Box<str>>,
//     pub user_properties: Vec<(Box<str>, Box<str>)>,
// }

super::macros::define_properties!(PubRelProperties, ReasonString, UserProperty);

impl PubRelProperties {
    pub fn is_empty(&self) -> bool {
        self.reason_string.is_none() && self.user_properties.is_empty()
    }
}

impl MqttRead for PubRelProperties {
    fn read(buf: &mut bytes::Bytes) -> Result<Self, super::error::DeserializeError> {
        let (len, _) = read_variable_integer(buf)?;

        if len == 0 {
            return Ok(Self::default());
        }
        if buf.len() < len {
            return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), buf.len(), len));
        }

        let mut properties = PubRelProperties::default();

        loop {
            match PropertyType::try_from(u8::read(buf)?)? {
                PropertyType::ReasonString => {
                    if properties.reason_string.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::ReasonString));
                    }
                    properties.reason_string = Some(Box::<str>::read(buf)?);
                }
                PropertyType::UserProperty => properties.user_properties.push((Box::<str>::read(buf)?, Box::<str>::read(buf)?)),
                e => return Err(DeserializeError::UnexpectedProperty(e, PacketType::PubRel)),
            }
            if buf.is_empty() {
                break;
            }
        }
        Ok(properties)
    }
}

// impl<S> MqttAsyncRead<S> for PubRelProperties where S: tokio::io::AsyncReadExt + Unpin {
//     async fn async_read(stream: &mut S) -> Result<(Self, usize), super::error::ReadError> {
//         let (len, length_variable_integer) = read_async_variable_integer(stream).await?;
//         if len == 0 {
//             return Ok((Self::default(), length_variable_integer));
//         }

//         let mut properties = PubRelProperties::default();

//         let mut read_property_bytes = 0;
//         loop {
//             let (prop, read_bytes) = PropertyType::async_read(stream).await?;
//             read_property_bytes += read_bytes;
//             match prop {
//                 PropertyType::ReasonString => {
//                     if properties.reason_string.is_some() {
//                         return Err(super::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(PropertyType::ReasonString)));
//                     }
//                     let (prop_body, read_bytes) = Box::<str>::async_read(stream).await?;
//                     read_property_bytes += read_bytes;
//                     properties.reason_string = Some(prop_body);
//                 }
//                 PropertyType::UserProperty => {
//                     let (prop_body_key, read_bytes) = Box::<str>::async_read(stream).await?;
//                     read_property_bytes += read_bytes;
//                     let (prop_body_value, read_bytes) = Box::<str>::async_read(stream).await?;
//                     read_property_bytes += read_bytes;
                
//                     properties.user_properties.push((prop_body_key, prop_body_value))
//                 },
//                 e => return Err(super::error::ReadError::DeserializeError(DeserializeError::UnexpectedProperty(e, PacketType::PubRel))),
//             }
//             if read_property_bytes == len {
//                 break;
//             }
//         }

//         Ok((properties, length_variable_integer + read_property_bytes))
//     }
// }

impl MqttWrite for PubRelProperties {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
        let len = self.wire_len();

        write_variable_integer(buf, len)?;

        if let Some(reason_string) = &self.reason_string {
            PropertyType::ReasonString.write(buf)?;
            reason_string.write(buf)?;
        }
        for (key, value) in &self.user_properties {
            PropertyType::UserProperty.write(buf)?;
            key.write(buf)?;
            value.write(buf)?
        }

        Ok(())
    }
}

// impl WireLength for PubRelProperties {
//     fn wire_len(&self) -> usize {
//         let mut len = 0;
//         if let Some(reason_string) = &self.reason_string {
//             len += reason_string.wire_len() + 1;
//         }
//         for (key, value) in &self.user_properties {
//             len += 1 + key.wire_len() + value.wire_len();
//         }

//         len
//     }
// }

#[cfg(test)]
mod tests {
    use crate::packets::{
        mqtt_traits::{MqttAsyncRead, MqttRead, MqttWrite, PacketAsyncRead, PacketRead, PacketWrite, WireLength},
        pubrel::{PubRel, PubRelProperties},
        reason_codes::PubRelReasonCode,
        write_variable_integer, PropertyType,
    };
    use bytes::{Buf, BufMut, Bytes, BytesMut};
    use tokio::{io::ReadBuf, stream};

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

        let prop = PubRelProperties{
            reason_string: Some("reason string, test 1-2-3.".into()), // 26 + 1 + 2
            user_properties: vec![
                ("This is the key".into(), "This is the value".into()), // 32 + 1 + 2 + 2
                ("Another thingy".into(), "The thingy".into()), // 24 + 1 + 2 + 2
            ],
        };

        let len = prop.wire_len();
        // determine length of variable integer
        let len_of_wire_len = write_variable_integer(&mut buf, len).unwrap();
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

        write_variable_integer(&mut buf, properties.len()).unwrap();

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

        write_variable_integer(&mut buf, properties.len()).unwrap();

        buf.extend(properties);


        let mut stream = &*buf;
        // flags can be 0 because not used.
        // remaining_length must be at least 4
        let (p_ack, read_bytes) = PubRel::async_read(0, buf.len(), &mut stream).await.unwrap();

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
        write_variable_integer(&mut buf, properties_data.len()).unwrap();
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
        write_variable_integer(&mut buf, properties_data.len()).unwrap();
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
