pub mod error;
pub mod mqtt_traits;
pub mod publish;

pub mod auth;
pub mod connack;
pub mod connect;
pub mod disconnect;
pub mod packets;
pub mod puback;
pub mod pubcomp;
pub mod pubrec;
pub mod pubrel;
pub mod reason_codes;
pub mod suback;
pub mod subscribe;
pub mod unsuback;
pub mod unsubscribe;

use bytes::{Buf, BufMut, Bytes, BytesMut};
use core::slice::Iter;

use self::error::{DeserializeError, ReadBytes, SerializeError};
use self::mqtt_traits::{MqttRead, MqttWrite, WireLength};

use self::connect::ConnectFlags;
use self::packets::PacketType;

/// Protocol version
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum ProtocolVersion {
    V5,
}

impl MqttWrite for ProtocolVersion {
    fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError> {
        buf.put_u8(5u8);
        Ok(())
    }
}

impl MqttRead for ProtocolVersion {
    fn read(buf: &mut Bytes) -> Result<Self, DeserializeError> {
        if buf.is_empty() {
            return Err(DeserializeError::InsufficientDataForProtocolVersion);
        }

        match buf.get_u8() {
            3 => Err(DeserializeError::UnsupportedProtocolVersion),
            4 => Err(DeserializeError::UnsupportedProtocolVersion),
            5 => Ok(ProtocolVersion::V5),
            _ => Err(DeserializeError::UnknownProtocolVersion),
        }
    }
}

/// Quality of service
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum QoS {
    AtMostOnce = 0,
    AtLeastOnce = 1,
    ExactlyOnce = 2,
}
impl QoS {
    pub fn from_u8(value: u8) -> Result<Self, DeserializeError> {
        match value {
            0 => Ok(QoS::AtMostOnce),
            1 => Ok(QoS::AtLeastOnce),
            2 => Ok(QoS::ExactlyOnce),
            _ => Err(DeserializeError::UnknownQoS(value)),
        }
    }
    pub fn into_u8(self) -> u8 {
        match self {
            QoS::AtMostOnce => 0,
            QoS::AtLeastOnce => 1,
            QoS::ExactlyOnce => 2,
        }
    }
}

impl MqttRead for QoS {
    fn read(buf: &mut Bytes) -> Result<Self, DeserializeError> {
        if buf.is_empty() {
            return Err(DeserializeError::InsufficientData("QoS".to_string(), 0, 1));
        }

        match buf.get_u8() {
            0 => Ok(QoS::AtMostOnce),
            1 => Ok(QoS::AtLeastOnce),
            2 => Ok(QoS::ExactlyOnce),
            q => Err(DeserializeError::UnknownQoS(q)),
        }
    }
}

impl MqttWrite for QoS {
    fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError> {
        let val = match self {
            QoS::AtMostOnce => 0,
            QoS::AtLeastOnce => 1,
            QoS::ExactlyOnce => 2,
        };
        buf.put_u8(val);
        Ok(())
    }
}

impl TryFrom<ConnectFlags> for QoS {
    type Error = DeserializeError;

    fn try_from(c: ConnectFlags) -> Result<Self, Self::Error> {
        if c.contains(ConnectFlags::WILL_QOS1 | ConnectFlags::WILL_QOS2) {
            Err(DeserializeError::MalformedPacket)
        } else if c.contains(ConnectFlags::WILL_QOS2) {
            Ok(QoS::ExactlyOnce)
        } else if c.contains(ConnectFlags::WILL_QOS1) {
            Ok(QoS::AtLeastOnce)
        } else {
            Ok(QoS::AtMostOnce)
        }
    }
}

impl MqttWrite for &str {
    fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError> {
        buf.put_u16(self.len() as u16);
        buf.extend(self.as_bytes());
        Ok(())
    }
}

impl WireLength for &str {
    #[inline(always)]
    fn wire_len(&self) -> usize {
        self.len() + 2
    }
}

