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

use futures_concurrency::future::{Race};

// use crate::{Incoming, MqttOptions, MqttState, Outgoing, Packet, Request, StateError, Transport};

use async_channel::{Receiver, Sender};
use async_mutex::Mutex;
// use futures::{select, FutureExt};
use tracing::{debug, error};



use std::future::Future;
#[cfg(unix)]
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

/// Eventloop with all the state of a connection
pub struct EventHandlerTask{

    /// Options of the current mqtt connection
    // options: ConnectOptions,
    /// Current state of the connection
    state: State,

    network_receiver: Receiver<Packet>,

    network_sender: Sender<Packet>,

    client_to_handler_r: Receiver<Packet>,

    last_network_action: Arc<Mutex<Instant>>,
}

impl EventHandlerTask {

    /// New MQTT `EventLoop`
    ///
    /// When connection encounters critical errors (like auth failure), user has a choice to
    /// access and update `options`, `state` and `requests`.
    pub(crate) fn new(
        receive_maximum: u16,
        network_receiver: Receiver<Packet>,
        network_sender: Sender<Packet>,
        client_to_handler_r: Receiver<Packet>,
        last_network_action: Arc<Mutex<Instant>>,
    ) -> (Self, Receiver<u16>) {

        let (state, packet_id_channel) = State::new(receive_maximum);

        let task = EventHandlerTask {
            state,

            network_receiver,
            network_sender,

            client_to_handler_r,

            last_network_action,
        };
        (task, packet_id_channel)
    }

    pub async fn handle<H: EventHandler + Send + Sized + 'static> (&mut self, handler: &mut H) -> Result<(), MqttError>{       

        let a = async {
            loop{
                debug!("Event Handler, network receiver len {:?}", self.network_receiver.len());
                let network_event = self.network_receiver.recv().await;
                match network_event {
                    Ok(event) => {
                        debug!("Event Handler, handling incoming packet: {:?}", event);
                        self.handle_incoming_packet(handler, event).await?
                    },
                    Err(err) => Err(MqttError::IncomingNetworkChannelClosed(err))?,
                }
            }
        };

        let b = async{
            loop{
                debug!("Event Handler, client receiver len {:?}", self.client_to_handler_r.len());
                let client_event = self.client_to_handler_r.recv().await;
                let ret = match client_event {
                    Ok(event) => {
                        debug!("Event Handler, handling outgoing packet: {:?}", event);
                        self.handle_outgoing_packet(event).await
                    },
                    Err(err) => Err(MqttError::ClientChannelClosed(err)),
                };
                if ret.is_err(){
                    return ret;
                }
            }
        };

