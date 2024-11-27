use crate::available_packet_ids::AvailablePacketIds;
use crate::connect_options::ConnectOptions;
use crate::error::HandlerError;
use crate::packets::PubComp;
use crate::packets::PubRec;
use crate::packets::PubRel;
use crate::packets::Publish;
use crate::packets::QoS;
use crate::packets::SubAck;
use crate::packets::Subscribe;
use crate::packets::UnsubAck;
use crate::packets::Unsubscribe;
use crate::packets::{ConnAck, Disconnect};
use crate::packets::{ConnAckReasonCode, PubAckReasonCode, PubRecReasonCode};
use crate::packets::{Packet, PacketType};
use crate::packets::{PubAck, PubAckProperties};
use crate::state::State;

#[cfg(feature = "logs")]
use tracing::{debug, error, info, warn};

/// Eventloop with all the state of a connection
pub(crate) struct StateHandler {
    state: State,
    clean_start: bool,
}

/// [`MqttHandler`] is used to properly handle incoming and outgoing packets according to the MQTT specifications.
/// Only the incoming messages are shown to the user via the user provided handler.
impl StateHandler {
    pub(crate) fn new(options: &ConnectOptions, apkids: AvailablePacketIds) -> Self {
        StateHandler {
            state: State::new(options.receive_maximum(), apkids),

            clean_start: options.clean_start,
        }
    }

    /// This function handles the incoming packet `packet` depending on the packet type.
    /// Any packets that are produced as a response to the incoming packet are added to the outgoing_packet_buffer.
    ///
    /// # Return value
    /// This function returns either an error or an indication wether the users handler needs to be called on this packet.
    /// In some cases (retransmitted Publish packets) the users handler should not be called to avoid duplicate delivery.
    /// true is returned if the users handler should be called
    /// false otherwise
    pub fn handle_incoming_packet(&self, packet: &Packet) -> Result<(Option<Packet>, bool), HandlerError> {
        let reply_packet = match packet {
            Packet::Publish(publish) => return self.handle_incoming_publish(publish),
            Packet::PubAck(puback) => self.handle_incoming_puback(puback),
            Packet::PubRec(pubrec) => self.handle_incoming_pubrec(pubrec),
            Packet::PubRel(pubrel) => self.handle_incoming_pubrel(pubrel),
            Packet::PubComp(pubcomp) => self.handle_incoming_pubcomp(pubcomp),
            Packet::SubAck(suback) => self.handle_incoming_suback(suback),
            Packet::UnsubAck(unsuback) => self.handle_incoming_unsuback(unsuback),
            a => return Err(HandlerError::UnexpectedPacket(a.packet_type())),
        }?;

        Ok((reply_packet, true))
    }

    fn handle_incoming_publish(&self, publish: &Publish) -> Result<(Option<Packet>, bool), HandlerError> {
        match publish.qos {
            QoS::AtMostOnce => Ok((None, true)),
            QoS::AtLeastOnce => {
                let puback = PubAck {
                    packet_identifier: publish.packet_identifier.ok_or(HandlerError::MissingPacketId)?,
                    reason_code: PubAckReasonCode::Success,
                    properties: PubAckProperties::default(),
                };
                Ok((Some(Packet::PubAck(puback)), true))
            }
            QoS::ExactlyOnce => {
                let pkid = publish.packet_identifier.ok_or(HandlerError::MissingPacketId)?;

                let should_client_handle = self.state.add_incoming_pub(pkid) && !publish.dup;

                #[cfg(feature = "logs")]
                if should_client_handle {
                    error!("Received publish with an packet ID ({}) that is in use and the packet was not a duplicate", pkid,);
                }
                Ok((Some(Packet::PubRec(PubRec::new(pkid))), should_client_handle))
            }
        }
    }

