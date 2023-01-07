use std::time::Instant;

use async_channel::{Receiver, Sender};
use futures::{AsyncRead, AsyncWrite, FutureExt};
use smol::io::{AsyncReadExt, AsyncWriteExt};

use crate::connect_options::ConnectOptions;
use crate::connections::smol_stream::SmolStream;
use crate::error::ConnectionError;
use crate::packets::error::ReadBytes;
use crate::packets::Packet;

pub struct SmolNetwork<S> {
	network: Option<SmolStream<S>>,

	/// Options of the current mqtt connection
	options: ConnectOptions,

	last_network_action: Option<Instant>,

	network_to_handler_s: Sender<Packet>,

	to_network_r: Receiver<Packet>,
}

impl<S> SmolNetwork<S>
where
	S: AsyncRead + AsyncWrite + AsyncReadExt + AsyncWriteExt + Sized + Unpin,
{
	pub fn new(
		options: ConnectOptions,
		network_to_handler_s: Sender<Packet>,
		to_network_r: Receiver<Packet>,
	) -> Self {
		Self {
			network: None,

			options,

			last_network_action: None,

			network_to_handler_s,
			to_network_r,
		}
	}

	pub fn reset(&mut self) {
		self.network = None;
	}

	pub async fn connect(&mut self, stream: S) -> Result<(), ConnectionError> {
		let (network, connack) = SmolStream::connect(&self.options, stream).await?;

		self.network = Some(network);
		self.network_to_handler_s.send(connack).await?;
		Ok(())
	}

	pub async fn run(&mut self) -> Result<(), ConnectionError> {
		if self.network.is_none() {
			return Err(ConnectionError::NoNetwork);
		}

		let SmolNetwork {
			network,
			options: _,
			last_network_action,
			network_to_handler_s,
			to_network_r,
		} = self;

		if let Some(stream) = network {
			futures::select! {
				_ = stream.read_bytes().fuse() => {

					match stream.parse_messages(network_to_handler_s).await {
						Err(ReadBytes::InsufficientBytes(_)) => (),
						Err(ReadBytes::Err(err)) => return Err(err),
						Ok(()) => (),
					};
				},
				outgoing = to_network_r.recv().fuse() => {
					stream.write(&(outgoing?)).await?;
				}
			}

			Ok(())
		}
		else {
			Err(ConnectionError::NoNetwork)
		}
	}
}
