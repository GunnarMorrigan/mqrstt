use std::{
    collections::{BTreeSet, VecDeque},
    sync::Mutex,
};

use crate::{
    available_packet_ids::AvailablePacketIds,
    error::HandlerError,
    packets::{Packet, PubRel, Publish},
};

#[derive(Debug)]
/// [`State`] keeps track of the outgoing and incoming messages on which actions needs to be taken.
/// In the future this will be adjusted to rebroadcast packets that have not been acked and thus need to be rebroadcast.
pub struct State {
    apkid: AvailablePacketIds,

    /// Outgoing Subcribe requests which aren't acked yet
    outgoing_sub: Mutex<BTreeSet<u16>>,
    /// Outgoing Unsubcribe requests which aren't acked yet
    outgoing_unsub: Mutex<BTreeSet<u16>>,

    /// Outgoing QoS 1, 2 publishes which aren't acked yet
    outgoing_pub: Mutex<Vec<Option<Publish>>>,
    /// The order of the publish packets. This needs to be tracked to maintain in order communicaion on retransmit
    outgoing_pub_order: Mutex<VecDeque<u16>>,

    /// Packet ids of released QoS 2 publishes
    outgoing_rel: Mutex<BTreeSet<u16>>,

    /// Packet IDs of packets that arrive with QoS 2
    incoming_pub: Mutex<BTreeSet<u16>>,
}

impl State {
    pub fn new(receive_maximum: u16, apkid: AvailablePacketIds) -> Self {
        Self {
            apkid,

            outgoing_sub: Mutex::new(BTreeSet::new()),
            outgoing_unsub: Mutex::new(BTreeSet::new()),
            outgoing_pub: Mutex::new(vec![None; receive_maximum as usize]),

            // shifting should be minimal with a vecdeque
            outgoing_pub_order: Mutex::new(VecDeque::new()),
            outgoing_rel: Mutex::new(BTreeSet::new()),
            incoming_pub: Mutex::new(BTreeSet::new()),
        }
    }

    pub fn make_pkid_available(&self, pkid: u16) -> Result<(), HandlerError> {
        self.apkid.mark_available(pkid)
    }

    /// Returns true is newly inserted.
    /// False otherwise
    pub fn add_incoming_pub(&self, pkid: u16) -> bool {
        self.incoming_pub.lock().unwrap().insert(pkid)
    }

    /// Returns whether the packett id was present.
    pub fn remove_incoming_pub(&self, pkid: u16) -> bool {
        self.incoming_pub.lock().unwrap().remove(&pkid)
    }

    pub fn add_outgoing_pub(&self, pkid: u16, publish: Publish) -> Result<(), HandlerError> {
        let mut outgoing_pub = self.outgoing_pub.lock().unwrap();
        let current_pub = outgoing_pub[(pkid - 1) as usize].replace(publish);

        if current_pub.is_some() {
            Err(HandlerError::PacketIdCollision(pkid))
        } else {
            let mut outgoing_pub_order = self.outgoing_pub_order.lock().unwrap();
            outgoing_pub_order.push_back(pkid);
            Ok(())
        }
    }

    pub fn remove_outgoing_pub(&self, pkid: u16) -> Option<Publish> {
        {
            let mut outgoing_pub_order = self.outgoing_pub_order.lock().unwrap();

            for (index, id) in outgoing_pub_order.iter().enumerate() {
                if pkid == *id {
                    outgoing_pub_order.remove(index);
                    break;
                }
            }
        }

        let mut outgoing_pub = self.outgoing_pub.lock().unwrap();
        outgoing_pub[pkid as usize - 1].take()
    }

    pub fn add_outgoing_rel(&self, pkid: u16) -> bool {
        self.outgoing_rel.lock().unwrap().insert(pkid)
    }

    /// Returns whether the packett id was present.
    pub fn remove_outgoing_rel(&self, pkid: &u16) -> bool {
        self.outgoing_rel.lock().unwrap().remove(pkid)
    }

