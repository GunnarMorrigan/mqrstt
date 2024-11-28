use rstest::*;

use crate::packets::*;

pub fn connack_case() -> (&'static [u8], Packet) {
    let packet: &'static [u8] = &[
        0x20, 0x13, 0x01, 0x00, 0x10, 0x27, 0x00, 0x10, 0x00, 0x00, 0x25, 0x01, 0x2a, 0x01, 0x29, 0x01, 0x22, 0xff, 0xff, 0x28, 0x01,
    ];

    let expected = ConnAck {
        connack_flags: ConnAckFlags { session_present: true },
        reason_code: ConnAckReasonCode::Success,
        connack_properties: ConnAckProperties {
            session_expiry_interval: None,
            receive_maximum: None,
            maximum_qos: None,
            retain_available: Some(true),
            maximum_packet_size: Some(1048576),
            assigned_client_identifier: None,
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

    (packet, Packet::ConnAck(expected))
}

pub fn disconnect_case() -> (&'static [u8], Packet) {
    let packet: &'static [u8] = &[0xe0, 0x02, 0x8e, 0x00];

    let expected = Disconnect {
        reason_code: DisconnectReasonCode::SessionTakenOver,
        properties: DisconnectProperties {
            session_expiry_interval: None,
            reason_string: None,
            user_properties: vec![],
            server_reference: None,
        },
    };

    (packet, Packet::Disconnect(expected))
}

pub fn ping_req_case() -> (&'static [u8], Packet) {
    let packet: &'static [u8] = &[0xc0, 0x00];

    (packet, Packet::PingReq)
}

pub fn ping_resp_case() -> (&'static [u8], Packet) {
    let packet: &'static [u8] = &[0xd0, 0x00];

    (packet, Packet::PingResp)
}
pub fn publish_case() -> (&'static [u8], Packet) {
    let packet: &'static [u8] = &[
        0x35, 0x24, 0x00, 0x14, 0x74, 0x65, 0x73, 0x74, 0x2f, 0x31, 0x32, 0x33, 0x2f, 0x74, 0x65, 0x73, 0x74, 0x2f, 0x62, 0x6c, 0x61, 0x62, 0x6c, 0x61, 0x35, 0xd3, 0x0b, 0x01, 0x01, 0x09, 0x00, 0x04,
        0x31, 0x32, 0x31, 0x32, 0x0b, 0x01,
    ];

    let expected = Publish {
        dup: false,
        qos: QoS::ExactlyOnce,
        retain: true,
        topic: "test/123/test/blabla".into(),
        packet_identifier: Some(13779),
        publish_properties: PublishProperties {
            payload_format_indicator: Some(1),
            message_expiry_interval: None,
            topic_alias: None,
            response_topic: None,
            correlation_data: Some(b"1212".to_vec()),
            subscription_identifiers: vec![1],
            user_properties: vec![],
            content_type: None,
        },
        payload: b"".to_vec(),
    };

    (packet, Packet::Publish(expected))
}

pub fn pubrel_case() -> (&'static [u8], Packet) {
    let packet: &'static [u8] = &[0x62, 0x02, 0x35, 0xd3];

    let expected = PubRel {
        packet_identifier: 13779,
        reason_code: PubRelReasonCode::Success,
        properties: PubRelProperties {
            reason_string: None,
            user_properties: vec![],
        },
    };

    (packet, Packet::PubRel(expected))
}

pub fn pubrel_smallest_case() -> (&'static [u8], Packet) {
    let packet: &'static [u8] = &[0x62, 0x02, 0x35, 0xd3];

    let expected = PubRel {
        packet_identifier: 13779,
        reason_code: PubRelReasonCode::Success,
        properties: PubRelProperties {
            reason_string: None,
            user_properties: vec![],
        },
    };

    (packet, Packet::PubRel(expected))
}

pub fn publish_packet_1() -> Packet {
    Packet::Publish(Publish {
        dup: false,
        qos: QoS::ExactlyOnce,
        retain: true,
        topic: "test/123/test/blabla".into(),
        packet_identifier: Some(13779),
        publish_properties: PublishProperties {
            payload_format_indicator: Some(1),
            message_expiry_interval: None,
            topic_alias: None,
            response_topic: None,
            correlation_data: Some(b"1212".to_vec()),
            subscription_identifiers: vec![1],
            user_properties: vec![],
            content_type: None,
        },
        payload: b"".to_vec(),
    })
}
pub fn publish_packet_2() -> Packet {
    Packet::Publish(Publish {
        dup: true,
        qos: QoS::ExactlyOnce,
        retain: false,
        topic: "test/#".into(),
        packet_identifier: Some(4566),
        publish_properties: PublishProperties {
            payload_format_indicator: None,
            message_expiry_interval: Some(3600),
            topic_alias: Some(1),
            response_topic: None,
            correlation_data: Some(b"1212".to_vec()),
            subscription_identifiers: vec![1],
            user_properties: vec![],
            content_type: None,
        },
        payload: b"".to_vec(),
    })
}
pub fn publish_packet_3() -> Packet {
    Packet::Publish(Publish {
        dup: true,
        qos: QoS::AtLeastOnce,
        retain: false,
        topic: "test/#".into(),
        packet_identifier: Some(4566),
        publish_properties: PublishProperties {
            payload_format_indicator: None,
            message_expiry_interval: Some(3600),
            topic_alias: None,
            response_topic: Some("Please respond here thank you".into()),
            correlation_data: Some(b"5420874".to_vec()),
            subscription_identifiers: vec![],
            user_properties: vec![("blabla".into(), "another blabla".into())],
            content_type: None,
        },
        payload: b"".to_vec(),
    })
}
pub fn publish_packet_4() -> Packet {
    Packet::Publish(Publish {
        dup: true,
        qos: QoS::AtLeastOnce,
        retain: false,
        topic: "test/#".into(),
        packet_identifier: Some(4566),
        publish_properties: PublishProperties {
            payload_format_indicator: None,
            message_expiry_interval: Some(3600),
            topic_alias: Some(1),
            response_topic: None,
            correlation_data: Some(b"1212".to_vec()),
            subscription_identifiers: vec![1],
            user_properties: vec![],
            content_type: Some("Garbage".into()),
        },
        payload: b"".to_vec(),
        // payload: Bytes::from_iter(b"abcdefg".repeat(500)),
    })
}

