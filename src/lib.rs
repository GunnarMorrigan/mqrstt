//! A pure rust MQTT client which is easy to use, efficient and provides both sync and async options.
//!
//! Because this crate aims to be runtime agnostic the user is required to provide their own data stream.
//! For an async approach the stream has to implement the smol or tokio [`AsyncReadExt`] and [`AsyncWriteExt`] traits.
//! For a sync approach the stream has to implement the [`std::io::Read`] and [`std::io::Write`] traits.
//!
//!
//! Features:
//! ----------------------------
//!  - MQTT v5
//!  - Runtime agnostic (Smol, Tokio)
//!  - Sync
//!  - TLS/TCP
//!  - Lean
//!  - Keep alive depends on actual communication
//!  
//!
//! To do
//! ----------------------------
//!  - Enforce size of outbound messages (e.g. Publish)
//!  - QUIC via QUINN
//!  - Even More testing
//!  - More documentation
//!  - Remove logging calls or move all to test flag
//!
//!
//! Notes:
//! ----------------------------
//! - Your handler should not wait too long
//! - Create a new connection when an error or disconnect is encountered
//! - Handlers only get incoming packets
//! - Sync mode requires a non blocking stream
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
//!                                 p.topic.clone(),
//!                                 p.qos,
//!                                 p.retain,
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
//!     
//!     let mut pingpong = PingPong {
//!         client: client.clone(),
//!     };
//!
//!     network.connect(stream, &mut pingpong).await.unwrap();
//!
//!     // This subscribe is only processed when we run the network
//!     client.subscribe("mqrstt").await.unwrap();
//!
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
//!                                 p.topic.clone(),
//!                                 p.qos,
//!                                 p.retain,
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

//!     let mut pingpong = PingPong {
//!         client: client.clone(),
//!     };
//!     
//!     network.connect(stream, &mut pingpong).await.unwrap();
//!     
//!     client.subscribe("mqrstt").await.unwrap();
//!     
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
//!
//! Sync example:
//! ----------------------------
//! ```rust
//! use mqrstt::{
//!     MqttClient,
//!     ConnectOptions,
//!     new_sync,
//!     packets::{self, Packet},
//!     EventHandler, NetworkStatus,
//! };
//! use std::net::TcpStream;
//! use bytes::Bytes;
//!
//! pub struct PingPong {
//!     pub client: MqttClient,
//! }
//!
//! impl EventHandler for PingPong {
//!     // Handlers only get INCOMING packets. This can change later.
//!     fn handle(&mut self, event: packets::Packet) -> () {
//!         match event {
//!             Packet::Publish(p) => {
//!                 if let Ok(payload) = String::from_utf8(p.payload.to_vec()) {
//!                     if payload.to_lowercase().contains("ping") {
//!                         self.client
//!                             .publish_blocking(
//!                                 p.topic.clone(),
//!                                 p.qos,
//!                                 p.retain,
//!                                 Bytes::from_static(b"pong"),
//!                             ).unwrap();
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
//!
//! let mut client_id: String = "SyncTcpPingReqTestExample".to_string();
//! let options = ConnectOptions::new(client_id);
//!
//! let address = "broker.emqx.io";
//! let port = 1883;
//!
//! let (mut network, client) = new_sync(options);
//!
//! // IMPORTANT: Set nonblocking to true! No progression will be made when stream reads block!
//! let stream = TcpStream::connect((address, port)).unwrap();
//! stream.set_nonblocking(true).unwrap();
//!
//! let mut pingpong = PingPong {
//!     client: client.clone(),
//! };
//! 
//! network.connect(stream, &mut pingpong).unwrap();
//!
//! let res_join_handle = std::thread::spawn(move ||
//!     loop {
//!         match network.poll(&mut pingpong) {
//!             Ok(NetworkStatus::Active) => continue,
//!             otherwise => return otherwise,
//!         }
//!     }
//! );
//!
//! std::thread::sleep(std::time::Duration::from_secs(30));
//! client.disconnect_blocking().unwrap();
//! let join_res = res_join_handle.join();
//! assert!(join_res.is_ok());
//! let res = join_res.unwrap();
//! assert!(res.is_ok());
//! ```

mod available_packet_ids;
mod client;
mod connect_options;
mod mqtt_handler;
mod util;

