use crate::{error::PacketValidationError, util::constants::MAXIMUM_TOPIC_SIZE};

use super::{
    error::DeserializeError,
    mqtt_traits::{MqttRead, MqttWrite, VariableHeaderRead, VariableHeaderWrite, WireLength, PacketValidation},
    read_variable_integer, variable_integer_len, write_variable_integer, PacketType, PropertyType,
};
use bytes::BufMut;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unsubscribe {
    pub packet_identifier: u16,
    pub properties: UnsubscribeProperties,
    pub topics: Vec<String>,
}

impl Unsubscribe {
    pub fn new(packet_identifier: u16, topics: Vec<String>) -> Self {
        Self {
            packet_identifier,
            properties: UnsubscribeProperties::default(),
            topics,
        }
    }
}

impl VariableHeaderRead for Unsubscribe {
    fn read(_: u8, _: usize, mut buf: bytes::Bytes) -> Result<Self, super::error::DeserializeError> {
        let packet_identifier = u16::read(&mut buf)?;
        let properties = UnsubscribeProperties::read(&mut buf)?;
        let mut topics = vec![];
        loop {
            let topic = String::read(&mut buf)?;

            topics.push(topic);

            if buf.is_empty() {
                break;
            }
        }

        Ok(Self {
            packet_identifier,
            properties,
            topics,
        })
    }
}

impl VariableHeaderWrite for Unsubscribe {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
        buf.put_u16(self.packet_identifier);
        self.properties.write(buf)?;

        for topic in &self.topics {
            topic.write(buf)?;
        }

        Ok(())
    }
}

impl WireLength for Unsubscribe {
    fn wire_len(&self) -> usize {
        let mut len = 2 + variable_integer_len(self.properties.wire_len()) + self.properties.wire_len();
        for topic in &self.topics {
            len += topic.wire_len();
        }
        len
    }
}

impl PacketValidation for Unsubscribe{
    fn validate(&self, max_packet_size: usize) -> Result<(), PacketValidationError> {
        if self.wire_len() > max_packet_size{
            return Err(PacketValidationError::MaxPacketSize(self.wire_len()));
        }
        for topic in &self.topics{
            if topic.len() > MAXIMUM_TOPIC_SIZE{
                return Err(PacketValidationError::TopicSize(topic.len()));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct UnsubscribeProperties {
    pub user_properties: Vec<(String, String)>,
}

impl MqttRead for UnsubscribeProperties {
    fn read(buf: &mut bytes::Bytes) -> Result<Self, super::error::DeserializeError> {
        let (len, _) = read_variable_integer(buf)?;

        let mut properties = UnsubscribeProperties::default();

        if len == 0 {
            return Ok(properties);
        } else if buf.len() < len {
            return Err(DeserializeError::InsufficientData("UnsubscribeProperties".to_string(), buf.len(), len));
        }

        let mut properties_data = buf.split_to(len);

        loop {
            match PropertyType::read(&mut properties_data)? {
                PropertyType::UserProperty => {
                    properties.user_properties.push((String::read(&mut properties_data)?, String::read(&mut properties_data)?));
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
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
        write_variable_integer(buf, self.wire_len())?;
        for (key, value) in &self.user_properties {
            PropertyType::UserProperty.write(buf)?;
            key.write(buf)?;
            value.write(buf)?;
        }
        Ok(())
    }
}

impl WireLength for UnsubscribeProperties {
    fn wire_len(&self) -> usize {
        let mut len = 0;
        for (key, value) in &self.user_properties {
            len += 1 + key.wire_len() + value.wire_len();
        }
        len
    }
}

pub struct UnsubscribeTopics(pub Vec<String>);

impl From<&str> for UnsubscribeTopics {
    fn from(value: &str) -> Self {
        Self(vec![value.to_string()])
    }
}

impl From<&[&str]> for UnsubscribeTopics {
    fn from(value: &[&str]) -> Self {
        Self(value.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }
}

impl From<String> for UnsubscribeTopics {
    fn from(value: String) -> Self {
        Self(vec![value])
    }
}

impl From<Vec<String>> for UnsubscribeTopics {
    fn from(value: Vec<String>) -> Self {
        Self(value)
    }
}

impl From<&[String]> for UnsubscribeTopics {
    fn from(value: &[String]) -> Self {
        Self(value.to_vec())
    }
}

#[cfg(test)]
mod tests {

    // TCP and MQTT packet
    // let unsubscribe_packet = &[0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x00, 0x05, 0x9a, 0x3c, 0x7a, 0x00, 0x08, 0x00, 0x45, 0x00,
    // 0x00, 0x45, 0x70, 0xec, 0x40, 0x00, 0x80, 0x06, 0x4a, 0x70, 0xac, 0x14, 0x07, 0x50, 0x0a, 0xea,
    // 0x81, 0x08, 0xfc, 0xbb, 0x07, 0x5b, 0xb2, 0x8c, 0x95, 0xee, 0xcf, 0x54, 0x35, 0xaf, 0x50, 0x18,
    // 0x01, 0x04, 0xe7, 0x3f, 0x00, 0x00, 0xa2, 0x1b, 0x35, 0xd7, 0x00, 0x00, 0x16, 0x73, 0x75, 0x62,
    // 0x73, 0x63, 0x72, 0x69, 0x62, 0x65, 0x2f, 0x65, 0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x2f, 0x74,
    // 0x65, 0x73, 0x74];

    // MQTT packet
    // let unsubscribe_packet =
    //     &[0xa2, 0x1b, 0x35, 0xd7, 0x00, 0x00, 0x16, 0x73, 0x75, 0x62,
    //     0x73, 0x63, 0x72, 0x69, 0x62, 0x65, 0x2f, 0x65, 0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x2f, 0x74,
    //     0x65, 0x73, 0x74];

    use bytes::{Bytes, BytesMut};

    use crate::packets::mqtt_traits::{VariableHeaderRead, VariableHeaderWrite};

    use super::Unsubscribe;

    #[test]
    fn read_write_unsubscribe() {
        let unsubscribe_packet = &[
            0x35, 0xd7, 0x00, 0x00, 0x16, 0x73, 0x75, 0x62, 0x73, 0x63, 0x72, 0x69, 0x62, 0x65, 0x2f, 0x65, 0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x2f, 0x74, 0x65, 0x73, 0x74,
        ];

        let mut bufmut = BytesMut::new();
        bufmut.extend(&unsubscribe_packet[..]);

        let buf: Bytes = bufmut.into();

        let s = Unsubscribe::read(0, 0, buf.clone()).unwrap();

        let mut result = BytesMut::new();
        s.write(&mut result).unwrap();

        assert_eq!(buf.to_vec(), result.to_vec());
    }
}
