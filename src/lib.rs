//! A pure rust MQTT client which strives to be easy to use and efficient.
//! Providing both async and sync options.
//!
//! Because this crate aims to be runtime agnostic the user is required to provide their own data stream.
//! The stream has to implement the smol or tokio [`AsyncReadExt`] and [`AsyncWriteExt`] traits.
//!
//! Notes:
//! ----------------------------
//! - Your handler should not wait too long
//! - Create a new connection when an error or disconnect is encountered
//! - Handlers only get incoming packets
//!
//!
//! Smol example:
//! ----------------------------
//! ```rust
//! use mqrstt::{
//!     MqttClient,
//!     ConnectOptions,
//!     new_smol,
//!     packets::{self, Packet},
//!     AsyncEventHandler, NetworkStatus,
//! };
//! use async_trait::async_trait;
//! use bytes::Bytes;
//! pub struct PingPong {
//!     pub client: MqttClient,
//! }
//! #[async_trait]
//! impl AsyncEventHandler for PingPong {
//!     // Handlers only get INCOMING packets. This can change later.
//!     async fn handle(&mut self, event: packets::Packet) -> () {
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
//!     let options = ConnectOptions::new("mqrsttSmolExample".to_string());
//!     let (mut network, client) = new_smol(options);
//!     let stream = smol::net::TcpStream::connect(("broker.emqx.io", 1883))
//!         .await
//!         .unwrap();
//!     network.connect(stream).await.unwrap();
//!
//!     // This subscribe is only processed when we run the network
//!     client.subscribe("mqrstt").await.unwrap();
//!
//!     let mut pingpong = PingPong {
//!         client: client.clone(),
//!     };
//!     let (n, t) = futures::join!(
//!         async {
//!             loop {
//!                 return match network.poll(&mut pingpong).await {
//!                     Ok(NetworkStatus::Active) => continue,
//!                     otherwise => otherwise,
//!                 };
//!             }
//!         },
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
//!
//! use mqrstt::{
//!     MqttClient,
//!     ConnectOptions,
//!     new_tokio,
//!     packets::{self, Packet},
//!     AsyncEventHandler, NetworkStatus,
//! };
//! use tokio::time::Duration;
//! use async_trait::async_trait;
//! use bytes::Bytes;
//!
//! pub struct PingPong {
//!     pub client: MqttClient,
//! }
//! #[async_trait]
//! impl AsyncEventHandler for PingPong {
//!     // Handlers only get INCOMING packets. This can change later.
//!     async fn handle(&mut self, event: packets::Packet) -> () {
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
//!
//! #[tokio::main]
//! async fn main() {
//!     let options = ConnectOptions::new("TokioTcpPingPongExample".to_string());
//!     
//!     let (mut network, client) = new_tokio(options);
//!     
//!     let stream = tokio::net::TcpStream::connect(("broker.emqx.io", 1883))
//!         .await
//!         .unwrap();
//!     
//!     network.connect(stream).await.unwrap();
//!     
//!     client.subscribe("mqrstt").await.unwrap();
//!     
//!     let mut pingpong = PingPong {
//!         client: client.clone(),
//!     };
//!
//!     let (n, _) = tokio::join!(
//!         async {
//!             loop {
//!                 return match network.poll(&mut pingpong).await {
//!                     Ok(NetworkStatus::Active) => continue,
//!                     otherwise => otherwise,
//!                 };
//!             }
//!         },
//!         async {
//!             tokio::time::sleep(Duration::from_secs(30)).await;
//!             client.disconnect().await.unwrap();
//!         }
//!     );
//!     assert!(n.is_ok());
//! }
//!
//! ```

mod available_packet_ids;
mod client;
mod connect_options;
pub mod error;
mod mqtt_handler;
mod network;
pub mod packets;
pub mod state;
pub mod stream;
mod util;

pub use client::MqttClient;
pub use connect_options::ConnectOptions;
pub use mqtt_handler::MqttHandler;
use packets::Packet;

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

// #[cfg(all(feature = "smol", feature = "tokio"))]
// compile_error!("Both smol and tokio runtimes not supported at once.");

/// Handlers are used to deal with packets before they are further processed (acked)
/// This guarantees that the end user has handlded the packet.
/// Trait for async mutable access to handler.
/// Usefull when you have a single handler
#[async_trait::async_trait]
pub trait AsyncEventHandler {
    async fn handle(&mut self, event: Packet);
}

pub trait EventHandler {
    fn handle(&mut self, event: Packet);
}

