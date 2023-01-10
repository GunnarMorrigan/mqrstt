use std::collections::{BTreeMap, BTreeSet};

use async_channel::Receiver;

use crate::{
    available_packet_ids::AvailablePacketIds,
    packets::{Publish, Subscribe, Unsubscribe},
};

#[derive(Debug)]
pub struct State {
    pub(crate) apkid: AvailablePacketIds,

    /// Outgoing Subcribe requests which aren't acked yet
    pub(crate) outgoing_sub: BTreeMap<u16, Subscribe>,
    /// Outgoing Unsubcribe requests which aren't acked yet
    pub(crate) outgoing_unsub: BTreeMap<u16, Unsubscribe>,
    /// Outgoing QoS 1, 2 publishes which aren't acked yet
    pub(crate) outgoing_pub: BTreeMap<u16, Publish>,
    /// Packet ids of released QoS 2 publishes
    pub(crate) outgoing_rel: BTreeSet<u16>,

    /// Packets on incoming QoS 2 publishes
    pub(crate) incoming_pub: BTreeSet<u16>,
}

impl State {
    pub fn new(receive_maximum: u16) -> (Self, Receiver<u16>) {
        let (apkid, r) = AvailablePacketIds::new(receive_maximum);

        let state = Self {
            apkid,
            outgoing_sub: BTreeMap::new(),
            outgoing_unsub: BTreeMap::new(),
            outgoing_pub: BTreeMap::new(),
            outgoing_rel: BTreeSet::new(),
            incoming_pub: BTreeSet::new(),
        };

        (state, r)
    }
}
