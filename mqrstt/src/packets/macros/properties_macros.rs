macro_rules! define_properties {
    ($name:ident, $($prop_variant:ident),*) => {
        $crate::packets::macros::properties_struct!(@ $name { $($prop_variant,)* } -> ());

        impl<S> $crate::packets::mqtt_traits::MqttAsyncRead<S> for $name where S: tokio::io::AsyncReadExt + Unpin {
            async fn async_read(stream: &mut S) -> Result<(Self, usize), super::error::ReadError> {
                let (len, length_variable_integer) = $crate::packets::read_async_variable_integer(stream).await?;
                if len == 0 {
                    return Ok((Self::default(), length_variable_integer));
                }

                let mut properties = $name::default();

                let mut read_property_bytes = 0;
                loop {
                    let (prop, read_bytes) = PropertyType::async_read(stream).await?;
                    read_property_bytes += read_bytes;
                    match prop {
                        $(
                            PropertyType::$prop_variant => $crate::packets::macros::properties_read_matches!(stream, properties, read_property_bytes, PropertyType::$prop_variant),
                        )*
                        e => return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::UnexpectedProperty(e, PacketType::PubRel))),
                    }
                    if read_property_bytes == len {
                        break;
                    }
                }

                Ok((properties, length_variable_integer + read_property_bytes))
            }
        }

        impl $crate::packets::mqtt_traits::WireLength for $name {
            fn wire_len(&self) -> usize {
                let mut len: usize = 0;
                $(
                    $crate::packets::macros::properties_wire_length!(self, len , PropertyType::$prop_variant);
                )*;
                len
            }
        }
    };
}

