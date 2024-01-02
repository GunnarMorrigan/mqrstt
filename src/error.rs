use std::io;

use async_channel::{RecvError, SendError};

use crate::packets::{
    error::{DeserializeError, ReadBytes, SerializeError},
    reason_codes::ConnAckReasonCode,
    {Packet, PacketType},
};

/// Critical errors that can happen during the operation of the entire client
#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error("No network connection")]
    NoNetwork,

    #[error("No incoming packet handler available: {0}")]
    NoIncomingPacketHandler(#[from] SendError<Packet>),

    #[error("No outgoing packet sender: {0}")]
    NoOutgoingPacketSender(#[from] RecvError),

    #[error("Deserialization error: {0}")]
    DeserializationError(#[from] DeserializeError),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] SerializeError),

    #[error("I/O: {0}")]
    Io(#[from] io::Error),

    #[error("Connection refused, return code: {0:?}")]
    ConnectionRefused(ConnAckReasonCode),

    #[error("Expected ConnAck packet, received: {0:?}")]
    NotConnAck(Packet),

    #[error("Handler Error: {0:?}")]
    HandlerError(#[from] HandlerError),

    #[cfg(feature = "tokio")]
    #[error("Join error")]
    JoinError(#[from] tokio::task::JoinError),
}

/// Errors that the [`mqrstt::MqttHandler`] can emit
#[derive(Debug, Clone, thiserror::Error)]
pub enum HandlerError {
    #[error("Missing Packet ID")]
    MissingPacketId,

    #[error("The incoming channel between network and handler is closed")]
    IncomingNetworkChannelClosed,

    #[error("The outgoing channel between handler and network is closed: {0}")]
    OutgoingNetworkChannelClosed(#[from] SendError<Packet>),

    #[error("Channel between client and handler closed")]
    ClientChannelClosed,

    #[error("Packet Id Channel error, pkid: {0}")]
    PacketIdChannelError(u16),

    #[error("Packet collision error. packet ID: {0}")]
    PacketIdCollision(u16),

    #[error("Received unsolicited ack pkid: {0}")]
    Unsolicited(u16, PacketType),

    #[error("Received an unexpected packet: {0}")]
    UnexpectedPacket(PacketType),
}

/// Errors producable by the [`mqrstt::AsyncClient`]
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ClientError {
    #[error("Internal network channel is closed")]
    NoNetworkChannel,
    #[error("Packet validation error: {0}")]
    ValidationError(#[from] PacketValidationError),
}

/// Errors that can be produced if the packet does not conform to the specifications.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PacketValidationError {
    #[error("Packet size is larger than allowed {0}")]
    MaxPacketSize(usize),
    #[error("Topic too large: {0}")]
    TopicSize(usize),
}

impl From<ReadBytes<DeserializeError>> for ReadBytes<ConnectionError> {
    fn from(value: ReadBytes<DeserializeError>) -> Self {
        match value {
            ReadBytes::Err(err) => ReadBytes::Err(err.into()),
            ReadBytes::InsufficientBytes(id) => ReadBytes::InsufficientBytes(id),
        }
    }
}

impl From<DeserializeError> for ReadBytes<ConnectionError> {
    fn from(value: DeserializeError) -> Self {
        ReadBytes::Err(value.into())
    }
}

impl From<SendError<Packet>> for ReadBytes<ConnectionError> {
    fn from(value: SendError<Packet>) -> Self {
        ReadBytes::Err(value.into())
    }
}
