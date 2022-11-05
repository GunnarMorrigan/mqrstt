use std::string::FromUtf8Error;

use thiserror::Error;

use super::{PacketType, PropertyType};

#[derive(Error, Debug)]
pub enum DeserializeError {
    #[error("Malformed packet: {0}")]
    MalformedPacketWithInfo(String),

    #[error("Malformed packet")]
    MalformedPacket,

    #[error("There is insufficient data ({0}) to take {1} bytes")]
    InsufficientData(usize, usize),

    #[error("Reason code {0} is not allowed for packet type {1:?}")]
    UnexpectedReasonCode(u8, PacketType),

    #[error("Property field {0:?} was found at least twice")]
    DuplicateProperty(PropertyType),

    #[error("Property type {0:?} is not allowed for packet type {1:?}")]
    UnexpectedProperty(PropertyType, PacketType),

    #[error("Property with id {0} is unknown.")]
    UnknownProperty(u8),

    #[error("Encountered an QoS higher than 2, namely {0:?}")]
    UnknownQoS(u8),

    #[error("Encountered an error when reading in a UTF-8 string. {0}")]
    Utf8Error(FromUtf8Error),
}

impl From<String> for DeserializeError{
    fn from(s: String) -> Self {
        DeserializeError::MalformedPacketWithInfo(s)
    }
}

#[derive(Error, Debug)]
pub enum SerializeError{
    #[error("Can not write {0} in a 4 byte variable integer.")]
    VariableIntegerOverflow(usize)
}