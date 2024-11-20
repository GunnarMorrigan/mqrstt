

crate::packets::macros::reason_code!(PubRecReasonCode,
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
// pub enum PubRecReasonCode {
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

// impl MqttRead for PubRecReasonCode {
//     fn read(buf: &mut bytes::Bytes) -> Result<Self, DeserializeError> {
//         if buf.is_empty() {
//             return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), 0, 1));
//         }

//         match buf.get_u8() {
//             0x00 => Ok(PubRecReasonCode::Success),
//             0x10 => Ok(PubRecReasonCode::NoMatchingSubscribers),
//             0x80 => Ok(PubRecReasonCode::UnspecifiedError),
//             0x83 => Ok(PubRecReasonCode::ImplementationSpecificError),
//             0x87 => Ok(PubRecReasonCode::NotAuthorized),
//             0x90 => Ok(PubRecReasonCode::TopicNameInvalid),
//             0x91 => Ok(PubRecReasonCode::PacketIdentifierInUse),
//             0x97 => Ok(PubRecReasonCode::QuotaExceeded),
//             0x99 => Ok(PubRecReasonCode::PayloadFormatInvalid),
//             t => Err(DeserializeError::UnknownProperty(t)),
//         }
//     }
// }

// impl MqttWrite for PubRecReasonCode {
//     fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
//         let val = match self {
//             PubRecReasonCode::Success => 0x00,
//             PubRecReasonCode::NoMatchingSubscribers => 0x10,
//             PubRecReasonCode::UnspecifiedError => 0x80,
//             PubRecReasonCode::ImplementationSpecificError => 0x83,
//             PubRecReasonCode::NotAuthorized => 0x87,
//             PubRecReasonCode::TopicNameInvalid => 0x90,
//             PubRecReasonCode::PacketIdentifierInUse => 0x91,
//             PubRecReasonCode::QuotaExceeded => 0x97,
//             PubRecReasonCode::PayloadFormatInvalid => 0x99,
//         };

//         buf.put_u8(val);
//         Ok(())
//     }
// }