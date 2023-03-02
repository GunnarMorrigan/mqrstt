use bytes::Bytes;

use rstest::*;

use crate::packets::{
    reason_codes::{DisconnectReasonCode, PubAckReasonCode},
    ConnAck, Disconnect, DisconnectProperties, Packet, PubAck, PubAckProperties, Publish, PublishProperties, QoS, Subscribe, Subscription, Unsubscribe,
};

fn publish_packet_1() -> Packet {
    Packet::Publish(Publish {
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
    })
}
fn publish_packet_2() -> Packet {
    Packet::Publish(Publish {
        dup: true,
        qos: QoS::ExactlyOnce,
        retain: false,
        topic: "test/#".to_string(),
        packet_identifier: Some(4566),
        publish_properties: PublishProperties {
            payload_format_indicator: None,
            message_expiry_interval: Some(3600),
            topic_alias: Some(1),
            response_topic: None,
            correlation_data: Some(Bytes::from_static(b"1212")),
            subscription_identifier: vec![1],
            user_properties: vec![],
            content_type: None,
        },
        payload: Bytes::from_static(b""),
    })
}
fn publish_packet_3() -> Packet {
    Packet::Publish(Publish {
        dup: true,
        qos: QoS::AtLeastOnce,
        retain: false,
        topic: "test/#".to_string(),
        packet_identifier: Some(4566),
        publish_properties: PublishProperties {
            payload_format_indicator: None,
            message_expiry_interval: Some(3600),
            topic_alias: None,
            response_topic: Some("Please respond here thank you".to_string()),
            correlation_data: Some(Bytes::from_static(b"5420874")),
            subscription_identifier: vec![],
            user_properties: vec![("blabla".to_string(), "another blabla".to_string())],
            content_type: None,
        },
        payload: Bytes::from_static(b""),
    })
}
fn publish_packet_4() -> Packet {
    Packet::Publish(Publish {
        dup: true,
        qos: QoS::AtLeastOnce,
        retain: false,
        topic: "test/#".to_string(),
        packet_identifier: Some(4566),
        publish_properties: PublishProperties {
            payload_format_indicator: None,
            message_expiry_interval: Some(3600),
            topic_alias: Some(1),
            response_topic: None,
            correlation_data: Some(Bytes::from_static(b"1212")),
            subscription_identifier: vec![1],
            user_properties: vec![],
            content_type: Some("Garbage".to_string()),
        },
        payload: Bytes::from_static(b""),
        // payload: Bytes::from_iter(b"abcdefg".repeat(500)),
    })
}

pub fn create_subscribe_packet(packet_identifier: u16) -> Packet {
    let subscription: Subscription = "test/topic".into();
    let sub = Subscribe::new(packet_identifier, subscription.0);
    Packet::Subscribe(sub)
}

pub fn create_unsubscribe_packet(packet_identifier: u16) -> Packet {
    let sub = Unsubscribe::new(packet_identifier, vec!["test/topic".to_string()]);
    Packet::Unsubscribe(sub)
}

pub fn create_publish_packet(qos: QoS, dup: bool, retain: bool, packet_identifier: Option<u16>) -> Packet {
    Packet::Publish(Publish {
        dup,
        qos,
        retain,
        topic: "test/#".to_string(),
        packet_identifier,
        publish_properties: PublishProperties {
            payload_format_indicator: None,
            message_expiry_interval: Some(3600),
            topic_alias: Some(1),
            response_topic: None,
            correlation_data: Some(Bytes::from_static(b"1212")),
            subscription_identifier: vec![1],
            user_properties: vec![],
            content_type: Some("Garbage".to_string()),
        },
        payload: Bytes::from_iter(b"testabcbba==asdasdasdasdasd".repeat(500)),
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

    let read_packet = Packet::read_from_buffer(&mut buffer).unwrap();

    assert_eq!(packet, read_packet);
}
