use bytes::BufMut;

use crate::packets::VariableInteger;

use crate::packets::mqtt_trait::{MqttRead, MqttWrite, WireLength};
use crate::packets::{
    PacketType, PropertyType,
    error::{DeserializeError, SerializeError},
};

crate::packets::macros::define_properties!(
    /// Publish Properties
    PublishProperties,
    PayloadFormatIndicator,
    MessageExpiryInterval,
    ContentType,
    ResponseTopic,
    CorrelationData,
    ListSubscriptionIdentifier,
    TopicAlias,
    UserProperty
);

impl MqttRead for PublishProperties {
    fn read(buf: &mut bytes::Bytes) -> Result<Self, crate::packets::error::DeserializeError> {
        let (len, _) = VariableInteger::read_variable_integer(buf)?;

        if len == 0 {
            return Ok(Self::default());
        } else if buf.len() < len {
            return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), buf.len(), len));
        }

        let mut property_data = buf.split_to(len);

        let mut properties = Self::default();

        loop {
            match PropertyType::try_from(u8::read(&mut property_data)?)? {
                PropertyType::PayloadFormatIndicator => {
                    if properties.payload_format_indicator.is_some() {
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
                PropertyType::TopicAlias => {
                    if properties.topic_alias.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::TopicAlias));
                    }
                    properties.topic_alias = Some(u16::read(&mut property_data)?);
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
                PropertyType::SubscriptionIdentifier => {
                    properties.subscription_identifiers.push(VariableInteger::read_variable_integer(&mut property_data)?.0);
                }
                PropertyType::UserProperty => properties.user_properties.push((Box::<str>::read(&mut property_data)?, Box::<str>::read(&mut property_data)?)),
                PropertyType::ContentType => {
                    if properties.content_type.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::ContentType));
                    }
                    properties.content_type = Some(Box::<str>::read(&mut property_data)?);
                }
                t => return Err(DeserializeError::UnexpectedProperty(t, PacketType::Publish)),
            }
            if property_data.is_empty() {
                break;
            }
        }

        Ok(properties)
    }
}

impl MqttWrite for PublishProperties {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), SerializeError> {
        self.wire_len().write_variable_integer(buf)?;

        if let Some(payload_format_indicator) = self.payload_format_indicator {
            buf.put_u8(PropertyType::PayloadFormatIndicator.into());
            buf.put_u8(payload_format_indicator);
        }
        if let Some(message_expiry_interval) = self.message_expiry_interval {
            buf.put_u8(PropertyType::MessageExpiryInterval.into());
            buf.put_u32(message_expiry_interval);
        }
        if let Some(topic_alias) = self.topic_alias {
            buf.put_u8(PropertyType::TopicAlias.into());
            buf.put_u16(topic_alias);
        }
        if let Some(response_topic) = &self.response_topic {
            buf.put_u8(PropertyType::ResponseTopic.into());
            response_topic.as_ref().write(buf)?;
        }
        if let Some(correlation_data) = &self.correlation_data {
            buf.put_u8(PropertyType::CorrelationData.into());
            correlation_data.write(buf)?;
        }
        for sub_id in &self.subscription_identifiers {
            buf.put_u8(PropertyType::SubscriptionIdentifier.into());
            sub_id.write_variable_integer(buf)?;
        }
        for (key, val) in &self.user_properties {
            buf.put_u8(PropertyType::UserProperty.into());
            key.write(buf)?;
            val.write(buf)?;
        }
        if let Some(content_type) = &self.content_type {
            buf.put_u8(PropertyType::ContentType.into());
            content_type.write(buf)?;
        }

        Ok(())
    }
}
