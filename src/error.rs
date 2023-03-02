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

    #[error("The handler encountered an error")]
    HandlerError(#[from] HandlerError)
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

    #[error("Packet Id error, pkid: {0}")]
    PacketIdError(u16),

    #[error("Packet collision error. packet ID: {0}")]
    PacketIdCollision(u16),

    #[error("Received unsolicited ack pkid: {0}")]
    Unsolicited(u16, PacketType),
}

/// Errors producable by the [`mqrstt::AsyncClient`]
#[derive(Debug, Clone, thiserror::Error)]
pub enum ClientError {
    #[error("Internal network channel is closed")]
    NoNetwork,
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