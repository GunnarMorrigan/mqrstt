//! A pure rust MQTT client which strives to be as efficient as possible.
//! This crate strives to provide an ergonomic API and design that fits Rust.
//!
//! There are three parts to the design of the MQTT client. The network, the event handler and the client.
//!
//! - The network - which simply reads and forms packets from the network.
//! - The event handler - which makes sure that the MQTT protocol is followed.
//!   By providing a custom handler messages are handled before they are acked, meaning that they are always handled.
//! - The client - which is used to send messages from different places.
//!
//! To Do:
//! - Rebroadcast unacked packets
//! - Enforce size of outbound messages (e.g. Publish)
//! - Sync API
//! - More testing
//! - More documentation
//! - Remove logging calls or move all to test flag
//!
//! A few questions still remain:
//! - This crate uses async channels to perform communication across its parts. Is there a better approach?
//!   These channels do allow the user to decouple the network, handlers, and clients very easily.
//! - MPL-2.0 vs MIT OR APACHE 2.0 license? [poll](https://github.com/GunnarMorrigan/mqrstt/discussions/2)
//! - The handler currently only gets INCOMING packets
//!
//!
//!
//! You want to reconnect (with a new stream) after the network encountered an error or a disconnect took place!
//!
//! Smol example:
//! ```
//! use mqrstt::{
//!     client::AsyncClient,
//!     connect_options::ConnectOptions,
//!     new_smol,
//!     packets::{self, Packet},
//!     AsyncEventHandlerMut, HandlerStatus, NetworkStatus,
//! };
//! use async_trait::async_trait;
//! use bytes::Bytes;
//! pub struct PingPong {
//!     pub client: AsyncClient,
//! }
//! #[async_trait]
//! impl AsyncEventHandlerMut for PingPong {
//!     // Handlers only get INCOMING packets. This can change later.
//!     async fn handle(&mut self, event: &packets::Packet) -> () {
//!         match event {
//!             Packet::Publish(p) => {
//!                 if let Ok(payload) = String::from_utf8(p.payload.to_vec()) {
//!                     if payload.to_lowercase().contains("ping") {
//!                         self.client
//!                             .publish(
//!                                 p.qos,
//!                                 p.retain,
//!                                 p.topic.clone(),
//!                                 Bytes::from_static(b"pong"),
//!                             )
//!                             .await
//!                             .unwrap();
//!                         println!("Received Ping, Send pong!");
//!                     }
//!                 }
//!             },
//!             Packet::ConnAck(_) => { println!("Connected!") },
//!             _ => (),
//!         }
//!     }
//! }
//! smol::block_on(async {
//!     let options = ConnectOptions::new("mqrsttExample".to_string());
//!     let (mut network, mut handler, client) = new_smol(options);
//!     let stream = smol::net::TcpStream::connect(("broker.emqx.io", 1883))
//!         .await
//!         .unwrap();
//!     network.connect(stream).await.unwrap();
//!     client.subscribe("mqrstt").await.unwrap();
//!     let mut pingpong = PingPong {
//!         client: client.clone(),
//!     };
//!     let (n, h, t) = futures::join!(
//!         async {
//!             loop {
//!                 return match network.run().await {
//!                     Ok(NetworkStatus::Active) => continue,
//!                     otherwise => otherwise,
//!                 };
//!             }
//!         },
//!         async {
//!             loop {
//!                 return match handler.handle_mut(&mut pingpong).await {
//!                     Ok(HandlerStatus::Active) => continue,
//!                     otherwise => otherwise,
//!                 };
//!             }
//!         },
//!         async {
//!             smol::Timer::after(std::time::Duration::from_secs(60)).await;
//!             client.disconnect().await.unwrap();
//!         }
//!     );
//!     assert!(n.is_ok());
//!     assert!(h.is_ok());
//! });
//! ```
//!
//!
//!  Tokio example:
//! ----------------------------
//! ```ignore
//! let options = ConnectOptions::new("TokioTcpPingPongExample".to_string());
//!
//! let (mut network, mut handler, client) = new_tokio(options);
//!
//! let stream = tokio::net::TcpStream::connect(("broker.emqx.io", 1883))
//!     .await
//!     .unwrap();
//!
//! network.connect(stream).await.unwrap();
//!
//! client.subscribe("mqrstt").await.unwrap();
//!
//! let mut pingpong = PingPong {
//!     client: client.clone(),
//! };
//!
//! let (n, h, _) = tokio::join!(
//!     async {
//!         loop {
//!             return match network.run().await {
//!                 Ok(NetworkStatus::Active) => continue,
//!                 otherwise => otherwise,
//!             };
//!         }
//!     },
//!     async {
//!         loop {
//!             return match handler.handle_mut(&mut pingpong).await {
//!                 Ok(HandlerStatus::Active) => continue,
//!                 otherwise => otherwise,
//!             };
//!         }
//!     },
//!     async {
//!         tokio::time::sleep(Duration::from_secs(60)).await;
//!         client.disconnect().await.unwrap();
//!     }
//! );
//! ```

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

