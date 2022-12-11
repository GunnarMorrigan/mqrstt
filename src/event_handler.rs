use crate::error::MqttError;
use crate::packets::subscribe::Subscribe;
use crate::packets::unsubscribe::Unsubscribe;
use crate::packets::QoS;
use crate::packets::puback::{PubAck, PubAckProperties};
use crate::packets::pubcomp::PubComp;
use crate::packets::publish::Publish;
use crate::packets::pubrec::PubRec;
use crate::packets::pubrel::PubRel;
use crate::packets::reason_codes::{PubAckReasonCode, PubRecReasonCode};
use crate::packets::packets::{Packet, PacketType};
use crate::state::State;

// use crate::{Incoming, MqttOptions, MqttState, Outgoing, Packet, Request, StateError, Transport};

use async_channel::{Receiver, Sender};
use async_mutex::Mutex;
use futures::{select, FutureExt};
use tracing::{debug, error, trace};



use std::future::Future;
#[cfg(unix)]
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

/// Eventloop with all the state of a connection
pub struct EventHandlerTask<H>{

    /// Options of the current mqtt connection
    // options: ConnectOptions,
    /// Current state of the connection
    state: State,

    handler: H,

    network_receiver: Receiver<Packet>,

    network_sender: Sender<Packet>,

    client_receiver: Receiver<Packet>,

    last_network_action: Arc<Mutex<Instant>>,
}

