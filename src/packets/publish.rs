use bytes::{BufMut, Bytes};

use super::mqtt_traits::{
    MqttRead, MqttWrite, VariableHeaderRead, VariableHeaderWrite, WireLength,
};
use super::{
    error::{DeserializeError, SerializeError},
    read_variable_integer, variable_integer_len, write_variable_integer, PacketType, PropertyType,
    QoS,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Publish {
    /// 3.3.1.1 dup
    pub dup: bool,
    /// 3.3.1.2 QoS
    pub qos: QoS,
    /// 3.3.1.3 retain
    pub retain: bool,

    /// 3.3.2.1 Topic Name
    /// The Topic Name identifies the information channel to which Payload data is published.
    pub topic: String,

    /// 3.3.2.2 Packet Identifier
    /// The Packet Identifier field is only present in PUBLISH packets where the QoS level is 1 or 2. Section 2.2.1 provides more information about Packet Identifiers.
    pub packet_identifier: Option<u16>,

    /// 3.3.2.3 PUBLISH Properties
    pub publish_properties: PublishProperties,

    /// 3.3.3 PUBLISH Payload
    pub payload: Bytes,
}

impl Publish {
    pub fn new(
        qos: QoS,
        retain: bool,
        topic: String,
        packet_identifier: Option<u16>,
        publish_properties: PublishProperties,
        payload: Bytes,
    ) -> Self {
        Self {
            dup: false,
            qos,
            retain,
            topic,
            packet_identifier,
            publish_properties,
            payload,
        }
    }
}

impl VariableHeaderRead for Publish {
    fn read(
        flags: u8,
        remaining_length: usize,
        mut buf: bytes::Bytes,
    ) -> Result<Self, DeserializeError> {
        let dup = flags & 0b1000 != 0;
        let qos = QoS::from_u8((flags & 0b110) >> 1)?;
        let retain = flags & 0b1 != 0;

        let topic = String::read(&mut buf)?;
        let mut packet_identifier = None;
        if qos != QoS::AtMostOnce {
            packet_identifier = Some(u16::read(&mut buf)?);
        }

        let publish_properties = PublishProperties::read(&mut buf)?;
        
        todo!("Fix this below");
        let payload_len = remaining_length - remaining_length;
        if buf.len() < payload_len {
            return Err(DeserializeError::InsufficientData(
                "Publish".to_string(),
                buf.len(),
                payload_len,
            ));
        }

        Ok(Self {
            dup,
            qos,
            retain,
            topic,
            packet_identifier,
            publish_properties,
            payload: buf,
        })
    }
}

impl VariableHeaderWrite for Publish {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), SerializeError> {
        self.topic.write(buf)?;

        if let Some(pkid) = self.packet_identifier {
            buf.put_u16(pkid);
        }

        self.publish_properties.write(buf)?;

        buf.extend(&self.payload);

        Ok(())
    }
}

