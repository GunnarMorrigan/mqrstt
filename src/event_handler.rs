use crate::connect_options::ConnectOptions;
use crate::error::MqttError;
use crate::packets::Disconnect;
use crate::packets::{Packet, PacketType};
use crate::packets::{PubAck, PubAckProperties};
use crate::packets::PubComp;
use crate::packets::Publish;
use crate::packets::PubRec;
use crate::packets::PubRel;
use crate::packets::reason_codes::{PubAckReasonCode, PubRecReasonCode};
use crate::packets::SubAck;
use crate::packets::Subscribe;
use crate::packets::UnsubAck;
use crate::packets::Unsubscribe;
use crate::packets::QoS;
use crate::state::State;

use futures::FutureExt;
use futures_concurrency::future::Race;

use async_channel::{Receiver, Sender};
use async_mutex::Mutex;
use tracing::{error, debug};

use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Eventloop with all the state of a connection
pub struct EventHandlerTask {
    /// Options of the current mqtt connection
    // options: ConnectOptions,
    /// Current state of the connection
    state: State,

    network_receiver: Receiver<Packet>,

    network_sender: Sender<Packet>,

    client_to_handler_r: Receiver<Packet>,

    last_network_action: Arc<Mutex<Instant>>,
    waiting_for_pingresp: AtomicBool,
    keep_alive_s: u64,
    disconnect: AtomicBool,
}

impl EventHandlerTask {
    /// New MQTT `EventLoop`
    ///
    /// When connection encounters critical errors (like auth failure), user has a choice to
    /// access and update `options`, `state` and `requests`.
    pub(crate) fn new(
        options: &ConnectOptions,
        network_receiver: Receiver<Packet>,
        network_sender: Sender<Packet>,
        client_to_handler_r: Receiver<Packet>,
        last_network_action: Arc<Mutex<Instant>>,
    ) -> (Self, Receiver<u16>) {
        let (state, packet_id_channel) = State::new(options.receive_maximum());

        let task = EventHandlerTask {
            state,

            network_receiver,
            network_sender,

            client_to_handler_r,

            last_network_action,
            waiting_for_pingresp: AtomicBool::new(false),
            keep_alive_s: options.keep_alive_interval_s,
            disconnect: AtomicBool::new(false),
        };
        (task, packet_id_channel)
    }