    fn handle_incoming_puback(&self, puback: &PubAck) -> Result<Option<Packet>, HandlerError> {
        if self.state.remove_outgoing_pub(puback.packet_identifier).is_some() {
            #[cfg(feature = "logs")]
            debug!("Publish {:?} has been acknowledged", puback.packet_identifier);
            self.state.make_pkid_available(puback.packet_identifier)?;
            Ok(None)
        } else {
            #[cfg(feature = "logs")]
            error!("Publish {:?} was not found, while receiving a PubAck for it", puback.packet_identifier,);
            Err(HandlerError::Unsolicited(puback.packet_identifier, PacketType::PubAck))
        }
    }

    fn handle_incoming_pubrec(&self, pubrec: &PubRec) -> Result<Option<Packet>, HandlerError> {
        match self.state.remove_outgoing_pub(pubrec.packet_identifier) {
            Some(_) => match pubrec.reason_code {
                PubRecReasonCode::Success | PubRecReasonCode::NoMatchingSubscribers => {
                    let pubrel = PubRel::new(pubrec.packet_identifier);

                    self.state.add_outgoing_rel(pubrec.packet_identifier);

                    #[cfg(feature = "logs")]
                    debug!("Publish {:?} has been PubReced", pubrec.packet_identifier);

                    Ok(Some(Packet::PubRel(pubrel)))
                }
                _ => Ok(None),
            },
            None => {
                #[cfg(feature = "logs")]
                error!("Publish {} was not found, while receiving a PubRec for it", pubrec.packet_identifier,);
                Err(HandlerError::Unsolicited(pubrec.packet_identifier, PacketType::PubRec))
            }
        }
    }

    fn handle_incoming_pubrel(&self, pubrel: &PubRel) -> Result<Option<Packet>, HandlerError> {
        let pubcomp = PubComp::new(pubrel.packet_identifier);
        if !self.state.remove_incoming_pub(pubrel.packet_identifier) {
            #[cfg(feature = "logs")]
            warn!("Received an unexpected / unsolicited PubRel packet with id {}", pubrel.packet_identifier);
            let _pubcomp = PubComp::new(pubrel.packet_identifier);
        }
        Ok(Some(Packet::PubComp(pubcomp)))
    }

    fn handle_incoming_pubcomp(&self, pubcomp: &PubComp) -> Result<Option<Packet>, HandlerError> {
        if self.state.remove_outgoing_rel(&pubcomp.packet_identifier) {
            self.state.make_pkid_available(pubcomp.packet_identifier)?;
            Ok(None)
        } else {
            #[cfg(feature = "logs")]
            error!("PubRel {} was not found, while receiving a PubComp for it", pubcomp.packet_identifier,);
            Err(HandlerError::Unsolicited(pubcomp.packet_identifier, PacketType::PubComp))
        }
    }

    fn handle_incoming_suback(&self, suback: &SubAck) -> Result<Option<Packet>, HandlerError> {
        if self.state.remove_outgoing_sub(suback.packet_identifier) {
            self.state.make_pkid_available(suback.packet_identifier)?;
            Ok(None)
        } else {
            #[cfg(feature = "logs")]
            error!("Sub {} was not found, while receiving a SubAck for it", suback.packet_identifier,);
            Err(HandlerError::Unsolicited(suback.packet_identifier, PacketType::SubAck))
        }
    }

    fn handle_incoming_unsuback(&self, unsuback: &UnsubAck) -> Result<Option<Packet>, HandlerError> {
        if self.state.remove_outgoing_unsub(unsuback.packet_identifier) {
            self.state.make_pkid_available(unsuback.packet_identifier)?;
            Ok(None)
        } else {
            #[cfg(feature = "logs")]
            error!("Unsub {} was not found, while receiving a unsuback for it", unsuback.packet_identifier,);
            Err(HandlerError::Unsolicited(unsuback.packet_identifier, PacketType::UnsubAck))
        }
    }

    pub fn handle_incoming_connack(&self, conn_ack: &ConnAck) -> Result<Option<Vec<Packet>>, HandlerError> {
        if conn_ack.reason_code == ConnAckReasonCode::Success {
            let retransmission = conn_ack.connack_flags.session_present && !self.clean_start;
            let (freeable_ids, republish) = self.state.reset(retransmission);

            for i in freeable_ids {
                self.state.make_pkid_available(i)?;
            }
            Ok(Some(republish))
        } else {
            Ok(None)
        }
    }

