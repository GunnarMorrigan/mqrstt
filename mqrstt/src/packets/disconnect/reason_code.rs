crate::packets::macros::reason_code!(DisconnectReasonCode,
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
    WildcardSubscriptionsNotSupported
);

// #[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
// pub enum DisconnectReasonCode {
//     #[default]
//     NormalDisconnection,
//     DisconnectWithWillMessage,
//     UnspecifiedError,
//     MalformedPacket,
//     ProtocolError,
//     ImplementationSpecificError,
//     NotAuthorized,
//     ServerBusy,
//     ServerShuttingDown,
//     KeepAliveTimeout,
//     SessionTakenOver,
//     TopicFilterInvalid,
//     TopicNameInvalid,
//     ReceiveMaximumExceeded,
//     TopicAliasInvalid,
//     PacketTooLarge,
//     MessageRateTooHigh,
//     QuotaExceeded,
//     AdministrativeAction,
//     PayloadFormatInvalid,
//     RetainNotSupported,
//     QosNotSupported,
//     UseAnotherServer,
//     ServerMoved,
//     SharedSubscriptionsNotSupported,
//     ConnectionRateExceeded,
//     MaximumConnectTime,
//     SubscriptionIdentifiersNotSupported,
//     WildcardSubscriptionsNotSupported,
// }

// impl MqttRead for DisconnectReasonCode {
//     fn read(buf: &mut bytes::Bytes) -> Result<Self, DeserializeError> {
//         if buf.is_empty() {
//             return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), 0, 1));
//         }

//         match buf.get_u8() {
//             0x00 => Ok(DisconnectReasonCode::NormalDisconnection),
//             0x04 => Ok(DisconnectReasonCode::DisconnectWithWillMessage),
//             0x80 => Ok(DisconnectReasonCode::UnspecifiedError),
//             0x81 => Ok(DisconnectReasonCode::MalformedPacket),
//             0x82 => Ok(DisconnectReasonCode::ProtocolError),
//             0x83 => Ok(DisconnectReasonCode::ImplementationSpecificError),
//             0x87 => Ok(DisconnectReasonCode::NotAuthorized),
//             0x89 => Ok(DisconnectReasonCode::ServerBusy),
//             0x8B => Ok(DisconnectReasonCode::ServerShuttingDown),
//             0x8D => Ok(DisconnectReasonCode::KeepAliveTimeout),
//             0x8E => Ok(DisconnectReasonCode::SessionTakenOver),
//             0x8F => Ok(DisconnectReasonCode::TopicFilterInvalid),
//             0x90 => Ok(DisconnectReasonCode::TopicNameInvalid),
//             0x93 => Ok(DisconnectReasonCode::ReceiveMaximumExceeded),
//             0x94 => Ok(DisconnectReasonCode::TopicAliasInvalid),
//             0x95 => Ok(DisconnectReasonCode::PacketTooLarge),
//             0x96 => Ok(DisconnectReasonCode::MessageRateTooHigh),
//             0x97 => Ok(DisconnectReasonCode::QuotaExceeded),
//             0x98 => Ok(DisconnectReasonCode::AdministrativeAction),
//             0x99 => Ok(DisconnectReasonCode::PayloadFormatInvalid),
//             0x9A => Ok(DisconnectReasonCode::RetainNotSupported),
//             0x9B => Ok(DisconnectReasonCode::QosNotSupported),
//             0x9C => Ok(DisconnectReasonCode::UseAnotherServer),
//             0x9D => Ok(DisconnectReasonCode::ServerMoved),
//             0x9E => Ok(DisconnectReasonCode::SharedSubscriptionsNotSupported),
//             0x9F => Ok(DisconnectReasonCode::ConnectionRateExceeded),
//             0xA0 => Ok(DisconnectReasonCode::MaximumConnectTime),
//             0xA1 => Ok(DisconnectReasonCode::SubscriptionIdentifiersNotSupported),
//             0xA2 => Ok(DisconnectReasonCode::WildcardSubscriptionsNotSupported),
//             t => Err(DeserializeError::UnknownProperty(t)),
//         }
//     }
// }

// impl MqttWrite for DisconnectReasonCode {
//     fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
//         let val = match self {
//             DisconnectReasonCode::NormalDisconnection => 0x00,
//             DisconnectReasonCode::DisconnectWithWillMessage => 0x04,
//             DisconnectReasonCode::UnspecifiedError => 0x80,
//             DisconnectReasonCode::MalformedPacket => 0x81,
//             DisconnectReasonCode::ProtocolError => 0x82,
//             DisconnectReasonCode::ImplementationSpecificError => 0x83,
//             DisconnectReasonCode::NotAuthorized => 0x87,
//             DisconnectReasonCode::ServerBusy => 0x89,
//             DisconnectReasonCode::ServerShuttingDown => 0x8B,

//             DisconnectReasonCode::KeepAliveTimeout => 0x8D,
//             DisconnectReasonCode::SessionTakenOver => 0x8E,
//             DisconnectReasonCode::TopicFilterInvalid => 0x8F,

//             DisconnectReasonCode::TopicNameInvalid => 0x90,
//             DisconnectReasonCode::ReceiveMaximumExceeded => 0x93,
//             DisconnectReasonCode::TopicAliasInvalid => 0x94,
//             DisconnectReasonCode::PacketTooLarge => 0x95,
//             DisconnectReasonCode::MessageRateTooHigh => 0x96,
//             DisconnectReasonCode::QuotaExceeded => 0x97,
//             DisconnectReasonCode::AdministrativeAction => 0x98,
//             DisconnectReasonCode::PayloadFormatInvalid => 0x99,
//             DisconnectReasonCode::RetainNotSupported => 0x9A,
//             DisconnectReasonCode::QosNotSupported => 0x9B,
//             DisconnectReasonCode::UseAnotherServer => 0x9C,
//             DisconnectReasonCode::ServerMoved => 0x9D,
//             DisconnectReasonCode::SharedSubscriptionsNotSupported => 0x9E,
//             DisconnectReasonCode::ConnectionRateExceeded => 0x9F,
//             DisconnectReasonCode::MaximumConnectTime => 0xA0,
//             DisconnectReasonCode::SubscriptionIdentifiersNotSupported => 0xA1,
//             DisconnectReasonCode::WildcardSubscriptionsNotSupported => 0xA2,
//         };

//         buf.put_u8(val);

//         Ok(())
//     }
// }