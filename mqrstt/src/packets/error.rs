use std::string::FromUtf8Error;

use thiserror::Error;

use super::{PacketType, PropertyType};

#[derive(Error, Debug)]
pub enum WriteError {
    #[error("{0}")]
    SerializeError(#[from] SerializeError),
    #[error("{0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum ReadError{
    #[error("{0}")]
    DeserializeError(#[from] DeserializeError),
    #[error("{0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Error, Clone, Debug)]
pub enum DeserializeError {
    #[error("Malformed packet: {0}")]
    MalformedPacketWithInfo(String),

    #[error("Malformed packet")]
    MalformedPacket,

    #[error("Malformed FixedHeader {0:b}")]
    UnknownFixedHeader(u8),

    #[error("Unsupported protocol version")]
    UnsupportedProtocolVersion,

    #[error("Unknown protocol version")]
    UnknownProtocolVersion,

    #[error("There is insufficient for {0} data ({1}) to take {2} bytes")]
    InsufficientData(&'static str, usize, usize),
    
    #[error("There is insufficient to read the protocol version.")]
    InsufficientDataForProtocolVersion,
    
    #[error("Read more data for the packet than indicated length")]
    ReadTooMuchData(&'static str, usize, usize),

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

impl From<String> for DeserializeError {
    fn from(s: String) -> Self {
        DeserializeError::MalformedPacketWithInfo(s)
    }
}

#[derive(Error, Clone, Debug)]
pub enum ReadBytes<T> {
    #[error("Normal error")]
    Err(#[from] T),

    #[error("Should read more data from the stream.")]
    InsufficientBytes(usize),
}

#[derive(Error, Debug)]
pub enum SerializeError {
    #[error("Can not write {0} in a 4 byte variable integer.")]
    VariableIntegerOverflow(usize),

    #[error("UTF-8 encoded strings was {0} bytes long, max length is 65535 bytes")]
    StringTooLong(usize),

    #[error("Authentication data without authentication method")]
    AuthDataWithoutAuthMethod,
}
