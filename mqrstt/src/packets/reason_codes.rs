use std::default;

use bytes::{Buf, BufMut};

use super::error::DeserializeError;
use super::mqtt_trait::{MqttAsyncRead, MqttRead, MqttWrite};


// #[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
// pub enum ConnAckReasonCode {
//     #[default]
//     Success,

//     UnspecifiedError,
//     MalformedPacket,
//     ProtocolError,
//     ImplementationSpecificError,
//     UnsupportedProtocolVersion,
//     ClientIdentifierNotValid,
//     BadUsernameOrPassword,
//     NotAuthorized,
//     ServerUnavailable,
//     ServerBusy,
//     Banned,
//     BadAuthenticationMethod,
//     TopicNameInvalid,
//     PacketTooLarge,
//     QuotaExceeded,
//     PayloadFormatInvalid,
//     RetainNotSupported,
//     QosNotSupported,
//     UseAnotherServer,
//     ServerMoved,
//     ConnectionRateExceeded,
// }

// impl MqttRead for ConnAckReasonCode {
//     fn read(buf: &mut bytes::Bytes) -> Result<Self, DeserializeError> {
//         if buf.is_empty() {
//             return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), 0, 1));
//         }
//         let res = buf.get_u8();

//         crate::packets::macros::reason_code_match!(@ ConnAckReasonCode, res, {
//             Success,
//             UnspecifiedError,
//             MalformedPacket,
//             ProtocolError,
//             ImplementationSpecificError,
//             UnsupportedProtocolVersion,
//             ClientIdentifierNotValid,
//             BadUsernameOrPassword,
//             NotAuthorized,
//             ServerUnavailable,
//             ServerBusy,
//             Banned,
//             BadAuthenticationMethod,
//             TopicNameInvalid,
//             PacketTooLarge,
//             QuotaExceeded,
//             PayloadFormatInvalid,
//             RetainNotSupported,
//             QosNotSupported,
//             UseAnotherServer,
//             ServerMoved,
//             ConnectionRateExceeded,
//         } -> ())
//         // match buf.get_u8() {
//         //     0x00 => Ok(ConnAckReasonCode::Success),
//         //     0x80 => Ok(ConnAckReasonCode::UnspecifiedError),
//         //     0x81 => Ok(ConnAckReasonCode::MalformedPacket),
//         //     0x82 => Ok(ConnAckReasonCode::ProtocolError),
//         //     0x83 => Ok(ConnAckReasonCode::ImplementationSpecificError),
//         //     0x84 => Ok(ConnAckReasonCode::UnsupportedProtocolVersion),
//         //     0x85 => Ok(ConnAckReasonCode::ClientIdentifierNotValid),
//         //     0x86 => Ok(ConnAckReasonCode::BadUsernameOrPassword),
//         //     0x87 => Ok(ConnAckReasonCode::NotAuthorized),
//         //     0x88 => Ok(ConnAckReasonCode::ServerUnavailable),
//         //     0x89 => Ok(ConnAckReasonCode::ServerBusy),
//         //     0x8A => Ok(ConnAckReasonCode::Banned),
//         //     0x8C => Ok(ConnAckReasonCode::BadAuthenticationMethod),
//         //     0x90 => Ok(ConnAckReasonCode::TopicNameInvalid),
//         //     0x95 => Ok(ConnAckReasonCode::PacketTooLarge),
//         //     0x97 => Ok(ConnAckReasonCode::QuotaExceeded),
//         //     0x99 => Ok(ConnAckReasonCode::PayloadFormatInvalid),
//         //     0x9A => Ok(ConnAckReasonCode::RetainNotSupported),
//         //     0x9B => Ok(ConnAckReasonCode::QosNotSupported),
//         //     0x9C => Ok(ConnAckReasonCode::UseAnotherServer),
//         //     0x9D => Ok(ConnAckReasonCode::ServerMoved),
//         //     0x9F => Ok(ConnAckReasonCode::ConnectionRateExceeded),
//         //     t => Err(DeserializeError::UnknownProperty(t)),
//         // }
//     }
// }

// impl MqttWrite for ConnAckReasonCode {
//     fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
//         let val = match self {
//             ConnAckReasonCode::Success => 0x00,
//             ConnAckReasonCode::UnspecifiedError => 0x80,
//             ConnAckReasonCode::MalformedPacket => 0x81,
//             ConnAckReasonCode::ProtocolError => 0x82,
//             ConnAckReasonCode::ImplementationSpecificError => 0x83,
//             ConnAckReasonCode::UnsupportedProtocolVersion => 0x84,
//             ConnAckReasonCode::ClientIdentifierNotValid => 0x85,
//             ConnAckReasonCode::BadUsernameOrPassword => 0x86,
//             ConnAckReasonCode::NotAuthorized => 0x87,
//             ConnAckReasonCode::ServerUnavailable => 0x88,
//             ConnAckReasonCode::ServerBusy => 0x89,
//             ConnAckReasonCode::Banned => 0x8A,
//             ConnAckReasonCode::BadAuthenticationMethod => 0x8C,
//             ConnAckReasonCode::TopicNameInvalid => 0x90,
//             ConnAckReasonCode::PacketTooLarge => 0x95,
//             ConnAckReasonCode::QuotaExceeded => 0x97,
//             ConnAckReasonCode::PayloadFormatInvalid => 0x99,
//             ConnAckReasonCode::RetainNotSupported => 0x9A,
//             ConnAckReasonCode::QosNotSupported => 0x9B,
//             ConnAckReasonCode::UseAnotherServer => 0x9C,
//             ConnAckReasonCode::ServerMoved => 0x9D,
//             ConnAckReasonCode::ConnectionRateExceeded => 0x9F,
//         };

