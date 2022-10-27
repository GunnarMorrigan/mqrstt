use std::vec;

use bytes::{Bytes, Buf, BytesMut, BufMut};
use bitflags::bitflags;

use super::{QoS, SimpleSerialize, read_variable_integer, write_variable_integer, ProtocolVersion, PropertyType, WireLength};

/// Variable connect header:
/// 
/// 
/// ╔═══════════╦═══════════════════╦══════╦══════╦══════╦══════╦══════╦══════╦══════╦══════╗
/// ║           ║                   ║      ║      ║      ║      ║      ║      ║      ║      ║
/// ║           ║ Description       ║ 7    ║ 6    ║ 5    ║ 4    ║ 3    ║ 2    ║ 1    ║ 0    ║
/// ╠═══════════╩═══════════════════╩══════╩══════╩══════╩══════╩══════╩══════╩══════╩══════╣
/// ║                                                                                       ║
/// ║ Protocol Name                                                                         ║
/// ╠═══════════╦═══════════════════╦══════╦══════╦══════╦══════╦══════╦══════╦══════╦══════╣
/// ║           ║                   ║      ║      ║      ║      ║      ║      ║      ║      ║
/// ║ byte 1    ║ Length MSB (0)    ║ 0    ║ 0    ║ 0    ║ 0    ║ 0    ║ 0    ║ 0    ║ 0    ║
/// ╠═══════════╬═══════════════════╬══════╬══════╬══════╬══════╬══════╬══════╬══════╬══════╣
/// ║           ║                   ║      ║      ║      ║      ║      ║      ║      ║      ║
/// ║ byte 2    ║ Length LSB (4)    ║ 0    ║ 0    ║ 0    ║ 0    ║ 0    ║ 1    ║ 0    ║ 0    ║
/// ╠═══════════╬═══════════════════╬══════╬══════╬══════╬══════╬══════╬══════╬══════╬══════╣
/// ║           ║                   ║      ║      ║      ║      ║      ║      ║      ║      ║
/// ║ byte 3    ║ ‘M’               ║ 0    ║ 1    ║ 0    ║ 0    ║ 1    ║ 1    ║ 0    ║ 1    ║
/// ╠═══════════╬═══════════════════╬══════╬══════╬══════╬══════╬══════╬══════╬══════╬══════╣
/// ║           ║                   ║      ║      ║      ║      ║      ║      ║      ║      ║
/// ║ byte 4    ║ ‘Q’               ║ 0    ║ 1    ║ 0    ║ 1    ║ 0    ║ 0    ║ 0    ║ 1    ║
/// ╠═══════════╬═══════════════════╬══════╬══════╬══════╬══════╬══════╬══════╬══════╬══════╣
/// ║           ║                   ║      ║      ║      ║      ║      ║      ║      ║      ║
/// ║ byte 5    ║ ‘T’               ║ 0    ║ 1    ║ 0    ║ 1    ║ 0    ║ 1    ║ 0    ║ 0    ║
/// ╠═══════════╬═══════════════════╬══════╬══════╬══════╬══════╬══════╬══════╬══════╬══════╣
/// ║           ║                   ║      ║      ║      ║      ║      ║      ║      ║      ║
/// ║ byte 6    ║ ‘T’               ║ 0    ║ 1    ║ 0    ║ 1    ║ 0    ║ 1    ║ 0    ║ 0    ║
/// ╚═══════════╩═══════════════════╩══════╩══════╩══════╩══════╩══════╩══════╩══════╩══════╝
/// 
/// Byte 7:
/// The protocol version
/// 
/// Byte 8:
/// 3.1.2.3 Connect Flags : 
/// ╔═════╦═══════════╦══════════╦═════════════╦═════╦════╦═══════════╦═════════════╦══════════╗
/// ║ Bit ║ 7         ║ 6        ║ 5           ║ 4   ║ 3  ║ 2         ║ 1           ║ 0        ║
/// ╠═════╬═══════════╬══════════╬═════════════╬═════╩════╬═══════════╬═════════════╬══════════╣
/// ║     ║ User Name ║ Password ║ Will Retain ║ Will QoS ║ Will Flag ║ Clean Start ║ Reserved ║
/// ╚═════╩═══════════╩══════════╩═════════════╩══════════╩═══════════╩═════════════╩══════════╝
/// 
/// Byte 9 and 10:
/// The keep alive
/// 
/// Byte 11:
/// Length of [`ConnectProperties`]
/// 
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connect{
    /// Byte 7
    pub protocol_version: ProtocolVersion,
    
    /// 3.1.2.4 Clean Start Flag
    /// bit 1
    pub clean_session: bool,

    /// 3.1.2.5 Will Flag through option
    pub last_will: Option<LastWill>,
    
    /// 3.1.2.8 User Name Flag
    pub username: Option<String>,
    /// 3.1.2.9 Password Flag
    pub password: Option<String>,
    
    /// 3.1.2.10 Keep Alive
    /// Byte 9 and 10
    pub keep_alive: u16,

    /// 3.1.2.11 CONNECT Properties
    pub connect_properties: ConnectProperties,

    /// 3.1.3.1 Client Identifier (ClientID)
    pub client_id: String,

}

