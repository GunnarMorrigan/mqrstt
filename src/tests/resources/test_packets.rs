use bytes::Bytes;

use crate::packets::{packets::Packet, publish::{Publish, PublishProperties}, QoS, puback::{PubAck, PubAckProperties}, reason_codes::{PubAckReasonCode, DisconnectReasonCode}, disconnect::{Disconnect, DisconnectProperties}, pubrel::PubRel, subscribe::{Subscription, Subscribe}};


pub fn publish_packets() -> Vec::<Packet>{
    let mut ret = vec![];

    let packet = Packet::Publish(Publish {
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
    });
    ret.push(packet);


    let packet = Packet::Publish(Publish {
        dup: true,
        qos: QoS::AtMostOnce,
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
    });
    ret.push(packet);


    let packet = Packet::Publish(Publish {
        dup: true,
        qos: QoS::AtMostOnce,
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
    });
    ret.push(packet);


    let packet = Packet::Publish(Publish {
        dup: true,
        qos: QoS::AtMostOnce,
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
        payload: Bytes::from_iter(b"abcdefg".repeat(500)),
    });
    ret.push(packet);

    ret
}


pub fn create_subscribe_packet(packet_identifier: u16) -> Packet{
        let subscription: Subscription = "test/topic".into();
        let sub = Subscribe::new(packet_identifier, subscription.0);
        Packet::Subscribe(sub)
}

pub fn create_publish_packet(qos: QoS, dup: bool, retain: bool, packet_identifier: Option<u16>) -> Packet{
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

pub fn create_puback_packet(packet_identifier: u16) -> Packet{
    Packet::PubAck(PubAck{
        packet_identifier,
        reason_code: PubAckReasonCode::Success,
        properties: PubAckProperties::default(),
    })
}

// pub fn create_pubrel_packet(packet_identifier: u16) -> Packet{
//     Packet::PubRel(PubRel{
//         packet_identifier,
//         reason_code: PubAckReasonCode::Success,
//         properties: PubAckProperties::default(),
//     })
// }

pub fn create_disconnect_packet() -> Packet{
    Packet::Disconnect(Disconnect{
        reason_code: DisconnectReasonCode::NormalDisconnection,
        properties: DisconnectProperties::default(),
    })
}