        // We do not use an select! because that has a high possibility of data loss.
        // Instead, we use a endless loop of which both are polled.
        // Nevertheless, they are raced because they only return if there is a fatel error
        let r = (a,b).race().await;
        error!("Ending event_handler.handle()");
        r
    }

    async fn handle_incoming_packet<H: EventHandler + Send + Sized + 'static>(&self, handler: &mut H, packet: Packet) -> Result<(), MqttError>{
        handler.handle(&packet).await;

        match packet {
            Packet::Publish(publish) => self.handle_incoming_publish(&publish).await?,
            Packet::PubAck(puback) => self.handle_incoming_puback(&puback).await?,
            Packet::PubRec(pubrec) => self.handle_incoming_pubrec(&pubrec).await?,
            Packet::PubRel(pubrel) => self.handle_incoming_pubrel(&pubrel).await?,
            Packet::PubComp(pubcomp) => self.handle_incoming_pubcomp(&pubcomp).await?,
            Packet::PingResp => todo!(),
            _ => (),
        };
        Ok(())
    }

    async fn handle_incoming_publish(&self, publish: &Publish) -> Result<(), MqttError> {
        match publish.qos {
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
                let mut incoming_pub = self.state.incoming_pub.lock().await;
                if !incoming_pub.insert(pkid) && !publish.dup{
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
        if let Some(p) = outgoing_pub.remove(&puback.packet_identifier){
            debug!(
                "Publish {:?} with QoS {} has been acknowledged",
                p.packet_identifier,
                p.qos.into_u8(),
            );
            self.state.apkid.mark_available(puback.packet_identifier).await;
            Ok(())
        }
        else{
            error!(
                "Publish {:?} was not found, while receiving a PubAck for it",
                puback.packet_identifier,
            );
            Err(MqttError::Unsolicited(puback.packet_identifier, PacketType::PubAck))
        }
    }

    async fn handle_incoming_pubrec(&self, pubrec: &PubRec) -> Result<(), MqttError> {
        let mut outgoing_pub = self.state.outgoing_pub.lock().await;
        match outgoing_pub.remove(&pubrec.packet_identifier){
            Some(p) => {
                match pubrec.reason_code {
                    PubRecReasonCode::Success | PubRecReasonCode::NoMatchingSubscribers => {
                        let pubrel = PubRel::new(pubrec.packet_identifier);
                        {
                            let mut outgoing_rel = self.state.outgoing_rel.lock().await;
                            outgoing_rel.insert(pubrec.packet_identifier);
                        }
                        self.network_sender.send(Packet::PubRel(pubrel)).await?;

                        debug!(
                            "Publish {:?} with QoS {} has been PubReced",
                            p.packet_identifier,
                            p.qos.into_u8(),
                        );
                        Ok(())
                    },
                    _ => Ok(()),
                }
            },
            None => {
                error!(
                    "Publish {} was not found, while receiving a PubRec for it",
                    pubrec.packet_identifier,
                );
                Err(MqttError::Unsolicited(pubrec.packet_identifier, PacketType::PubRec))
            },
        }
    }

    async fn handle_incoming_pubrel(&self, pubrel: &PubRel) -> Result<(), MqttError> {
        let pubcomp = PubComp::new(pubrel.packet_identifier);
        self.network_sender.send(Packet::PubComp(pubcomp)).await?;
        Ok(())
    }

    async fn handle_incoming_pubcomp(&self, pubcomp: &PubComp) -> Result<(), MqttError>{
        let mut outgoing_rel = self.state.outgoing_rel.lock().await;
        if outgoing_rel.remove(&pubcomp.packet_identifier){
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

    async fn handle_outgoing_packet(&self, packet: Packet) -> Result<(), MqttError>{
        match packet {
            Packet::Publish(publish) => self.handle_outgoing_publish(publish).await,
            Packet::Subscribe(sub) => self.handle_outgoing_subscribe(sub).await,
            Packet::Unsubscribe(unsub) => self.handle_outgoing_unsubscribe(unsub).await,
            Packet::Disconnect(p) => todo!(),
            Packet::Auth(_) => todo!(),

            _ => unreachable!()
        }
    }
    
    async fn handle_outgoing_publish(&self, publish: Publish) -> Result<(), MqttError>{
        match publish.qos{
            QoS::AtMostOnce => {
                self.network_sender.send(Packet::Publish(publish)).await?;
            },
            QoS::AtLeastOnce => {
                self.network_sender.send(Packet::Publish(publish.clone())).await?;
                let mut outgoing_pub = self.state.outgoing_pub.lock().await;
                if let Some(pub_collision) = outgoing_pub.insert(publish.packet_identifier.unwrap(), publish){
                    error!(
                        "Encountered a colliding packet ID ({:?}) in a publish QoS 1 packet",
                        pub_collision.packet_identifier,
                    )
                }
            },
            QoS::ExactlyOnce => {
                self.network_sender.send(Packet::Publish(publish.clone())).await?;
                let mut outgoing_pub = self.state.outgoing_pub.lock().await;
                if let Some(pub_collision) = outgoing_pub.insert(publish.packet_identifier.unwrap(), publish){
                    error!(
                        "Encountered a colliding packet ID ({:?}) in a publish QoS 2 packet",
                        pub_collision.packet_identifier,
                    )
                }
            },
        }
        Ok(())
    }
    
    async fn handle_outgoing_subscribe(&self, sub: Subscribe) -> Result<(), MqttError>{
        let mut outgoing_sub = self.state.outgoing_sub.lock().await;
        if let Some(_) = outgoing_sub.insert(sub.packet_identifier, sub.clone()){
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

    async fn handle_outgoing_unsubscribe(&self, unsub: Unsubscribe) -> Result<(), MqttError>{
        let mut outgoing_unsub = self.state.outgoing_unsub.lock().await;
        if let Some(_) = outgoing_unsub.insert(unsub.packet_identifier, unsub.clone()){
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
}