    pub fn handle_outgoing_packet(&self, packet: Packet) -> Result<(), HandlerError> {
        #[cfg(feature = "logs")]
        info!("Handling outgoing packet {}", packet);
        match packet {
            Packet::Publish(publish) => self.handle_outgoing_publish(publish),
            Packet::Subscribe(sub) => self.handle_outgoing_subscribe(sub),
            Packet::Unsubscribe(unsub) => self.handle_outgoing_unsubscribe(unsub),
            Packet::Disconnect(discon) => self.handle_outgoing_disconnect(discon),
            _a => {
                #[cfg(test)]
                unreachable!("Was given unexpected packet {:?} ", _a);
                #[cfg(not(test))]
                Ok(())
            }
        }
    }

    fn handle_outgoing_publish(&self, publish: Publish) -> Result<(), HandlerError> {
        #[cfg(feature = "logs")]
        let id = publish.packet_identifier;
        match publish.qos {
            QoS::AtMostOnce => Ok(()),
            QoS::AtLeastOnce => match self.state.add_outgoing_pub(publish.packet_identifier.unwrap(), publish) {
                Ok(_) => Ok(()),
                Err(err) => {
                    #[cfg(feature = "logs")]
                    error!("Encountered a colliding packet ID ({:?}) in a publish QoS 1 packet", id);
                    Err(err)
                }
            },
            QoS::ExactlyOnce => match self.state.add_outgoing_pub(publish.packet_identifier.unwrap(), publish) {
                Ok(_) => Ok(()),
                Err(err) => {
                    #[cfg(feature = "logs")]
                    error!("Encountered a colliding packet ID ({:?}) in a publish QoS 2 packet", id);
                    Err(err)
                }
            },
        }
    }

    fn handle_outgoing_subscribe(&self, sub: Subscribe) -> Result<(), HandlerError> {
        #[cfg(feature = "logs")]
        info!("handling outgoing subscribe with ID: {}", sub.packet_identifier);
        if !self.state.add_outgoing_sub(sub.packet_identifier) {
            #[cfg(feature = "logs")]
            error!("Encountered a colliding packet ID ({}) in a subscribe packet\n {:?}", sub.packet_identifier, sub,);
        }
        Ok(())
    }

    fn handle_outgoing_unsubscribe(&self, unsub: Unsubscribe) -> Result<(), HandlerError> {
        if !self.state.add_outgoing_unsub(unsub.packet_identifier) {
            #[cfg(feature = "logs")]
            error!("Encountered a colliding packet ID ({}) in a unsubscribe packet", unsub.packet_identifier,);
        }
        Ok(())
    }

    fn handle_outgoing_disconnect(&self, _: Disconnect) -> Result<(), HandlerError> {
        Ok(())
    }
}

#[cfg(test)]
mod handler_tests {
    use async_channel::Receiver;

    use crate::{
        available_packet_ids::AvailablePacketIds,
        packets::{
            Packet, PubComp, PubCompProperties, PubCompReasonCode, PubRec, PubRecProperties, PubRecReasonCode, PubRel, PubRelProperties, PubRelReasonCode, QoS, SubAck, SubAckProperties,
            SubAckReasonCode, UnsubAck, UnsubAckProperties, UnsubAckReasonCode,
        },
        tests::test_packets::{create_connack_packet, create_puback_packet, create_publish_packet, create_subscribe_packet, create_unsubscribe_packet},
        ConnectOptions, StateHandler,
    };
    fn handler(clean_start: bool) -> (StateHandler, Receiver<u16>) {
        let (apkids, apkids_r) = AvailablePacketIds::new(100);

        let mut opt = ConnectOptions::new("test123123");
        opt.set_receive_maximum(100).set_clean_start(clean_start);

        (StateHandler::new(&opt, apkids), apkids_r)
    }