/// Creates the needed components to run the MQTT client using a stream that implements [`smol::io::AsyncReadExt`] and [`smol::io::AsyncWriteExt`]
#[cfg(feature = "smol")]
pub fn new_smol<S>(options: ConnectOptions) -> (network::smol::Network<S>, MqttClient)
where
    S: smol::io::AsyncReadExt + smol::io::AsyncWriteExt + Sized + Unpin,
{
    let (to_network_s, to_network_r) = async_channel::bounded(100);

    let (handler, packet_ids) = MqttHandler::new(&options);

    let network = network::smol::Network::<S>::new(options, handler, to_network_r);

    let client = MqttClient::new(packet_ids, to_network_s);

    (network, client)
}

/// Creates the needed components to run the MQTT client using a stream that implements [`tokio::io::AsyncReadExt`] and [`tokio::io::AsyncWriteExt`]
#[cfg(feature = "tokio")]
pub fn new_tokio<S>(options: ConnectOptions) -> (network::tokio::Network<S>, MqttClient)
where
    S: tokio::io::AsyncReadExt + tokio::io::AsyncWriteExt + Sized + Unpin,
{
    let (to_network_s, to_network_r) = async_channel::bounded(100);

    let (mqtt_handler, apkid) = MqttHandler::new(&options);

    let network = network::tokio::Network::new(options, mqtt_handler, to_network_r);

    let client = MqttClient::new(apkid, to_network_s);

    (network, client)
}

pub fn new_sync<S>(options: ConnectOptions) -> (network::sync::Network<S>, MqttClient)
where
    S: std::io::Read + std::io::Write + Sized + Unpin,
{
    let (to_network_s, to_network_r) = async_channel::bounded(100);

    let (mqtt_handler, apkid) = MqttHandler::new(&options);

    let network = network::sync::Network::new(options, mqtt_handler, to_network_r);

    let client = MqttClient::new(apkid, to_network_s);

    (network, client)
}
#[cfg(test)]
mod lib_test {
    use std::{
        net::TcpStream,
        thread::{self},
        time::Duration,
    };

    #[cfg(feature = "tokio")]
    use crate::new_tokio;

    use rand::Rng;

    use crate::{
        new_smol, new_sync,
        packets::{self, Packet},
        tests::tls::tests::simple_rust_tls,
        AsyncEventHandler, ConnectOptions, EventHandler, MqttClient, NetworkStatus,
    };
    use async_trait::async_trait;
    use bytes::Bytes;
    use packets::QoS;
    use rustls::ServerName;

    pub struct PingPong {
        pub client: MqttClient,
    }

