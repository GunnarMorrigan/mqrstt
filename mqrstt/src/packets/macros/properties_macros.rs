macro_rules! define_properties {
    ($(#[$attr:meta])* $name:ident, $($prop_variant:ident),*) => {
        $crate::packets::macros::properties_struct!(@
            $(#[$attr])*
            $name { $($prop_variant,)* } -> ()
        );

        impl<S> $crate::packets::mqtt_trait::MqttAsyncRead<S> for $name where S: tokio::io::AsyncRead + Unpin {
            async fn async_read(stream: &mut S) -> Result<(Self, usize), $crate::packets::error::ReadError> {
                let (len, length_variable_integer) = <usize as crate::packets::primitive::VariableInteger>::read_async_variable_integer(stream).await?;
                if len == 0 {
                    return Ok((Self::default(), length_variable_integer));
                }

                let mut properties = $name::default();

                let mut read_property_bytes = 0;
                loop {
                    let (prop, read_bytes) = crate::packets::PropertyType::async_read(stream).await?;
                    read_property_bytes += read_bytes;
                    match prop {
                        $(
                            $crate::packets::macros::properties_read_match_branch_name!($prop_variant) => $crate::packets::macros::properties_read_match_branch_body!(stream, properties, read_property_bytes, PropertyType::$prop_variant),
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

        impl<S> $crate::packets::mqtt_trait::MqttAsyncWrite<S> for $name where S: tokio::io::AsyncWrite + Unpin {
            async fn async_write(&self, stream: &mut S) -> Result<usize, crate::packets::error::WriteError> {
                let mut bytes_writen = 0;
                $crate::packets::VariableInteger::write_async_variable_integer(&self.wire_len(), stream).await?;
                $(
                    $crate::packets::macros::properties_write!(self, bytes_writen, stream, PropertyType::$prop_variant);
                )*

                Ok(bytes_writen)
            }
        }

        impl $crate::packets::mqtt_trait::WireLength for $name {
            fn wire_len(&self) -> usize {
                let mut len: usize = 0;
                $(
                    $crate::packets::macros::properties_wire_length!(self, len , PropertyType::$prop_variant);
                )*
                len
            }
        }
    };
}

macro_rules! properties_struct {
    ( @ $(#[$attr:meta])*  $name:ident { } -> ($($result:tt)*) ) => (
        // $(#[$attr])*
        #[derive(Debug, PartialEq, Eq, Clone, Hash, Default)]
        pub struct $name {
            $($result)*
        }
    );
    ( @ $(#[$attr:meta])*  $name:ident { PayloadFormatIndicator, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $(#[$attr])*  $name { $($rest)* } -> (
            $($result)*
            /// 3.3.2.3.2 Payload Format Indicator
            /// 1 (0x01) Byte, Identifier of the Payload Format Indicator.
            pub payload_format_indicator: Option<u8>,
        ));
    );
    ( @ $(#[$attr:meta])*  $name:ident { MessageExpiryInterval, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $(#[$attr])*  $name { $($rest)* } -> (
            $($result)*
            /// 3.3.2.3.3 Message Expiry Interval
            /// 2 (0x02) Byte, Identifier of the Message Expiry Interval.
            pub message_expiry_interval: Option<u32>,
        ));
    );
    ( @ $(#[$attr:meta])*  $name:ident { ContentType, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $(#[$attr])*  $name { $($rest)* } -> (
            $($result)*
            /// 3.3.2.3.9 Content Type
            /// 3 (0x03) Identifier of the Content Type
            pub content_type: Option<Box<str>>,
        ));
    );
    ( @ $(#[$attr:meta])*  $name:ident { ResponseTopic, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $(#[$attr])*  $name { $($rest)* } -> (
            $($result)*
            /// 3.3.2.3.5 Response Topic
            /// 8 (0x08) Byte, Identifier of the Response Topic.
            pub response_topic: Option<Box<str>>,
        ));
    );
    ( @ $(#[$attr:meta])*  $name:ident { CorrelationData, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $(#[$attr])*  $name { $($rest)* } -> (
            $($result)*
            /// 3.3.2.3.6 Correlation Data
            /// 9 (0x09) Byte, Identifier of the Correlation Data.
            pub correlation_data: Option<Vec<u8>>,
        ));
    );
    ( @ $(#[$attr:meta])*  $name:ident { ListSubscriptionIdentifier, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $(#[$attr])*  $name { $($rest)* } -> (
            $($result)*
            /// 3.3.2.3.8 Subscription Identifier
            /// 11 (0x0B), Identifier of the Subscription Identifier.
            /// Multiple Subscription Identifiers used in the Publish packet.
            pub subscription_identifiers: Vec<u32>,
        ));
    );
    ( @ $(#[$attr:meta])*  $name:ident { SubscriptionIdentifier, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $(#[$attr])*  $name { $($rest)* } -> (
            $($result)*
            /// 3.3.2.3.8 Subscription Identifier
            /// 11 (0x0B), Identifier of the Subscription Identifier.
            pub subscription_identifier: Option<u32>,
        ));
    );
    ( @ $(#[$attr:meta])*  $name:ident { SessionExpiryInterval, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $(#[$attr])*  $name { $($rest)* } -> (
            $($result)*
            /// 3.2.2.3.2 Session Expiry Interval
            /// 17 (0x11) Byte Identifier of the Session Expiry Interval
            pub session_expiry_interval: Option<u32>,
        ));
    );
    ( @ $(#[$attr:meta])*  $name:ident { AssignedClientIdentifier, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $(#[$attr])*  $name { $($rest)* } -> (
            $($result)*
            /// 3.2.2.3.7 Assigned Client Identifier
            /// 18 (0x12) Byte, Identifier of the Assigned Client Identifier.
            pub assigned_client_identifier: Option<Box<str>>,
        ));
    );
    ( @ $(#[$attr:meta])*  $name:ident { ServerKeepAlive, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $(#[$attr])*  $name { $($rest)* } -> (
            $($result)*
            /// 3.2.2.3.14 Server Keep Alive
            /// 19 (0x13) Byte, Identifier of the Server Keep Alive
            pub server_keep_alive: Option<u16>,
        ));
    );
    ( @ $(#[$attr:meta])*  $name:ident { AuthenticationMethod, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $(#[$attr])*  $name { $($rest)* } -> (
            $($result)*
            /// 3.2.2.3.17 Authentication Method
            /// 21 (0x15) Byte, Identifier of the Authentication Method
            pub authentication_method: Option<Box<str>>,
        ));
    );
    ( @ $(#[$attr:meta])*  $name:ident { AuthenticationData, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $(#[$attr])*  $name { $($rest)* } -> (
            $($result)*
            /// 3.2.2.3.18 Authentication Data
            /// 22 (0x16) Byte, Identifier of the Authentication Data
            pub authentication_data: Option<Vec<u8>>,
        ));
    );
    ( @ $(#[$attr:meta])*  $name:ident { RequestProblemInformation, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $(#[$attr])*  $name { $($rest)* } -> (
            $($result)*
            /// 3.1.2.11.7 Request Problem Information
            /// 23 (0x17) Byte, Identifier of the Request Problem Information
            pub request_problem_information: Option<u8>,
        ));
    );
    ( @ $(#[$attr:meta])*  $name:ident { WillDelayInterval, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $(#[$attr])*  $name { $($rest)* } -> (
            $($result)*
            /// 3.1.3.2.2 Request Problem Information
            /// 24 (0x18) Byte, Identifier of the Will Delay Interval.
            pub will_delay_interval: Option<u32>,
        ));
    );
    ( @ $(#[$attr:meta])*  $name:ident { RequestResponseInformation, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $(#[$attr])*  $name { $($rest)* } -> (
            $($result)*
            /// 3.1.2.11.6 Request Response Information
            /// 25 (0x19) Byte, Identifier of the Request Response Information
            pub request_response_information: Option<u8>,
        ));
    );
    ( @ $(#[$attr:meta])*  $name:ident { ResponseInformation, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $(#[$attr])*  $name { $($rest)* } -> (
            $($result)*
            /// 3.2.2.3.15 Response Information
            /// 26 (0x1A) Byte, Identifier of the Response Information.
            pub response_info: Option<Box<str>>,
        ));
    );
    ( @ $(#[$attr:meta])*  $name:ident { ServerReference, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $(#[$attr])*  $name { $($rest)* } -> (
            $($result)*
            /// 3.2.2.3.16 Server Reference
            /// 28 (0x1C) Byte, Identifier of the Server Reference
            pub server_reference: Option<Box<str>>,
        ));
    );
    ( @ $(#[$attr:meta])*  $name:ident { ReasonString, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $(#[$attr])*  $name { $($rest)* } -> (
            $($result)*
            /// 3.2.2.3.9 Reason String
            /// 31 (0x1F) Byte Identifier of the Reason String.
            pub reason_string: Option<Box<str>>,
        ));
    );
    ( @ $(#[$attr:meta])*  $name:ident { ReceiveMaximum, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $(#[$attr])*  $name { $($rest)* } -> (
            $($result)*
            /// 3.2.2.3.3 Receive Maximum
            /// 33 (0x21) Byte, Identifier of the Receive Maximum
            pub receive_maximum: Option<u16>,
        ));
    );
    ( @ $(#[$attr:meta])*  $name:ident { TopicAliasMaximum, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $(#[$attr])*  $name { $($rest)* } -> (
            $($result)*
            /// 3.2.2.3.8 Topic Alias Maximum
            /// 34 (0x22) Byte, Identifier of the Topic Alias Maximum.
            pub topic_alias_maximum: Option<u16>,
        ));
    );
    ( @ $(#[$attr:meta])*  $name:ident { TopicAlias, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $(#[$attr])*  $name { $($rest)* } -> (
            $($result)*
            /// 3.3.2.3.4 Topic Alias
            /// 35 (0x23) Byte, Identifier of the Topic Alias.
            pub topic_alias: Option<u16>,
        ));
    );
    ( @ $(#[$attr:meta])*  $name:ident { MaximumQos, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $(#[$attr])*  $name { $($rest)* } -> (
            $($result)*
            /// 3.2.2.3.4 Maximum QoS
            /// 36 (0x24) Byte, Identifier of the Maximum QoS.
            pub maximum_qos: Option<$crate::packets::QoS>,
        ));
    );
    ( @ $(#[$attr:meta])*  $name:ident { RetainAvailable, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $(#[$attr])*  $name { $($rest)* } -> (
            $($result)*
            /// 3.2.2.3.5 Retain Available
            /// 37 (0x25) Byte, Identifier of Retain Available.
            pub retain_available: Option<bool>,
        ));
    );
    ( @ $(#[$attr:meta])*  $name:ident { UserProperty, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $(#[$attr])*  $name { $($rest)* } -> (
            $($result)*
            /// 3.2.2.3.10 User Property
            /// 38 (0x26) Byte, Identifier of User Property.
            pub user_properties: Vec<(Box<str>, Box<str>)>,
        ));
    );
    ( @ $(#[$attr:meta])*  $name:ident { MaximumPacketSize, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $(#[$attr])*  $name { $($rest)* } -> (
            $($result)*
            /// 3.2.2.3.6 Maximum Packet Size
            /// 39 (0x27) Byte, Identifier of the Maximum Packet Size.
            pub maximum_packet_size: Option<u32>,
        ));
    );
    ( @ $(#[$attr:meta])*  $name:ident { WildcardSubscriptionAvailable, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $(#[$attr])*  $name { $($rest)* } -> (
            $($result)*
            /// 3.2.2.3.11 Wildcard Subscription Available
            /// 40 (0x28) Byte, Identifier of Wildcard Subscription Available.
            pub wildcards_available: Option<bool>,
        ));
    );
    ( @ $(#[$attr:meta])*  $name:ident { SubscriptionIdentifierAvailable, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $(#[$attr])*  $name { $($rest)* } -> (
            $($result)*
            /// 3.2.2.3.12 Subscription Identifiers Available
            /// 41 (0x29) Byte, Identifier of Subscription Identifier Available.
            pub subscription_ids_available: Option<bool>,
        ));
    );
    ( @ $(#[$attr:meta])*  $name:ident { SharedSubscriptionAvailable, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::properties_struct!(@ $(#[$attr])*  $name { $($rest)* } -> (
            $($result)*
            /// 3.2.2.3.13 Shared Subscription Available
            /// 42 (0x2A) Byte, Identifier of Shared Subscription Available.
            pub shared_subscription_available: Option<bool>,
        ));
    );
    ( @ $(#[$attr:meta])*  $name:ident { $unknown:ident, $($rest:tt)* } -> ($($result:tt)*) ) => (
        compile_error!(concat!("Unknown property: ", stringify!($unknown)));
    );
}