    pub fn add_outgoing_sub(&self, pkid: u16) -> bool {
        self.outgoing_sub.lock().unwrap().insert(pkid)
    }

    /// Returns whether the packett id was present.
    pub fn remove_outgoing_sub(&self, pkid: u16) -> bool {
        self.outgoing_sub.lock().unwrap().remove(&pkid)
    }

    pub fn add_outgoing_unsub(&self, pkid: u16) -> bool {
        self.outgoing_unsub.lock().unwrap().insert(pkid)
    }

    /// Returns whether the packett id was present.
    pub fn remove_outgoing_unsub(&self, pkid: u16) -> bool {
        self.outgoing_unsub.lock().unwrap().remove(&pkid)
    }

    /// Returns the identifiers that are in use but can be freed
    pub fn reset(&self, retransmission: bool) -> (Vec<u16>, Vec<Packet>) {
        let State {
            apkid: _,
            outgoing_sub,
            outgoing_unsub,
            outgoing_pub,
            outgoing_pub_order,
            outgoing_rel,
            incoming_pub,
        } = self;

        let mut outgoing_sub = outgoing_sub.lock().unwrap();
        let mut outgoing_unsub = outgoing_unsub.lock().unwrap();
        let mut outgoing_pub = outgoing_pub.lock().unwrap();
        let mut outgoing_pub_order = outgoing_pub_order.lock().unwrap();
        let mut outgoing_rel = outgoing_rel.lock().unwrap();
        let mut incoming_pub = incoming_pub.lock().unwrap();

        let mut freeable_ids = Vec::<u16>::with_capacity(outgoing_sub.len() + outgoing_unsub.len());
        // let mut freeable_ids = outgoing_sub.iter().chain(outgoing_unsub.iter()).collect::<Vec<u16>>();
        let mut retransmit = Vec::with_capacity(outgoing_pub_order.len());

        freeable_ids.extend(outgoing_sub.iter());
        freeable_ids.extend(outgoing_unsub.iter());

        if retransmission {
            for i in outgoing_pub_order.iter() {
                let mut packet = outgoing_pub[(*i - 1) as usize].clone().unwrap();
                packet.dup = true;
                retransmit.push(Packet::Publish(packet));
            }

            for &rel in outgoing_rel.iter() {
                retransmit.push(Packet::PubRel(PubRel::new(rel)));
            }
        } else {
            freeable_ids.extend(outgoing_pub_order.iter());

            *outgoing_pub = vec![None; outgoing_pub.len()];
            outgoing_pub_order.clear();
            outgoing_rel.clear();
        }

        outgoing_sub.clear();
        outgoing_unsub.clear();

        incoming_pub.clear();

        (freeable_ids, retransmit)
    }
}

#[cfg(test)]
impl State {
    pub fn outgoing_sub(&mut self) -> std::sync::MutexGuard<'_, BTreeSet<u16>> {
        self.outgoing_sub.lock().unwrap()
    }
    pub fn outgoing_unsub(&mut self) -> std::sync::MutexGuard<'_, BTreeSet<u16>> {
        self.outgoing_unsub.lock().unwrap()
    }
    pub fn outgoing_pub(&mut self) -> std::sync::MutexGuard<'_, Vec<Option<Publish>>> {
        self.outgoing_pub.lock().unwrap()
    }
    pub fn outgoing_pub_order(&mut self) -> std::sync::MutexGuard<'_, VecDeque<u16>> {
        self.outgoing_pub_order.lock().unwrap()
    }
    pub fn outgoing_rel(&mut self) -> std::sync::MutexGuard<'_, BTreeSet<u16>> {
        self.outgoing_rel.lock().unwrap()
    }
    pub fn incoming_pub(&mut self) -> std::sync::MutexGuard<'_, BTreeSet<u16>> {
        self.incoming_pub.lock().unwrap()
    }
}
