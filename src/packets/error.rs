use std::string::FromUtf8Error;

use thiserror::Error;

use super::{PacketType, PropertyType};

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
    InsufficientData(String, usize, usize),

    #[error("There is insufficient to read the protocol version.")]
    InsufficientDataForProtocolVersion,
    
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

#[derive(Error, Clone, Debug)]
pub enum ReadBytes<T>{
    #[error("Normal error")]
    Err(#[from] T),
    
    #[error("Should read more data from the stream.")]
    InsufficientBytes(usize),
}

#[derive(Error, Debug)]
pub enum SerializeError{
    #[error("Can not write {0} in a 4 byte variable integer.")]
    VariableIntegerOverflow(usize),

    #[error("Authentication data without authentication method")]
    AuthDataWithoutAuthMethod,
}