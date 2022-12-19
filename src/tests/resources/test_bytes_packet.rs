use bytes::Bytes;

use crate::packets::{connack::{ConnAck, ConnAckFlags, ConnAckProperties}, reason_codes::{ConnAckReasonCode, DisconnectReasonCode}, packets::Packet, disconnect::{Disconnect, DisconnectProperties}, publish::{Publish, PublishProperties}, QoS};




pub fn connack_bytes_packet() -> ([u8; 21], Packet){
    const CONNACK_BYTES: [u8; 21] = [
        0x20, 0x13, 0x01, 0x00, 0x10, 0x27, 0x00, 0x10, 0x00, 0x00, 0x25, 0x01, 0x2a, 0x01,
        0x29, 0x01, 0x22, 0xff, 0xff, 0x28, 0x01,
    ];

    let expected = ConnAck {
        connack_flags: ConnAckFlags::SESSION_PRESENT,
        reason_code: ConnAckReasonCode::Success,
        connack_properties: ConnAckProperties {
            session_expiry_interval: None,
            receive_maximum: None,
            maximum_qos: None,
            retain_available: Some(true),
            maximum_packet_size: Some(1048576),
            assigned_client_id: None,
            topic_alias_maximum: Some(65535),
            reason_string: None,
            user_properties: vec![],
            wildcards_available: Some(true),
            subscription_ids_available: Some(true),
            shared_subscription_available: Some(true),
            server_keep_alive: None,
            response_info: None,
            server_reference: None,
            authentication_method: None,
            authentication_data: None,
        },
    };
    (CONNACK_BYTES, Packet::ConnAck(expected))
}

pub fn disconnect_bytes_packet() -> ([u8; 4], Packet){
    const DISCONNECT_BYTES: [u8; 4] = [0xe0, 0x02, 0x8e, 0x00];

    let expected = Disconnect {
        reason_code: DisconnectReasonCode::SessionTakenOver,
        properties: DisconnectProperties {
            session_expiry_interval: None,
            reason_string: None,
            user_properties: vec![],
            server_reference: None,
        },
    };
    (DISCONNECT_BYTES, Packet::Disconnect(expected))
}

pub fn pingreq_bytes_packet() -> ([u8; 2], Packet){
    ([0xc0, 0x00], Packet::PingReq)
}

pub fn pingresp_bytes_packet() -> ([u8; 2], Packet){
    ([0xd0, 0x00], Packet::PingResp)
}

pub(crate) fn publish_bytes_packet_qos_2() -> ([u8; 38], Packet){
    const PUBLISH_BYTES: [u8; 38] = [
        0x35, 0x24, 0x00, 0x14, 0x74, 0x65, 0x73, 0x74, 0x2f, 0x31, 0x32, 0x33, 0x2f, 0x74,
        0x65, 0x73, 0x74, 0x2f, 0x62, 0x6c, 0x61, 0x62, 0x6c, 0x61, 0x35, 0xd3, 0x0b, 0x01,
        0x01, 0x09, 0x00, 0x04, 0x31, 0x32, 0x31, 0x32, 0x0b, 0x01,
    ];

    let expected = Publish {
        dup: false,
        qos: QoS::ExactlyOnce,
        retain: true,
        topic: "test/123/test/blabla".to_string(),
        packet_identifier: Some(13779),
        publish_properties: PublishProperties {
            payload_format_indicator: Some(1),
            message_expiry_interval: None,
            topic_alias: None,
            response_topic: None,
            correlation_data: Some(Bytes::from_static(b"1212")),
            subscription_identifier: vec![1],
            user_properties: vec![],
            content_type: None,
        },
        payload: Bytes::from_static(b""),
    };
    (PUBLISH_BYTES, Packet::Publish(expected))
}