impl MqttRead for String {
    fn read(buf: &mut Bytes) -> Result<Self, DeserializeError> {
        let content = Bytes::read(buf)?;

        match String::from_utf8(content.to_vec()) {
            Ok(s) => Ok(s),
            Err(e) => Err(DeserializeError::Utf8Error(e)),
        }
    }
}

impl MqttWrite for String {
    fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError> {
        buf.put_u16(self.len() as u16);
        buf.extend(self.as_bytes());
        Ok(())
    }
}

impl WireLength for String {
    #[inline(always)]
    fn wire_len(&self) -> usize {
        self.len() + 2
    }
}

impl MqttRead for Bytes {
    fn read(buf: &mut Bytes) -> Result<Self, DeserializeError> {
        let len = buf.get_u16() as usize;

        if len > buf.len() {
            return Err(DeserializeError::InsufficientData(
                "Bytes".to_string(),
                buf.len(),
                len,
            ));
        }

        Ok(buf.split_to(len))
    }
}

impl MqttWrite for Bytes {
    fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError> {
        buf.put_u16(self.len() as u16);
        buf.extend(self);

        Ok(())
    }
}

impl WireLength for Bytes {
    #[inline(always)]
    fn wire_len(&self) -> usize {
        self.len() + 2
    }
}

impl MqttRead for bool {
    fn read(buf: &mut Bytes) -> Result<Self, error::DeserializeError> {
        if buf.is_empty() {
            return Err(DeserializeError::InsufficientData("bool".to_string(), 0, 1));
        }

        match buf.get_u8() {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(error::DeserializeError::MalformedPacket),
        }
    }
}

impl MqttWrite for bool {
    fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError> {
        if *self {
            buf.put_u8(1);
            Ok(())
        } else {
            buf.put_u8(0);
            Ok(())
        }
    }
}

impl MqttRead for u8 {
    fn read(buf: &mut Bytes) -> Result<Self, DeserializeError> {
        if buf.is_empty() {
            return Err(DeserializeError::InsufficientData("u8".to_string(), 0, 1));
        }
        Ok(buf.get_u8())
    }
}

impl MqttRead for u16 {
    fn read(buf: &mut Bytes) -> Result<Self, DeserializeError> {
        if buf.len() < 2 {
            return Err(DeserializeError::InsufficientData(
                "u16".to_string(),
                buf.len(),
                2,
            ));
        }
        Ok(buf.get_u16())
    }
}

impl MqttRead for u32 {
    fn read(buf: &mut Bytes) -> Result<Self, DeserializeError> {
        if buf.len() < 4 {
            return Err(DeserializeError::InsufficientData(
                "u32".to_string(),
                buf.len(),
                4,
            ));
        }
        Ok(buf.get_u32())
    }
}

pub fn read_fixed_header_rem_len(
    mut buf: Iter<u8>,
) -> Result<(usize, usize), ReadBytes<DeserializeError>> {
    let mut integer = 0;
    let mut length = 0;

    for i in 0..4 {
        if let Some(byte) = buf.next() {
            length += 1;
            integer += (*byte as usize & 0x7f) << 7 * i;

            if (*byte & 0b1000_0000) == 0 {
                return Ok((integer, length));
            }
        } else {
            return Err(ReadBytes::InsufficientBytes(1));
        }
    }
    Err(ReadBytes::Err(DeserializeError::MalformedPacket))
}

pub fn read_variable_integer(buf: &mut Bytes) -> Result<(usize, usize), DeserializeError> {
    let mut integer = 0;
    let mut length = 0;

    for i in 0..4 {
        if buf.is_empty() {
            return Err(DeserializeError::MalformedPacket);
        }
        length += 1;
        let byte = buf.get_u8();

        integer += (byte as usize & 0x7f) << 7 * i;

        if (byte & 0b1000_0000) == 0 {
            return Ok((integer, length));
        }
    }
    Err(DeserializeError::MalformedPacket)
}