macro_rules! properties_struct {
    ( @ $name:ident { } -> ($($result:tt)*) ) => (
        #[derive(Debug, PartialEq, Eq, Clone, Hash, Default)]
        pub struct $name {
            $($result)*
        }
    );
    ( @ $name:ident { PayloadFormatIndicator, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $name { $($rest)* } -> (
            $($result)*
            /// 3.3.2.3.2 Payload Format Indicator
            /// 1 (0x01) Byte, Identifier of the Payload Format Indicator.
            pub payload_format_indicator: Option<u8>,
        ));
    );
    ( @ $name:ident { MessageExpiryInterval, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $name { $($rest)* } -> (
            $($result)*
            /// 3.3.2.3.3 Message Expiry Interval
            /// 2 (0x02) Byte, Identifier of the Message Expiry Interval.
            pub message_expiry_interval: Option<u32>,
        ));
    );
    ( @ $name:ident { ContentType, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $name { $($rest)* } -> (
            $($result)*
            /// 3.3.2.3.9 Content Type
            /// 3 (0x03) Identifier of the Content Type
            pub content_type: Option<Box<str>>,
        ));
    );
    ( @ $name:ident { ResponseTopic, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $name { $($rest)* } -> (
            $($result)*
            /// 3.3.2.3.5 Response Topic
            /// 8 (0x08) Byte, Identifier of the Response Topic.
            pub response_topic: Option<Box<str>>,
        ));
    );
    ( @ $name:ident { CorrelationData, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $name { $($rest)* } -> (
            $($result)*
            /// 3.3.2.3.6 Correlation Data
            /// 9 (0x09) Byte, Identifier of the Correlation Data.
            pub correlation_data: Option<Vec<u8>>,
        ));
    );
    ( @ $name:ident { SubscriptionIdentifier, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $name { $($rest)* } -> (
            $($result)*
            /// 3.3.2.3.8 Subscription Identifier
            /// 11 (0x0B), Identifier of the Subscription Identifier.
            pub subscription_identifier: Vec<usize>,
        ));
    );
    ( @ $name:ident { SessionExpiryInterval, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $name { $($rest)* } -> (
            $($result)*
            /// 3.2.2.3.2 Session Expiry Interval
            /// 17 (0x11) Byte Identifier of the Session Expiry Interval
            pub session_expiry_interval: Option<u32>,
        ));
    );
    ( @ $name:ident { AssignedClientIdentifier, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $name { $($rest)* } -> (
            $($result)*
            /// 3.2.2.3.7 Assigned Client Identifier
            /// 18 (0x12) Byte, Identifier of the Assigned Client Identifier.
            pub assigned_client_id: Option<Box<str>>,
        ));
    );
    ( @ $name:ident { ServerKeepAlive, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $name { $($rest)* } -> (
            $($result)*
            /// 3.2.2.3.14 Server Keep Alive
            /// 19 (0x13) Byte, Identifier of the Server Keep Alive
            pub server_keep_alive: Option<u16>,
        ));
    );
    ( @ $name:ident { AuthenticationMethod, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $name { $($rest)* } -> (
            $($result)*
            /// 3.2.2.3.17 Authentication Method
            /// 21 (0x15) Byte, Identifier of the Authentication Method
            pub authentication_method: Option<Box<str>>,
        ));
    );
    ( @ $name:ident { AuthenticationData, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $name { $($rest)* } -> (
            $($result)*
            /// 3.2.2.3.18 Authentication Data
            /// 22 (0x16) Byte, Identifier of the Authentication Data
            pub authentication_data: Option<Vec<u8>>,
        ));
    );
    // ( @ $name:ident { RequestProblemInformation, $($rest:tt)* } -> ($($result:tt)*) ) => (
    //     // Missing
    // );
    // ( @ $name:ident { WillDelayInterval, $($rest:tt)* } -> ($($result:tt)*) ) => (
    //     // Missing
    // );
    // ( @ $name:ident { RequestResponseInformation, $($rest:tt)* } -> ($($result:tt)*) ) => (
    //     // Missing
    // );
    ( @ $name:ident { ResponseInformation, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $name { $($rest)* } -> (
            $($result)*
            /// 3.2.2.3.15 Response Information
            /// 26 (0x1A) Byte, Identifier of the Response Information.
            pub response_info: Option<Box<str>>,
        ));
    );
    ( @ $name:ident { ServerReference, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $name { $($rest)* } -> (
            $($result)*
            /// 3.2.2.3.16 Server Reference
            /// 28 (0x1C) Byte, Identifier of the Server Reference
            pub server_reference: Option<Box<str>>,
        ));
    );
    ( @ $name:ident { ReasonString, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $name { $($rest)* } -> (
            $($result)*
            /// 3.2.2.3.9 Reason String
            /// 31 (0x1F) Byte Identifier of the Reason String.
            pub reason_string: Option<Box<str>>,
        ));
    );
    ( @ $name:ident { ReceiveMaximum, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $name { $($rest)* } -> (
            $($result)*
            /// 3.2.2.3.3 Receive Maximum
            /// 33 (0x21) Byte, Identifier of the Receive Maximum
            pub receive_maximum: Option<u16>,
        ));
    );
    ( @ $name:ident { TopicAliasMaximum, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $name { $($rest)* } -> (
            $($result)*
            /// 3.2.2.3.8 Topic Alias Maximum
            /// 34 (0x22) Byte, Identifier of the Topic Alias Maximum.
            pub topic_alias_maximum: Option<u16>,
        ));
    );
    ( @ $name:ident { TopicAlias, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $name { $($rest)* } -> (
            $($result)*
            /// 3.3.2.3.4 Topic Alias
            /// 35 (0x23) Byte, Identifier of the Topic Alias.
            pub topic_alias: Option<u16>,
        ));
    );
    ( @ $name:ident { MaximumQos, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $name { $($rest)* } -> (
            $($result)*
            /// 3.2.2.3.4 Maximum QoS
            /// 36 (0x24) Byte, Identifier of the Maximum QoS.
            pub maximum_qos: Option<$crate::packets::QoS>,
        ));
    );
    ( @ $name:ident { RetainAvailable, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $name { $($rest)* } -> (
            $($result)*
            /// 3.2.2.3.5 Retain Available
            /// 37 (0x25) Byte, Identifier of Retain Available.
            pub retain_available: Option<bool>,
        ));
    );
    ( @ $name:ident { UserProperty, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $name { $($rest)* } -> (
            $($result)*
            /// 3.2.2.3.10 User Property
            /// 38 (0x26) Byte, Identifier of User Property.
            pub user_properties: Vec<(Box<str>, Box<str>)>,
        ));
    );
    ( @ $name:ident { MaximumPacketSize, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $name { $($rest)* } -> (
            $($result)*
            /// 3.2.2.3.6 Maximum Packet Size
            /// 39 (0x27) Byte, Identifier of the Maximum Packet Size.
            pub maximum_packet_size: Option<u32>,
        ));
    );
    ( @ $name:ident { WildcardSubscriptionAvailable, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $name { $($rest)* } -> (
            $($result)*
            /// 3.2.2.3.11 Wildcard Subscription Available
            /// 40 (0x28) Byte, Identifier of Wildcard Subscription Available.
            pub wildcards_available: Option<bool>,
        ));
    );
    ( @ $name:ident { SubscriptionIdentifierAvailable, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $name { $($rest)* } -> (
            $($result)*
            /// 3.2.2.3.12 Subscription Identifiers Available
            /// 41 (0x29) Byte, Identifier of Subscription Identifier Available.
            pub subscription_ids_available: Option<bool>,
        ));
    );
    ( @ $name:ident { SharedSubscriptionAvailable, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $name { $($rest)* } -> (
            $($result)*
            /// 3.2.2.3.13 Shared Subscription Available
            /// 42 (0x2A) Byte, Identifier of Shared Subscription Available.
            pub shared_subscription_available: Option<bool>,
        ));
    );
    ( @ $name:ident { $unknown:ident, $($rest:tt)* } -> ($($result:tt)*) ) => (
        compile_error!(concat!("Unknown property: ", stringify!($unknown)));
    );
}

