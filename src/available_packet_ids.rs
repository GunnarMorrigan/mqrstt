use async_channel::{Receiver, Sender, TrySendError};
use tracing::error;

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

    // pub fn try_mark_available(&self, pkid: u16) -> Result<(), MqttError> {
    // 	match self.sender.try_send(pkid) {
    // 		Ok(_) => {
    // 			Ok(())
    // 			// debug!("Marked packet id as available: {}", pkid);
    // 		}
    // 		Err(err) => {
    // 			error!(
    // 				"Encountered an error while marking an packet id as available. Error: {}",
    // 				err
    // 			);
    // 			Err(MqttError::PacketIdError(err.into_inner()))
    // 		}
    // 	}
    // }

    pub fn mark_available(&self, pkid: u16) -> Result<(), HandlerError> {
        match self.sender.try_send(pkid) {
            Ok(_) => {
                Ok(())
                // debug!("Marked packet id as available: {}", pkid);
            }
            Err(TrySendError::Closed(pkid)) => {
                error!(
                    "Packet Id channel was closed"
                );
                Err(HandlerError::PacketIdError(pkid))
            }
            Err(TrySendError::Full(_)) => {
                // There can never be more than the predetermined number of packet ids.
                // Meaning that they then all fit in the channel
                unreachable!()
            },
        }
    }
}
