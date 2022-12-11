use bytes::BufMut;

use super::{reason_codes::{UnsubAckReasonCode}, read_variable_integer, PropertyType, PacketType, write_variable_integer, variable_integer_len};
use super::error::{DeserializeError, SerializeError};
use super::mqtt_traits::{VariableHeaderRead, MqttRead, MqttWrite, WireLength, VariableHeaderWrite};

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct UnsubAck{
    pub packet_identifier: u16,
    pub properties: UnsubAckProperties,
    pub reason_codes: Vec<UnsubAckReasonCode>,
}


impl VariableHeaderRead for UnsubAck{
    fn read(_: u8, _: usize,  mut buf: bytes::Bytes) -> Result<Self, super::error::DeserializeError> {
        let packet_identifier = u16::read(&mut buf)?;
        let properties = UnsubAckProperties::read(&mut buf)?;
        let mut reason_codes = vec![];
        loop{

            let reason_code = UnsubAckReasonCode::read(&mut buf)?;

            reason_codes.push(reason_code);

            if buf.len() == 0{
                break;
            }
        }

        Ok(Self{
            packet_identifier,
            properties,
            reason_codes,
        })
    }
}

impl VariableHeaderWrite for UnsubAck{
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), SerializeError> {
        buf.put_u16(self.packet_identifier);
        self.properties.write(buf)?;
        for reason_code in &self.reason_codes{
            reason_code.write(buf)?;
        }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct UnsubAckProperties{
    /// 3.11.2.1.2 Reason String
    /// 31 (0x1F) Byte, Identifier of the Reason String. 
    pub reason_string: Option<String>,

    pub user_properties: Vec<(String, String)>,
}

impl MqttRead for UnsubAckProperties{
    fn read(buf: &mut bytes::Bytes) -> Result<Self, DeserializeError> {
        let (len, _) = read_variable_integer(buf)?;
        
        let mut properties = UnsubAckProperties::default();

        if len == 0{
            return Ok(properties);
        }
        else if buf.len() < len{
            return Err(DeserializeError::InsufficientData("UnsubAckProperties".to_string(), buf.len(), len));
        }

        let mut properties_data = buf.split_to(len);

        loop{
            match PropertyType::read(&mut properties_data)? {
                PropertyType::ReasonString => {
                    if properties.reason_string.is_none(){

                        properties.reason_string = Some(String::read(&mut properties_data)?);
                    }
                    else{
                        return Err(DeserializeError::DuplicateProperty(PropertyType::SubscriptionIdentifier))
                    }
                },
                PropertyType::UserProperty => {                 
                    properties.user_properties.push((String::read(&mut properties_data)?, String::read(&mut properties_data)?));
                },
                e => return Err(DeserializeError::UnexpectedProperty(e, PacketType::UnsubAck)),
            }

            if buf.len() == 0{
                break;
            }
        }
        Ok(properties)
    }
}

impl MqttWrite for UnsubAckProperties{
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
        write_variable_integer(buf, self.wire_len())?;
        if let Some(reason_string) = &self.reason_string{
            PropertyType::ReasonString.write(buf)?;
            reason_string.write(buf)?;
        }
        for (key, value) in &self.user_properties{
            PropertyType::UserProperty.write(buf)?;
            key.write(buf)?;
            value.write(buf)?;
        }
        Ok(())
    }
}

impl WireLength for UnsubAckProperties{
    fn wire_len(&self) -> usize {
        let mut len = 0;
        if let Some(reason_string) = &self.reason_string{
            len += 1 + reason_string.wire_len();
        }
        for (key, value) in &self.user_properties{
            len += 1 + key.wire_len() + value.wire_len();
        }
        len
    }
}


#[cfg(test)]
mod tests{
    use bytes::{BytesMut, Bytes};

    use crate::packets::{unsuback::UnsubAck, mqtt_traits::{VariableHeaderRead, VariableHeaderWrite}};


    #[test]
    fn read_write_unsub_ack(){
        // let entire_mqtt_packet = [0xb0, 0x04, 0x35, 0xd7, 0x00, 0x00];
        let unsub_ack = [0x35, 0xd7, 0x00, 0x00];

        let mut bufmut = BytesMut::new();
        bufmut.extend(&unsub_ack[..]);

        let buf: Bytes = bufmut.into();

        let s = UnsubAck::read(0xb0, 4, buf.clone()).unwrap();

        let mut result = BytesMut::new();
        s.write(&mut result).unwrap();

        assert_eq!(buf.to_vec(), result.to_vec());
    }
}