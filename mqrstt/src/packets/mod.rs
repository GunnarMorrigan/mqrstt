pub mod error;
pub mod mqtt_traits;
pub mod reason_codes;

mod macros;

mod auth;
mod connack;
mod connect;
mod disconnect;
mod puback;
mod pubcomp;
mod publish;
mod pubrec;
mod pubrel;
mod suback;
mod subscribe;
mod unsuback;
mod unsubscribe;

pub use auth::*;
pub use connack::*;
pub use connect::*;
pub use disconnect::*;
use error::ReadError;
use mqtt_traits::MqttAsyncRead;
pub use puback::*;
pub use pubcomp::*;
pub use publish::*;
pub use pubrec::*;
pub use pubrel::*;
pub use suback::*;
pub use subscribe::*;
pub use unsuback::*;
pub use unsubscribe::*;

use bytes::{Buf, BufMut, Bytes, BytesMut};
use core::slice::Iter;
use std::fmt::Display;

use self::error::{DeserializeError, ReadBytes, SerializeError};
use self::mqtt_traits::{MqttRead, MqttWrite, PacketRead, PacketWrite, WireLength};

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

impl<S> MqttAsyncRead<S> for ProtocolVersion where S: tokio::io::AsyncReadExt + std::marker::Unpin {
    async fn async_read(stream: &mut S) -> Result<(Self, usize), ReadError> {
        match stream.read_u8().await {
            Ok(5) => Ok((ProtocolVersion::V5, 1)),
            Ok(4) => Err(ReadError::DeserializeError(DeserializeError::UnsupportedProtocolVersion)),
            Ok(3) => Err(ReadError::DeserializeError(DeserializeError::UnsupportedProtocolVersion)),
            Ok(_) => Err(ReadError::DeserializeError(DeserializeError::UnknownProtocolVersion)),
            Err(e) => Err(ReadError::IoError(e)),
        }
    }
}

/// Quality of service
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum QoS {
    #[default]
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
    #[inline]
    fn read(buf: &mut Bytes) -> Result<Self, DeserializeError> {
        if buf.is_empty() {
            return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), 0, 1));
        }

        match buf.get_u8() {
            0 => Ok(QoS::AtMostOnce),
            1 => Ok(QoS::AtLeastOnce),
            2 => Ok(QoS::ExactlyOnce),
            q => Err(DeserializeError::UnknownQoS(q)),
        }
    }
}

impl<T> MqttAsyncRead<T> for QoS where T: tokio::io::AsyncReadExt + std::marker::Unpin {
    async fn async_read(buf: &mut T) -> Result<(Self, usize), ReadError> {
        match buf.read_u8().await {
            Ok(0) => Ok((QoS::AtMostOnce, 1)),
            Ok(1) => Ok((QoS::AtLeastOnce, 1)),
            Ok(2) => Ok((QoS::ExactlyOnce, 1)),
            Ok(q) => Err(ReadError::DeserializeError(DeserializeError::UnknownQoS(q))),
            Err(e) => Err(ReadError::IoError(e)),
        }
    }
}