pub fn create_subscribe_packet(packet_identifier: u16) -> Packet {
    let subscription: SubscribeTopics = "test/topic".into();
    let sub = Subscribe::new(packet_identifier, subscription.0);
    Packet::Subscribe(sub)
}

pub fn create_unsubscribe_packet(packet_identifier: u16) -> Packet {
    let sub = Unsubscribe::new(packet_identifier, vec!["test/topic".into()]);
    Packet::Unsubscribe(sub)
}

pub fn create_publish_packet(qos: QoS, dup: bool, retain: bool, packet_identifier: Option<u16>) -> Packet {
    Packet::Publish(Publish {
        dup,
        qos,
        retain,
        topic: "test/#".into(),
        packet_identifier,
        publish_properties: PublishProperties {
            payload_format_indicator: None,
            message_expiry_interval: Some(3600),
            topic_alias: Some(1),
            response_topic: None,
            correlation_data: Some(b"1212".to_vec()),
            subscription_identifiers: vec![1],
            user_properties: vec![],
            content_type: Some("Garbage".into()),
        },
        payload: b"testabcbba==asdasdasdasdasd".repeat(500).to_vec(),
    })
}

pub fn create_empty_publish_packet() -> Packet {
    Packet::Publish(Publish {
        dup: false,
        qos: QoS::AtMostOnce,
        retain: false,
        topic: "test/#".into(),
        packet_identifier: None,
        publish_properties: PublishProperties {
            payload_format_indicator: None,
            message_expiry_interval: Some(3600),
            topic_alias: Some(1),
            response_topic: None,
            correlation_data: Some(b"1212".to_vec()),
            subscription_identifiers: vec![1],
            user_properties: vec![],
            content_type: Some("Garbage".into()),
        },
        payload: vec![],
    })
}

pub fn create_puback_packet(packet_identifier: u16) -> Packet {
    Packet::PubAck(PubAck {
        packet_identifier,
        reason_code: PubAckReasonCode::Success,
        properties: PubAckProperties::default(),
    })
}

pub fn create_connack_packet(session_present: bool) -> Packet {
    let mut connack = ConnAck::default();
    connack.connack_flags.session_present = session_present;

    Packet::ConnAck(connack)
}

pub fn create_disconnect_packet() -> Packet {
    Packet::Disconnect(Disconnect {
        reason_code: DisconnectReasonCode::NormalDisconnection,
        properties: DisconnectProperties::default(),
    })
}

pub fn suback_case() -> Packet {
    let expected = SubAck {
        packet_identifier: 3,
        reason_codes: vec![SubAckReasonCode::GrantedQoS0, SubAckReasonCode::GrantedQoS1, SubAckReasonCode::GrantedQoS2],
        properties: SubAckProperties {
            user_properties: vec![(String::from("test").into(), String::from("test").into())],
            subscription_identifier: Some(2000),
        },
    };

    Packet::SubAck(expected)
}

pub fn subscribe_case() -> Packet {
    let expected = Subscribe {
        packet_identifier: 3,
        topics: vec![("test/topic".into(), SubscriptionOptions::default())],
        properties: SubscribeProperties {
            user_properties: vec![(String::from("test").into(), String::from("test").into())],
            subscription_identifier: Some(2000),
        },
    };

    Packet::Subscribe(expected)
}

// return a crazy big packet
pub fn unsuback_case() -> Packet {
    let expected = UnsubAck {
        packet_identifier: 3,
        reason_codes: vec![
            UnsubAckReasonCode::NoSubscriptionExisted,
            UnsubAckReasonCode::UnspecifiedError,
            UnsubAckReasonCode::ImplementationSpecificError,
        ],
        properties: UnsubAckProperties {
            user_properties: vec![],
            reason_string: None,
        },
    };

    Packet::UnsubAck(expected)
}

pub fn unsubscribe_case() -> Packet {
    let expected = Unsubscribe {
        packet_identifier: 3,
        topics: vec!["test/topic".into()],
        properties: UnsubscribeProperties { user_properties: vec![("written += 1;".into(), "value".into())] },
    };

    Packet::Unsubscribe(expected)
}

#[rstest]
#[case(create_subscribe_packet(1))]
#[case(create_subscribe_packet(65335))]
#[case(create_puback_packet(1))]
#[case(create_puback_packet(65335))]
#[case(create_disconnect_packet())]
#[case(publish_packet_1())]
#[case(publish_packet_2())]
#[case(publish_packet_3())]
#[case(publish_packet_4())]
/// Test if the input == output after read packet form input and write packet to output
fn test_equal_write_read(#[case] packet: Packet) {
    let mut buffer = bytes::BytesMut::new();

    packet.write(&mut buffer).unwrap();

    let read_packet = Packet::read(&mut buffer).unwrap();

    assert_eq!(packet, read_packet);
}