impl Connect{
    fn read(_: u8, _: usize,  buf: &mut Bytes) -> Result<Self, String> {
        if String::read(buf)? != "MQTT" {
            return Err("Protocol not MQTT".to_string());
        }
        
        let protocol_version = ProtocolVersion::read(buf)?;

        let connect_flags_byte = buf.get_u8();
        let connect_flags = ConnectFlags::from_bits(connect_flags_byte).ok_or("Can't read ConnectFlags".to_string())?;

        let clean_session = connect_flags.contains(ConnectFlags::CLEAN_START);
        let keep_alive = buf.get_u16();

        let connect_properties = ConnectProperties::read(buf)?;

        let client_id = String::read(buf)?;
        let mut last_will = None;
        if connect_flags.contains(ConnectFlags::WILL_FLAG){
            let retain = connect_flags.contains(ConnectFlags::WILL_RETAIN);

            let qos = QoS::try_from(connect_flags)?;

            last_will = Some(LastWill::read(qos, retain, buf)?);
        }

        let username;
        let password;
        if connect_flags.contains(ConnectFlags::USERNAME){
            username = Some(String::read(buf)?);
        }
        else{
            username = None;
        }
        if connect_flags.contains(ConnectFlags::PASSWORD){
            password = Some(String::read(buf)?);
        }
        else{
            password = None;
        }

        let connect = Connect {
            protocol_version,
            clean_session,
            last_will,
            username,
            password,
            keep_alive,
            connect_properties,
            client_id,
        };

        Ok(connect)
    }

    fn write(&self, buf: &mut BytesMut){
        "MQTT".write(buf);

        self.protocol_version.write(buf);

        let mut connect_flags = ConnectFlags::empty();

        if self.clean_session{
            connect_flags |= ConnectFlags::CLEAN_START;
        }
        if let Some(last_will) = &self.last_will {
            connect_flags |= ConnectFlags::WILL_FLAG;
            if last_will.retain{
                connect_flags |= ConnectFlags::WILL_RETAIN;
            }
            connect_flags |= last_will.qos.into();
        }
        if self.username.is_some(){
            connect_flags |= ConnectFlags::USERNAME;
        }
        if self.password.is_some(){
            connect_flags |= ConnectFlags::PASSWORD;
        }

        buf.put_u8(connect_flags.bits());

        buf.put_u16(self.keep_alive);

        self.connect_properties.write(buf);

        self.client_id.write(buf);

        if let Some(last_will) = &self.last_will{
            last_will.write(buf);
        }
        if let Some(username) = &self.username{
            username.write(buf);
        }
        if let Some(password) = &self.password{
            password.write(buf);
        }
    }


    fn write_bits(&self, buf: &mut BytesMut){
        "MQTT".write(buf);

        self.protocol_version.write(buf);

        let mut connect_flags = 0u8;

        if self.clean_session{
            connect_flags |= 0b00000010;
        }
        if let Some(last_will) = &self.last_will {
            connect_flags |= 0b00000100;
            if last_will.retain{
                connect_flags |= 0b00100000;
            }
            connect_flags |=  (last_will.qos as u8) << 3;

        }
        if self.username.is_some(){
            connect_flags |= 0b10000000;
        }
        if self.password.is_some(){
            connect_flags |= 0b01000000;
        }

        buf.put_u8(connect_flags);

        buf.put_u16(self.keep_alive);

        self.connect_properties.write(buf);

        self.client_id.write(buf);

        if let Some(last_will) = &self.last_will{
            last_will.write(buf);
        }
        if let Some(username) = &self.username{
            username.write(buf);
        }
        if let Some(password) = &self.password{
            password.write(buf);
        }
    }
}

