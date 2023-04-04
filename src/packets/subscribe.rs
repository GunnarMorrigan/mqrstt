use crate::{error::PacketValidationError, util::constants::MAXIMUM_TOPIC_SIZE};

use super::{
    error::DeserializeError,
    mqtt_traits::{MqttRead, MqttWrite, PacketValidation, VariableHeaderRead, VariableHeaderWrite, WireLength},
    read_variable_integer, variable_integer_len, write_variable_integer, PacketType, PropertyType, QoS,
};
use bytes::{Buf, BufMut};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subscribe {
    pub packet_identifier: u16,
    pub properties: SubscribeProperties,
    pub topics: Vec<(String, SubscriptionOptions)>,
}

impl Subscribe {
    pub fn new(packet_identifier: u16, topics: Vec<(String, SubscriptionOptions)>) -> Self {
        Self {
            packet_identifier,
            properties: SubscribeProperties::default(),
            topics,
        }
    }
}

impl VariableHeaderRead for Subscribe {
    fn read(_: u8, _: usize, mut buf: bytes::Bytes) -> Result<Self, super::error::DeserializeError> {
        let packet_identifier = u16::read(&mut buf)?;
        let properties = SubscribeProperties::read(&mut buf)?;
        let mut topics = vec![];
        loop {
            let topic = String::read(&mut buf)?;
            let options = SubscriptionOptions::read(&mut buf)?;

            topics.push((topic, options));

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

impl VariableHeaderWrite for Subscribe {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
        buf.put_u16(self.packet_identifier);

        self.properties.write(buf)?;
        for (topic, options) in &self.topics {
            topic.write(buf)?;
            options.write(buf)?;
        }

        Ok(())
    }
}

impl WireLength for Subscribe {
    fn wire_len(&self) -> usize {
        let mut len = 2;
        let properties_len = self.properties.wire_len();
        len += properties_len + variable_integer_len(properties_len);
        for topic in &self.topics {
            len += topic.0.wire_len() + 1;
        }
        len
    }
}

impl PacketValidation for Subscribe {
    fn validate(&self, max_packet_size: usize) -> Result<(), PacketValidationError> {
        if self.wire_len() > max_packet_size {
            return Err(PacketValidationError::MaxPacketSize(self.wire_len()));
        }
        for (topic, _) in &self.topics {
            if topic.len() > MAXIMUM_TOPIC_SIZE {
                return Err(PacketValidationError::TopicSize(topic.len()));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct SubscribeProperties {
    /// 3.8.2.1.2 Subscription Identifier
    /// 11 (0x0B) Byte, Identifier of the Subscription Identifier.
    pub subscription_id: Option<usize>,

    /// 3.8.2.1.3 User Property
    /// 38 (0x26) Byte, Identifier of the User Property.
    pub user_properties: Vec<(String, String)>,
}

impl MqttRead for SubscribeProperties {
    fn read(buf: &mut bytes::Bytes) -> Result<Self, super::error::DeserializeError> {
        let (len, _) = read_variable_integer(buf)?;

        let mut properties = SubscribeProperties::default();

        if len == 0 {
            return Ok(properties);
        } else if buf.len() < len {
            return Err(DeserializeError::InsufficientData("SubscribeProperties".to_string(), buf.len(), len));
        }

        let mut properties_data = buf.split_to(len);

        loop {
            match PropertyType::read(&mut properties_data)? {
                PropertyType::SubscriptionIdentifier => {
                    if properties.subscription_id.is_none() {
                        let (subscription_id, _) = read_variable_integer(&mut properties_data)?;

                        properties.subscription_id = Some(subscription_id);
                    } else {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::SubscriptionIdentifier));
                    }
                }
                PropertyType::UserProperty => {
                    properties.user_properties.push((String::read(&mut properties_data)?, String::read(&mut properties_data)?));
                }
                e => return Err(DeserializeError::UnexpectedProperty(e, PacketType::Subscribe)),
            }

            if properties_data.is_empty() {
                break;
            }
        }
        Ok(properties)
    }
}

impl MqttWrite for SubscribeProperties {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
        write_variable_integer(buf, self.wire_len())?;
        if let Some(sub_id) = self.subscription_id {
            PropertyType::SubscriptionIdentifier.write(buf)?;
            write_variable_integer(buf, sub_id)?;
        }
        for (key, value) in &self.user_properties {
            PropertyType::UserProperty.write(buf)?;
            key.write(buf)?;
            value.write(buf)?;
        }
        Ok(())
    }
}

impl WireLength for SubscribeProperties {
    fn wire_len(&self) -> usize {
        let mut len = 0;
        if let Some(sub_id) = self.subscription_id {
            len += 1 + variable_integer_len(sub_id);
        }
        for (key, value) in &self.user_properties {
            len += 1 + key.wire_len() + value.wire_len();
        }
        len
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct SubscriptionOptions {
    pub retain_handling: RetainHandling,
    pub retain_as_publish: bool,
    pub no_local: bool,
    pub qos: QoS,
}

impl Default for SubscriptionOptions {
    fn default() -> Self {
        Self {
            retain_handling: RetainHandling::ZERO,
            retain_as_publish: false,
            no_local: false,
            qos: QoS::AtMostOnce,
        }
    }
}

impl MqttRead for SubscriptionOptions {
    fn read(buf: &mut bytes::Bytes) -> Result<Self, DeserializeError> {
        if buf.is_empty() {
            return Err(DeserializeError::InsufficientData("SubscriptionOptions".to_string(), 0, 1));
        }

        let byte = buf.get_u8();

        let retain_handling_part = (byte & 0b00110000) >> 4;
        let retain_as_publish_part = (byte & 0b00001000) >> 3;
        let no_local_part = (byte & 0b00000100) >> 2;
        let qos_part = byte & 0b00000011;

        let options = Self {
            retain_handling: RetainHandling::from_u8(retain_handling_part)?,
            retain_as_publish: retain_as_publish_part != 0,
            no_local: no_local_part != 0,
            qos: QoS::from_u8(qos_part)?,
        };

        Ok(options)
    }
}

impl MqttWrite for SubscriptionOptions {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
        let byte = (self.retain_handling.into_u8() << 4) | ((self.retain_as_publish as u8) << 3) | ((self.no_local as u8) << 2) | self.qos.into_u8();

        buf.put_u8(byte);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetainHandling {
    ZERO,
    ONE,
    TWO,
}

impl RetainHandling {
    pub fn from_u8(value: u8) -> Result<Self, DeserializeError> {
        match value {
            0 => Ok(RetainHandling::ZERO),
            1 => Ok(RetainHandling::ONE),
            2 => Ok(RetainHandling::TWO),
            _ => Err(DeserializeError::MalformedPacket),
        }
    }
    pub fn into_u8(&self) -> u8 {
        match self {
            RetainHandling::ZERO => 0,
            RetainHandling::ONE => 1,
            RetainHandling::TWO => 2,
        }
    }
}

pub struct Subscription(pub Vec<(String, SubscriptionOptions)>);

impl From<String> for Subscription {
    fn from(value: String) -> Self {
        Self(vec![(value, SubscriptionOptions::default())])
    }
}

impl From<(&str, QoS)> for Subscription {
    fn from(value: (&str, QoS)) -> Self {
        Self(vec![(value.0.to_string(), SubscriptionOptions { qos: value.1, ..Default::default() })])
    }
}

impl From<Vec<(&str, QoS)>> for Subscription {
    fn from(value: Vec<(&str, QoS)>) -> Self {
        Self(value.into_iter().map(|f| (f.0.to_string(), SubscriptionOptions { qos: f.1, ..Default::default() }) ).collect())
    }
}

impl From<Vec<(String, SubscriptionOptions)>> for Subscription {
    fn from(value: Vec<(String, SubscriptionOptions)>) -> Self {
        Self(value)
    }
}

impl From<(String, SubscriptionOptions)> for Subscription {
    fn from(value: (String, SubscriptionOptions)) -> Self {
        Self(vec![value])
    }
}

impl From<Vec<(&str, SubscriptionOptions)>> for Subscription {
    fn from(value: Vec<(&str, SubscriptionOptions)>) -> Self {
        Self(value.into_iter().map(|s| (s.0.to_string(), s.1)).collect())
    }
}

impl From<(&str, SubscriptionOptions)> for Subscription {
    fn from(value: (&str, SubscriptionOptions)) -> Self {
        Self(vec![(value.0.to_string(), value.1)])
    }
}

impl From<&str> for Subscription {
    fn from(value: &str) -> Self {
        Self(vec![(value.to_string(), SubscriptionOptions::default())])
    }
}

impl From<&[String]> for Subscription {
    fn from(value: &[String]) -> Self {
        Self(value.iter().map(|topic| (topic.clone(), SubscriptionOptions::default())).collect())
    }
}

impl From<&[&str]> for Subscription {
    fn from(value: &[&str]) -> Self {
        Self(value.iter().map(|topic| (topic.to_string(), SubscriptionOptions::default())).collect())
    }
}

impl From<&[(&str, QoS)]> for Subscription {
    fn from(value: &[(&str, QoS)]) -> Self {
        Self(value.iter().map(|(topic, qos)| (topic.to_string(), SubscriptionOptions { qos: *qos, ..Default::default() })).collect())
    }
}

#[cfg(test)]
mod tests {
    use bytes::{Bytes, BytesMut};

    use crate::packets::{
        mqtt_traits::{MqttRead, VariableHeaderRead, VariableHeaderWrite},
        Packet,
    };

    use super::WireLength;

    use super::{Subscribe, SubscribeProperties, SubscriptionOptions};

    #[test]
    fn test_read_write_subscribe() {
        let _entire_sub_packet = [
            0x82, 0x1e, 0x35, 0xd6, 0x02, 0x0b, 0x01, 0x00, 0x16, 0x73, 0x75, 0x62, 0x73, 0x63, 0x72, 0x69, 0x62, 0x65, 0x2f, 0x65, 0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x2f, 0x74, 0x65, 0x73, 0x74,
            0x15,
        ];

        let sub_data = [
            0x35, 0xd6, 0x02, 0x0b, 0x01, 0x00, 0x16, 0x73, 0x75, 0x62, 0x73, 0x63, 0x72, 0x69, 0x62, 0x65, 0x2f, 0x65, 0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x2f, 0x74, 0x65, 0x73, 0x74, 0x15,
        ];

        // let sub_data = &[0x35, 0xd6, 0x02, 0x0b, 0x01, 0x00, 0x16, 0x73,
        // 0x75, 0x62, 0x73, 0x63, 0x72, 0x69, 0x62, 0x65, 0x2f, 0x65, 0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65,
        // 0x2f, 0x74, 0x65, 0x73, 0x74, 0x15];

        // let sub_data = &[
        //     53, 214, 2, 11, 1, 0, 22, 115, 117, 98, 115,
        //     99, 114, 105, 98, 101, 47, 101, 120, 97, 109,
        //     112, 108, 101, 47, 116, 101, 115, 116, 21,
        // ];

        let mut bufmut = BytesMut::new();
        bufmut.extend(&sub_data[..]);

        let buf: Bytes = bufmut.into();

        let s = Subscribe::read(0, 30, buf.clone()).unwrap();

        let s = dbg!(s);

        let mut result = BytesMut::new();
        s.write(&mut result).unwrap();

        assert_eq!(buf.to_vec(), result.to_vec());
    }

    #[test]
    fn test_write() {
        let expected_bytes = [0x82, 0x0e, 0x00, 0x01, 0x00, 0x00, 0x08, 0x74, 0x65, 0x73, 0x74, 0x2f, 0x31, 0x32, 0x33, 0x00];

        let sub = Subscribe {
            packet_identifier: 1,
            properties: SubscribeProperties::default(),
            topics: vec![("test/123".to_string(), SubscriptionOptions::default())],
        };

        assert_eq!(14, sub.wire_len());

        let packet = Packet::Subscribe(sub);

        let mut write_buffer = BytesMut::new();

        packet.write(&mut write_buffer).unwrap();

        assert_eq!(expected_bytes.to_vec(), write_buffer.to_vec())
    }

    #[test]
    fn test_subscription_options() {
        let mut buf = Bytes::from_static(&[0b00101110u8]);
        let _ = SubscriptionOptions::read(&mut buf).unwrap();
    }
}
