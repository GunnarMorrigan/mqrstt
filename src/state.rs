use std::{collections::{BTreeSet, VecDeque}};

use async_channel::Receiver;

use crate::{
    available_packet_ids::AvailablePacketIds,
    packets::{Publish, Packet, PubRel}, error::HandlerError,
};

#[derive(Debug)]
/// [`State`] keeps track of the outgoing and incoming messages on which actions needs to be taken.
/// In the future this will be adjusted to rebroadcast packets that have not been acked and thus need to be rebroadcast.
pub struct State {
    apkid: AvailablePacketIds,

    /// Outgoing Subcribe requests which aren't acked yet
    outgoing_sub: BTreeSet<u16>,
    /// Outgoing Unsubcribe requests which aren't acked yet
    outgoing_unsub: BTreeSet<u16>,

    /// Outgoing QoS 1, 2 publishes which aren't acked yet
    outgoing_pub: Vec<Option<Publish>>,
    outgoing_pub_order: VecDeque<u16>,
    /// Packet ids of released QoS 2 publishes
    outgoing_rel: BTreeSet<u16>,

    /// Packet IDs of packets that arrive with QoS 2
    incoming_pub: BTreeSet<u16>,
}

impl State {
    pub fn new(receive_maximum: u16) -> (Self, Receiver<u16>) {
        let (apkid, r) = AvailablePacketIds::new(receive_maximum);

        let state = Self {
            apkid,

            // make everything an option. We do not want to use vec::remove because it will shift everything right of the element to the left.
            // Which because we ussually remove the oldest (most left) items first there will be a lot of shifting!
            // If we just swap in place with None than we should be good.

            outgoing_sub: BTreeSet::new(),
            outgoing_unsub: BTreeSet::new(),
            outgoing_pub: vec![None; receive_maximum as usize],
            outgoing_pub_order: VecDeque::new(),
            outgoing_rel: BTreeSet::new(),
            incoming_pub: BTreeSet::new(),
        };

        (state, r)
    }
    
    pub fn make_pkid_available(&mut self, pkid: u16) -> Result<(), HandlerError>{
        self.apkid.mark_available(pkid)
    }

    pub fn add_incoming_pub(&mut self, pkid: u16) -> bool{
        self.incoming_pub.insert(pkid)
    }

    /// Returns whether the packett id was present.
    pub fn remove_incoming_pub(&mut self, pkid: u16) -> bool{
        self.incoming_pub.remove(&pkid)
    }

    pub fn add_outgoing_pub(&mut self, pkid: u16, publish: Publish) -> Result<(), HandlerError>{
        let current_pub = self.outgoing_pub[(pkid-1) as usize].take();
        self.outgoing_pub[(pkid-1) as usize] = Some(publish);

        
        if current_pub.is_some(){
            Err(HandlerError::PacketIdCollision(pkid))
        }
        else{
            self.outgoing_pub_order.push_back(pkid);
            Ok(())
        }
    }

    pub fn remove_outgoing_pub(&mut self, pkid: u16) -> Option<Publish>{
        for (index, id) in self.outgoing_pub_order.iter().enumerate(){
            if pkid == *id{
                self.outgoing_pub_order.remove(index);
                break;
            }
        }

        self.outgoing_pub[pkid as usize - 1].take()
    }

    pub fn add_outgoing_rel(&mut self, pkid: u16) -> bool{
        self.outgoing_rel.insert(pkid)
    }

    /// Returns whether the packett id was present.
    pub fn remove_outgoing_rel(&mut self, pkid: &u16) -> bool{
        self.outgoing_rel.remove(pkid)
    }

    pub fn add_outgoing_sub(&mut self, pkid: u16) -> bool{
        self.outgoing_sub.insert(pkid)
    }

    /// Returns whether the packett id was present.
    pub fn remove_outgoing_sub(&mut self, pkid: u16) -> bool{
        self.outgoing_sub.remove(&pkid)
    }

    pub fn add_outgoing_unsub(&mut self, pkid: u16) -> bool{
        self.outgoing_unsub.insert(pkid)
    }

    /// Returns whether the packett id was present.
    pub fn remove_outgoing_unsub(&mut self, pkid: u16) -> bool{
        self.outgoing_unsub.remove(&pkid)
    }

    /// Returns the identifiers that are in use but can be freed
    pub fn reset(&mut self, retransmission: bool) -> (Vec<u16>, Vec<Packet>) {

        let State{
            apkid: _,
            outgoing_sub,
            outgoing_unsub,
            outgoing_pub,
            outgoing_pub_order,
            outgoing_rel,
            incoming_pub,
        } = self;

        let mut freeable_ids = Vec::<u16>::with_capacity(outgoing_sub.len() + outgoing_unsub.len());
        // let mut freeable_ids = outgoing_sub.iter().chain(outgoing_unsub.iter()).collect::<Vec<u16>>();
        let mut retransmit = Vec::with_capacity(outgoing_pub_order.len());

        freeable_ids.extend(outgoing_sub.iter());
        freeable_ids.extend(outgoing_unsub.iter());

        if retransmission{
            for i in outgoing_pub_order{
                let mut packet = outgoing_pub[(*i - 1) as usize].take().unwrap();
                packet.dup = true;
                retransmit.push(Packet::Publish(packet));
            }

            for &rel in outgoing_rel.iter(){
                retransmit.push(Packet::PubRel(PubRel::new(rel)));
            }
        }
        else{
            freeable_ids.extend(outgoing_pub_order.iter());
            
            outgoing_pub.clear();
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
impl State{
    pub fn outgoing_sub(&mut self) -> &mut BTreeSet<u16>{
        &mut self.outgoing_sub
    }
    pub fn outgoing_unsub(&mut self) -> &mut BTreeSet<u16>{
        &mut self.outgoing_unsub
    }
    pub fn outgoing_pub(&mut self) -> &mut Vec<Option<Publish>>{
        &mut self.outgoing_pub
    }
    pub fn outgoing_pub_order(&mut self) -> &mut VecDeque<u16>{
        &mut self.outgoing_pub_order
    }
    pub fn outgoing_rel(&mut self) -> &mut BTreeSet<u16>{
        &mut self.outgoing_rel
    }
    pub fn incoming_pub(&mut self) -> &mut BTreeSet<u16>{
        &mut self.incoming_pub
    }
}
