use bytes::Bytes;

use crate::packets::{
    error::DeserializeError,
    mqtt_trait::{MqttRead, MqttWrite, WireLength},
    PacketType, PropertyType, VariableInteger,
};

crate::packets::macros::define_properties!(
    /// Properties of the AUTH packet
    AuthProperties,
    AuthenticationMethod,
    AuthenticationData,
    ReasonString,
    UserProperty
);

impl MqttRead for AuthProperties {
    fn read(buf: &mut Bytes) -> Result<Self, crate::packets::error::DeserializeError> {
        let (len, _) = VariableInteger::read_variable_integer(buf)?;

        let mut properties = AuthProperties::default();

        if len == 0 {
            return Ok(properties);
        } else if buf.len() < len {
            return Err(DeserializeError::MalformedPacket);
        }

        let mut property_data = buf.split_to(len);

        loop {
            match PropertyType::read(&mut property_data)? {
                PropertyType::ReasonString => {
                    if properties.reason_string.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::SessionExpiryInterval));
                    }
                    properties.reason_string = Some(Box::<str>::read(&mut property_data)?);
                }
                PropertyType::UserProperty => properties.user_properties.push((Box::<str>::read(&mut property_data)?, Box::<str>::read(&mut property_data)?)),
                PropertyType::AuthenticationMethod => {
                    if properties.authentication_method.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::AuthenticationMethod));
                    }
                    properties.authentication_method = Some(Box::<str>::read(&mut property_data)?);
                }
                PropertyType::AuthenticationData => {
                    if properties.authentication_data.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::AuthenticationData));
                    }
                    properties.authentication_data = Some(Vec::<u8>::read(&mut property_data)?);
                }
                e => return Err(DeserializeError::UnexpectedProperty(e, PacketType::Auth)),
            }

            if property_data.is_empty() {
                break;
            }
        }

        Ok(properties)
    }
}

impl MqttWrite for AuthProperties {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::packets::error::SerializeError> {
        self.wire_len().write_variable_integer(buf)?;

        if let Some(authentication_method) = &self.authentication_method {
            PropertyType::AuthenticationMethod.write(buf)?;
            authentication_method.write(buf)?;
        }
        if let Some(authentication_data) = &self.authentication_data {
            if !authentication_data.is_empty() && self.authentication_method.is_some() {
                PropertyType::AuthenticationData.write(buf)?;
                authentication_data.write(buf)?;
            }
        }
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
