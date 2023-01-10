use bytes::{Buf, BufMut};

use super::error::DeserializeError;
use super::mqtt_traits::{MqttRead, MqttWrite};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ConnAckReasonCode {
    Success,
    UnspecifiedError,
    MalformedPacket,
    ProtocolError,
    ImplementationSpecificError,
    UnsupportedProtocolVersion,
    ClientIdentifierNotValid,
    BadUsernameOrPassword,
    NotAuthorized,
    ServerUnavailable,
    ServerBusy,
    Banned,
    BadAuthenticationMethod,
    TopicNameInvalid,
    PacketTooLarge,
    QuotaExceeded,
    PayloadFormatInvalid,
    RetainNotSupported,
    QosNotSupported,
    UseAnotherServer,
    ServerMoved,
    ConnectionRateExceeded,
}

impl MqttRead for ConnAckReasonCode {
    fn read(buf: &mut bytes::Bytes) -> Result<Self, DeserializeError> {
        if buf.is_empty() {
            return Err(DeserializeError::InsufficientData(
                "ConAckReasonCode".to_string(),
                0,
                1,
            ));
        }

        match buf.get_u8() {
            0x00 => Ok(ConnAckReasonCode::Success),
            0x80 => Ok(ConnAckReasonCode::UnspecifiedError),
            0x81 => Ok(ConnAckReasonCode::MalformedPacket),
            0x82 => Ok(ConnAckReasonCode::ProtocolError),
            0x83 => Ok(ConnAckReasonCode::ImplementationSpecificError),
            0x84 => Ok(ConnAckReasonCode::UnsupportedProtocolVersion),
            0x85 => Ok(ConnAckReasonCode::ClientIdentifierNotValid),
            0x86 => Ok(ConnAckReasonCode::BadUsernameOrPassword),
            0x87 => Ok(ConnAckReasonCode::NotAuthorized),
            0x88 => Ok(ConnAckReasonCode::ServerUnavailable),
            0x89 => Ok(ConnAckReasonCode::ServerBusy),
            0x8A => Ok(ConnAckReasonCode::Banned),
            0x8C => Ok(ConnAckReasonCode::BadAuthenticationMethod),
            0x90 => Ok(ConnAckReasonCode::TopicNameInvalid),
            0x95 => Ok(ConnAckReasonCode::PacketTooLarge),
            0x97 => Ok(ConnAckReasonCode::QuotaExceeded),
            0x99 => Ok(ConnAckReasonCode::PayloadFormatInvalid),
            0x9A => Ok(ConnAckReasonCode::RetainNotSupported),
            0x9B => Ok(ConnAckReasonCode::QosNotSupported),
            0x9C => Ok(ConnAckReasonCode::UseAnotherServer),
            0x9D => Ok(ConnAckReasonCode::ServerMoved),
            0x9F => Ok(ConnAckReasonCode::ConnectionRateExceeded),
            t => Err(DeserializeError::UnknownProperty(t)),
        }
    }
}

impl MqttWrite for ConnAckReasonCode {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
        let val = match self {
            ConnAckReasonCode::Success => 0x00,
            ConnAckReasonCode::UnspecifiedError => 0x80,
            ConnAckReasonCode::MalformedPacket => 0x81,
            ConnAckReasonCode::ProtocolError => 0x82,
            ConnAckReasonCode::ImplementationSpecificError => 0x83,
            ConnAckReasonCode::UnsupportedProtocolVersion => 0x84,
            ConnAckReasonCode::ClientIdentifierNotValid => 0x85,
            ConnAckReasonCode::BadUsernameOrPassword => 0x86,
            ConnAckReasonCode::NotAuthorized => 0x87,
            ConnAckReasonCode::ServerUnavailable => 0x88,
            ConnAckReasonCode::ServerBusy => 0x89,
            ConnAckReasonCode::Banned => 0x8A,
            ConnAckReasonCode::BadAuthenticationMethod => 0x8C,
            ConnAckReasonCode::TopicNameInvalid => 0x90,
            ConnAckReasonCode::PacketTooLarge => 0x95,
            ConnAckReasonCode::QuotaExceeded => 0x97,
            ConnAckReasonCode::PayloadFormatInvalid => 0x99,
            ConnAckReasonCode::RetainNotSupported => 0x9A,
            ConnAckReasonCode::QosNotSupported => 0x9B,
            ConnAckReasonCode::UseAnotherServer => 0x9C,
            ConnAckReasonCode::ServerMoved => 0x9D,
            ConnAckReasonCode::ConnectionRateExceeded => 0x9F,
        };

