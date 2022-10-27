mod connect;
mod publish;
mod errors;
pub mod mqtt_traits;

pub mod connack;
pub mod reason_codes;

use bytes::{BufMut, Bytes, BytesMut, Buf};

use self::connect::ConnectFlags;
use self::mqtt_traits::{SimpleSerialize, WireLength, MqttRead, MqttWrite};
use self::{connect::Connect};
use self::publish::{Publish};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Packet {
    Connect(Connect),
    ConnAck(),
    Publish(Publish),
    PubAck(),
    PubRec(),
    PubRel(),
    PubComp(),
    Subscribe(),
    SubAck(),
    Unsubscribe(),
    UnsubAck(),
    PingReq(),
    PingResp(),
    Disconnect(),
    Auth(),
}

/// Protocol version
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum ProtocolVersion {
    V5,
}

impl SimpleSerialize for ProtocolVersion {

    fn read(buf: &mut Bytes) -> Result<Self, String> {
        match buf.get_u8() {
            3 => Err("Unsupported protocol version".to_string()),
            4 => Err("Unsupported protocol version".to_string()),
            5 => Ok(ProtocolVersion::V5),
            _ => Err("Unknown protocol version".to_string()),
        }
    }

    fn write(&self, buf: &mut BytesMut) {
        buf.put_u8(5u8)
    }

    // fn size(&self) -> usize{
    //     1
    // }
}


/// Quality of service
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum QoS {
    AtMostOnce = 0,
    AtLeastOnce = 1,
    ExactlyOnce = 2,
}
impl SimpleSerialize for QoS{
    fn read(buf: &mut Bytes) -> Result<Self, String> {
        match buf.get_u8() {
            0 => Ok(QoS::AtMostOnce),
            1 => Ok(QoS::AtLeastOnce),
            2 => Ok(QoS::ExactlyOnce),
            _ => Err("Error serializing. This should be replaced with proper error type".to_string())
        }
    }

    fn write(&self, buf: &mut BytesMut) {
        let val = match self {
            QoS::AtMostOnce => 0,
            QoS::AtLeastOnce => 1,
            QoS::ExactlyOnce => 2,
        };
        buf.put_u8(val);
    }
}


impl TryFrom<ConnectFlags> for QoS{
    type Error = String;

    fn try_from(c: ConnectFlags) -> Result<Self, Self::Error> {
        if c.contains(ConnectFlags::WILL_QOS1 | ConnectFlags::WILL_QOS2){
            Err("QoS2 and QoS1 can't be active at the same time".to_string())
        }
        else if c.contains(ConnectFlags::WILL_QOS2){
            Ok(QoS::ExactlyOnce)
        }
        else if c.contains(ConnectFlags::WILL_QOS1){
            Ok(QoS::AtLeastOnce)
        }
        else {
            Ok(QoS::AtMostOnce)
        }
    }
}

// 2.1.1 Fixed Header
// ```
//          7                          3                          0
//          +--------------------------+--------------------------+
// byte 1   | MQTT Control Packet Type | Flags for Packet type    |
//          +--------------------------+--------------------------+
//          |                   Remaining Length                  |
//          +-----------------------------------------------------+
//
// https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901021
// ```



/// 2.1.2 MQTT Control Packet type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum PacketType {
    Connect,
    ConnAck,
    Publish,
    PubAck,
    PubRec,
    PubRel,
    PubComp,
    Subscribe,
    SubAck,
    Unsubscribe,
    UnsubAck,
    PingReq,
    PingResp,
    Disconnect,
    Auth,
}

impl SimpleSerialize for &str{

    fn read(_: &mut Bytes) -> Result<Self, String> {
        Err("Can't read a &str".to_string())
    }

    fn write(&self, buf: &mut BytesMut) {
        buf.put_u16(self.len() as u16);
        buf.extend(self.as_bytes());
    }

}

impl SimpleSerialize for String{

    fn read(buf: &mut Bytes) -> Result<Self, String> {
        let content = Bytes::read(buf)?;

        match String::from_utf8(content.to_vec()){
            Ok(s) => Ok(s),
            Err(_) => Err("AAA not good string".to_string()),
        }
    }

    fn write(&self, buf: &mut BytesMut) {
        buf.put_u16(self.len() as u16);
        buf.extend(self.as_bytes());
    }

}

impl WireLength for String {
    #[inline(always)]
    fn wire_len(&self) -> usize{
        self.len() + 2
    }
}

impl SimpleSerialize for Bytes {
    fn read(buf: &mut Bytes) -> Result<Self, String> {
        let len = buf.get_u16() as usize;
        
        if len > buf.len() {
            return Err(format!("Not enough data to take {} bytes", len));
        }
    
        Ok(buf.split_to(len))
    }

    fn write(&self, buf: &mut BytesMut) {
        buf.put_u16(self.len() as u16);
        buf.extend(self);
    }
}

impl WireLength for Bytes {
    #[inline(always)]
    fn wire_len(&self) -> usize{
        self.len() + 2
    }
}

impl MqttRead for bool{
    fn read(buf: &mut Bytes) -> Result<Self, errors::PacketError> {
        match buf.get_u8(){
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(errors::PacketError::MalformedPacket)
        }
    }
}

impl MqttWrite for bool{
    fn write(&self, buf: &mut BytesMut) {
        if *self {
            buf.put_u8(1);
        }
        else{
            buf.put_u8(0);
        }
    }
}

pub fn read_variable_integer(buf: &mut Bytes) -> Result<(usize, usize), String>{
    let mut integer = 0;
    let mut length = 0;

    for i in 0..4{
        length += 1;
        let byte = buf.get_u8();

        integer += (byte as usize & 0x7f) << 7*i;

        if (byte & 0b1000_0000) == 0{
            return Ok((integer, length));
        }
    }
    Err("Variable integer malformed".to_string())
}

pub fn write_variable_integer(buf: &mut BytesMut, integer: usize) -> Result<(), String>{
    if integer > 268_435_455 {
        return Err("Too long!".to_string());
    }

    let mut write = integer;

    for _ in 0..4{
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
    Err("Could not write variable integer in 4 bytes".to_string())
}

pub fn variable_integer_len(integer: usize) -> usize{
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
enum PropertyType {
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

impl PropertyType{
    pub fn from_u8(value: u8) -> Result<Self, String>{
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
            _ => Err("Unkown property type".to_string())
        }
    }
    pub fn to_u8(self) -> u8{
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