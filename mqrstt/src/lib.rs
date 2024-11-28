//! A pure rust MQTT client which is easy to use, efficient and provides both sync and async options.
//!
//! Because this crate aims to be runtime agnostic the user is required to provide their own data stream.
//! For an async approach the stream has to implement the `AsyncRead` and `AsyncWrite` traits.
//! That is [`::tokio::io::AsyncRead`] and [`::tokio::io::AsyncWrite`] for tokio and [`::smol::io::AsyncRead`] and [`::smol::io::AsyncWrite`] for smol.
//!
//! Features:
//! ----------------------------
//!  - MQTT v5
//!  - Runtime agnostic (Smol, Tokio)
//!  - Packets are acknoledged after handler has processed them
//!  - Runs on just a stream so you can use all TCP backends
//!  - Lean
//!  - Keep alive depends on actual communication
//!
//! To do
//! ----------------------------
//!  - Even More testing
//!
//! Notes:
//! ----------------------------
//! - Handlers only get incoming packets
//! - Create a new connection when an error or disconnect is encountered
//!
//! Smol example:
//! ----------------------------
//! ```rust
//! use mqrstt::{
//!     MqttClient,
//!     example_handlers::NOP,
//!     ConnectOptions,
//!     packets::{self, Packet},
//!     AsyncEventHandler,
//!     NetworkStatus,
//!     NetworkBuilder,
//! };
//!
//! smol::block_on(async {
//!     // Construct a no op handler
//!     let mut nop = NOP{};
//!
//!     // In normal operations you would want to loop this connection
//!     // To reconnect after a disconnect or error
//!     let (mut network, client) = NetworkBuilder
//!         ::new_from_client_id("mqrsttSmolExample")
//!         .smol_network();
//!     let stream = smol::net::TcpStream::connect(("broker.emqx.io", 1883))
//!         .await
//!         .unwrap();
//!     network.connect(stream, &mut nop).await.unwrap();
//!     
//!     // This subscribe is only processed when we run the network
//!     client.subscribe("mqrstt").await.unwrap();
//!
//!     let (n, t) = futures::join!(
//!         network.run(&mut nop),
//!         async {
//!             smol::Timer::after(std::time::Duration::from_secs(30)).await;
//!             client.disconnect().await.unwrap();
//!         }
//!     );
//!     assert!(n.is_ok());
//! });
//! ```
//!
//!
//!  Tokio example:
//! ----------------------------
//! ```rust
//! use mqrstt::{
//!     MqttClient,
//!     example_handlers::NOP,
//!     ConnectOptions,
//!     packets::{self, Packet},
//!     AsyncEventHandler,
//!     NetworkStatus,
//!     NetworkBuilder,
//! };
//! use tokio::time::Duration;
//!
//! #[tokio::main]
//! async fn main() {
//!     let (mut network, client) = NetworkBuilder
//!         ::new_from_client_id("TokioTcpPingPongExample")
//!         .tokio_network();
//!
//!     // Construct a no op handler
//!     let mut nop = NOP{};
//!
//!     // In normal operations you would want to loop this connection
//!     // To reconnect after a disconnect or error    
//!     let stream = tokio::net::TcpStream::connect(("broker.emqx.io", 1883))
//!         .await
//!         .unwrap();
//!     network.connect(stream, &mut nop).await.unwrap();
//!     
//!     client.subscribe("mqrstt").await.unwrap();
//!     
//!     let (n, _) = futures::join!(
//!         network.run(&mut nop),
//!         async {
//!             tokio::time::sleep(Duration::from_secs(30)).await;
//!             client.disconnect().await.unwrap();
//!         }
//!     );
//!     assert!(n.is_ok());
//! }
//! ```

const CHANNEL_SIZE: usize = 100;

mod available_packet_ids;
mod client;
mod connect_options;
mod state_handler;
mod util;

/// Contains the reader writer parts for the smol runtime.
///
/// Module [`crate::smol`] only contains a synchronized approach to call the users `Handler`.
#[cfg(feature = "smol")]
pub mod smol;
/// Contains the reader and writer parts for the tokio runtime.
///
/// Module [`crate::tokio`] contains both a synchronized and concurrent approach to call the users `Handler`.
#[cfg(feature = "tokio")]
pub mod tokio;

