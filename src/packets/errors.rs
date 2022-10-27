use thiserror::Error;

use super::{PacketType, PropertyType};

#[derive(Error, Debug)]
pub enum PacketError {
    #[error("Malformed packet: {0}")]
    MalformedPacketWithInfo(String),

    #[error("Malformed packet")]
    MalformedPacket,


    #[error("There is insufficient data to create a complete packet")]
    InsufficientDataForPacket,

    #[error("Reason code {0} is not allowed for packet type {1:?}")]
    UnexpectedReasonCode(u8, PacketType),

    #[error("Property field {0:?} was found at least twice")]
    DuplicateProperty(PropertyType),

    #[error("Property type {0:?} is not allowed for packet type {1:?}")]
    UnexpectedProperty(PropertyType, PacketType),



    // #[error("the data for key `{0}` is not available")]
    // Redaction(String),
    // #[error("invalid header (expected {expected:?}, found {found:?})")]
    // InvalidHeader {
    //     expected: String,
    //     found: String,
    // },
    // #[error("unknown data store error")]
    // Unknown,
}

impl From<String> for PacketError{
    fn from(s: String) -> Self {
        PacketError::MalformedPacketWithInfo(s)
    }
}