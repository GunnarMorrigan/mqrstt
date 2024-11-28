mod properties;

pub use properties::SubscribeProperties;
use tokio::io::AsyncReadExt;

use crate::{error::PacketValidationError, util::constants::MAXIMUM_TOPIC_SIZE};

use super::{
    error::DeserializeError,
    mqtt_trait::{MqttAsyncRead, MqttAsyncWrite, MqttRead, MqttWrite, PacketAsyncRead, PacketRead, PacketValidation, PacketWrite, WireLength},
    QoS, VariableInteger,
};
use bytes::{Buf, BufMut};

/// Used to subscribe to topic(s).
///
/// Multiple topics can be subscribed from at once.
/// For convenience [`SubscribeTopics`] is provided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subscribe {
    pub packet_identifier: u16,
    pub properties: SubscribeProperties,
    pub topics: Vec<(Box<str>, SubscriptionOptions)>,
}

impl Subscribe {
    pub fn new(packet_identifier: u16, topics: Vec<(Box<str>, SubscriptionOptions)>) -> Self {
        Self {
            packet_identifier,
            properties: SubscribeProperties::default(),
            topics,
        }
    }
}

impl PacketRead for Subscribe {
    fn read(_: u8, _: usize, mut buf: bytes::Bytes) -> Result<Self, super::error::DeserializeError> {
        let packet_identifier = u16::read(&mut buf)?;
        let properties = SubscribeProperties::read(&mut buf)?;
        let mut topics = vec![];
        loop {
            let topic = Box::<str>::read(&mut buf)?;
            let options = SubscriptionOptions::read(&mut buf)?;

            topics.push((topic, options));

            if buf.is_empty() {
                break;
            }
        }

        Ok(Self {
            packet_identifier,
            properties,
            topics,
        })
    }
}

impl<S> PacketAsyncRead<S> for Subscribe
where
    S: tokio::io::AsyncRead + Unpin,
{
    async fn async_read(_: u8, remaining_length: usize, stream: &mut S) -> Result<(Self, usize), crate::packets::error::ReadError> {
        let mut total_read_bytes = 0;
        let packet_identifier = stream.read_u16().await?;
        let (properties, proproperties_read_bytes) = SubscribeProperties::async_read(stream).await?;
        total_read_bytes += 2 + proproperties_read_bytes;

        let mut topics = vec![];
        loop {
            let (topic, topic_read_bytes) = Box::<str>::async_read(stream).await?;
            let (options, options_read_bytes) = SubscriptionOptions::async_read(stream).await?;
            total_read_bytes += topic_read_bytes + options_read_bytes;
            topics.push((topic, options));

            if remaining_length >= total_read_bytes {
                break;
            }
        }

        Ok((
            Self {
                packet_identifier,
                properties,
                topics,
            },
            total_read_bytes,
        ))
    }
}

impl PacketWrite for Subscribe {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
        buf.put_u16(self.packet_identifier);

        self.properties.write(buf)?;
        for (topic, options) in &self.topics {
            topic.write(buf)?;
            options.write(buf)?;
        }

        Ok(())
    }
}

impl<S> crate::packets::mqtt_trait::PacketAsyncWrite<S> for Subscribe
where
    S: tokio::io::AsyncWrite + Unpin,
{
    async fn async_write(&self, stream: &mut S) -> Result<usize, crate::packets::error::WriteError> {
        use crate::packets::mqtt_trait::MqttAsyncWrite;
        use tokio::io::AsyncWriteExt;
        let mut total_written_bytes = 2;
        stream.write_u16(self.packet_identifier).await?;

        total_written_bytes += self.properties.async_write(stream).await?;
        for (topic, options) in &self.topics {
            total_written_bytes += topic.async_write(stream).await?;
            total_written_bytes += options.async_write(stream).await?;
        }
        Ok(total_written_bytes)
    }
}