impl MqttWrite for QoS {
    #[inline]
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

impl MqttRead for Box<str> {
    #[inline]
    fn read(buf: &mut Bytes) -> Result<Self, DeserializeError> {
        let content = Bytes::read(buf)?;

        match String::from_utf8(content.to_vec()) {
            Ok(s) => Ok(s.into()),
            Err(e) => Err(DeserializeError::Utf8Error(e)),
        }
    }
}

impl<S> MqttAsyncRead<S> for Box<str> where S: tokio::io::AsyncReadExt + std::marker::Unpin {
    async fn async_read(stream: &mut S) -> Result<(Self, usize), ReadError> {
        let (content, read_bytes) = Vec::async_read(stream).await?;
        match String::from_utf8(content) {
            Ok(s) => Ok((s.into(), read_bytes)),
            Err(e) => Err(ReadError::DeserializeError(DeserializeError::Utf8Error(e))),
        }
    }
}

impl MqttWrite for Box<str> {
    #[inline(always)]
    fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError> {
        self.as_ref().write(buf)
    }
}

impl WireLength for Box<str> {
    #[inline(always)]
    fn wire_len(&self) -> usize {
        self.as_ref().wire_len()
    }
}

impl MqttWrite for &str {
    #[inline]
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
    #[inline]
    fn read(buf: &mut Bytes) -> Result<Self, DeserializeError> {
        let content = Bytes::read(buf)?;

        match String::from_utf8(content.to_vec()) {
            Ok(s) => Ok(s),
            Err(e) => Err(DeserializeError::Utf8Error(e)),
        }
    }
}

impl<T> MqttAsyncRead<T> for String where T: tokio::io::AsyncReadExt + std::marker::Unpin {
    async fn async_read(buf: &mut T) -> Result<(Self, usize), ReadError> {
        let (content, read_bytes) = Bytes::async_read(buf).await?;
        match String::from_utf8(content.to_vec()) {
            Ok(s) => Ok((s, read_bytes)),
            Err(e) => Err(ReadError::DeserializeError(DeserializeError::Utf8Error(e))),
        }
    }
}


impl MqttWrite for String {
    #[inline]
    fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError> {
        if self.len() > 65535 {
            return Err(SerializeError::StringTooLong(self.len()));
        }

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
    #[inline]
    fn read(buf: &mut Bytes) -> Result<Self, DeserializeError> {
        let len = buf.get_u16() as usize;

        if len > buf.len() {
            return Err(DeserializeError::InsufficientData(std::any::type_name::<Bytes>(), buf.len(), len));
        }

        Ok(buf.split_to(len))
    }
}

impl<S> MqttAsyncRead<S> for Bytes where S: tokio::io::AsyncReadExt + std::marker::Unpin {
    async fn async_read(stream: &mut S) -> Result<(Self, usize), ReadError> {
        let size = stream.read_u16().await? as usize;
        // let mut data = BytesMut::with_capacity(size);
        let mut data = Vec::with_capacity(size);
        let read_bytes = stream.read_exact(&mut data).await?;
        assert_eq!(size, read_bytes);
        Ok((data.into(), 2 + size))
    }
}

impl MqttWrite for Bytes {
    #[inline]
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

impl MqttRead for Vec<u8> {
    #[inline]
    fn read(buf: &mut Bytes) -> Result<Self, DeserializeError> {
        let len = buf.get_u16() as usize;

        if len > buf.len() {
            return Err(DeserializeError::InsufficientData(std::any::type_name::<Bytes>(), buf.len(), len));
        }

        Ok(buf.split_to(len).into())
    }
}
impl MqttWrite for  Vec<u8> {
    #[inline]
    fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError> {
        buf.put_u16(self.len() as u16);
        buf.extend(self);

        Ok(())
    }
}
impl WireLength for Vec<u8> {
    #[inline(always)]
    fn wire_len(&self) -> usize {
        self.len() + 2
    }
}



impl<S> MqttAsyncRead<S> for Vec<u8> where S: tokio::io::AsyncReadExt + std::marker::Unpin {
    async fn async_read(stream: &mut S) -> Result<(Self, usize), ReadError> {
        let size = stream.read_u16().await? as usize;
        // let mut data = BytesMut::with_capacity(size);
        let mut data = vec![0u8; size];
        let read_bytes = stream.read_exact(&mut data).await?;
        assert_eq!(size, read_bytes);
        Ok((data, 2 + size))
    }
}


impl MqttRead for bool {
    fn read(buf: &mut Bytes) -> Result<Self, error::DeserializeError> {
        if buf.is_empty() {
            return Err(DeserializeError::InsufficientData(std::any::type_name::<bool>(), 0, 1));
        }

        match buf.get_u8() {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(error::DeserializeError::MalformedPacket),
        }
    }
}

impl<T> MqttAsyncRead<T> for bool where T: tokio::io::AsyncReadExt + std::marker::Unpin {
    async fn async_read(buf: &mut T) -> Result<(Self, usize), ReadError> {
        match buf.read_u8().await? {
            0 => Ok((false, 1)),
            1 => Ok((true, 1)),
            _ => Err(ReadError::DeserializeError(DeserializeError::MalformedPacket)),
        }
    }
}

impl MqttWrite for bool {
    #[inline]
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
    #[inline]
    fn read(buf: &mut Bytes) -> Result<Self, DeserializeError> {
        if buf.is_empty() {
            return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), 0, 1));
        }
        Ok(buf.get_u8())
    }
}