bitflags! {
    /// ╔═════╦═══════════╦══════════╦═════════════╦═════╦════╦═══════════╦═════════════╦══════════╗
    /// ║ Bit ║ 7         ║ 6        ║ 5           ║ 4   ║ 3  ║ 2         ║ 1           ║ 0        ║
    /// ╠═════╬═══════════╬══════════╬═════════════╬═════╩════╬═══════════╬═════════════╬══════════╣
    /// ║     ║ User Name ║ Password ║ Will Retain ║ Will QoS ║ Will Flag ║ Clean Start ║ Reserved ║
    /// ╚═════╩═══════════╩══════════╩═════════════╩══════════╩═══════════╩═════════════╩══════════╝
    pub struct ConnectFlags: u8 {
        const CLEAN_START   = 0b00000010;
        const WILL_FLAG     = 0b00000100;
        const WILL_QOS1     = 0b00001000;
        const WILL_QOS2     = 0b00010000;
        const WILL_RETAIN   = 0b00100000;
        const PASSWORD      = 0b01000000;
        const USERNAME      = 0b10000000;
    }
}

impl From<QoS> for ConnectFlags{
    fn from(q: QoS) -> Self {
        match q {
            QoS::AtMostOnce => ConnectFlags::empty(),
            QoS::AtLeastOnce => ConnectFlags::WILL_QOS1,
            QoS::ExactlyOnce => ConnectFlags::WILL_QOS2,
        }
    }
}


/// Connect Properties
/// 
/// The wire representation starts with the length of all properties after which 
/// the identifiers and their actual value are given
/// 
/// 3.1.2.11.1 Property Length
/// The length of the Properties in the CONNECT packet Variable Header encoded as a Variable Byte Integer.
/// Followed by all possible connect properties:
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectProperties {
    /// 3.1.2.11.2 Session Expiry Interval
    /// 17 (0x11) Byte Identifier of the Session Expiry Interval
    pub session_expiry_interval: Option<u32>,
    
    /// 3.1.2.11.3 Receive Maximum
    /// 33 (0x21) Byte, Identifier of the Receive Maximum
    pub receive_maximum: Option<u16>,

    /// 3.1.2.11.4 Maximum Packet Size
    /// 39 (0x27) Byte, Identifier of the Maximum Packet Size
    pub maximum_packet_size: Option<u32>,

    /// 3.1.2.11.5 Topic Alias Maximum
    /// 34 (0x22) Byte, Identifier of the Topic Alias Maximum
    pub topic_alias_maximum: Option<u16>,

    /// 3.1.2.11.6 Request Response Information
    /// 25 (0x19) Byte, Identifier of the Request Response Information
    pub request_response_information: Option<u8>,

    /// 3.1.2.11.7 Request Problem Information
    /// 23 (0x17) Byte, Identifier of the Request Problem Information
    pub request_problem_information: Option<u8>,

    /// 3.1.2.11.8 User Property
    /// 38 (0x26) Byte, Identifier of the User Property
    pub user_properties: Vec<(String, String)>,

    /// 3.1.2.11.9 Authentication Method
    /// 21 (0x15) Byte, Identifier of the Authentication Method
    pub authentication_method: Option<String>,

    /// 3.1.2.11.10 Authentication Data
    /// 22 (0x16) Byte, Identifier of the Authentication Data
    pub authentication_data: Bytes,
}

impl ConnectProperties{
    fn _new() -> Self{
        Self{
            session_expiry_interval: None,
            receive_maximum: None,
            maximum_packet_size: None,
            topic_alias_maximum: None,
            request_response_information: None,
            request_problem_information: None,
            user_properties: vec![],
            authentication_method: None,
            authentication_data: Bytes::new(),
        }
    }

