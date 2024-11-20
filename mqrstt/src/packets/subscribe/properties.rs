

use crate::{error::PacketValidationError, util::constants::MAXIMUM_TOPIC_SIZE};

use crate::packets::{
    error::DeserializeError,
    mqtt_trait::{MqttRead, MqttWrite, WireLength},
    PacketType, PropertyType, VariableInteger,
};

crate::packets::macros::define_properties!(
    SubscribeProperties,
    SubscriptionIdentifier,
    UserProperty
);

// #[derive(Debug, Default, PartialEq, Eq, Clone)]
// pub struct SubscribeProperties {
//     /// 3.8.2.1.2 Subscription Identifier
//     /// 11 (0x0B) Byte, Identifier of the Subscription Identifier.
//     pub subscription_id: Option<usize>,

//     /// 3.8.2.1.3 User Property
//     /// 38 (0x26) Byte, Identifier of the User Property.
//     pub user_properties: Vec<(Box<str>, Box<str>)>,
// }

impl MqttRead for SubscribeProperties {
    fn read(buf: &mut bytes::Bytes) -> Result<Self, crate::packets::error::DeserializeError> {
        let (len, _) = VariableInteger::read_variable_integer(buf)?;

        let mut properties = SubscribeProperties::default();

        if len == 0 {
            return Ok(properties);
        } else if buf.len() < len {
            return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), buf.len(), len));
        }

        let mut properties_data = buf.split_to(len);

        loop {
            match PropertyType::read(&mut properties_data)? {
                PropertyType::SubscriptionIdentifier => {
                    if properties.subscription_identifier.is_none() {
                        let (subscription_id, _) = VariableInteger::read_variable_integer(&mut properties_data)?;

                        properties.subscription_identifier = Some(subscription_id);
                    } else {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::SubscriptionIdentifier));
                    }
                }
                PropertyType::UserProperty => {
                    properties.user_properties.push((Box::<str>::read(&mut properties_data)?, Box::<str>::read(&mut properties_data)?));
                }
                e => return Err(DeserializeError::UnexpectedProperty(e, PacketType::Subscribe)),
            }

            if properties_data.is_empty() {
                break;
            }
        }
        Ok(properties)
    }
}

impl MqttWrite for SubscribeProperties {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::packets::error::SerializeError> {
        self.wire_len().write_variable_integer(buf)?;
        if let Some(sub_id) = self.subscription_identifier {
            PropertyType::SubscriptionIdentifier.write(buf)?;
            sub_id.write_variable_integer(buf)?;
        }
        for (key, value) in &self.user_properties {
            PropertyType::UserProperty.write(buf)?;
            key.write(buf)?;
            value.write(buf)?;
        }
        Ok(())
    }
}

// impl WireLength for SubscribeProperties {
//     fn wire_len(&self) -> usize {
//         let mut len = 0;
//         if let Some(sub_id) = self.subscription_identifier {
//             len += 1 + sub_id.variable_integer_len();
//         }
//         for (key, value) in &self.user_properties {
//             len += 1 + key.wire_len() + value.wire_len();
//         }
//         len
//     }
// }