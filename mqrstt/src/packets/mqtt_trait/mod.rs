mod primitive_impl;
pub use primitive_impl::*;

use std::{future::Future};

use bytes::{Bytes, BytesMut};

// mod sealed {
//     /// Sealed trait to prevent downstream users from implementing the 
//     /// [`crate::packets::mqtt_trait::PacketRead`], [`crate::packets::mqtt_trait::PacketWrite`], 
//     /// [`crate::packets::mqtt_trait::PacketAsyncRead`] [`crate::packets::mqtt_trait::PacketAsyncWrite`], 
//     /// [`crate::packets::mqtt_trait::MqttRead`], [`crate::packets::mqtt_trait::MqttWrite`] 
//     /// and [`crate::packets::mqtt_trait::WireLength`]  traits.
//     pub trait Sealed {}
//     impl Sealed for crate::packets::ConnAck {}

// }

// pub(crate) trait PacketRead: Sized + sealed::Sealed {
//     fn read(flags: u8, remaining_length: usize, buf: Bytes) -> Result<Self, crate::packets::error::DeserializeError>;
// }

// pub(crate) trait PacketAsyncRead<S>: Sized + sealed::Sealed where S: tokio::io::AsyncReadExt + Unpin {
//     fn async_read(flags: u8, remaining_length: usize, stream: &mut S) -> impl Future<Output = Result<(Self, usize), crate::packets::error::ReadError>>;
// }

// pub(crate) trait PacketWrite: Sized + sealed::Sealed {
//     fn write(&self, buf: &mut BytesMut) -> Result<(), crate::packets::error::SerializeError>;
// }

// pub(crate) trait WireLength: sealed::Sealed {
//     fn wire_len(&self) -> usize;
// }

// pub(crate) trait MqttRead: Sized + sealed::Sealed {
//     fn read(buf: &mut Bytes) -> Result<Self, crate::packets::error::DeserializeError>;
// }
// pub trait MqttAsyncRead<S>: Sized + sealed::Sealed
// {
//     /// Reads `Self` from the provided stream.
//     /// Returns the deserialized instance and the number of bytes read from the stream.
//     fn async_read(stream: &mut S) -> impl Future<Output = Result<(Self, usize), crate::packets::error::ReadError>>;
// }


// pub trait MqttWrite: Sized + sealed::Sealed {
//     fn write(&self, buf: &mut BytesMut) -> Result<(), crate::packets::error::SerializeError>;
// }

// impl<'a, T> MqttWrite for &'a T
// where
//     T: MqttWrite,
//     &'a T: sealed::Sealed,
// {
//     fn write(&self, buf: &mut BytesMut) -> Result<(), crate::packets::error::SerializeError> {
//         <T>::write(self, buf)
//     }
// }

// pub trait PacketValidation: Sized + sealed::Sealed {
//     fn validate(&self, max_packet_size: usize) -> Result<(), crate::error::PacketValidationError>;
// }

pub(crate) trait PacketRead: Sized  {
    fn read(flags: u8, remaining_length: usize, buf: Bytes) -> Result<Self, crate::packets::error::DeserializeError>;
}

pub(crate) trait PacketAsyncRead<S>: Sized  where S: tokio::io::AsyncReadExt + Unpin {
    fn async_read(flags: u8, remaining_length: usize, stream: &mut S) -> impl Future<Output = Result<(Self, usize), crate::packets::error::ReadError>>;
}

pub(crate) trait PacketWrite: Sized  {
    fn write(&self, buf: &mut BytesMut) -> Result<(), crate::packets::error::SerializeError>;
}

pub(crate) trait WireLength {
    fn wire_len(&self) -> usize;
}

pub(crate) trait MqttRead: Sized  {
    fn read(buf: &mut Bytes) -> Result<Self, crate::packets::error::DeserializeError>;
}
pub(crate) trait MqttAsyncRead<S>: Sized 
{
    /// Reads `Self` from the provided stream.
    /// Returns the deserialized instance and the number of bytes read from the stream.
    fn async_read(stream: &mut S) -> impl Future<Output = Result<(Self, usize), crate::packets::error::ReadError>>;
}


pub trait MqttWrite: Sized {
    fn write(&self, buf: &mut BytesMut) -> Result<(), crate::packets::error::SerializeError>;
}

impl<'a, T> MqttWrite for &'a T
where
    T: MqttWrite,
{
    fn write(&self, buf: &mut BytesMut) -> Result<(), crate::packets::error::SerializeError> {
        <T>::write(self, buf)
    }
}

pub trait PacketValidation: Sized {
    fn validate(&self, max_packet_size: usize) -> Result<(), crate::error::PacketValidationError>;
}
