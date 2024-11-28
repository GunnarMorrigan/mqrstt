mod properties;
pub use properties::SubAckProperties;

mod reason_code;
pub use reason_code::SubAckReasonCode;

use super::{
    error::SerializeError,
    mqtt_trait::{MqttAsyncRead, MqttRead, MqttWrite, PacketAsyncRead, PacketRead, PacketWrite},
};
use bytes::BufMut;
use tokio::io::AsyncReadExt;

/// SubAck packet is sent by the server in response to a [`crate::packets::Subscribe`] packet.
///
/// A SUBACK packet is sent by the Server to the Client to confirm receipt and processing of a SUBSCRIBE packet.
/// A SUBACK packet contains a list of Reason Codes, that specify the maximum QoS level that was granted or the error which was found for each Subscription that was requested by the SUBSCRIBE.
#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct SubAck {
    pub packet_identifier: u16,
    pub properties: SubAckProperties,
    pub reason_codes: Vec<SubAckReasonCode>,
}

impl PacketRead for SubAck {
    fn read(_: u8, _: usize, mut buf: bytes::Bytes) -> Result<Self, super::error::DeserializeError> {
        let packet_identifier = u16::read(&mut buf)?;
        let properties = SubAckProperties::read(&mut buf)?;

        let mut reason_codes = vec![];
        loop {
            let reason_code = SubAckReasonCode::read(&mut buf)?;

            reason_codes.push(reason_code);

            if buf.is_empty() {
                break;
            }
        }

        Ok(Self {
            packet_identifier,
            properties,
            reason_codes,
        })
    }
}

impl<S> PacketAsyncRead<S> for SubAck
where
    S: tokio::io::AsyncRead + Unpin,
{
    async fn async_read(_: u8, remaining_length: usize, stream: &mut S) -> Result<(Self, usize), crate::packets::error::ReadError> {
        let mut total_read_bytes = 0;
        let packet_identifier = stream.read_u16().await?;
        let (properties, proproperties_read_bytes) = SubAckProperties::async_read(stream).await?;
        total_read_bytes += 2 + proproperties_read_bytes;
        let mut reason_codes = vec![];
        loop {
            let (reason_code, reason_code_read_bytes) = SubAckReasonCode::async_read(stream).await?;
            total_read_bytes += reason_code_read_bytes;
            reason_codes.push(reason_code);

            if remaining_length == total_read_bytes {
                break;
            }
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

impl PacketWrite for SubAck {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), SerializeError> {
        buf.put_u16(self.packet_identifier);

        self.properties.write(buf)?;
        for reason_code in &self.reason_codes {
            reason_code.write(buf)?;
        }

        Ok(())
    }
}

impl<S> crate::packets::mqtt_trait::PacketAsyncWrite<S> for SubAck
where
    S: tokio::io::AsyncWrite + Unpin,
{
    async fn async_write(&self, stream: &mut S) -> Result<usize, crate::packets::error::WriteError> {
        use crate::packets::mqtt_trait::MqttAsyncWrite;
        use tokio::io::AsyncWriteExt;
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

#[cfg(test)]
mod test {
    use bytes::BytesMut;

    use super::SubAck;
    use crate::packets::mqtt_trait::{PacketRead, PacketWrite};

    #[test]
    fn read_write_suback() {
        let buf = vec![
            0x00, 0x0F, // variable header. pkid = 15
            0x00, // Property length 0
            0x01, // Payload reason code codes Granted QoS 1,
            0x80, // Payload Unspecified error
        ];

        let data = BytesMut::from(&buf[..]);
        let sub_ack = SubAck::read(0, 0, data.clone().into()).unwrap();

        let mut result = BytesMut::new();
        sub_ack.write(&mut result).unwrap();

        assert_eq!(data.to_vec(), result.to_vec());
    }
}
