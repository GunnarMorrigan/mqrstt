mod properties;
pub use properties::UnsubAckProperties;

mod reason_code;
pub use reason_code::UnsubAckReasonCode;

use crate::packets::mqtt_trait::MqttAsyncRead;

use bytes::BufMut;

use tokio::io::AsyncReadExt;

use super::error::SerializeError;
use super::mqtt_trait::{MqttRead, MqttWrite, PacketRead, PacketWrite};
use super::{PacketAsyncRead, VariableInteger, WireLength};

/// UnsubAck packet is sent by the server in response to an [`crate::packets::Unsubscribe`] packet.
#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct UnsubAck {
    pub packet_identifier: u16,
    pub properties: UnsubAckProperties,
    pub reason_codes: Vec<UnsubAckReasonCode>,
}

impl PacketRead for UnsubAck {
    fn read(_: u8, remaining_length: usize, mut buf: bytes::Bytes) -> Result<Self, super::error::DeserializeError> {
        let packet_identifier = u16::read(&mut buf)?;
        let properties = UnsubAckProperties::read(&mut buf)?;
        let mut reason_codes = vec![];

        let mut read = 2 + properties.wire_len().variable_integer_len() + properties.wire_len();
        loop {
            if read == remaining_length {
                break;
            }

            let reason_code = UnsubAckReasonCode::read(&mut buf)?;
            reason_codes.push(reason_code);
            read += 1;
        }

        Ok(Self {
            packet_identifier,
            properties,
            reason_codes,
        })
    }
}

impl<S> PacketAsyncRead<S> for UnsubAck
where
    S: tokio::io::AsyncRead + Unpin,
{
    async fn async_read(_: u8, remaining_length: usize, stream: &mut S) -> Result<(Self, usize), crate::packets::error::ReadError> {
        let mut total_read_bytes = 2;
        let packet_identifier = stream.read_u16().await?;

        let (properties, properties_read_bytes) = UnsubAckProperties::async_read(stream).await?;
        total_read_bytes += properties_read_bytes;

        let mut reason_codes = vec![];
        loop {
            if total_read_bytes >= remaining_length {
                break;
            }

            let (reason_code, reason_code_read_bytes) = UnsubAckReasonCode::async_read(stream).await?;
            total_read_bytes += reason_code_read_bytes;

            reason_codes.push(reason_code);
        }

        Ok((
            Self {
                packet_identifier,
                properties,
                reason_codes,
            },
            total_read_bytes,
        ))
    }
}

impl PacketWrite for UnsubAck {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), SerializeError> {
        buf.put_u16(self.packet_identifier);
        self.properties.write(buf)?;
        for reason_code in &self.reason_codes {
            reason_code.write(buf)?;
        }
        Ok(())
    }
}

impl<S> crate::packets::mqtt_trait::PacketAsyncWrite<S> for UnsubAck
where
    S: tokio::io::AsyncWrite + Unpin,
{
    fn async_write(&self, stream: &mut S) -> impl std::future::Future<Output = Result<usize, crate::packets::error::WriteError>> {
        use crate::packets::mqtt_trait::MqttAsyncWrite;
        use tokio::io::AsyncWriteExt;
        async move {
            let mut total_written_bytes = 2;
            stream.write_u16(self.packet_identifier).await?;

            total_written_bytes += self.properties.async_write(stream).await?;

            for reason_code in &self.reason_codes {
                reason_code.async_write(stream).await?;
            }
            total_written_bytes += self.reason_codes.len();

            Ok(total_written_bytes)
        }
    }
}

impl WireLength for UnsubAck {
    fn wire_len(&self) -> usize {
        2 + self.properties.wire_len().variable_integer_len() + self.properties.wire_len() + self.reason_codes.len()
    }
}

#[cfg(test)]
mod tests {
    use bytes::{Bytes, BytesMut};

    use crate::packets::{
        mqtt_trait::{PacketRead, PacketWrite},
        unsuback::UnsubAck,
    };

    #[test]
    fn read_write_unsub_ack() {
        // let entire_mqtt_packet = [0xb0, 0x04, 0x35, 0xd7, 0x00, 0x00];
        let unsub_ack = [0x35, 0xd7, 0x00, 0x00];

        let mut bufmut = BytesMut::new();
        bufmut.extend(&unsub_ack[..]);

        let buf: Bytes = bufmut.into();

        let s = UnsubAck::read(0xb0, 4, buf.clone()).unwrap();

        let mut result = BytesMut::new();
        s.write(&mut result).unwrap();

        assert_eq!(buf.to_vec(), result.to_vec());
    }
}