/// Error types that the user can see during operation of the client.
///
/// Wraps all other errors that can be encountered.
pub mod error;

/// All event handler traits are defined here.
///
/// Event handlers are used to process incoming packets.
mod event_handlers;
/// All MQTT packets are defined here
pub mod packets;
mod state;

pub use event_handlers::*;

pub use client::MqttClient;
pub use connect_options::ConnectOptions;
use state_handler::StateHandler;

use std::marker::PhantomData;
#[cfg(test)]
pub mod tests;

/// [`NetworkStatus`] Represents status of the Network object.
/// It is returned when the run handle returns from performing an operation.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum NetworkStatus {
    /// The other side indicated a shutdown shutdown, Only used in concurrent context
    ShutdownSignal,
    /// Indicate that there was an incoming disconnect and the socket has been closed.
    IncomingDisconnect,
    /// Indicate that an outgoing disconnect has been transmited and the socket is closed
    OutgoingDisconnect,
    /// The server did not respond to the ping request and the socket has been closed
    KeepAliveTimeout,
}

#[derive(Debug)]
pub struct NetworkBuilder<H, S> {
    handler: PhantomData<H>,
    stream: PhantomData<S>,
    options: ConnectOptions,
}

impl<H, S> NetworkBuilder<H, S> {
    #[inline]
    pub const fn new_from_options(options: ConnectOptions) -> Self {
        Self {
            handler: PhantomData,
            stream: PhantomData,
            options,
        }
    }
    #[inline]
    pub fn new_from_client_id<C: AsRef<str>>(client_id: C) -> Self {
        let options = ConnectOptions::new(client_id);
        Self {
            handler: PhantomData,
            stream: PhantomData,
            options,
        }
    }
}

#[cfg(feature = "tokio")]
impl<H, S> NetworkBuilder<H, S>
where
    H: AsyncEventHandler,
    S: ::tokio::io::AsyncRead + ::tokio::io::AsyncWrite + Sized + Unpin,
{
    /// Creates the needed components to run the MQTT client using a stream that implements [`::tokio::io::AsyncRead`] and [`::tokio::io::AsyncWrite`]
    ///
    /// # Example
    /// ```
    /// use mqrstt::ConnectOptions;
    ///
    /// let options = ConnectOptions::new("ExampleClient");
    /// let (mut network, client) = mqrstt::NetworkBuilder::<(), tokio::net::TcpStream>
    ///     ::new_from_options(options)
    ///     .tokio_network();
    /// ```
    pub fn tokio_network(self) -> (tokio::Network<H, S>, MqttClient)
    where
        H: AsyncEventHandler,
    {
        let (to_network_s, to_network_r) = async_channel::bounded(CHANNEL_SIZE);

        let (apkids, apkids_r) = available_packet_ids::AvailablePacketIds::new(self.options.send_maximum());

        let max_packet_size = self.options.maximum_packet_size();

        let client = MqttClient::new(apkids_r, to_network_s, max_packet_size);

        let network = tokio::Network::new(self.options, to_network_r, apkids);

        (network, client)
    }
}

#[cfg(feature = "smol")]
impl<H, S> NetworkBuilder<H, S>
where
    H: AsyncEventHandler,
    S: ::smol::io::AsyncRead + ::smol::io::AsyncWrite + Sized + Unpin,
{
    /// Creates the needed components to run the MQTT client using a stream that implements [`::tokio::io::AsyncRead`]  and [`::tokio::io::AsyncWrite`]
    /// ```
    /// let (mut network, client) = mqrstt::NetworkBuilder::<(), smol::net::TcpStream>
    ///     ::new_from_client_id("ExampleClient")
    ///     .smol_network();
    /// ```
    pub fn smol_network(self) -> (smol::Network<H, S>, MqttClient) {
        let (to_network_s, to_network_r) = async_channel::bounded(CHANNEL_SIZE);

        let (apkids, apkids_r) = available_packet_ids::AvailablePacketIds::new(self.options.send_maximum());

        let max_packet_size = self.options.maximum_packet_size();

        let client = MqttClient::new(apkids_r, to_network_s, max_packet_size);

        let network = smol::Network::<H, S>::new(self.options, to_network_r, apkids);

        (network, client)
    }
}

