use bytes::{BufMut, Bytes, BytesMut};
use mqrstt::packets::{Disconnect, Packet, Publish};

pub mod tokio;

fn fill_stuff(buffer: &mut BytesMut, publ_count: usize, publ_size: usize) {
    // empty_connect(buffer);
    // for i in 0..publ_count {
    //     very_large_publish(i as u16, publ_size / 5).write(buffer).unwrap();
    // }
    // empty_disconnect().write(buffer).unwrap();
}

fn empty_disconnect() -> Packet {
    // let discon = Disconnect {
    //     reason_code: mqrstt::packets::reason_codes::DisconnectReasonCode::ServerBusy,
    //     properties: Default::default(),
    // };

    // Packet::Disconnect(discon)
    todo!()
}

fn empty_connect(buffer: &mut BytesMut) {
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
    let publ = Publish {
        dup: false,
        qos: mqrstt::packets::QoS::ExactlyOnce,
        retain: false,
        topic: "BlaBla".into(),
        packet_identifier: Some(id),
        publish_properties: Default::default(),
        payload: Bytes::from_iter("ping".repeat(repeat).into_bytes()),
    };

    Packet::Publish(publ)
}

mod test_handlers {
    use std::{
        sync::{atomic::AtomicU16, Arc},
        time::Duration,
    };

    use bytes::Bytes;
    use mqrstt::{
        packets::{self, Packet},
        AsyncEventHandler, AsyncEventHandlerMut, MqttClient,
    };

    pub struct PingPong {
        pub client: MqttClient,
        pub number: Arc<AtomicU16>,
    }

    impl PingPong {
        pub fn new(client: MqttClient) -> Self {
            Self {
                client,
                number: Arc::new(AtomicU16::new(0)),
            }
        }
    }

    impl AsyncEventHandler for PingPong {
        async fn handle(&self, event: packets::Packet) -> () {
            self.number.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            match event {
                Packet::Publish(p) => {
                    if let Ok(payload) = String::from_utf8(p.payload.to_vec()) {
                        let max_len = payload.len().min(10);
                        let _a = &payload[0..max_len];
                        if payload.to_lowercase().contains("ping") {
                            self.client.publish(p.topic.clone(), p.qos, p.retain, Bytes::from_static(b"pong")).await.unwrap();
                        }
                    }
                }
                Packet::ConnAck(_) => (),
                _ => (),
            }
        }
    }

    impl AsyncEventHandlerMut for PingPong {
        async fn handle(&mut self, event: packets::Packet) -> () {
            self.number.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            match event {
                Packet::Publish(p) => {
                    if let Ok(payload) = String::from_utf8(p.payload.to_vec()) {
                        let max_len = payload.len().min(10);
                        let _a = &payload[0..max_len];
                        if payload.to_lowercase().contains("ping") {
                            self.client.publish(p.topic.clone(), p.qos, p.retain, Bytes::from_static(b"pong")).await.unwrap();
                        }
                    }
                }
                Packet::ConnAck(_) => (),
                _ => (),
            }
        }
    }

    pub struct SimpleDelay {
        delay: Duration,
    }

    impl SimpleDelay {
        pub fn new(delay: Duration) -> Self {
            Self { delay }
        }
    }

    impl AsyncEventHandler for SimpleDelay {
        fn handle(&self, _: Packet) -> impl futures::prelude::Future<Output = ()> + Send + Sync {
            tokio::time::sleep(self.delay)
        }
    }
    impl AsyncEventHandlerMut for SimpleDelay {
        fn handle(&mut self, _: Packet) -> impl futures::prelude::Future<Output = ()> + Send + Sync {
            tokio::time::sleep(self.delay)
        }
    }
}