impl WireLength for Subscribe {
    fn wire_len(&self) -> usize {
        let mut len = 2;
        let properties_len = self.properties.wire_len();
        len += properties_len + properties_len.variable_integer_len();
        for topic in &self.topics {
            len += topic.0.wire_len() + 1;
        }
        len
    }
}

impl PacketValidation for Subscribe {
    fn validate(&self, max_packet_size: usize) -> Result<(), PacketValidationError> {
        if self.wire_len() > max_packet_size {
            return Err(PacketValidationError::MaxPacketSize(self.wire_len()));
        }
        for (topic, _) in &self.topics {
            if topic.len() > MAXIMUM_TOPIC_SIZE {
                return Err(PacketValidationError::TopicSize(topic.len()));
            }
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct SubscriptionOptions {
    pub retain_handling: RetainHandling,
    pub retain_as_publish: bool,
    pub no_local: bool,
    pub qos: QoS,
}

impl Default for SubscriptionOptions {
    fn default() -> Self {
        Self {
            retain_handling: RetainHandling::ZERO,
            retain_as_publish: false,
            no_local: false,
            qos: QoS::AtMostOnce,
        }
    }
}

impl MqttRead for SubscriptionOptions {
    fn read(buf: &mut bytes::Bytes) -> Result<Self, DeserializeError> {
        if buf.is_empty() {
            return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), 0, 1));
        }

        let byte = buf.get_u8();

        let retain_handling_part = (byte & 0b00110000) >> 4;
        let retain_as_publish_part = (byte & 0b00001000) >> 3;
        let no_local_part = (byte & 0b00000100) >> 2;
        let qos_part = byte & 0b00000011;

        let options = Self {
            retain_handling: RetainHandling::from_u8(retain_handling_part)?,
            retain_as_publish: retain_as_publish_part != 0,
            no_local: no_local_part != 0,
            qos: QoS::from_u8(qos_part)?,
        };

        Ok(options)
    }
}

impl<S> MqttAsyncRead<S> for SubscriptionOptions
where
    S: tokio::io::AsyncRead + Unpin,
{
    async fn async_read(stream: &mut S) -> Result<(Self, usize), crate::packets::error::ReadError> {
        let byte = stream.read_u8().await?;

        let retain_handling_part = (byte & 0b00110000) >> 4;
        let retain_as_publish_part = (byte & 0b00001000) >> 3;
        let no_local_part = (byte & 0b00000100) >> 2;
        let qos_part = byte & 0b00000011;

        let options = Self {
            retain_handling: RetainHandling::from_u8(retain_handling_part)?,
            retain_as_publish: retain_as_publish_part != 0,
            no_local: no_local_part != 0,
            qos: QoS::from_u8(qos_part)?,
        };

        Ok((options, 1))
    }
}

impl MqttWrite for SubscriptionOptions {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
        let byte = (self.retain_handling.into_u8() << 4) | ((self.retain_as_publish as u8) << 3) | ((self.no_local as u8) << 2) | self.qos.into_u8();

        buf.put_u8(byte);
        Ok(())
    }
}

impl<S> MqttAsyncWrite<S> for SubscriptionOptions
where
    S: tokio::io::AsyncWrite + Unpin,
{
    fn async_write(&self, stream: &mut S) -> impl std::future::Future<Output = Result<usize, crate::packets::error::WriteError>> {
        use tokio::io::AsyncWriteExt;
        async move {
            let byte = (self.retain_handling.into_u8() << 4) | ((self.retain_as_publish as u8) << 3) | ((self.no_local as u8) << 2) | self.qos.into_u8();
            stream.write_u8(byte).await?;
            Ok(1)
        }
    }
}

/// Controls how retained messages are handled
///
/// Used when a new subscription is established. Here are the three options for retain handling:
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetainHandling {
    /// Send Retained Messages at Subscription: This is the default behavior. When a client subscribes to a topic, the broker sends any retained messages for that topic immediately.
    ZERO,
    /// Send Retained Messages Only for New Subscriptions: Retained messages are sent only if the subscription did not previously exist.
    ONE,
    /// Do Not Send Retained Messages: Retained messages are not sent when the subscription is established
    TWO,
}

