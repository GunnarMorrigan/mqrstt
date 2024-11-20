use bytes::BufMut;

use crate::packets::{
    error::DeserializeError,
    mqtt_trait::{MqttRead, MqttWrite, WireLength},
    PacketType, PropertyType, VariableInteger,
};

crate::packets::macros::define_properties!(DisconnectProperties, SessionExpiryInterval, ReasonString, UserProperty, ServerReference);

impl MqttRead for DisconnectProperties {
    fn read(buf: &mut bytes::Bytes) -> Result<Self, DeserializeError> {
        let (len, _) = VariableInteger::read_variable_integer(buf).map_err(DeserializeError::from)?;

        let mut properties = Self::default();
        if len == 0 {
            return Ok(properties);
        } else if buf.len() < len {
            return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), buf.len(), len));
        }

        let mut property_data = buf.split_to(len);

        loop {
            match PropertyType::try_from(u8::read(&mut property_data)?)? {
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
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::packets::error::SerializeError> {
        self.wire_len().write_variable_integer(buf)?;

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
