mod properties;

pub use properties::AuthProperties;
mod reason_code;
pub use reason_code::AuthReasonCode;

use bytes::Bytes;

use super::{
    mqtt_trait::{MqttAsyncRead, MqttAsyncWrite, MqttRead, MqttWrite, PacketAsyncRead, PacketRead, PacketWrite, WireLength},
    VariableInteger,
};

#[derive(Debug, Clone, PartialEq, Eq)]
/// The AUTH packet is used to perform more intriquite authentication methods.
///
/// At the time of writing this client does not (yet) provide the user a method of handling the auth handshake.
/// There are several other ways to perform authentication, for example using TLS.
/// Additionally, not many clients support this packet fully.
pub struct Auth {
    pub reason_code: AuthReasonCode,
    pub properties: AuthProperties,
}

impl PacketRead for Auth {
    fn read(_: u8, _: usize, mut buf: Bytes) -> Result<Self, super::error::DeserializeError> {
        let reason_code = AuthReasonCode::read(&mut buf)?;
        let properties = AuthProperties::read(&mut buf)?;

        Ok(Self { reason_code, properties })
    }
}

impl<S> PacketAsyncRead<S> for Auth
where
    S: tokio::io::AsyncRead + Unpin,
{
    async fn async_read(_: u8, _: usize, stream: &mut S) -> Result<(Self, usize), crate::packets::error::ReadError> {
        let (reason_code, reason_code_read_bytes) = AuthReasonCode::async_read(stream).await?;
        let (properties, properties_read_bytes) = AuthProperties::async_read(stream).await?;

        Ok((Self { reason_code, properties }, reason_code_read_bytes + properties_read_bytes))
    }
}

impl<S> crate::packets::mqtt_trait::PacketAsyncWrite<S> for Auth
where
    S: tokio::io::AsyncWrite + Unpin,
{
    async fn async_write(&self, stream: &mut S) -> Result<usize, crate::packets::error::WriteError> {
        let reason_code_written = self.reason_code.async_write(stream).await?;
        let properties_written = self.properties.async_write(stream).await?;
        Ok(reason_code_written + properties_written)
    }
}

impl PacketWrite for Auth {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
        self.reason_code.write(buf)?;
        self.properties.write(buf)?;
        Ok(())
    }
}

impl WireLength for Auth {
    fn wire_len(&self) -> usize {
        1 + self.properties.wire_len().variable_integer_len() + self.properties.wire_len()
    }
}
