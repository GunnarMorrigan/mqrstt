use bytes::{BytesMut, Bytes};

use super::errors::PacketError;

pub trait MqttPacketRead: Sized {
    fn read(flags: u8, remaining_length: usize,  buf: &mut Bytes) -> Result<Self, PacketError>;
}

pub trait MqttPacketWrite: Sized{
    fn write(&self, buf: &mut BytesMut) -> Option<String>;
}

pub trait WireLength {
    fn wire_len(&self) -> usize;
}

pub trait MqttRead: Sized{
    fn read(buf: &mut Bytes) -> Result<Self, PacketError>;
}

pub trait MqttWrite: Sized{
    fn write(&self, buf: &mut BytesMut);
}

#[deprecated(note = "Implement MqttRead or MqttWrite.")]
pub trait SimpleSerialize: Sized {
    fn read(buf: &mut Bytes) -> Result<Self, String>;

    fn write(&self, buf: &mut BytesMut);
}