        buf.put_u8(val);

        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum AuthReasonCode {
    Success,
    ContinueAuthentication,
    ReAuthenticate,
}

impl MqttRead for AuthReasonCode {
    fn read(buf: &mut bytes::Bytes) -> Result<Self, DeserializeError> {
        if buf.is_empty() {
            return Err(DeserializeError::InsufficientData(
                "AuthReasonCode".to_string(),
                0,
                1,
            ));
        }

        match buf.get_u8() {
            0x00 => Ok(AuthReasonCode::Success),
            0x18 => Ok(AuthReasonCode::ContinueAuthentication),
            0x19 => Ok(AuthReasonCode::ReAuthenticate),
            t => Err(DeserializeError::UnknownProperty(t)),
        }
    }
}

impl MqttWrite for AuthReasonCode {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
        let val = match self {
            AuthReasonCode::Success => 0x00,
            AuthReasonCode::ContinueAuthentication => 0x18,
            AuthReasonCode::ReAuthenticate => 0x19,
        };

        buf.put_u8(val);

        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum DisconnectReasonCode {
    NormalDisconnection,
    DisconnectWithWillMessage,
    UnspecifiedError,
    MalformedPacket,
    ProtocolError,
    ImplementationSpecificError,
    NotAuthorized,
    ServerBusy,
    ServerShuttingDown,
    KeepAliveTimeout,
    SessionTakenOver,
    TopicFilterInvalid,
    TopicNameInvalid,
    ReceiveMaximumExceeded,
    TopicAliasInvalid,
    PacketTooLarge,
    MessageRateTooHigh,
    QuotaExceeded,
    AdministrativeAction,
    PayloadFormatInvalid,
    RetainNotSupported,
    QosNotSupported,
    UseAnotherServer,
    ServerMoved,
    SharedSubscriptionsNotSupported,
    ConnectionRateExceeded,
    MaximumConnectTime,
    SubscriptionIdentifiersNotSupported,
    WildcardSubscriptionsNotSupported,
}

impl MqttRead for DisconnectReasonCode {
    fn read(buf: &mut bytes::Bytes) -> Result<Self, DeserializeError> {
        if buf.is_empty() {
            return Err(DeserializeError::InsufficientData(
                "DisconnectReasonCode".to_string(),
                0,
                1,
            ));
        }

        match buf.get_u8() {
            0x00 => Ok(DisconnectReasonCode::NormalDisconnection),
            0x04 => Ok(DisconnectReasonCode::DisconnectWithWillMessage),
            0x80 => Ok(DisconnectReasonCode::UnspecifiedError),
            0x81 => Ok(DisconnectReasonCode::MalformedPacket),
            0x82 => Ok(DisconnectReasonCode::ProtocolError),
            0x83 => Ok(DisconnectReasonCode::ImplementationSpecificError),
            0x87 => Ok(DisconnectReasonCode::NotAuthorized),
            0x89 => Ok(DisconnectReasonCode::ServerBusy),
            0x8B => Ok(DisconnectReasonCode::ServerShuttingDown),
            0x8D => Ok(DisconnectReasonCode::KeepAliveTimeout),
            0x8E => Ok(DisconnectReasonCode::SessionTakenOver),
            0x8F => Ok(DisconnectReasonCode::TopicFilterInvalid),
            0x90 => Ok(DisconnectReasonCode::TopicNameInvalid),
            0x93 => Ok(DisconnectReasonCode::ReceiveMaximumExceeded),
            0x94 => Ok(DisconnectReasonCode::TopicAliasInvalid),
            0x95 => Ok(DisconnectReasonCode::PacketTooLarge),
            0x96 => Ok(DisconnectReasonCode::MessageRateTooHigh),
            0x97 => Ok(DisconnectReasonCode::QuotaExceeded),
            0x98 => Ok(DisconnectReasonCode::AdministrativeAction),
            0x99 => Ok(DisconnectReasonCode::PayloadFormatInvalid),
            0x9A => Ok(DisconnectReasonCode::RetainNotSupported),
            0x9B => Ok(DisconnectReasonCode::QosNotSupported),
            0x9C => Ok(DisconnectReasonCode::UseAnotherServer),
            0x9D => Ok(DisconnectReasonCode::ServerMoved),
            0x9E => Ok(DisconnectReasonCode::SharedSubscriptionsNotSupported),
            0x9F => Ok(DisconnectReasonCode::ConnectionRateExceeded),
            0xA0 => Ok(DisconnectReasonCode::MaximumConnectTime),
            0xA1 => Ok(DisconnectReasonCode::SubscriptionIdentifiersNotSupported),
            0xA2 => Ok(DisconnectReasonCode::WildcardSubscriptionsNotSupported),
            t => Err(DeserializeError::UnknownProperty(t)),
        }
    }
}

impl MqttWrite for DisconnectReasonCode {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
        let val = match self {
            DisconnectReasonCode::NormalDisconnection => 0x00,
            DisconnectReasonCode::DisconnectWithWillMessage => 0x04,
            DisconnectReasonCode::UnspecifiedError => 0x80,
            DisconnectReasonCode::MalformedPacket => 0x81,
            DisconnectReasonCode::ProtocolError => 0x82,
            DisconnectReasonCode::ImplementationSpecificError => 0x83,
            DisconnectReasonCode::NotAuthorized => 0x87,
            DisconnectReasonCode::ServerBusy => 0x89,
            DisconnectReasonCode::ServerShuttingDown => 0x8B,
            DisconnectReasonCode::KeepAliveTimeout => 0x8D,
            DisconnectReasonCode::SessionTakenOver => 0x8E,
            DisconnectReasonCode::TopicFilterInvalid => 0x8F,
            DisconnectReasonCode::TopicNameInvalid => 0x90,
            DisconnectReasonCode::ReceiveMaximumExceeded => 0x93,
            DisconnectReasonCode::TopicAliasInvalid => 0x94,
            DisconnectReasonCode::PacketTooLarge => 0x95,
            DisconnectReasonCode::MessageRateTooHigh => 0x96,
            DisconnectReasonCode::QuotaExceeded => 0x97,
            DisconnectReasonCode::AdministrativeAction => 0x98,
            DisconnectReasonCode::PayloadFormatInvalid => 0x99,
            DisconnectReasonCode::RetainNotSupported => 0x9A,
            DisconnectReasonCode::QosNotSupported => 0x9B,
            DisconnectReasonCode::UseAnotherServer => 0x9C,
            DisconnectReasonCode::ServerMoved => 0x9D,
            DisconnectReasonCode::SharedSubscriptionsNotSupported => 0x9E,
            DisconnectReasonCode::ConnectionRateExceeded => 0x9F,
            DisconnectReasonCode::MaximumConnectTime => 0xA0,
            DisconnectReasonCode::SubscriptionIdentifiersNotSupported => 0xA1,
            DisconnectReasonCode::WildcardSubscriptionsNotSupported => 0xA2,
        };