impl<H> EventHandlerTask<H>
    where 
        H: EventHandler + Send + Sized + 'static{

    /// New MQTT `EventLoop`
    ///
    /// When connection encounters critical errors (like auth failure), user has a choice to
    /// access and update `options`, `state` and `requests`.
    pub(crate) fn new(
        receive_maximum: u16,
        network_receiver: Receiver<Packet>,
        network_sender: Sender<Packet>,
        handler: H,
        last_network_action: Arc<Mutex<Instant>>,
    ) -> (Self, Sender<Packet>, Receiver<u16>) {

        let (client_sender, client_receiver) = async_channel::bounded(receive_maximum as usize);

        let (state, packet_id_channel) = State::new(receive_maximum);

        let task = EventHandlerTask {
            state,
            
            // keep_alive: KeepAlive::new_empty(),
            
            handler,

            network_receiver,
            network_sender,

            client_receiver,

            last_network_action,
        };
        (task, client_sender, packet_id_channel)
    }

    pub async fn handle(&mut self) -> Result<(), MqttError>{
        tokio::select! {
            network_event = self.network_receiver.recv() => {
                match network_event {
                    Ok(event) => {
                        self.handle_incoming_packet(event).await
                    },
                    Err(err) => Err(MqttError::IncomingNetworkChannelClosed(err)),
                }
            },
            client_event = self.client_receiver.recv() => {
                match client_event {
                    Ok(event) => {
                        self.handle_outgoing_packet(event).await
                    },
                    Err(err) => Err(MqttError::ClientChannelClosed(err)),
                }
            },
        }
    }

    async fn handle_incoming_packet(&mut self, packet: Packet) -> Result<(), MqttError>{
        self.handler.handle(&packet).await;

        match packet {
            Packet::Publish(publish) => self.handle_incoming_publish(&publish).await?,
            Packet::PubAck(puback) => self.handle_incoming_puback(&puback).await?,
            Packet::PubRec(pubrec) => self.handle_incoming_pubrec(&pubrec).await?,
            Packet::PubRel(pubrel) => self.handle_incoming_pubrel(&pubrel).await?,
            Packet::PubComp(_) => todo!(),
            Packet::PingResp => todo!(),

            _ => (),
        };
        Ok(())
    }

    async fn handle_incoming_publish(&mut self, publish: &Publish) -> Result<(), MqttError> {
        let qos = publish.qos;

        match qos {
            QoS::AtMostOnce => Ok(()),
            QoS::AtLeastOnce => {
                let puback = PubAck{
                    packet_identifier: publish.packet_identifier.ok_or(MqttError::MissingPacketId)?,
                    reason_code: PubAckReasonCode::Success,
                    properties: PubAckProperties::default()
                };
                self.network_sender.send(Packet::PubAck(puback)).await?;
                Ok(())
            }
            QoS::ExactlyOnce => {
                let pkid = publish.packet_identifier.ok_or(MqttError::MissingPacketId)?;
                if !self.state.incoming_pub.insert(pkid) && !publish.dup{
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
        if let Some(p) = self.state.outgoing_pub.remove(&puback.packet_identifier){
            debug!(
                "Publish {:?} with QoS {} has been acknowledged",
                p.packet_identifier,
                p.qos.into_u8(),
            );
        }
        else{
            error!(
                "Publish {:?} was not found, while receiving a PubAck for it",
                puback.packet_identifier,
            );
        }

        self.state.apkid.mark_available(puback.packet_identifier).await;
        Ok(())
    }

    async fn handle_incoming_pubrec(&mut self, pubrec: &PubRec) -> Result<(), MqttError> {
        match self.state.outgoing_pub.remove(&pubrec.packet_identifier){
            Some(p) => {
                debug!(
                    "Publish {:?} with QoS {} has been PubReced",
                    p.packet_identifier,
                    p.qos.into_u8(),
                );

                match pubrec.reason_code {
                    PubRecReasonCode::Success | PubRecReasonCode::NoMatchingSubscribers => {
                        let pubrel = PubRel::new(pubrec.packet_identifier);
                        self.state.outgoing_rel.insert(pubrec.packet_identifier);
                        self.network_sender.send(Packet::PubRel(pubrel)).await?;
                    },
                    _ => (),
                }
            },
            None => {
                error!(
                    "Publish {} was not found, while receiving a PubRec for it",
                    pubrec.packet_identifier,
                );
            },
        }

        
        Ok(())
    }

    async fn handle_incoming_pubrel(&mut self, pubrel: &PubRel) -> Result<(), MqttError> {
        let pubrel = PubComp::new(pubrel.packet_identifier);
        self.network_sender.send(Packet::PubComp(pubrel)).await?;
        Ok(())
    }

    async fn handle_incoming_pubcomp(&mut self, pubcomp: PubComp) -> Result<(), MqttError>{
        if self.state.outgoing_rel.remove(&pubcomp.packet_identifier){
            self.state.apkid.mark_available(pubcomp.packet_identifier).await;
            Ok(())
        }
        else{
            error!(
                "PubRel {} was not found, while receiving a PubComp for it",
                pubcomp.packet_identifier,
            );
            Err(MqttError::Unsolicited(pubcomp.packet_identifier, PacketType::PubComp))
        }
    }

    async fn handle_outgoing_packet(&mut self, packet: Packet) -> Result<(), MqttError>{
        match packet {
            Packet::Publish(publish) => self.handle_outgoing_publish(publish).await,
            Packet::Subscribe(sub) => self.handle_outgoing_subscribe(sub).await,
            Packet::Unsubscribe(unsub) => self.handle_outgoing_unsubscribe(unsub).await,
            Packet::Disconnect(p) => todo!(),
            Packet::Auth(_) => todo!(),

            _ => unreachable!()
        }
    }
    
    async fn handle_outgoing_publish(&mut self, publish: Publish) -> Result<(), MqttError>{
        match publish.qos{
            QoS::AtMostOnce => {
                self.network_sender.send(Packet::Publish(publish)).await?;
            },
            QoS::AtLeastOnce => {
                self.network_sender.send(Packet::Publish(publish.clone())).await?;
                if let Some(pub_collision) = self.state.outgoing_pub.insert(publish.packet_identifier.unwrap(), publish){
                    error!(
                        "Encountered a colliding packet ID ({:?}) in a publish QoS 1 packet",
                        pub_collision.packet_identifier,
                    )
                }
            },
            QoS::ExactlyOnce => {
                self.network_sender.send(Packet::Publish(publish.clone())).await?;
                if let Some(pub_collision) = self.state.outgoing_pub.insert(publish.packet_identifier.unwrap(), publish){
                    error!(
                        "Encountered a colliding packet ID ({:?}) in a publish QoS 2 packet",
                        pub_collision.packet_identifier,
                    )
                }
            },
        }
        Ok(())
    }
    
    async fn handle_outgoing_subscribe(&mut self, sub: Subscribe) -> Result<(), MqttError>{
        if let Some(_) = self.state.outgoing_sub.insert(sub.packet_identifier, sub.clone()){
            error!(
                "Encountered a colliding packet ID ({}) in a subscribe packet",
                sub.packet_identifier,
            )
        }
        else{
            self.network_sender.send(Packet::Subscribe(sub)).await?;
        }
        Ok(())
    }

    async fn handle_outgoing_unsubscribe(&mut self, unsub: Unsubscribe) -> Result<(), MqttError>{
        if let Some(_) = self.state.outgoing_unsub.insert(unsub.packet_identifier, unsub.clone()){
            error!(
                "Encountered a colliding packet ID ({}) in a unsubscribe packet",
                unsub.packet_identifier,
            )
        }
        else{
            self.network_sender.send(Packet::Unsubscribe(unsub)).await?;
        }
        Ok(())
    }
}

pub trait EventHandler: Sized + Sync + 'static{
    fn handle<'a> (&mut self, event: &'a Packet) -> impl Future<Output = ()> + Send + 'a;
    // fn handle<'a, 'b> (&mut self, event: &'a Packet) -> impl Future<Output = ()> + Send + 'b where 'b : 'a, 'a: 'b;
}