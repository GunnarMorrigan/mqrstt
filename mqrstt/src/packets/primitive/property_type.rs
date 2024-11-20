use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::packets::{
    error::{DeserializeError, ReadError, SerializeError},
    mqtt_trait::{MqttAsyncRead, MqttRead, MqttWrite},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyType {
    PayloadFormatIndicator = 1,
    MessageExpiryInterval = 2,
    ContentType = 3,
    ResponseTopic = 8,
    CorrelationData = 9,
    SubscriptionIdentifier = 11,
    SessionExpiryInterval = 17,
    AssignedClientIdentifier = 18,
    ServerKeepAlive = 19,
    AuthenticationMethod = 21,
    AuthenticationData = 22,
    RequestProblemInformation = 23,
    WillDelayInterval = 24,
    RequestResponseInformation = 25,
    ResponseInformation = 26,
    ServerReference = 28,
    ReasonString = 31,
    ReceiveMaximum = 33,
    TopicAliasMaximum = 34,
    TopicAlias = 35,
    MaximumQos = 36,
    RetainAvailable = 37,
    UserProperty = 38,
    MaximumPacketSize = 39,
    WildcardSubscriptionAvailable = 40,
    SubscriptionIdentifierAvailable = 41,
    SharedSubscriptionAvailable = 42,
}

impl TryFrom<u8> for PropertyType {
    type Error = DeserializeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::PayloadFormatIndicator),
            2 => Ok(Self::MessageExpiryInterval),
            3 => Ok(Self::ContentType),
            8 => Ok(Self::ResponseTopic),
            9 => Ok(Self::CorrelationData),
            11 => Ok(Self::SubscriptionIdentifier),
            17 => Ok(Self::SessionExpiryInterval),
            18 => Ok(Self::AssignedClientIdentifier),
            19 => Ok(Self::ServerKeepAlive),
            21 => Ok(Self::AuthenticationMethod),
            22 => Ok(Self::AuthenticationData),
            23 => Ok(Self::RequestProblemInformation),
            24 => Ok(Self::WillDelayInterval),
            25 => Ok(Self::RequestResponseInformation),
            26 => Ok(Self::ResponseInformation),
            28 => Ok(Self::ServerReference),
            31 => Ok(Self::ReasonString),
            33 => Ok(Self::ReceiveMaximum),
            34 => Ok(Self::TopicAliasMaximum),
            35 => Ok(Self::TopicAlias),
            36 => Ok(Self::MaximumQos),
            37 => Ok(Self::RetainAvailable),
            38 => Ok(Self::UserProperty),
            39 => Ok(Self::MaximumPacketSize),
            40 => Ok(Self::WildcardSubscriptionAvailable),
            41 => Ok(Self::SubscriptionIdentifierAvailable),
            42 => Ok(Self::SharedSubscriptionAvailable),
            t => Err(DeserializeError::UnknownProperty(t)),
        }
    }
}

impl From<&PropertyType> for u8 {
    fn from(value: &PropertyType) -> Self {
        match value {
            PropertyType::PayloadFormatIndicator => 1,
            PropertyType::MessageExpiryInterval => 2,
            PropertyType::ContentType => 3,
            PropertyType::ResponseTopic => 8,
            PropertyType::CorrelationData => 9,
            PropertyType::SubscriptionIdentifier => 11,
            PropertyType::SessionExpiryInterval => 17,
            PropertyType::AssignedClientIdentifier => 18,
            PropertyType::ServerKeepAlive => 19,
            PropertyType::AuthenticationMethod => 21,
            PropertyType::AuthenticationData => 22,
            PropertyType::RequestProblemInformation => 23,
            PropertyType::WillDelayInterval => 24,
            PropertyType::RequestResponseInformation => 25,
            PropertyType::ResponseInformation => 26,
            PropertyType::ServerReference => 28,
            PropertyType::ReasonString => 31,
            PropertyType::ReceiveMaximum => 33,
            PropertyType::TopicAliasMaximum => 34,
            PropertyType::TopicAlias => 35,
            PropertyType::MaximumQos => 36,
            PropertyType::RetainAvailable => 37,
            PropertyType::UserProperty => 38,
            PropertyType::MaximumPacketSize => 39,
            PropertyType::WildcardSubscriptionAvailable => 40,
            PropertyType::SubscriptionIdentifierAvailable => 41,
            PropertyType::SharedSubscriptionAvailable => 42,
        }
    }
}

impl From<PropertyType> for u8 {
    fn from(value: PropertyType) -> Self {
        value as u8
    }
}

impl MqttRead for PropertyType {
    fn read(buf: &mut Bytes) -> Result<Self, DeserializeError> {
        if buf.is_empty() {
            return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), 0, 1));
        }

        buf.get_u8().try_into()
    }
}

impl<T> MqttAsyncRead<T> for PropertyType
where
    T: tokio::io::AsyncReadExt + std::marker::Unpin,
{
    async fn async_read(buf: &mut T) -> Result<(Self, usize), ReadError> {
        match buf.read_u8().await {
            Ok(t) => Ok((t.try_into()?, 1)),
            Err(e) => Err(ReadError::IoError(e)),
        }
    }
}

impl MqttWrite for PropertyType {
    fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError> {
        buf.put_u8(self.into());
        Ok(())
    }
}
