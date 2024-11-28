use crate::packets::error::DeserializeError;
use crate::packets::mqtt_trait::{MqttRead, MqttWrite, WireLength};
use crate::packets::{PacketType, PropertyType, VariableInteger};

crate::packets::macros::define_properties!(
    /// UnsubAck Properties
    UnsubAckProperties,
    ReasonString,
    UserProperty
);

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

            if properties_data.is_empty() {
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
