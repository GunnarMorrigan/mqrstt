

crate::packets::macros::reason_code!(
    SubAckReasonCode,
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
    WildcardSubscriptionsNotSupported
);




// #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
// pub enum SubAckReasonCode {
//     GrantedQoS0,
//     GrantedQoS1,
//     GrantedQoS2,
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