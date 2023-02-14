use std::collections::{BTreeMap, BTreeSet, VecDeque};

use async_channel::Receiver;

use crate::{
    available_packet_ids::AvailablePacketIds,
    packets::{Publish, Subscribe, Unsubscribe}, error::MqttError,
};

#[derive(Debug)]
/// [`State`] keeps track of the outgoing and incoming messages on which actions needs to be taken.
/// In the future this will be adjusted to rebroadcast packets that have not been acked and thus need to be rebroadcast.
pub struct State {
    apkid: AvailablePacketIds,

    /// Outgoing Subcribe requests which aren't acked yet
    pub(crate) outgoing_sub: Vec<u16>,
    /// Outgoing Unsubcribe requests which aren't acked yet
    pub(crate) outgoing_unsub: Vec<u16>,
    /// Outgoing QoS 1, 2 publishes which aren't acked yet
    pub(crate) outgoing_pub: Vec<Option<Publish>>,
    /// Packet ids of released QoS 2 publishes
    pub(crate) outgoing_rel: Vec<u16>,

    /// Packets on incoming QoS 2 publishes
    pub(crate) incoming_pub: BTreeSet<u16>,
}

impl State {
    pub fn new(receive_maximum: u16) -> (Self, Receiver<u16>) {
        let (apkid, r) = AvailablePacketIds::new(receive_maximum);

        let state = Self {
            apkid,

            // make everything an option. We do not want to use vec::remove because it will shift everything right of the element to the left.
            // Which because we ussually remove the oldest (most left) items first there will be a lot of shifting!
            // If we just swap in place with None than we should be good.

            outgoing_sub: Vec::new(),
            outgoing_unsub: Vec::new(),
            outgoing_pub: vec![None; receive_maximum as usize],
            outgoing_rel: Vec::new(),
            incoming_pub: BTreeSet::new(),
        };

        (state, r)
    }

    pub fn make_pkid_available(&mut self, pkid: u16) -> Result<(), MqttError>{
        self.apkid.mark_available(pkid)
    }

    pub fn add_incoming_publish(&mut self, pkid: u16){
        self.incoming_pub.insert(pkid);
    }

    pub fn remove_incoming_publish(&mut self, pkid: u16) -> bool{
        self.incoming_pub.remove(&pkid)
    }

    pub fn add_outgoing_publish(&mut self, pkid: u16, publish: Publish){
        self.outgoing_pub[(pkid-1) as usize] = Some(publish)
    }

    pub fn remove_outgoing_publish(&mut self, pkid: u16) -> Option<Publish>{
        self.outgoing_pub[pkid as usize - 1].take()
    }


    
}
