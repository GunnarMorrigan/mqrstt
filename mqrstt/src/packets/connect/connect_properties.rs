use bytes::{Buf, BufMut, Bytes, BytesMut};


use crate::packets::VariableInteger;
use crate::packets::{
    error::{DeserializeError, SerializeError},
    mqtt_trait::{MqttAsyncRead, MqttRead, MqttWrite, PacketAsyncRead, PacketRead, PacketWrite},
    PacketType, PropertyType, WireLength,
};


// /// Connect Properties
// ///
// /// The wire representation starts with the length of all properties after which
// /// the identifiers and their actual value are given
// ///
// /// 3.1.2.11.1 Property Length
// /// The length of the Properties in the CONNECT packet Variable Header encoded as a Variable Byte Integer.
// /// Followed by all possible connect properties:
crate::packets::macros::define_properties!(ConnectProperties,
    SessionExpiryInterval,
    ReceiveMaximum,
    MaximumPacketSize,
    TopicAliasMaximum,
    RequestResponseInformation,
    RequestProblemInformation,
    UserProperty,
    AuthenticationMethod,
    AuthenticationData
);

// #[derive(Debug, Default, Clone, PartialEq, Eq)]
// pub struct ConnectProperties {
//     /// 3.1.2.11.2 Session Expiry Interval
//     /// 17 (0x11) Byte Identifier of the Session Expiry Interval
//     pub session_expiry_interval: Option<u32>,

//     /// 3.1.2.11.3 Receive Maximum
//     /// 33 (0x21) Byte, Identifier of the Receive Maximum
//     pub receive_maximum: Option<u16>,

//     /// 3.1.2.11.4 Maximum Packet Size
//     /// 39 (0x27) Byte, Identifier of the Maximum Packet Size
//     pub maximum_packet_size: Option<u32>,

//     /// 3.1.2.11.5 Topic Alias Maximum
//     /// 34 (0x22) Byte, Identifier of the Topic Alias Maximum
//     pub topic_alias_maximum: Option<u16>,

//     /// 3.1.2.11.6 Request Response Information
//     /// 25 (0x19) Byte, Identifier of the Request Response Information
//     pub request_response_information: Option<u8>,

//     /// 3.1.2.11.7 Request Problem Information
//     /// 23 (0x17) Byte, Identifier of the Request Problem Information
//     pub request_problem_information: Option<u8>,

//     /// 3.1.2.11.8 User Property
//     /// 38 (0x26) Byte, Identifier of the User Property
//     pub user_properties: Vec<(Box<str>, Box<str>)>,

//     /// 3.1.2.11.9 Authentication Method
//     /// 21 (0x15) Byte, Identifier of the Authentication Method
//     pub authentication_method: Option<Box<str>>,

//     /// 3.1.2.11.10 Authentication Data
//     /// 22 (0x16) Byte, Identifier of the Authentication Data
//     pub authentication_data: Bytes,
// }

impl MqttWrite for ConnectProperties {
    fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError> {
        self.wire_len().write_variable_integer(buf)?;

        if let Some(session_expiry_interval) = self.session_expiry_interval {
            PropertyType::SessionExpiryInterval.write(buf)?;
            buf.put_u32(session_expiry_interval);
        }
        if let Some(receive_maximum) = self.receive_maximum {
            PropertyType::ReceiveMaximum.write(buf)?;
            buf.put_u16(receive_maximum);
        }
        if let Some(maximum_packet_size) = self.maximum_packet_size {
            PropertyType::MaximumPacketSize.write(buf)?;
            buf.put_u32(maximum_packet_size);
        }
        if let Some(topic_alias_maximum) = self.topic_alias_maximum {
            PropertyType::TopicAliasMaximum.write(buf)?;
            buf.put_u16(topic_alias_maximum);
        }
        if let Some(request_response_information) = self.request_response_information {
            PropertyType::RequestResponseInformation.write(buf)?;
            buf.put_u8(request_response_information);
        }
        if let Some(request_problem_information) = self.request_problem_information {
            PropertyType::RequestProblemInformation.write(buf)?;
            buf.put_u8(request_problem_information);
        }
        for (key, value) in &self.user_properties {
            PropertyType::UserProperty.write(buf)?;
            key.write(buf)?;
            value.write(buf)?;
        }
        if let Some(authentication_method) = &self.authentication_method {
            PropertyType::AuthenticationMethod.write(buf)?;
            authentication_method.write(buf)?;
        }
        if let Some(authentication_data) = &self.authentication_data {
            if self.authentication_method.is_none() {
                return Err(SerializeError::AuthDataWithoutAuthMethod);
            }
            PropertyType::AuthenticationData.write(buf)?;
            authentication_data.write(buf)?;
        }
        Ok(())
    }
}

