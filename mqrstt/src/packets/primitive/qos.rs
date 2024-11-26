use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::packets::{
    error::{DeserializeError, ReadError, SerializeError},
    mqtt_trait::{MqttAsyncRead, MqttAsyncWrite, MqttRead, MqttWrite},
};

use tokio::io::AsyncWriteExt;

/// Quality of service
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum QoS {
    #[default]
    AtMostOnce = 0,
    AtLeastOnce = 1,
    ExactlyOnce = 2,
}
impl QoS {
    pub fn from_u8(value: u8) -> Result<Self, DeserializeError> {
        match value {
            0 => Ok(QoS::AtMostOnce),
            1 => Ok(QoS::AtLeastOnce),
            2 => Ok(QoS::ExactlyOnce),
            _ => Err(DeserializeError::UnknownQoS(value)),
        }
    }
    pub fn into_u8(self) -> u8 {
        match self {
            QoS::AtMostOnce => 0,
            QoS::AtLeastOnce => 1,
            QoS::ExactlyOnce => 2,
        }
    }
}

impl MqttRead for QoS {
    #[inline]
    fn read(buf: &mut Bytes) -> Result<Self, DeserializeError> {
        if buf.is_empty() {
            return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), 0, 1));
        }

        match buf.get_u8() {
            0 => Ok(QoS::AtMostOnce),
            1 => Ok(QoS::AtLeastOnce),
            2 => Ok(QoS::ExactlyOnce),
            q => Err(DeserializeError::UnknownQoS(q)),
        }
    }
}

impl<T> MqttAsyncRead<T> for QoS
where
    T: tokio::io::AsyncReadExt + std::marker::Unpin,
{
    async fn async_read(buf: &mut T) -> Result<(Self, usize), ReadError> {
        match buf.read_u8().await {
            Ok(0) => Ok((QoS::AtMostOnce, 1)),
            Ok(1) => Ok((QoS::AtLeastOnce, 1)),
            Ok(2) => Ok((QoS::ExactlyOnce, 1)),
            Ok(q) => Err(ReadError::DeserializeError(DeserializeError::UnknownQoS(q))),
            Err(e) => Err(ReadError::IoError(e)),
        }
    }
}

impl MqttWrite for QoS {
    #[inline]
    fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError> {
        let val = self.into_u8();
        buf.put_u8(val);
        Ok(())
    }
}
impl<S> MqttAsyncWrite<S> for QoS
where
    S: tokio::io::AsyncWrite + std::marker::Unpin,
{
    fn async_write(&self, stream: &mut S) -> impl std::future::Future<Output = Result<usize, crate::packets::error::WriteError>> {
        async move {
            let buf: [u8; 1] = [self.into_u8()];
            stream.write_all(&buf).await?;
            Ok(1)
        }
    }
}