macro_rules! properties_read_matches {

    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::PayloadFormatIndicator) => {
        {
            if $properties.payload_format_indicator.is_some() {
                return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(PropertyType::PayloadFormatIndicator)));
            }
            let (prop_body, read_bytes) = u8::async_read($stream).await?;
            $read_property_bytes += read_bytes;
            $properties.payload_format_indicator = Some(prop_body);
        } 
    };
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::MessageExpiryInterval) => {
        {
            if $properties.message_expiry_interval.is_some() {
                return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(PropertyType::MessageExpiryInterval)));
            }
            let (prop_body, read_bytes) = u32::async_read($stream).await?;
            $read_property_bytes += read_bytes;
            $properties.message_expiry_interval = Some(prop_body);
        } 
    };
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::ContentType) => {
        {
            if $properties.content_type.is_some() {
                return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(PropertyType::ContentType)));
            }
            let (prop_body, read_bytes) = Box::<str>::async_read($stream).await?;
            $read_property_bytes += read_bytes;
            $properties.content_type = Some(prop_body);
        } 
    };
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::ResponseTopic) => {
        {
            if $properties.response_topic.is_some() {
                return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(PropertyType::ResponseTopic)));
            }
            let (prop_body, read_bytes) = Box::<str>::async_read($stream).await?;
            $read_property_bytes += read_bytes;
            $properties.response_topic = Some(prop_body);
        } 
    };
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::CorrelationData) => {
        {
            if $properties.correlation_data.is_some() {
                return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(PropertyType::CorrelationData)));
            }
            let (prop_body, read_bytes) = Vec::<u8>::async_read($stream).await?;
            $read_property_bytes += read_bytes;
            $properties.correlation_data = Some(prop_body);
        } 
    };
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::SubscriptionIdentifier) => {
        {
            let (prop_body, read_bytes) = $crate::packets::read_async_variable_integer($stream).await?;
            $read_property_bytes += read_bytes;
            $properties.subscription_identifier.push(prop_body);
        } 
    };
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::SessionExpiryInterval) => {
        {
            if $properties.session_expiry_interval.is_some() {
                return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(PropertyType::SessionExpiryInterval)));
            }
            let (prop_body, read_bytes) = u32::async_read($stream).await?;
            $read_property_bytes += read_bytes;
            $properties.session_expiry_interval = Some(prop_body);
        } 
    };
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::AssignedClientIdentifier) => {
        {
            if $properties.assigned_client_id.is_some() {
                return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(PropertyType::AssignedClientIdentifier)));
            }
            let (prop_body, read_bytes) = Box::<str>::async_read($stream).await?;
            $read_property_bytes += read_bytes;
            $properties.assigned_client_id = Some(prop_body);
        } 
    };
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::ServerKeepAlive) => {
        {
            if $properties.server_keep_alive.is_some() {
                return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(PropertyType::ServerKeepAlive)));
            }
            let (prop_body, read_bytes) = u16::async_read($stream).await?;
            $read_property_bytes += read_bytes;
            $properties.server_keep_alive = Some(prop_body);
        } 
    };
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::AuthenticationMethod) => {
        {
            if $properties.authentication_method.is_some() {
                return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(PropertyType::AuthenticationMethod)));
            }
            let (prop_body, read_bytes) = Box::<str>::async_read($stream).await?;
            $read_property_bytes += read_bytes;
            $properties.authentication_method = Some(prop_body);
        } 
    };
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::AuthenticationData) => {
        {
            if $properties.authentication_data.is_some() {
                return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(PropertyType::AuthenticationData)));
            }
            let (prop_body, read_bytes) = Vec::<u8>::async_read($stream).await?;
            $read_property_bytes += read_bytes;
            $properties.authentication_data = Some(prop_body);
        } 
    };
    // ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::RequestResponseInformation) => {
    //     {
    //         if $properties.authentication_data.is_some() {
    //             return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(PropertyType::RequestResponseInformation)));
    //         }
    //         let (prop_body, read_bytes) = Vec::<u8>::async_read($stream).await?;
    //         $read_property_bytes += read_bytes;
    //         $properties.authentication_data = Some(prop_body);
    //     } 
    // };
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::ResponseInformation) => {
        {
            if $properties.response_info.is_some() {
                return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(PropertyType::ResponseInformation)));
            }
            let (prop_body, read_bytes) = Box::<str>::async_read($stream).await?;
            $read_property_bytes += read_bytes;
            $properties.response_info = Some(prop_body);
        } 
    };
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::ServerReference) => {
        {
            if $properties.server_reference.is_some() {
                return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(PropertyType::ServerReference)));
            }
            let (prop_body, read_bytes) = Box::<str>::async_read($stream).await?;
            $read_property_bytes += read_bytes;
            $properties.server_reference = Some(prop_body);
        } 
    };
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::ReasonString) => {
        {
            if $properties.reason_string.is_some() {
                return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(PropertyType::ReasonString)));
            }
            let (prop_body, read_bytes) = Box::<str>::async_read($stream).await?;
            $read_property_bytes += read_bytes;
            $properties.reason_string = Some(prop_body);
        }
    };
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::ReceiveMaximum) => {
        {
            if $properties.receive_maximum.is_some() {
                return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(PropertyType::ReceiveMaximum)));
            }
            let (prop_body, read_bytes) = u16::async_read($stream).await?;
            $read_property_bytes += read_bytes;
            $properties.receive_maximum = Some(prop_body);
        } 
    };
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::TopicAliasMaximum) => {
        {
            if $properties.topic_alias_maximum.is_some() {
                return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(PropertyType::TopicAliasMaximum)));
            }
            let (prop_body, read_bytes) = u16::async_read($stream).await?;
            $read_property_bytes += read_bytes;
            $properties.topic_alias_maximum = Some(prop_body);
        } 
    };
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::TopicAlias) => {
        {
            if $properties.topic_alias.is_some() {
                return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(PropertyType::MessageExpiryInterval)));
            }
            let (prop_body, read_bytes) = u16::async_read($stream).await?;
            $read_property_bytes += read_bytes;
            $properties.topic_alias = Some(prop_body);
        } 
    };
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::MaximumQos) => {
        {
            if $properties.maximum_qos.is_some() {
                return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(PropertyType::MaximumQos)));
            }
            let (prop_body, read_bytes) = $crate::packets::QoS::async_read($stream).await?;
            $read_property_bytes += read_bytes;
            $properties.maximum_qos = Some(prop_body);
        } 
    };
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::RetainAvailable) => {
        {
            if $properties.retain_available.is_some() {
                return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(PropertyType::RetainAvailable)));
            }
            let (prop_body, read_bytes) = bool::async_read($stream).await?;
            $read_property_bytes += read_bytes;
            $properties.retain_available = Some(prop_body);
        } 
    };
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::UserProperty) => {
        {
            let (prop_body_key, read_bytes) = Box::<str>::async_read($stream).await?;
            $read_property_bytes += read_bytes;
            let (prop_body_value, read_bytes) = Box::<str>::async_read($stream).await?;
            $read_property_bytes += read_bytes;
            
            $properties.user_properties.push((prop_body_key, prop_body_value))
        }
    };
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::MaximumPacketSize) => {
        {
            if $properties.maximum_packet_size.is_some() {
                return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(PropertyType::RetainAvailable)));
            }
            let (prop_body, read_bytes) = u32::async_read($stream).await?;
            $read_property_bytes += read_bytes;
            $properties.maximum_packet_size = Some(prop_body);
        } 
    };
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::WildcardSubscriptionAvailable) => {
        {
            if $properties.wildcards_available.is_some() {
                return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(PropertyType::WildcardSubscriptionAvailable)));
            }
            let (prop_body, read_bytes) = bool::async_read($stream).await?;
            $read_property_bytes += read_bytes;
            $properties.wildcards_available = Some(prop_body);
        } 
    };
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::SubscriptionIdentifierAvailable) => {
        {
            if $properties.subscription_ids_available.is_some() {
                return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(PropertyType::SubscriptionIdentifierAvailable)));
            }
            let (prop_body, read_bytes) = bool::async_read($stream).await?;
            $read_property_bytes += read_bytes;
            $properties.subscription_ids_available = Some(prop_body);
        } 
    };
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::SharedSubscriptionAvailable) => {
        {
            if $properties.shared_subscription_available.is_some() {
                return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(PropertyType::SharedSubscriptionAvailable)));
            }
            let (prop_body, read_bytes) = bool::async_read($stream).await?;
            $read_property_bytes += read_bytes;
            $properties.shared_subscription_available = Some(prop_body);
        } 
    };
}

