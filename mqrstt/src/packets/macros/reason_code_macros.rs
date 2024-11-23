macro_rules! reason_code {
    ($name:ident, $($code:ident),*) => {
        use tokio::io::AsyncReadExt;
        #[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $name {
            #[default]
            $($code),*
        }

        impl<S> $crate::packets::mqtt_trait::MqttAsyncRead<S> for $name where S: tokio::io::AsyncRead + std::marker::Unpin{
            async fn async_read(stream: &mut S) -> Result<(Self, usize), $crate::packets::error::ReadError> {
                let input = stream.read_u8().await?;
                let res = $crate::packets::macros::reason_code_match!(@ $name, input, {
                    $($code,)*
                } -> ())?;
                Ok((res, 1))
            }
        }

        impl $crate::packets::mqtt_trait::MqttRead for $name {
            fn read(buf: &mut bytes::Bytes) -> Result<Self, $crate::packets::error::DeserializeError> {
                if buf.is_empty() {
                    return Err($crate::packets::error::DeserializeError::InsufficientData(std::any::type_name::<Self>(), 0, 1));
                }
                use bytes::Buf;
                let res = buf.get_u8();
                $crate::packets::macros::reason_code_match!(@ $name, res, {
                    $($code,)*
                } -> ())
            }
        }

        impl $crate::packets::mqtt_trait::MqttWrite for $name {
            fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), $crate::packets::error::SerializeError> {
                let val = $crate::packets::macros::reason_code_match_write!(@ $name, buf, self, {
                    $($code,)*
                } -> ());
                use bytes::BufMut;
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
    ( @ $name:ident, $input:ident, { Success, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x00 => Ok($name::Success),
        ))
    );
    ( @ $name:ident, $input:ident, { NormalDisconnection, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x00 => Ok($name::NormalDisconnection),
        ))
    );
    ( @ $name:ident, $input:ident, { GrantedQoS0, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x00 => Ok($name::GrantedQoS0),
        ))
    );
    ( @ $name:ident, $input:ident, { GrantedQoS1, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x01 => Ok($name::GrantedQoS1),
        ))
    );
    ( @ $name:ident, $input:ident, { GrantedQoS2, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x02 => Ok($name::GrantedQoS2),
        ))
    );
    ( @ $name:ident, $input:ident, { DisconnectWithWillMessage, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x04 => Ok($name::DisconnectWithWillMessage),
        ))
    );
    ( @ $name:ident, $input:ident, { NoMatchingSubscribers, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x10 => Ok($name::NoMatchingSubscribers),
        ))
    );
    ( @ $name:ident, $input:ident, { NoSubscriptionExisted, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x11 => Ok($name::NoSubscriptionExisted),
        ))
    );
    ( @ $name:ident, $input:ident, { ContinueAuthentication, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x18 => Ok($name::ContinueAuthentication),
        ))
    );
    ( @ $name:ident, $input:ident, { ReAuthenticate, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x19 => Ok($name::ReAuthenticate),
        ))
    );
    ( @ $name:ident, $input:ident, { UnspecifiedError, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x80 => Ok($name::UnspecifiedError),
        ))
    );
    ( @ $name:ident, $input:ident, { MalformedPacket, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x81 => Ok($name::MalformedPacket),
        ))
    );
    ( @ $name:ident, $input:ident, { ProtocolError, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x82 => Ok($name::ProtocolError),
        ))
    );
    ( @ $name:ident, $input:ident, { ImplementationSpecificError, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x83 => Ok($name::ImplementationSpecificError),
        ))
    );
    ( @ $name:ident, $input:ident, { UnsupportedProtocolVersion, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x84 => Ok($name::UnsupportedProtocolVersion),
        ))
    );
    ( @ $name:ident, $input:ident, { ClientIdentifierNotValid, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x85 => Ok($name::ClientIdentifierNotValid),
        ))
    );
    ( @ $name:ident, $input:ident, { BadUsernameOrPassword, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x86 => Ok($name::BadUsernameOrPassword),
        ))
    );
    ( @ $name:ident, $input:ident, { NotAuthorized, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x87 => Ok($name::NotAuthorized),
        ))
    );
    ( @ $name:ident, $input:ident, { ServerUnavailable, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x88 => Ok($name::ServerUnavailable),
        ))
    );
    ( @ $name:ident, $input:ident, { ServerBusy, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x89 => Ok($name::ServerBusy),
        ))
    );
    ( @ $name:ident, $input:ident, { Banned, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x8A => Ok($name::Banned),
        ))
    );
    ( @ $name:ident, $input:ident, { ServerShuttingDown, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x8B => Ok($name::ServerShuttingDown),
        ))
    );
    ( @ $name:ident, $input:ident, { BadAuthenticationMethod, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x8C => Ok($name::BadAuthenticationMethod),
        ))
    );
    ( @ $name:ident, $input:ident, { KeepAliveTimeout, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x8D => Ok($name::KeepAliveTimeout),
        ))
    );
    ( @ $name:ident, $input:ident, { SessionTakenOver, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x8E => Ok($name::SessionTakenOver),
        ))
    );
    ( @ $name:ident, $input:ident, { TopicFilterInvalid, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x8F => Ok($name::TopicFilterInvalid),
        ))
    );
    ( @ $name:ident, $input:ident, { TopicNameInvalid, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x90 => Ok($name::TopicNameInvalid),
        ))
    );
    ( @ $name:ident, $input:ident, { PacketIdentifierInUse, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x91 => Ok($name::PacketIdentifierInUse),
        ))
    );
    ( @ $name:ident, $input:ident, { PacketIdentifierNotFound, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x92 => Ok($name::PacketIdentifierNotFound),
        ))
    );
    ( @ $name:ident, $input:ident, { ReceiveMaximumExceeded, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x93 => Ok($name::ReceiveMaximumExceeded),
        ))
    );
    ( @ $name:ident, $input:ident, { TopicAliasInvalid, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x94 => Ok($name::TopicAliasInvalid),
        ))
    );
    ( @ $name:ident, $input:ident, { PacketTooLarge, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x95 => Ok($name::PacketTooLarge),
        ))
    );
    ( @ $name:ident, $input:ident, { MessageRateTooHigh, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x96 => Ok($name::MessageRateTooHigh),
        ))
    );
    ( @ $name:ident, $input:ident, { QuotaExceeded, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x97 => Ok($name::QuotaExceeded),
        ))
    );
    ( @ $name:ident, $input:ident, { AdministrativeAction, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x98 => Ok($name::AdministrativeAction),
        ))
    );
    ( @ $name:ident, $input:ident, { PayloadFormatInvalid, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x99 => Ok($name::PayloadFormatInvalid),
        ))
    );
    ( @ $name:ident, $input:ident, { RetainNotSupported, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x9A => Ok($name::RetainNotSupported),
        ))
    );
    ( @ $name:ident, $input:ident, { QosNotSupported, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x9B => Ok($name::QosNotSupported),
        ))
    );
    ( @ $name:ident, $input:ident, { UseAnotherServer, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x9C => Ok($name::UseAnotherServer),
        ))
    );
    ( @ $name:ident, $input:ident, { ServerMoved, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x9D => Ok($name::ServerMoved),
        ))
    );
    ( @ $name:ident, $input:ident, { SharedSubscriptionsNotSupported, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x9E => Ok($name::SharedSubscriptionsNotSupported),
        ))
    );
    ( @ $name:ident, $input:ident, { ConnectionRateExceeded, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0x9F => Ok($name::ConnectionRateExceeded),
        ))
    );
    ( @ $name:ident, $input:ident, { MaximumConnectTime, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0xA0 => Ok($name::MaximumConnectTime),
        ))
    );
    ( @ $name:ident, $input:ident, { SubscriptionIdentifiersNotSupported, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0xA1 => Ok($name::SubscriptionIdentifiersNotSupported),
        ))
    );
    ( @ $name:ident, $input:ident, { WildcardSubscriptionsNotSupported, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match!(@ $name, $input, { $($rest)* } -> (
            $($result)*
            0xA2 => Ok($name::WildcardSubscriptionsNotSupported),
        ))
    );
    ( @ $name:ident, $input:ident, { $unknown:ident, $($rest:tt)* } -> ($($result:tt)*) ) => (
        compile_error!(concat!("Unknown reason_code: ", stringify!($unknown)))
    );
}

