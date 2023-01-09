//! A pure rust MQTT client which strives to be as efficient as possible.
//! This crate strives to provide an ergonomic API and design that fits Rust.
//!
//! There are three parts to the design of the MQTT client. The network, the event handler and the client.
//!
//! The network - which simply reads and forms packets from the network.
//! The event handler - which makes sure that the MQTT protocol is followed.
//! By providing a custom handler during the internal handling, messages are handled before they are acked.
//! The client - which is used to send messages from different places.
//!
//! To Do:
//! - Rebroadcast unacked packets.
//! - Enforce size of outbound messages (e.g. Publish)
//! - Sync API
//!
//! A few questions still remain:
//! - This crate uses async channels to perform communication across its parts. Is there a better approach?
//!   These channels do allow the user to decouple the network, handlers, and clients very easily.
//! - This crate provides network implementation which hinder sync and async agnosticism.
//!   Would a true sansio implementation be better?
//!   At first this crate used custom async traits which are not stable (async_fn_in_trait).
//!   The current version allows the user to provide the appropriate stream.
//!   This also nicely relives us from having to deal with TLS configuration.
//!
//! For the future it could be nice to be sync, async and runtime agnostic.
//! This can be achieved by decoupling the MQTT internals from the network communication.
//! The user could provide the received packets while this crate returns the response packets.
//! Another approach could be providing the read bytes, however, QUIC supports multiple streams.
//!
//! Currently, we do provide network implementations for the smol and tokio runtimes that you can enable with feature flags.
//!
//! Tokio example:
//! ----------------------------
//! ```no_run
//! let config = RustlsConfig::Simple {
//!     ca: EMQX_CERT.to_vec(),
//!     alpn: None,
//!     client_auth: None,
//! };
//! let opt = ConnectOptions::new("broker.emqx.io".to_string(), 8883, "test123123".to_string());
//! let (mqtt_network, handler, client) = create_tokio_rustls(opt, config);
//!
//! task::spawn(async move {
//!     join!(mqtt_network.run(), handler.handle(/* Custom handler */));
//! });
//!
//! for i in 0..10 {
//!     client.publish("test", QoS::AtLeastOnce, false, b"test payload").await.unwrap();
//!     time::sleep(Duration::from_millis(100)).await;
//! }
//! ```
//!
//! Smol example:
//! ```no_run
//! smol::block_on(async{
//!     let options = ConnectOptions::new("mqrstt".to_string());
//!
//!     let address = "broker.emqx.io";
//!     let port = 8883;
//!
//!     let (mut network, mut handler, client) = smol(options);
//!
//!     let arc_client_config = simple_rust_tls(crate::tests::resources::EMQX_CERT.to_vec(), None, None).unwrap();
//!     
//!     let domain = ServerName::try_from(address).unwrap();
//!     let connector = TlsConnector::from(arc_client_config);
//!     
//!     let stream = smol::net::TcpStream::connect((address, port)).await.unwrap();
//!     let connection = connector.connect(domain, stream).await.unwrap();
//!
//!     client.subscribe("mqrstt").await.unwrap();
//!     let mut pingpong = PingPong{ client };
//!     join!(
//!         async{
//!             network.connect(connection).await.unwrap();
//!             loop{
//!                 network.run().await.unwrap();
//!             }
//!         },
//!         async {
//!             loop{
//!                 handler.handle(&mut pingpong).await.unwrap();
//!             }
//!         }
//!     );
//! });
//! ```
//!

// #![feature(async_fn_in_trait)]
// #![feature(return_position_impl_trait_in_trait)]

use std::{sync::Arc, time::Instant};

use async_mutex::Mutex;
use client::AsyncClient;
use connect_options::ConnectOptions;

use event_handler::EventHandlerTask;

use packets::Packet;
use smol_network::SmolNetwork;

mod available_packet_ids;
pub mod client;
pub mod connect_options;
pub mod connections;
pub mod error;
pub mod event_handler;
pub mod packets;
pub mod smol_network;
pub mod state;
mod util;

#[cfg(test)]
pub mod tests;
pub mod tokio_network;

/// Represents status of the Network object.
/// It is returned when the run handle returns from performing an operation.
pub enum NetworkStatus{
    Active,
    IncomingDisconnect,
    OutgoingDisconnect,
    NoPingResp,
}

pub enum HandlerStatus{
    Active,
    IncomingDisconnect,
    OutgoingDisconnect,
}

