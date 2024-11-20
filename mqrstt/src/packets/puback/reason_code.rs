
crate::packets::macros::reason_code!(
    PubAckReasonCode,
    Success,
    NoMatchingSubscribers,
    UnspecifiedError,
    ImplementationSpecificError,
    NotAuthorized,
    TopicNameInvalid,
    PacketIdentifierInUse,
    QuotaExceeded,
    PayloadFormatInvalid
);

// #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
// pub enum PubAckReasonCode {
//     Success,
//     NoMatchingSubscribers,
//     UnspecifiedError,
//     ImplementationSpecificError,
//     NotAuthorized,
//     TopicNameInvalid,
//     PacketIdentifierInUse,
//     QuotaExceeded,
//     PayloadFormatInvalid,
// }

// impl MqttRead for PubAckReasonCode {
//     fn read(buf: &mut bytes::Bytes) -> Result<Self, DeserializeError> {
//         if buf.is_empty() {
//             return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), 0, 1));
//         }

//         match buf.get_u8() {
//             0x00 => Ok(PubAckReasonCode::Success),
//             0x10 => Ok(PubAckReasonCode::NoMatchingSubscribers),
//             0x80 => Ok(PubAckReasonCode::UnspecifiedError),
//             0x83 => Ok(PubAckReasonCode::ImplementationSpecificError),
//             0x87 => Ok(PubAckReasonCode::NotAuthorized),
//             0x90 => Ok(PubAckReasonCode::TopicNameInvalid),
//             0x91 => Ok(PubAckReasonCode::PacketIdentifierInUse),
//             0x97 => Ok(PubAckReasonCode::QuotaExceeded),
//             0x99 => Ok(PubAckReasonCode::PayloadFormatInvalid),
//             t => Err(DeserializeError::UnknownProperty(t)),
//         }
//     }
// }

// impl MqttWrite for PubAckReasonCode {
//     fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
//         let val = match self {
//             PubAckReasonCode::Success => 0x00,
//             PubAckReasonCode::NoMatchingSubscribers => 0x10,
//             PubAckReasonCode::UnspecifiedError => 0x80,
//             PubAckReasonCode::ImplementationSpecificError => 0x83,
//             PubAckReasonCode::NotAuthorized => 0x87,
//             PubAckReasonCode::TopicNameInvalid => 0x90,
//             PubAckReasonCode::PacketIdentifierInUse => 0x91,
//             PubAckReasonCode::QuotaExceeded => 0x97,
//             PubAckReasonCode::PayloadFormatInvalid => 0x99,
//         };

//         buf.put_u8(val);

//         Ok(())
//     }
// }