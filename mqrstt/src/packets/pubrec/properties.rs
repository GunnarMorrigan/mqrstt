use crate::packets::{
    error::DeserializeError,
    mqtt_trait::{MqttRead, MqttWrite, WireLength},
    PacketType, PropertyType, VariableInteger,
};

crate::packets::macros::define_properties!(PubRecProperties, ReasonString, UserProperty);

impl MqttRead for PubRecProperties {
    fn read(buf: &mut bytes::Bytes) -> Result<Self, crate::packets::error::DeserializeError> {
        let (len, _) = VariableInteger::read_variable_integer(buf)?;

        if len == 0 {
            return Ok(Self::default());
        }
        if buf.len() < len {
            return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), buf.len(), len));
        }

        let mut properties = PubRecProperties::default();

        loop {
            match PropertyType::try_from(u8::read(buf)?)? {
                PropertyType::ReasonString => {
                    if properties.reason_string.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::ReasonString));
                    }
                    properties.reason_string = Some(Box::<str>::read(buf)?);
                }
                PropertyType::UserProperty => properties.user_properties.push((Box::<str>::read(buf)?, Box::<str>::read(buf)?)),
                e => return Err(DeserializeError::UnexpectedProperty(e, PacketType::PubRec)),
            }
            if buf.is_empty() {
                break;
            }
        }
        Ok(properties)
    }
}

impl MqttWrite for PubRecProperties {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::packets::error::SerializeError> {
        let len = self.wire_len();

        len.write_variable_integer(buf)?;

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
