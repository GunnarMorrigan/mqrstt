use async_channel::{Receiver, Sender, TrySendError};

#[cfg(feature = "logs")]
use tracing::{error, debug};

use crate::error::HandlerError;

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

    pub fn mark_available(&self, pkid: u16) -> Result<(), HandlerError> {
        match self.sender.try_send(pkid) {
            Ok(_) => {
                #[cfg(feature = "logs")]
                debug!("Marked packet id as available: {}", pkid);
                Ok(())
            }
            Err(TrySendError::Closed(pkid)) => {
                #[cfg(feature = "logs")]
                error!("Packet Id channel was closed");
                Err(HandlerError::PacketIdChannelError(pkid))
            }
            Err(TrySendError::Full(_)) => {
                // There can never be more than the predetermined number of packet ids.
                // Meaning that they then all fit in the channel
                unreachable!()
            }
        }
    }
}
