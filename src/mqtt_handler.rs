use crate::connect_options::ConnectOptions;
use crate::error::HandlerError;
use crate::packets::reason_codes::{ConnAckReasonCode, PubAckReasonCode, PubRecReasonCode};
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
use crate::packets::{Packet, PacketType};
use crate::packets::{PubAck, PubAckProperties};
use crate::state::State;

use async_channel::Receiver;
#[cfg(feature = "logs")]
use tracing::{debug, error, info};

/// Eventloop with all the state of a connection
pub struct MqttHandler {
    state: State,
    clean_start: bool,
}

/// [`MqttHandler`] is used to properly handle incoming and outgoing packets according to the MQTT specifications.
/// Only the incoming messages are shown to the user via the user provided handler.
impl MqttHandler {
    pub(crate) fn new(options: &ConnectOptions) -> (Self, Receiver<u16>) {
        let (state, packet_id_channel) = State::new(options.receive_maximum());

        let handler = MqttHandler {
            state,

            clean_start: options.clean_start,
        };
        (handler, packet_id_channel)
    }

    /// This function handles the incoming packet `packet` depending on the packet type.
    /// Any packets that are produced as a response to the incoming packet are added to the outgoing_packet_buffer.
    ///
    /// # Return value
    /// This function returns either an error or an indication wether the users handler needs to be called on this packet.
    /// In some cases (retransmitted Publish packets) the users handler should not be called to avoid duplicate delivery.
    /// true is returned if the users handler should be called
    /// false otherwise
    pub fn handle_incoming_packet(&mut self, packet: &Packet, outgoing_packet_buffer: &mut Vec<Packet>) -> Result<bool, HandlerError> {
        match packet {
            Packet::Publish(publish) => return self.handle_incoming_publish(publish, outgoing_packet_buffer),
            Packet::PubAck(puback) => self.handle_incoming_puback(puback, outgoing_packet_buffer)?,
            Packet::PubRec(pubrec) => self.handle_incoming_pubrec(pubrec, outgoing_packet_buffer)?,
            Packet::PubRel(pubrel) => self.handle_incoming_pubrel(pubrel, outgoing_packet_buffer)?,
            Packet::PubComp(pubcomp) => self.handle_incoming_pubcomp(pubcomp, outgoing_packet_buffer)?,
            Packet::SubAck(suback) => self.handle_incoming_suback(suback, outgoing_packet_buffer)?,
            Packet::UnsubAck(unsuback) => self.handle_incoming_unsuback(unsuback, outgoing_packet_buffer)?,
            Packet::ConnAck(connack) => self.handle_incoming_connack(connack, outgoing_packet_buffer)?,
            a => unreachable!("Should not receive {}", a),
        };
        Ok(false)
    }

    fn handle_incoming_publish(&mut self, publish: &Publish, outgoing_packet_buffer: &mut Vec<Packet>) -> Result<bool, HandlerError> {
        match publish.qos {
            QoS::AtMostOnce => Ok(true),
            QoS::AtLeastOnce => {
                let puback = PubAck {
                    packet_identifier: publish.packet_identifier.ok_or(HandlerError::MissingPacketId)?,
                    reason_code: PubAckReasonCode::Success,
                    properties: PubAckProperties::default(),
                };
                outgoing_packet_buffer.push(Packet::PubAck(puback));
                Ok(true)
            }
            QoS::ExactlyOnce => {
                let mut should_client_handle = true;
                let pkid = publish.packet_identifier.ok_or(HandlerError::MissingPacketId)?;

                if !self.state.add_incoming_pub(pkid) && !publish.dup {
                    #[cfg(feature = "logs")]
                    error!("Received publish with an packet ID ({}) that is in use and the packet was not a duplicate", pkid,);
                    should_client_handle = false;
                }
                outgoing_packet_buffer.push(Packet::PubRec(PubRec::new(pkid)));
                Ok(should_client_handle)
            }
        }
    }

    fn handle_incoming_puback(&mut self, puback: &PubAck, _: &mut Vec<Packet>) -> Result<(), HandlerError> {
        if self.state.remove_outgoing_pub(puback.packet_identifier).is_some() {
            #[cfg(feature = "logs")]
            debug!("Publish {:?} has been acknowledged", puback.packet_identifier);
            self.state.make_pkid_available(puback.packet_identifier)?;
            Ok(())
        } else {
            #[cfg(feature = "logs")]
            error!("Publish {:?} was not found, while receiving a PubAck for it", puback.packet_identifier,);
            Err(HandlerError::Unsolicited(puback.packet_identifier, PacketType::PubAck))
        }
    }

