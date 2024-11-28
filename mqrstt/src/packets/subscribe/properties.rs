use crate::packets::{
    error::DeserializeError,
    mqtt_trait::{MqttRead, MqttWrite, WireLength},
    PacketType, PropertyType, VariableInteger,
};

crate::packets::macros::define_properties!(
    /// Subscribe Properties
    SubscribeProperties,
    SubscriptionIdentifier,
    UserProperty
);

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