impl<T> MqttAsyncRead<T> for u8 where T: tokio::io::AsyncReadExt + std::marker::Unpin {
    async fn async_read(buf: &mut T) -> Result<(Self, usize), ReadError> {
        Ok((buf.read_u8().await?, 1))
    }
}

impl MqttRead for u16 {
    #[inline]
    fn read(buf: &mut Bytes) -> Result<Self, DeserializeError> {
        if buf.len() < 2 {
            return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), buf.len(), 2));
        }
        Ok(buf.get_u16())
    }
}

impl<T> MqttAsyncRead<T> for u16 where T: tokio::io::AsyncReadExt + std::marker::Unpin {
    async fn async_read(buf: &mut T) -> Result<(Self, usize), ReadError> {
        Ok((buf.read_u16().await?, 2))
    }
}

impl MqttWrite for u16 {
    #[inline]
    fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError> {
        buf.put_u16(*self);
        Ok(())
    }
}

impl MqttRead for u32 {
    #[inline]
    fn read(buf: &mut Bytes) -> Result<Self, DeserializeError> {
        if buf.len() < 4 {
            return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), buf.len(), 4));
        }
        Ok(buf.get_u32())
    }
}
impl<T> MqttAsyncRead<T> for u32 where T: tokio::io::AsyncReadExt + std::marker::Unpin {
    async fn async_read(buf: &mut T) -> Result<(Self, usize), ReadError> {
        Ok((buf.read_u32().await?, 4))
    }
}

impl MqttWrite for u32 {
    fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError> {
        buf.put_u32(*self);
        Ok(())
    }
}

