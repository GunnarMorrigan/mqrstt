use async_channel::{Receiver, Sender};

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use std::time::{Duration, Instant};

use crate::connect_options::ConnectOptions;
use crate::connections::tokio_stream::TokioStream;
use crate::error::ConnectionError;
use crate::packets::error::ReadBytes;
use crate::packets::{Packet, PacketType};
use crate::NetworkStatus;

pub struct TokioNetwork<S> {
	network: Option<TokioStream<S>>,

	/// Options of the current mqtt connection
	options: ConnectOptions,

	last_network_action: Instant,
	await_pingresp: Option<Instant>,
	perform_keep_alive: bool,

	network_to_handler_s: Sender<Packet>,

	to_network_r: Receiver<Packet>,
}

impl<S> TokioNetwork<S>
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

			last_network_action: Instant::now(),
			await_pingresp: None,
			perform_keep_alive: true,

			network_to_handler_s,
			to_network_r,
		}
	}

	pub fn reset(&mut self) {
		self.network = None;
	}

	pub async fn connect(&mut self, stream: S) -> Result<(), ConnectionError> {
		let (network, connack) = TokioStream::connect(&self.options, stream).await?;

		self.network = Some(network);
		self.network_to_handler_s.send(connack).await?;
		self.last_network_action = Instant::now();
		if self.options.keep_alive_interval_s == 0 {
			self.perform_keep_alive = false;
		}
		Ok(())
	}

	pub async fn run(&mut self) -> Result<NetworkStatus, ConnectionError> {
		if self.network.is_none() {
			return Err(ConnectionError::NoNetwork);
		}

		match self.select().await {
			Ok(NetworkStatus::Active) => Ok(NetworkStatus::Active),
			otherwise => {
				self.reset();
				otherwise
			}
		}
	}

	async fn select(&mut self) -> Result<NetworkStatus, ConnectionError> {
		let TokioNetwork {
			network,
			options: _,
			last_network_action,
			await_pingresp,
			perform_keep_alive,
			network_to_handler_s,
			to_network_r,
		} = self;

		let sleep;
		if let Some(instant) = await_pingresp {
			sleep =
				*instant + Duration::from_secs(self.options.keep_alive_interval_s) - Instant::now();
		}
		else {
			sleep = *last_network_action + Duration::from_secs(self.options.keep_alive_interval_s)
				- Instant::now();
		}

		if let Some(stream) = network {
			loop {
				tokio::select! {
					_ = stream.read_bytes() => {
						match stream.parse_messages(network_to_handler_s).await {
							Err(ReadBytes::Err(err)) => return Err(err),
							Err(ReadBytes::InsufficientBytes(_)) => continue,
							Ok(Some(PacketType::PingResp)) => {
								*await_pingresp = None;
								return Ok(NetworkStatus::Active)
							},
							Ok(Some(PacketType::Disconnect)) => {
								return Ok(NetworkStatus::IncomingDisconnect)
							},
							Ok(_) => {
								return Ok(NetworkStatus::Active)
							}
						};
					},
					outgoing = to_network_r.recv() => {
						let packet = outgoing?;
						stream.write(&packet).await?;
						*last_network_action = Instant::now();
						if packet.packet_type() == PacketType::Disconnect{
							return Ok(NetworkStatus::OutgoingDisconnect);
						}
						return Ok(NetworkStatus::Active);
					},
					_ = tokio::time::sleep(sleep), if await_pingresp.is_none() && *perform_keep_alive => {
						let packet = Packet::PingReq;
						stream.write(&packet).await?;
						*last_network_action = Instant::now();
						*await_pingresp = Some(Instant::now());
						return Ok(NetworkStatus::Active);
					},
					_ = tokio::time::sleep(sleep), if await_pingresp.is_some() => {
						return Ok(NetworkStatus::NoPingResp);
					}
				}
			}
		}
		else {
			Err(ConnectionError::NoNetwork)
		}
	}
}

#[test]
fn test() {
	let a = Instant::now() - Duration::from_secs(100);

	let sleep = a + Duration::from_secs(60) - Instant::now();
	dbg!(sleep);
}
