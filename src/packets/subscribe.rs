use super::{
    error::DeserializeError,
    mqtt_traits::{MqttRead, MqttWrite, VariableHeaderRead, VariableHeaderWrite, WireLength},
    read_variable_integer, variable_integer_len, write_variable_integer, PacketType, PropertyType,
};
use bitflags::bitflags;
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
    fn read(
        _: u8,
        _: usize,
        mut buf: bytes::Bytes,
    ) -> Result<Self, super::error::DeserializeError> {
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
            return Err(DeserializeError::InsufficientData(
                "SubscribeProperties".to_string(),
                buf.len(),
                len,
            ));
        }

        let mut properties_data = buf.split_to(len);

        loop {
            match PropertyType::read(&mut properties_data)? {
                PropertyType::SubscriptionIdentifier => {
                    if properties.subscription_id.is_none() {
                        let (subscription_id, _) = read_variable_integer(&mut properties_data)?;

                        properties.subscription_id = Some(subscription_id);
                    } else {
                        return Err(DeserializeError::DuplicateProperty(
                            PropertyType::SubscriptionIdentifier,
                        ));
                    }
                }
                PropertyType::UserProperty => {
                    properties.user_properties.push((
                        String::read(&mut properties_data)?,
                        String::read(&mut properties_data)?,
                    ));
                }
                e => {
                    return Err(DeserializeError::UnexpectedProperty(
                        e,
                        PacketType::Subscribe,
                    ))
                }
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

bitflags! {
    pub struct SubscriptionOptions: u8 {
        const MAX_QOS1     = 0b00000001;
        const MAX_QOS2     = 0b00000010;
        const RETAIN_AS_PUBLISH = 0b00000100;
        const RETAIN_HANDLING1  = 0b00001000;
        const RETAIN_HANDLING2  = 0b00010000;
    }
}

impl Default for SubscriptionOptions {
    fn default() -> Self {
        Self { bits: 0b00000010 }
    }
}

impl MqttRead for SubscriptionOptions {
    fn read(buf: &mut bytes::Bytes) -> Result<Self, DeserializeError> {
        if buf.is_empty() {
            return Err(DeserializeError::InsufficientData(
                "SubscriptionOptions".to_string(),
                0,
                1,
            ));
        }

        let options = SubscriptionOptions::from_bits(buf.get_u8())
            .ok_or(DeserializeError::MalformedPacket)?;

        if options.contains(SubscriptionOptions::MAX_QOS1 | SubscriptionOptions::MAX_QOS2) {
            return Err(DeserializeError::MalformedPacketWithInfo(
                "3.8.3.1 Subscription Options, Maximum QoS 1 and 2 enabled at once.".to_string(),
            ));
        }
        if options
            .contains(SubscriptionOptions::RETAIN_HANDLING1 | SubscriptionOptions::RETAIN_HANDLING2)
        {
            return Err(DeserializeError::MalformedPacketWithInfo(
                "3.8.3.1 Subscription Options, RETAIN_HANDLING 1 and 2 enabled at once."
                    .to_string(),
            ));
        }

        Ok(options)
    }
}

impl MqttWrite for SubscriptionOptions {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
        buf.put_u8(self.bits);
        Ok(())
    }
}

pub struct Subscription(pub Vec<(String, SubscriptionOptions)>);

impl From<String> for Subscription {
    fn from(value: String) -> Self {
        Self(vec![(value, SubscriptionOptions::default())])
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

impl From<&str> for Subscription {
    fn from(value: &str) -> Self {
        Self(vec![(value.to_string(), SubscriptionOptions::default())])
    }
}

impl From<&[&str]> for Subscription {
    fn from(value: &[&str]) -> Self {
        Self(
            value
                .iter()
                .map(|topic| (topic.to_string(), SubscriptionOptions::default()))
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use bytes::{BufMut, Bytes, BytesMut};

    use crate::packets::{
        mqtt_traits::{MqttRead, VariableHeaderRead, VariableHeaderWrite},
        packets::Packet,
    };

    use super::{WireLength};

    use super::{Subscribe, SubscribeProperties, SubscriptionOptions};

    #[test]
    fn test_read_write_subscribe() {
        let _entire_sub_packet = [
            0x82, 0x1e, 0x35, 0xd6, 0x02, 0x0b, 0x01, 0x00, 0x16, 0x73, 0x75, 0x62, 0x73, 0x63,
            0x72, 0x69, 0x62, 0x65, 0x2f, 0x65, 0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x2f, 0x74,
            0x65, 0x73, 0x74, 0x15,
        ];

        let sub_data = [
            0x35, 0xd6, 0x02, 0x0b, 0x01, 0x00, 0x16, 0x73, 0x75, 0x62, 0x73, 0x63, 0x72, 0x69,
            0x62, 0x65, 0x2f, 0x65, 0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x2f, 0x74, 0x65, 0x73,
            0x74, 0x15,
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
    fn test_read_subscription_option_err() {
        let mut h = SubscriptionOptions::empty();
        h |= SubscriptionOptions::MAX_QOS1;
        h |= SubscriptionOptions::MAX_QOS2;
        h |= SubscriptionOptions::RETAIN_AS_PUBLISH;
        h |= SubscriptionOptions::RETAIN_HANDLING1;

        let mut buf = bytes::BytesMut::new();
        buf.put_u8(h.bits);

        assert!(SubscriptionOptions::read(&mut buf.into()).is_err());
    }

    #[test]
    fn test_read_subscription_option_ok() {
        let mut h = SubscriptionOptions::empty();
        h |= SubscriptionOptions::MAX_QOS1;
        h |= SubscriptionOptions::RETAIN_AS_PUBLISH;
        h |= SubscriptionOptions::RETAIN_HANDLING1;

        let mut buf = bytes::BytesMut::new();
        buf.put_u8(h.bits);

        assert!(SubscriptionOptions::read(&mut buf.into()).is_ok());
    }

    #[test]
    fn test_write() {
        let expected_bytes = [
            0x82, 0x0e, 0x00, 0x01, 0x00, 0x00, 0x08, 0x74, 0x65, 0x73, 0x74, 0x2f, 0x31, 0x32,
            0x33, 0x00,
        ];

        let sub = Subscribe {
            packet_identifier: 1,
            properties: SubscribeProperties::default(),
            topics: vec![("test/123".to_string(), SubscriptionOptions::empty())],
        };

        assert_eq!(14, sub.wire_len());

        let packet = Packet::Subscribe(sub);

        let mut write_buffer = BytesMut::new();

        packet.write(&mut write_buffer).unwrap();

        assert_eq!(expected_bytes.to_vec(), write_buffer.to_vec())
    }
}
