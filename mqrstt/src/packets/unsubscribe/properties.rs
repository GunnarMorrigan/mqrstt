use crate::packets::VariableInteger;

use crate::packets::{
    error::DeserializeError,
    mqtt_trait::{MqttRead, MqttWrite, PacketRead, PacketWrite, WireLength},
    PacketType, PropertyType,
};


crate::packets::macros::define_properties!(UnsubscribeProperties, UserProperty);

// #[derive(Debug, Default, PartialEq, Eq, Clone)]
// pub struct UnsubscribeProperties {
//     pub user_properties: Vec<(String, String)>,
// }

impl MqttRead for UnsubscribeProperties {
    fn read(buf: &mut bytes::Bytes) -> Result<Self, crate::packets::error::DeserializeError> {
        let (len, _) = VariableInteger::read_variable_integer(buf)?;

        let mut properties = UnsubscribeProperties::default();

        if len == 0 {
            return Ok(properties);
        } else if buf.len() < len {
            return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), buf.len(), len));
        }

        let mut properties_data = buf.split_to(len);

        loop {
            match PropertyType::read(&mut properties_data)? {
                PropertyType::UserProperty => {
                    properties.user_properties.push((Box::<str>::read(&mut properties_data)?, Box::<str>::read(&mut properties_data)?));
                }
                e => return Err(DeserializeError::UnexpectedProperty(e, PacketType::Unsubscribe)),
            }

            if properties_data.is_empty() {
                break;
            }
        }
        Ok(properties)
    }
}

impl MqttWrite for UnsubscribeProperties {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::packets::error::SerializeError> {
        self.wire_len().write_variable_integer(buf)?;
        for (key, value) in &self.user_properties {
            PropertyType::UserProperty.write(buf)?;
            key.write(buf)?;
            value.write(buf)?;
        }
        Ok(())
    }
}

// impl WireLength for UnsubscribeProperties {
//     fn wire_len(&self) -> usize {
//         let mut len = 0;
//         for (key, value) in &self.user_properties {
//             len += 1 + key.wire_len() + value.wire_len();
//         }
//         len
//     }
// }