impl WireLength for Publish {
    fn wire_len(&self) -> usize {
        let len = self.topic.wire_len()
            + if self.packet_identifier.is_some() {
                2
            } else {
                0
            }
            + self.publish_properties.wire_len()
            + self.payload.len();

        len + variable_integer_len(len)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PublishProperties {
    /// 3.3.2.3.2 Payload Format Indicator
    /// 1 (0x01) Byte, Identifier of the Payload Format Indicator.
    pub(crate) payload_format_indicator: Option<u8>,

    /// 3.3.2.3.3 Message Expiry Interval
    /// 2 (0x02) Byte, Identifier of the Message Expiry Interval.
    pub(crate) message_expiry_interval: Option<u32>,

    /// 3.3.2.3.4 Topic Alias
    /// 35 (0x23) Byte, Identifier of the Topic Alias.
    pub(crate) topic_alias: Option<u16>,

    /// 3.3.2.3.5 Response Topic
    /// 8 (0x08) Byte, Identifier of the Response Topic.
    pub(crate) response_topic: Option<String>,

    /// 3.3.2.3.6 Correlation Data
    /// 9 (0x09) Byte, Identifier of the Correlation Data.
    pub(crate) correlation_data: Option<Bytes>,

    /// 3.3.2.3.8 Subscription Identifier
    /// 11 (0x0B), Identifier of the Subscription Identifier.
    pub(crate) subscription_identifier: Vec<usize>,

    /// 3.3.2.3.7 User Property
    /// 38 (0x26) Byte, Identifier of the User Property.
    pub(crate) user_properties: Vec<(String, String)>,

    /// 3.3.2.3.9 Content Type
    /// 3 (0x03) Identifier of the Content Type
    pub(crate) content_type: Option<String>,
}

impl MqttRead for PublishProperties {
    fn read(buf: &mut bytes::Bytes) -> Result<Self, super::error::DeserializeError> {
        let (len, _) = read_variable_integer(buf).map_err(DeserializeError::from)?;

        if len == 0 {
            return Ok(Self::default());
        } else if buf.len() < len {
            return Err(DeserializeError::InsufficientData(
                "PublishProperties".to_string(),
                buf.len(),
                len,
            ));
        }

        let mut property_data = buf.split_to(len);

        let mut properties = Self::default();

        loop {
            match PropertyType::from_u8(u8::read(&mut property_data)?)? {
                PropertyType::PayloadFormatIndicator => {
                    if properties.payload_format_indicator.is_some() {
                        return Err(DeserializeError::DuplicateProperty(
                            PropertyType::PayloadFormatIndicator,
                        ));
                    }
                    properties.payload_format_indicator = Some(u8::read(&mut property_data)?);
                }
                PropertyType::MessageExpiryInterval => {
                    if properties.message_expiry_interval.is_some() {
                        return Err(DeserializeError::DuplicateProperty(
                            PropertyType::MessageExpiryInterval,
                        ));
                    }
                    properties.message_expiry_interval = Some(u32::read(&mut property_data)?);
                }
                PropertyType::TopicAlias => {
                    if properties.topic_alias.is_some() {
                        return Err(DeserializeError::DuplicateProperty(
                            PropertyType::TopicAlias,
                        ));
                    }
                    properties.topic_alias = Some(u16::read(&mut property_data)?);
                }
                PropertyType::ResponseTopic => {
                    if properties.response_topic.is_some() {
                        return Err(DeserializeError::DuplicateProperty(
                            PropertyType::ResponseTopic,
                        ));
                    }
                    properties.response_topic = Some(String::read(&mut property_data)?);
                }
                PropertyType::CorrelationData => {
                    if properties.correlation_data.is_some() {
                        return Err(DeserializeError::DuplicateProperty(
                            PropertyType::CorrelationData,
                        ));
                    }
                    properties.correlation_data = Some(Bytes::read(&mut property_data)?);
                }
                PropertyType::SubscriptionIdentifier => {
                    properties
                        .subscription_identifier
                        .push(read_variable_integer(&mut property_data)?.0);
                }
                PropertyType::UserProperty => properties.user_properties.push((
                    String::read(&mut property_data)?,
                    String::read(&mut property_data)?,
                )),
                PropertyType::ContentType => {
                    if properties.content_type.is_some() {
                        return Err(DeserializeError::DuplicateProperty(
                            PropertyType::ContentType,
                        ));
                    }
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
        write_variable_integer(buf, self.wire_len())?;

        if let Some(payload_format_indicator) = self.payload_format_indicator {
            buf.put_u8(PropertyType::PayloadFormatIndicator.to_u8());
            buf.put_u8(payload_format_indicator);
        }
        if let Some(message_expiry_interval) = self.message_expiry_interval {
            buf.put_u8(PropertyType::MessageExpiryInterval.to_u8());
            buf.put_u32(message_expiry_interval);
        }
        if let Some(topic_alias) = self.topic_alias {
            buf.put_u8(PropertyType::TopicAlias.to_u8());
            buf.put_u16(topic_alias);
        }
        if let Some(response_topic) = &self.response_topic {
            buf.put_u8(PropertyType::ResponseTopic.to_u8());
            response_topic.write(buf)?;
        }
        if let Some(correlation_data) = &self.correlation_data {
            buf.put_u8(PropertyType::CorrelationData.to_u8());
            correlation_data.write(buf)?;
        }
        for sub_id in &self.subscription_identifier {
            buf.put_u8(PropertyType::SubscriptionIdentifier.to_u8());
            write_variable_integer(buf, *sub_id)?;
        }
        for (key, val) in &self.user_properties {
            buf.put_u8(PropertyType::UserProperty.to_u8());
            key.write(buf)?;
            val.write(buf)?;
        }
        if let Some(content_type) = &self.content_type {
            buf.put_u8(PropertyType::ContentType.to_u8());
            content_type.write(buf)?;
        }

        Ok(())
    }
}

impl WireLength for PublishProperties {
    fn wire_len(&self) -> usize {
        let mut len = 0;

        if self.payload_format_indicator.is_some() {
            len += 2;
        }
        if self.message_expiry_interval.is_some() {
            len += 5;
        }
        if self.topic_alias.is_some() {
            len += 3;
        }
        if let Some(response_topic) = &self.response_topic {
            len += 1 + response_topic.wire_len();
        }
        if let Some(correlation_data) = &self.correlation_data {
            len += 1 + correlation_data.wire_len();
        }
        for sub_id in &self.subscription_identifier {
            len += 1 + variable_integer_len(*sub_id);
        }
        for (key, val) in &self.user_properties {
            len += 1 + key.wire_len() + val.wire_len();
        }
        if let Some(content_type) = &self.content_type {
            len += 1 + content_type.wire_len();
        }

        len
    }
}

#[cfg(test)]
mod tests {
    use bytes::{BufMut, BytesMut};

    use crate::packets::{
        mqtt_traits::{VariableHeaderRead, VariableHeaderWrite},
        write_variable_integer,
    };

    use super::Publish;

    #[test]
    fn test_read_write_properties() {
        let first_byte = 0b0011_0100;

        let mut properties = [1, 0, 2].to_vec();
        properties.extend(4_294_967_295u32.to_be_bytes());
        properties.push(35);
        properties.extend(3456u16.to_be_bytes());
        properties.push(8);
        let resp_topic = "hellogoodbye";
        properties.extend((resp_topic.len() as u16).to_be_bytes());
        properties.extend(resp_topic.as_bytes());

        let mut buf_one = BytesMut::from(
            &[
                0x00, 0x03, b'a', b'/', b'b', // variable header. topic name = 'a/b'
            ][..],
        );
        buf_one.put_u16(10);
        write_variable_integer(&mut buf_one, properties.len()).unwrap();
        buf_one.extend(properties);
        buf_one.extend(
            [
                0x01, // Payload
                0x02, 0xDE, 0xAD, 0xBE,
            ]
            .to_vec(),
        );

        let rem_len = buf_one.len();

        let buf = BytesMut::from(&buf_one[..]);

        let p = Publish::read(first_byte & 0b0000_1111, rem_len, buf.into()).unwrap();

        let mut result_buf = BytesMut::new();
        p.write(&mut result_buf).unwrap();

        dbg!(p.clone());

        assert_eq!(buf_one.to_vec(), result_buf.to_vec())
    }

    #[test]
    fn test_read_write() {
        let first_byte = 0b0011_0000;
        let buf_one = &[
            0x00, 0x03, b'a', b'/', b'b', // variable header. topic name = 'a/b'
            0x00, 0x01, 0x02, // payload
            0xDE, 0xAD, 0xBE,
        ];
        let rem_len = buf_one.len();

        let buf = BytesMut::from(&buf_one[..]);

        let p = Publish::read(first_byte & 0b0000_1111, rem_len, buf.into()).unwrap();

        let mut result_buf = BytesMut::new();
        p.write(&mut result_buf).unwrap();

        assert_eq!(buf_one.to_vec(), result_buf.to_vec())
    }
}