impl MqttRead for ConnectProperties {
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
                PropertyType::SessionExpiryInterval => {
                    if properties.session_expiry_interval.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::SessionExpiryInterval));
                    }
                    properties.session_expiry_interval = Some(property_data.get_u32());
                }
                PropertyType::ReceiveMaximum => {
                    if properties.receive_maximum.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::ReceiveMaximum));
                    }
                    properties.receive_maximum = Some(property_data.get_u16());
                }
                PropertyType::MaximumPacketSize => {
                    if properties.maximum_packet_size.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::MaximumPacketSize));
                    }
                    properties.maximum_packet_size = Some(property_data.get_u32());
                }
                PropertyType::TopicAliasMaximum => {
                    if properties.topic_alias_maximum.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::TopicAliasMaximum));
                    }
                    properties.topic_alias_maximum = Some(property_data.get_u16());
                }
                PropertyType::RequestResponseInformation => {
                    if properties.request_response_information.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::RequestResponseInformation));
                    }
                    properties.request_response_information = Some(property_data.get_u8());
                }
                PropertyType::RequestProblemInformation => {
                    if properties.request_problem_information.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::RequestProblemInformation));
                    }
                    properties.request_problem_information = Some(property_data.get_u8());
                }
                PropertyType::UserProperty => properties.user_properties.push((Box::<str>::read(&mut property_data)?, Box::<str>::read(&mut property_data)?)),
                PropertyType::AuthenticationMethod => {
                    if properties.authentication_method.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::AuthenticationMethod));
                    }
                    properties.authentication_method = Some(Box::<str>::read(&mut property_data)?);
                }
                PropertyType::AuthenticationData => {
                    if properties.authentication_data.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::AuthenticationData));
                    }
                    properties.authentication_data = Some(Vec::<u8>::read(&mut property_data)?);
                }
                e => return Err(DeserializeError::UnexpectedProperty(e, PacketType::Connect)),
            }

            if property_data.is_empty() {
                break;
            }
        }

        if properties.authentication_data.as_ref().is_some_and(|data| !data.is_empty()) && properties.authentication_method.is_none() {
            return Err(DeserializeError::MalformedPacketWithInfo("Authentication data is not empty while authentication method is".to_string()));
        }

        Ok(properties)
    }
}

// impl WireLength for ConnectProperties {
//     fn wire_len(&self) -> usize {
//         let mut len: usize = 0;

//         if self.session_expiry_interval.is_some() {
//             len += 1 + 4;
//         }
//         if self.receive_maximum.is_some() {
//             len += 1 + 2;
//         }
//         if self.maximum_packet_size.is_some() {
//             len += 1 + 4;
//         }
//         if self.topic_alias_maximum.is_some() {
//             len += 1 + 2;
//         }
//         if self.request_response_information.is_some() {
//             len += 2;
//         }
//         if self.request_problem_information.is_some() {
//             len += 2;
//         }
//         for (key, value) in &self.user_properties {
//             len += 1;
//             len += key.wire_len();
//             len += value.wire_len();
//         }
//         if let Some(authentication_method) = &self.authentication_method {
//             len += 1 + authentication_method.wire_len();
//         }
//         if !self.authentication_data.is_empty() && self.authentication_method.is_some() {
//             len += 1 + self.authentication_data.wire_len();
//         }

//         len
//     }
// }