    pub async fn handle<H: EventHandler + Send + Sized + 'static>(
        &self,
        handler: &mut H,
    ) -> Result<(), MqttError> {
        futures::select! {
            incoming = self.network_receiver.recv().fuse() => {
                match incoming {
                    Ok(event) => {
                        // debug!("Event Handler, handling incoming packet: {}", event);
                        self.handle_incoming_packet(handler, event).await?
                    }
                    Err(err) => return Err(MqttError::IncomingNetworkChannelClosed(err)),
                }
                if self.disconnect.load(Ordering::Acquire) {
                    self.disconnect.store(false, Ordering::Release);
                    return Ok::<(), MqttError>(());
                }
            },
            outgoing = self.client_to_handler_r.recv().fuse() => {
                match outgoing {
                    Ok(event) => {
                        // debug!("Event Handler, handling outgoing packet: {}", event);
                        self.handle_outgoing_packet(event).await?
                    }
                    Err(err) => return Err(MqttError::ClientChannelClosed(err)),
                }
                if self.disconnect.load(Ordering::Acquire) {
                    self.disconnect.store(false, Ordering::Release);
                    return Ok::<(), MqttError>(());
                }
            }
        }
        Ok(())
        // let keepalive = async {
        //     let initial_keep_alive_duration = std::time::Duration::new(self.keep_alive_s, 0);
        //     let mut keep_alive_duration = std::time::Duration::new(self.keep_alive_s, 0);
        //     loop {
        //         warn!("Awaiting PING sleep");
        //         #[cfg(feature = "tokio")]
        //         tokio::time::sleep(keep_alive_duration).await;

        //         if (!self.waiting_for_pingresp.load(Ordering::Acquire))
        //             && self.last_network_action.lock().await.elapsed()
        //                 >= initial_keep_alive_duration
        //         {
        //             self.network_sender.send(Packet::PingReq).await?;
        //             self.waiting_for_pingresp.store(true, Ordering::Release);
        //             keep_alive_duration = initial_keep_alive_duration;
        //         } else {
        //             keep_alive_duration = initial_keep_alive_duration
        //                 - self.last_network_action.lock().await.elapsed();
        //         }
        //         if self.disconnect.load(Ordering::Acquire) {
        //             return Ok::<(), MqttError>(());
        //         }
        //     }
        // };
    }

    async fn handle_incoming_packet<H: EventHandler>(
        &self,
        handler: &mut H,
        packet: Packet,
    ) -> Result<(), MqttError> {
        handler.handle(&packet).await;

        match packet {
            Packet::Publish(publish) => self.handle_incoming_publish(&publish).await?,
            Packet::PubAck(puback) => self.handle_incoming_puback(&puback).await?,
            Packet::PubRec(pubrec) => self.handle_incoming_pubrec(&pubrec).await?,
            Packet::PubRel(pubrel) => self.handle_incoming_pubrel(&pubrel).await?,
            Packet::PubComp(pubcomp) => self.handle_incoming_pubcomp(&pubcomp).await?,
            Packet::SubAck(suback) => self.handle_incoming_suback(suback).await?,
            Packet::UnsubAck(unsuback) => self.handle_incoming_unsuback(unsuback).await?,
            Packet::PingResp => self.handle_incoming_pingresp().await,
            Packet::ConnAck(_) => (),
            Packet::Disconnect(_) => {
                self.disconnect.store(true, Ordering::Release);
            },
            a => unreachable!("Should not receive {}", a),
        };
        Ok(())
    }

    async fn handle_incoming_publish(&self, publish: &Publish) -> Result<(), MqttError> {
        match publish.qos {
            QoS::AtMostOnce => Ok(()),
            QoS::AtLeastOnce => {
                let puback = PubAck {
                    packet_identifier: publish
                        .packet_identifier
                        .ok_or(MqttError::MissingPacketId)?,
                    reason_code: PubAckReasonCode::Success,
                    properties: PubAckProperties::default(),
                };
                self.network_sender.send(Packet::PubAck(puback)).await?;
                Ok(())
            }
            QoS::ExactlyOnce => {
                let pkid = publish
                    .packet_identifier
                    .ok_or(MqttError::MissingPacketId)?;
                let mut incoming_pub = self.state.incoming_pub.lock().await;
                if !incoming_pub.insert(pkid) && !publish.dup {
                    error!(
                        "Received publish with an packet ID ({}) that is in use and the packet was not a duplicate",
                        pkid,
                    );
                }

                let pubrec = PubRec::new(pkid);
                self.network_sender.send(Packet::PubRec(pubrec)).await?;

                Ok(())
            }
        }
    }

    async fn handle_incoming_puback(&self, puback: &PubAck) -> Result<(), MqttError> {
        let mut outgoing_pub = self.state.outgoing_pub.lock().await;
        if let Some(_) = outgoing_pub.remove(&puback.packet_identifier) {
            #[cfg(test)]
            debug!(
                "Publish {:?} has been acknowledged",
                puback.packet_identifier
            );
            self.state
                .apkid
                .mark_available(puback.packet_identifier)
                .await;
            Ok(())
        } else {
            error!(
                "Publish {:?} was not found, while receiving a PubAck for it",
                puback.packet_identifier,
            );
            Err(MqttError::Unsolicited(
                puback.packet_identifier,
                PacketType::PubAck,
            ))
        }
    }

    async fn handle_incoming_pubrec(&self, pubrec: &PubRec) -> Result<(), MqttError> {
        let mut outgoing_pub = self.state.outgoing_pub.lock().await;
        match outgoing_pub.remove(&pubrec.packet_identifier) {
            Some(_) => match pubrec.reason_code {
                PubRecReasonCode::Success | PubRecReasonCode::NoMatchingSubscribers => {
                    let pubrel = PubRel::new(pubrec.packet_identifier);
                    {
                        let mut outgoing_rel = self.state.outgoing_rel.lock().await;
                        outgoing_rel.insert(pubrec.packet_identifier);
                    }
                    self.network_sender.send(Packet::PubRel(pubrel)).await?;

                    #[cfg(test)]
                    debug!(
                        "Publish {:?} has been PubReced",
                        pubrec.packet_identifier
                    );
                    Ok(())
                }
                _ => Ok(()),
            },
            None => {
                error!(
                    "Publish {} was not found, while receiving a PubRec for it",
                    pubrec.packet_identifier,
                );
                Err(MqttError::Unsolicited(
                    pubrec.packet_identifier,
                    PacketType::PubRec,
                ))
            }
        }
    }

    async fn handle_incoming_pubrel(&self, pubrel: &PubRel) -> Result<(), MqttError> {
        let pubcomp = PubComp::new(pubrel.packet_identifier);
        self.network_sender.send(Packet::PubComp(pubcomp)).await?;
        Ok(())
    }

    async fn handle_incoming_pubcomp(&self, pubcomp: &PubComp) -> Result<(), MqttError> {
        let mut outgoing_rel = self.state.outgoing_rel.lock().await;
        if outgoing_rel.remove(&pubcomp.packet_identifier) {
            self.state
                .apkid
                .mark_available(pubcomp.packet_identifier)
                .await;
            Ok(())
        } else {
            error!(
                "PubRel {} was not found, while receiving a PubComp for it",
                pubcomp.packet_identifier,
            );
            Err(MqttError::Unsolicited(
                pubcomp.packet_identifier,
                PacketType::PubComp,
            ))
        }
    }

    async fn handle_incoming_pingresp(&self) {
        self.waiting_for_pingresp.store(false, Ordering::Release);
    }

    async fn handle_incoming_suback(&self, suback: SubAck) -> Result<(), MqttError> {
        let mut subs = self.state.outgoing_sub.lock().await;
        if subs.remove(&suback.packet_identifier).is_some() {
            self.state
                .apkid
                .mark_available(suback.packet_identifier)
                .await;
            Ok(())
        } else {
            error!(
                "Sub {} was not found, while receiving a SubAck for it",
                suback.packet_identifier,
            );
            Err(MqttError::Unsolicited(
                suback.packet_identifier,
                PacketType::SubAck,
            ))
        }
    }

    async fn handle_incoming_unsuback(&self, unsuback: UnsubAck) -> Result<(), MqttError> {
        let mut unsubs = self.state.outgoing_unsub.lock().await;
        if unsubs.remove(&unsuback.packet_identifier).is_some() {
            self.state
                .apkid
                .mark_available(unsuback.packet_identifier)
                .await;
            Ok(())
        } else {
            error!(
                "Unsub {} was not found, while receiving a unsuback for it",
                unsuback.packet_identifier,
            );
            Err(MqttError::Unsolicited(
                unsuback.packet_identifier,
                PacketType::UnsubAck,
            ))
        }
    }

    async fn handle_outgoing_packet(&self, packet: Packet) -> Result<(), MqttError> {
        match packet {
            Packet::Publish(publish) => self.handle_outgoing_publish(publish).await,
            Packet::Subscribe(sub) => self.handle_outgoing_subscribe(sub).await,
            Packet::Unsubscribe(unsub) => self.handle_outgoing_unsubscribe(unsub).await,
            Packet::Disconnect(d) => self.handle_outgoing_disconnect(d).await,
            _ => unreachable!(),
        }
    }

    async fn handle_outgoing_publish(&self, publish: Publish) -> Result<(), MqttError> {
        match publish.qos {
            QoS::AtMostOnce => {
                self.network_sender.send(Packet::Publish(publish)).await?;
            }
            QoS::AtLeastOnce => {
                self.network_sender
                    .send(Packet::Publish(publish.clone()))
                    .await?;
                let mut outgoing_pub = self.state.outgoing_pub.lock().await;
                if let Some(pub_collision) =
                    outgoing_pub.insert(publish.packet_identifier.unwrap(), publish)
                {
                    error!(
                        "Encountered a colliding packet ID ({:?}) in a publish QoS 1 packet",
                        pub_collision.packet_identifier,
                    )
                }
            }
            QoS::ExactlyOnce => {
                self.network_sender
                    .send(Packet::Publish(publish.clone()))
                    .await?;
                let mut outgoing_pub = self.state.outgoing_pub.lock().await;
                if let Some(pub_collision) =
                    outgoing_pub.insert(publish.packet_identifier.unwrap(), publish)
                {
                    error!(
                        "Encountered a colliding packet ID ({:?}) in a publish QoS 2 packet",
                        pub_collision.packet_identifier,
                    )
                }
            }
        }
        Ok(())
    }

    async fn handle_outgoing_subscribe(&self, sub: Subscribe) -> Result<(), MqttError> {
        let mut outgoing_sub = self.state.outgoing_sub.lock().await;
        if outgoing_sub
            .insert(sub.packet_identifier, sub.clone())
            .is_some()
        {
            error!(
                "Encountered a colliding packet ID ({}) in a subscribe packet",
                sub.packet_identifier,
            )
        } else {
            self.network_sender.send(Packet::Subscribe(sub)).await?;
        }
        Ok(())
    }

    async fn handle_outgoing_unsubscribe(&self, unsub: Unsubscribe) -> Result<(), MqttError> {
        let mut outgoing_unsub = self.state.outgoing_unsub.lock().await;
        if outgoing_unsub
            .insert(unsub.packet_identifier, unsub.clone())
            .is_some()
        {
            error!(
                "Encountered a colliding packet ID ({}) in a unsubscribe packet",
                unsub.packet_identifier,
            )
        } else {
            self.network_sender.send(Packet::Unsubscribe(unsub)).await?;
        }
        Ok(())
    }

    async fn handle_outgoing_disconnect(&self, disconnect: Disconnect) -> Result<(), MqttError> {
        self.disconnect.store(true, Ordering::Release);
        self.network_sender
            .send(Packet::Disconnect(disconnect))
            .await?;
        Ok(())
    }
}