pub fn read_fixed_header_rem_len(mut buf: Iter<u8>) -> Result<(usize, usize), ReadBytes<DeserializeError>> {
    let mut integer = 0;
    let mut length = 0;

    for i in 0..4 {
        if let Some(byte) = buf.next() {
            length += 1;
            integer += (*byte as usize & 0x7f) << (7 * i);

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

        integer += (byte as usize & 0x7f) << (7 * i);

        if (byte & 0b1000_0000) == 0 {
            return Ok((integer, length));
        }
    }
    Err(DeserializeError::MalformedPacket)
}

pub async fn read_async_variable_integer<S: tokio::io::AsyncReadExt + Unpin>(stream: &mut S) -> Result<(usize, usize), ReadError> {
    let mut integer = 0;
    let mut length = 0;

    for i in 0..4 {
        let byte = stream.read_u8().await?;
        length += 1;

        integer += (byte as usize & 0x7f) << (7 * i);

        if (byte & 0b1000_0000) == 0 {
            return Ok((integer, length));
        }
    }
    Err(ReadError::DeserializeError(DeserializeError::MalformedPacket))
}

pub fn write_variable_integer(buf: &mut BytesMut, integer: usize) -> Result<usize, SerializeError> {
    if integer > 268_435_455 {
        return Err(SerializeError::VariableIntegerOverflow(integer));
    }

    let mut write = integer;

    for i in 0..4 {
        let mut byte = (write % 128) as u8;
        write /= 128;
        if write > 0 {
            byte |= 128;
        }
        buf.put_u8(byte);
        if write == 0 {
            return Ok(i+1);
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

impl TryFrom<u8> for PropertyType {
    type Error = DeserializeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
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
            t => Err(DeserializeError::UnknownProperty(t)),
        }
    }
}

impl From<&PropertyType> for u8 {
    fn from(value: &PropertyType) -> Self {
        match value {
            PropertyType::PayloadFormatIndicator => 1,
            PropertyType::MessageExpiryInterval => 2,
            PropertyType::ContentType => 3,
            PropertyType::ResponseTopic => 8,
            PropertyType::CorrelationData => 9,
            PropertyType::SubscriptionIdentifier => 11,
            PropertyType::SessionExpiryInterval => 17,
            PropertyType::AssignedClientIdentifier => 18,
            PropertyType::ServerKeepAlive => 19,
            PropertyType::AuthenticationMethod => 21,
            PropertyType::AuthenticationData => 22,
            PropertyType::RequestProblemInformation => 23,
            PropertyType::WillDelayInterval => 24,
            PropertyType::RequestResponseInformation => 25,
            PropertyType::ResponseInformation => 26,
            PropertyType::ServerReference => 28,
            PropertyType::ReasonString => 31,
            PropertyType::ReceiveMaximum => 33,
            PropertyType::TopicAliasMaximum => 34,
            PropertyType::TopicAlias => 35,
            PropertyType::MaximumQos => 36,
            PropertyType::RetainAvailable => 37,
            PropertyType::UserProperty => 38,
            PropertyType::MaximumPacketSize => 39,
            PropertyType::WildcardSubscriptionAvailable => 40,
            PropertyType::SubscriptionIdentifierAvailable => 41,
            PropertyType::SharedSubscriptionAvailable => 42,
        }
    }
}

impl From<PropertyType> for u8 {
    fn from(value: PropertyType) -> Self {
        value as u8
    }
}

impl MqttRead for PropertyType {
    fn read(buf: &mut Bytes) -> Result<Self, DeserializeError> {
        if buf.is_empty() {
            return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), 0, 1));
        }

        buf.get_u8().try_into()
    }
}

impl<T> MqttAsyncRead<T> for PropertyType where T: tokio::io::AsyncReadExt + std::marker::Unpin {
    async fn async_read(buf: &mut T) -> Result<(Self, usize), ReadError> {
        match buf.read_u8().await {
            Ok(t) => Ok((t.try_into()?, 1)),
            Err(e) => Err(ReadError::IoError(e)),
        }
    }
}

impl MqttWrite for PropertyType {
    fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError> {
        buf.put_u8(self.into());
        Ok(())
    }
}

// ==================== Packets ====================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Packet {
    Connect(Connect),
    ConnAck(ConnAck),
    Publish(Publish),
    PubAck(PubAck),
    PubRec(PubRec),
    PubRel(PubRel),
    PubComp(PubComp),
    Subscribe(Subscribe),
    SubAck(SubAck),
    Unsubscribe(Unsubscribe),
    UnsubAck(UnsubAck),
    PingReq,
    PingResp,
    Disconnect(Disconnect),
    Auth(Auth),
}

impl Packet {
    pub fn packet_type(&self) -> PacketType {
        match self {
            Packet::Connect(_) => PacketType::Connect,
            Packet::ConnAck(_) => PacketType::ConnAck,
            Packet::Publish(_) => PacketType::Publish,
            Packet::PubAck(_) => PacketType::PubAck,
            Packet::PubRec(_) => PacketType::PubRec,
            Packet::PubRel(_) => PacketType::PubRel,
            Packet::PubComp(_) => PacketType::PubComp,
            Packet::Subscribe(_) => PacketType::Subscribe,
            Packet::SubAck(_) => PacketType::SubAck,
            Packet::Unsubscribe(_) => PacketType::Unsubscribe,
            Packet::UnsubAck(_) => PacketType::UnsubAck,
            Packet::PingReq => PacketType::PingReq,
            Packet::PingResp => PacketType::PingResp,
            Packet::Disconnect(_) => PacketType::Disconnect,
            Packet::Auth(_) => PacketType::Auth,
        }
    }