macro_rules! properties_read_match_branch_body {
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::PayloadFormatIndicator) => {{
        if $properties.payload_format_indicator.is_some() {
            return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(
                PropertyType::PayloadFormatIndicator,
            )));
        }
        let (prop_body, read_bytes) = u8::async_read($stream).await?;
        $read_property_bytes += read_bytes;
        $properties.payload_format_indicator = Some(prop_body);
    }};
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::MessageExpiryInterval) => {{
        if $properties.message_expiry_interval.is_some() {
            return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(
                PropertyType::MessageExpiryInterval,
            )));
        }
        let (prop_body, read_bytes) = u32::async_read($stream).await?;
        $read_property_bytes += read_bytes;
        $properties.message_expiry_interval = Some(prop_body);
    }};
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::ContentType) => {{
        if $properties.content_type.is_some() {
            return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(PropertyType::ContentType)));
        }
        let (prop_body, read_bytes) = Box::<str>::async_read($stream).await?;
        $read_property_bytes += read_bytes;
        $properties.content_type = Some(prop_body);
    }};
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::ResponseTopic) => {{
        if $properties.response_topic.is_some() {
            return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(PropertyType::ResponseTopic)));
        }
        let (prop_body, read_bytes) = Box::<str>::async_read($stream).await?;
        $read_property_bytes += read_bytes;
        $properties.response_topic = Some(prop_body);
    }};
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::CorrelationData) => {{
        if $properties.correlation_data.is_some() {
            return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(PropertyType::CorrelationData)));
        }
        let (prop_body, read_bytes) = Vec::<u8>::async_read($stream).await?;
        $read_property_bytes += read_bytes;
        $properties.correlation_data = Some(prop_body);
    }};
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::SubscriptionIdentifier) => {{
        let (prop_body, read_bytes) = <u32 as $crate::packets::primitive::VariableInteger>::read_async_variable_integer($stream).await?;
        $read_property_bytes += read_bytes;
        $properties.subscription_identifier = Some(prop_body as u32);
    }};
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::ListSubscriptionIdentifier) => {{
        let (prop_body, read_bytes) = <u32 as $crate::packets::primitive::VariableInteger>::read_async_variable_integer($stream).await?;
        $read_property_bytes += read_bytes;
        $properties.subscription_identifiers.push(prop_body as u32);
    }};
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::SessionExpiryInterval) => {{
        if $properties.session_expiry_interval.is_some() {
            return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(
                PropertyType::SessionExpiryInterval,
            )));
        }
        let (prop_body, read_bytes) = u32::async_read($stream).await?;
        $read_property_bytes += read_bytes;
        $properties.session_expiry_interval = Some(prop_body);
    }};
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::AssignedClientIdentifier) => {{
        if $properties.assigned_client_identifier.is_some() {
            return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(
                PropertyType::AssignedClientIdentifier,
            )));
        }
        let (prop_body, read_bytes) = Box::<str>::async_read($stream).await?;
        $read_property_bytes += read_bytes;
        $properties.assigned_client_identifier = Some(prop_body);
    }};
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::ServerKeepAlive) => {{
        if $properties.server_keep_alive.is_some() {
            return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(PropertyType::ServerKeepAlive)));
        }
        let (prop_body, read_bytes) = u16::async_read($stream).await?;
        $read_property_bytes += read_bytes;
        $properties.server_keep_alive = Some(prop_body);
    }};
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::AuthenticationMethod) => {{
        if $properties.authentication_method.is_some() {
            return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(
                PropertyType::AuthenticationMethod,
            )));
        }
        let (prop_body, read_bytes) = Box::<str>::async_read($stream).await?;
        $read_property_bytes += read_bytes;
        $properties.authentication_method = Some(prop_body);
    }};
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::AuthenticationData) => {{
        if $properties.authentication_data.is_some() {
            return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(
                PropertyType::AuthenticationData,
            )));
        }
        let (prop_body, read_bytes) = Vec::<u8>::async_read($stream).await?;
        $read_property_bytes += read_bytes;
        $properties.authentication_data = Some(prop_body);
    }};
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::RequestResponseInformation) => {{
        if $properties.authentication_data.is_some() {
            return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(
                PropertyType::RequestResponseInformation,
            )));
        }
        let (prop_body, read_bytes) = u8::async_read($stream).await?;
        $read_property_bytes += read_bytes;
        $properties.request_problem_information = Some(prop_body);
    }};
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::RequestProblemInformation) => {{
        if $properties.authentication_data.is_some() {
            return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(
                PropertyType::RequestProblemInformation,
            )));
        }
        let (prop_body, read_bytes) = u8::async_read($stream).await?;
        $read_property_bytes += read_bytes;
        $properties.request_problem_information = Some(prop_body);
    }};
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::WillDelayInterval) => {{
        if $properties.will_delay_interval.is_some() {
            return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(
                PropertyType::WillDelayInterval,
            )));
        }
        let (prop_body, read_bytes) = u32::async_read($stream).await?;
        $read_property_bytes += read_bytes;
        $properties.will_delay_interval = Some(prop_body);
    }};
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::ResponseInformation) => {{
        if $properties.response_info.is_some() {
            return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(
                PropertyType::ResponseInformation,
            )));
        }
        let (prop_body, read_bytes) = Box::<str>::async_read($stream).await?;
        $read_property_bytes += read_bytes;
        $properties.response_info = Some(prop_body);
    }};
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::ServerReference) => {{
        if $properties.server_reference.is_some() {
            return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(PropertyType::ServerReference)));
        }
        let (prop_body, read_bytes) = Box::<str>::async_read($stream).await?;
        $read_property_bytes += read_bytes;
        $properties.server_reference = Some(prop_body);
    }};
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::ReasonString) => {{
        if $properties.reason_string.is_some() {
            return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(PropertyType::ReasonString)));
        }
        let (prop_body, read_bytes) = Box::<str>::async_read($stream).await?;
        $read_property_bytes += read_bytes;
        $properties.reason_string = Some(prop_body);
    }};
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::ReceiveMaximum) => {{
        if $properties.receive_maximum.is_some() {
            return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(PropertyType::ReceiveMaximum)));
        }
        let (prop_body, read_bytes) = u16::async_read($stream).await?;
        $read_property_bytes += read_bytes;
        $properties.receive_maximum = Some(prop_body);
    }};
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::TopicAliasMaximum) => {{
        if $properties.topic_alias_maximum.is_some() {
            return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(
                PropertyType::TopicAliasMaximum,
            )));
        }
        let (prop_body, read_bytes) = u16::async_read($stream).await?;
        $read_property_bytes += read_bytes;
        $properties.topic_alias_maximum = Some(prop_body);
    }};
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::TopicAlias) => {{
        if $properties.topic_alias.is_some() {
            return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(
                PropertyType::MessageExpiryInterval,
            )));
        }
        let (prop_body, read_bytes) = u16::async_read($stream).await?;
        $read_property_bytes += read_bytes;
        $properties.topic_alias = Some(prop_body);
    }};
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::MaximumQos) => {{
        if $properties.maximum_qos.is_some() {
            return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(PropertyType::MaximumQos)));
        }
        let (prop_body, read_bytes) = $crate::packets::QoS::async_read($stream).await?;
        $read_property_bytes += read_bytes;
        $properties.maximum_qos = Some(prop_body);
    }};
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::RetainAvailable) => {{
        if $properties.retain_available.is_some() {
            return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(PropertyType::RetainAvailable)));
        }
        let (prop_body, read_bytes) = bool::async_read($stream).await?;
        $read_property_bytes += read_bytes;
        $properties.retain_available = Some(prop_body);
    }};
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::UserProperty) => {{
        let (prop_body_key, read_bytes) = Box::<str>::async_read($stream).await?;
        $read_property_bytes += read_bytes;
        let (prop_body_value, read_bytes) = Box::<str>::async_read($stream).await?;
        $read_property_bytes += read_bytes;

        $properties.user_properties.push((prop_body_key, prop_body_value))
    }};
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::MaximumPacketSize) => {{
        if $properties.maximum_packet_size.is_some() {
            return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(PropertyType::RetainAvailable)));
        }
        let (prop_body, read_bytes) = u32::async_read($stream).await?;
        $read_property_bytes += read_bytes;
        $properties.maximum_packet_size = Some(prop_body);
    }};
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::WildcardSubscriptionAvailable) => {{
        if $properties.wildcards_available.is_some() {
            return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(
                PropertyType::WildcardSubscriptionAvailable,
            )));
        }
        let (prop_body, read_bytes) = bool::async_read($stream).await?;
        $read_property_bytes += read_bytes;
        $properties.wildcards_available = Some(prop_body);
    }};
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::SubscriptionIdentifierAvailable) => {{
        if $properties.subscription_ids_available.is_some() {
            return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(
                PropertyType::SubscriptionIdentifierAvailable,
            )));
        }
        let (prop_body, read_bytes) = bool::async_read($stream).await?;
        $read_property_bytes += read_bytes;
        $properties.subscription_ids_available = Some(prop_body);
    }};
    ($stream:ident, $properties:ident, $read_property_bytes:ident, PropertyType::SharedSubscriptionAvailable) => {{
        if $properties.shared_subscription_available.is_some() {
            return Err($crate::packets::error::ReadError::DeserializeError(DeserializeError::DuplicateProperty(
                PropertyType::SharedSubscriptionAvailable,
            )));
        }
        let (prop_body, read_bytes) = bool::async_read($stream).await?;
        $read_property_bytes += read_bytes;
        $properties.shared_subscription_available = Some(prop_body);
    }};
}