pub trait EventHandler: Sized + Sync + 'static {
    fn handle<'a>(&'a mut self, event: &'a Packet) -> impl Future<Output = ()> + Send + 'a;
}

#[cfg(test)]
mod handler_tests {
    use std::{sync::Arc, time::Duration};

    use async_channel::{Receiver, Sender};
    use async_mutex::Mutex;

    use crate::{
        connect_options::ConnectOptions,
        event_handler::{EventHandler, EventHandlerTask},
        packets::{
            {Packet, PacketType},
            {PubComp, PubCompProperties},
            {PubRec, PubRecProperties},
            {PubRel, PubRelProperties},
            reason_codes::{
                PubCompReasonCode, PubRecReasonCode, PubRelReasonCode, SubAckReasonCode,
            },
            {SubAck, SubAckProperties},
            QoS,
        },
        tests::resources::test_packets::{
            create_disconnect_packet, create_puback_packet, create_publish_packet,
            create_subscribe_packet,
        },
    };

    pub struct Nop {}
    impl EventHandler for Nop {
        fn handle<'a>(
            &'a mut self,
            _event: &'a Packet,
        ) -> impl core::future::Future<Output = ()> + Send + 'a {
            async move { () }
        }
    }

    fn handler() -> (
        EventHandlerTask,
        Receiver<Packet>,
        Sender<Packet>,
        Sender<Packet>,
    ) {
        let opt = ConnectOptions::new("127.0.0.1".to_string(), 1883, "test123123".to_string());

        let (to_network_s, to_network_r) = async_channel::bounded(100);
        let (network_to_handler_s, network_to_handler_r) = async_channel::bounded(100);
        let (client_to_handler_s, client_to_handler_r) = async_channel::bounded(100);

        let (handler, _apkid) = EventHandlerTask::new(
            &opt,
            network_to_handler_r,
            to_network_s,
            client_to_handler_r,
            Arc::new(Mutex::new(
                std::time::Instant::now() + Duration::new(600, 0),
            )),
        );
        (
            handler,
            to_network_r,
            network_to_handler_s,
            client_to_handler_s,
        )
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn outgoing_publish_qos_0() {
        let mut nop = Nop {};

        let (handler, to_network_r, _network_to_handler_s, client_to_handler_s) = handler();

        let handler_task = tokio::task::spawn(async move {
            // Ignore the error that this will return
            let _ = dbg!(handler.handle(&mut nop).await);
            return handler;
        });
        let pub_packet = create_publish_packet(QoS::AtMostOnce, false, false, None);

        client_to_handler_s.send(pub_packet.clone()).await.unwrap();

        let packet = to_network_r.recv().await.unwrap();

        assert_eq!(packet, pub_packet);

        // If we drop the client to handler channel the handler will stop executing and we can inspect its internals.
        drop(client_to_handler_s);

        let handler = handler_task.await.unwrap();

        assert!(handler.state.incoming_pub.lock().await.is_empty());
        assert!(handler.state.outgoing_pub.lock().await.is_empty());
        assert!(handler.state.outgoing_rel.lock().await.is_empty());
        assert!(handler.state.outgoing_sub.lock().await.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn outgoing_publish_qos_1() {
        pub struct TestPubQoS1 {
            stage: StagePubQoS1,
        }
        pub enum StagePubQoS1 {
            PubAck,
            Done,
        }
        impl TestPubQoS1 {
            fn new() -> Self {
                TestPubQoS1 {
                    stage: StagePubQoS1::PubAck,
                }
            }
        }
        impl EventHandler for TestPubQoS1 {
            fn handle<'a>(
                &'a mut self,
                event: &'a Packet,
            ) -> impl core::future::Future<Output = ()> + Send + 'a {
                async move {
                    match self.stage {
                        StagePubQoS1::PubAck => {
                            assert_eq!(event.packet_type(), PacketType::PubAck);
                            self.stage = StagePubQoS1::Done;
                        }
                        StagePubQoS1::Done => (),
                    }
                }
            }
        }

        let mut nop = TestPubQoS1::new();

        let (handler, to_network_r, network_to_handler_s, client_to_handler_s) = handler();

        let handler_task = tokio::task::spawn(async move {
            // Ignore the error that this will return
            let _ = dbg!(handler.handle(&mut nop).await);
            return handler;
        });
        let pub_packet = create_publish_packet(QoS::AtLeastOnce, false, false, Some(1));

        client_to_handler_s.send(pub_packet.clone()).await.unwrap();

        let publish = to_network_r.recv().await.unwrap();

        assert_eq!(pub_packet, publish);

        let puback = create_puback_packet(1);

        network_to_handler_s.send(puback).await.unwrap();

        tokio::time::sleep(Duration::new(5, 0)).await;

        // If we drop the client_to_handler channel the handler will stop executing and we can inspect its internals.
        drop(client_to_handler_s);

        let handler = handler_task.await.unwrap();

        assert!(handler.state.incoming_pub.lock().await.is_empty());
        assert!(handler.state.outgoing_pub.lock().await.is_empty());
        assert!(handler.state.outgoing_rel.lock().await.is_empty());
        assert!(handler.state.outgoing_sub.lock().await.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn incoming_publish_qos_q(){

        let mut nop = Nop{};

        let (handler, to_network_r, network_to_handler_s, client_to_handler_s) = handler();

        let handler_task = tokio::task::spawn(async move {
            // Ignore the error that this will return
            let _ = dbg!(handler.handle(&mut nop).await);
            return handler;
        });
        let pub_packet = create_publish_packet(QoS::AtLeastOnce, false, false, Some(1));

        network_to_handler_s.send(pub_packet.clone()).await.unwrap();

        let puback = to_network_r.recv().await.unwrap();

        assert_eq!(PacketType::PubAck, puback.packet_type());

        let expected_puback = create_puback_packet(1);

        assert_eq!(expected_puback, puback);

        // If we drop the client_to_handler channel the handler will stop executing and we can inspect its internals.
        drop(client_to_handler_s);

        let handler = handler_task.await.unwrap();

        assert!(handler.state.incoming_pub.lock().await.is_empty());
        assert!(handler.state.outgoing_pub.lock().await.is_empty());
        assert!(handler.state.outgoing_rel.lock().await.is_empty());
        assert!(handler.state.outgoing_sub.lock().await.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn outgoing_publish_qos_2() {
        pub struct TestPubQoS2 {
            stage: StagePubQoS2,
            client_to_handler_s: Sender<Packet>,
        }
        pub enum StagePubQoS2 {
            PubRec,
            PubComp,
            Done,
        }
        impl TestPubQoS2 {
            fn new(client_to_handler_s: Sender<Packet>) -> Self {
                TestPubQoS2 {
                    stage: StagePubQoS2::PubRec,
                    client_to_handler_s,
                }
            }
        }
        impl EventHandler for TestPubQoS2 {
            fn handle<'a>(
                &'a mut self,
                event: &'a Packet,
            ) -> impl core::future::Future<Output = ()> + Send + 'a {
                async move {
                    match self.stage {
                        StagePubQoS2::PubRec => {
                            assert_eq!(event.packet_type(), PacketType::PubRec);
                            self.stage = StagePubQoS2::PubComp;
                        }
                        StagePubQoS2::PubComp => {
                            assert_eq!(event.packet_type(), PacketType::PubComp);
                            self.stage = StagePubQoS2::Done;
                            self.client_to_handler_s
                                .send(create_disconnect_packet())
                                .await
                                .unwrap();
                        }
                        StagePubQoS2::Done => (),
                    }
                }
            }
        }

        let (handler, to_network_r, network_to_handler_s, client_to_handler_s) = handler();

        let mut nop = TestPubQoS2::new(client_to_handler_s.clone());

        let handler_task = tokio::task::spawn(async move {
            dbg!(handler.handle(&mut nop).await).unwrap();
            return handler;
        });
        let pub_packet = create_publish_packet(QoS::AtLeastOnce, false, false, Some(1));

        client_to_handler_s.send(pub_packet.clone()).await.unwrap();

        let publish = to_network_r.recv().await.unwrap();

        assert_eq!(pub_packet, publish);

        let pubrec = Packet::PubRec(PubRec {
            packet_identifier: 1,
            reason_code: PubRecReasonCode::Success,
            properties: PubRecProperties::default(),
        });

        network_to_handler_s.send(pubrec).await.unwrap();

        let packet = to_network_r.recv().await.unwrap();

        let expected_pubrel = Packet::PubRel(PubRel {
            packet_identifier: 1,
            reason_code: PubRelReasonCode::Success,
            properties: PubRelProperties::default(),
        });

        assert_eq!(expected_pubrel, packet);

        let pubcomp = Packet::PubComp(PubComp {
            packet_identifier: 1,
            reason_code: PubCompReasonCode::Success,
            properties: PubCompProperties::default(),
        });

        network_to_handler_s.send(pubcomp).await.unwrap();

        let handler = handler_task.await.unwrap();

        assert!(handler.state.incoming_pub.lock().await.is_empty());
        assert!(handler.state.outgoing_pub.lock().await.is_empty());
        assert!(handler.state.outgoing_rel.lock().await.is_empty());
        assert!(handler.state.outgoing_sub.lock().await.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn outgoing_subscribe() {
        let (handler, to_network_r, network_to_handler_s, client_to_handler_s) = handler();

        let mut nop = Nop {};

        let handler_task = tokio::task::spawn(async move {
            // Ignore the error that this will return
            let _ = dbg!(handler.handle(&mut nop).await);
            return handler;
        });

        let sub_packet = create_subscribe_packet(1);

        client_to_handler_s.send(sub_packet.clone()).await.unwrap();

        let sub_result = to_network_r.recv().await.unwrap();

        assert_eq!(sub_packet, sub_result);

        let suback = Packet::SubAck(SubAck {
            packet_identifier: 1,
            reason_codes: vec![SubAckReasonCode::GrantedQoS0],
            properties: SubAckProperties::default(),
        });

        network_to_handler_s.send(suback).await.unwrap();

        tokio::time::sleep(Duration::new(2, 0)).await;

        drop(client_to_handler_s);

        let handler = handler_task.await.unwrap();
        assert!(handler.state.incoming_pub.lock().await.is_empty());
        assert!(handler.state.outgoing_pub.lock().await.is_empty());
        assert!(handler.state.outgoing_rel.lock().await.is_empty());
        assert!(handler.state.outgoing_sub.lock().await.is_empty());
    }
}
