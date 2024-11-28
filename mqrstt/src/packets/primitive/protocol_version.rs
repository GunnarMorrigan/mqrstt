use bytes::{Buf, BufMut, Bytes, BytesMut};

use tokio::io::AsyncReadExt;

use crate::packets::{
    error::{DeserializeError, ReadError, SerializeError},
    mqtt_trait::{MqttAsyncRead, MqttAsyncWrite, MqttRead, MqttWrite},
};

/// Protocol version of the MQTT connection
///
/// This client only supports MQTT v5.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum ProtocolVersion {
    V5,
}

impl MqttWrite for ProtocolVersion {
    fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError> {
        buf.put_u8(5u8);
        Ok(())
    }
}

impl<S> MqttAsyncWrite<S> for ProtocolVersion
where
    S: tokio::io::AsyncWrite + Unpin,
{
    fn async_write(&self, stream: &mut S) -> impl std::future::Future<Output = Result<usize, crate::packets::error::WriteError>> {
        use tokio::io::AsyncWriteExt;
        async move {
            stream.write_u8(5).await?;
            Ok(1)
        }
    }
}

impl MqttRead for ProtocolVersion {
    fn read(buf: &mut Bytes) -> Result<Self, DeserializeError> {
        if buf.is_empty() {
            return Err(DeserializeError::InsufficientDataForProtocolVersion);
        }

        match buf.get_u8() {
            3 => Err(DeserializeError::UnsupportedProtocolVersion),
            4 => Err(DeserializeError::UnsupportedProtocolVersion),
            5 => Ok(ProtocolVersion::V5),
            _ => Err(DeserializeError::UnknownProtocolVersion),
        }
    }
}

impl<S> MqttAsyncRead<S> for ProtocolVersion
where
    S: tokio::io::AsyncRead + std::marker::Unpin,
{
    async fn async_read(stream: &mut S) -> Result<(Self, usize), ReadError> {
        match stream.read_u8().await {
            Ok(5) => Ok((ProtocolVersion::V5, 1)),
            Ok(4) => Err(ReadError::DeserializeError(DeserializeError::UnsupportedProtocolVersion)),
            Ok(3) => Err(ReadError::DeserializeError(DeserializeError::UnsupportedProtocolVersion)),
            Ok(_) => Err(ReadError::DeserializeError(DeserializeError::UnknownProtocolVersion)),
            Err(e) => Err(ReadError::IoError(e)),
        }
    }
}
