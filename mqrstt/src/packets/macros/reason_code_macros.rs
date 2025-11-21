macro_rules! reason_code {
    ($name:ident, $($code:ident),*) => {
        use tokio::io::AsyncReadExt;
        #[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $name {
            #[default]
            $(
                $code = $crate::packets::macros::reason_code_internal!($code),
            )*
        }

        impl $name {
            #[inline]
            pub(crate) const fn from_u8(val: u8) -> Result<Self, $crate::packets::error::DeserializeError> {
                $crate::packets::macros::reason_code_match!(@ $name, val, {
                    $($code,)*
                } -> ())
            }

            #[inline]
            pub(crate) const fn to_u8(self) -> u8 {
                $crate::packets::macros::reason_code_match_write!(@ $name, self, {
                    $($code,)*
                } -> ())
            }
        }

        impl<S> $crate::packets::mqtt_trait::MqttAsyncRead<S> for $name where S: tokio::io::AsyncRead + std::marker::Unpin{
            async fn async_read(stream: &mut S) -> Result<(Self, usize), $crate::packets::error::ReadError> {
                let input = stream.read_u8().await?;
                let res = Self::from_u8(input)?;
                Ok((res, 1))
            }
        }

        impl $crate::packets::mqtt_trait::MqttRead for $name {
            fn read(buf: &mut bytes::Bytes) -> Result<Self, $crate::packets::error::DeserializeError> {
                if buf.is_empty() {
                    return Err($crate::packets::error::DeserializeError::InsufficientData(std::any::type_name::<Self>(), 0, 1));
                }
                use bytes::Buf;
                let input = buf.get_u8();
                Self::from_u8(input)
            }
        }

        impl<S> $crate::packets::mqtt_trait::MqttAsyncWrite<S> for $name where S: tokio::io::AsyncWrite + std::marker::Unpin{
            async fn async_write(&self, stream: &mut S) -> Result<usize, $crate::packets::error::WriteError> {
                use tokio::io::AsyncWriteExt;
                let val = self.to_u8();
                stream.write_u8(val).await?;
                Ok(1)
            }
        }

        impl $crate::packets::mqtt_trait::MqttWrite for $name {
            fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), $crate::packets::error::SerializeError> {
                use bytes::BufMut;
                let val = self.to_u8();
                buf.put_u8(val);
                Ok(())
            }
        }

    };
}

macro_rules! reason_code_match {
    ( @ $name:ident, $input:ident, { } -> ($($result:tt)*) ) => (
        match $input {
            $($result)*
            t => Err($crate::packets::error::DeserializeError::UnknownProperty(t)),
        }
    );

    ( @ $name:ident, $input:ident, { $variant:ident, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            $crate::packets::macros::reason_code_internal!($variant) => Ok($name::$variant),
        ))
    );

    ( @ $name:ident, $input:ident, { $unknown:ident, $($rest:tt)* } -> ($($result:tt)*) ) => (
        compile_error!(concat!("Unknown reason_code: ", stringify!($unknown)))
    );
}

#[macro_export]
macro_rules! reason_code_match_write {
    ( @ $name:ident, $input:ident, { } -> ($($result:tt)*) ) => (
        match $input {
            $($result)*
        }
    );

    ( @ $name:ident, $input:ident, { $variant:ident, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            $name::$variant => $crate::packets::macros::reason_code_internal!($variant),
        ))
    );

    ( @ $name:ident, $input:ident, { $unknown:ident, $($rest:tt)* } -> ($($result:tt)*) ) => (
        compile_error!(concat!("Unknown reason_code: ", stringify!($unknown)))
    );
}

macro_rules! reason_code_internal {
    (Success) => {
        0x00
    };
    (NormalDisconnection) => {
        0x00
    };
    (GrantedQoS0) => {
        0x00
    };
    (GrantedQoS1) => {
        0x01
    };
    (GrantedQoS2) => {
        0x02
    };
    (DisconnectWithWillMessage) => {
        0x04
    };
    (NoMatchingSubscribers) => {
        0x10
    };
    (NoSubscriptionExisted) => {
        0x11
    };
    (ContinueAuthentication) => {
        0x18
    };
    (ReAuthenticate) => {
        0x19
    };
    (UnspecifiedError) => {
        0x80
    };
    (MalformedPacket) => {
        0x81
    };
    (ProtocolError) => {
        0x82
    };
    (ImplementationSpecificError) => {
        0x83
    };
    (UnsupportedProtocolVersion) => {
        0x84
    };
    (ClientIdentifierNotValid) => {
        0x85
    };
    (BadUsernameOrPassword) => {
        0x86
    };
    (NotAuthorized) => {
        0x87
    };
    (ServerUnavailable) => {
        0x88
    };
    (ServerBusy) => {
        0x89
    };
    (Banned) => {
        0x8A
    };
    (ServerShuttingDown) => {
        0x8B
    };
    (BadAuthenticationMethod) => {
        0x8C
    };
    (KeepAliveTimeout) => {
        0x8D
    };
    (SessionTakenOver) => {
        0x8E
    };
    (TopicFilterInvalid) => {
        0x8F
    };
    (TopicNameInvalid) => {
        0x90
    };
    (PacketIdentifierInUse) => {
        0x91
    };
    (PacketIdentifierNotFound) => {
        0x92
    };
    (ReceiveMaximumExceeded) => {
        0x93
    };
    (TopicAliasInvalid) => {
        0x94
    };
    (PacketTooLarge) => {
        0x95
    };
    (MessageRateTooHigh) => {
        0x96
    };
    (QuotaExceeded) => {
        0x97
    };
    (AdministrativeAction) => {
        0x98
    };
    (PayloadFormatInvalid) => {
        0x99
    };
    (RetainNotSupported) => {
        0x9A
    };
    (QosNotSupported) => {
        0x9B
    };
    (UseAnotherServer) => {
        0x9C
    };
    (ServerMoved) => {
        0x9D
    };
    (SharedSubscriptionsNotSupported) => {
        0x9E
    };
    (ConnectionRateExceeded) => {
        0x9F
    };
    (MaximumConnectTime) => {
        0xA0
    };
    (SubscriptionIdentifiersNotSupported) => {
        0xA1
    };
    (WildcardSubscriptionsNotSupported) => {
        0xA2
    };
    ($unknown:ident) => {
        compile_error!(concat!("Unknown reason_code: ", stringify!($unknown)))
    };
}

pub(crate) use reason_code;
pub(crate) use reason_code_internal;
pub(crate) use reason_code_match;
pub(crate) use reason_code_match_write;
