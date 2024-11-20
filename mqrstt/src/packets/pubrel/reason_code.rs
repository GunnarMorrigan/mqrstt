crate::packets::macros::reason_code!(
    PubRelReasonCode,
    Success,
    PacketIdentifierNotFound
);


// #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
// pub enum PubRelReasonCode {
//     Success,
//     PacketIdentifierNotFound,
// }

// impl MqttRead for PubRelReasonCode {
//     fn read(buf: &mut bytes::Bytes) -> Result<Self, DeserializeError> {
//         if buf.is_empty() {
//             return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), 0, 1));
//         }

//         match buf.get_u8() {
//             0x00 => Ok(PubRelReasonCode::Success),
//             0x92 => Ok(PubRelReasonCode::PacketIdentifierNotFound),
//             t => Err(DeserializeError::UnknownProperty(t)),
//         }
//     }
// }

// impl<S> MqttAsyncRead<S> for PubRelReasonCode where S: tokio::io::AsyncReadExt + Unpin {
//     async fn async_read(stream: &mut S) -> Result<(Self, usize), super::error::ReadError> {
//         let code = match stream.read_u8().await? {
//             0x00 => PubRelReasonCode::Success,
//             0x92 => PubRelReasonCode::PacketIdentifierNotFound,
//             t => return Err(super::error::ReadError::DeserializeError(DeserializeError::UnknownProperty(t))),
//         };
//         Ok((code, 1))
//     }
// }

// impl MqttWrite for PubRelReasonCode {
//     fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
//         let val = match self {
//             PubRelReasonCode::Success => 0x00,
//             PubRelReasonCode::PacketIdentifierNotFound => 0x92,
//         };

//         buf.put_u8(val);
//         Ok(())
//     }
// }
