use tokio::io::AsyncWriteExt;

use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::packets::error::{DeserializeError, ReadError, SerializeError};
use crate::packets::mqtt_trait::{MqttAsyncRead, MqttRead, MqttWrite, WireLength};

use super::MqttAsyncWrite;

impl MqttRead for Box<str> {
    #[inline]
    fn read(buf: &mut Bytes) -> Result<Self, DeserializeError> {
        let content = Vec::<u8>::read(buf)?;

        match String::from_utf8(content) {
            Ok(s) => Ok(s.into_boxed_str()),
            Err(e) => Err(DeserializeError::Utf8Error(e)),
        }
    }
}

impl<S> MqttAsyncRead<S> for Box<str>
where
    S: tokio::io::AsyncRead + std::marker::Unpin,
{
    async fn async_read(stream: &mut S) -> Result<(Self, usize), ReadError> {
        let (content, read_bytes) = Vec::async_read(stream).await?;
        match String::from_utf8(content) {
            Ok(s) => Ok((s.into(), read_bytes)),
            Err(e) => Err(ReadError::DeserializeError(DeserializeError::Utf8Error(e))),
        }
    }
}

impl MqttWrite for Box<str> {
    #[inline(always)]
    fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError> {
        self.as_ref().write(buf)
    }
}

impl<S> MqttAsyncWrite<S> for Box<str>
where
    S: tokio::io::AsyncWrite + Unpin,
{
    async fn async_write(&self, stream: &mut S) -> Result<usize, crate::packets::error::WriteError> {
        let size = (self.len() as u16).to_be_bytes();
        stream.write_all(&size).await?;
        stream.write_all(self.as_bytes()).await?;
        Ok(2 + self.len())
    }
}

impl WireLength for Box<str> {
    #[inline(always)]
    fn wire_len(&self) -> usize {
        self.as_ref().wire_len()
    }
}

impl MqttWrite for &str {
    #[inline]
    fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError> {
        buf.put_u16(self.len() as u16);
        buf.extend(self.as_bytes());
        Ok(())
    }
}

impl<S> MqttAsyncWrite<S> for &str
where
    S: tokio::io::AsyncWrite + Unpin,
{
    async fn async_write(&self, stream: &mut S) -> Result<usize, crate::packets::error::WriteError> {
        let size = (self.len() as u16).to_be_bytes();
        stream.write_all(&size).await?;
        stream.write_all(self.as_bytes()).await?;
        Ok(2 + self.len())
    }
}

impl WireLength for &str {
    #[inline(always)]
    fn wire_len(&self) -> usize {
        self.len() + 2
    }
}

// impl MqttRead for Bytes {
//     #[inline]
//     fn read(buf: &mut Bytes) -> Result<Self, DeserializeError> {
//         if buf.len() < 2 {
//             return Err(DeserializeError::InsufficientData(std::any::type_name::<Bytes>(), buf.len(), 2));
//         }
//         let len = buf.get_u16() as usize;

//         if len > buf.len() {
//             return Err(DeserializeError::InsufficientData(std::any::type_name::<Bytes>(), buf.len(), len));
//         }

//         Ok(buf.split_to(len))
//     }
// }
// impl<S> MqttAsyncRead<S> for Bytes
// where
//     S: tokio::io::AsyncReadExt + std::marker::Unpin,
// {
//     async fn async_read(stream: &mut S) -> Result<(Self, usize), ReadError> {
//         let size = stream.read_u16().await? as usize;
//         // let mut data = BytesMut::with_capacity(size);
//         let mut data = Vec::with_capacity(size);
//         let read_bytes = stream.read_exact(&mut data).await?;
//         assert_eq!(size, read_bytes);
//         Ok((data.into(), 2 + size))
//     }
// }
// impl MqttWrite for Bytes {
//     #[inline]
//     fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError> {
//         buf.put_u16(self.len() as u16);
//         buf.extend(self);

//         Ok(())
//     }
// }
// impl<S> MqttAsyncWrite<S> for Bytes
// where
//     S: tokio::io::AsyncWrite + Unpin,
// {
//     async fn async_write(&self, stream: &mut S) -> Result<usize, crate::packets::error::WriteError> {
//         let size = (self.len() as u16).to_be_bytes();
//         stream.write_all(&size).await?;
//         stream.write_all(self.as_ref()).await?;
//         Ok(2 + self.len())
//     }
// }

// impl WireLength for Bytes {
//     #[inline(always)]
//     fn wire_len(&self) -> usize {
//         self.len() + 2
//     }
// }