    fn handle_incoming_pubrec(&mut self, pubrec: &PubRec, outgoing_packet_buffer: &mut Vec<Packet>) -> Result<(), HandlerError> {
        match self.state.remove_outgoing_pub(pubrec.packet_identifier) {
            Some(_) => match pubrec.reason_code {
                PubRecReasonCode::Success | PubRecReasonCode::NoMatchingSubscribers => {
                    let pubrel = PubRel::new(pubrec.packet_identifier);

                    self.state.add_outgoing_rel(pubrec.packet_identifier);

                    #[cfg(feature = "logs")]
                    debug!("Publish {:?} has been PubReced", pubrec.packet_identifier);

                    outgoing_packet_buffer.push(Packet::PubRel(pubrel));
                    Ok(())
                }
                _ => Ok(()),
            },
            None => {
                #[cfg(feature = "logs")]
                error!("Publish {} was not found, while receiving a PubRec for it", pubrec.packet_identifier,);
                Err(HandlerError::Unsolicited(pubrec.packet_identifier, PacketType::PubRec))
            }
        }
    }

    fn handle_incoming_pubrel(&mut self, pubrel: &PubRel, outgoing_packet_buffer: &mut Vec<Packet>) -> Result<(), HandlerError> {
        if self.state.remove_incoming_pub(pubrel.packet_identifier) {
            let pubcomp = PubComp::new(pubrel.packet_identifier);
            outgoing_packet_buffer.push(Packet::PubComp(pubcomp));
            Ok(())
        } else {
            Err(HandlerError::Unsolicited(pubrel.packet_identifier, PacketType::PubRel))
        }
    }

    fn handle_incoming_pubcomp(&mut self, pubcomp: &PubComp, _: &mut Vec<Packet>) -> Result<(), HandlerError> {
        if self.state.remove_outgoing_rel(&pubcomp.packet_identifier) {
            self.state.make_pkid_available(pubcomp.packet_identifier)?;
            Ok(())
        } else {
            #[cfg(feature = "logs")]
            error!("PubRel {} was not found, while receiving a PubComp for it", pubcomp.packet_identifier,);
            Err(HandlerError::Unsolicited(pubcomp.packet_identifier, PacketType::PubComp))
        }
    }

    fn handle_incoming_suback(&mut self, suback: &SubAck, _: &mut Vec<Packet>) -> Result<(), HandlerError> {
        if self.state.remove_outgoing_sub(suback.packet_identifier) {
            self.state.make_pkid_available(suback.packet_identifier)?;
            Ok(())
        } else {
            #[cfg(feature = "logs")]
            error!("Sub {} was not found, while receiving a SubAck for it", suback.packet_identifier,);
            Err(HandlerError::Unsolicited(suback.packet_identifier, PacketType::SubAck))
        }
    }

    fn handle_incoming_unsuback(&mut self, unsuback: &UnsubAck, _: &mut Vec<Packet>) -> Result<(), HandlerError> {
        if self.state.remove_outgoing_unsub(unsuback.packet_identifier) {
            self.state.make_pkid_available(unsuback.packet_identifier)?;
            Ok(())
        } else {
            #[cfg(feature = "logs")]
            error!("Unsub {} was not found, while receiving a unsuback for it", unsuback.packet_identifier,);
            Err(HandlerError::Unsolicited(unsuback.packet_identifier, PacketType::UnsubAck))
        }
    }

    fn handle_incoming_connack(&mut self, packet: &ConnAck, outgoing_packet_buffer: &mut Vec<Packet>) -> Result<(), HandlerError> {
        if packet.reason_code == ConnAckReasonCode::Success {
            let retransmission = packet.connack_flags.session_present && !self.clean_start;
            let (freeable_ids, mut republish) = self.state.reset(retransmission);

            for i in freeable_ids {
                self.state.make_pkid_available(i)?;
            }
            outgoing_packet_buffer.append(&mut republish);
        }
        Ok(())
    }

    pub fn handle_outgoing_packet(&mut self, packet: Packet) -> Result<(), HandlerError> {
        #[cfg(feature = "logs")]
        info!("Handling outgoing packet {}", packet);
        match packet {
            Packet::Publish(publish) => self.handle_outgoing_publish(publish),
            Packet::Subscribe(sub) => self.handle_outgoing_subscribe(sub),
            Packet::Unsubscribe(unsub) => self.handle_outgoing_unsubscribe(unsub),
            Packet::Disconnect(discon) => self.handle_outgoing_disconnect(discon),
            _ => unreachable!(),
        }
    }

