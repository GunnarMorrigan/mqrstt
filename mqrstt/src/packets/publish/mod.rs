mod properties;
pub use properties::PublishProperties;

use tokio::io::AsyncReadExt;

use bytes::BufMut;

use crate::error::PacketValidationError;
use crate::util::constants::MAXIMUM_TOPIC_SIZE;

use super::mqtt_trait::{MqttAsyncRead, MqttRead, MqttWrite, PacketAsyncRead, PacketRead, PacketValidation, PacketWrite, WireLength};
use super::VariableInteger;
use super::{
    error::{DeserializeError, SerializeError},
    QoS,
};

/// The PUBLISH Packet is used to send data from either side of the connection.
/// This packet is handed to the [`crate::AsyncEventHandler`] to be handled by the user.
///
/// The following flow is determined by the QoS level used in PUBLISH Packet.
/// QoS 0: Send and forget, no deliviery garantee.
/// QoS 1: Send and acknowledgement, uised to ensure that the packet is delivered at least once.
/// QoS 2: Send and 2-step acknowledgement, used to ensure that the packet is delivered only once.
/// The packet can be send using for example [`crate::MqttClient::publish`] or [`crate::MqttClient::publish_with_properties`].
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Publish {
    /// 3.3.1.1 dup
    pub dup: bool,
    /// 3.3.1.2 QoS
    pub qos: QoS,
    /// 3.3.1.3 retain
    pub retain: bool,

    /// 3.3.2.1 Topic Name
    /// The Topic Name identifies the information channel to which Payload data is published.
    pub topic: Box<str>,

    /// 3.3.2.2 Packet Identifier
    /// The Packet Identifier field is only present in PUBLISH packets where the QoS level is 1 or 2. Section 2.2.1 provides more information about Packet Identifiers.
    pub packet_identifier: Option<u16>,

    /// 3.3.2.3 PUBLISH Properties
    pub publish_properties: PublishProperties,

    /// 3.3.3 PUBLISH Payload
    pub payload: Vec<u8>,
}

impl Publish {
    pub fn new<S: AsRef<str>, P: Into<Vec<u8>>>(qos: QoS, retain: bool, topic: S, packet_identifier: Option<u16>, publish_properties: PublishProperties, payload: P) -> Self {
        Self {
            dup: false,
            qos,
            retain,
            topic: topic.as_ref().into(),
            packet_identifier,
            publish_properties,
            payload: payload.into(),
        }
    }

    pub fn payload(&self) -> &Vec<u8> {
        &self.payload
    }
}

impl PacketRead for Publish {
    fn read(flags: u8, _: usize, mut buf: bytes::Bytes) -> Result<Self, DeserializeError> {
        let dup = flags & 0b1000 != 0;
        let qos = QoS::from_u8((flags & 0b110) >> 1)?;
        let retain = flags & 0b1 != 0;

        let topic = Box::<str>::read(&mut buf)?;
        let mut packet_identifier = None;
        if qos != QoS::AtMostOnce {
            packet_identifier = Some(u16::read(&mut buf)?);
        }

        let publish_properties = PublishProperties::read(&mut buf)?;

        Ok(Self {
            dup,
            qos,
            retain,
            topic,
            packet_identifier,
            publish_properties,
            payload: buf.to_vec(),
        })
    }
}

