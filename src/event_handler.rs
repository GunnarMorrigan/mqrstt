use crate::connect_options::ConnectOptions;
use crate::error::MqttError;
use crate::packets::reason_codes::{PubAckReasonCode, PubRecReasonCode};
use crate::packets::Disconnect;
use crate::packets::PubComp;
use crate::packets::PubRec;
use crate::packets::PubRel;
use crate::packets::Publish;
use crate::packets::QoS;
use crate::packets::SubAck;
use crate::packets::Subscribe;
use crate::packets::UnsubAck;
use crate::packets::Unsubscribe;
use crate::packets::{Packet, PacketType};
use crate::packets::{PubAck, PubAckProperties};
use crate::state::State;
use crate::{AsyncEventHandler, AsyncEventHandlerMut, HandlerStatus};

use futures::FutureExt;

use async_channel::{Receiver, Sender};
use tracing::error;

#[cfg(test)]
use tracing::debug;

/// Eventloop with all the state of a connection
pub struct EventHandlerTask {
    state: State,

    network_receiver: Receiver<Packet>,

    network_sender: Sender<Packet>,

    client_to_handler_r: Receiver<Packet>,

    disconnect: bool,
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
    ) -> (Self, Receiver<u16>) {
        let (state, packet_id_channel) = State::new(options.receive_maximum());

        let task = EventHandlerTask {
            state,

            network_receiver,
            network_sender,

            client_to_handler_r,

            disconnect: false,
        };
        (task, packet_id_channel)
    }

    // pub fn sync_handle<H: EventHandler>(&self, handler: &mut H) -> Result<(), MqttError> {
    // 	match self.network_receiver.try_recv() {
    // 		Ok(event) => {
    // 			handler.handle(&event);
    // 		}
    // 		Err(err) => {
    // 			if err.is_closed() {
    // 				return Err(MqttError::IncomingNetworkChannelClosed);
    // 			}
    // 		}
    // 	}
    // 	match self.client_to_handler_r.try_recv() {
    // 		Ok(_) => {}
    // 		Err(err) => {
    // 			if err.is_closed() {
    // 				return Err(MqttError::IncomingNetworkChannelClosed);
    // 			}
    // 		}
    // 	}
    // 	Ok(())
    // }

    pub async fn handle<H>(&mut self, handler: &H) -> Result<HandlerStatus, MqttError>
    where
        H: AsyncEventHandler, {
        futures::select! {
            incoming = self.network_receiver.recv().fuse() => {
                match incoming {
                    Ok(event) => {
                        // debug!("Event Handler, handling incoming packet: {}", event);
                        handler.handle(&event).await;
                        self.handle_incoming_packet(event).await?;
                    }
                    Err(_) => return Err(MqttError::IncomingNetworkChannelClosed),
                }
                if self.disconnect {
                    self.disconnect = true;
                    return Ok(HandlerStatus::IncomingDisconnect);
                }
            },
            outgoing = self.client_to_handler_r.recv().fuse() => {
                match outgoing {
                    Ok(event) => {
                        // debug!("Event Handler, handling outgoing packet: {}", event);
                        self.handle_outgoing_packet(event).await?
                    }
                    Err(_) => return Err(MqttError::ClientChannelClosed),
                }
                if self.disconnect {
                    self.disconnect = true;
                    return Ok(HandlerStatus::OutgoingDisconnect);
                }
            }
        }
        Ok(HandlerStatus::Active)
    }

    pub async fn handle_mut<H>(&mut self, handler: &mut H) -> Result<HandlerStatus, MqttError>
    where
        H: AsyncEventHandlerMut, {
        futures::select! {
            incoming = self.network_receiver.recv().fuse() => {
                match incoming {
                    Ok(event) => {
                        // debug!("Event Handler, handling incoming packet: {}", event);
                        handler.handle(&event).await;
                        self.handle_incoming_packet(event).await?;
                    }
                    Err(_) => return Err(MqttError::IncomingNetworkChannelClosed),
                }
                if self.disconnect {
                    self.disconnect = true;
                    return Ok(HandlerStatus::IncomingDisconnect);
                }
            },
            outgoing = self.client_to_handler_r.recv().fuse() => {
                match outgoing {
                    Ok(event) => {
                        // debug!("Event Handler, handling outgoing packet: {}", event);
                        self.handle_outgoing_packet(event).await?
                    }
                    Err(_) => return Err(MqttError::ClientChannelClosed),
                }
                if self.disconnect {
                    self.disconnect = true;
                    return Ok(HandlerStatus::OutgoingDisconnect);
                }
            }
        }
        Ok(HandlerStatus::Active)
    }

    async fn handle_incoming_packet(&mut self, packet: Packet) -> Result<(), MqttError> {
        match packet {
            Packet::Publish(publish) => self.handle_incoming_publish(&publish).await?,
            Packet::PubAck(puback) => self.handle_incoming_puback(&puback).await?,
            Packet::PubRec(pubrec) => self.handle_incoming_pubrec(&pubrec).await?,
            Packet::PubRel(pubrel) => self.handle_incoming_pubrel(&pubrel).await?,
            Packet::PubComp(pubcomp) => self.handle_incoming_pubcomp(&pubcomp).await?,
            Packet::SubAck(suback) => self.handle_incoming_suback(suback).await?,
            Packet::UnsubAck(unsuback) => self.handle_incoming_unsuback(unsuback).await?,
            Packet::PingResp => (),
            Packet::ConnAck(_) => (),
            Packet::Disconnect(_) => {
                self.disconnect = true;
            }
            a => unreachable!("Should not receive {}", a),
        };
        Ok(())
    }

    async fn handle_incoming_publish(&mut self, publish: &Publish) -> Result<(), MqttError> {
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
                if !self.state.incoming_pub.insert(pkid) && !publish.dup {
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

    async fn handle_incoming_puback(&mut self, puback: &PubAck) -> Result<(), MqttError> {
        if self
            .state
            .outgoing_pub
            .remove(&puback.packet_identifier)
            .is_some()
        {
            #[cfg(test)]
            debug!(
                "Publish {:?} has been acknowledged",
                puback.packet_identifier
            );
            self.state
                .apkid
                .mark_available(puback.packet_identifier)
                .await?;
            Ok(())
        }
        else {
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

    async fn handle_incoming_pubrec(&mut self, pubrec: &PubRec) -> Result<(), MqttError> {
        match self.state.outgoing_pub.remove(&pubrec.packet_identifier) {
            Some(_) => match pubrec.reason_code {
                PubRecReasonCode::Success | PubRecReasonCode::NoMatchingSubscribers => {
                    let pubrel = PubRel::new(pubrec.packet_identifier);
                    self.state.outgoing_rel.insert(pubrec.packet_identifier);
                    self.network_sender.send(Packet::PubRel(pubrel)).await?;

                    #[cfg(test)]
                    debug!("Publish {:?} has been PubReced", pubrec.packet_identifier);
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

    async fn handle_incoming_pubcomp(&mut self, pubcomp: &PubComp) -> Result<(), MqttError> {
        if self.state.outgoing_rel.remove(&pubcomp.packet_identifier) {
            self.state
                .apkid
                .mark_available(pubcomp.packet_identifier)
                .await?;
            Ok(())
        }
        else {
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

    async fn handle_incoming_suback(&mut self, suback: SubAck) -> Result<(), MqttError> {
        if self
            .state
            .outgoing_sub
            .remove(&suback.packet_identifier)
            .is_some()
        {
            self.state
                .apkid
                .mark_available(suback.packet_identifier)
                .await?;
            Ok(())
        }
        else {
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

    async fn handle_incoming_unsuback(&mut self, unsuback: UnsubAck) -> Result<(), MqttError> {
        if self
            .state
            .outgoing_unsub
            .remove(&unsuback.packet_identifier)
            .is_some()
        {
            self.state
                .apkid
                .mark_available(unsuback.packet_identifier)
                .await?;
            Ok(())
        }
        else {
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

    async fn handle_outgoing_packet(&mut self, packet: Packet) -> Result<(), MqttError> {
        match packet {
            Packet::Publish(publish) => self.handle_outgoing_publish(publish).await,
            Packet::Subscribe(sub) => self.handle_outgoing_subscribe(sub).await,
            Packet::Unsubscribe(unsub) => self.handle_outgoing_unsubscribe(unsub).await,
            Packet::Disconnect(d) => self.handle_outgoing_disconnect(d).await,
            _ => unreachable!(),
        }
    }

    async fn handle_outgoing_publish(&mut self, publish: Publish) -> Result<(), MqttError> {
        match publish.qos {
            QoS::AtMostOnce => {
                self.network_sender.send(Packet::Publish(publish)).await?;
            }
            QoS::AtLeastOnce => {
                self.network_sender
                    .send(Packet::Publish(publish.clone()))
                    .await?;
                if let Some(pub_collision) = self
                    .state
                    .outgoing_pub
                    .insert(publish.packet_identifier.unwrap(), publish)
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
                if let Some(pub_collision) = self
                    .state
                    .outgoing_pub
                    .insert(publish.packet_identifier.unwrap(), publish)
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

    async fn handle_outgoing_subscribe(&mut self, sub: Subscribe) -> Result<(), MqttError> {
        if self
            .state
            .outgoing_sub
            .insert(sub.packet_identifier, sub.clone())
            .is_some()
        {
            error!(
                "Encountered a colliding packet ID ({}) in a subscribe packet",
                sub.packet_identifier,
            )
        }
        else {
            self.network_sender.send(Packet::Subscribe(sub)).await?;
        }
        Ok(())
    }

    async fn handle_outgoing_unsubscribe(&mut self, unsub: Unsubscribe) -> Result<(), MqttError> {
        if self
            .state
            .outgoing_unsub
            .insert(unsub.packet_identifier, unsub.clone())
            .is_some()
        {
            error!(
                "Encountered a colliding packet ID ({}) in a unsubscribe packet",
                unsub.packet_identifier,
            )
        }
        else {
            self.network_sender.send(Packet::Unsubscribe(unsub)).await?;
        }
        Ok(())
    }

    async fn handle_outgoing_disconnect(
        &mut self,
        disconnect: Disconnect,
    ) -> Result<(), MqttError> {
        // self.atomic_disconnect.store(true, Ordering::Release);
        self.disconnect = true;
        self.network_sender
            .send(Packet::Disconnect(disconnect))
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod handler_tests {
    use std::{sync::Arc, time::Duration};

    use async_channel::{Receiver, Sender};

    use crate::{
    	connect_options::ConnectOptions,
    	event_handler::{EventHandlerTask},
    	packets::{
    		reason_codes::{
    			PubCompReasonCode, PubRecReasonCode, PubRelReasonCode, SubAckReasonCode,
    		},
    		QoS, {Packet, PacketType}, {PubComp, PubCompProperties}, {PubRec, PubRecProperties},
    		{PubRel, PubRelProperties}, {SubAck, SubAckProperties},
    	},
    	// tests::resources::test_packets::{
    	// 	create_disconnect_packet, create_puback_packet, create_publish_packet,
    	// 	create_subscribe_packet,
    	// }, 
        AsyncEventHandlerMut, tests::test_packets::{create_publish_packet, create_puback_packet, create_disconnect_packet, create_subscribe_packet},
    };

    pub struct Nop {}
    #[async_trait::async_trait]
    impl AsyncEventHandlerMut for Nop {
    	async fn handle(&mut self, _event: &Packet){
            
        }
    }

    fn handler() -> (
    	EventHandlerTask,
    	Receiver<Packet>,
    	Sender<Packet>,
    	Sender<Packet>,
    ) {
    	let opt = ConnectOptions::new("test123123".to_string());

    	let (to_network_s, to_network_r) = async_channel::bounded(100);
    	let (network_to_handler_s, network_to_handler_r) = async_channel::bounded(100);
    	let (client_to_handler_s, client_to_handler_r) = async_channel::bounded(100);

    	let (handler, _apkid) = EventHandlerTask::new(
    		&opt,
    		network_to_handler_r,
    		to_network_s,
    		client_to_handler_r,
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

    	let (mut handler, to_network_r, _network_to_handler_s, client_to_handler_s) = handler();

    	let handler_task = tokio::task::spawn(async move {
    		let _ = loop {
    			match handler.handle_mut(&mut nop).await {
    				Ok(_) => (),
    				Err(_) => break,
    			}
    		};
    		return handler;
    	});
    	let pub_packet = create_publish_packet(QoS::AtMostOnce, false, false, None);

    	client_to_handler_s.send(pub_packet.clone()).await.unwrap();

    	let packet = to_network_r.recv().await.unwrap();

    	assert_eq!(packet, pub_packet);

    	// If we drop the client to handler channel the handler will stop executing and we can inspect its internals.
    	drop(client_to_handler_s);

    	let handler = handler_task.await.unwrap();

    	assert!(handler.state.incoming_pub.is_empty());
    	assert!(handler.state.outgoing_pub.is_empty());
    	assert!(handler.state.outgoing_rel.is_empty());
    	assert!(handler.state.outgoing_sub.is_empty());
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
        #[async_trait::async_trait]
    	impl AsyncEventHandlerMut for TestPubQoS1 {
    		async fn handle(&mut self, event: &Packet) {
                match self.stage {
                    StagePubQoS1::PubAck => {
                        assert_eq!(event.packet_type(), PacketType::PubAck);
                        self.stage = StagePubQoS1::Done;
                    }
                    StagePubQoS1::Done => (),
                }
    		}
    	}

    	let mut nop = TestPubQoS1::new();

    	let (mut handler, to_network_r, network_to_handler_s, client_to_handler_s) = handler();

    	let handler_task = tokio::task::spawn(async move {
    		// Ignore the error that this will return
    		let _ = loop {
    			match handler.handle_mut(&mut nop).await {
    				Ok(_) => (),
    				Err(_) => break,
    			}
    		};
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
    	drop(network_to_handler_s);

    	let handler = handler_task.await.unwrap();

    	assert!(handler.state.incoming_pub.is_empty());
    	assert!(handler.state.outgoing_pub.is_empty());
    	assert!(handler.state.outgoing_rel.is_empty());
    	assert!(handler.state.outgoing_sub.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn incoming_publish_qos_1() {
    	let mut nop = Nop {};

    	let (mut handler, to_network_r, network_to_handler_s, client_to_handler_s) = handler();

    	let handler_task = tokio::task::spawn(async move {
    		// Ignore the error that this will return
    		let _ = loop {
    			match handler.handle_mut(&mut nop).await {
    				Ok(_) => (),
    				Err(_) => break,
    			}
    		};
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
    	drop(network_to_handler_s);

    	let handler = handler_task.await.unwrap();

    	assert!(handler.state.incoming_pub.is_empty());
    	assert!(handler.state.outgoing_pub.is_empty());
    	assert!(handler.state.outgoing_rel.is_empty());
    	assert!(handler.state.outgoing_sub.is_empty());
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
        #[async_trait::async_trait]
    	impl AsyncEventHandlerMut for TestPubQoS2 {
    		async fn handle(&mut self, event: &Packet){
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

    	let (mut handler, to_network_r, network_to_handler_s, client_to_handler_s) = handler();

    	let mut nop = TestPubQoS2::new(client_to_handler_s.clone());

    	let handler_task = tokio::task::spawn(async move {
    		let _ = loop {
    			match handler.handle_mut (&mut nop).await {
    				Ok(_) => (),
    				Err(_) => break,
    			}
    		};
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

    	drop(client_to_handler_s);
    	drop(network_to_handler_s);

    	let handler = handler_task.await.unwrap();

    	assert!(handler.state.incoming_pub.is_empty());
    	assert!(handler.state.outgoing_pub.is_empty());
    	assert!(handler.state.outgoing_rel.is_empty());
    	assert!(handler.state.outgoing_sub.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn outgoing_subscribe() {
    	let (mut handler, to_network_r, network_to_handler_s, client_to_handler_s) = handler();

    	let mut nop = Nop {};

    	let handler_task = tokio::task::spawn(async move {
    		// Ignore the error that this will return
    		let _ = loop {
    			match handler.handle_mut(&mut nop).await {
    				Ok(_) => (),
    				Err(_) => break,
    			}
    		};
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
    	drop(network_to_handler_s);

    	let handler = handler_task.await.unwrap();
    	assert!(handler.state.incoming_pub.is_empty());
    	assert!(handler.state.outgoing_pub.is_empty());
    	assert!(handler.state.outgoing_rel.is_empty());
    	assert!(handler.state.outgoing_sub.is_empty());
    }
}