#[cfg(test)]
fn random_chars() -> String {
    rand::Rng::sample_iter(rand::thread_rng(), &rand::distributions::Alphanumeric).take(7).map(char::from).collect()
}

#[cfg(feature = "smol")]
#[cfg(test)]
mod smol_lib_test {

    use std::time::Duration;

    use rand::Rng;

    use crate::{example_handlers::PingPong, packets::QoS, random_chars, ConnectOptions, NetworkBuilder};

    #[test]
    fn test_smol_tcp() {
        smol::block_on(async {
            let mut client_id: String = random_chars();
            client_id += "_SmolTcpPingPong";
            let options = ConnectOptions::new(client_id);

            let address = "broker.emqx.io";
            let port = 1883;

            let (mut network, client) = NetworkBuilder::new_from_options(options).smol_network();

            let stream = smol::net::TcpStream::connect((address, port)).await.unwrap();
            let mut pingpong = PingPong::new(client.clone());

            network.connect(stream, &mut pingpong).await.unwrap();

            client.subscribe("mqrstt").await.unwrap();

            let (n, _) = futures::join!(async { network.run(&mut pingpong).await }, async {
                client.publish("mqrstt".to_string(), QoS::ExactlyOnce, false, b"ping".repeat(500)).await.unwrap();
                client.publish("mqrstt".to_string(), QoS::AtMostOnce, true, b"ping".to_vec()).await.unwrap();
                client.publish("mqrstt".to_string(), QoS::AtLeastOnce, false, b"ping".to_vec()).await.unwrap();
                client.publish("mqrstt".to_string(), QoS::ExactlyOnce, false, b"ping".repeat(500)).await.unwrap();

                smol::Timer::after(std::time::Duration::from_secs(20)).await;
                client.unsubscribe("mqrstt").await.unwrap();
                smol::Timer::after(std::time::Duration::from_secs(5)).await;
                client.disconnect().await.unwrap();
            });
            assert!(n.is_ok());
        });
    }

    #[test]
    fn test_smol_ping_req() {
        smol::block_on(async {
            let mut client_id: String = rand::thread_rng().sample_iter(&rand::distributions::Alphanumeric).take(7).map(char::from).collect();
            client_id += "_SmolTcppingrespTest";
            let mut options = ConnectOptions::new(client_id);
            options.set_keep_alive_interval(Duration::from_secs(5));

            let sleep_duration = options.get_keep_alive_interval() * 2 + options.get_keep_alive_interval() / 2;

            let address = "broker.emqx.io";
            let port = 1883;

            let (mut network, client) = NetworkBuilder::new_from_options(options).smol_network();
            let stream = smol::net::TcpStream::connect((address, port)).await.unwrap();

            let mut pingresp = crate::example_handlers::PingResp::new(client.clone());

            network.connect(stream, &mut pingresp).await.unwrap();

            let (n, _) = futures::join!(
                async {
                    match network.run(&mut pingresp).await {
                        Ok(crate::NetworkStatus::OutgoingDisconnect) => return Ok(pingresp),
                        Ok(crate::NetworkStatus::ShutdownSignal) => unreachable!(),
                        Ok(crate::NetworkStatus::KeepAliveTimeout) => panic!(),
                        Ok(crate::NetworkStatus::IncomingDisconnect) => panic!(),
                        Err(err) => return Err(err),
                    }
                },
                async {
                    smol::Timer::after(sleep_duration).await;
                    client.disconnect().await.unwrap();
                }
            );
            assert!(n.is_ok());
            let pingresp = n.unwrap();
            assert_eq!(2, pingresp.ping_resp_received);
        });
    }

