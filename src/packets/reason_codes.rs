use super::errors::PacketError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ConAckReasonCode {
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

impl TryFrom<u8> for ConAckReasonCode {
    type Error = PacketError;

    fn try_from(reason_code: u8) -> Result<Self, Self::Error> {
        match reason_code {
            0x00 => Ok(ConAckReasonCode::Success),
            0x80 => Ok(ConAckReasonCode::UnspecifiedError),
            0x81 => Ok(ConAckReasonCode::MalformedPacket),
            0x82 => Ok(ConAckReasonCode::ProtocolError),
            0x83 => Ok(ConAckReasonCode::ImplementationSpecificError),
            0x84 => Ok(ConAckReasonCode::UnsupportedProtocolVersion),
            0x85 => Ok(ConAckReasonCode::ClientIdentifierNotValid),
            0x86 => Ok(ConAckReasonCode::BadUsernameOrPassword),
            0x87 => Ok(ConAckReasonCode::NotAuthorized),
            0x88 => Ok(ConAckReasonCode::ServerUnavailable),
            0x89 => Ok(ConAckReasonCode::ServerBusy),
            0x8A => Ok(ConAckReasonCode::Banned),
            0x8C => Ok(ConAckReasonCode::BadAuthenticationMethod),
            0x90 => Ok(ConAckReasonCode::TopicNameInvalid),
            0x95 => Ok(ConAckReasonCode::PacketTooLarge),
            0x97 => Ok(ConAckReasonCode::QuotaExceeded),
            0x99 => Ok(ConAckReasonCode::PayloadFormatInvalid),
            0x9A => Ok(ConAckReasonCode::RetainNotSupported),
            0x9B => Ok(ConAckReasonCode::QosNotSupported),
            0x9C => Ok(ConAckReasonCode::UseAnotherServer),
            0x9D => Ok(ConAckReasonCode::ServerMoved),
            0x9F => Ok(ConAckReasonCode::ConnectionRateExceeded),
            r => Err(PacketError::UnexpectedReasonCode(r, super::PacketType::ConnAck)),
        }
    }
}

impl Into<u8> for ConAckReasonCode {
    fn into(self) -> u8 {
        match self {
            ConAckReasonCode::Success => 0x00,
            ConAckReasonCode::UnspecifiedError => 0x80,
            ConAckReasonCode::MalformedPacket => 0x81,
            ConAckReasonCode::ProtocolError => 0x82,
            ConAckReasonCode::ImplementationSpecificError => 0x83,
            ConAckReasonCode::UnsupportedProtocolVersion => 0x84,
            ConAckReasonCode::ClientIdentifierNotValid => 0x85,
            ConAckReasonCode::BadUsernameOrPassword => 0x86,
            ConAckReasonCode::NotAuthorized => 0x87,
            ConAckReasonCode::ServerUnavailable => 0x88,
            ConAckReasonCode::ServerBusy => 0x89,
            ConAckReasonCode::Banned => 0x8A,
            ConAckReasonCode::BadAuthenticationMethod => 0x8C,
            ConAckReasonCode::TopicNameInvalid => 0x90,
            ConAckReasonCode::PacketTooLarge => 0x95,
            ConAckReasonCode::QuotaExceeded => 0x97,
            ConAckReasonCode::PayloadFormatInvalid => 0x99,
            ConAckReasonCode::RetainNotSupported => 0x9A,
            ConAckReasonCode::QosNotSupported => 0x9B,
            ConAckReasonCode::UseAnotherServer => 0x9C,
            ConAckReasonCode::ServerMoved => 0x9D,
            ConAckReasonCode::ConnectionRateExceeded => 0x9F,
        }
    }
}


pub enum AuthReasonCode {
    Success,
    ContinueAuthentication,
    ReAuthenticate,
}

impl TryFrom<u8> for AuthReasonCode {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(AuthReasonCode::Success),
            0x18 => Ok(AuthReasonCode::ContinueAuthentication),
            0x19 => Ok(AuthReasonCode::ReAuthenticate),
            _ => Err("Error serializing. This should be replaced with proper error type".to_string())
        }
    }
}

impl Into<u8> for AuthReasonCode {
    fn into(self) -> u8 {
        match self {
            AuthReasonCode::Success => 0x00,
            AuthReasonCode::ContinueAuthentication => 0x18,
            AuthReasonCode::ReAuthenticate => 0x19,
        }
    }
}

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

impl TryFrom<u8> for DisconnectReasonCode {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
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
            _ => Err("Error serializing. This should be replaced with proper error type".to_string())
        }
    }
}


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

impl TryFrom<u8> for PubAckReasonCode{
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(PubAckReasonCode::Success),
            0x10 => Ok(PubAckReasonCode::NoMatchingSubscribers),
            0x80 => Ok(PubAckReasonCode::UnspecifiedError),
            0x83 => Ok(PubAckReasonCode::ImplementationSpecificError),
            0x87 => Ok(PubAckReasonCode::NotAuthorized),
            0x90 => Ok(PubAckReasonCode::TopicNameInvalid),
            0x91 => Ok(PubAckReasonCode::PacketIdentifierInUse),
            0x97 => Ok(PubAckReasonCode::QuotaExceeded),
            0x99 => Ok(PubAckReasonCode::PayloadFormatInvalid),
            _ => Err("Error serializing. This should be replaced with proper error type".to_string())
        }
    }
}