    #[tokio::test]
    async fn outgoing_publish_qos_0() {
        let (mut handler, _apkid) = handler(false);

        let pub_packet = create_publish_packet(QoS::AtMostOnce, false, false, None);

        handler.handle_outgoing_packet(pub_packet).unwrap();

        assert!(handler.state.incoming_pub().is_empty());
        assert_eq!(handler.state.outgoing_pub_order().len(), 0);
        assert_eq!(handler.state.outgoing_pub().len(), 100);
        assert!(handler.state.outgoing_rel().is_empty());
        assert!(handler.state.outgoing_sub().is_empty());
    }

    #[tokio::test]
    async fn outgoing_publish_qos_1() {
        let (mut handler, apkid) = handler(false);

        let pkid = apkid.recv().await.unwrap();
        let pub_packet = create_publish_packet(QoS::AtLeastOnce, false, false, Some(pkid));

        handler.handle_outgoing_packet(pub_packet.clone()).unwrap();

        assert!(handler.state.incoming_pub().is_empty());
        assert_eq!(handler.state.outgoing_pub_order().len(), 1);
        assert_eq!(Packet::Publish(handler.state.outgoing_pub()[pkid as usize - 1].clone().unwrap()), pub_packet);
        assert!(handler.state.outgoing_rel().is_empty());
        assert!(handler.state.outgoing_sub().is_empty());

        let puback = create_puback_packet(pkid);
        let res = handler.handle_incoming_packet(&puback).unwrap();

        assert!(res.0.is_none());
        assert!(handler.state.incoming_pub().is_empty());
        assert!(handler.state.outgoing_pub()[pkid as usize - 1].is_none());
        assert!(handler.state.outgoing_pub_order().is_empty());
        assert!(handler.state.outgoing_rel().is_empty());
        assert!(handler.state.outgoing_sub().is_empty());
    }

    #[tokio::test]
    async fn incoming_publish_qos_1() {
        let (mut handler, _apkid) = handler(false);

        let pub_packet = create_publish_packet(QoS::AtLeastOnce, false, false, Some(1));

        let (packet, should_handle) = handler.handle_incoming_packet(&pub_packet).unwrap();

        assert!(handler.state.incoming_pub().is_empty());
        assert!(handler.state.outgoing_pub_order().is_empty());
        assert!(handler.state.outgoing_rel().is_empty());
        assert!(handler.state.outgoing_sub().is_empty());

        assert!(should_handle);
        assert!(packet.is_some());
        let expected_puback = create_puback_packet(1);
        assert_eq!(expected_puback, packet.unwrap());
    }

    #[tokio::test]
    async fn outgoing_publish_qos_2() {
        let (mut handler, apkid) = handler(false);

        let pkid = apkid.recv().await.unwrap();
        let pub_packet = create_publish_packet(QoS::ExactlyOnce, false, false, Some(pkid));

        handler.handle_outgoing_packet(pub_packet.clone()).unwrap();

        assert!(handler.state.incoming_pub().is_empty());
        assert_eq!(pub_packet, Packet::Publish(handler.state.outgoing_pub()[pkid as usize - 1].clone().unwrap()));
        assert_eq!(1, handler.state.outgoing_pub_order().len());
        assert!(handler.state.outgoing_rel().is_empty());
        assert!(handler.state.outgoing_sub().is_empty());

        let pubrec = Packet::PubRec(PubRec {
            packet_identifier: pkid,
            reason_code: PubRecReasonCode::Success,
            properties: PubRecProperties::default(),
        });

        let (packet1, _should_handle) = handler.handle_incoming_packet(&pubrec).unwrap();

        assert!(handler.state.incoming_pub().is_empty());
        assert!(handler.state.outgoing_pub()[pkid as usize - 1].clone().is_none());
        assert_eq!(0, handler.state.outgoing_pub_order().len());
        assert_eq!(1, handler.state.outgoing_rel().len());
        assert_eq!(1, *handler.state.outgoing_rel().first().unwrap());
        assert!(handler.state.outgoing_sub().is_empty());

        let expected_pubrel = Packet::PubRel(PubRel {
            packet_identifier: pkid,
            reason_code: PubRelReasonCode::Success,
            properties: PubRelProperties::default(),
        });
        assert_eq!(expected_pubrel, packet1.unwrap());

        let pubcomp = Packet::PubComp(PubComp {
            packet_identifier: pkid,
            reason_code: PubCompReasonCode::Success,
            properties: PubCompProperties::default(),
        });

        let (packet3, should_handle) = handler.handle_incoming_packet(&pubcomp).unwrap();

        assert!(packet3.is_none());
        assert!(should_handle);
        assert!(handler.state.incoming_pub().is_empty());
        assert!(handler.state.outgoing_pub()[pkid as usize - 1].clone().is_none());
        assert!(handler.state.outgoing_pub_order().is_empty());
        assert!(handler.state.outgoing_rel().is_empty());
        assert!(handler.state.outgoing_sub().is_empty());
    }

