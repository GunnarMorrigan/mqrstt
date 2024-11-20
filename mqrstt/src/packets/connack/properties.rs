use crate::packets::{
    error::{DeserializeError, SerializeError},
    mqtt_trait::{MqttAsyncRead, MqttRead, MqttWrite, PacketAsyncRead, PacketRead, PacketWrite, WireLength},
    PacketType, PropertyType, QoS, VariableInteger,
};
use bytes::BufMut;

crate::packets::macros::define_properties!(ConnAckProperties,
    SessionExpiryInterval,
    ReceiveMaximum,
    MaximumQos,
    RetainAvailable,
    MaximumPacketSize,
    AssignedClientIdentifier,
    TopicAliasMaximum,
    ReasonString,
    UserProperty,
    WildcardSubscriptionAvailable,
    SubscriptionIdentifierAvailable,
    SharedSubscriptionAvailable,
    ServerKeepAlive,
    ResponseInformation,
    ServerReference,
    AuthenticationMethod,
    AuthenticationData
);

// #[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
// pub struct ConnAckProperties {
//     /// 3.2.2.3.2 Session Expiry Interval
//     /// 17 (0x11) Byte Identifier of the Session Expiry Interval
//     pub session_expiry_interval: Option<u32>,

//     /// 3.2.2.3.3 Receive Maximum
//     /// 33 (0x21) Byte, Identifier of the Receive Maximum
//     pub receive_maximum: Option<u16>,

//     /// 3.2.2.3.4 Maximum QoS
//     /// 36 (0x24) Byte, Identifier of the Maximum QoS.
//     pub maximum_qos: Option<QoS>,

//     /// 3.2.2.3.5 Retain Available
//     /// 37 (0x25) Byte, Identifier of Retain Available.
//     pub retain_available: Option<bool>,

//     /// 3.2.2.3.6 Maximum Packet Size
//     /// 39 (0x27) Byte, Identifier of the Maximum Packet Size.
//     pub maximum_packet_size: Option<u32>,

//     /// 3.2.2.3.7 Assigned Client Identifier
//     /// 18 (0x12) Byte, Identifier of the Assigned Client Identifier.
//     pub assigned_client_id: Option<Box<str>>,

//     /// 3.2.2.3.8 Topic Alias Maximum
//     /// 34 (0x22) Byte, Identifier of the Topic Alias Maximum.
//     pub topic_alias_maximum: Option<u16>,

//     /// 3.2.2.3.9 Reason String
//     /// 31 (0x1F) Byte Identifier of the Reason String.
//     pub reason_string: Option<Box<str>>,

//     /// 3.2.2.3.10 User Property
//     /// 38 (0x26) Byte, Identifier of User Property.
//     pub user_properties: Vec<(Box<str>, Box<str>)>,

//     /// 3.2.2.3.11 Wildcard Subscription Available
//     /// 40 (0x28) Byte, Identifier of Wildcard Subscription Available.
//     pub wildcards_available: Option<bool>,

//     /// 3.2.2.3.12 Subscription Identifiers Available
//     /// 41 (0x29) Byte, Identifier of Subscription Identifier Available.
//     pub subscription_ids_available: Option<bool>,

//     /// 3.2.2.3.13 Shared Subscription Available
//     /// 42 (0x2A) Byte, Identifier of Shared Subscription Available.
//     pub shared_subscription_available: Option<bool>,

//     /// 3.2.2.3.14 Server Keep Alive
//     /// 19 (0x13) Byte, Identifier of the Server Keep Alive
//     pub server_keep_alive: Option<u16>,

//     /// 3.2.2.3.15 Response Information
//     /// 26 (0x1A) Byte, Identifier of the Response Information.
//     pub response_info: Option<Box<str>>,

//     /// 3.2.2.3.16 Server Reference
//     /// 28 (0x1C) Byte, Identifier of the Server Reference
//     pub server_reference: Option<Box<str>>,

//     /// 3.2.2.3.17 Authentication Method
//     /// 21 (0x15) Byte, Identifier of the Authentication Method
//     pub authentication_method: Option<Box<str>>,

