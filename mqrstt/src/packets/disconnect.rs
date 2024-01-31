use bytes::BufMut;

use super::{
    error::DeserializeError,
    mqtt_traits::{MqttRead, MqttWrite, VariableHeaderRead, VariableHeaderWrite, WireLength},
    read_variable_integer,
    reason_codes::DisconnectReasonCode,
    variable_integer_len, write_variable_integer, PacketType, PropertyType,
};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Disconnect {
    pub reason_code: DisconnectReasonCode,
    pub properties: DisconnectProperties,
}

impl VariableHeaderRead for Disconnect {
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
impl VariableHeaderWrite for Disconnect {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
        if self.reason_code != DisconnectReasonCode::NormalDisconnection || self.properties.wire_len() != 0 {
            self.reason_code.write(buf)?;
            self.properties.write(buf)?;
        }
        Ok(())
    }
}
impl WireLength for Disconnect {
    fn wire_len(&self) -> usize {
        if self.reason_code != DisconnectReasonCode::NormalDisconnection || self.properties.wire_len() != 0 {
            let property_len = self.properties.wire_len();
            // reasoncode, length of property length, property length
            1 + variable_integer_len(property_len) + property_len
        } else {
            0
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DisconnectProperties {
    pub session_expiry_interval: Option<u32>,
    pub reason_string: Option<Box<str>>,
    pub user_properties: Vec<(Box<str>, Box<str>)>,
    pub server_reference: Option<Box<str>>,
}

impl MqttRead for DisconnectProperties {
    fn read(buf: &mut bytes::Bytes) -> Result<Self, DeserializeError> {
        let (len, _) = read_variable_integer(buf).map_err(DeserializeError::from)?;

        let mut properties = Self::default();
        if len == 0 {
            return Ok(properties);
        } else if buf.len() < len {
            return Err(DeserializeError::InsufficientData("DisconnectProperties".to_string(), buf.len(), len));
        }

        let mut property_data = buf.split_to(len);

        loop {
            match PropertyType::from_u8(u8::read(&mut property_data)?)? {
                PropertyType::SessionExpiryInterval => {
                    if properties.session_expiry_interval.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::SessionExpiryInterval));
                    }
                    properties.session_expiry_interval = Some(u32::read(&mut property_data)?);
                }
                PropertyType::ReasonString => {
                    if properties.reason_string.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::ReasonString));
                    }
                    properties.reason_string = Some(Box::<str>::read(&mut property_data)?);
                }
                PropertyType::ServerReference => {
                    if properties.server_reference.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::ServerReference));
                    }
                    properties.server_reference = Some(Box::<str>::read(&mut property_data)?);
                }
                PropertyType::UserProperty => properties.user_properties.push((Box::<str>::read(&mut property_data)?, Box::<str>::read(&mut property_data)?)),
                e => return Err(DeserializeError::UnexpectedProperty(e, PacketType::Disconnect)),
            }

            if property_data.is_empty() {
                break;
            }
        }

        Ok(properties)
    }
}

impl MqttWrite for DisconnectProperties {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
        write_variable_integer(buf, self.wire_len())?;

        if let Some(session_expiry_interval) = self.session_expiry_interval {
            PropertyType::SessionExpiryInterval.write(buf)?;
            buf.put_u32(session_expiry_interval);
        }
        if let Some(reason_string) = &self.reason_string {
            PropertyType::ReasonString.write(buf)?;
            reason_string.write(buf)?;
        }
        for (key, val) in self.user_properties.iter() {
            PropertyType::UserProperty.write(buf)?;
            key.write(buf)?;
            val.write(buf)?;
        }
        if let Some(server_refrence) = &self.server_reference {
            PropertyType::ServerReference.write(buf)?;
            server_refrence.write(buf)?;
        }
        Ok(())
    }
}

impl WireLength for DisconnectProperties {
    fn wire_len(&self) -> usize {
        let mut len = 0;
        if self.session_expiry_interval.is_some() {
            len += 4 + 1;
        }
        if let Some(reason_string) = &self.reason_string {
            len += reason_string.wire_len() + 1;
        }
        len += self.user_properties.iter().fold(0, |acc, (k, v)| acc + k.wire_len() + v.wire_len() + 1);
        if let Some(server_refrence) = &self.server_reference {
            len += server_refrence.wire_len() + 1;
        }
        len
    }
}

#[cfg(test)]
mod tests {
    use super::*;


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_and_read_disconnect() {
        let mut buf = bytes::BytesMut::new();
        let packet = Disconnect {
            properties: DisconnectProperties {
                session_expiry_interval: Some(123),
                reason_string: Some(Box::from("Some reason")),
                user_properties: vec![
                    (Box::from("key1"), Box::from("value1")),
                    (Box::from("key2"), Box::from("value2")),
                ],
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
            vec![
                (Box::from("key1"), Box::from("value1")),
                (Box::from("key2"), Box::from("value2")),
            ]
        );
        assert_eq!(
            read_packet.properties.server_reference,
            Some(Box::from("Server reference"))
        );
    }
}


    #[test]
    fn test_write_and_read_disconnect_properties() {
        let mut buf = bytes::BytesMut::new();
        let properties = DisconnectProperties {
            session_expiry_interval: Some(123),
            reason_string: Some(Box::from("Some reason")),
            user_properties: vec![
                (Box::from("key1"), Box::from("value1")),
                (Box::from("key2"), Box::from("value2")),
            ],
            server_reference: Some(Box::from("Server reference")),
        };

        properties.write(&mut buf).unwrap();

        let read_properties = DisconnectProperties::read(&mut buf.into()).unwrap();

        assert_eq!(read_properties.session_expiry_interval, Some(123));
        assert_eq!(read_properties.reason_string, Some(Box::from("Some reason")));
        assert_eq!(
            read_properties.user_properties,
            vec![
                (Box::from("key1"), Box::from("value1")),
                (Box::from("key2"), Box::from("value2")),
            ]
        );
        assert_eq!(
            read_properties.server_reference,
            Some(Box::from("Server reference"))
        );
    }
}