impl RetainHandling {
    pub fn from_u8(value: u8) -> Result<Self, DeserializeError> {
        match value {
            0 => Ok(RetainHandling::ZERO),
            1 => Ok(RetainHandling::ONE),
            2 => Ok(RetainHandling::TWO),
            _ => Err(DeserializeError::MalformedPacket),
        }
    }
    pub fn into_u8(&self) -> u8 {
        match self {
            RetainHandling::ZERO => 0,
            RetainHandling::ONE => 1,
            RetainHandling::TWO => 2,
        }
    }
}

// -------------------- Simple types --------------------
trait IntoSingleSubscription {
    fn into(value: Self) -> (Box<str>, SubscriptionOptions);
}

impl IntoSingleSubscription for &str {
    #[inline]
    fn into(value: Self) -> (Box<str>, SubscriptionOptions) {
        (Box::from(value), SubscriptionOptions::default())
    }
}
impl IntoSingleSubscription for &String {
    #[inline]
    fn into(value: Self) -> (Box<str>, SubscriptionOptions) {
        (Box::from(value.as_str()), SubscriptionOptions::default())
    }
}
impl IntoSingleSubscription for String {
    #[inline]
    fn into(value: Self) -> (Box<str>, SubscriptionOptions) {
        (Box::from(value.as_str()), SubscriptionOptions::default())
    }
}
impl IntoSingleSubscription for Box<str> {
    #[inline]
    fn into(value: Self) -> (Box<str>, SubscriptionOptions) {
        (value, SubscriptionOptions::default())
    }
}
impl<T> IntoSingleSubscription for &(T, QoS)
where
    T: AsRef<str>,
{
    #[inline]
    fn into((topic, qos): Self) -> (Box<str>, SubscriptionOptions) {
        (Box::from(topic.as_ref()), SubscriptionOptions { qos: *qos, ..Default::default() })
    }
}
impl<T> IntoSingleSubscription for (T, QoS)
where
    T: AsRef<str>,
{
    #[inline]
    fn into((topic, qos): Self) -> (Box<str>, SubscriptionOptions) {
        (Box::from(topic.as_ref()), SubscriptionOptions { qos, ..Default::default() })
    }
}
impl<T> IntoSingleSubscription for &(T, SubscriptionOptions)
where
    T: AsRef<str>,
{
    #[inline]
    fn into((topic, sub): Self) -> (Box<str>, SubscriptionOptions) {
        (Box::from(topic.as_ref()), *sub)
    }
}
impl<T> IntoSingleSubscription for (T, SubscriptionOptions)
where
    T: AsRef<str>,
{
    #[inline]
    fn into((topic, sub): Self) -> (Box<str>, SubscriptionOptions) {
        (Box::from(topic.as_ref()), sub)
    }
}

macro_rules! impl_subscription {
    ($t:ty) => {
        impl From<$t> for SubscribeTopics {
            #[inline]
            fn from(value: $t) -> Self {
                Self(vec![IntoSingleSubscription::into(value)])
            }
        }
    };
}

pub struct SubscribeTopics(pub Vec<(Box<str>, SubscriptionOptions)>);