//     /// 3.2.2.3.18 Authentication Data
//     /// 22 (0x16) Byte, Identifier of the Authentication Data
//     // There is a small inconsistency here with authentication_data in the connect packet.
//     // This is Option<Bytes> while that type uses just Bytes.
//     pub authentication_data: Option<Vec<u8>>,
// }

impl MqttRead for ConnAckProperties {
    fn read(buf: &mut bytes::Bytes) -> Result<Self, DeserializeError> {
        let (len, _) = VariableInteger::read_variable_integer(buf).map_err(DeserializeError::from)?;

        let mut properties = Self::default();
        if len == 0 {
            return Ok(properties);
        } else if buf.len() < len {
            return Err(DeserializeError::InsufficientData(std::any::type_name::<Self>(), buf.len(), len));
        }

        let mut property_data = buf.split_to(len);

        loop {
            let property = PropertyType::read(&mut property_data)?;
            match property {
                PropertyType::SessionExpiryInterval => {
                    if properties.session_expiry_interval.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::SessionExpiryInterval));
                    }
                    properties.session_expiry_interval = Some(u32::read(&mut property_data)?);
                }
                PropertyType::ReceiveMaximum => {
                    if properties.receive_maximum.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::ReceiveMaximum));
                    }
                    properties.receive_maximum = Some(u16::read(&mut property_data)?);
                }
                PropertyType::MaximumQos => {
                    if properties.maximum_qos.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::MaximumQos));
                    }
                    properties.maximum_qos = Some(QoS::read(&mut property_data)?);
                }
                PropertyType::RetainAvailable => {
                    if properties.retain_available.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::RetainAvailable));
                    }
                    properties.retain_available = Some(bool::read(&mut property_data)?);
                }
                PropertyType::MaximumPacketSize => {
                    if properties.maximum_packet_size.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::MaximumPacketSize));
                    }
                    properties.maximum_packet_size = Some(u32::read(&mut property_data)?);
                }
                PropertyType::AssignedClientIdentifier => {
                    if properties.assigned_client_id.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::AssignedClientIdentifier));
                    }
                    properties.assigned_client_id = Some(Box::<str>::read(&mut property_data)?);
                }
                PropertyType::TopicAliasMaximum => {
                    if properties.topic_alias_maximum.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::TopicAliasMaximum));
                    }
                    properties.topic_alias_maximum = Some(u16::read(&mut property_data)?);
                }
                PropertyType::ReasonString => {
                    if properties.reason_string.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::ReasonString));
                    }
                    properties.reason_string = Some(Box::<str>::read(&mut property_data)?);
                }
                PropertyType::UserProperty => properties.user_properties.push((Box::<str>::read(&mut property_data)?, Box::<str>::read(&mut property_data)?)),
                PropertyType::WildcardSubscriptionAvailable => {
                    if properties.wildcards_available.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::WildcardSubscriptionAvailable));
                    }
                    properties.wildcards_available = Some(bool::read(&mut property_data)?);
                }
                PropertyType::SubscriptionIdentifierAvailable => {
                    if properties.subscription_ids_available.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::SubscriptionIdentifierAvailable));
                    }
                    properties.subscription_ids_available = Some(bool::read(&mut property_data)?);
                }
                PropertyType::SharedSubscriptionAvailable => {
                    if properties.shared_subscription_available.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::SharedSubscriptionAvailable));
                    }
                    properties.shared_subscription_available = Some(bool::read(&mut property_data)?);
                }
                PropertyType::ServerKeepAlive => {
                    if properties.server_keep_alive.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::ServerKeepAlive));
                    }
                    properties.server_keep_alive = Some(u16::read(&mut property_data)?);
                }
                PropertyType::ResponseInformation => {
                    if properties.response_info.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::ResponseInformation));
                    }
                    properties.response_info = Some(Box::<str>::read(&mut property_data)?);
                }
                PropertyType::ServerReference => {
                    if properties.server_reference.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::ServerReference));
                    }
                    properties.server_reference = Some(Box::<str>::read(&mut property_data)?);
                }
                PropertyType::AuthenticationMethod => {
                    if properties.authentication_method.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::AuthenticationMethod));
                    }
                    properties.authentication_method = Some(Box::<str>::read(&mut property_data)?);
                }
                PropertyType::AuthenticationData => {
                    if properties.authentication_data.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::AuthenticationData));
                    }
                    properties.authentication_data = Some(Vec::<u8>::read(&mut property_data)?);
                }

                e => return Err(DeserializeError::UnexpectedProperty(e, PacketType::ConnAck)),
            }

            if property_data.is_empty() {
                break;
            }
        }

        Ok(properties)
    }
}

