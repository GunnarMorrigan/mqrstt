use bytes::{Bytes, BytesMut};

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
	fn read(buf: &mut Bytes) -> Result<Self, DeserializeError>;
}

pub trait MqttWrite: Sized {
	fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError>;
}

#[deprecated(note = "Implement MqttRead or MqttWrite.")]
pub trait SimpleSerialize: Sized {
	fn read(buf: &mut Bytes) -> Result<Self, String>;

	fn write(&self, buf: &mut BytesMut);
}