#[async_trait::async_trait]
pub trait AsyncEventHandlerMut {
	async fn handle(&mut self, event: &Packet);
}

#[async_trait::async_trait]
pub trait AsyncEventHandler {
	async fn handle(&self, event: &Packet);
}

pub trait EventHandlerMut {
	fn handle(&mut self, event: &Packet);
}

pub trait EventHandler {
	fn handle(&self, event: &Packet);
}

// #[cfg(all(feature = "smol", feature = "tokio"))]
// std::compile_error!("The features smol and tokio can not be enabled simultaiously.");

#[cfg(feature = "smol")]
pub fn new_smol<S>(options: ConnectOptions) -> (SmolNetwork<S>, EventHandlerTask, AsyncClient)
where
	S: futures::AsyncRead
		+ futures::AsyncWrite
		+ smol::io::AsyncReadExt
		+ smol::io::AsyncWriteExt
		+ Sized
		+ Unpin, {
	let receive_maximum = options.receive_maximum();

	let (to_network_s, to_network_r) = async_channel::bounded(100);
	let (network_to_handler_s, network_to_handler_r) = async_channel::bounded(100);
	let (client_to_handler_s, client_to_handler_r) =
		async_channel::bounded(receive_maximum as usize);

	let last_network_action = Arc::new(Mutex::new(Instant::now()));

	let (handler, packet_ids) = EventHandlerTask::new(
		&options,
		network_to_handler_r,
		to_network_s.clone(),
		client_to_handler_r,
	);

	let network = SmolNetwork::<S>::new(options, network_to_handler_s, to_network_r);

	let client = AsyncClient::new(packet_ids, client_to_handler_s, to_network_s);

	(network, handler, client)
}

#[cfg(feature = "tokio")]
pub fn new_tokio<S>(
	options: ConnectOptions,
) -> (
	tokio_network::TokioNetwork<S>,
	EventHandlerTask,
	AsyncClient,
)
where
	S: tokio::io::AsyncRead
		+ tokio::io::AsyncWrite
		+ tokio::io::AsyncReadExt
		+ tokio::io::AsyncWriteExt
		+ Sized
		+ Unpin, {
	let receive_maximum = options.receive_maximum();

	let (to_network_s, to_network_r) = async_channel::bounded(100);
	let (network_to_handler_s, network_to_handler_r) = async_channel::bounded(100);
	let (client_to_handler_s, client_to_handler_r) =
		async_channel::bounded(receive_maximum as usize);

	let (handler, packet_ids) = EventHandlerTask::new(
		&options,
		network_to_handler_r,
		to_network_s.clone(),
		client_to_handler_r,
	);

	let network =
		tokio_network::TokioNetwork::<S>::new(options, network_to_handler_s, to_network_r);

	let client = AsyncClient::new(packet_ids, client_to_handler_s, to_network_s);

	(network, handler, client)
}

#[cfg(test)]
mod lib_test {
	use async_trait::async_trait;
use bytes::Bytes;
	use futures::join;
	use rustls::ServerName;
	use crate::{
		client::AsyncClient,
		connect_options::ConnectOptions,
		new_smol, new_tokio,
		packets::{self, Packet},
		util::tls::tests::simple_rust_tls, AsyncEventHandler, AsyncEventHandlerMut, NetworkStatus, HandlerStatus,
	};

	pub struct PingPong {
		pub client: AsyncClient,
		pub counter: u16,
	}

	impl PingPong{
		async fn handle(&mut self, event: &packets::Packet) -> () {
			match event {
				Packet::Publish(p) => {
					if let Ok(payload) = String::from_utf8(p.payload.to_vec()) {
						if payload.to_lowercase().contains("ping") {
							self.client
								.publish(
									p.qos,
									p.retain,
									p.topic.clone(),
									Bytes::from_static(b"pong"),
								)
								.await.unwrap();
							self.counter += 1;
							println!("Received Ping, Send pong!");
						}
					}
				}
				Packet::PingResp => {
					self.client.disconnect().await.unwrap();
				}
				Packet::ConnAck(_) => {
					println!("Connected!");
				}
				_ => (),
			}
			if self.counter > 10 {
				self.client.disconnect().await.unwrap();
			}
		}
	}

