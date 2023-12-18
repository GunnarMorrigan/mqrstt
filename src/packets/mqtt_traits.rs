use bytes::{Bytes, BytesMut};

use crate::mqtt_async_traits::AsyncMqttRead;

use super::error::{DeserializeError, SerializeError};

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
    async fn read<S: AsyncMqttRead>(stream: &mut S) -> Result<Self, DeserializeError>;
}

pub trait MqttWrite: Sized {
    fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError>;
}

pub trait PacketValidation: Sized {
    fn validate(&self, max_packet_size: usize) -> Result<(), crate::error::PacketValidationError>;
}