macro_rules! properties_read_match_branch_name {
    (ListSubscriptionIdentifier) => {
        PropertyType::SubscriptionIdentifier
    };
    ($name:ident) => {
        PropertyType::$name
    };
}

macro_rules! properties_write {
    ($self:ident, $bytes_writen:ident, $stream:ident, PropertyType::PayloadFormatIndicator) => {
        if let Some(payload_format_indicator) = &($self.payload_format_indicator) {
            $bytes_writen += PropertyType::PayloadFormatIndicator.async_write($stream).await?;
            $bytes_writen += payload_format_indicator.async_write($stream).await?;
        }
    };
    ($self:ident, $bytes_writen:ident, $stream:ident, PropertyType::MessageExpiryInterval) => {
        if let Some(message_expiry_interval) = &($self.message_expiry_interval) {
            $bytes_writen += PropertyType::MessageExpiryInterval.async_write($stream).await?;
            $bytes_writen += message_expiry_interval.async_write($stream).await?;
        }
    };
    ($self:ident, $bytes_writen:ident, $stream:ident, PropertyType::ContentType) => {
        if let Some(content_type) = &($self.content_type) {
            $bytes_writen += PropertyType::ContentType.async_write($stream).await?;
            $bytes_writen += content_type.async_write($stream).await?;
        }
    };
    ($self:ident, $bytes_writen:ident, $stream:ident, PropertyType::ResponseTopic) => {
        if let Some(response_topic) = &($self.response_topic) {
            $bytes_writen += PropertyType::ResponseTopic.async_write($stream).await?;
            $bytes_writen += response_topic.as_ref().async_write($stream).await?;
        }
    };
    ($self:ident, $bytes_writen:ident, $stream:ident, PropertyType::CorrelationData) => {
        if let Some(correlation_data) = &($self.correlation_data) {
            $bytes_writen += PropertyType::CorrelationData.async_write($stream).await?;
            $bytes_writen += correlation_data.async_write($stream).await?;
        }
    };
    ($self:ident, $bytes_writen:ident, $stream:ident, PropertyType::SubscriptionIdentifier) => {
        if let Some(sub_id) = &($self.subscription_identifier) {
            $bytes_writen += PropertyType::SubscriptionIdentifier.async_write($stream).await?;
            $bytes_writen += $crate::packets::primitive::VariableInteger::write_async_variable_integer(sub_id, $stream).await?;
        }
    };
    ($self:ident, $bytes_writen:ident, $stream:ident, PropertyType::ListSubscriptionIdentifier) => {
        for sub_id in &($self.subscription_identifiers) {
            $bytes_writen += PropertyType::SubscriptionIdentifier.async_write($stream).await?;
            $bytes_writen += $crate::packets::primitive::VariableInteger::write_async_variable_integer(sub_id, $stream).await?;
        }
    };
    ($self:ident, $bytes_writen:ident, $stream:ident, PropertyType::SessionExpiryInterval) => {
        if let Some(session_expiry_interval) = &($self.session_expiry_interval) {
            $bytes_writen += PropertyType::SessionExpiryInterval.async_write($stream).await?;
            $bytes_writen += session_expiry_interval.async_write($stream).await?;
        }
    };
    ($self:ident, $bytes_writen:ident, $stream:ident, PropertyType::AssignedClientIdentifier) => {};
    ($self:ident, $bytes_writen:ident, $stream:ident, PropertyType::ServerKeepAlive) => {
        if let Some(server_keep_alive) = &($self.server_keep_alive) {
            $bytes_writen += PropertyType::ServerKeepAlive.async_write($stream).await?;
            $bytes_writen += server_keep_alive.async_write($stream).await?;
        }
    };
    ($self:ident, $bytes_writen:ident, $stream:ident, PropertyType::AuthenticationMethod) => {
        if let Some(authentication_method) = &($self.authentication_method) {
            $bytes_writen += PropertyType::AuthenticationMethod.async_write($stream).await?;
            $bytes_writen += authentication_method.async_write($stream).await?;
        }
    };
    ($self:ident, $bytes_writen:ident, $stream:ident, PropertyType::AuthenticationData) => {
        if let Some(authentication_data) = &($self.authentication_data) {
            if !authentication_data.is_empty() && ($self.authentication_method).is_some() {
                $bytes_writen += PropertyType::AuthenticationData.async_write($stream).await?;
                $bytes_writen += authentication_data.async_write($stream).await?;
            }
        }
    };
    ($self:ident, $bytes_writen:ident, $stream:ident, PropertyType::RequestProblemInformation) => {
        if let Some(request_problem_information) = &($self.request_problem_information) {
            $bytes_writen += PropertyType::RequestProblemInformation.async_write($stream).await?;
            $bytes_writen += request_problem_information.async_write($stream).await?;
        }
    };
    ($self:ident, $bytes_writen:ident, $stream:ident, PropertyType::WillDelayInterval) => {
        if let Some(delay_interval) = &($self.will_delay_interval) {
            $bytes_writen += PropertyType::WillDelayInterval.async_write($stream).await?;
            $bytes_writen += delay_interval.async_write($stream).await?;
        }
    };
    ($self:ident, $bytes_writen:ident, $stream:ident, PropertyType::RequestResponseInformation) => {
        if let Some(request_response_information) = &($self.request_response_information) {
            $bytes_writen += PropertyType::RequestResponseInformation.async_write($stream).await?;
            $bytes_writen += request_response_information.async_write($stream).await?;
        }
    };
    ($self:ident, $bytes_writen:ident, $stream:ident, PropertyType::ResponseInformation) => {
        if let Some(response_info) = &($self.response_info) {
            $bytes_writen += PropertyType::ResponseInformation.async_write($stream).await?;
            $bytes_writen += response_info.async_write($stream).await?;
        }
    };
    ($self:ident, $bytes_writen:ident, $stream:ident, PropertyType::ServerReference) => {
        if let Some(server_refrence) = &($self.server_reference) {
            $bytes_writen += PropertyType::ServerReference.async_write($stream).await?;
            server_refrence.async_write($stream).await?;
        }
    };
    ($self:ident, $bytes_writen:ident, $stream:ident, PropertyType::ReasonString) => {
        if let Some(reason_string) = &($self.reason_string) {
            $bytes_writen += PropertyType::ReasonString.async_write($stream).await?;
            $bytes_writen += reason_string.async_write($stream).await?;
        }
    };
    ($self:ident, $bytes_writen:ident, $stream:ident, PropertyType::ReceiveMaximum) => {
        if let Some(receive_maximum) = &($self.receive_maximum) {
            $bytes_writen += PropertyType::ReceiveMaximum.async_write($stream).await?;
            $bytes_writen += receive_maximum.async_write($stream).await?;
        }
    };
    ($self:ident, $bytes_writen:ident, $stream:ident, PropertyType::TopicAliasMaximum) => {
        if let Some(topic_alias_maximum) = &($self.topic_alias_maximum) {
            $bytes_writen += PropertyType::TopicAliasMaximum.async_write($stream).await?;
            $bytes_writen += topic_alias_maximum.async_write($stream).await?;
        }
    };
    ($self:ident, $bytes_writen:ident, $stream:ident, PropertyType::TopicAlias) => {
        if let Some(topic_alias) = &($self.topic_alias) {
            $bytes_writen += PropertyType::TopicAlias.async_write($stream).await?;
            $bytes_writen += topic_alias.async_write($stream).await?;
        }
    };
    ($self:ident, $bytes_writen:ident, $stream:ident, PropertyType::MaximumQos) => {
        if let Some(maximum_qos) = &($self.maximum_qos) {
            $bytes_writen += PropertyType::MaximumQos.async_write($stream).await?;
            $bytes_writen += maximum_qos.async_write($stream).await?;
        }
    };
    ($self:ident, $bytes_writen:ident, $stream:ident, PropertyType::RetainAvailable) => {
        if let Some(retain_available) = &($self.retain_available) {
            $bytes_writen += PropertyType::RetainAvailable.async_write($stream).await?;
            $bytes_writen += retain_available.async_write($stream).await?;
        }
    };
    ($self:ident, $bytes_writen:ident, $stream:ident, PropertyType::UserProperty) => {
        for (key, value) in &($self.user_properties) {
            $bytes_writen += PropertyType::UserProperty.async_write($stream).await?;
            $bytes_writen += key.async_write($stream).await?;
            $bytes_writen += value.async_write($stream).await?;
        }
    };
    ($self:ident, $bytes_writen:ident, $stream:ident, PropertyType::MaximumPacketSize) => {
        if let Some(maximum_packet_size) = &($self.maximum_packet_size) {
            $bytes_writen += PropertyType::MaximumPacketSize.async_write($stream).await?;
            $bytes_writen += maximum_packet_size.async_write($stream).await?;
        }
    };
    ($self:ident, $bytes_writen:ident, $stream:ident, PropertyType::WildcardSubscriptionAvailable) => {
        if let Some(wildcards_available) = &($self.wildcards_available) {
            $bytes_writen += PropertyType::WildcardSubscriptionAvailable.async_write($stream).await?;
            $bytes_writen += wildcards_available.async_write($stream).await?;
        }
    };
    ($self:ident, $bytes_writen:ident, $stream:ident, PropertyType::SubscriptionIdentifierAvailable) => {
        if let Some(subscription_ids_available) = &($self.subscription_ids_available) {
            $bytes_writen += PropertyType::SubscriptionIdentifierAvailable.async_write($stream).await?;
            $bytes_writen += subscription_ids_available.async_write($stream).await?;
        }
    };
    ($self:ident, $bytes_writen:ident, $stream:ident, PropertyType::SharedSubscriptionAvailable) => {
        if let Some(shared_subscription_available) = &($self.shared_subscription_available) {
            $bytes_writen += PropertyType::SharedSubscriptionAvailable.async_write($stream).await?;
            $bytes_writen += shared_subscription_available.async_write($stream).await?;
        }
    };
    ($self:ident, $bytes_writen:ident, $stream:ident, $unknown:ident) => {
        compile_error!(concat!("Unknown property: ", stringify!($unknown)));
    };
}