    #[cfg(target_family = "windows")]
    #[test]
    fn test_close_write_tcp_stream_smol() {
        use crate::error::ConnectionError;
        use std::io::ErrorKind;

        smol::block_on(async {
            let mut client_id: String = rand::thread_rng().sample_iter(&rand::distributions::Alphanumeric).take(7).map(char::from).collect();
            client_id += "_SmolTcppingrespTest";
            let options = ConnectOptions::new(client_id);

            let address = "127.0.0.1";
            let port = 2001;

            let listener = smol::net::TcpListener::bind((address, port)).await.unwrap();

            let (n, _) = futures::join!(
                async {
                    let (mut network, client) = NetworkBuilder::new_from_options(options).smol_network();
                    let stream = smol::net::TcpStream::connect((address, port)).await.unwrap();
                    let mut pingresp = crate::example_handlers::PingResp::new(client.clone());
                    network.connect(stream, &mut pingresp).await
                },
                async move {
                    let (stream, _) = listener.accept().await.unwrap();
                    smol::Timer::after(std::time::Duration::from_secs(10)).await;
                    stream.shutdown(std::net::Shutdown::Write).unwrap();
                }
            );
            if let ConnectionError::Io(err) = n.unwrap_err() {
                assert_eq!(ErrorKind::ConnectionReset, err.kind());
                assert_eq!("Connection reset by peer".to_string(), err.to_string());
            } else {
                panic!();
            }
        });
    }
}

#[cfg(feature = "tokio")]
#[cfg(test)]
mod tokio_lib_test {
    use crate::example_handlers::PingResp;
    use crate::random_chars;
    use crate::ConnectOptions;

    use std::time::Duration;

    #[tokio::test]
    async fn test_tokio_ping_req() {
        let mut client_id: String = random_chars();
        client_id += "_TokioTcppingrespTest";
        let mut options = ConnectOptions::new(client_id);
        let keep_alive_interval = 5;
        options.set_keep_alive_interval(Duration::from_secs(keep_alive_interval));

        let wait_duration = options.get_keep_alive_interval() * 2 + options.get_keep_alive_interval() / 2;

        let (mut network, client) = crate::NetworkBuilder::new_from_options(options).tokio_network();

        let stream = tokio::net::TcpStream::connect(("broker.emqx.io", 1883)).await.unwrap();

        let mut pingresp = PingResp::new(client.clone());

        network.connect(stream, &mut pingresp).await.unwrap();

        let network_handle = tokio::task::spawn(async move {
            let _result = network.run(&mut pingresp).await;
            // check result and or restart the connection
            pingresp
        });

        tokio::time::sleep(wait_duration).await;
        client.disconnect().await.unwrap();

        tokio::time::sleep(Duration::from_secs(1)).await;

        let result = network_handle.await;
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(2, result.ping_resp_received);
    }

    #[cfg(all(feature = "tokio", target_family = "windows"))]
    #[tokio::test]
    async fn test_close_write_tcp_stream_tokio() {
        use crate::{error::ConnectionError, NetworkBuilder};
        use core::panic;
        use std::io::ErrorKind;

        let address = ("127.0.0.1", 2000);

        let client_id: String = crate::random_chars() + "_TokioTcppingrespTest";
        let options = crate::ConnectOptions::new(client_id);

        let (n, _) = tokio::join!(
            async move {
                let (mut network, client) = NetworkBuilder::new_from_options(options).tokio_network();

                let stream = tokio::net::TcpStream::connect(address).await.unwrap();

                let mut pingresp = crate::example_handlers::PingResp::new(client.clone());

                network.connect(stream, &mut pingresp).await
            },
            async move {
                let listener = smol::net::TcpListener::bind(address).await.unwrap();
                let (stream, _) = listener.accept().await.unwrap();
                tokio::time::sleep(Duration::new(10, 0)).await;
                stream.shutdown(std::net::Shutdown::Write).unwrap();
            }
        );

        if let ConnectionError::Io(err) = n.unwrap_err() {
            assert_eq!(ErrorKind::UnexpectedEof, err.kind());
        } else {
            panic!();
        }
    }
}