    pub fn read(buf: &mut Bytes) -> Result<Self, String>{
        let (len, _) = read_variable_integer(buf)?;
        
        if len == 0 {
            return Ok(Self::_new());
        }
        else if buf.len() < len{
            return Err("Not enough data for the given length".to_string());
        }

        let mut property_data =  buf.split_to(len);
        
        let mut properties = Self::_new();
        loop{
            match PropertyType::from_u8(property_data.get_u8())? {
                PropertyType::SessionExpiryInterval => {
                    if properties.session_expiry_interval.is_some(){
                        return Err("Duplicate Session expiry interval".to_string());
                    }
                    properties.session_expiry_interval = Some(property_data.get_u32());
                },
                PropertyType::ReceiveMaximum => {
                    if properties.receive_maximum.is_some(){
                        return Err("Duplicate receive maximum".to_string());
                    }
                    properties.receive_maximum = Some(property_data.get_u16());
                },
                PropertyType::MaximumPacketSize => {
                    if properties.maximum_packet_size.is_some(){
                        return Err("Duplicate maximum packet size".to_string());
                    }
                    properties.maximum_packet_size = Some(property_data.get_u32());
                },
                PropertyType::TopicAliasMaximum => {
                    if properties.topic_alias_maximum.is_some(){
                        return Err("Duplicate topic alias maximum".to_string());
                    }
                    properties.topic_alias_maximum = Some(property_data.get_u16());
                },
                PropertyType::RequestResponseInformation => {
                    if properties.request_response_information.is_some(){
                        return Err("Duplicate maximum packet size".to_string());
                    }
                    properties.request_response_information = Some(property_data.get_u8());
                },
                PropertyType::RequestProblemInformation => {
                    if properties.request_problem_information.is_some(){
                        return Err("Duplicate request_problem_information".to_string());
                    }
                    properties.request_problem_information = Some(property_data.get_u8());
                },
                PropertyType::UserProperty => {
                    properties.user_properties.push((String::read(&mut property_data)?,String::read(&mut property_data)?))
                },
                PropertyType::AuthenticationMethod => {
                    if properties.authentication_method.is_some(){
                        return Err("Duplicate authentication_method".to_string());
                    }
                    properties.authentication_method = Some(String::read(&mut property_data)?);
                },
                PropertyType::AuthenticationData => {
                    if properties.authentication_data.is_empty(){
                        return Err("Duplicate authentication_data".to_string());
                    }
                    properties.authentication_data = Bytes::read(&mut property_data)?;
                },
                e => return Err(format!("Unexpected property type: {:?}", e)),
            }
        
            if property_data.is_empty(){
                break;
            }
        }

        if !properties.authentication_data.is_empty() && properties.authentication_method.is_none(){
            return Err("Authentication data is not empty while authentication method is".to_string());
        }

        Ok(properties)
    }

    pub fn write(&self, buf: &mut BytesMut){
        write_variable_integer(buf, self.wire_len());

        if let Some(session_expiry_interval) = self.session_expiry_interval{
            buf.put_u8(PropertyType::SessionExpiryInterval.to_u8());
            buf.put_u32(session_expiry_interval);
        }
        if let Some(receive_maximum) = self.receive_maximum{
            buf.put_u8(PropertyType::ReceiveMaximum.to_u8());
            buf.put_u16(receive_maximum);
        }
        if let Some(maximum_packet_size) = self.maximum_packet_size{
            buf.put_u8(PropertyType::MaximumPacketSize.to_u8());
            buf.put_u32(maximum_packet_size);
        }
        if let Some(topic_alias_maximum) = self.topic_alias_maximum{
            buf.put_u8(PropertyType::TopicAliasMaximum.to_u8());
            buf.put_u16(topic_alias_maximum);
        }
        if let Some(request_response_information) = self.request_response_information{
            buf.put_u8(PropertyType::RequestResponseInformation.to_u8());
            buf.put_u8(request_response_information);
        }
        if let Some(request_problem_information) = self.request_problem_information{
            buf.put_u8(PropertyType::RequestProblemInformation.to_u8());
            buf.put_u8(request_problem_information);
        }
        for (key, value) in &self.user_properties{
            String::write(key, buf);
            String::write(value, buf);
        }
        if let Some(authentication_method) = &self.authentication_method{
            String::write(authentication_method, buf);
        }
        if !self.authentication_data.is_empty() && self.authentication_method.is_some(){
            self.authentication_data.write(buf);
        }

    }