macro_rules! properties_wire_length {
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
        if let Some(sub_id) = &($self.subscription_identifier) {
            $len += 1 + crate::packets::primitive::VariableInteger::variable_integer_len(sub_id);
        }
    };
    ($self:ident, $len:ident, PropertyType::ListSubscriptionIdentifier) => {
        for sub_id in &($self.subscription_identifiers) {
            $len += 1 + crate::packets::primitive::VariableInteger::variable_integer_len(sub_id);
        }
    };
    ($self:ident, $len:ident, PropertyType::SessionExpiryInterval) => {
        if $self.session_expiry_interval.is_some() {
            $len += 1 + 4;
        }
    };
    ($self:ident, $len:ident, PropertyType::AssignedClientIdentifier) => {
        if let Some(client_id) = $self.assigned_client_identifier.as_ref() {
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
        if let Some(authentication_data) = &($self).authentication_data {
            if !authentication_data.is_empty() && $self.authentication_method.is_some() {
                $len += 1 + authentication_data.wire_len();
            }
        }
    };
    ($self:ident, $len:ident, PropertyType::RequestProblemInformation) => {
        if $self.request_problem_information.is_some() {
            $len += 2;
        }
    };
    ($self:ident, $len:ident, PropertyType::WillDelayInterval) => {
        if $self.will_delay_interval.is_some() {
            $len += 5;
        }
    };
    ($self:ident, $len:ident, PropertyType::RequestResponseInformation) => {
        if $self.request_response_information.is_some() {
            $len += 2;
        }
    };
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
    ($self:ident, $len:ident, $unknown:ident) => {
        compile_error!(concat!("Unknown property: ", stringify!($unknown)));
    };
}

pub(crate) use define_properties;
pub(crate) use properties_read_match_branch_body;
pub(crate) use properties_read_match_branch_name;
pub(crate) use properties_struct;
pub(crate) use properties_wire_length;
pub(crate) use properties_write;
