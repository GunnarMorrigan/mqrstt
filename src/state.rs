use std::{collections::{BTreeMap, BTreeSet}, sync::atomic::AtomicBool};

use async_channel::Receiver;
use async_mutex::Mutex;

use crate::{
    available_packet_ids::AvailablePacketIds,
    packets::{publish::Publish, subscribe::Subscribe, unsubscribe::Unsubscribe},
};

#[derive(Debug)]
pub struct State {
    pub(crate) apkid: AvailablePacketIds,

    /// Outgoing Subcribe requests which aren't acked yet
    pub(crate) outgoing_sub: Mutex<BTreeMap<u16, Subscribe>>,
    /// Outgoing Unsubcribe requests which aren't acked yet
    pub(crate) outgoing_unsub: Mutex<BTreeMap<u16, Unsubscribe>>,
    /// Outgoing QoS 1, 2 publishes which aren't acked yet
    pub(crate) outgoing_pub: Mutex<BTreeMap<u16, Publish>>,
    /// Packet ids of released QoS 2 publishes
    pub(crate) outgoing_rel: Mutex<BTreeSet<u16>>,

    /// Packets on incoming QoS 2 publishes
    pub(crate) incoming_pub: Mutex<BTreeSet<u16>>,
}

impl State {
    pub fn new(receive_maximum: u16) -> (Self, Receiver<u16>) {
        let (apkid, r) = AvailablePacketIds::new(receive_maximum);

        let state = Self {
            apkid,
            outgoing_sub: Mutex::new(BTreeMap::new()),
            outgoing_unsub: Mutex::new(BTreeMap::new()),
            outgoing_pub: Mutex::new(BTreeMap::new()),
            outgoing_rel: Mutex::new(BTreeSet::new()),
            incoming_pub: Mutex::new(BTreeSet::new()),
        };

        (state, r)
    }
}