#[cfg(feature = "smol")]
pub mod smol;
#[cfg(feature = "sync")]
pub mod sync;
#[cfg(feature = "tokio")]
pub mod tokio;

pub mod error;
pub mod packets;
pub mod state;

pub use client::MqttClient;
pub use connect_options::ConnectOptions;
pub use mqtt_handler::MqttHandler;
use packets::{Connect, Packet, ConnectProperties};

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

/// Handlers are used to deal with packets before they are further processed (acked)
/// This guarantees that the end user has handlded the packet.
/// Trait for async mutable access to handler.
/// Usefull when you have a single handler
#[async_trait::async_trait]
pub trait AsyncEventHandler {
    async fn handle(&mut self, incoming_packet: Packet);
}

pub trait EventHandler {
    fn handle(&mut self, incoming_packet: Packet);
}

#[cfg(feature = "smol")]
/// Creates the needed components to run the MQTT client using a stream that implements [`smol::io::AsyncReadExt`] and [`smol::io::AsyncWriteExt`]
/// ```
/// use mqrstt::ConnectOptions;
/// 
/// let options = ConnectOptions::new("ExampleClient".to_string());
/// let (network, client) = mqrstt::new_tokio::<tokio::net::TcpStream>(options);
/// ```
pub fn new_smol<S>(options: ConnectOptions) -> (smol::Network<S>, MqttClient)
where
    S: ::smol::io::AsyncReadExt + ::smol::io::AsyncWriteExt + Sized + Unpin,
{
    let (to_network_s, to_network_r) = async_channel::bounded(100);

    let (handler, packet_ids) = MqttHandler::new(&options);

    let max_packet_size = options.maximum_packet_size;

    let network = smol::Network::<S>::new(options, handler, to_network_r);

    let client = MqttClient::new(packet_ids, to_network_s, max_packet_size);

    (network, client)
}

/// Creates the needed components to run the MQTT client using a stream that implements [`tokio::io::AsyncReadExt`] and [`tokio::io::AsyncWriteExt`]
#[cfg(feature = "tokio")]
/// # Example
/// 
/// ```
/// use mqrstt::ConnectOptions;
/// 
/// let options = ConnectOptions::new("ExampleClient".to_string());
/// let (network, client) = mqrstt::new_tokio::<tokio::net::TcpStream>(options);
/// ```
pub fn new_tokio<S>(options: ConnectOptions) -> (tokio::Network<S>, MqttClient)
where
    S: ::tokio::io::AsyncReadExt + ::tokio::io::AsyncWriteExt + Sized + Unpin,
{
    let (to_network_s, to_network_r) = async_channel::bounded(100);

    let (mqtt_handler, apkid) = MqttHandler::new(&options);

    let max_packet_size = options.maximum_packet_size;

    let network = tokio::Network::new(options, mqtt_handler, to_network_r);

    let client = MqttClient::new(apkid, to_network_s, max_packet_size);

    (network, client)
}
#[cfg(feature = "sync")]
/// Creates a new [`sync::Network<S>`] and [`MqttClient`] that can be connected to a broker.
/// S should implement [`std::io::Read`] and [`std::io::Write`].
/// Additionally, S should be made non_blocking otherwise it will not progress.
/// 
/// # Example
/// 
/// ```
/// use mqrstt::ConnectOptions;
/// 
/// let options = ConnectOptions::new("ExampleClient".to_string());
/// let (network, client) = mqrstt::new_sync::<std::net::TcpStream>(options);
/// ```
pub fn new_sync<S>(options: ConnectOptions) -> (sync::Network<S>, MqttClient)
where
    S: std::io::Read + std::io::Write + Sized + Unpin,
{
    let (to_network_s, to_network_r) = async_channel::bounded(100);

    let (mqtt_handler, apkid) = MqttHandler::new(&options);

    let max_packet_size = options.maximum_packet_size;

    let network = sync::Network::new(options, mqtt_handler, to_network_r);

    let client = MqttClient::new(apkid, to_network_s, max_packet_size);

    (network, client)
}