pub enum PubCompReasonCode{
        Success,
        PacketIdentifierNotFound,
}

impl TryFrom<u8> for PubCompReasonCode{
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(PubCompReasonCode::Success),
            0x92 => Ok(PubCompReasonCode::PacketIdentifierNotFound),
            _ => Err("Error serializing. This should be replaced with proper error type".to_string())
        }
    }
}

impl Into<u8> for PubCompReasonCode {
    fn into(self) -> u8 {
        match self {
            PubCompReasonCode::Success => 0x00,
            PubCompReasonCode::PacketIdentifierNotFound => 0x92,
        }
    }
}

pub enum PubRecReasonCode{
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

impl TryFrom<u8> for PubRecReasonCode{
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(PubRecReasonCode::Success),
            0x10 => Ok(PubRecReasonCode::NoMatchingSubscribers),
            0x80 => Ok(PubRecReasonCode::UnspecifiedError),
            0x83 => Ok(PubRecReasonCode::ImplementationSpecificError),
            0x87 => Ok(PubRecReasonCode::NotAuthorized),
            0x90 => Ok(PubRecReasonCode::TopicNameInvalid),
            0x91 => Ok(PubRecReasonCode::PacketIdentifierInUse),
            0x97 => Ok(PubRecReasonCode::QuotaExceeded),
            0x99 => Ok(PubRecReasonCode::PayloadFormatInvalid),
            _ => Err("Error serializing. This should be replaced with proper error type".to_string()),
        }
    }
}

impl Into<u8> for PubRecReasonCode{
    fn into(self) -> u8 {
        match self {
            PubRecReasonCode::Success => 0x00,
            PubRecReasonCode::NoMatchingSubscribers => 0x10,
            PubRecReasonCode::UnspecifiedError => 0x80,
            PubRecReasonCode::ImplementationSpecificError => 0x83,
            PubRecReasonCode::NotAuthorized => 0x87,
            PubRecReasonCode::TopicNameInvalid => 0x90,
            PubRecReasonCode::PacketIdentifierInUse => 0x91,
            PubRecReasonCode::QuotaExceeded => 0x97,
            PubRecReasonCode::PayloadFormatInvalid => 0x99,
        }
    }
}

pub enum PubRelReasonCode{
    Success,
    PacketIdentifierNotFound,
}

impl TryFrom<u8> for PubRelReasonCode{
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(PubRelReasonCode::Success),
            0x92 => Ok(PubRelReasonCode::PacketIdentifierNotFound),
            _ => Err("Error serializing. This should be replaced with proper error type".to_string())
        }
    }
}

impl Into<u8> for PubRelReasonCode {
    fn into(self) -> u8 {
        match self {
            PubRelReasonCode::Success => 0x00,
            PubRelReasonCode::PacketIdentifierNotFound => 0x92,
        }
    }
}

pub enum SubAckReasonCode{
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


impl TryFrom<u8> for SubAckReasonCode{
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
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
            _ => Err("Error serializing. This should be replaced with proper error type".to_string())
        }
    }
}

impl Into<u8> for SubAckReasonCode {
    fn into(self) -> u8 {
        match self {
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
        }
    }
}

pub enum UnsubAckReasonCode {
    Success,
    NoSubscriptionExisted,
    UnspecifiedError,
    ImplementationSpecificError,
    NotAuthorized,
    TopicFilterInvalid,
    PacketIdentifierInUse,
}

impl TryFrom<u8> for UnsubAckReasonCode{
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(UnsubAckReasonCode::Success),
            0x11 => Ok(UnsubAckReasonCode::NoSubscriptionExisted),
            0x80 => Ok(UnsubAckReasonCode::UnspecifiedError),
            0x83 => Ok(UnsubAckReasonCode::ImplementationSpecificError),
            0x87 => Ok(UnsubAckReasonCode::NotAuthorized),
            0x8F => Ok(UnsubAckReasonCode::TopicFilterInvalid),
            0x91 => Ok(UnsubAckReasonCode::PacketIdentifierInUse),
            _ => Err("Error serializing. This should be replaced with proper error type".to_string())
        }
    }
}

impl Into<u8> for UnsubAckReasonCode {
    fn into(self) -> u8 {
        match self {
            UnsubAckReasonCode::Success => 0x00,
            UnsubAckReasonCode::NoSubscriptionExisted => 0x11,
            UnsubAckReasonCode::UnspecifiedError => 0x80,
            UnsubAckReasonCode::ImplementationSpecificError => 0x83,
            UnsubAckReasonCode::NotAuthorized => 0x87,
            UnsubAckReasonCode::TopicFilterInvalid => 0x8F,
            UnsubAckReasonCode::PacketIdentifierInUse => 0x91,
        }
    }
}