macro_rules! reason_code_match_write{
    ( @ $name:ident, $buf:ident, $input:ident, { } -> ($($result:tt)*) ) => (
        match $input {
            $($result)*
        }
    );
    ( @ $name:ident, $buf:ident, $input:ident, { Success, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::Success => 0x00,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { NormalDisconnection, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::NormalDisconnection => 0x00,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { GrantedQoS0, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::GrantedQoS0 => 0x00,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { GrantedQoS1, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::GrantedQoS1 => 0x01,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { GrantedQoS2, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::GrantedQoS2 => 0x02,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { DisconnectWithWillMessage, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::DisconnectWithWillMessage => 0x04,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { NoMatchingSubscribers, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::NoMatchingSubscribers => 0x10,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { NoSubscriptionExisted, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::NoSubscriptionExisted => 0x11,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { ContinueAuthentication, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::ContinueAuthentication => 0x18,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { ReAuthenticate, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::ReAuthenticate => 0x19,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { UnspecifiedError, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::UnspecifiedError => 0x80,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { MalformedPacket, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::MalformedPacket => 0x81,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { ProtocolError, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::ProtocolError => 0x82,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { ImplementationSpecificError, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::ImplementationSpecificError => 0x83,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { UnsupportedProtocolVersion, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::UnsupportedProtocolVersion => 0x84,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { ClientIdentifierNotValid, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::ClientIdentifierNotValid => 0x85,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { BadUsernameOrPassword, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::BadUsernameOrPassword => 0x86,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { NotAuthorized, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::NotAuthorized => 0x87,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { ServerUnavailable, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::ServerUnavailable => 0x88,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { ServerBusy, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::ServerBusy => 0x89,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { Banned, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::Banned => 0x8A,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { ServerShuttingDown, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::ServerShuttingDown => 0x8B ,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { BadAuthenticationMethod, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::BadAuthenticationMethod => 0x8C,
        ))
    );

    ( @ $name:ident, $buf:ident, $input:ident, { KeepAliveTimeout, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::KeepAliveTimeout => 0x8D,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { SessionTakenOver, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::SessionTakenOver => 0x8E,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { TopicFilterInvalid, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::TopicFilterInvalid => 0x8F,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { TopicNameInvalid, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::TopicNameInvalid => 0x90,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { PacketIdentifierInUse, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::PacketIdentifierInUse => 0x91,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { PacketIdentifierNotFound, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::PacketIdentifierNotFound => 0x92,

        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { ReceiveMaximumExceeded, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::ReceiveMaximumExceeded => 0x93,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { TopicAliasInvalid, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::TopicAliasInvalid => 0x94,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { PacketTooLarge, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::PacketTooLarge => 0x95,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { MessageRateTooHigh, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::MessageRateTooHigh => 0x96,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { QuotaExceeded, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::QuotaExceeded => 0x97,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { AdministrativeAction, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::AdministrativeAction => 0x98,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { PayloadFormatInvalid, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::PayloadFormatInvalid => 0x99,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { RetainNotSupported, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::RetainNotSupported => 0x9A,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { QosNotSupported, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::QosNotSupported => 0x9B,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { UseAnotherServer, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::UseAnotherServer => 0x9C,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { ServerMoved, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::ServerMoved => 0x9D,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { SharedSubscriptionsNotSupported, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::SharedSubscriptionsNotSupported => 0x9E,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { ConnectionRateExceeded, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::ConnectionRateExceeded => 0x9F,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { MaximumConnectTime, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::MaximumConnectTime => 0xA0,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { SubscriptionIdentifiersNotSupported, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::SubscriptionIdentifiersNotSupported => 0xA1,
        ))
    );
    ( @ $name:ident, $buf:ident, $input:ident, { WildcardSubscriptionsNotSupported, $($rest:tt)* } -> ($($result:tt)*) ) => (
        $crate::packets::macros::reason_code_match_write!(@ $name, $buf, $input, { $($rest)* } -> (
            $($result)*
            $name::WildcardSubscriptionsNotSupported => 0xA2,
        ))
    );

    ( @ $name:ident, $buf:ident, $input:ident, { $unknown:ident, $($rest:tt)* } -> ($($result:tt)*) ) => (
        compile_error!(concat!("Unknown reason_code: ", stringify!($unknown)))
    );
}

pub(crate) use reason_code;
pub(crate) use reason_code_match;
pub(crate) use reason_code_match_write;