#[cfg(feature = "smol")]
pub mod smol_network;
#[cfg(feature = "tokio")]
pub mod tokio_network;

pub mod state;

mod util;

#[cfg(test)]
pub mod tests;

/// [`NetworkStatus`] Represents status of the Network object.
/// It is returned when the run handle returns from performing an operation.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum NetworkStatus {
    Active,
    IncomingDisconnect,
    OutgoingDisconnect,
    NoPingResp,
}

/// [`HandlerStatus`] Represents status of the Network object.
/// It is returned when the run handle returns from performing an operation.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum HandlerStatus {
    Active,
    IncomingDisconnect,
    OutgoingDisconnect,
}

/// Handlers are used to deal with packets before they are further processed (acked)
/// This guarantees that the end user has handlded the packet.
/// Trait for async mutable access to handler.
/// Usefull when you have a single handler
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

#[cfg(feature = "smol")]
/// Creates the needed components to run the MQTT client using a stream that implements [`smol::io::AsyncReadExt`] and [`smol::io::AsyncWriteExt`]
pub fn new_smol<S>(options: ConnectOptions) -> (SmolNetwork<S>, EventHandlerTask, AsyncClient)
where
    S: smol::io::AsyncReadExt + smol::io::AsyncWriteExt + Sized + Unpin,
{
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

    let network = SmolNetwork::<S>::new(options, network_to_handler_s, to_network_r);

    let client = AsyncClient::new(packet_ids, client_to_handler_s, to_network_s);

    (network, handler, client)
}