    fn handle_outgoing_publish(&mut self, publish: Publish) -> Result<(), HandlerError> {
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

    fn handle_outgoing_subscribe(&mut self, sub: Subscribe) -> Result<(), HandlerError> {
        #[cfg(feature = "logs")]
        info!("handling outgoing subscribe with ID: {}", sub.packet_identifier);
        if !self.state.add_outgoing_sub(sub.packet_identifier) {
            #[cfg(feature = "logs")]
            error!("Encountered a colliding packet ID ({}) in a subscribe packet\n {:?}", sub.packet_identifier, sub,);
        }
        Ok(())
    }

    fn handle_outgoing_unsubscribe(&mut self, unsub: Unsubscribe) -> Result<(), HandlerError> {
        if !self.state.add_outgoing_unsub(unsub.packet_identifier) {
            #[cfg(feature = "logs")]
            error!("Encountered a colliding packet ID ({}) in a unsubscribe packet", unsub.packet_identifier,);
        }
        Ok(())
    }

    fn handle_outgoing_disconnect(&mut self, _: Disconnect) -> Result<(), HandlerError> {
        Ok(())
    }
}

#[cfg(test)]
mod handler_tests {
    use async_channel::Receiver;

    use crate::{
        packets::{
            reason_codes::{PubCompReasonCode, PubRecReasonCode, PubRelReasonCode, SubAckReasonCode, UnsubAckReasonCode},
            Packet, QoS, UnsubAck, UnsubAckProperties, {PubComp, PubCompProperties}, {PubRec, PubRecProperties}, {PubRel, PubRelProperties}, {SubAck, SubAckProperties},
        },
        tests::test_packets::{create_connack_packet, create_puback_packet, create_publish_packet, create_subscribe_packet, create_unsubscribe_packet},
        AsyncEventHandler, ConnectOptions, MqttHandler,
    };

    pub struct Nop {}
    
    impl AsyncEventHandler for Nop {
        async fn handle(&mut self, _event: Packet) {}
    }

    fn handler(clean_start: bool) -> (MqttHandler, Receiver<u16>) {
        let mut opt = ConnectOptions::new("test123123".to_string());
        opt.receive_maximum = Some(100);
        opt.clean_start = clean_start;

        MqttHandler::new(&opt)
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
        let mut resp_vec = Vec::new();

        let pkid = apkid.recv().await.unwrap();
        let pub_packet = create_publish_packet(QoS::AtLeastOnce, false, false, Some(pkid));

        handler.handle_outgoing_packet(pub_packet.clone()).unwrap();

        assert!(handler.state.incoming_pub().is_empty());
        assert_eq!(handler.state.outgoing_pub_order().len(), 1);
        assert_eq!(Packet::Publish(handler.state.outgoing_pub()[pkid as usize - 1].clone().unwrap()), pub_packet);
        assert!(handler.state.outgoing_rel().is_empty());
        assert!(handler.state.outgoing_sub().is_empty());

        let puback = create_puback_packet(pkid);
        handler.handle_incoming_packet(&puback, &mut resp_vec).unwrap();

        assert!(resp_vec.is_empty());
        assert!(handler.state.incoming_pub().is_empty());
        assert!(handler.state.outgoing_pub()[pkid as usize - 1].is_none());
        assert!(handler.state.outgoing_pub_order().is_empty());
        assert!(handler.state.outgoing_rel().is_empty());
        assert!(handler.state.outgoing_sub().is_empty());
    }

    #[tokio::test]
    async fn incoming_publish_qos_1() {
        let (mut handler, _apkid) = handler(false);
        let mut resp_vec = Vec::new();

        let pub_packet = create_publish_packet(QoS::AtLeastOnce, false, false, Some(1));

        handler.handle_incoming_packet(&pub_packet, &mut resp_vec).unwrap();

        assert!(handler.state.incoming_pub().is_empty());
        assert!(handler.state.outgoing_pub_order().is_empty());
        assert!(handler.state.outgoing_rel().is_empty());
        assert!(handler.state.outgoing_sub().is_empty());

        assert_eq!(1, resp_vec.len());
        let expected_puback = create_puback_packet(1);
        assert_eq!(expected_puback, resp_vec.pop().unwrap());
    }

