mod test_cases;

pub use test_cases::*;

#[cfg(test)]
mod tests {

    use bytes::BytesMut;

    use crate::packets::Packet;

    use rstest::*;

    use crate::packets::mqtt_trait::WireLength;
    use crate::tests::test_packets::*;

    #[rstest::rstest]
    #[case::connect_case(connect_case())]
    #[case::ping_req_case(ping_req_case().1)]
    #[case::ping_resp_case(ping_resp_case().1)]
    #[case::connack_case(connack_case().1)]
    #[case::create_subscribe_packet(create_subscribe_packet(1))]
    #[case::create_subscribe_packet(create_subscribe_packet(65335))]
    #[case::create_puback_packet(create_puback_packet(1))]
    #[case::create_puback_packet(create_puback_packet(65335))]
    #[case::create_disconnect_packet(create_disconnect_packet())]
    #[case::create_connack_packet(create_connack_packet(true))]
    #[case::create_connack_packet(create_connack_packet(false))]
    #[case::publish_packet_1(publish_packet_1())]
    #[case::publish_packet_2(publish_packet_2())]
    #[case::publish_packet_3(publish_packet_3())]
    #[case::publish_packet_4(publish_packet_4())]
    #[case::create_empty_publish_packet(create_empty_publish_packet())]
    #[case::subscribe(subscribe_case())]
    #[case::suback(suback_case())]
    #[case::unsubscribe(unsubscribe_case())]
    #[case::unsuback(unsuback_case())]
    #[case::pubcomp_case(pubcomp_case())]
    #[case::pubrec_case(pubrec_case())]
    #[case::pubrec_case(pubrel_case2())]
    #[case::auth_case(auth_case())]
    fn test_write_read_write_read_cases(#[case] packet: Packet) {
        let mut buffer = BytesMut::new();

        packet.write(&mut buffer).unwrap();

        let wire_len = packet.wire_len();
        assert_eq!(wire_len, buffer.len());

        let res1 = Packet::read(&mut buffer).unwrap();

        assert_eq!(packet, res1);

        let mut buffer = BytesMut::new();
        res1.write(&mut buffer).unwrap();
        let res2 = Packet::read(&mut buffer).unwrap();

        assert_eq!(res1, res2);
    }

    #[rstest::rstest]
    #[case::connect_case(connect_case())]
    #[case::ping_req_case(ping_req_case().1)]
    #[case::ping_resp_case(ping_resp_case().1)]
    #[case::connack_case(connack_case().1)]
    #[case::create_subscribe_packet(create_subscribe_packet(1))]
    #[case::create_subscribe_packet(create_subscribe_packet(65335))]
    #[case::create_puback_packet(create_puback_packet(1))]
    #[case::create_puback_packet(create_puback_packet(65335))]
    #[case::create_disconnect_packet(create_disconnect_packet())]
    #[case::create_connack_packet(create_connack_packet(true))]
    #[case::create_connack_packet(create_connack_packet(false))]
    #[case::publish_packet_1(publish_packet_1())]
    #[case::publish_packet_2(publish_packet_2())]
    #[case::publish_packet_3(publish_packet_3())]
    #[case::publish_packet_4(publish_packet_4())]
    #[case::create_empty_publish_packet(create_empty_publish_packet())]
    #[case::subscribe(subscribe_case())]
    #[case::suback(suback_case())]
    #[case::unsubscribe(unsubscribe_case())]
    #[case::unsuback(unsuback_case())]
    #[case::pubcomp_case(pubcomp_case())]
    #[case::pubrec_case(pubrec_case())]
    #[case::pubrec_case(pubrel_case2())]
    #[case::auth_case(auth_case())]
    #[tokio::test]
    async fn test_async_write_read_write_read_cases(#[case] packet: Packet) {
        let mut buffer = Vec::with_capacity(1000);
        let res = packet.async_write(&mut buffer).await.unwrap();

        let wire_len = packet.wire_len();

        assert_eq!(res, buffer.len());
        assert_eq!(wire_len, buffer.len());

        let mut buf = buffer.as_slice();

        let res1 = Packet::async_read(&mut buf).await.unwrap();

        pretty_assertions::assert_eq!(packet, res1);
    }

    #[rstest::rstest]
    #[case::disconnect(disconnect_case())]
    #[case::ping_req(ping_req_case())]
    #[case::ping_resp(ping_resp_case())]
    #[case::publish(publish_case())]
    #[case::pubrel(pubrel_case())]
    #[case::pubrel_smallest(pubrel_smallest_case())]
    fn test_read_write_cases(#[case] (bytes, expected_packet): (&[u8], Packet)) {
        let mut buffer = BytesMut::from_iter(bytes);

        let res = Packet::read(&mut buffer);

        assert!(res.is_ok());

        let packet = res.unwrap();

        assert_eq!(packet, expected_packet);

        buffer.clear();

        packet.write(&mut buffer).unwrap();

        assert_eq!(buffer.to_vec(), bytes.to_vec())
    }

    #[rstest::rstest]
    #[case::disconnect(disconnect_case())]
    #[case::ping_req(ping_req_case())]
    #[case::ping_resp(ping_resp_case())]
    #[case::publish(publish_case())]
    #[case::pubrel(pubrel_case())]
    #[case::pubrel_smallest(pubrel_smallest_case())]
    #[tokio::test]
    async fn test_async_read_write(#[case] (mut bytes, expected_packet): (&[u8], Packet)) {
        let input = bytes.to_vec();

        let res = Packet::async_read(&mut bytes).await;

        assert!(res.is_ok());

        let packet = res.unwrap();

        assert_eq!(packet, expected_packet);

        let mut out = Vec::with_capacity(1000);

        packet.async_write(&mut out).await.unwrap();

        assert_eq!(out, input)
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

    // #[rstest::rstest]
    // #[case(&[59, 1, 0, 59])]
    // #[case(&[16, 14, 0, 4, 77, 81, 84, 84, 5, 247, 247, 252, 1, 17, 247, 247, 247])]
    // fn test_read_error(#[case] bytes: &[u8]) {
    //     let mut buffer = BytesMut::from_iter(bytes);

    //     let res = Packet::read_from_buffer(&mut buffer);

    //     assert!(res.is_err());
    // }
}
