mod primitive_impl;

use std::future::Future;

use bytes::{Bytes, BytesMut};

pub(crate) trait PacketRead: Sized {
    fn read(flags: u8, remaining_length: usize, buf: Bytes) -> Result<Self, crate::packets::error::DeserializeError>;
}

pub(crate) trait PacketAsyncRead<S>: Sized
where
    S: tokio::io::AsyncRead + Unpin,
{
    fn async_read(flags: u8, remaining_length: usize, stream: &mut S) -> impl Future<Output = Result<(Self, usize), crate::packets::error::ReadError>>;
}

pub(crate) trait PacketAsyncWrite<S>: Sized
where
    S: tokio::io::AsyncWriteExt + Unpin,
{
    fn async_write(&self, stream: &mut S) -> impl Future<Output = Result<usize, crate::packets::error::WriteError>>;
}

pub(crate) trait PacketWrite: Sized {
    fn write(&self, buf: &mut BytesMut) -> Result<(), crate::packets::error::SerializeError>;
}

pub(crate) trait WireLength {
    fn wire_len(&self) -> usize;
}

pub(crate) trait MqttRead: Sized {
    fn read(buf: &mut Bytes) -> Result<Self, crate::packets::error::DeserializeError>;
}
pub(crate) trait MqttAsyncRead<S>: Sized {
    /// Reads `Self` from the provided stream.
    /// Returns the deserialized instance and the number of bytes read from the stream.
    fn async_read(stream: &mut S) -> impl Future<Output = Result<(Self, usize), crate::packets::error::ReadError>>;
}

pub trait MqttWrite: Sized {
    fn write(&self, buf: &mut BytesMut) -> Result<(), crate::packets::error::SerializeError>;
}

impl<T> MqttWrite for &T
where
    T: MqttWrite,
{
    fn write(&self, buf: &mut BytesMut) -> Result<(), crate::packets::error::SerializeError> {
        <T>::write(self, buf)
    }
}
pub(crate) trait MqttAsyncWrite<S>: Sized {
    /// Write `Self` to the provided stream.
    /// Returns the deserialized instance and the number of bytes read from the stream.
    fn async_write(&self, stream: &mut S) -> impl Future<Output = Result<usize, crate::packets::error::WriteError>>;
}

pub trait PacketValidation: Sized {
    fn validate(&self, max_packet_size: usize) -> Result<(), crate::error::PacketValidationError>;
}