pub fn write_variable_integer(buf: &mut BytesMut, integer: usize) -> Result<(), SerializeError> {
    if integer > 268_435_455 {
        return Err(SerializeError::VariableIntegerOverflow(integer));
    }

    let mut write = integer;

    for _ in 0..4 {
        let mut byte = (write % 128) as u8;
        write /= 128;
        if write > 0 {
            byte |= 128;
        }
        buf.put_u8(byte);
        if write == 0 {
            return Ok(());
        }
    }
    Err(SerializeError::VariableIntegerOverflow(integer))
}

pub fn variable_integer_len(integer: usize) -> usize {
    if integer >= 2_097_152 {
        4
    } else if integer >= 16_384 {
        3
    } else if integer >= 128 {
        2
    } else {
        1
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyType {
    PayloadFormatIndicator = 1,
    MessageExpiryInterval = 2,
    ContentType = 3,
    ResponseTopic = 8,
    CorrelationData = 9,
    SubscriptionIdentifier = 11,
    SessionExpiryInterval = 17,
    AssignedClientIdentifier = 18,
    ServerKeepAlive = 19,
    AuthenticationMethod = 21,
    AuthenticationData = 22,
    RequestProblemInformation = 23,
    WillDelayInterval = 24,
    RequestResponseInformation = 25,
    ResponseInformation = 26,
    ServerReference = 28,
    ReasonString = 31,
    ReceiveMaximum = 33,
    TopicAliasMaximum = 34,
    TopicAlias = 35,
    MaximumQos = 36,
    RetainAvailable = 37,
    UserProperty = 38,
    MaximumPacketSize = 39,
    WildcardSubscriptionAvailable = 40,
    SubscriptionIdentifierAvailable = 41,
    SharedSubscriptionAvailable = 42,
}

impl MqttRead for PropertyType {
    fn read(buf: &mut Bytes) -> Result<Self, DeserializeError> {
        if buf.is_empty() {
            return Err(DeserializeError::InsufficientData(
                "PropertyType".to_string(),
                0,
                1,
            ));
        }

        match buf.get_u8() {
            1 => Ok(Self::PayloadFormatIndicator),
            2 => Ok(Self::MessageExpiryInterval),
            3 => Ok(Self::ContentType),
            8 => Ok(Self::ResponseTopic),
            9 => Ok(Self::CorrelationData),
            11 => Ok(Self::SubscriptionIdentifier),
            17 => Ok(Self::SessionExpiryInterval),
            18 => Ok(Self::AssignedClientIdentifier),
            19 => Ok(Self::ServerKeepAlive),
            21 => Ok(Self::AuthenticationMethod),
            22 => Ok(Self::AuthenticationData),
            23 => Ok(Self::RequestProblemInformation),
            24 => Ok(Self::WillDelayInterval),
            25 => Ok(Self::RequestResponseInformation),
            26 => Ok(Self::ResponseInformation),
            28 => Ok(Self::ServerReference),
            31 => Ok(Self::ReasonString),
            33 => Ok(Self::ReceiveMaximum),
            34 => Ok(Self::TopicAliasMaximum),
            35 => Ok(Self::TopicAlias),
            36 => Ok(Self::MaximumQos),
            37 => Ok(Self::RetainAvailable),
            38 => Ok(Self::UserProperty),
            39 => Ok(Self::MaximumPacketSize),
            40 => Ok(Self::WildcardSubscriptionAvailable),
            41 => Ok(Self::SubscriptionIdentifierAvailable),
            42 => Ok(Self::SharedSubscriptionAvailable),
            t => Err(DeserializeError::UnknownProperty(t)),
        }
    }
}

impl MqttWrite for PropertyType {
    fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError> {
        let val = match self {
            Self::PayloadFormatIndicator => 1,
            Self::MessageExpiryInterval => 2,
            Self::ContentType => 3,
            Self::ResponseTopic => 8,
            Self::CorrelationData => 9,
            Self::SubscriptionIdentifier => 11,
            Self::SessionExpiryInterval => 17,
            Self::AssignedClientIdentifier => 18,
            Self::ServerKeepAlive => 19,
            Self::AuthenticationMethod => 21,
            Self::AuthenticationData => 22,
            Self::RequestProblemInformation => 23,
            Self::WillDelayInterval => 24,
            Self::RequestResponseInformation => 25,
            Self::ResponseInformation => 26,
            Self::ServerReference => 28,
            Self::ReasonString => 31,
            Self::ReceiveMaximum => 33,
            Self::TopicAliasMaximum => 34,
            Self::TopicAlias => 35,
            Self::MaximumQos => 36,
            Self::RetainAvailable => 37,
            Self::UserProperty => 38,
            Self::MaximumPacketSize => 39,
            Self::WildcardSubscriptionAvailable => 40,
            Self::SubscriptionIdentifierAvailable => 41,
            Self::SharedSubscriptionAvailable => 42,
        };

        buf.put_u8(val);
        Ok(())
    }
}

impl PropertyType {
    pub fn from_u8(value: u8) -> Result<Self, String> {
        match value {
            1 => Ok(Self::PayloadFormatIndicator),
            2 => Ok(Self::MessageExpiryInterval),
            3 => Ok(Self::ContentType),
            8 => Ok(Self::ResponseTopic),
            9 => Ok(Self::CorrelationData),
            11 => Ok(Self::SubscriptionIdentifier),
            17 => Ok(Self::SessionExpiryInterval),
            18 => Ok(Self::AssignedClientIdentifier),
            19 => Ok(Self::ServerKeepAlive),
            21 => Ok(Self::AuthenticationMethod),
            22 => Ok(Self::AuthenticationData),
            23 => Ok(Self::RequestProblemInformation),
            24 => Ok(Self::WillDelayInterval),
            25 => Ok(Self::RequestResponseInformation),
            26 => Ok(Self::ResponseInformation),
            28 => Ok(Self::ServerReference),
            31 => Ok(Self::ReasonString),
            33 => Ok(Self::ReceiveMaximum),
            34 => Ok(Self::TopicAliasMaximum),
            35 => Ok(Self::TopicAlias),
            36 => Ok(Self::MaximumQos),
            37 => Ok(Self::RetainAvailable),
            38 => Ok(Self::UserProperty),
            39 => Ok(Self::MaximumPacketSize),
            40 => Ok(Self::WildcardSubscriptionAvailable),
            41 => Ok(Self::SubscriptionIdentifierAvailable),
            42 => Ok(Self::SharedSubscriptionAvailable),
            _ => Err("Unkown property type".to_string()),
        }
    }
    pub fn to_u8(self) -> u8 {
        match self {
            Self::PayloadFormatIndicator => 1,
            Self::MessageExpiryInterval => 2,
            Self::ContentType => 3,
            Self::ResponseTopic => 8,
            Self::CorrelationData => 9,
            Self::SubscriptionIdentifier => 11,
            Self::SessionExpiryInterval => 17,
            Self::AssignedClientIdentifier => 18,
            Self::ServerKeepAlive => 19,
            Self::AuthenticationMethod => 21,
            Self::AuthenticationData => 22,
            Self::RequestProblemInformation => 23,
            Self::WillDelayInterval => 24,
            Self::RequestResponseInformation => 25,
            Self::ResponseInformation => 26,
            Self::ServerReference => 28,
            Self::ReasonString => 31,
            Self::ReceiveMaximum => 33,
            Self::TopicAliasMaximum => 34,
            Self::TopicAlias => 35,
            Self::MaximumQos => 36,
            Self::RetainAvailable => 37,
            Self::UserProperty => 38,
            Self::MaximumPacketSize => 39,
            Self::WildcardSubscriptionAvailable => 40,
            Self::SubscriptionIdentifierAvailable => 41,
            Self::SharedSubscriptionAvailable => 42,
        }
    }
}
