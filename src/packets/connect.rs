use bytes::{Buf, BufMut, Bytes, BytesMut};

use super::{
    error::{DeserializeError, SerializeError},
    mqtt_traits::{MqttRead, MqttWrite, VariableHeaderRead, VariableHeaderWrite},
    read_variable_integer, variable_integer_len, write_variable_integer, PacketType, PropertyType,
    ProtocolVersion, QoS, WireLength,
};

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
pub struct Connect {
    /// Byte 7
    pub protocol_version: ProtocolVersion,

    /// 3.1.2.4 Clean Start Flag
    /// bit 1
    pub clean_start: bool,

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

impl Default for Connect {
    fn default() -> Self {
        Self {
            protocol_version: ProtocolVersion::V5,
            clean_start: true,
            last_will: None,
            username: None,
            password: None,
            keep_alive: 60,
            connect_properties: ConnectProperties::default(),
            client_id: "MQRSTT".to_string(),
        }
    }
}

impl VariableHeaderRead for Connect {
    fn read(_: u8, _: usize, mut buf: Bytes) -> Result<Self, DeserializeError> {
        if String::read(&mut buf)? != "MQTT" {
            return Err(DeserializeError::MalformedPacketWithInfo(
                "Protocol not MQTT".to_string(),
            ));
        }

        let protocol_version = ProtocolVersion::read(&mut buf)?;

        let connect_flags = ConnectFlags::read(&mut buf)?;

        let clean_start = connect_flags.clean_start;
        let keep_alive = buf.get_u16();

        let connect_properties = ConnectProperties::read(&mut buf)?;

        let client_id = String::read(&mut buf)?;
        let mut last_will = None;
        if connect_flags.will_flag {
            let retain = connect_flags.will_retain;

            last_will = Some(LastWill::read(connect_flags.will_qos, retain, &mut buf)?);
        }

        let username = if connect_flags.username {
            Some(String::read(&mut buf)?)
        } else {
            None
        };
        let password = if connect_flags.password {
            Some(String::read(&mut buf)?)
        } else {
            None
        };

        let connect = Connect {
            protocol_version,
            clean_start,
            last_will,
            username,
            password,
            keep_alive,
            connect_properties,
            client_id,
        };

        Ok(connect)
    }
}

impl VariableHeaderWrite for Connect {
    fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError> {
        "MQTT".write(buf)?;

        self.protocol_version.write(buf)?;

        let mut connect_flags = ConnectFlags {
            clean_start: self.clean_start,
            ..Default::default()
        };

        if let Some(last_will) = &self.last_will {
            connect_flags.will_flag = true;
            connect_flags.will_retain = last_will.retain;
            connect_flags.will_qos = last_will.qos;
        }
        connect_flags.username = self.username.is_some();
        connect_flags.password = self.password.is_some();

        connect_flags.write(buf)?;

        buf.put_u16(self.keep_alive);

        self.connect_properties.write(buf)?;

        self.client_id.write(buf)?;

        if let Some(last_will) = &self.last_will {
            last_will.write(buf)?;
        }
        if let Some(username) = &self.username {
            username.write(buf)?;
        }
        if let Some(password) = &self.password {
            password.write(buf)?;
        }
        Ok(())
    }
}

impl WireLength for Connect {
    fn wire_len(&self) -> usize {
        let mut len = "MQTT".wire_len() + 1 + 1 + 2; // protocol version, connect_flags and keep alive

        len += variable_integer_len(self.connect_properties.wire_len());
        len += self.connect_properties.wire_len();

        if let Some(last_will) = &self.last_will {
            len += last_will.wire_len();
        }
        if let Some(username) = &self.username {
            len += username.wire_len()
        }
        if let Some(password) = &self.password {
            len += password.wire_len()
        }

        len += self.client_id.wire_len();

        len
    }
}

/// ╔═════╦═══════════╦══════════╦═════════════╦═════╦════╦═══════════╦═════════════╦══════════╗
/// ║ Bit ║ 7         ║ 6        ║ 5           ║ 4   ║ 3  ║ 2         ║ 1           ║ 0        ║
/// ╠═════╬═══════════╬══════════╬═════════════╬═════╩════╬═══════════╬═════════════╬══════════╣
/// ║     ║ User Name ║ Password ║ Will Retain ║ Will QoS ║ Will Flag ║ Clean Start ║ Reserved ║
/// ╚═════╩═══════════╩══════════╩═════════════╩══════════╩═══════════╩═════════════╩══════════╝
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ConnectFlags {
    pub clean_start: bool,
    pub will_flag: bool,
    pub will_qos: QoS,
    pub will_retain: bool,
    pub password: bool,
    pub username: bool,
}

impl ConnectFlags {
    pub fn from_u8(value: u8) -> Result<Self, DeserializeError> {
        Ok(Self {
            clean_start: ((value & 0b00000010) >> 1) != 0,
            will_flag: ((value & 0b00000100) >> 2) != 0,
            will_qos: QoS::from_u8((value & 0b00011000) >> 3)?,
            will_retain: ((value & 0b00100000) >> 5) != 0,
            password: ((value & 0b01000000) >> 6) != 0,
            username: ((value & 0b10000000) >> 7) != 0,
        })
    }