    pub fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError> {
        match self {
            Packet::Connect(p) => {
                buf.put_u8(0b0001_0000);
                write_variable_integer(buf, p.wire_len())?;

                p.write(buf)?;
            }
            Packet::ConnAck(p) => {
                buf.put_u8(0b0010_0000);
                write_variable_integer(buf, p.wire_len())?;
                p.write(buf)?;
            }
            Packet::Publish(p) => {
                let mut first_byte = 0b0011_0000u8;
                if p.dup {
                    first_byte |= 0b1000;
                }

                first_byte |= p.qos.into_u8() << 1;

                if p.retain {
                    first_byte |= 0b0001;
                }
                buf.put_u8(first_byte);
                write_variable_integer(buf, p.wire_len())?;
                p.write(buf)?;
            }
            Packet::PubAck(p) => {
                buf.put_u8(0b0100_0000);
                write_variable_integer(buf, p.wire_len())?;
                p.write(buf)?;
            }
            Packet::PubRec(p) => {
                buf.put_u8(0b0101_0000);
                write_variable_integer(buf, p.wire_len())?;
                p.write(buf)?;
            }
            Packet::PubRel(p) => {
                buf.put_u8(0b0110_0010);
                write_variable_integer(buf, p.wire_len())?;
                p.write(buf)?;
            }
            Packet::PubComp(p) => {
                buf.put_u8(0b0111_0000);
                write_variable_integer(buf, p.wire_len())?;
                p.write(buf)?;
            }
            Packet::Subscribe(p) => {
                buf.put_u8(0b1000_0010);
                write_variable_integer(buf, p.wire_len())?;
                p.write(buf)?;
            }
            Packet::SubAck(_) => {
                unreachable!()
            }
            Packet::Unsubscribe(p) => {
                buf.put_u8(0b1010_0010);
                write_variable_integer(buf, p.wire_len())?;
                p.write(buf)?;
            }
            Packet::UnsubAck(_) => {
                unreachable!();
                buf.put_u8(0b1011_0000);
            }
            Packet::PingReq => {
                buf.put_u8(0b1100_0000);
                buf.put_u8(0); // Variable header length.
            }
            Packet::PingResp => {
                buf.put_u8(0b1101_0000);
                buf.put_u8(0); // Variable header length.
            }
            Packet::Disconnect(p) => {
                buf.put_u8(0b1110_0000);
                write_variable_integer(buf, p.wire_len())?;
                p.write(buf)?;
            }
            Packet::Auth(p) => {
                buf.put_u8(0b1111_0000);
                write_variable_integer(buf, p.wire_len())?;
                p.write(buf)?;
            }
        }
        Ok(())
    }

    pub fn read(header: FixedHeader, buf: Bytes) -> Result<Packet, DeserializeError> {
        let packet = match header.packet_type {
            PacketType::Connect => Packet::Connect(Connect::read(header.flags, header.remaining_length, buf)?),
            PacketType::ConnAck => Packet::ConnAck(ConnAck::read(header.flags, header.remaining_length, buf)?),
            PacketType::Publish => Packet::Publish(Publish::read(header.flags, header.remaining_length, buf)?),
            PacketType::PubAck => Packet::PubAck(PubAck::read(header.flags, header.remaining_length, buf)?),
            PacketType::PubRec => Packet::PubRec(PubRec::read(header.flags, header.remaining_length, buf)?),
            PacketType::PubRel => Packet::PubRel(PubRel::read(header.flags, header.remaining_length, buf)?),
            PacketType::PubComp => Packet::PubComp(PubComp::read(header.flags, header.remaining_length, buf)?),
            PacketType::Subscribe => Packet::Subscribe(Subscribe::read(header.flags, header.remaining_length, buf)?),
            PacketType::SubAck => Packet::SubAck(SubAck::read(header.flags, header.remaining_length, buf)?),
            PacketType::Unsubscribe => Packet::Unsubscribe(Unsubscribe::read(header.flags, header.remaining_length, buf)?),
            PacketType::UnsubAck => Packet::UnsubAck(UnsubAck::read(header.flags, header.remaining_length, buf)?),
            PacketType::PingReq => Packet::PingReq,
            PacketType::PingResp => Packet::PingResp,
            PacketType::Disconnect => Packet::Disconnect(Disconnect::read(header.flags, header.remaining_length, buf)?),
            PacketType::Auth => Packet::Auth(Auth::read(header.flags, header.remaining_length, buf)?),
        };
        Ok(packet)
    }