// -------------------- Simple types --------------------
impl_subscription!(&str);
impl_subscription!(&String);
impl_subscription!(String);
impl_subscription!(Box<str>);
impl From<&(&str, QoS)> for SubscribeTopics {
    fn from(value: &(&str, QoS)) -> Self {
        Self(vec![IntoSingleSubscription::into(value)])
    }
}
impl<T> From<(T, QoS)> for SubscribeTopics
where
    (T, QoS): IntoSingleSubscription,
{
    fn from(value: (T, QoS)) -> Self {
        Self(vec![IntoSingleSubscription::into(value)])
    }
}
impl<T> From<(T, SubscriptionOptions)> for SubscribeTopics
where
    (T, SubscriptionOptions): IntoSingleSubscription,
{
    fn from(value: (T, SubscriptionOptions)) -> Self {
        Self(vec![IntoSingleSubscription::into(value)])
    }
}
// -------------------- Arrays --------------------
impl<T, const S: usize> From<&[T; S]> for SubscribeTopics
where
    for<'any> &'any T: IntoSingleSubscription,
{
    fn from(value: &[T; S]) -> Self {
        Self(value.iter().map(IntoSingleSubscription::into).collect())
    }
}
// -------------------- Slices --------------------
impl<T> From<&[T]> for SubscribeTopics
where
    for<'any> &'any T: IntoSingleSubscription,
{
    fn from(value: &[T]) -> Self {
        Self(value.iter().map(IntoSingleSubscription::into).collect())
    }
}
// -------------------- Vecs --------------------
impl<T> From<Vec<T>> for SubscribeTopics
where
    T: IntoSingleSubscription,
{
    fn from(value: Vec<T>) -> Self {
        Self(value.into_iter().map(IntoSingleSubscription::into).collect())
    }
}
impl<T> From<&Vec<T>> for SubscribeTopics
where
    for<'any> &'any T: IntoSingleSubscription,
{
    fn from(value: &Vec<T>) -> Self {
        Self(value.iter().map(IntoSingleSubscription::into).collect())
    }
}

#[cfg(test)]
mod tests {
    use bytes::{Bytes, BytesMut};

    use crate::packets::{
        mqtt_trait::{MqttRead, PacketRead, PacketWrite},
        Packet,
    };

    use super::WireLength;

    use super::{Subscribe, SubscribeProperties, SubscriptionOptions};

    #[test]
    fn test_read_write_subscribe() {
        let _entire_sub_packet = [
            0x82, 0x1e, 0x35, 0xd6, 0x02, 0x0b, 0x01, 0x00, 0x16, 0x73, 0x75, 0x62, 0x73, 0x63, 0x72, 0x69, 0x62, 0x65, 0x2f, 0x65, 0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x2f, 0x74, 0x65, 0x73, 0x74,
            0x15,
        ];

        let sub_data = [
            0x35, 0xd6, 0x02, 0x0b, 0x01, 0x00, 0x16, 0x73, 0x75, 0x62, 0x73, 0x63, 0x72, 0x69, 0x62, 0x65, 0x2f, 0x65, 0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x2f, 0x74, 0x65, 0x73, 0x74, 0x15,
        ];


        let mut bufmut = BytesMut::new();
        bufmut.extend(&sub_data[..]);

        let buf: Bytes = bufmut.into();

        let s = Subscribe::read(0, 30, buf.clone()).unwrap();

        let mut result = BytesMut::new();
        s.write(&mut result).unwrap();

        assert_eq!(buf.to_vec(), result.to_vec());
    }

    #[test]
    fn test_write() {
        let expected_bytes = [0x82, 0x0e, 0x00, 0x01, 0x00, 0x00, 0x08, 0x74, 0x65, 0x73, 0x74, 0x2f, 0x31, 0x32, 0x33, 0x00];

        let sub = Subscribe {
            packet_identifier: 1,
            properties: SubscribeProperties::default(),
            topics: vec![("test/123".into(), SubscriptionOptions::default())],
        };

        assert_eq!(14, sub.wire_len());

        let packet = Packet::Subscribe(sub);

        let mut write_buffer = BytesMut::new();

        packet.write(&mut write_buffer).unwrap();

        assert_eq!(expected_bytes.to_vec(), write_buffer.to_vec())
    }

    #[test]
    fn test_subscription_options() {
        let mut buf = Bytes::from_static(&[0b00101110u8]);
        let _ = SubscriptionOptions::read(&mut buf).unwrap();
    }
}