    pub fn wire_len(&self) -> usize{
        let mut len:usize = 0;

        if self.session_expiry_interval.is_some(){
            len+=1+4;
        }
        if self.receive_maximum.is_some(){
            len+=1+2;
        }
        if self.maximum_packet_size.is_some(){
            len+=1+4;
        }
        if self.topic_alias_maximum.is_some(){
            len+=1+4;
        }
        if self.request_response_information.is_some(){
            len+=2;
        }
        if self.request_problem_information.is_some(){
            len+=2;
        }
        for (key, value) in &self.user_properties{
            len += key.wire_len();
            len += value.wire_len();
        }
        if let Some(authentication_method) = &self.authentication_method{
            len += authentication_method.wire_len();
        }
        if !self.authentication_data.is_empty() && self.authentication_method.is_some(){
            len += self.authentication_data.wire_len();
        }

        len
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastWill{
    /// 3.1.2.6 Will QoS
    pub qos: QoS,
    /// 3.1.2.7 Will Retain
    pub retain: bool,

    /// 3.1.3.2 Will properties
    pub last_will_properties: LastWillProperties,
    /// 3.1.3.3 Will Topic
    pub topic: String,
    /// 3.1.3.4 Will payload
    pub payload: Bytes,
}

impl LastWill {
    pub fn new<T: Into<String>, P: Into<Vec<u8>>>(qos: QoS, retain: bool, topic: T, payload: P) -> LastWill {
        Self{
            qos,
            retain,
            last_will_properties: LastWillProperties::new_empty(),
            topic: topic.into(),
            payload: Bytes::from(payload.into()),
        }
    }

    pub fn read(qos: QoS, retain: bool, buf: &mut Bytes) -> Result<Self, String>{
        let last_will_properties = LastWillProperties::read(buf)?;
        let topic = String::read(buf)?;
        let payload = Bytes::read(buf)?;
        
        Ok(Self{
            qos,
            retain,
            topic,
            payload,
            last_will_properties,
        })
    }

    pub fn write(&self, buf: &mut BytesMut){
        self.last_will_properties.write(buf);
        self.topic.write(buf);
        self.payload.write(buf);
    }

    /// The length of a will information is just the Topic, Payload and properties
    /// The QoS and Retain information is stored in byte 8 of the variable connect header.
    pub fn wire_len(&self) -> usize{
        self.topic.wire_len() + self.payload.wire_len() + self.last_will_properties.wire_len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastWillProperties{
    /// 3.1.3.2.2 Will Delay Interval
    delay_interval: Option<u32>,
    /// 3.1.3.2.3 Payload Format Indicator
    payload_format_indicator: Option<u8>,
    /// 3.1.3.2.4 Message Expiry Interval
    message_expiry_interval: Option<u32>,
    /// 3.1.3.2.5 Content Type
    content_type: Option<String>,
    /// 3.1.3.2.6 Response Topic
    response_topic: Option<String>,
    /// 3.1.3.2.7 Correlation Data
    correlation_data: Option<Bytes>,
    /// 3.1.3.2.8 User Property
    user_properties: Vec::<(String, String)>,
}

impl LastWillProperties{
    pub fn new_empty() -> Self{
        Self{
            delay_interval: None,
            payload_format_indicator: None,
            message_expiry_interval: None,
            content_type: None,
            response_topic: None,
            correlation_data: None,
            user_properties: vec![],
        }
    }

    pub fn read(buf: &mut Bytes) -> Result<Self, String>{
        let (len, _) = read_variable_integer(buf)?;
        
        if len == 0 {
            return Ok(Self::new_empty());
        }
        else if buf.len() < len{
            return Err("Not enough data for the given length".to_string());
        }

        let mut property_data =  buf.split_to(len);
        
        let mut properties = Self::new_empty();

        loop{
            match PropertyType::from_u8(property_data.get_u8())? {
                PropertyType::WillDelayInterval => {
                    if properties.delay_interval.is_some(){
                        return Err("Duplicate delay_interval".to_string());
                    }
                    properties.delay_interval = Some(property_data.get_u32());
                },
                PropertyType::PayloadFormatIndicator => {
                    if properties.payload_format_indicator.is_none(){
                        return Err("Duplicate is_utf8_encoded_data".to_string());
                    }
                    properties.payload_format_indicator = Some(property_data.get_u8());
                },
                PropertyType::MessageExpiryInterval => {
                    if properties.message_expiry_interval.is_some(){
                        return Err("Duplicate message_expiry_interval".to_string());
                    }
                    properties.message_expiry_interval = Some(property_data.get_u32());
                },
                PropertyType::ContentType => {
                    if properties.content_type.is_some(){
                        return Err("Duplicate content_type".to_string());
                    }
                    properties.content_type = Some(String::read(&mut property_data)?);
                },
                PropertyType::ResponseTopic => {
                    if properties.response_topic.is_some(){
                        return Err("Duplicate response_topic".to_string());
                    }
                    properties.response_topic = Some(String::read(&mut property_data)?);
                },
                PropertyType::CorrelationData => {
                    if properties.correlation_data.is_some(){
                        return Err("Duplicate correlation_data".to_string());
                    }
                    properties.correlation_data = Some(Bytes::read(&mut property_data)?);
                },
                PropertyType::UserProperty => {
                    properties.user_properties.push((String::read(&mut property_data)?,String::read(&mut property_data)?))
                },
                e => return Err(format!("Unexpected property type: {:?}", e)),
            }
        
            if property_data.is_empty(){
                break;
            }
        }

        Ok(properties)
    }

    pub fn write(&self, buf: &mut BytesMut){
        write_variable_integer(buf, self.wire_len());

        if let Some(delay_interval) = self.delay_interval{
            buf.put_u8(PropertyType::WillDelayInterval.to_u8());
            buf.put_u32(delay_interval);
        }
        if let Some(payload_format_indicator) = self.payload_format_indicator{
            buf.put_u8(PropertyType::PayloadFormatIndicator.to_u8());
            buf.put_u8(payload_format_indicator);
        }
        if let Some(message_expiry_interval) = self.message_expiry_interval{
            buf.put_u8(PropertyType::MessageExpiryInterval.to_u8());
            buf.put_u32(message_expiry_interval);
        }
        if let Some(content_type) = &self.content_type{
            buf.put_u8(PropertyType::ContentType.to_u8());
            content_type.write(buf);
        }
        if let Some(response_topic) = &self.response_topic{
            buf.put_u8(PropertyType::ResponseTopic.to_u8());
            response_topic.write(buf);
        }
        if let Some(correlation_data) = &self.correlation_data{
            buf.put_u8(PropertyType::CorrelationData.to_u8());
            correlation_data.write(buf);
        }
        if !self.user_properties.is_empty(){
            for (key, value) in &self.user_properties{
                buf.put_u8(PropertyType::UserProperty.to_u8());
                key.write(buf);
                value.write(buf);
            } 
        }
    }

    pub fn wire_len(&self) -> usize{
        let mut len:usize = 0;

        len += self.delay_interval.map_or(0, |_| 5);
        len += self.payload_format_indicator.map_or(0, |_| 2);
        len += self.message_expiry_interval.map_or(0, |_| 5);
        len += self.content_type.as_ref().map_or_else(|| 0, |s| s.wire_len());
        len += self.response_topic.as_ref().map_or_else(|| 0, |s| s.wire_len());
        len += self.correlation_data.as_ref().map_or_else(|| 0, |b| b.wire_len());
        for (key, value) in &self.user_properties{
            len += key.wire_len() + value.wire_len();
        }
        
        len
    }
}

#[cfg(test)]
mod tests{
    use crate::packets::QoS;

    use super::{Connect, LastWill};

    #[test]
    fn read_connect(){
        let mut buf = bytes::BytesMut::new();
        let packet = &[
            // 0x10,
            // 39, // packet type, flags and remaining len
            0x00,
            0x04,
            b'M',
            b'Q',
            b'T',
            b'T',
            0x05,
            0b1100_1110, // Connect Flags, username, password, will retain=false, will qos=1, last_will, clean_session
            0x00, // Keep alive = 10 sec
            0x0a, 
            0x00, // Length of Connect properties
            0x00, // client_id length
            0x04, 
            b't', // client_id
            b'e', 
            b's', 
            b't', 
            0x00, // Will properties length
            0x00, // length topic
            0x02, 
            b'/', // Will topic = '/a'
            b'a', 
            0x00, // Will payload length 
            0x0B, 
            b'h', // Will payload = 'hello world'
            b'e',
            b'l',
            b'l',
            b'o',
            b' ',
            b'w',
            b'o',
            b'r',
            b'l',
            b'd',
            0x00, // length username
            0x04,
            b'u', // username = 'user'
            b's',
            b'e',
            b'r', 
            0x00, // length password
            0x04,
            b'p', // Password = 'pass'
            b'a', 
            b's', 
            b's', 
            0xAB, // extra packets in the stream
            0xCD,
            0xEF,
        ];

        buf.extend_from_slice(packet);
        let c = Connect::read(0, 0, &mut buf.into()).unwrap();

        dbg!(c);

    }

    #[test]
    fn read_and_write_connect(){
        let mut buf = bytes::BytesMut::new();
        let packet = &[
            // 0x10,
            // 39, // packet type, flags and remaining len
            0x00,
            0x04,
            b'M',
            b'Q',
            b'T',
            b'T',
            0x05,        // variable header
            0b1100_1110, // variable header. +username, +password, -will retain, will qos=1, +last_will, +clean_session
            0x00, // Keep alive = 10 sec
            0x0a, 
            0x00, // Length of Connect properties
            0x00, // client_id length
            0x04, 
            b't', // client_id
            b'e', 
            b's', 
            b't', 
            0x00, // Will properties length
            0x00, // length topic
            0x02, 
            b'/', // Will topic = '/a'
            b'a', 
            0x00, // Will payload length 
            0x0B, 
            b'h', // Will payload = 'hello world'
            b'e',
            b'l',
            b'l',
            b'o',
            b' ',
            b'w',
            b'o',
            b'r',
            b'l',
            b'd',
            0x00, // length username
            0x04,
            b'u', // username
            b's',
            b'e',
            b'r', 
            0x00, // length password
            0x04,
            b'p', // payload. password = 'pass'
            b'a', 
            b's', 
            b's', 
        ];

        buf.extend_from_slice(packet);
        let c = Connect::read(0, 0, &mut buf.into()).unwrap();

        let mut write_buf = bytes::BytesMut::new();
        c.write(&mut write_buf);

        assert_eq!(packet.to_vec(), write_buf.to_vec());

        dbg!(c);

    }

    #[test]
    fn parsing_last_will(){
        let last_will = &[
            0x00, // Will properties length
            0x00, // length topic
            0x02, 
            b'/', // Will topic = '/a'
            b'a', 
            0x00, // Will payload length 
            0x0B, 
            b'h', // Will payload = 'hello world'
            b'e',
            b'l',
            b'l',
            b'o',
            b' ',
            b'w',
            b'o',
            b'r',
            b'l',
            b'd',
        ];
        let mut buf = bytes::Bytes::from_static(last_will);

        dbg!(LastWill::read(QoS::AtLeastOnce, false, &mut buf));

    }

    #[test]
    fn parsing_and_writing_last_will(){
        let last_will = &[
            0x00, // Will properties length
            0x00, // length topic
            0x02, 
            b'/', // Will topic = '/a'
            b'a', 
            0x00, // Will payload length 
            0x0B, 
            b'h', // Will payload = 'hello world'
            b'e',
            b'l',
            b'l',
            b'o',
            b' ',
            b'w',
            b'o',
            b'r',
            b'l',
            b'd',
        ];
        let mut buf = bytes::Bytes::from_static(last_will);

        let lw = LastWill::read(QoS::AtLeastOnce, false, &mut buf).unwrap();

        let mut write_buf = bytes::BytesMut::new();
        lw.write(&mut write_buf);

        assert_eq!(last_will.to_vec(), write_buf.to_vec());
    }
}