use rstest::*;

use bytes::BytesMut;

use crate::packets::{mqtt_trait::WireLength, Packet};

fn publish_packet() -> Vec<u8> {
    const PUBLISH_BYTES: [u8; 79] = [
        0x35, 0x4d, 0x00, 0x1a, 0x63, 0x75, 0x2f, 0x39, 0x2e, 0x30, 0x2e, 0x31, 0x2f, 0x72, 0x65, 0x70, 0x6c, 0x79, 0x2f, 0x72, 0x65, 0x73, 0x74, 0x61, 0x72, 0x74, 0x74, 0x65, 0x73, 0x74, 0x06, 0x47,
        0x29, 0x23, 0x00, 0x01, 0x09, 0x00, 0x0b, 0x43, 0x6f, 0x72, 0x72, 0x65, 0x6c, 0x61, 0x74, 0x69, 0x6f, 0x6e, 0x26, 0x00, 0x01, 0x41, 0x00, 0x01, 0x42, 0x26, 0x00, 0x01, 0x43, 0x00, 0x01, 0x44,
        0x03, 0x00, 0x07, 0x72, 0x65, 0x73, 0x74, 0x61, 0x72, 0x74, 0x68, 0x65, 0x6c, 0x6c, 0x6f,
    ];

    PUBLISH_BYTES.to_vec()
}
#[fixture]
fn publish_packet_2() -> Vec<u8> {
    const PUBLISH_BYTES: [u8; 76] = [
        0x34, 0x4a, 0x00, 0x1a, 0x63, 0x75, 0x2f, 0x39, 0x2e, 0x30, 0x2e, 0x31, 0x2f, 0x72, 0x65, 0x70, 0x6c, 0x79, 0x2f, 0x72, 0x65, 0x73, 0x74, 0x61, 0x72, 0x74, 0x74, 0x65, 0x73, 0x74, 0x00, 0x02,
        0x26, 0x03, 0x00, 0x07, 0x72, 0x65, 0x73, 0x74, 0x61, 0x72, 0x74, 0x09, 0x00, 0x0b, 0x43, 0x6f, 0x72, 0x72, 0x65, 0x6c, 0x61, 0x74, 0x69, 0x6f, 0x6e, 0x26, 0x00, 0x01, 0x41, 0x00, 0x01, 0x42,
        0x26, 0x00, 0x01, 0x43, 0x00, 0x01, 0x44, 0x68, 0x65, 0x6c, 0x6c, 0x6f,
    ];

    PUBLISH_BYTES.to_vec()
}

fn connect_packet() -> Vec<u8> {
    vec![
        0x10, 0xbe, 0x02, 0x00, 0x04, 0x4d, 0x51, 0x54, 0x54, 0x05, 0xc6, 0x00, 0x3c, 0x2a, 0x11, 0x00, 0x00, 0x00, 0x00, 0x21, 0x00, 0x7b, 0x27, 0x00, 0x00, 0x1f, 0x40, 0x22, 0x09, 0xc4, 0x19, 0x01,
        0x17, 0x01, 0x26, 0x00, 0x01, 0x41, 0x00, 0x01, 0x42, 0x26, 0x00, 0x02, 0x43, 0x44, 0x00, 0x08, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x4b, 0x4c, 0x00, 0x0e, 0x6d, 0x71, 0x74, 0x74, 0x78, 0x5f,
        0x36, 0x37, 0x62, 0x65, 0x32, 0x33, 0x38, 0x34, 0x06, 0x03, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x12, 0x61, 0x6e, 0x6f, 0x74, 0x68, 0x65, 0x72, 0x2f, 0x74, 0x65, 0x73, 0x74, 0x2f, 0x74, 0x6f,
        0x70, 0x69, 0x63, 0x00, 0xc5, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41,
        0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41,
        0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41,
        0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41,
        0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41,
        0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41,
        0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x00, 0x08, 0x54, 0x65, 0x73, 0x74, 0x54, 0x65, 0x73, 0x74, 0x00, 0x0b, 0x50, 0x61, 0x73, 0x73, 0x77, 0x6f, 0x72, 0x64, 0x31, 0x32,
        33,
    ]
}

pub fn subscribe_packet() -> Vec<u8> {
    vec![
        0x82, 0x22, 0x82, 0x02, 0x02, 0x0b, 0x7b, 0x00, 0x1a, 0x63, 0x75, 0x2f, 0x39, 0x2e, 0x30, 0x2e, 0x31, 0x2f, 0x72, 0x65, 0x70, 0x6c, 0x79, 0x2f, 0x72, 0x65, 0x73, 0x74, 0x61, 0x72, 0x74, 0x74,
        0x65, 0x73, 0x74, 0x2e,
    ]
}

#[rstest]
#[case(publish_packet_2())]
fn publish_packet_test(#[case] bytes: Vec<u8>) {
    let mut read_buffer = BytesMut::from_iter(bytes.iter());
    let mut write_buffer = BytesMut::new();
    let packet = Packet::read(&mut read_buffer).unwrap();
    packet.write(&mut write_buffer).unwrap();

    assert_eq!(bytes.len(), write_buffer.len());

    let packet_from_write_buffer = Packet::read(&mut write_buffer).unwrap();

    assert_eq!(packet, packet_from_write_buffer);
}

#[test]
fn test_connect() {
    let bytes = connect_packet();

    let mut read_buffer = BytesMut::from_iter(bytes.iter());
    let mut write_buffer = BytesMut::new();
    let packet = Packet::read(&mut read_buffer).unwrap();
    packet.write(&mut write_buffer).unwrap();

    if let Packet::Connect(p) = &packet {
        assert_eq!(42, p.connect_properties.wire_len());
        assert_eq!(6, p.last_will.as_ref().unwrap().last_will_properties.wire_len());
    }

    assert_eq!(bytes.len(), write_buffer.len());
    assert_eq!(bytes, write_buffer.to_vec());

    let packet_from_write_buffer = Packet::read(&mut write_buffer).unwrap();

    assert_eq!(packet, packet_from_write_buffer);
}

// In some cases the properties are not written in the same order as they are originally read.
#[rstest]
#[case(publish_packet())]
#[case(connect_packet())]
#[case(subscribe_packet())]
/// Test if the input == output after read packet form input and write packet to output
fn test_equal_read_write_packet_from_bytes(#[case] bytes: Vec<u8>) {
    let mut read_buffer = BytesMut::from_iter(bytes.iter());
    let mut write_buffer = BytesMut::new();
    let packet = Packet::read(&mut read_buffer).unwrap();
    packet.write(&mut write_buffer).unwrap();

    assert_eq!(bytes, write_buffer.to_vec());
}