        buf.put_u8(val);

        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum PubAckReasonCode {
    Success,
    NoMatchingSubscribers,
    UnspecifiedError,
    ImplementationSpecificError,
    NotAuthorized,
    TopicNameInvalid,
    PacketIdentifierInUse,
    QuotaExceeded,
    PayloadFormatInvalid,
}

impl MqttRead for PubAckReasonCode {
    fn read(buf: &mut bytes::Bytes) -> Result<Self, DeserializeError> {
        if buf.is_empty() {
            return Err(DeserializeError::InsufficientData(
                "PubAckReasonCode".to_string(),
                0,
                1,
            ));
        }

        match buf.get_u8() {
            0x00 => Ok(PubAckReasonCode::Success),
            0x10 => Ok(PubAckReasonCode::NoMatchingSubscribers),
            0x80 => Ok(PubAckReasonCode::UnspecifiedError),
            0x83 => Ok(PubAckReasonCode::ImplementationSpecificError),
            0x87 => Ok(PubAckReasonCode::NotAuthorized),
            0x90 => Ok(PubAckReasonCode::TopicNameInvalid),
            0x91 => Ok(PubAckReasonCode::PacketIdentifierInUse),
            0x97 => Ok(PubAckReasonCode::QuotaExceeded),
            0x99 => Ok(PubAckReasonCode::PayloadFormatInvalid),
            t => Err(DeserializeError::UnknownProperty(t)),
        }
    }
}

impl MqttWrite for PubAckReasonCode {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
        let val = match self {
            PubAckReasonCode::Success => 0x00,
            PubAckReasonCode::NoMatchingSubscribers => 0x10,
            PubAckReasonCode::UnspecifiedError => 0x80,
            PubAckReasonCode::ImplementationSpecificError => 0x83,
            PubAckReasonCode::NotAuthorized => 0x87,
            PubAckReasonCode::TopicNameInvalid => 0x90,
            PubAckReasonCode::PacketIdentifierInUse => 0x91,
            PubAckReasonCode::QuotaExceeded => 0x97,
            PubAckReasonCode::PayloadFormatInvalid => 0x99,
        };