    #[tokio::test]
    async fn incoming_publish_qos_2() {
        let (mut handler, apkid) = handler(false);

        let pkid = apkid.recv().await.unwrap();
        let pub_packet = create_publish_packet(QoS::ExactlyOnce, false, false, Some(pkid));

        let (reply_packet, _should_handle) = handler.handle_incoming_packet(&pub_packet).unwrap();

        assert_eq!(1, handler.state.incoming_pub().len());
        assert!(handler.state.outgoing_pub_order().is_empty());
        assert!(handler.state.outgoing_rel().is_empty());
        assert!(handler.state.outgoing_sub().is_empty());
        assert!(handler.state.outgoing_unsub().is_empty());

        let pubrec = Packet::PubRec(PubRec {
            packet_identifier: pkid,
            reason_code: PubRecReasonCode::Success,
            properties: PubRecProperties::default(),
        });

        assert_eq!(pubrec, reply_packet.unwrap());
        assert_eq!(1, handler.state.incoming_pub().len());
        assert_eq!(0, handler.state.outgoing_pub_order().len());
        assert_eq!(0, handler.state.outgoing_rel().len());
        assert!(handler.state.outgoing_sub().is_empty());
        assert!(handler.state.outgoing_unsub().is_empty());

        let pubrel = Packet::PubRel(PubRel {
            packet_identifier: pkid,
            reason_code: PubRelReasonCode::Success,
            properties: PubRelProperties::default(),
        });

        let (reply_packet, _should_handle) = handler.handle_incoming_packet(&pubrel).unwrap();

        let pubcomp = Packet::PubComp(PubComp {
            packet_identifier: pkid,
            reason_code: PubCompReasonCode::Success,
            properties: PubCompProperties::default(),
        });

        assert_eq!(pubcomp, reply_packet.unwrap());
        assert!(handler.state.incoming_pub().is_empty());
        assert!(handler.state.outgoing_pub_order().is_empty());
        assert!(handler.state.outgoing_rel().is_empty());
        assert!(handler.state.outgoing_sub().is_empty());
    }

    #[tokio::test]
    async fn outgoing_subscribe() {
        let (mut handler, apkid) = handler(false);

        let pkid = apkid.recv().await.unwrap();

        let sub_packet = create_subscribe_packet(pkid);

        handler.handle_outgoing_packet(sub_packet).unwrap();

        assert!(handler.state.incoming_pub().is_empty());
        assert!(handler.state.outgoing_pub_order().is_empty());
        assert!(handler.state.outgoing_rel().is_empty());
        assert_eq!(1, handler.state.outgoing_sub().len());
        assert!(handler.state.outgoing_unsub().is_empty());

        let suback = Packet::SubAck(SubAck {
            packet_identifier: 1,
            reason_codes: vec![SubAckReasonCode::GrantedQoS0],
            properties: SubAckProperties::default(),
        });

        let res = handler.handle_incoming_packet(&suback).unwrap();

        assert!(res.0.is_none());
        assert!(handler.state.incoming_pub().is_empty());
        assert!(handler.state.outgoing_pub_order().is_empty());
        assert!(handler.state.outgoing_rel().is_empty());
        assert!(handler.state.outgoing_sub().is_empty());
        assert!(handler.state.outgoing_unsub().is_empty());
    }

