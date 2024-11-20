use bytes::BufMut;

use crate::packets::error::{DeserializeError};
use crate::packets::mqtt_trait::{MqttRead, MqttWrite, WireLength};
use crate::packets::{PacketType, PropertyType, VariableInteger};

crate::packets::macros::define_properties!(UnsubAckProperties, ReasonString, UserProperty);

// #[derive(Debug, Default, PartialEq, Eq, Clone)]
// pub struct UnsubAckProperties {
//     /// 3.11.2.1.2 Reason String
//     /// 31 (0x1F) Byte, Identifier of the Reason String.
//     pub reason_string: Option<Box<str>>,

//     pub user_properties: Vec<(Box<str>, Box<str>)>,
// }

impl MqttRead for UnsubAckProperties {
    fn read(buf: &mut bytes::Bytes) -> Result<Self, DeserializeError> {
        let (len, _) = VariableInteger::read_variable_integer(buf)?;

        let mut properties = UnsubAckProperties::default();

        if len == 0 {
            return Ok(properties);
        } else if buf.len() < len {
            return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), buf.len(), len));
        }

        let mut properties_data = buf.split_to(len);

        loop {
            match PropertyType::read(&mut properties_data)? {
                PropertyType::ReasonString => {
                    if properties.reason_string.is_none() {
                        properties.reason_string = Some(Box::<str>::read(&mut properties_data)?);
                    } else {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::SubscriptionIdentifier));
                    }
                }
                PropertyType::UserProperty => {
                    properties.user_properties.push((Box::<str>::read(&mut properties_data)?, Box::<str>::read(&mut properties_data)?));
                }
                e => return Err(DeserializeError::UnexpectedProperty(e, PacketType::UnsubAck)),
            }

            if buf.is_empty() {
                break;
            }
        }
        Ok(properties)
    }
}

impl MqttWrite for UnsubAckProperties {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::packets::error::SerializeError> {
        self.wire_len().write_variable_integer(buf)?;
        if let Some(reason_string) = &self.reason_string {
            PropertyType::ReasonString.write(buf)?;
            reason_string.write(buf)?;
        }
        for (key, value) in &self.user_properties {
            PropertyType::UserProperty.write(buf)?;
            key.write(buf)?;
            value.write(buf)?;
        }
        Ok(())
    }
}

// impl WireLength for UnsubAckProperties {
//     fn wire_len(&self) -> usize {
//         let mut len = 0;
//         if let Some(reason_string) = &self.reason_string {
//             len += 1 + reason_string.wire_len();
//         }
//         for (key, value) in &self.user_properties {
//             len += 1 + key.wire_len() + value.wire_len();
//         }
//         len
//     }
// }
