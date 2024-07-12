use bytes::{Bytes, BytesMut};

use super::error::{DeserializeError, ReadError, SerializeError};

pub trait VariableHeaderRead: Sized {
    fn read(flags: u8, remaining_length: usize, buf: Bytes) -> Result<Self, DeserializeError>;
}

pub trait VariableHeaderWrite: Sized {
    fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError>;
}

pub trait WireLength {
    fn wire_len(&self) -> usize;
}

pub trait MqttRead: Sized {
    fn read(buf: &mut Bytes) -> Result<Self, DeserializeError>;
}
pub trait MqttAsyncRead<T>: Sized where T: tokio::io::AsyncReadExt {
    async fn async_read(buf: &mut T) -> Result<Self, ReadError>;
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