	#[async_trait]
	impl AsyncEventHandlerMut for PingPong {
		async fn handle(&mut self, event: &packets::Packet) -> () {
			match event {
				Packet::Publish(p) => {
					if let Ok(payload) = String::from_utf8(p.payload.to_vec()) {
						if payload.to_lowercase().contains("ping") {
							self.client
								.publish(
									p.qos,
									p.retain,
									p.topic.clone(),
									Bytes::from_static(b"pong"),
								)
								.await.unwrap();
							self.counter += 1;
							println!("Received Ping, Send pong!");
						}
					}
				}
				Packet::PingResp => {
					self.client.disconnect().await.unwrap();
				}
				Packet::ConnAck(_) => {
					println!("Connected!");
				}
				_ => (),
			}
			if self.counter > 10 {
				self.client.disconnect().await.unwrap();
			}
		}
	}

	#[test]
	fn test_smol() {
		let filter = tracing_subscriber::filter::EnvFilter::new("none,mqrstt=trace");

		let subscriber = tracing_subscriber::FmtSubscriber::builder()
			.with_env_filter(filter)
			.with_max_level(tracing::Level::TRACE)
			.with_line_number(true)
			.finish();

		tracing::subscriber::set_global_default(subscriber)
			.expect("setting default subscriber failed");

		smol::block_on(async {
			let options = ConnectOptions::new("mqrstt".to_string());

			let address = "broker.emqx.io";
			let port = 8883;

			let (mut network, mut handler, client) = new_smol(options);

			let arc_client_config =
				simple_rust_tls(crate::tests::resources::EMQX_CERT.to_vec(), None, None).unwrap();

			let domain = ServerName::try_from(address).unwrap();
			let connector = async_rustls::TlsConnector::from(arc_client_config);

			let stream = smol::net::TcpStream::connect((address, port))
				.await
				.unwrap();
			let connection = connector.connect(domain, stream).await.unwrap();

			network.connect(connection).await.unwrap();

			client.subscribe("mqrstt").await.unwrap();

			let mut pingpong = PingPong { client, counter: 0 };

			join!(
				async {
					loop {
						network.run().await.unwrap();
					}
				},
				async {
					loop {
						// handler.handle2(hello);
						// handler.handle(&mut pingpong).await.unwrap();
					}
				}
			);
		});
	}

	#[tokio::test]
	async fn test_tokio() {
		let filter = tracing_subscriber::filter::EnvFilter::new("none,mqrstt=trace");

		let subscriber = tracing_subscriber::FmtSubscriber::builder()
			.with_env_filter(filter)
			.with_max_level(tracing::Level::TRACE)
			.with_line_number(true)
			.finish();

		tracing::subscriber::set_global_default(subscriber)
			.expect("setting default subscriber failed");

		let options = ConnectOptions::new("mqrstt".to_string());

		let address = "broker.emqx.io";
		let port = 8883;

		let (mut network, mut handler, client) = new_tokio(options);

		let arc_client_config =
			simple_rust_tls(crate::tests::resources::EMQX_CERT.to_vec(), None, None).unwrap();

		let domain = ServerName::try_from(address).unwrap();
		let connector = tokio_rustls::TlsConnector::from(arc_client_config);

		let stream = tokio::net::TcpStream::connect((address, port))
			.await
			.unwrap();
		let connection = connector.connect(domain, stream).await.unwrap();

		network.connect(connection).await.unwrap();

		client.subscribe("mqrstt").await.unwrap();

		let mut pingpong = PingPong { client, counter: 0 };

		join!(
			async {
				loop {
					return match network.run().await {
						Ok(NetworkStatus::IncomingDisconnect) => Ok(NetworkStatus::IncomingDisconnect),
						Ok(NetworkStatus::OutgoingDisconnect) => Ok(NetworkStatus::OutgoingDisconnect),
						Ok(NetworkStatus::NoPingResp) => Ok(NetworkStatus::NoPingResp),
						Ok(NetworkStatus::Active) => continue,
						Err(a) => Err(a),
					};
				}
			},
			async {
				loop {
					return match handler.handle_mut(&mut pingpong).await {
						Ok(HandlerStatus::IncomingDisconnect) => Ok(NetworkStatus::IncomingDisconnect),
						Ok(HandlerStatus::OutgoingDisconnect) => Ok(NetworkStatus::OutgoingDisconnect),
						Ok(HandlerStatus::Active) => continue,
						Err(a) => Err(a),
					};
				}
			}
		);
	}
}

