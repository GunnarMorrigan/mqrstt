use std::sync::Arc;

use futures::Future;

use crate::packets::Packet;

/// Handlers are used to deal with packets before they are further processed (acked)
/// This guarantees that the end user has handlded the packet.
/// Trait for async mutable access to handler.
/// Usefull when you have a single handler

/// This trait can be used types which 
pub trait AsyncEventHandler {
    fn handle(&self, incoming_packet: Packet) -> impl Future<Output = ()> + Send + Sync;
}
impl<T> AsyncEventHandler for &T where T: AsyncEventHandler {
    #[inline]
    fn handle(&self, incoming_packet: Packet) -> impl Future<Output = ()> + Send + Sync{
        AsyncEventHandler::handle(*self, incoming_packet)
    }
}
impl<T> AsyncEventHandler for Arc<T> where T: AsyncEventHandler {
    #[inline]
    fn handle(&self, incoming_packet: Packet) -> impl Future<Output = ()> + Send + Sync{
        <T>::handle(&self, incoming_packet)
    }
}
impl AsyncEventHandler for () {
    fn handle(&self, _: Packet) -> impl Future<Output = ()> + Send + Sync {
        async {}
    }
}

pub trait AsyncEventHandlerMut {
    fn handle(&mut self, incoming_packet: Packet) -> impl Future<Output = ()> + Send + Sync;
}

impl AsyncEventHandlerMut for () {
    fn handle(&mut self, _: Packet) -> impl Future<Output = ()> + Send + Sync {
        async {}
    }
}


pub trait EventHandler {
    fn handle(&mut self, incoming_packet: Packet);
}

impl EventHandler for () {
    fn handle(&mut self, _: Packet) {}
}

pub mod example_handlers{
    use std::{sync::atomic::AtomicU16, ops::AddAssign};

    use bytes::Bytes;

    use crate::{AsyncEventHandlerMut, packets::{Packet, self}, EventHandler, MqttClient, AsyncEventHandler};

    /// Most basic no op handler
    /// This handler performs no operations on incoming messages.
    pub struct NOP {}

    impl AsyncEventHandlerMut for NOP {
        async fn handle(&mut self, _: Packet) {}
    }

    impl EventHandler for NOP {
        fn handle(&mut self, _: Packet) {}
    }

    pub struct PingResp {
        pub client: MqttClient,
        pub ping_resp_received: AtomicU16,
    }

    impl PingResp {
        pub fn new(client: MqttClient) -> Self {
            Self {
                client,
                ping_resp_received: AtomicU16::new(0),
            }
        }
    }

    impl AsyncEventHandlerMut for PingResp {
        async fn handle(&mut self, event: packets::Packet) -> () {
            use Packet::*;
            if event == PingResp {
                self.ping_resp_received.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
            println!("Received packet: {}", event);
        }
    }

    impl EventHandler for PingResp {
        fn handle(&mut self, event: Packet) {
            use Packet::*;
            if event == PingResp {
                self.ping_resp_received.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
            println!("Received packet: {}", event);
        }
    }

    pub struct PingPong {
        pub client: MqttClient,
        pub number: AtomicU16,
    }

    impl PingPong{
        pub fn new(client: MqttClient) -> Self {
            Self { 
                client,
                number: AtomicU16::new(0),
            }
        }
    }

    impl AsyncEventHandler for PingPong {
        async fn handle(&self, event: packets::Packet) -> () {
            match event {
                Packet::Publish(p) => {
                    if let Ok(payload) = String::from_utf8(p.payload.to_vec()) {
                        let max_len = payload.len().min(10);
                        let a = &payload[0..max_len];
                        if payload.to_lowercase().contains("ping") {
                            self.client.publish(p.topic.clone(), p.qos, p.retain, Bytes::from_static(b"pong")).await.unwrap();
                            println!("Received publish payload: {}", a);
                            
                            if !p.retain{
                                self.number.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            }

                            println!("DBG: \n {}", &Packet::Publish(p));
                        }
                    }
                }
                Packet::ConnAck(_) => {
                    println!("Connected!")
                }
                _ => (),
            }
        }
    }

    impl AsyncEventHandlerMut for PingPong {
        async fn handle(&mut self, event: packets::Packet) -> () {
            match event {
                Packet::Publish(p) => {
                    if let Ok(payload) = String::from_utf8(p.payload.to_vec()) {
                        let max_len = payload.len().min(10);
                        let a = &payload[0..max_len];
                        if payload.to_lowercase().contains("ping") {
                            self.client.publish(p.topic.clone(), p.qos, p.retain, Bytes::from_static(b"pong")).await.unwrap();
                            println!("Received publish payload: {}", a);
                            
                            if !p.retain{
                                self.number.get_mut().add_assign(1);
                            }

                            println!("DBG: \n {}", &Packet::Publish(p));
                        }
                    }
                }
                Packet::ConnAck(_) => {
                    println!("Connected!")
                }
                _ => (),
            }
        }
    }

    #[cfg(feature = "sync")]
    impl EventHandler for PingPong {
        fn handle(&mut self, event: Packet) {
            match event {
                Packet::Publish(p) => {
                    if let Ok(payload) = String::from_utf8(p.payload.to_vec()) {
                        if payload.to_lowercase().contains("ping") {
                            self.client.publish_blocking(p.topic.clone(), p.qos, p.retain, Bytes::from_static(b"pong")).unwrap();
                        }
                    }
                }
                Packet::ConnAck(_) => {
                    println!("Connected!")
                }
                _ => (),
            }
        }
    }
}