//         buf.put_u8(val);

//         Ok(())
//     }
// }

// #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
// pub enum AuthReasonCode {
//     Success,
//     ContinueAuthentication,
//     ReAuthenticate,
// }

// impl MqttRead for AuthReasonCode {
//     fn read(buf: &mut bytes::Bytes) -> Result<Self, DeserializeError> {
//         if buf.is_empty() {
//             return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), 0, 1));
//         }

//         match buf.get_u8() {
//             0x00 => Ok(AuthReasonCode::Success),
//             0x18 => Ok(AuthReasonCode::ContinueAuthentication),
//             0x19 => Ok(AuthReasonCode::ReAuthenticate),
//             t => Err(DeserializeError::UnknownProperty(t)),
//         }
//     }
// }

// impl MqttWrite for AuthReasonCode {
//     fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
//         let val = match self {
//             AuthReasonCode::Success => 0x00,
//             AuthReasonCode::ContinueAuthentication => 0x18,
//             AuthReasonCode::ReAuthenticate => 0x19,
//         };

//         buf.put_u8(val);

//         Ok(())
//     }
// }



// #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
// pub enum PubCompReasonCode {
//     Success,
//     PacketIdentifierNotFound,
// }

// impl MqttRead for PubCompReasonCode {
//     fn read(buf: &mut bytes::Bytes) -> Result<Self, DeserializeError> {
//         if buf.is_empty() {
//             return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), 0, 1));
//         }

//         match buf.get_u8() {
//             0x00 => Ok(PubCompReasonCode::Success),
//             0x92 => Ok(PubCompReasonCode::PacketIdentifierNotFound),
//             t => Err(DeserializeError::UnknownProperty(t)),
//         }
//     }
// }
// impl MqttWrite for PubCompReasonCode {
//     fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
//         let val = match self {
//             PubCompReasonCode::Success => 0x00,
//             PubCompReasonCode::PacketIdentifierNotFound => 0x92,
//         };

//         buf.put_u8(val);
//         Ok(())
//     }
// }

// #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
// pub enum SubAckReasonCode {
//     GrantedQoS0,
//     GrantedQoS1,
//     GrantedQoS2,
//     UnspecifiedError,
//     ImplementationSpecificError,
//     NotAuthorized,
//     TopicFilterInvalid,
//     PacketIdentifierInUse,
//     QuotaExceeded,
//     SharedSubscriptionsNotSupported,
//     SubscriptionIdentifiersNotSupported,
//     WildcardSubscriptionsNotSupported,
// }

// impl MqttRead for SubAckReasonCode {
//     fn read(buf: &mut bytes::Bytes) -> Result<Self, DeserializeError> {
//         if buf.is_empty() {
//             return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), 0, 1));
//         }

//         match buf.get_u8() {
//             0x00 => Ok(SubAckReasonCode::GrantedQoS0),
//             0x01 => Ok(SubAckReasonCode::GrantedQoS1),
//             0x02 => Ok(SubAckReasonCode::GrantedQoS2),
//             0x80 => Ok(SubAckReasonCode::UnspecifiedError),
//             0x83 => Ok(SubAckReasonCode::ImplementationSpecificError),
//             0x87 => Ok(SubAckReasonCode::NotAuthorized),
//             0x8F => Ok(SubAckReasonCode::TopicFilterInvalid),
//             0x91 => Ok(SubAckReasonCode::PacketIdentifierInUse),
//             0x97 => Ok(SubAckReasonCode::QuotaExceeded),
//             0x9E => Ok(SubAckReasonCode::SharedSubscriptionsNotSupported),
//             0xA1 => Ok(SubAckReasonCode::SubscriptionIdentifiersNotSupported),
//             0xA2 => Ok(SubAckReasonCode::WildcardSubscriptionsNotSupported),
//             t => Err(DeserializeError::UnknownProperty(t)),
//         }
//     }
// }

// impl MqttWrite for SubAckReasonCode {
//     fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
//         let val = match self {
//             SubAckReasonCode::GrantedQoS0 => 0x00,
//             SubAckReasonCode::GrantedQoS1 => 0x01,
//             SubAckReasonCode::GrantedQoS2 => 0x02,
//             SubAckReasonCode::UnspecifiedError => 0x80,
//             SubAckReasonCode::ImplementationSpecificError => 0x83,
//             SubAckReasonCode::NotAuthorized => 0x87,
//             SubAckReasonCode::TopicFilterInvalid => 0x8F,
//             SubAckReasonCode::PacketIdentifierInUse => 0x91,
//             SubAckReasonCode::QuotaExceeded => 0x97,
//             SubAckReasonCode::SharedSubscriptionsNotSupported => 0x9E,
//             SubAckReasonCode::SubscriptionIdentifiersNotSupported => 0xA1,
//             SubAckReasonCode::WildcardSubscriptionsNotSupported => 0xA2,
//         };

//         buf.put_u8(val);
//         Ok(())
//     }
// }