impl MqttRead for Vec<u8> {
    #[inline]
    fn read(buf: &mut Bytes) -> Result<Self, DeserializeError> {
        if buf.len() < 2 {
            return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), buf.len(), 2));
        }
        let len = buf.get_u16() as usize;

        if len > buf.len() {
            return Err(DeserializeError::InsufficientData(std::any::type_name::<Bytes>(), buf.len(), len));
        }

        Ok(buf.split_to(len).into())
    }
}
impl MqttWrite for Vec<u8> {
    #[inline]
    fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError> {
        buf.put_u16(self.len() as u16);
        buf.extend(self);

        Ok(())
    }
}
impl<S> MqttAsyncWrite<S> for Vec<u8>
where
    S: tokio::io::AsyncWrite + Unpin,
{
    async fn async_write(&self, stream: &mut S) -> Result<usize, crate::packets::error::WriteError> {
        let size = (self.len() as u16).to_be_bytes();
        stream.write_all(&size).await?;
        stream.write_all(self).await?;
        Ok(2 + self.len())
    }
}
impl WireLength for Vec<u8> {
    #[inline(always)]
    fn wire_len(&self) -> usize {
        self.len() + 2
    }
}
impl<S> MqttAsyncRead<S> for Vec<u8>
where
    S: tokio::io::AsyncReadExt + std::marker::Unpin,
{
    async fn async_read(stream: &mut S) -> Result<(Self, usize), ReadError> {
        let size = stream.read_u16().await? as usize;
        // let mut data = BytesMut::with_capacity(size);
        let mut data = vec![0u8; size];
        let read_bytes = stream.read_exact(&mut data).await?;
        assert_eq!(size, read_bytes);
        Ok((data, 2 + size))
    }
}

impl MqttRead for bool {
    fn read(buf: &mut Bytes) -> Result<Self, crate::packets::error::DeserializeError> {
        if buf.is_empty() {
            return Err(DeserializeError::InsufficientData(std::any::type_name::<bool>(), 0, 1));
        }

        match buf.get_u8() {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(crate::packets::error::DeserializeError::MalformedPacket),
        }
    }
}
impl<T> MqttAsyncRead<T> for bool
where
    T: tokio::io::AsyncReadExt + std::marker::Unpin,
{
    async fn async_read(buf: &mut T) -> Result<(Self, usize), ReadError> {
        match buf.read_u8().await? {
            0 => Ok((false, 1)),
            1 => Ok((true, 1)),
            _ => Err(ReadError::DeserializeError(DeserializeError::MalformedPacket)),
        }
    }
}
impl MqttWrite for bool {
    #[inline]
    fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError> {
        if *self {
            buf.put_u8(1);
            Ok(())
        } else {
            buf.put_u8(0);
            Ok(())
        }
    }
}
impl<S> MqttAsyncWrite<S> for bool
where
    S: tokio::io::AsyncWrite + Unpin,
{
    async fn async_write(&self, stream: &mut S) -> Result<usize, crate::packets::error::WriteError> {
        if *self {
            stream.write_all(&[1]).await?;
        } else {
            stream.write_all(&[0]).await?;
        }
        Ok(1)
    }
}
impl MqttRead for u8 {
    #[inline]
    fn read(buf: &mut Bytes) -> Result<Self, DeserializeError> {
        if buf.is_empty() {
            return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), 0, 1));
        }
        Ok(buf.get_u8())
    }
}
impl<T> MqttAsyncRead<T> for u8
where
    T: tokio::io::AsyncReadExt + std::marker::Unpin,
{
    async fn async_read(buf: &mut T) -> Result<(Self, usize), ReadError> {
        Ok((buf.read_u8().await?, 1))
    }
}
impl<S> MqttAsyncWrite<S> for u8
where
    S: tokio::io::AsyncWrite + Unpin,
{
    async fn async_write(&self, stream: &mut S) -> Result<usize, crate::packets::error::WriteError> {
        stream.write_all(self.to_be_bytes().as_slice()).await?;
        Ok(1)
    }
}

impl MqttRead for u16 {
    #[inline]
    fn read(buf: &mut Bytes) -> Result<Self, DeserializeError> {
        if buf.len() < 2 {
            return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), buf.len(), 2));
        }
        Ok(buf.get_u16())
    }
}
impl<T> MqttAsyncRead<T> for u16
where
    T: tokio::io::AsyncReadExt + std::marker::Unpin,
{
    async fn async_read(buf: &mut T) -> Result<(Self, usize), ReadError> {
        Ok((buf.read_u16().await?, 2))
    }
}
impl MqttWrite for u16 {
    #[inline]
    fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError> {
        buf.put_u16(*self);
        Ok(())
    }
}
impl<S> MqttAsyncWrite<S> for u16
where
    S: tokio::io::AsyncWrite + Unpin,
{
    async fn async_write(&self, stream: &mut S) -> Result<usize, crate::packets::error::WriteError> {
        stream.write_all(self.to_be_bytes().as_slice()).await?;
        Ok(2)
    }
}

impl MqttRead for u32 {
    #[inline]
    fn read(buf: &mut Bytes) -> Result<Self, DeserializeError> {
        if buf.len() < 4 {
            return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), buf.len(), 4));
        }
        Ok(buf.get_u32())
    }
}
impl<T> MqttAsyncRead<T> for u32
where
    T: tokio::io::AsyncReadExt + std::marker::Unpin,
{
    async fn async_read(buf: &mut T) -> Result<(Self, usize), ReadError> {
        Ok((buf.read_u32().await?, 4))
    }
}
impl MqttWrite for u32 {
    fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError> {
        buf.put_u32(*self);
        Ok(())
    }
}
impl<S> MqttAsyncWrite<S> for u32
where
    S: tokio::io::AsyncWrite + Unpin,
{
    async fn async_write(&self, stream: &mut S) -> Result<usize, crate::packets::error::WriteError> {
        stream.write_all(self.to_be_bytes().as_slice()).await?;
        Ok(4)
    }
}