#[cfg(feature = "tokio")]
/// Creates the needed components to run the MQTT client using a stream that implements [`tokio::io::AsyncReadExt`] and [`tokio::io::AsyncWriteExt`]
pub fn new_tokio<S>(
    options: ConnectOptions,
) -> (
    tokio_network::TokioNetwork<S>,
    EventHandlerTask,
    AsyncClient,
)
where
    S: tokio::io::AsyncReadExt + tokio::io::AsyncWriteExt + Sized + Unpin,
{
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
    use std::time::Duration;

    use crate::{
        client::AsyncClient,
        connect_options::ConnectOptions,
        new_smol, new_tokio,
        packets::{self, Packet},
        tests::tls::tests::simple_rust_tls,
        AsyncEventHandlerMut, HandlerStatus, NetworkStatus,
    };
    use async_trait::async_trait;
    use bytes::Bytes;
    use rustls::ServerName;

    pub struct PingPong {
        pub client: AsyncClient,
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
                                .await
                                .unwrap();
                            println!("Received Ping, Send pong!");
                        }
                    }
                }
                Packet::ConnAck(_) => {
                    println!("Connected!")
                }
                _ => (),
            }
        }
    }

    #[test]
    fn test_smol_tcp() {
        smol::block_on(async {
            let options = ConnectOptions::new("SmolTcpPingPong".to_string());

            let address = "broker.emqx.io";
            let port = 1883;

            let (mut network, mut handler, client) = new_smol(options);

            let stream = smol::net::TcpStream::connect((address, port))
                .await
                .unwrap();

            network.connect(stream).await.unwrap();

            client.subscribe("mqrstt").await.unwrap();

            let mut pingpong = PingPong {
                client: client.clone(),
            };

            let (n, h, _) = futures::join!(
                async {
                    loop {
                        return match network.run().await {
                            Ok(NetworkStatus::Active) => continue,
                            otherwise => otherwise,
                        };
                    }
                },
                async {
                    loop {
                        return match handler.handle_mut(&mut pingpong).await {
                            Ok(HandlerStatus::Active) => continue,
                            otherwise => otherwise,
                        };
                    }
                },
                async {
                    smol::Timer::after(std::time::Duration::from_secs(30)).await;
                    client.disconnect().await.unwrap();
                }
            );
            assert!(n.is_ok());
            assert!(h.is_ok());
        });
    }

    #[test]
    fn test_smol_tls() {
        smol::block_on(async {
            let options = ConnectOptions::new("SmolTlsPingPong".to_string());

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

            let mut pingpong = PingPong {
                client: client.clone(),
            };

            let (n, h, _) = futures::join!(
                async {
                    loop {
                        return match network.run().await {
                            Ok(NetworkStatus::Active) => continue,
                            otherwise => otherwise,
                        };
                    }
                },
                async {
                    loop {
                        return match handler.handle_mut(&mut pingpong).await {
                            Ok(HandlerStatus::Active) => continue,
                            otherwise => otherwise,
                        };
                    }
                },
                async {
                    smol::Timer::after(std::time::Duration::from_secs(30)).await;
                    client.disconnect().await.unwrap();
                }
            );
            assert!(n.is_ok());
            assert!(h.is_ok());
        });
    }

    #[tokio::test]
    async fn test_tokio_tcp() {
        let options = ConnectOptions::new("TokioTcpPingPong".to_string());

        let (mut network, mut handler, client) = new_tokio(options);

        let stream = tokio::net::TcpStream::connect(("broker.emqx.io", 1883))
            .await
            .unwrap();

        network.connect(stream).await.unwrap();

        client.subscribe("mqrstt").await.unwrap();

        let mut pingpong = PingPong {
            client: client.clone(),
        };

        let (n, h, _) = tokio::join!(
            async {
                loop {
                    return match network.run().await {
                        Ok(NetworkStatus::Active) => continue,
                        otherwise => otherwise,
                    };
                }
            },
            async {
                loop {
                    return match handler.handle_mut(&mut pingpong).await {
                        Ok(HandlerStatus::Active) => continue,
                        otherwise => otherwise,
                    };
                }
            },
            async {
                tokio::time::sleep(Duration::from_secs(30)).await;
                client.disconnect().await.unwrap();
            }
        );
        assert!(n.is_ok());
        assert!(h.is_ok());

        assert_eq!(NetworkStatus::OutgoingDisconnect, n.unwrap());
        assert_eq!(HandlerStatus::OutgoingDisconnect, h.unwrap());
    }

    #[tokio::test]
    async fn test_tokio_tls() {
        let options = ConnectOptions::new("TokioTlsPingPong".to_string());

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

        let mut pingpong = PingPong {
            client: client.clone(),
        };

        let (n, h, _) = tokio::join!(
            async {
                loop {
                    return match network.run().await {
                        Ok(NetworkStatus::IncomingDisconnect) => {
                            Ok(NetworkStatus::IncomingDisconnect)
                        }
                        Ok(NetworkStatus::OutgoingDisconnect) => {
                            Ok(NetworkStatus::OutgoingDisconnect)
                        }
                        Ok(NetworkStatus::NoPingResp) => Ok(NetworkStatus::NoPingResp),
                        Ok(NetworkStatus::Active) => continue,
                        Err(a) => Err(a),
                    };
                }
            },
            async {
                loop {
                    return match handler.handle_mut(&mut pingpong).await {
                        Ok(HandlerStatus::IncomingDisconnect) => {
                            Ok(NetworkStatus::IncomingDisconnect)
                        }
                        Ok(HandlerStatus::OutgoingDisconnect) => {
                            Ok(NetworkStatus::OutgoingDisconnect)
                        }
                        Ok(HandlerStatus::Active) => continue,
                        Err(a) => Err(a),
                    };
                }
            },
            async {
                tokio::time::sleep(Duration::from_secs(30)).await;
                client.disconnect().await.unwrap();
            }
        );
        assert!(n.is_ok());
        assert!(h.is_ok());
    }
}