    #[tokio::test]
    async fn outgoing_unsubscribe() {
        let (mut handler, apkid) = handler(false);

        let pkid = apkid.recv().await.unwrap();

        let unsub_packet = create_unsubscribe_packet(pkid);

        handler.handle_outgoing_packet(unsub_packet).unwrap();

        assert!(handler.state.incoming_pub().is_empty());
        assert!(handler.state.outgoing_pub_order().is_empty());
        assert!(handler.state.outgoing_rel().is_empty());
        assert!(handler.state.outgoing_sub().is_empty());
        assert_eq!(1, handler.state.outgoing_unsub().len());

        let unsuback = Packet::UnsubAck(UnsubAck {
            packet_identifier: 1,
            reason_codes: vec![UnsubAckReasonCode::Success],
            properties: UnsubAckProperties::default(),
        });

        let (packet, _should_handle) = handler.handle_incoming_packet(&unsuback).unwrap();

        assert!(packet.is_none());
        assert!(handler.state.incoming_pub().is_empty());
        assert!(handler.state.outgoing_pub_order().is_empty());
        assert!(handler.state.outgoing_rel().is_empty());
        assert!(handler.state.outgoing_sub().is_empty());
        assert!(handler.state.outgoing_unsub().is_empty());
    }

    #[tokio::test]
    async fn retransmit_test_1() {
        let (handler, apkid) = handler(false);
        let mut stored_published_packets = Vec::new();
        // let resp_vec = Vec::new();

        let pkid = apkid.recv().await.unwrap();
        let pub1 = create_publish_packet(QoS::AtLeastOnce, false, false, Some(pkid));
        stored_published_packets.push(pub1.clone());
        handler.handle_outgoing_packet(pub1).unwrap();

        let pkid = apkid.recv().await.unwrap();
        let pub1 = create_publish_packet(QoS::ExactlyOnce, false, false, Some(pkid));
        stored_published_packets.push(pub1.clone());
        handler.handle_outgoing_packet(pub1).unwrap();

        let pub1 = create_publish_packet(QoS::AtMostOnce, false, false, None);
        handler.handle_outgoing_packet(pub1).unwrap();

        let pkid = apkid.recv().await.unwrap();
        let sub1 = create_subscribe_packet(pkid);
        let pkid = apkid.recv().await.unwrap();
        let unsub1 = create_unsubscribe_packet(pkid);

        handler.handle_outgoing_packet(sub1).unwrap();
        handler.handle_outgoing_packet(unsub1).unwrap();

        assert_eq!(100 - 4, apkid.len());

        let Packet::ConnAck(conn_ack) = create_connack_packet(true) else {
            panic!("Should return ConnAck packet")
        };
        let retransmit_vec = handler.handle_incoming_connack(&conn_ack).unwrap().unwrap();

        assert_eq!(100 - 2, apkid.len());

        assert_eq!(stored_published_packets.len(), retransmit_vec.len());
        for i in 0..stored_published_packets.len() {
            let expected_pub = stored_published_packets.get(i).unwrap();
            let res_pub = retransmit_vec.get(i).unwrap();
            match (expected_pub, res_pub) {
                (Packet::Publish(expected), Packet::Publish(res)) => {
                    assert!(res.dup);
                    assert_eq!(expected.qos, res.qos);
                    assert_eq!(expected.retain, res.retain);
                    assert_eq!(expected.topic, res.topic);
                    assert_eq!(expected.packet_identifier, res.packet_identifier);
                    assert_eq!(expected.publish_properties, res.publish_properties);
                    assert_eq!(expected.payload, res.payload);
                }
                (_, _) => panic!("Should only contain publish packets"),
            }
        }
    }
}
