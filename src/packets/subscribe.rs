use super::{mqtt_traits::{MqttPacketRead, MqttRead, MqttWrite, WireLength, MqttPacketWrite}, QoS, read_variable_integer, PropertyType, errors::DeserializeError, PacketType, variable_integer_len, write_variable_integer};
use bitflags::bitflags;
use bytes::{Buf, BufMut};


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subscribe{
    pub packet_identifier: u16,
    pub properties: SubscribeProperties,
    pub topics: Vec<(String, SubscriptionOption)>,

}

impl MqttPacketRead for Subscribe{
    fn read(_: u8, _: usize,  mut buf: bytes::Bytes) -> Result<Self, super::errors::DeserializeError> {
        let packet_identifier = u16::read(&mut buf)?;
        let properties = SubscribeProperties::read(&mut buf)?;
        let mut topics = vec![];
        loop{

            let topic = String::read(&mut buf)?;
            let options = SubscriptionOption::read(&mut buf)?;

            topics.push((topic, options));

            if buf.len() == 0{
                break;
            }
        }

        Ok(Self{
            packet_identifier,
            properties,
            topics,
        })
    }
}

impl MqttPacketWrite for Subscribe{
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::errors::SerializeError> {
        buf.put_u16(self.packet_identifier);

        self.properties.write(buf)?;
        for (topic, options) in &self.topics{
            topic.write(buf)?;
            options.write(buf)?;
        }

        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct SubscribeProperties{
    /// 3.8.2.1.2 Subscription Identifier
    /// 11 (0x0B) Byte, Identifier of the Subscription Identifier. 
    pub subscription_id: Option<usize>,

    /// 3.8.2.1.3 User Property
    /// 38 (0x26) Byte, Identifier of the User Property. 
    pub user_properties: Vec<(String, String)>,
}

impl MqttRead for SubscribeProperties{
    fn read(buf: &mut bytes::Bytes) -> Result<Self, super::errors::DeserializeError> {
        let (len, _) = read_variable_integer(buf)?;
        
        let mut properties = SubscribeProperties::default();

        if len == 0{
            return Ok(properties);
        }
        else if buf.len() < len{
            return Err(DeserializeError::InsufficientData(buf.len(), len));
        }

        let mut properties_data = buf.split_to(len);

        loop{
            match PropertyType::read(&mut properties_data)? {
                PropertyType::SubscriptionIdentifier => {
                    if properties.subscription_id.is_none(){
                        let (subscription_id, _) = read_variable_integer(&mut properties_data)?;
    
                        properties.subscription_id = Some(subscription_id);
                    }
                    else{
                        return Err(DeserializeError::DuplicateProperty(PropertyType::SubscriptionIdentifier))
                    }
                },
                PropertyType::UserProperty => {                 
                    properties.user_properties.push((String::read(&mut properties_data)?, String::read(&mut properties_data)?));
                },
                e => return Err(DeserializeError::UnexpectedProperty(e, PacketType::Connect)),
            }

            if properties_data.len() == 0{
                break;
            }
        }
        Ok(properties)
    }
}

impl MqttWrite for SubscribeProperties{
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::errors::SerializeError> {
        write_variable_integer(buf, self.wire_len())?;
        if let Some(sub_id) = self.subscription_id{
            PropertyType::SubscriptionIdentifier.write(buf)?;
            write_variable_integer(buf, sub_id)?;
        }
        for (key, value) in &self.user_properties{
            PropertyType::UserProperty.write(buf)?;
            key.write(buf)?;
            value.write(buf)?;
        }
        Ok(())
    }
}

impl WireLength for SubscribeProperties{
    fn wire_len(&self) -> usize {
        let mut len = 0;
        if let Some(sub_id) = self.subscription_id{
            len += 1 + variable_integer_len(sub_id);
        }
        for (key, value) in &self.user_properties{
            len += 1 + key.wire_len() + value.wire_len();
        }
        len
    }
}

bitflags! {
    /// ╔═════╦═══════════╦══════════╦═════════════╦═════╦════╦═══════════╦═════════════╦══════════╗
    /// ║ Bit ║ 7         ║ 6        ║ 5           ║ 4   ║ 3  ║ 2         ║ 1           ║ 0        ║
    /// ╠═════╬═══════════╬══════════╬═════════════╬═════╩════╬═══════════╬═════════════╬══════════╣
    /// ║     ║ User Name ║ Password ║ Will Retain ║ Will QoS ║ Will Flag ║ Clean Start ║ Reserved ║
    /// ╚═════╩═══════════╩══════════╩═════════════╩══════════╩═══════════╩═════════════╩══════════╝
    pub struct SubscriptionOption: u8 {
        const MAX_QOS1     = 0b00000001;
        const MAX_QOS2     = 0b00000010;
        const RETAIN_AS_PUBLISH = 0b00000100;
        const RETAIN_HANDLING1  = 0b00001000;
        const RETAIN_HANDLING2  = 0b00010000;
    }
}

impl MqttRead for SubscriptionOption{
    fn read(buf: &mut bytes::Bytes) -> Result<Self, DeserializeError> {
        if buf.is_empty(){
            return Err(DeserializeError::InsufficientData(0, 1));
        }

        let options = SubscriptionOption::from_bits(buf.get_u8()).ok_or(DeserializeError::MalformedPacket)?;

        if options.contains(SubscriptionOption::MAX_QOS1 | SubscriptionOption::MAX_QOS2){
            return Err(DeserializeError::MalformedPacketWithInfo("3.8.3.1 Subscription Options, Maximum QoS 1 and 2 enabled at once.".to_string()));
        }
        if options.contains(SubscriptionOption::RETAIN_HANDLING1 | SubscriptionOption::RETAIN_HANDLING2){
            return Err(DeserializeError::MalformedPacketWithInfo("3.8.3.1 Subscription Options, RETAIN_HANDLING 1 and 2 enabled at once.".to_string()));
        }

        Ok(options)
    }
}

impl MqttWrite for SubscriptionOption{
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::errors::SerializeError> {
        buf.put_u8(self.bits);
        Ok(())
    }
}


#[cfg(test)]
mod tests{
    use bytes::{BufMut, Bytes, BytesMut};

    use crate::packets::{mqtt_traits::{MqttRead, MqttPacketRead, MqttPacketWrite}, read_variable_integer};

    use super::{SubscriptionOption, Subscribe};

    // 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x00, 0x05, 0x9a, 0x3c, 0x7a, 0x00, 0x08, 0x00, 0x45, 0x00,
    // 0x00, 0x48, 0x70, 0xea, 0x40, 0x00, 0x80, 0x06, 0x4a, 0x6f, 0xac, 0x14, 0x07, 0x50, 0x0a, 0xea,
    // 0x81, 0x08, 0xfc, 0xbb, 0x07, 0x5b, 0xb2, 0x8c, 0x95, 0xce, 0xcf, 0x54, 0x35, 0xa9, 0x50, 0x18,
    // 0x01, 0x04, 0x04, 0x41, 0x00, 0x00, 

    // Entire subscribe packet
    //     &[0x82, 0x1e, 0x35, 0xd6, 0x02, 0x0b, 0x01, 0x00, 0x16, 0x73,
    //     0x75, 0x62, 0x73, 0x63, 0x72, 0x69, 0x62, 0x65, 0x2f, 0x65, 0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65,
    //     0x2f, 0x74, 0x65, 0x73, 0x74, 0x15];


    #[test]
    fn test_read_write_subscribe(){
        // let sub_data = &[0x1e, 0x35, 0xd6, 0x02, 0x0b, 0x01, 0x00, 0x16, 0x73,
        // 0x75, 0x62, 0x73, 0x63, 0x72, 0x69, 0x62, 0x65, 0x2f, 0x65, 0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65,
        // 0x2f, 0x74, 0x65, 0x73, 0x74, 0x15];

        let sub_data = &[
            53, 214,2, 11, 1, 0, 22, 115, 117, 98, 115,
            99, 114, 105, 98, 101, 47, 101, 120, 97, 109,
            112, 108, 101, 47, 116, 101, 115, 116, 21,
        ];

        let mut bufmut = BytesMut::new();
        bufmut.extend(&sub_data[..]);

        let buf: Bytes = bufmut.into();

        let s = Subscribe::read(0, 0, buf.clone()).unwrap();

        let mut result = BytesMut::new();
        s.write(&mut result).unwrap();

        assert_eq!(buf.to_vec(), result.to_vec());
    }

        
    #[test]
    fn test_read_subscription_option_err(){

        let mut h = SubscriptionOption::empty();
        h |= SubscriptionOption::MAX_QOS1;
        h |= SubscriptionOption::MAX_QOS2;
        h |= SubscriptionOption::RETAIN_AS_PUBLISH;
        h |= SubscriptionOption::RETAIN_HANDLING1;

        let mut buf = bytes::BytesMut::new();
        buf.put_u8(h.bits);

        assert!(SubscriptionOption::read(&mut buf.into()).is_err());
    }

    #[test]
    fn test_read_subscription_option_ok(){

        let mut h = SubscriptionOption::empty();
        h |= SubscriptionOption::MAX_QOS1;
        h |= SubscriptionOption::RETAIN_AS_PUBLISH;
        h |= SubscriptionOption::RETAIN_HANDLING1;

        let mut buf = bytes::BytesMut::new();
        buf.put_u8(h.bits);

        assert!(SubscriptionOption::read(&mut buf.into()).is_ok());
    }
}