    pub fn into_u8(&self) -> Result<u8, SerializeError> {
        let byte = ((self.clean_start as u8) << 1)
            | ((self.will_flag as u8) << 2)
            | (self.will_qos.into_u8() << 3)
            | ((self.will_retain as u8) << 5)
            | ((self.password as u8) << 6)
            | ((self.username as u8) << 7);
        Ok(byte)
    }
}

impl Default for ConnectFlags {
    fn default() -> Self {
        Self {
            clean_start: false,
            will_flag: false,
            will_qos: QoS::AtMostOnce,
            will_retain: false,
            password: false,
            username: false,
        }
    }
}

impl MqttRead for ConnectFlags {
    fn read(buf: &mut bytes::Bytes) -> Result<Self, DeserializeError> {
        if buf.is_empty() {
            return Err(DeserializeError::InsufficientData(
                "ConnectFlags".to_string(),
                0,
                1,
            ));
        }

        let byte = buf.get_u8();

        ConnectFlags::from_u8(byte)
    }
}

impl MqttWrite for ConnectFlags {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
        buf.put_u8(self.into_u8()?);
        Ok(())
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
#[derive(Debug, Default, Clone, PartialEq, Eq)]
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

impl MqttWrite for ConnectProperties {
    fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError> {
        write_variable_integer(buf, self.wire_len())?;

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
        if !self.authentication_data.is_empty() {
            if self.authentication_method.is_none() {
                return Err(SerializeError::AuthDataWithoutAuthMethod);
            }
            PropertyType::AuthenticationData.write(buf)?;
            self.authentication_data.write(buf)?;
        }

        Ok(())
    }
}

impl MqttRead for ConnectProperties {
    fn read(buf: &mut Bytes) -> Result<Self, DeserializeError> {
        let (len, _) = read_variable_integer(buf)?;

        let mut properties = Self::default();
        if len == 0 {
            return Ok(properties);
        } else if buf.len() < len {
            return Err(DeserializeError::InsufficientData(
                "ConnectProperties".to_string(),
                buf.len(),
                len,
            ));
        }

        let mut property_data = buf.split_to(len);

        loop {
            match PropertyType::read(&mut property_data)? {
                PropertyType::SessionExpiryInterval => {
                    if properties.session_expiry_interval.is_some() {
                        return Err(DeserializeError::DuplicateProperty(
                            PropertyType::SessionExpiryInterval,
                        ));
                    }
                    properties.session_expiry_interval = Some(property_data.get_u32());
                }
                PropertyType::ReceiveMaximum => {
                    if properties.receive_maximum.is_some() {
                        return Err(DeserializeError::DuplicateProperty(
                            PropertyType::ReceiveMaximum,
                        ));
                    }
                    properties.receive_maximum = Some(property_data.get_u16());
                }
                PropertyType::MaximumPacketSize => {
                    if properties.maximum_packet_size.is_some() {
                        return Err(DeserializeError::DuplicateProperty(
                            PropertyType::MaximumPacketSize,
                        ));
                    }
                    properties.maximum_packet_size = Some(property_data.get_u32());
                }
                PropertyType::TopicAliasMaximum => {
                    if properties.topic_alias_maximum.is_some() {
                        return Err(DeserializeError::DuplicateProperty(
                            PropertyType::TopicAliasMaximum,
                        ));
                    }
                    properties.topic_alias_maximum = Some(property_data.get_u16());
                }
                PropertyType::RequestResponseInformation => {
                    if properties.request_response_information.is_some() {
                        return Err(DeserializeError::DuplicateProperty(
                            PropertyType::RequestResponseInformation,
                        ));
                    }
                    properties.request_response_information = Some(property_data.get_u8());
                }
                PropertyType::RequestProblemInformation => {
                    if properties.request_problem_information.is_some() {
                        return Err(DeserializeError::DuplicateProperty(
                            PropertyType::RequestProblemInformation,
                        ));
                    }
                    properties.request_problem_information = Some(property_data.get_u8());
                }
                PropertyType::UserProperty => properties.user_properties.push((
                    String::read(&mut property_data)?,
                    String::read(&mut property_data)?,
                )),
                PropertyType::AuthenticationMethod => {
                    if properties.authentication_method.is_some() {
                        return Err(DeserializeError::DuplicateProperty(
                            PropertyType::AuthenticationMethod,
                        ));
                    }
                    properties.authentication_method = Some(String::read(&mut property_data)?);
                }
                PropertyType::AuthenticationData => {
                    if properties.authentication_data.is_empty() {
                        return Err(DeserializeError::DuplicateProperty(
                            PropertyType::AuthenticationData,
                        ));
                    }
                    properties.authentication_data = Bytes::read(&mut property_data)?;
                }
                e => return Err(DeserializeError::UnexpectedProperty(e, PacketType::Connect)),
            }

            if property_data.is_empty() {
                break;
            }
        }

        if !properties.authentication_data.is_empty() && properties.authentication_method.is_none()
        {
            return Err(DeserializeError::MalformedPacketWithInfo(
                "Authentication data is not empty while authentication method is".to_string(),
            ));
        }

        Ok(properties)
    }
}

impl WireLength for ConnectProperties {
    fn wire_len(&self) -> usize {
        let mut len: usize = 0;

        if self.session_expiry_interval.is_some() {
            len += 1 + 4;
        }
        if self.receive_maximum.is_some() {
            len += 1 + 2;
        }
        if self.maximum_packet_size.is_some() {
            len += 1 + 4;
        }
        if self.topic_alias_maximum.is_some() {
            len += 1 + 2;
        }
        if self.request_response_information.is_some() {
            len += 2;
        }
        if self.request_problem_information.is_some() {
            len += 2;
        }
        for (key, value) in &self.user_properties {
            len += 1;
            len += key.wire_len();
            len += value.wire_len();
        }
        if let Some(authentication_method) = &self.authentication_method {
            len += 1 + authentication_method.wire_len();
        }
        if !self.authentication_data.is_empty() && self.authentication_method.is_some() {
            len += 1 + self.authentication_data.wire_len();
        }

        len
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastWill {
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
    pub fn new<T: Into<String>, P: Into<Vec<u8>>>(
        qos: QoS,
        retain: bool,
        topic: T,
        payload: P,
    ) -> LastWill {
        Self {
            qos,
            retain,
            last_will_properties: LastWillProperties::default(),
            topic: topic.into(),
            payload: Bytes::from(payload.into()),
        }
    }
    pub fn read(qos: QoS, retain: bool, buf: &mut Bytes) -> Result<Self, DeserializeError> {
        let last_will_properties = LastWillProperties::read(buf)?;
        let topic = String::read(buf)?;
        let payload = Bytes::read(buf)?;

        Ok(Self {
            qos,
            retain,
            topic,
            payload,
            last_will_properties,
        })
    }
}

impl MqttWrite for LastWill {
    fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError> {
        self.last_will_properties.write(buf)?;
        self.topic.write(buf)?;
        self.payload.write(buf)?;
        Ok(())
    }
}

impl WireLength for LastWill {
    fn wire_len(&self) -> usize {
        let property_len = self.last_will_properties.wire_len();

        self.topic.wire_len()
            + self.payload.wire_len()
            + variable_integer_len(property_len)
            + property_len
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LastWillProperties {
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
    user_properties: Vec<(String, String)>,
}

impl MqttRead for LastWillProperties {
    fn read(buf: &mut Bytes) -> Result<Self, DeserializeError> {
        let (len, _) = read_variable_integer(buf)?;

        let mut properties = Self::default();
        if len == 0 {
            return Ok(properties);
        } else if buf.len() < len {
            return Err(DeserializeError::InsufficientData(
                "LastWillProperties".to_string(),
                buf.len(),
                len,
            ));
        }

        let mut property_data = buf.split_to(len);

        loop {
            match PropertyType::read(&mut property_data)? {
                PropertyType::WillDelayInterval => {
                    if properties.delay_interval.is_some() {
                        return Err(DeserializeError::DuplicateProperty(
                            PropertyType::WillDelayInterval,
                        ));
                    }
                    properties.delay_interval = Some(u32::read(&mut property_data)?);
                }
                PropertyType::PayloadFormatIndicator => {
                    if properties.payload_format_indicator.is_none() {
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
                PropertyType::ContentType => {
                    if properties.content_type.is_some() {
                        return Err(DeserializeError::DuplicateProperty(
                            PropertyType::ContentType,
                        ));
                    }
                    properties.content_type = Some(String::read(&mut property_data)?);
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
                PropertyType::UserProperty => properties.user_properties.push((
                    String::read(&mut property_data)?,
                    String::read(&mut property_data)?,
                )),
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
        write_variable_integer(buf, self.wire_len())?;

        if let Some(delay_interval) = self.delay_interval {
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

impl WireLength for LastWillProperties {
    fn wire_len(&self) -> usize {
        let mut len: usize = 0;

        if self.delay_interval.is_some() {
            len += 5;
        }
        if self.payload_format_indicator.is_some() {
            len += 2;
        }
        if self.message_expiry_interval.is_some() {
            len += 5;
        }
        // +1 for the property type
        len += self
            .content_type
            .as_ref()
            .map_or_else(|| 0, |s| s.wire_len() + 1);
        len += self
            .response_topic
            .as_ref()
            .map_or_else(|| 0, |s| s.wire_len() + 1);
        len += self
            .correlation_data
            .as_ref()
            .map_or_else(|| 0, |b| b.wire_len() + 1);
        for (key, value) in &self.user_properties {
            len += key.wire_len() + value.wire_len() + 1;
        }

        len
    }
}

#[cfg(test)]
mod tests {
    use crate::packets::{
        mqtt_traits::{MqttWrite, VariableHeaderRead, VariableHeaderWrite},
        QoS,
    };

    use super::{Connect, ConnectFlags, LastWill};

    #[test]
    fn read_connect() {
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
            0b1100_1110, // Connect Flags, username, password, will retain=false, will qos=1, last_will, clean_start
            0x00,        // Keep alive = 10 sec
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
        let c = Connect::read(0, 0, buf.into()).unwrap();

        dbg!(c);
    }

    #[test]
    fn read_and_write_connect() {
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
            0x00,        // Keep alive = 10 sec
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
        let c = Connect::read(0, 0, buf.into()).unwrap();

        let mut write_buf = bytes::BytesMut::new();
        c.write(&mut write_buf).unwrap();

        assert_eq!(packet.to_vec(), write_buf.to_vec());

        dbg!(c);
    }

    #[test]
    fn parsing_last_will() {
        let last_will = &[
            0x00, // Will properties length
            0x00, // length topic
            0x02, b'/', // Will topic = '/a'
            b'a', 0x00, // Will payload length
            0x0B, b'h', // Will payload = 'hello world'
            b'e', b'l', b'l', b'o', b' ', b'w', b'o', b'r', b'l', b'd',
        ];
        let mut buf = bytes::Bytes::from_static(last_will);

        assert!(LastWill::read(QoS::AtLeastOnce, false, &mut buf).is_ok());
    }

    #[test]
    fn read_and_write_connect2() {
        let _packet = [
            0x10, 0x1d, 0x00, 0x04, 0x4d, 0x51, 0x54, 0x54, 0x05, 0x80, 0x00, 0x3c, 0x05, 0x11,
            0xff, 0xff, 0xff, 0xff, 0x00, 0x05, 0x39, 0x2e, 0x30, 0x2e, 0x31, 0x00, 0x04, 0x54,
            0x65, 0x73, 0x74,
        ];

        let data = [
            0x00, 0x04, 0x4d, 0x51, 0x54, 0x54, 0x05, 0x80, 0x00, 0x3c, 0x05, 0x11, 0xff, 0xff,
            0xff, 0xff, 0x00, 0x05, 0x39, 0x2e, 0x30, 0x2e, 0x31, 0x00, 0x04, 0x54, 0x65, 0x73,
            0x74,
        ];

        let mut buf = bytes::BytesMut::new();
        buf.extend_from_slice(&data);

        let c = Connect::read(0, 0, buf.into()).unwrap();

        dbg!(c.clone());

        let mut write_buf = bytes::BytesMut::new();
        c.write(&mut write_buf).unwrap();

        assert_eq!(data.to_vec(), write_buf.to_vec());
    }

    #[test]
    fn parsing_and_writing_last_will() {
        let last_will = &[
            0x00, // Will properties length
            0x00, // length topic
            0x02, b'/', // Will topic = '/a'
            b'a', 0x00, // Will payload length
            0x0B, b'h', // Will payload = 'hello world'
            b'e', b'l', b'l', b'o', b' ', b'w', b'o', b'r', b'l', b'd',
        ];
        let mut buf = bytes::Bytes::from_static(last_will);

        let lw = LastWill::read(QoS::AtLeastOnce, false, &mut buf).unwrap();

        let mut write_buf = bytes::BytesMut::new();
        lw.write(&mut write_buf).unwrap();

        assert_eq!(last_will.to_vec(), write_buf.to_vec());
    }

    #[test]
    fn connect_flag() {
        let byte = 0b1100_1110;
        let flags = ConnectFlags::from_u8(byte).unwrap();
        assert_eq!(byte, flags.into_u8().unwrap());
    }
}