        buf.put_u8(val);

        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum PubCompReasonCode {
    Success,
    PacketIdentifierNotFound,
}

impl MqttRead for PubCompReasonCode {
    fn read(buf: &mut bytes::Bytes) -> Result<Self, DeserializeError> {
        if buf.is_empty() {
            return Err(DeserializeError::InsufficientData(
                "PubCompReasonCode".to_string(),
                0,
                1,
            ));
        }

        match buf.get_u8() {
            0x00 => Ok(PubCompReasonCode::Success),
            0x92 => Ok(PubCompReasonCode::PacketIdentifierNotFound),
            t => Err(DeserializeError::UnknownProperty(t)),
        }
    }
}

impl MqttWrite for PubCompReasonCode {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
        let val = match self {
            PubCompReasonCode::Success => 0x00,
            PubCompReasonCode::PacketIdentifierNotFound => 0x92,
        };

        buf.put_u8(val);
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum PubRecReasonCode {
    Success,
    NoMatchingSubscribers,
    UnspecifiedError,
    ImplementationSpecificError,
    NotAuthorized,
    TopicNameInvalid,
    PacketIdentifierInUse,
    QuotaExceeded,
    PayloadFormatInvalid,
}

impl MqttRead for PubRecReasonCode {
    fn read(buf: &mut bytes::Bytes) -> Result<Self, DeserializeError> {
        if buf.is_empty() {
            return Err(DeserializeError::InsufficientData(
                "PubRecReasonCode".to_string(),
                0,
                1,
            ));
        }

        match buf.get_u8() {
            0x00 => Ok(PubRecReasonCode::Success),
            0x10 => Ok(PubRecReasonCode::NoMatchingSubscribers),
            0x80 => Ok(PubRecReasonCode::UnspecifiedError),
            0x83 => Ok(PubRecReasonCode::ImplementationSpecificError),
            0x87 => Ok(PubRecReasonCode::NotAuthorized),
            0x90 => Ok(PubRecReasonCode::TopicNameInvalid),
            0x91 => Ok(PubRecReasonCode::PacketIdentifierInUse),
            0x97 => Ok(PubRecReasonCode::QuotaExceeded),
            0x99 => Ok(PubRecReasonCode::PayloadFormatInvalid),
            t => Err(DeserializeError::UnknownProperty(t)),
        }
    }
}

impl MqttWrite for PubRecReasonCode {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
        let val = match self {
            PubRecReasonCode::Success => 0x00,
            PubRecReasonCode::NoMatchingSubscribers => 0x10,
            PubRecReasonCode::UnspecifiedError => 0x80,
            PubRecReasonCode::ImplementationSpecificError => 0x83,
            PubRecReasonCode::NotAuthorized => 0x87,
            PubRecReasonCode::TopicNameInvalid => 0x90,
            PubRecReasonCode::PacketIdentifierInUse => 0x91,
            PubRecReasonCode::QuotaExceeded => 0x97,
            PubRecReasonCode::PayloadFormatInvalid => 0x99,
        };

        buf.put_u8(val);
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum PubRelReasonCode {
    Success,
    PacketIdentifierNotFound,
}

impl MqttRead for PubRelReasonCode {
    fn read(buf: &mut bytes::Bytes) -> Result<Self, DeserializeError> {
        if buf.is_empty() {
            return Err(DeserializeError::InsufficientData(
                "PubRelReasonCode".to_string(),
                0,
                1,
            ));
        }

        match buf.get_u8() {
            0x00 => Ok(PubRelReasonCode::Success),
            0x92 => Ok(PubRelReasonCode::PacketIdentifierNotFound),
            t => Err(DeserializeError::UnknownProperty(t)),
        }
    }
}

impl MqttWrite for PubRelReasonCode {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
        let val = match self {
            PubRelReasonCode::Success => 0x00,
            PubRelReasonCode::PacketIdentifierNotFound => 0x92,
        };

