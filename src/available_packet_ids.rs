use std::future::Future;

use async_channel::{Receiver, Sender};
use tracing::{debug, error};

#[derive(Debug, Clone)]
pub struct AvailablePacketIds {
    max_inflight: u16,
    sender: Sender<u16>,
    receiver: Receiver<u16>,
}

impl AvailablePacketIds {
    pub fn new(max_inflight: u16) -> (Self, Receiver<u16>) {
        let (s, r) = async_channel::bounded(max_inflight as usize);

        for pkid in 1..=max_inflight {
            s.send_blocking(pkid).unwrap();
        }

        let apkid = Self {
            max_inflight,
            sender: s,
            receiver: r.clone(),
        };
        (apkid, r)
    }

    pub fn get_receiver(&self) -> Receiver<u16> {
        self.receiver.clone()
    }

    pub async fn mark_available(&self, pkid: u16) {
        match self.sender.send(pkid).await {
            Ok(_) => {
                debug!("Marked packet id as available: {}", pkid);
            }
            Err(err) => {
                error!(
                    "Encountered an error while marking an packet id as available. Error: {}",
                    err
                );
            }
        }
    }
}

pub struct PacketId {}

impl Future for PacketId {
    type Output = u16;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        todo!()
    }
}