impl MqttWrite for ConnAckProperties {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), crate::packets::error::SerializeError> {
        self.wire_len().write_variable_integer(buf)?;

        let Self {
            session_expiry_interval,
            receive_maximum,
            maximum_qos,
            retain_available,
            maximum_packet_size,
            assigned_client_id,
            topic_alias_maximum,
            reason_string,
            user_properties,
            wildcards_available,
            subscription_ids_available,
            shared_subscription_available,
            server_keep_alive,
            response_info,
            server_reference,
            authentication_method,
            authentication_data,
        } = self;

        if let Some(session_expiry_interval) = session_expiry_interval {
            PropertyType::SessionExpiryInterval.write(buf)?;
            buf.put_u32(*session_expiry_interval);
        }
        if let Some(receive_maximum) = receive_maximum {
            PropertyType::ReceiveMaximum.write(buf)?;
            buf.put_u16(*receive_maximum);
        }
        if let Some(maximum_qos) = maximum_qos {
            PropertyType::MaximumQos.write(buf)?;
            maximum_qos.write(buf)?;
        }
        if let Some(retain_available) = retain_available {
            PropertyType::RetainAvailable.write(buf)?;
            retain_available.write(buf)?;
        }
        if let Some(maximum_packet_size) = maximum_packet_size {
            PropertyType::MaximumPacketSize.write(buf)?;
            buf.put_u32(*maximum_packet_size);
        }
        if let Some(client_id) = assigned_client_id {
            PropertyType::AssignedClientIdentifier.write(buf)?;
            client_id.write(buf)?;
        }
        if let Some(topic_alias_maximum) = topic_alias_maximum {
            PropertyType::TopicAliasMaximum.write(buf)?;
            buf.put_u16(*topic_alias_maximum);
        }
        if let Some(reason_string) = reason_string {
            PropertyType::ReasonString.write(buf)?;
            reason_string.write(buf)?;
        }
        for (key, val) in user_properties.iter() {
            PropertyType::UserProperty.write(buf)?;
            key.write(buf)?;
            val.write(buf)?;
        }
        if let Some(wildcards_available) = wildcards_available {
            PropertyType::WildcardSubscriptionAvailable.write(buf)?;
            wildcards_available.write(buf)?;
        }
        if let Some(subscription_ids_available) = subscription_ids_available {
            PropertyType::SubscriptionIdentifierAvailable.write(buf)?;
            subscription_ids_available.write(buf)?;
        }
        if let Some(shared_subscription_available) = shared_subscription_available {
            PropertyType::SharedSubscriptionAvailable.write(buf)?;
            shared_subscription_available.write(buf)?;
        }
        if let Some(server_keep_alive) = server_keep_alive {
            PropertyType::ServerKeepAlive.write(buf)?;
            server_keep_alive.write(buf)?;
        }
        if let Some(response_info) = response_info {
            PropertyType::ResponseInformation.write(buf)?;
            response_info.write(buf)?;
        }
        if let Some(server_reference) = server_reference {
            PropertyType::ServerReference.write(buf)?;
            server_reference.write(buf)?;
        }
        if let Some(authentication_method) = &authentication_method {
            PropertyType::AuthenticationMethod.write(buf)?;
            authentication_method.write(buf)?;
        }
        if let Some(authentication_data) = authentication_data {
            if authentication_method.is_none() {
                return Err(SerializeError::AuthDataWithoutAuthMethod);
            }
            PropertyType::AuthenticationData.write(buf)?;
            authentication_data.write(buf)?;
        }

        Ok(())
    }
}