    pub fn read_from_buffer(buffer: &mut BytesMut) -> Result<Packet, ReadBytes<DeserializeError>> {
        let (header, header_length) = FixedHeader::read_fixed_header(buffer.iter())?;
        if header.remaining_length + header_length > buffer.len() {
            return Err(ReadBytes::InsufficientBytes(header.remaining_length + header_length - buffer.len()));
        }
        buffer.advance(header_length);

        let buf = buffer.split_to(header.remaining_length);

        Ok(Packet::read(header, buf.into())?)
    }
}

impl Display for Packet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Packet::Connect(c) => write!(
                f,
                "Connect(version: {:?}, clean: {}, username: {:?}, password: {:?}, keep_alive: {}, client_id: {})",
                c.protocol_version, c.clean_start, c.username, c.password, c.keep_alive, c.client_id
            ),
            Packet::ConnAck(c) => write!(f, "ConnAck(session:{:?}, reason code{:?})", c.connack_flags, c.reason_code),
            Packet::Publish(p) => write!(
                f,
                "Publish(topic: {}, qos: {:?}, dup: {:?}, retain: {:?}, packet id: {:?})",
                &p.topic, p.qos, p.dup, p.retain, p.packet_identifier
            ),
            Packet::PubAck(p) => write!(f, "PubAck(id:{:?}, reason code: {:?})", p.packet_identifier, p.reason_code),
            Packet::PubRec(p) => write!(f, "PubRec(id: {}, reason code: {:?})", p.packet_identifier, p.reason_code),
            Packet::PubRel(p) => write!(f, "PubRel(id: {}, reason code: {:?})", p.packet_identifier, p.reason_code),
            Packet::PubComp(p) => write!(f, "PubComp(id: {}, reason code: {:?})", p.packet_identifier, p.reason_code),
            Packet::Subscribe(_) => write!(f, "Subscribe()"),
            Packet::SubAck(_) => write!(f, "SubAck()"),
            Packet::Unsubscribe(_) => write!(f, "Unsubscribe()"),
            Packet::UnsubAck(_) => write!(f, "UnsubAck()"),
            Packet::PingReq => write!(f, "PingReq"),
            Packet::PingResp => write!(f, "PingResp"),
            Packet::Disconnect(d) => write!(f, "Disconnect(reason code: {:?})", d.reason_code),
            Packet::Auth(_) => write!(f, "Auth()"),
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub struct FixedHeader {
    pub packet_type: PacketType,
    pub flags: u8,
    pub remaining_length: usize,
}

