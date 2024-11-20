use bytes::{BufMut, Bytes, BytesMut};


use crate::packets::VariableInteger;
use crate::packets::{
    error::{DeserializeError, SerializeError},
    mqtt_trait::{MqttRead, MqttWrite},
    PacketType, PropertyType, WireLength,
};

crate::packets::macros::define_properties!(LastWillProperties,
    WillDelayInterval,
    PayloadFormatIndicator,
    MessageExpiryInterval,
    ContentType,
    ResponseTopic,
    CorrelationData,
    UserProperty
);


// #[derive(Debug, Default, Clone, PartialEq, Eq)]
// pub struct LastWillProperties {
//     /// 3.1.3.2.2 Will Delay Interval
//     will_delay_interval: Option<u32>,
//     /// 3.1.3.2.3 Payload Format Indicator
//     payload_format_indicator: Option<u8>,
//     /// 3.1.3.2.4 Message Expiry Interval
//     message_expiry_interval: Option<u32>,
//     /// 3.1.3.2.5 Content Type
//     content_type: Option<Box<str>>,
//     /// 3.1.3.2.6 Response Topic
//     response_topic: Option<Box<str>>,
//     /// 3.1.3.2.7 Correlation Data
//     correlation_data: Option<Bytes>,
//     /// 3.1.3.2.8 User Property
//     user_properties: Vec<(Box<str>, Box<str>)>,
// }

impl MqttRead for LastWillProperties {
    fn read(buf: &mut Bytes) -> Result<Self, DeserializeError> {
        let (len, _) = VariableInteger::read_variable_integer(buf)?;

        let mut properties = Self::default();
        if len == 0 {
            return Ok(properties);
        } else if buf.len() < len {
            return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), buf.len(), len));
        }

        let mut property_data = buf.split_to(len);

        loop {
            match PropertyType::read(&mut property_data)? {
                PropertyType::WillDelayInterval => {
                    if properties.will_delay_interval.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::WillDelayInterval));
                    }
                    properties.will_delay_interval = Some(u32::read(&mut property_data)?);
                }
                PropertyType::PayloadFormatIndicator => {
                    if properties.payload_format_indicator.is_none() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::PayloadFormatIndicator));
                    }
                    properties.payload_format_indicator = Some(u8::read(&mut property_data)?);
                }
                PropertyType::MessageExpiryInterval => {
                    if properties.message_expiry_interval.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::MessageExpiryInterval));
                    }
                    properties.message_expiry_interval = Some(u32::read(&mut property_data)?);
                }
                PropertyType::ContentType => {
                    if properties.content_type.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::ContentType));
                    }
                    properties.content_type = Some(Box::<str>::read(&mut property_data)?);
                }
                PropertyType::ResponseTopic => {
                    if properties.response_topic.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::ResponseTopic));
                    }
                    properties.response_topic = Some(Box::<str>::read(&mut property_data)?);
                }
                PropertyType::CorrelationData => {
                    if properties.correlation_data.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::CorrelationData));
                    }
                    properties.correlation_data = Some(Vec::<u8>::read(&mut property_data)?);
                }
                PropertyType::UserProperty => properties.user_properties.push((Box::<str>::read(&mut property_data)?, Box::<str>::read(&mut property_data)?)),
                e => return Err(DeserializeError::UnexpectedProperty(e, PacketType::Connect)),
            }

            if property_data.is_empty() {
                break;
            }
        }

        Ok(properties)
    }
}

impl MqttWrite for LastWillProperties {
    fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError> {
        self.wire_len().write_variable_integer(buf)?;

        if let Some(delay_interval) = self.will_delay_interval {
            PropertyType::WillDelayInterval.write(buf)?;
            buf.put_u32(delay_interval);
        }
        if let Some(payload_format_indicator) = self.payload_format_indicator {
            PropertyType::PayloadFormatIndicator.write(buf)?;
            buf.put_u8(payload_format_indicator);
        }
        if let Some(message_expiry_interval) = self.message_expiry_interval {
            PropertyType::MessageExpiryInterval.write(buf)?;
            buf.put_u32(message_expiry_interval);
        }
        if let Some(content_type) = &self.content_type {
            PropertyType::ContentType.write(buf)?;
            content_type.write(buf)?;
        }
        if let Some(response_topic) = &self.response_topic {
            PropertyType::ResponseTopic.write(buf)?;
            response_topic.write(buf)?;
        }
        if let Some(correlation_data) = &self.correlation_data {
            PropertyType::CorrelationData.write(buf)?;
            correlation_data.write(buf)?;
        }
        if !self.user_properties.is_empty() {
            for (key, value) in &self.user_properties {
                PropertyType::UserProperty.write(buf)?;
                key.write(buf)?;
                value.write(buf)?;
            }
        }
        Ok(())
    }
}

// impl WireLength for LastWillProperties {
//     fn wire_len(&self) -> usize {
//         let mut len: usize = 0;

//         if self.will_delay_interval.is_some() {
//             len += 5;
//         }
//         if self.payload_format_indicator.is_some() {
//             len += 2;
//         }
//         if self.message_expiry_interval.is_some() {
//             len += 5;
//         }
//         // +1 for the property type
//         len += self.content_type.as_ref().map_or_else(|| 0, |s| s.wire_len() + 1);
//         len += self.response_topic.as_ref().map_or_else(|| 0, |s| s.wire_len() + 1);
//         len += self.correlation_data.as_ref().map_or_else(|| 0, |b| b.wire_len() + 1);
//         for (key, value) in &self.user_properties {
//             len += key.wire_len() + value.wire_len() + 1;
//         }

//         len
//     }
// }