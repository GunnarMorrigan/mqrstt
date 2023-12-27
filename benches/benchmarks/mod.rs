use bytes::{Bytes, BytesMut, BufMut};
use mqrstt::packets::{Packet, ConnAck, ConnAckFlags, Publish, Disconnect};

pub mod tokio_concurrent;


fn fill_stuff(buffer: &mut BytesMut, publ_count: usize, publ_size: usize) {
    empty_connect(buffer);
    for i in 0..publ_count{
        very_large_publish(i as u16, publ_size/5).write(buffer).unwrap();
    }
    empty_disconnect().write(buffer).unwrap();
}

fn empty_disconnect() -> Packet{
    let discon = Disconnect{
        reason_code: mqrstt::packets::reason_codes::DisconnectReasonCode::ServerBusy,
        properties: Default::default(),
    };

    Packet::Disconnect(discon)
}

fn empty_connect(buffer: &mut BytesMut){
    // let conn_ack = ConnAck{
    //     connack_flags: ConnAckFlags::default(),
    //     reason_code: mqrstt::packets::reason_codes::ConnAckReasonCode::Success,
    //     connack_properties: Default::default(),
    // };

    // Packet::ConnAck(conn_ack)
    // buffer.put_u8(0b0010_0000); // Connack flags
    // buffer.put_u8(0x01); // Connack flags
    // buffer.put_u8(0x00); // Reason code,
    // buffer.put_u8(0x00); // empty properties

    buffer.put_u8(0x20);
    buffer.put_u8(0x13);
    buffer.put_u8(0x00);
    buffer.put_u8(0x00);
    buffer.put_u8(0x10);
    buffer.put_u8(0x27);
    buffer.put_u8(0x06);
    buffer.put_u8(0x40);
    buffer.put_u8(0x00);
    buffer.put_u8(0x00);
    buffer.put_u8(0x25);
    buffer.put_u8(0x01);
    buffer.put_u8(0x2a);
    buffer.put_u8(0x01);
    buffer.put_u8(0x29);
    buffer.put_u8(0x01);
    buffer.put_u8(0x22);
    buffer.put_u8(0xff);
    buffer.put_u8(0xff);
    buffer.put_u8(0x28);
    buffer.put_u8(0x01);
    

}


/// Returns Publish Packet with 5x `repeat` as payload in bytes.
fn very_large_publish(id: u16, repeat: usize) -> Packet {
    let publ = Publish{
        dup: false,
        qos: mqrstt::packets::QoS::ExactlyOnce,
        retain: false,
        topic: "BlaBla".into(),
        packet_identifier: Some(id),
        publish_properties: Default::default(),
        payload: Bytes::from_iter([0u8, 1u8, 2, 3, 4].repeat(repeat)),
    };

    Packet::Publish(publ)
}