use std::{future::Future, process::Output};

use bytes::{Bytes, BytesMut};

use super::error::{DeserializeError, ReadError, SerializeError};

pub trait PacketRead: Sized {
    fn read(flags: u8, remaining_length: usize, buf: Bytes) -> Result<Self, DeserializeError>;
}

pub trait PacketAsyncRead<S>: Sized where S: tokio::io::AsyncReadExt + Unpin {
    fn async_read(flags: u8, remaining_length: usize, stream: &mut S) -> impl Future<Output = Result<(Self, usize), ReadError>>;
}

pub trait PacketWrite: Sized {
    fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError>;
}

pub trait WireLength {
    fn wire_len(&self) -> usize;
}

pub trait MqttRead: Sized {
    fn read(buf: &mut Bytes) -> Result<Self, DeserializeError>;
}
pub trait MqttAsyncRead<S>: Sized 
// where S: tokio::io::AsyncReadExt + Unpin 
{
    /// Reads `Self` from the provided stream.
    /// Returns the deserialized instance and the number of bytes read from the stream.
    fn async_read(stream: &mut S) -> impl Future<Output = Result<(Self, usize), ReadError>>;
}


pub trait MqttWrite: Sized {
    fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError>;
}

impl<T> MqttWrite for &T
where
    T: MqttWrite,
{
    fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError> {
        <T>::write(self, buf)
    }
}

pub trait PacketValidation: Sized {
    fn validate(&self, max_packet_size: usize) -> Result<(), crate::error::PacketValidationError>;
}