impl FixedHeader {
    pub fn read_fixed_header(mut header: Iter<u8>) -> Result<(Self, usize), ReadBytes<DeserializeError>> {
        if header.len() < 2 {
            return Err(ReadBytes::InsufficientBytes(2 - header.len()));
        }

        let mut header_length = 1;
        let first_byte = header.next().unwrap();

        let (packet_type, flags) = PacketType::from_first_byte(*first_byte).map_err(ReadBytes::Err)?;

        let (remaining_length, length) = read_fixed_header_rem_len(header)?;
        header_length += length;

        Ok((Self { packet_type, flags, remaining_length }, header_length))
    }
}

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
impl PacketType {
    const fn from_first_byte(value: u8) -> Result<(Self, u8), DeserializeError> {
        match (value >> 4, value & 0x0f) {
            (0b0001, 0) => Ok((PacketType::Connect, 0)),
            (0b0010, 0) => Ok((PacketType::ConnAck, 0)),
            (0b0011, flags) => Ok((PacketType::Publish, flags)),
            (0b0100, 0) => Ok((PacketType::PubAck, 0)),
            (0b0101, 0) => Ok((PacketType::PubRec, 0)),
            (0b0110, 0b0010) => Ok((PacketType::PubRel, 0)),
            (0b0111, 0) => Ok((PacketType::PubComp, 0)),
            (0b1000, 0b0010) => Ok((PacketType::Subscribe, 0)),
            (0b1001, 0) => Ok((PacketType::SubAck, 0)),
            (0b1010, 0b0010) => Ok((PacketType::Unsubscribe, 0)),
            (0b1011, 0) => Ok((PacketType::UnsubAck, 0)),
            (0b1100, 0) => Ok((PacketType::PingReq, 0)),
            (0b1101, 0) => Ok((PacketType::PingResp, 0)),
            (0b1110, 0) => Ok((PacketType::Disconnect, 0)),
            (0b1111, 0) => Ok((PacketType::Auth, 0)),
            (_, _) => Err(DeserializeError::UnknownFixedHeader(value)),
        }
    }
}

impl std::fmt::Display for PacketType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self, f)
    }
}

#[cfg(test)]
mod tests {
    use bytes::{Bytes, BytesMut};

    use crate::packets::connack::{ConnAck, ConnAckFlags, ConnAckProperties};
    use crate::packets::disconnect::{Disconnect, DisconnectProperties};
    use crate::packets::QoS;

    use crate::packets::publish::{Publish, PublishProperties};
    use crate::packets::pubrel::{PubRel, PubRelProperties};
    use crate::packets::reason_codes::{ConnAckReasonCode, DisconnectReasonCode, PubRelReasonCode};
    use crate::packets::Packet;

    #[test]
    fn test_connack_read() {
        let connack = [
            0x20, 0x13, 0x01, 0x00, 0x10, 0x27, 0x00, 0x10, 0x00, 0x00, 0x25, 0x01, 0x2a, 0x01, 0x29, 0x01, 0x22, 0xff, 0xff, 0x28, 0x01,
        ];
        let mut buf = BytesMut::new();
        buf.extend(connack);

        let res = Packet::read_from_buffer(&mut buf);
        assert!(res.is_ok());
        let res = res.unwrap();

        let expected = ConnAck {
            connack_flags: ConnAckFlags { session_present: true },
            reason_code: ConnAckReasonCode::Success,
            connack_properties: ConnAckProperties {
                session_expiry_interval: None,
                receive_maximum: None,
                maximum_qos: None,
                retain_available: Some(true),
                maximum_packet_size: Some(1048576),
                assigned_client_id: None,
                topic_alias_maximum: Some(65535),
                reason_string: None,
                user_properties: vec![],
                wildcards_available: Some(true),
                subscription_ids_available: Some(true),
                shared_subscription_available: Some(true),
                server_keep_alive: None,
                response_info: None,
                server_reference: None,
                authentication_method: None,
                authentication_data: None,
            },
        };

        assert_eq!(Packet::ConnAck(expected), res);
    }

    #[test]
    fn test_disconnect_read() {
        let packet = [0xe0, 0x02, 0x8e, 0x00];
        let mut buf = BytesMut::new();
        buf.extend(packet);

        let res = Packet::read_from_buffer(&mut buf);
        assert!(res.is_ok());
        let res = res.unwrap();

        let expected = Disconnect {
            reason_code: DisconnectReasonCode::SessionTakenOver,
            properties: DisconnectProperties {
                session_expiry_interval: None,
                reason_string: None,
                user_properties: vec![],
                server_reference: None,
            },
        };

        assert_eq!(Packet::Disconnect(expected), res);
    }

