use super::{
    error::DeserializeError,
    mqtt_traits::{MqttRead, MqttWrite, VariableHeaderRead},
    read_variable_integer,
    reason_codes::ConnAckReasonCode,
    PacketType, PropertyType, QoS,
};
use bytes::{Buf, BufMut, Bytes};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ConnAck {
    /// 3.2.2.1 Connect Acknowledge Flags
    pub connack_flags: ConnAckFlags,

    /// 3.2.2.2 Connect Reason Code
    /// Byte 2 in the Variable Header is the Connect Reason Code.
    pub reason_code: ConnAckReasonCode,

    /// 3.2.2.3 CONNACK Properties
    pub connack_properties: ConnAckProperties,
}

impl VariableHeaderRead for ConnAck {
    fn read(_: u8, header_len: usize, mut buf: bytes::Bytes) -> Result<Self, DeserializeError> {
        if header_len > buf.len() {
            return Err(DeserializeError::InsufficientData("ConnAck".to_string(), buf.len(), header_len));
        }

        let connack_flags = ConnAckFlags::read(&mut buf)?;
        let reason_code = ConnAckReasonCode::read(&mut buf)?;
        let connack_properties = ConnAckProperties::read(&mut buf)?;

        Ok(Self {
            connack_flags,
            reason_code,
            connack_properties,
        })
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConnAckProperties {
    /// 3.2.2.3.2 Session Expiry Interval
    /// 17 (0x11) Byte Identifier of the Session Expiry Interval
    pub session_expiry_interval: Option<u32>,

    /// 3.2.2.3.3 Receive Maximum
    /// 33 (0x21) Byte, Identifier of the Receive Maximum
    pub receive_maximum: Option<u16>,

    /// 3.2.2.3.4 Maximum QoS
    /// 36 (0x24) Byte, Identifier of the Maximum QoS.
    pub maximum_qos: Option<QoS>,

    /// 3.2.2.3.5 Retain Available
    /// 37 (0x25) Byte, Identifier of Retain Available.
    pub retain_available: Option<bool>,

    /// 3.2.2.3.6 Maximum Packet Size
    /// 39 (0x27) Byte, Identifier of the Maximum Packet Size.
    pub maximum_packet_size: Option<u32>,

    /// 3.2.2.3.7 Assigned Client Identifier
    /// 18 (0x12) Byte, Identifier of the Assigned Client Identifier.
    pub assigned_client_id: Option<String>,

    /// 3.2.2.3.8 Topic Alias Maximum
    /// 34 (0x22) Byte, Identifier of the Topic Alias Maximum.
    pub topic_alias_maximum: Option<u16>,

    /// 3.2.2.3.9 Reason String
    /// 31 (0x1F) Byte Identifier of the Reason String.
    pub reason_string: Option<String>,

    /// 3.2.2.3.10 User Property
    /// 38 (0x26) Byte, Identifier of User Property.
    pub user_properties: Vec<(String, String)>,

    /// 3.2.2.3.11 Wildcard Subscription Available
    /// 40 (0x28) Byte, Identifier of Wildcard Subscription Available.
    pub wildcards_available: Option<bool>,

    /// 3.2.2.3.12 Subscription Identifiers Available
    /// 41 (0x29) Byte, Identifier of Subscription Identifier Available.
    pub subscription_ids_available: Option<bool>,

    /// 3.2.2.3.13 Shared Subscription Available
    /// 42 (0x2A) Byte, Identifier of Shared Subscription Available.
    pub shared_subscription_available: Option<bool>,

    /// 3.2.2.3.14 Server Keep Alive
    /// 19 (0x13) Byte, Identifier of the Server Keep Alive
    pub server_keep_alive: Option<u16>,

    /// 3.2.2.3.15 Response Information
    /// 26 (0x1A) Byte, Identifier of the Response Information.
    pub response_info: Option<String>,

    /// 3.2.2.3.16 Server Reference
    /// 28 (0x1C) Byte, Identifier of the Server Reference
    pub server_reference: Option<String>,

    /// 3.2.2.3.17 Authentication Method
    /// 21 (0x15) Byte, Identifier of the Authentication Method
    pub authentication_method: Option<String>,

    /// 3.2.2.3.18 Authentication Data
    /// 22 (0x16) Byte, Identifier of the Authentication Data
    pub authentication_data: Option<Bytes>,
}

impl MqttRead for ConnAckProperties {
    fn read(buf: &mut bytes::Bytes) -> Result<Self, DeserializeError> {
        let (len, _) = read_variable_integer(buf).map_err(DeserializeError::from)?;

        let mut properties = Self::default();
        if len == 0 {
            return Ok(properties);
        } else if buf.len() < len {
            return Err(DeserializeError::InsufficientData("ConnAckProperties".to_string(), buf.len(), len));
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
                    properties.assigned_client_id = Some(String::read(&mut property_data)?);
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
                    properties.reason_string = Some(String::read(&mut property_data)?);
                }
                PropertyType::UserProperty => properties.user_properties.push((String::read(&mut property_data)?, String::read(&mut property_data)?)),
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
                    properties.response_info = Some(String::read(&mut property_data)?);
                }
                PropertyType::ServerReference => {
                    if properties.server_reference.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::ServerReference));
                    }
                    properties.server_reference = Some(String::read(&mut property_data)?);
                }
                PropertyType::AuthenticationMethod => {
                    if properties.authentication_method.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::AuthenticationMethod));
                    }
                    properties.authentication_method = Some(String::read(&mut property_data)?);
                }
                PropertyType::AuthenticationData => {
                    if properties.authentication_data.is_some() {
                        return Err(DeserializeError::DuplicateProperty(PropertyType::AuthenticationData));
                    }
                    properties.authentication_data = Some(Bytes::read(&mut property_data)?);
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

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub struct ConnAckFlags {
    pub session_present: bool,
}

impl MqttRead for ConnAckFlags {
    fn read(buf: &mut bytes::Bytes) -> Result<Self, DeserializeError> {
        if buf.is_empty() {
            return Err(DeserializeError::InsufficientData("ConnAckFlags".to_string(), 0, 1));
        }

        let byte = buf.get_u8();

        Ok(Self {
            session_present: (byte & 0b00000001) != 0,
        })
    }
}

impl MqttWrite for ConnAckFlags {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
        let byte = self.session_present as u8;

        buf.put_u8(byte);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::packets::{
        connack::{ConnAck, ConnAckProperties},
        mqtt_traits::{MqttRead, VariableHeaderRead},
        reason_codes::ConnAckReasonCode,
    };

    #[test]
    fn read_connack() {
        let mut buf = bytes::BytesMut::new();
        let packet = &[
            0x01, // Connack flags
            0x00, // Reason code,
            0x00, // empty properties
        ];

        buf.extend_from_slice(packet);
        let c = ConnAck::read(0, packet.len(), buf.into()).unwrap();

        assert_eq!(ConnAckReasonCode::Success, c.reason_code);
        assert_eq!(ConnAckProperties::default(), c.connack_properties);
    }

    #[test]
    fn read_connack_properties() {
        let mut buf = bytes::BytesMut::new();
        let packet = &[
            56, // ConnAckProperties variable length
            17, // session_expiry_interval
            0xff, 0xff, 37,  // retain_available
            0x1, // true
            18,  // Assigned Client Id
            0, 11,   // 11 bytes
            b'K', // Keanu Reeves without space
            b'e', b'a', b'n', b'u', b'R', b'e', b'e', b'v', b'e', b's', 36, // Max QoS
            2,  // QoS 2 Exactly Once
            34, // Topic Alias Max = 255
            0, 255, 31, // Reason String = 'Houston we have got a problem'
            0, 29, b'H', b'o', b'u', b's', b't', b'o', b'n', b' ', b'w', b'e', b' ', b'h', b'a', b'v', b'e', b' ', b'g', b'o', b't', b' ', b'a', b' ', b'p', b'r', b'o', b'b', b'l', b'e', b'm',
        ];

        buf.extend_from_slice(packet);
        let c = ConnAckProperties::read(&mut buf.into()).unwrap();

        dbg!(c);
    }
}
