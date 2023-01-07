use tracing::{trace, warn};

use crate::{event_handler::AsyncEventHandler, packets::Packet};

pub struct Nop {}
impl AsyncEventHandler for Nop {
	fn handle<'a>(
		&'a mut self,
		event: &'a Packet,
	) -> impl core::future::Future<Output = ()> + Send + 'a {
		async move {
			// warn!("{:?}", event)
		}
	}
}

pub mod qos_2 {
	use crate::{
		client::AsyncClient,
		event_handler::AsyncEventHandler,
		packets::{Packet, PacketType},
		// packets::{Packet, PacketType},
	};

	pub struct TestPubQoS2 {
		stage: StagePubQoS2,
		client: AsyncClient,
	}
	pub enum StagePubQoS2 {
		ConnAck,
		PubRec,
		PubComp,
		Done,
	}
	impl TestPubQoS2 {
		#[allow(dead_code)]
		pub fn new(client: AsyncClient) -> Self {
			TestPubQoS2 {
				stage: StagePubQoS2::ConnAck,
				client,
			}
		}
	}
	impl AsyncEventHandler for TestPubQoS2 {
		fn handle<'a>(
			&'a mut self,
			event: &'a Packet,
		) -> impl core::future::Future<Output = ()> + Send + 'a {
			async move {
				match self.stage {
					StagePubQoS2::ConnAck => {
						assert_eq!(event.packet_type(), PacketType::ConnAck);
						self.stage = StagePubQoS2::PubRec;
					}
					StagePubQoS2::PubRec => {
						assert_eq!(event.packet_type(), PacketType::PubRec);
						self.stage = StagePubQoS2::PubComp;
					}
					StagePubQoS2::PubComp => {
						assert_eq!(event.packet_type(), PacketType::PubComp);
						self.stage = StagePubQoS2::Done;
						self.client.disconnect().await.unwrap();
					}
					StagePubQoS2::Done => (),
				}
			}
		}
	}
}
