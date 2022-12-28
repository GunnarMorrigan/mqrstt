use async_channel::{Receiver, Sender};
use tracing::{debug, error};

#[derive(Debug, Clone)]
pub struct AvailablePacketIds {
    sender: Sender<u16>,
}

impl AvailablePacketIds {
    pub fn new(max_inflight: u16) -> (Self, Receiver<u16>) {
        let (s, r) = async_channel::bounded(max_inflight as usize);

        for pkid in 1..=max_inflight {
            s.send_blocking(pkid).unwrap();
        }

        let apkid = Self { sender: s };
        (apkid, r)
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
