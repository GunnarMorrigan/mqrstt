crate::packets::macros::reason_code!(
    UnsubAckReasonCode,
    Success,
    NoSubscriptionExisted,
    UnspecifiedError,
    ImplementationSpecificError,
    NotAuthorized,
    TopicFilterInvalid,
    PacketIdentifierInUse
);


// #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
// pub enum UnsubAckReasonCode {
//     Success,
//     NoSubscriptionExisted,
//     UnspecifiedError,
//     ImplementationSpecificError,
//     NotAuthorized,
//     TopicFilterInvalid,
//     PacketIdentifierInUse,
// }

// impl MqttRead for UnsubAckReasonCode {
//     fn read(buf: &mut bytes::Bytes) -> Result<Self, DeserializeError> {
//         if buf.is_empty() {
//             return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), 0, 1));
//         }

//         match buf.get_u8() {
//             0x00 => Ok(UnsubAckReasonCode::Success),
//             0x11 => Ok(UnsubAckReasonCode::NoSubscriptionExisted),
//             0x80 => Ok(UnsubAckReasonCode::UnspecifiedError),
//             0x83 => Ok(UnsubAckReasonCode::ImplementationSpecificError),
//             0x87 => Ok(UnsubAckReasonCode::NotAuthorized),
//             0x8F => Ok(UnsubAckReasonCode::TopicFilterInvalid),
//             0x91 => Ok(UnsubAckReasonCode::PacketIdentifierInUse),
//             t => Err(DeserializeError::UnknownProperty(t)),
//         }
//     }
// }

// impl MqttWrite for UnsubAckReasonCode {
//     fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::packets::error::SerializeError> {
//         let val = match self {
//             UnsubAckReasonCode::Success => 0x00,
//             UnsubAckReasonCode::NoSubscriptionExisted => 0x11,
//             UnsubAckReasonCode::UnspecifiedError => 0x80,
//             UnsubAckReasonCode::ImplementationSpecificError => 0x83,
//             UnsubAckReasonCode::NotAuthorized => 0x87,
//             UnsubAckReasonCode::TopicFilterInvalid => 0x8F,
//             UnsubAckReasonCode::PacketIdentifierInUse => 0x91,
//         };

//         buf.put_u8(val);
//         Ok(())
//     }
// }
