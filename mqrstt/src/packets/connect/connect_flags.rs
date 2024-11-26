use bytes::{Buf, BufMut};

use tokio::io::AsyncReadExt;

use crate::packets::{
    error::{DeserializeError, SerializeError},
    mqtt_trait::{MqttAsyncRead, MqttAsyncWrite, MqttRead, MqttWrite},
    QoS,
};

/// The connect flags describe some information related the session.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ConnectFlags {
    /// Indicates whether to start a new session or continue an existing one.
    pub clean_start: bool,
    /// Specifies if a Will message is included.
    pub will_flag: bool,
    /// Defines the Quality of Service level for the Will message.
    pub will_qos: QoS,
    /// Indicates if the Will message should be retained by the broker.
    pub will_retain: bool,
    /// Shows if a password is included in the payload.
    pub password: bool,
    /// Shows if a username is included in the payload.
    pub username: bool,
}

impl ConnectFlags {
    pub fn from_u8(value: u8) -> Result<Self, DeserializeError> {
        Ok(Self {
            clean_start: ((value & 0b00000010) >> 1) != 0,
            will_flag: ((value & 0b00000100) >> 2) != 0,
            will_qos: QoS::from_u8((value & 0b00011000) >> 3)?,
            will_retain: ((value & 0b00100000) >> 5) != 0,
            password: ((value & 0b01000000) >> 6) != 0,
            username: ((value & 0b10000000) >> 7) != 0,
        })
    }

    pub fn into_u8(&self) -> Result<u8, SerializeError> {
        let byte = ((self.clean_start as u8) << 1)
            | ((self.will_flag as u8) << 2)
            | (self.will_qos.into_u8() << 3)
            | ((self.will_retain as u8) << 5)
            | ((self.password as u8) << 6)
            | ((self.username as u8) << 7);
        Ok(byte)
    }
}

impl Default for ConnectFlags {
    fn default() -> Self {
        Self {
            clean_start: false,
            will_flag: false,
            will_qos: QoS::AtMostOnce,
            will_retain: false,
            password: false,
            username: false,
        }
    }
}

impl MqttRead for ConnectFlags {
    fn read(buf: &mut bytes::Bytes) -> Result<Self, DeserializeError> {
        if buf.is_empty() {
            return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), 0, 1));
        }

        let byte = buf.get_u8();

        ConnectFlags::from_u8(byte)
    }
}

impl<S> MqttAsyncRead<S> for ConnectFlags
where
    S: tokio::io::AsyncRead + Unpin,
{
    fn async_read(stream: &mut S) -> impl std::future::Future<Output = Result<(Self, usize), crate::packets::error::ReadError>> {
        async move {
            let byte = stream.read_u8().await?;
            Ok((ConnectFlags::from_u8(byte)?, 1))
        }
    }
}

impl MqttWrite for ConnectFlags {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::packets::error::SerializeError> {
        buf.put_u8(self.into_u8()?);
        Ok(())
    }
}

impl<S> MqttAsyncWrite<S> for ConnectFlags
where
    S: tokio::io::AsyncWrite + Unpin,
{
    fn async_write(&self, stream: &mut S) -> impl std::future::Future<Output = Result<usize, crate::packets::error::WriteError>> {
        async move {
            use tokio::io::AsyncWriteExt;
            let byte = self.into_u8()?;
            stream.write_u8(byte).await?;

            Ok(1)
        }
    }
}