fn create_connect_from_options(options: &ConnectOptions) -> Packet {

    let connect_properties = ConnectProperties{
        session_expiry_interval: options.session_expiry_interval,
        receive_maximum: options.receive_maximum,
        maximum_packet_size: options.maximum_packet_size,
        topic_alias_maximum: options.topic_alias_maximum,
        request_response_information: options.request_response_information,
        request_problem_information: options.request_response_information,
        user_properties: options.user_properties.clone(),
        authentication_method: options.authentication_method.clone(),
        authentication_data: options.authentication_data.clone(),
    };

    let connect = Connect {
        client_id: options.client_id.clone(),
        clean_start: options.clean_start,
        keep_alive: options.keep_alive_interval_s as u16,
        username: options.username.clone(),
        password: options.password.clone(),
        connect_properties,
        protocol_version: packets::ProtocolVersion::V5,
        last_will: options.last_will.clone(),
    };

    Packet::Connect(connect)
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
        AsyncEventHandler, ConnectOptions, EventHandler, MqttClient, NetworkStatus,
    };
    use async_trait::async_trait;
    use bytes::Bytes;
    use packets::QoS;

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
                            self.client.publish(p.topic.clone(), p.qos, p.retain, Bytes::from_static(b"pong")).await.unwrap();
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
                            self.client.publish_blocking(p.topic.clone(), p.qos, p.retain, Bytes::from_static(b"pong")).unwrap();
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
        let mut client_id: String = rand::thread_rng().sample_iter(&rand::distributions::Alphanumeric).take(7).map(char::from).collect();
        client_id += "_SyncTcpPingPong";
        let options = ConnectOptions::new(client_id);

        let address = "broker.emqx.io";
        let port = 1883;

        // IMPORTANT: Set nonblocking to true! Blocking on reads will happen!
        let stream = TcpStream::connect((address, port)).unwrap();
        stream.set_nonblocking(true).unwrap();

        let (mut network, client) = new_sync(options);
        let mut pingpong = PingPong { client: client.clone() };

        network.connect(stream, &mut pingpong).unwrap();

        client.subscribe_blocking("mqrstt").unwrap();

        let res_join_handle = thread::spawn(move || loop {
            return match network.poll(&mut pingpong) {
                Ok(NetworkStatus::Active) => continue,
                otherwise => otherwise,
            };
        });

        client.publish_blocking("mqrstt".to_string(), QoS::ExactlyOnce, false, b"ping".repeat(500)).unwrap();
        client.publish_blocking("mqrstt".to_string(), QoS::AtMostOnce, true, b"ping".to_vec()).unwrap();
        client.publish_blocking("mqrstt".to_string(), QoS::AtLeastOnce, false, b"ping".to_vec()).unwrap();
        client.publish_blocking("mqrstt".to_string(), QoS::ExactlyOnce, false, b"ping".repeat(500)).unwrap();

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
            let mut client_id: String = rand::thread_rng().sample_iter(&rand::distributions::Alphanumeric).take(7).map(char::from).collect();
            client_id += "_SmolTcpPingPong";
            let options = ConnectOptions::new(client_id);

            let address = "broker.emqx.io";
            let port = 1883;

            let (mut network, client) = new_smol(options);

            let stream = smol::net::TcpStream::connect((address, port)).await.unwrap();
            let mut pingpong = PingPong { client: client.clone() };

            network.connect(stream, &mut pingpong).await.unwrap();

            client.subscribe("mqrstt").await.unwrap();

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
                    client.publish("mqrstt".to_string(), QoS::ExactlyOnce, false, b"ping".repeat(500)).await.unwrap();
                    client.publish("mqrstt".to_string(), QoS::AtMostOnce, true, b"ping".to_vec()).await.unwrap();
                    client.publish("mqrstt".to_string(), QoS::AtLeastOnce, false, b"ping".to_vec()).await.unwrap();
                    client.publish("mqrstt".to_string(), QoS::ExactlyOnce, false, b"ping".repeat(500)).await.unwrap();

                    smol::Timer::after(std::time::Duration::from_secs(20)).await;
                    client.unsubscribe("mqrstt").await.unwrap();
                    smol::Timer::after(std::time::Duration::from_secs(5)).await;
                    client.disconnect().await.unwrap();
                }
            );
            assert!(n.is_ok());
        });
    }

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn test_tokio_tcp() {
        let mut client_id: String = rand::thread_rng().sample_iter(&rand::distributions::Alphanumeric).take(7).map(char::from).collect();
        client_id += "_TokioTcpPingPong";
        let options = ConnectOptions::new(client_id);

        let (mut network, client) = new_tokio(options);

        let stream = tokio::net::TcpStream::connect(("broker.emqx.io", 1883)).await.unwrap();

        let mut pingpong = PingPong { client: client.clone() };

        network.connect(stream, &mut pingpong).await.unwrap();

        client.subscribe("mqrstt").await.unwrap();

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
                client.publish("mqrstt".to_string(), QoS::ExactlyOnce, false, b"ping".repeat(500)).await.unwrap();
                client.publish("mqrstt".to_string(), QoS::AtMostOnce, true, b"ping".to_vec()).await.unwrap();
                client.publish("mqrstt".to_string(), QoS::AtLeastOnce, false, b"ping".to_vec()).await.unwrap();
                client.publish("mqrstt".to_string(), QoS::ExactlyOnce, false, b"ping".repeat(500)).await.unwrap();

                client.unsubscribe("mqrstt").await.unwrap();

                tokio::time::sleep(Duration::from_secs(30)).await;
                client.disconnect().await.unwrap();
            }
        );
        let n = dbg!(n);
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

    impl EventHandler for PingResp {
        fn handle(&mut self, event: Packet) {
            use Packet::*;
            if event == PingResp {
                self.ping_resp_received += 1;
            }
            println!("Received packet: {}", event);
        }
    }

    #[test]
    fn test_sync_ping_req() {
        let mut client_id: String = rand::thread_rng().sample_iter(&rand::distributions::Alphanumeric).take(7).map(char::from).collect();
        client_id += "_SyncTcpPingReqTest";
        let options = ConnectOptions::new(client_id);

        let address = "broker.emqx.io";
        let port = 1883;

        let (mut network, client) = new_sync(options);

        // IMPORTANT: Set nonblocking to true! Blocking on reads will happen!
        let stream = TcpStream::connect((address, port)).unwrap();
        stream.set_nonblocking(true).unwrap();

        let mut pingresp = PingResp::new(client.clone());

        network.connect(stream, &mut pingresp).unwrap();

        let res_join_handle = thread::spawn(move || loop {
            loop {
                match network.poll(&mut pingresp) {
                    Ok(NetworkStatus::Active) => continue,
                    Ok(NetworkStatus::OutgoingDisconnect) => return Ok(pingresp),
                    Ok(NetworkStatus::NoPingResp) => panic!(),
                    Ok(NetworkStatus::IncomingDisconnect) => panic!(),
                    Err(err) => return Err(err),
                }
            }
        });

        std::thread::sleep(Duration::from_secs(150));
        client.disconnect_blocking().unwrap();
        let join_res = res_join_handle.join();
        assert!(join_res.is_ok());

        let res = join_res.unwrap();
        assert!(res.is_ok());
        let pingreq = res.unwrap();
        assert_eq!(2, pingreq.ping_resp_received);
    }

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn test_tokio_ping_req() {
        let mut client_id: String = rand::thread_rng().sample_iter(&rand::distributions::Alphanumeric).take(7).map(char::from).collect();
        client_id += "_TokioTcpPingReqTest";
        let options = ConnectOptions::new(client_id);

        let (mut network, client) = new_tokio(options);

        let stream = tokio::net::TcpStream::connect(("broker.emqx.io", 1883)).await.unwrap();

        let mut pingresp = PingResp::new(client.clone());

        network.connect(stream, &mut pingresp).await.unwrap();

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
                    smol::Timer::after(std::time::Duration::from_secs(150)).await;
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
            let mut client_id: String = rand::thread_rng().sample_iter(&rand::distributions::Alphanumeric).take(7).map(char::from).collect();
            client_id += "_SmolTcpPingReqTest";
            let options = ConnectOptions::new(client_id);

            let address = "broker.emqx.io";
            let port = 1883;

            let (mut network, client) = new_smol(options);
            let stream = smol::net::TcpStream::connect((address, port)).await.unwrap();

            let mut pingresp = PingResp::new(client.clone());

            network.connect(stream, &mut pingresp).await.unwrap();

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
