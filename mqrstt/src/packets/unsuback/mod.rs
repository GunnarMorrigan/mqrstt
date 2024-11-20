mod properties;
pub use properties::UnsubAckProperties;

mod reason_code;
pub use reason_code::UnsubAckReasonCode;


use bytes::BufMut;

use super::error::{SerializeError};
use super::mqtt_trait::{MqttRead, MqttWrite, PacketRead, PacketWrite};

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct UnsubAck {
    pub packet_identifier: u16,
    pub properties: UnsubAckProperties,
    pub reason_codes: Vec<UnsubAckReasonCode>,
}

impl PacketRead for UnsubAck {
    fn read(_: u8, _: usize, mut buf: bytes::Bytes) -> Result<Self, super::error::DeserializeError> {
        let packet_identifier = u16::read(&mut buf)?;
        let properties = UnsubAckProperties::read(&mut buf)?;
        let mut reason_codes = vec![];
        loop {
            let reason_code = UnsubAckReasonCode::read(&mut buf)?;

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
