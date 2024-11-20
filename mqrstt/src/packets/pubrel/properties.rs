use crate::packets::{
    error::DeserializeError, 
    mqtt_trait::{MqttRead, MqttWrite, WireLength}, PacketType, PropertyType, VariableInteger
};

crate::packets::macros::define_properties!(PubRelProperties, 
    ReasonString,
    UserProperty
);

impl PubRelProperties {
    pub fn is_empty(&self) -> bool {
        self.reason_string.is_none() && self.user_properties.is_empty()
    }
}

impl MqttRead for PubRelProperties {
    fn read(buf: &mut bytes::Bytes) -> Result<Self, crate::packets::error::DeserializeError> {
        let (len, _) = VariableInteger::read_variable_integer(buf)?;

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