    #[async_trait]
    impl AsyncEventHandler for PingPong {
        async fn handle(&mut self, event: packets::Packet) -> () {
            match event {
                Packet::Publish(p) => {
                    if let Ok(payload) = String::from_utf8(p.payload.to_vec()) {
                        if payload.to_lowercase().contains("ping") {
                            self.client.publish(p.qos, p.retain, p.topic.clone(), Bytes::from_static(b"pong")).await.unwrap();
                            // println!("Received Ping, Send pong!");
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

    impl EventHandler for PingPong {
        fn handle(&mut self, event: Packet) {
            match event {
                Packet::Publish(p) => {
                    if let Ok(payload) = String::from_utf8(p.payload.to_vec()) {
                        if payload.to_lowercase().contains("ping") {
                            self.client.publish_blocking(p.qos, p.retain, p.topic.clone(), Bytes::from_static(b"pong")).unwrap();
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
    fn test_sync_tcp() {
        let options = ConnectOptions::new("SyncTcpPingPong".to_string());

        let address = "broker.emqx.io";
        let port = 1883;

        // IMPORTANT: Set nonblocking to true! Blocking on reads will happen!
        let stream = TcpStream::connect((address, port)).unwrap();
        stream.set_nonblocking(true).unwrap();

        let (mut network, client) = new_sync(options);

        network.connect(stream).unwrap();

        client.subscribe_blocking("mqrstt").unwrap();

        let mut pingpong = PingPong { client: client.clone() };
        let res_join_handle = thread::spawn(move || loop {
            return match network.poll(&mut pingpong) {
                Ok(NetworkStatus::Active) => continue,
                otherwise => otherwise,
            };
        });

        client.publish_blocking(QoS::ExactlyOnce, false, "mqrstt".to_string(), b"ping".repeat(500)).unwrap();
        client.publish_blocking(QoS::AtMostOnce, true, "mqrstt".to_string(), b"ping".to_vec()).unwrap();
        client.publish_blocking(QoS::AtLeastOnce, false, "mqrstt".to_string(), b"ping".to_vec()).unwrap();
        client.publish_blocking(QoS::ExactlyOnce, false, "mqrstt".to_string(), b"ping".repeat(500)).unwrap();

        std::thread::sleep(std::time::Duration::from_secs(20));
        client.unsubscribe_blocking("mqrstt").unwrap();
        std::thread::sleep(std::time::Duration::from_secs(5));
        client.disconnect_blocking().unwrap();
        println!("Disconnect queued");

        let wrapped_res = res_join_handle.join();
        assert!(wrapped_res.is_ok());
        let res = dbg!(wrapped_res.unwrap());
        assert!(res.is_ok());
    }

    #[test]
    fn test_smol_tcp() {
        smol::block_on(async {
            let options = ConnectOptions::new("SmolTcpPingPong".to_string());

            let address = "broker.emqx.io";
            let port = 1883;

            let (mut network, client) = new_smol(options);

            let stream = smol::net::TcpStream::connect((address, port)).await.unwrap();

            network.connect(stream).await.unwrap();

            client.subscribe("mqrstt").await.unwrap();

            let mut pingpong = PingPong { client: client.clone() };

            let (n, _) = futures::join!(
                async {
                    loop {
                        return match network.poll(&mut pingpong).await {
                            Ok(NetworkStatus::Active) => continue,
                            otherwise => otherwise,
                        };
                    }
                },
                async {
                    client.publish(QoS::ExactlyOnce, false, "mqrstt".to_string(), b"ping".repeat(500)).await.unwrap();
                    client.publish(QoS::AtMostOnce, true, "mqrstt".to_string(), b"ping".to_vec()).await.unwrap();
                    client.publish(QoS::AtLeastOnce, false, "mqrstt".to_string(), b"ping".to_vec()).await.unwrap();
                    client.publish(QoS::ExactlyOnce, false, "mqrstt".to_string(), b"ping".repeat(500)).await.unwrap();

                    smol::Timer::after(std::time::Duration::from_secs(20)).await;
                    client.unsubscribe("mqrstt").await.unwrap();
                    smol::Timer::after(std::time::Duration::from_secs(5)).await;
                    client.disconnect().await.unwrap();
                }
            );
            assert!(n.is_ok());
        });
    }

    #[test]
    fn test_smol_tls() {
        smol::block_on(async {
            let options = ConnectOptions::new("SmolTlsPingPong".to_string());

            let address = "broker.emqx.io";
            let port = 8883;

            let (mut network, client) = new_smol(options);

            let arc_client_config = simple_rust_tls(crate::tests::resources::EMQX_CERT.to_vec(), None, None).unwrap();

            let domain = ServerName::try_from(address).unwrap();
            let connector = async_rustls::TlsConnector::from(arc_client_config);

            let stream = smol::net::TcpStream::connect((address, port)).await.unwrap();
            let connection = connector.connect(domain, stream).await.unwrap();

            network.connect(connection).await.unwrap();

            client.subscribe("mqrstt").await.unwrap();

            let mut pingpong = PingPong { client: client.clone() };

            let (n, _) = futures::join!(
                async {
                    loop {
                        return match network.poll(&mut pingpong).await {
                            Ok(NetworkStatus::Active) => continue,
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
        });
    }

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn test_tokio_tcp() {
        let options = ConnectOptions::new("TokioTcpPingPong".to_string());

        let (mut network, client) = new_tokio(options);

        let stream = tokio::net::TcpStream::connect(("broker.emqx.io", 1883)).await.unwrap();

        network.connect(stream).await.unwrap();

        client.subscribe("mqrstt").await.unwrap();

        let mut pingpong = PingPong { client: client.clone() };

        let (n, _) = tokio::join!(
            async {
                loop {
                    return match network.poll(&mut pingpong).await {
                        Ok(NetworkStatus::Active) => continue,
                        otherwise => otherwise,
                    };
                }
            },
            async {
                client.publish(QoS::ExactlyOnce, false, "mqrstt".to_string(), b"ping".repeat(500)).await.unwrap();
                client.publish(QoS::AtMostOnce, true, "mqrstt".to_string(), b"ping".to_vec()).await.unwrap();
                client.publish(QoS::AtLeastOnce, false, "mqrstt".to_string(), b"ping".to_vec()).await.unwrap();
                client.publish(QoS::ExactlyOnce, false, "mqrstt".to_string(), b"ping".repeat(500)).await.unwrap();

                client.unsubscribe("mqrstt").await.unwrap();

                tokio::time::sleep(Duration::from_secs(30)).await;
                client.disconnect().await.unwrap();
            }
        );
        let n = dbg!(n);
        assert!(n.is_ok());

        assert_eq!(NetworkStatus::OutgoingDisconnect, n.unwrap());
    }

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn test_tokio_tls() {
        let options = ConnectOptions::new("TokioTlsPingPong".to_string());

        let address = "broker.emqx.io";
        let port = 8883;

        let (mut network, client) = new_tokio(options);

        let arc_client_config = simple_rust_tls(crate::tests::resources::EMQX_CERT.to_vec(), None, None).unwrap();

        let domain = ServerName::try_from(address).unwrap();
        let connector = tokio_rustls::TlsConnector::from(arc_client_config);

        let stream = tokio::net::TcpStream::connect((address, port)).await.unwrap();
        let connection = connector.connect(domain, stream).await.unwrap();

        network.connect(connection).await.unwrap();

        client.subscribe("mqrstt").await.unwrap();

        let mut pingpong = PingPong { client: client.clone() };

        let (n, _) = tokio::join!(
            async {
                loop {
                    return match network.poll(&mut pingpong).await {
                        Ok(NetworkStatus::IncomingDisconnect) => Ok(NetworkStatus::IncomingDisconnect),
                        Ok(NetworkStatus::OutgoingDisconnect) => Ok(NetworkStatus::OutgoingDisconnect),
                        Ok(NetworkStatus::NoPingResp) => Ok(NetworkStatus::NoPingResp),
                        Ok(NetworkStatus::Active) => continue,
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
        assert_eq!(NetworkStatus::OutgoingDisconnect, n.unwrap());
    }

    pub struct PingResp {
        pub client: MqttClient,
        pub ping_resp_received: u64,
    }

    impl PingResp {
        pub fn new(client: MqttClient) -> Self {
            Self { client, ping_resp_received: 0 }
        }
    }

    #[async_trait]
    impl AsyncEventHandler for PingResp {
        async fn handle(&mut self, event: packets::Packet) -> () {
            use Packet::*;
            if event == PingResp {
                self.ping_resp_received += 1;
            }
            println!("Received packet: {}", event);
        }
    }

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn test_tokio_ping_req() {
        let mut client_id: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(7)
        .map(char::from)
        .collect();
        client_id += "_TokioTcpPingReqTest";
        let options = ConnectOptions::new(client_id);

        let (mut network, client) = new_tokio(options);

        let stream = tokio::net::TcpStream::connect(("broker.emqx.io", 1883)).await.unwrap();

        network.connect(stream).await.unwrap();

        let mut pingresp = PingResp::new(client.clone());

        let futs = tokio::task::spawn(async {
            tokio::join!(
                async move {
                    loop {
                        match network.poll(&mut pingresp).await {
                            Ok(NetworkStatus::Active) => continue,
                            Ok(NetworkStatus::OutgoingDisconnect) => return Ok(pingresp),
                            Ok(NetworkStatus::NoPingResp) => panic!(),
                            Ok(NetworkStatus::IncomingDisconnect) => panic!(),
                            Err(err) => return Err(err),
                        }
                    }
                },
                async move {
                    smol::Timer::after(std::time::Duration::from_secs(125)).await;
                    client.disconnect().await.unwrap();
                }
            )
        });

        tokio::time::sleep(Duration::new(150, 0)).await;

        let (n, _) = futs.await.unwrap();
        assert!(n.is_ok());
        let pingresp = n.unwrap();
        assert_eq!(2, pingresp.ping_resp_received);
    }

    #[test]
    fn test_smol_ping_req() {
        smol::block_on(async {
            let mut client_id: String = rand::thread_rng()
                .sample_iter(&rand::distributions::Alphanumeric)
                .take(7)
                .map(char::from)
                .collect();
            client_id += "_SmolTcpPingReqTest";
            let options = ConnectOptions::new(client_id);

            let address = "broker.emqx.io";
            let port = 1883;

            let (mut network, client) = new_smol(options);

            let stream = smol::net::TcpStream::connect((address, port)).await.unwrap();

            network.connect(stream).await.unwrap();

            let mut pingresp = PingResp::new(client.clone());

            let (n, _) = futures::join!(
                async {
                    loop {
                        match network.poll(&mut pingresp).await {
                            Ok(NetworkStatus::Active) => continue,
                            Ok(NetworkStatus::OutgoingDisconnect) => return Ok(pingresp),
                            Ok(NetworkStatus::NoPingResp) => panic!(),
                            Ok(NetworkStatus::IncomingDisconnect) => panic!(),
                            Err(err) => return Err(err),
                        }
                    }
                },
                async {
                    smol::Timer::after(std::time::Duration::from_secs(150)).await;
                    client.disconnect().await.unwrap();
                }
            );
            assert!(n.is_ok());
            let pingreq = n.unwrap();
            assert_eq!(2, pingreq.ping_resp_received);
        });
    }
}