impl<S> PacketAsyncRead<S> for Publish
where
    S: tokio::io::AsyncRead + Unpin,
{
    fn async_read(flags: u8, remaining_length: usize, stream: &mut S) -> impl std::future::Future<Output = Result<(Self, usize), crate::packets::error::ReadError>> {
        async move {
            let mut total_read_bytes = 0;
            let dup = flags & 0b1000 != 0;
            let qos = QoS::from_u8((flags & 0b110) >> 1)?;
            let retain = flags & 0b1 != 0;

            let (topic, topic_read_bytes) = Box::<str>::async_read(stream).await?;
            total_read_bytes += topic_read_bytes;
            let packet_identifier = if qos == QoS::AtMostOnce {
                None
            } else {
                total_read_bytes += 2;
                Some(stream.read_u16().await?)
            };
            let (publish_properties, properties_read_bytes) = PublishProperties::async_read(stream).await?;
            total_read_bytes += properties_read_bytes;

            let payload_len = remaining_length - total_read_bytes;
            let mut payload = vec![0u8; payload_len];
            let payload_read_bytes = stream.read_exact(&mut payload).await?;

            assert_eq!(payload_read_bytes, payload_len);

            Ok((
                Self {
                    dup,
                    qos,
                    retain,
                    topic,
                    packet_identifier,
                    publish_properties,
                    payload,
                },
                total_read_bytes + payload_read_bytes,
            ))
        }
    }
}

impl PacketWrite for Publish {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), SerializeError> {
        self.topic.write(buf)?;

        if let Some(pkid) = self.packet_identifier {
            buf.put_u16(pkid);
        }

        self.publish_properties.write(buf)?;

        buf.extend(&self.payload);

        Ok(())
    }
}

impl WireLength for Publish {
    fn wire_len(&self) -> usize {
        let mut len = self.topic.wire_len();
        if self.packet_identifier.is_some() {
            len += 2;
        }

        let properties_len = self.publish_properties.wire_len();

        len += properties_len.variable_integer_len();
        len += properties_len;
        len += self.payload.len();
        len
    }
}

impl PacketValidation for Publish {
    fn validate(&self, max_packet_size: usize) -> Result<(), PacketValidationError> {
        use PacketValidationError::*;
        if self.wire_len() > max_packet_size {
            Err(MaxPacketSize(self.wire_len()))
        } else if self.topic.len() > MAXIMUM_TOPIC_SIZE {
            Err(TopicSize(self.topic.len()))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::{BufMut, BytesMut};

    use crate::packets::{
        mqtt_trait::{PacketRead, PacketWrite},
        VariableInteger,
    };

    use super::Publish;

    #[test]
    fn test_read_write_properties() {
        let first_byte = 0b0011_0100;

        let mut properties = [1, 0, 2].to_vec();
        properties.extend(4_294_967_295u32.to_be_bytes());
        properties.push(35);
        properties.extend(3456u16.to_be_bytes());
        properties.push(8);
        let resp_topic = "hellogoodbye";
        properties.extend((resp_topic.len() as u16).to_be_bytes());
        properties.extend(resp_topic.as_bytes());

        let mut buf_one = BytesMut::from(
            &[
                0x00, 0x03, b'a', b'/', b'b', // variable header. topic name = 'a/b'
            ][..],
        );
        buf_one.put_u16(10);
        properties.len().write_variable_integer(&mut buf_one).unwrap();
        buf_one.extend(properties);
        buf_one.extend(
            [
                0x01, // Payload
                0x02, 0xDE, 0xAD, 0xBE,
            ]
            .to_vec(),
        );

        let rem_len = buf_one.len();

        let buf = buf_one.clone();

        let p = Publish::read(first_byte & 0b0000_1111, rem_len, buf.into()).unwrap();

        let mut result_buf = BytesMut::with_capacity(1000);
        p.write(&mut result_buf).unwrap();

        assert_eq!(buf_one.to_vec(), result_buf.to_vec())
    }

    #[test]
    fn test_read_write() {
        let first_byte = 0b0011_0000;
        let buf_one = &[
            0x00, 0x03, b'a', b'/', b'b', // variable header. topic name = 'a/b'
            0x00, 0x01, 0x02, // payload
            0xDE, 0xAD, 0xBE,
        ];
        let rem_len = buf_one.len();

        let buf = BytesMut::from(&buf_one[..]);

        let p = Publish::read(first_byte & 0b0000_1111, rem_len, buf.into()).unwrap();

        let mut result_buf = BytesMut::new();
        p.write(&mut result_buf).unwrap();

        assert_eq!(buf_one.to_vec(), result_buf.to_vec())
    }
}
