use bytes::{Bytes, BytesMut};

use crate::packets::{
    error::{DeserializeError, SerializeError},
    mqtt_trait::{MqttAsyncRead, MqttRead, MqttWrite},
    QoS, WireLength,
};

use super::{LastWillProperties, VariableInteger};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastWill {
    /// 3.1.2.6 Will QoS
    pub qos: QoS,
    /// 3.1.2.7 Will Retain
    pub retain: bool,

    /// 3.1.3.2 Will properties
    pub last_will_properties: LastWillProperties,
    /// 3.1.3.3 Will Topic
    pub topic: Box<str>,
    /// 3.1.3.4 Will payload
    pub payload: Vec<u8>,
}

impl LastWill {
    pub fn new<T: AsRef<str>, P: Into<Vec<u8>>>(qos: QoS, retain: bool, topic: T, payload: P) -> LastWill {
        Self {
            qos,
            retain,
            last_will_properties: LastWillProperties::default(),
            topic: topic.as_ref().into(),
            payload: payload.into(),
        }
    }
    pub(crate) fn read(qos: QoS, retain: bool, buf: &mut Bytes) -> Result<Self, DeserializeError> {
        let last_will_properties = LastWillProperties::read(buf)?;
        let topic = Box::<str>::read(buf)?;
        let payload = Vec::<u8>::read(buf)?;

        Ok(Self {
            qos,
            retain,
            topic,
            payload,
            last_will_properties,
        })
    }
    pub(crate) async fn async_read<S>(qos: QoS, retain: bool, stream: &mut S) -> Result<(Self, usize), crate::packets::error::ReadError>
    where
        S: tokio::io::AsyncRead + Unpin,
    {
        let (last_will_properties, last_will_properties_read_bytes) = LastWillProperties::async_read(stream).await?;
        let (topic, topic_read_bytes) = Box::<str>::async_read(stream).await?;
        let (payload, payload_read_bytes) = Vec::<u8>::async_read(stream).await?;

        let total_read_bytes = last_will_properties_read_bytes + topic_read_bytes + payload_read_bytes;

        Ok((
            Self {
                qos,
                retain,
                last_will_properties,
                topic,
                payload,
            },
            total_read_bytes,
        ))
    }
}

impl MqttWrite for LastWill {
    fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError> {
        self.last_will_properties.write(buf)?;
        self.topic.write(buf)?;
        self.payload.write(buf)?;
        Ok(())
    }
}

impl WireLength for LastWill {
    fn wire_len(&self) -> usize {
        let property_len = self.last_will_properties.wire_len();

        self.topic.wire_len() + self.payload.wire_len() + property_len.variable_integer_len() + property_len
    }
}