macro_rules! properties_wire_length{

    ($self:ident, $len:ident, PropertyType::PayloadFormatIndicator) => {
        if $self.payload_format_indicator.is_some() {
            $len += 2;
        }
    };
    ($self:ident, $len:ident, PropertyType::MessageExpiryInterval) => {
        if $self.message_expiry_interval.is_some() {
            $len += 1 + 4;
        }
    };
    ($self:ident, $len:ident, PropertyType::ContentType) => {
        if let Some(content_type) = &($self.content_type) {
            $len += 1 + content_type.wire_len();
        }
    };
    ($self:ident, $len:ident, PropertyType::ResponseTopic) => {
        if let Some(response_topic) = &($self.response_topic) {
            $len += 1 + response_topic.wire_len();
        }
    };
    ($self:ident, $len:ident, PropertyType::CorrelationData) => {
        if let Some(correlation_data) = &($self.correlation_data) {
            $len += 1 + correlation_data.wire_len();
        }
    };
    ($self:ident, $len:ident, PropertyType::SubscriptionIdentifier) => {
        for sub_id in  &($self.subscription_identifier) {
            $len += 1 + $crate::packets::variable_integer_len(*sub_id);
        }
    };
    ($self:ident, $len:ident, PropertyType::SessionExpiryInterval) => {
        if $self.session_expiry_interval.is_some() {
            $len += 1 + 4;
        }
    };
    ($self:ident, $len:ident, PropertyType::AssignedClientIdentifier) => {
        if let Some(client_id) = $self.assigned_client_id.as_ref() {
            $len += 1 + client_id.wire_len();
        }
    };
    ($self:ident, $len:ident, PropertyType::ServerKeepAlive) => {
        if $self.server_keep_alive.is_some() {
            $len += 1 + 2;
        }
    };
    ($self:ident, $len:ident, PropertyType::AuthenticationMethod) => {
        if let Some(authentication_method) = &($self.authentication_method) {
            $len += 1 + authentication_method.wire_len();
        }
    };
    ($self:ident, $len:ident, PropertyType::AuthenticationData) => {
        if $self.authentication_data.is_some() && $self.authentication_method.is_some() {
            $len += 1 + $self.authentication_data.as_ref().map(WireLength::wire_len).unwrap_or(0);
        }
    };
    // ($self:ident, $len:ident, PropertyType::RequestResponseInformation) => {
    //Will Delay Interval 
    // ($self:ident, $len:ident, PropertyType::RequestResponseInformation) => {
    ($self:ident, $len:ident, PropertyType::ResponseInformation) => {
        if let Some(response_info) = &($self.response_info) {
            $len += 1 + response_info.wire_len();
        }
    };
    ($self:ident, $len:ident, PropertyType::ServerReference) => {
        if let Some(server_reference) = &($self.server_reference) {
            $len += 1 + server_reference.wire_len();
        }
    };
    ($self:ident, $len:ident, PropertyType::ReasonString) => {
        if let Some(reason_string) = &($self.reason_string) {
            $len += 1 + reason_string.wire_len();
        }
    };
    ($self:ident, $len:ident, PropertyType::ReceiveMaximum) => {
        if $self.receive_maximum.is_some() {
            $len += 1 + 2;
        }
    };
    ($self:ident, $len:ident, PropertyType::TopicAliasMaximum) => {
        if $self.topic_alias_maximum.is_some() {
            $len += 1 + 2;
        }
    };
    ($self:ident, $len:ident, PropertyType::TopicAlias) => {
        if $self.topic_alias.is_some() {
            $len += 3;
        }
    };
    ($self:ident, $len:ident, PropertyType::MaximumQos) => {
        if $self.maximum_qos.is_some() {
            $len += 1 + 1;
        }
    };
    ($self:ident, $len:ident, PropertyType::RetainAvailable) => {
        if $self.retain_available.is_some() {
            $len += 1 + 1;
        }
    };
    ($self:ident, $len:ident, PropertyType::UserProperty) => {
        for (key, value) in &($self.user_properties) {
            $len += 1;
            $len += key.wire_len();
            $len += value.wire_len();
        }
    };
    ($self:ident, $len:ident, PropertyType::MaximumPacketSize) => {
        if $self.maximum_packet_size.is_some() {
            $len += 1 + 4;
        }
    };
    ($self:ident, $len:ident, PropertyType::WildcardSubscriptionAvailable) => {
        if $self.wildcards_available.is_some() {
            $len += 1 + 1;
        }
    };
    ($self:ident, $len:ident, PropertyType::SubscriptionIdentifierAvailable) => {
        if $self.subscription_ids_available.is_some() {
            $len += 1 + 1;
        }
    };
    ($self:ident, $len:ident, PropertyType::SharedSubscriptionAvailable) => {
        if $self.shared_subscription_available.is_some() {
            $len += 1 + 1;
        }
    };
    ($self:ident, $len:ident, $unknown:ident) => (
        compile_error!(concat!("Unknown property: ", stringify!($unknown)));
    );
}

pub(crate) use define_properties;
pub(crate) use properties_struct;
pub(crate) use properties_read_matches;
pub(crate) use properties_wire_length;