    #[test]
    fn test_pingreq_read_write() {
        let packet = [0xc0, 0x00];
        let mut buf = BytesMut::new();
        buf.extend(packet);

        let res = Packet::read_from_buffer(&mut buf);
        assert!(res.is_ok());
        let res = res.unwrap();

        assert_eq!(Packet::PingReq, res);

        buf.clear();
        Packet::PingReq.write(&mut buf).unwrap();
        assert_eq!(buf.to_vec(), packet);
    }

    #[test]
    fn test_pingresp_read_write() {
        let packet = [0xd0, 0x00];
        let mut buf = BytesMut::new();
        buf.extend(packet);

        let res = Packet::read_from_buffer(&mut buf);
        assert!(res.is_ok());
        let res = res.unwrap();

        assert_eq!(Packet::PingResp, res);

        buf.clear();
        Packet::PingResp.write(&mut buf).unwrap();
        assert_eq!(buf.to_vec(), packet);
    }

    #[test]
    fn test_publish_read() {
        let packet = [
            0x35, 0x24, 0x00, 0x14, 0x74, 0x65, 0x73, 0x74, 0x2f, 0x31, 0x32, 0x33, 0x2f, 0x74, 0x65, 0x73, 0x74, 0x2f, 0x62, 0x6c, 0x61, 0x62, 0x6c, 0x61, 0x35, 0xd3, 0x0b, 0x01, 0x01, 0x09, 0x00,
            0x04, 0x31, 0x32, 0x31, 0x32, 0x0b, 0x01,
        ];

        let mut buf = BytesMut::new();
        buf.extend(packet);

        let res = Packet::read_from_buffer(&mut buf);
        assert!(res.is_ok());
        let res = res.unwrap();

        let expected = Publish {
            dup: false,
            qos: QoS::ExactlyOnce,
            retain: true,
            topic: "test/123/test/blabla".into(),
            packet_identifier: Some(13779),
            publish_properties: PublishProperties {
                payload_format_indicator: Some(1),
                message_expiry_interval: None,
                topic_alias: None,
                response_topic: None,
                correlation_data: Some(b"1212".to_vec()),
                subscription_identifier: vec![1],
                user_properties: vec![],
                content_type: None,
            },
            payload: Bytes::from_static(b""),
        };

        assert_eq!(Packet::Publish(expected), res);
    }

    #[test]
    fn test_pubrel_read_write() {
        let bytes = [0x62, 0x03, 0x35, 0xd3, 0x00];

        let mut buffer = BytesMut::from_iter(bytes);

        let res = Packet::read_from_buffer(&mut buffer);

        assert!(res.is_ok());

        let packet = res.unwrap();

        let expected = PubRel {
            packet_identifier: 13779,
            reason_code: PubRelReasonCode::Success,
            properties: PubRelProperties {
                reason_string: None,
                user_properties: vec![],
            },
        };

        assert_eq!(packet, Packet::PubRel(expected));

        buffer.clear();

        packet.write(&mut buffer).unwrap();

        // The input is not in the smallest possible format but when writing we do expect it to be in the smallest possible format.
        assert_eq!(buffer.to_vec(), [0x62, 0x02, 0x35, 0xd3].to_vec())
    }

    #[test]
    fn test_pubrel_read_smallest_format() {
        let bytes = [0x62, 0x02, 0x35, 0xd3];

        let mut buffer = BytesMut::from_iter(bytes);

        let res = Packet::read_from_buffer(&mut buffer);

        assert!(res.is_ok());

        let packet = res.unwrap();

        let expected = PubRel {
            packet_identifier: 13779,
            reason_code: PubRelReasonCode::Success,
            properties: PubRelProperties {
                reason_string: None,
                user_properties: vec![],
            },
        };

        assert_eq!(packet, Packet::PubRel(expected));

        buffer.clear();

        packet.write(&mut buffer).unwrap();

        assert_eq!(buffer.to_vec(), bytes.to_vec())
    }
}