    #[tokio::test]
    async fn outgoing_publish_qos_2() {
        let (mut handler, apkid) = handler(false);
        let mut resp_vec = Vec::new();

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

        handler.handle_incoming_packet(&pubrec, &mut resp_vec).unwrap();

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
        assert_eq!(expected_pubrel, resp_vec.pop().unwrap());

        let pubcomp = Packet::PubComp(PubComp {
            packet_identifier: pkid,
            reason_code: PubCompReasonCode::Success,
            properties: PubCompProperties::default(),
        });

        handler.handle_incoming_packet(&pubcomp, &mut resp_vec).unwrap();

        assert!(resp_vec.is_empty());
        assert!(handler.state.incoming_pub().is_empty());
        assert!(handler.state.outgoing_pub()[pkid as usize - 1].clone().is_none());
        assert!(handler.state.outgoing_pub_order().is_empty());
        assert!(handler.state.outgoing_rel().is_empty());
        assert!(handler.state.outgoing_sub().is_empty());
    }

    #[tokio::test]
    async fn incoming_publish_qos_2() {
        let (mut handler, apkid) = handler(false);
        let mut resp_vec = Vec::new();

        let pkid = apkid.recv().await.unwrap();
        let pub_packet = create_publish_packet(QoS::ExactlyOnce, false, false, Some(pkid));

        handler.handle_incoming_packet(&pub_packet, &mut resp_vec).unwrap();

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

        assert_eq!(pubrec, resp_vec.pop().unwrap());
        assert!(resp_vec.is_empty());
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

        handler.handle_incoming_packet(&pubrel, &mut resp_vec).unwrap();

        let pubcomp = Packet::PubComp(PubComp {
            packet_identifier: pkid,
            reason_code: PubCompReasonCode::Success,
            properties: PubCompProperties::default(),
        });

        assert_eq!(pubcomp, resp_vec.pop().unwrap());
        assert!(resp_vec.is_empty());
        assert!(handler.state.incoming_pub().is_empty());
        assert!(handler.state.outgoing_pub_order().is_empty());
        assert!(handler.state.outgoing_rel().is_empty());
        assert!(handler.state.outgoing_sub().is_empty());
    }

    #[tokio::test]
    async fn outgoing_subscribe() {
        let (mut handler, apkid) = handler(false);
        let mut resp_vec = Vec::new();

        let pkid = apkid.recv().await.unwrap();

        let sub_packet = create_subscribe_packet(pkid);

        handler.handle_outgoing_packet(sub_packet).unwrap();

        assert!(resp_vec.is_empty());
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

        handler.handle_incoming_packet(&suback, &mut resp_vec).unwrap();

        assert!(handler.state.incoming_pub().is_empty());
        assert!(handler.state.outgoing_pub_order().is_empty());
        assert!(handler.state.outgoing_rel().is_empty());
        assert!(handler.state.outgoing_sub().is_empty());
        assert!(handler.state.outgoing_unsub().is_empty());
    }

    #[tokio::test]
    async fn outgoing_unsubscribe() {
        let (mut handler, apkid) = handler(false);
        let mut resp_vec = Vec::new();

        let pkid = apkid.recv().await.unwrap();

        let unsub_packet = create_unsubscribe_packet(pkid);

        handler.handle_outgoing_packet(unsub_packet).unwrap();

        assert!(resp_vec.is_empty());
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

        handler.handle_incoming_packet(&unsuback, &mut resp_vec).unwrap();

        assert!(handler.state.incoming_pub().is_empty());
        assert!(handler.state.outgoing_pub_order().is_empty());
        assert!(handler.state.outgoing_rel().is_empty());
        assert!(handler.state.outgoing_sub().is_empty());
        assert!(handler.state.outgoing_unsub().is_empty());
    }

    #[tokio::test]
    async fn retransmit_test_1() {
        let (mut handler, apkid) = handler(false);
        let mut stored_published_packets = Vec::new();
        let mut resp_vec = Vec::new();

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

        let connack = create_connack_packet(true);
        handler.handle_incoming_packet(&connack, &mut resp_vec).unwrap();

        assert_eq!(100 - 2, apkid.len());

        assert_eq!(stored_published_packets.len(), resp_vec.len());
        for i in 0..stored_published_packets.len() {
            let expected_pub = stored_published_packets.get(i).unwrap();
            let res_pub = resp_vec.get(i).unwrap();
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
                (_, _) => panic!(),
            }
        }
    }
}