        buf.put_u8(val);
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum SubAckReasonCode {
    GrantedQoS0,
    GrantedQoS1,
    GrantedQoS2,
    UnspecifiedError,
    ImplementationSpecificError,
    NotAuthorized,
    TopicFilterInvalid,
    PacketIdentifierInUse,
    QuotaExceeded,
    SharedSubscriptionsNotSupported,
    SubscriptionIdentifiersNotSupported,
    WildcardSubscriptionsNotSupported,
}

impl MqttRead for SubAckReasonCode {
    fn read(buf: &mut bytes::Bytes) -> Result<Self, DeserializeError> {
        if buf.is_empty() {
            return Err(DeserializeError::InsufficientData(
                "SubAckReasonCode".to_string(),
                0,
                1,
            ));
        }

        match buf.get_u8() {
            0x00 => Ok(SubAckReasonCode::GrantedQoS0),
            0x01 => Ok(SubAckReasonCode::GrantedQoS1),
            0x02 => Ok(SubAckReasonCode::GrantedQoS2),
            0x80 => Ok(SubAckReasonCode::UnspecifiedError),
            0x83 => Ok(SubAckReasonCode::ImplementationSpecificError),
            0x87 => Ok(SubAckReasonCode::NotAuthorized),
            0x8F => Ok(SubAckReasonCode::TopicFilterInvalid),
            0x91 => Ok(SubAckReasonCode::PacketIdentifierInUse),
            0x97 => Ok(SubAckReasonCode::QuotaExceeded),
            0x9E => Ok(SubAckReasonCode::SharedSubscriptionsNotSupported),
            0xA1 => Ok(SubAckReasonCode::SubscriptionIdentifiersNotSupported),
            0xA2 => Ok(SubAckReasonCode::WildcardSubscriptionsNotSupported),
            t => Err(DeserializeError::UnknownProperty(t)),
        }
    }
}

impl MqttWrite for SubAckReasonCode {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
        let val = match self {
            SubAckReasonCode::GrantedQoS0 => 0x00,
            SubAckReasonCode::GrantedQoS1 => 0x01,
            SubAckReasonCode::GrantedQoS2 => 0x02,
            SubAckReasonCode::UnspecifiedError => 0x80,
            SubAckReasonCode::ImplementationSpecificError => 0x83,
            SubAckReasonCode::NotAuthorized => 0x87,
            SubAckReasonCode::TopicFilterInvalid => 0x8F,
            SubAckReasonCode::PacketIdentifierInUse => 0x91,
            SubAckReasonCode::QuotaExceeded => 0x97,
            SubAckReasonCode::SharedSubscriptionsNotSupported => 0x9E,
            SubAckReasonCode::SubscriptionIdentifiersNotSupported => 0xA1,
            SubAckReasonCode::WildcardSubscriptionsNotSupported => 0xA2,
        };

        buf.put_u8(val);
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum UnsubAckReasonCode {
    Success,
    NoSubscriptionExisted,
    UnspecifiedError,
    ImplementationSpecificError,
    NotAuthorized,
    TopicFilterInvalid,
    PacketIdentifierInUse,
}

impl MqttRead for UnsubAckReasonCode {
    fn read(buf: &mut bytes::Bytes) -> Result<Self, DeserializeError> {
        if buf.is_empty() {
            return Err(DeserializeError::InsufficientData(
                "UnsubAckReasonCode".to_string(),
                0,
                1,
            ));
        }

        match buf.get_u8() {
            0x00 => Ok(UnsubAckReasonCode::Success),
            0x11 => Ok(UnsubAckReasonCode::NoSubscriptionExisted),
            0x80 => Ok(UnsubAckReasonCode::UnspecifiedError),
            0x83 => Ok(UnsubAckReasonCode::ImplementationSpecificError),
            0x87 => Ok(UnsubAckReasonCode::NotAuthorized),
            0x8F => Ok(UnsubAckReasonCode::TopicFilterInvalid),
            0x91 => Ok(UnsubAckReasonCode::PacketIdentifierInUse),
            t => Err(DeserializeError::UnknownProperty(t)),
        }
    }
}

impl MqttWrite for UnsubAckReasonCode {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
        let val = match self {
            UnsubAckReasonCode::Success => 0x00,
            UnsubAckReasonCode::NoSubscriptionExisted => 0x11,
            UnsubAckReasonCode::UnspecifiedError => 0x80,
            UnsubAckReasonCode::ImplementationSpecificError => 0x83,
            UnsubAckReasonCode::NotAuthorized => 0x87,
            UnsubAckReasonCode::TopicFilterInvalid => 0x8F,
            UnsubAckReasonCode::PacketIdentifierInUse => 0x91,
        };

        buf.put_u8(val);
        Ok(())
    }
}
