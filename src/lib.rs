//! A pure rust MQTT client which is easy to use, efficient and provides both sync and async options.
//! 
//! Because this crate aims to be runtime agnostic the user is required to provide their own data stream.
//! For an async approach the stream has to implement the smol or tokio [`AsyncReadExt`] and [`AsyncWriteExt`] traits.
//! For a sync approach the stream has to implement the [`std::io::Read`] and [`std::io::Write`] traits.
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
//! To do
//! ----------------------------
//!  - Enforce size of outbound messages (e.g. Publish)
//!  - QUIC via QUINN
//!  - Even More testing
//!  - More documentation
//!  - Remove logging calls or move all to test flag
//! 
//! Notes:
//! ----------------------------
//! - Your handler should not wait too long
//! - Create a new connection when an error or disconnect is encountered
//! - Handlers only get incoming packets
//! - Sync mode requires a non blocking stream
//! 
//! Smol example:
//! ----------------------------
//! ```rust
//! use mqrstt::{
//!     MqttClient,
//!     NOP,
//!     ConnectOptions,
//!     new_smol,
//!     packets::{self, Packet},
//!     AsyncEventHandler,
//!     smol::NetworkStatus,
//! };
//! 
//! smol::block_on(async {
//!     let options = ConnectOptions::new("mqrsttSmolExample".to_string());
//!     
//!     // Construct a no op handler
//!     let mut nop = NOP{};
//! 
//!     // In normal operations you would want to loop this connection
//!     // To reconnect after a disconnect or error
//!     let (mut network, client) = new_smol(options);
//!     let stream = smol::net::TcpStream::connect(("broker.emqx.io", 1883))
//!         .await
//!         .unwrap();
//!     network.connect(stream, &mut nop).await.unwrap();
//!     
//!     // This subscribe is only processed when we run the network
//!     client.subscribe("mqrstt").await.unwrap();
//! 
//!     let (n, t) = futures::join!(
//!         async {
//!             loop {
//!                 return match network.poll(&mut nop).await {
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
//! use mqrstt::{
//!     MqttClient,
//!     NOP,
//!     ConnectOptions,
//!     new_tokio,
//!     packets::{self, Packet},
//!     AsyncEventHandler,
//!     tokio::NetworkStatus,
//! };
//! use tokio::time::Duration;
//! 
//! #[tokio::main]
//! async fn main() {
//!     let options = ConnectOptions::new("TokioTcpPingPongExample".to_string());
//!     let (mut network, client) = new_tokio(options);
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
//!     let (n, _) = tokio::join!(
//!         async {
//!             loop {
//!                 return match network.poll(&mut nop).await {
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
//! ```
//! 
//! Sync example:
//! ----------------------------
//! ```rust
//! use mqrstt::{
//!     MqttClient,
//!     NOP,
//!     ConnectOptions,
//!     new_sync,
//!     packets::{self, Packet},
//!     EventHandler,
//!     sync::NetworkStatus,
//! };
//! use std::net::TcpStream;
//! 
//! let mut client_id: String = "SyncTcppingrespTestExample".to_string();
//! let options = ConnectOptions::new(client_id, true);
//! 
//! let address = "broker.emqx.io";
//! let port = 1883;
//! 
//! let (mut network, client) = new_sync(options);
//! 
//! // Construct a no op handler
//! let mut nop = NOP{};
//! 
//! // In normal operations you would want to loop connect
//! // To reconnect after a disconnect or error    
//! let stream = TcpStream::connect((address, port)).unwrap();
//! // IMPORTANT: Set nonblocking to true! No progression will be made when stream reads block!
//! stream.set_nonblocking(true).unwrap();
//! network.connect(stream, &mut nop).unwrap();
//! 
//! let res_join_handle = std::thread::spawn(move ||
//!     loop {
//!         match network.poll(&mut nop) {
//!             Ok(NetworkStatus::ActivePending) => {
//!                 std::thread::sleep(std::time::Duration::from_millis(100));
//!             },
//!             Ok(NetworkStatus::ActiveReady) => {
//!                 std::thread::sleep(std::time::Duration::from_millis(100));
//!             },
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

use std::sync::Arc;

pub use client::MqttClient;
pub use connect_options::ConnectOptions;
use futures::Future;
pub use mqtt_handler::StateHandler;
use packets::{Connect, ConnectProperties, Packet};

#[cfg(test)]
pub mod tests;

/// Handlers are used to deal with packets before they are further processed (acked)
/// This guarantees that the end user has handlded the packet.
/// Trait for async mutable access to handler.
/// Usefull when you have a single handler
pub trait AsyncEventHandler {
    fn handle(&self, incoming_packet: Packet) -> impl Future<Output = ()> + Send + Sync;
}

impl<T> AsyncEventHandler for Arc<T> where T: AsyncEventHandler{
    fn handle(&self, incoming_packet: Packet) -> impl Future<Output = ()> + Send + Sync {
        T::handle(&self, incoming_packet)
    }
}

pub trait EventHandler {
    fn handle(&mut self, incoming_packet: Packet);
}

/// Most basic no op handler
/// This handler performs no operations on incoming messages.
pub struct NOP{}

impl AsyncEventHandler for NOP{
    async fn handle(&self, _: Packet){

    }
}

impl EventHandler for NOP{
    fn handle(&mut self, _: Packet){

    }
}

// #[cfg(feature = "smol")]
// /// Creates the needed components to run the MQTT client using a stream that implements [`smol::io::AsyncReadExt`] and [`smol::io::AsyncWriteExt`]
// /// ```
// /// use mqrstt::ConnectOptions;
// ///
// /// let options = ConnectOptions::new("ExampleClient".to_string());
// /// let (network, client) = mqrstt::new_tokio::<tokio::net::TcpStream>(options);
// /// ```
// pub fn new_smol<S>(options: ConnectOptions) -> (smol::Network<S>, MqttClient)
// where
//     S: ::smol::io::AsyncReadExt + ::smol::io::AsyncWriteExt + Sized + Unpin,
// {
//     let (to_network_s, to_network_r) = async_channel::bounded(100);

//     let (handler, packet_ids) = StateHandler::new(&options);

//     let max_packet_size = options.maximum_packet_size;

//     let network = smol::Network::<S>::new(options, handler, to_network_r);

//     let client = MqttClient::new(packet_ids, to_network_s, max_packet_size);

//     (network, client)
// }

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
    use available_packet_ids::AvailablePacketIds;

    let (to_network_s, to_network_r) = async_channel::bounded(100);

    let (apkids, apkids_r) = AvailablePacketIds::new(options.send_maximum());

    let max_packet_size = options.maximum_packet_size();

    let network = tokio::Network::new(options, to_network_r, apkids);

    let client = MqttClient::new(apkids_r, to_network_s, max_packet_size);

    (network, client)
}
// #[cfg(feature = "sync")]
// /// Creates a new [`sync::Network<S>`] and [`MqttClient`] that can be connected to a broker.
// /// S should implement [`std::io::Read`] and [`std::io::Write`].
// /// Additionally, S should be made non_blocking otherwise it will not progress.
// ///
// /// # Example
// ///
// /// ```
// /// use mqrstt::ConnectOptions;
// ///
// /// let options = ConnectOptions::new("ExampleClient".to_string());
// /// let (network, client) = mqrstt::new_sync::<std::net::TcpStream>(options);
// /// ```
// pub fn new_sync<S>(options: ConnectOptions) -> (sync::Network<S>, MqttClient)
// where
//     S: std::io::Read + std::io::Write + Sized + Unpin,
// {
//     let (to_network_s, to_network_r) = async_channel::bounded(100);

//     let (mqtt_handler, apkid) = StateHandler::new(&options);

//     let max_packet_size = options.maximum_packet_size;

//     let network = sync::Network::new(options, mqtt_handler, to_network_r);

//     let client = MqttClient::new(apkid, to_network_s, max_packet_size);

//     (network, client)
// }

#[cfg(test)]
mod lib_test {
    use std::{
        net::TcpStream,
        thread::{self},
        time::Duration,
        sync::{Arc, atomic::AtomicU16},
    };

    #[cfg(feature = "tokio")]
    use crate::new_tokio;

    #[cfg(feature = "smol")]
    use crate::new_smol;

    #[cfg(feature = "sync")]
    use crate::new_sync;

    use rand::Rng;

    use crate::{
        packets::{self, Packet},
        AsyncEventHandler, ConnectOptions, EventHandler, MqttClient,
    };
    use bytes::Bytes;
    use packets::QoS;

    pub struct PingPong {
        pub client: MqttClient,
    }

    #[cfg(any(feature = "smol", feature = "tokio"))]
    impl AsyncEventHandler for PingPong {
        async fn handle(&self, event: packets::Packet) -> () {
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

    #[cfg(feature = "sync")]
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

    #[cfg(feature = "sync")]
    #[test]
    fn test_sync_tcp() {
        let mut client_id: String = rand::thread_rng().sample_iter(&rand::distributions::Alphanumeric).take(7).map(char::from).collect();
        client_id += "_SyncTcpPingPong";
        let options = ConnectOptions::new(client_id, true);

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
                Ok(crate::sync::NetworkStatus::ActivePending | crate::sync::NetworkStatus::ActiveReady) => continue,
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

    #[cfg(feature = "smol")]
    #[test]
    fn test_smol_tcp() {
        smol::block_on(async {
            let mut client_id: String = rand::thread_rng().sample_iter(&rand::distributions::Alphanumeric).take(7).map(char::from).collect();
            client_id += "_SmolTcpPingPong";
            let options = ConnectOptions::new(client_id, true);

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
                            Ok(crate::smol::NetworkStatus::Active) => continue,
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
        let options = ConnectOptions::new(client_id, true);

        let (mut network, client) = new_tokio(options);

        let stream = tokio::net::TcpStream::connect(("broker.emqx.io", 1883)).await.unwrap();

        let mut pingpong = Arc::new(PingPong {client: client.clone()});

        network.connect(stream, &mut pingpong).await.unwrap();

        client.subscribe("mqrstt").await.unwrap();

        let (n, _) = tokio::join!(
            async {
                loop {
                    return match network.poll(&mut pingpong).await {
                        Ok(crate::tokio::NetworkStatus::Active) => continue,
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

        assert_eq!(crate::tokio::NetworkStatus::OutgoingDisconnect, n.unwrap());
    }

    pub struct PingResp {
        pub client: MqttClient,
        pub ping_resp_received: AtomicU16,
    }

    impl PingResp {
        pub fn new(client: MqttClient) -> Self {
            Self { client, ping_resp_received: AtomicU16::new(0) }
        }
    }

    impl AsyncEventHandler for PingResp {
        async fn handle(&self, event: packets::Packet) -> () {
            use Packet::*;
            if event == PingResp {
                self.ping_resp_received.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
            println!("Received packet: {}", event);
        }
    }

    impl EventHandler for PingResp {
        fn handle(&mut self, event: Packet) {
            use Packet::*;
            if event == PingResp {
                self.ping_resp_received.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
            println!("Received packet: {}", event);
        }
    }

    #[cfg(feature = "sync")]
    #[test]
    fn test_sync_ping_req() {
        let mut client_id: String = rand::thread_rng().sample_iter(&rand::distributions::Alphanumeric).take(7).map(char::from).collect();
        client_id += "_SyncTcppingrespTest";
        let options = ConnectOptions::new(client_id, true);

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
                    Ok(crate::sync::NetworkStatus::ActivePending | crate::sync::NetworkStatus::ActiveReady) => continue,
                    Ok(crate::sync::NetworkStatus::OutgoingDisconnect) => return Ok(pingresp),
                    Ok(crate::sync::NetworkStatus::NoPingResp) => panic!(),
                    Ok(crate::sync::NetworkStatus::IncomingDisconnect) => panic!(),
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
        let pingresp = res.unwrap();
        assert_eq!(2, pingresp.ping_resp_received.load(std::sync::atomic::Ordering::Acquire));
    }

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn test_tokio_ping_req() {
        let mut client_id: String = rand::thread_rng().sample_iter(&rand::distributions::Alphanumeric).take(7).map(char::from).collect();
        client_id += "_TokioTcppingrespTest";
        let mut options = ConnectOptions::new(client_id, true);
        let mut keep_alive_interval = 5;
        options.set_keep_alive_interval(Duration::from_secs(keep_alive_interval));

        let wait_duration = options.get_keep_alive_interval() * 2 + options.get_keep_alive_interval() / 2;

        let (mut network, client) = new_tokio(options);

        let stream = tokio::net::TcpStream::connect(("broker.emqx.io", 1883)).await.unwrap();

        let mut pingresp = Arc::new(PingResp::new(client.clone()));

        network.connect(stream, &mut pingresp).await.unwrap();

        let futs = tokio::task::spawn(async move {
            tokio::join!(
                async move {
                    loop {
                        match network.poll(&mut pingresp).await {
                            Ok(crate::tokio::NetworkStatus::Active) => continue,
                            Ok(crate::tokio::NetworkStatus::OutgoingDisconnect) => return Ok(pingresp),
                            Ok(crate::tokio::NetworkStatus::NoPingResp) => panic!(),
                            Ok(crate::tokio::NetworkStatus::IncomingDisconnect) => panic!(),
                            Err(err) => return Err(err),
                        }
                    }
                },
                async move {
                    tokio::time::sleep(wait_duration).await;
                    client.disconnect().await.unwrap();
                }
            )
        });

        tokio::time::sleep(wait_duration + Duration::from_secs(1)).await;

        let (n, _) = futs.await.unwrap();
        assert!(n.is_ok());
        let pingresp = n.unwrap();
        assert_eq!(2, pingresp.ping_resp_received.load(std::sync::atomic::Ordering::Acquire));
    }

    #[cfg(all(feature = "tokio", target_family = "windows"))]
    #[tokio::test]
    async fn test_close_write_tcp_stream_tokio() {
        use crate::error::ConnectionError;
        use core::panic;
        use std::io::ErrorKind;

        let address = ("127.0.0.1", 2000);

        let mut client_id: String = rand::thread_rng().sample_iter(&rand::distributions::Alphanumeric).take(7).map(char::from).collect();
        client_id += "_TokioTcppingrespTest";
        let options = ConnectOptions::new(client_id, true);

        let (n, _) = tokio::join!(
            async move {
                let (mut network, client) = new_tokio(options);

                let stream = tokio::net::TcpStream::connect(address).await.unwrap();

                let mut pingresp = Arc::new(PingResp::new(client.clone()));

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
            assert_eq!(ErrorKind::ConnectionReset, err.kind());
            assert_eq!("Connection reset by peer".to_string(), err.to_string());
        } else {
            panic!();
        }
    }

    #[cfg(feature = "smol")]
    #[test]
    fn test_smol_ping_req() {
        smol::block_on(async {
            let mut client_id: String = rand::thread_rng().sample_iter(&rand::distributions::Alphanumeric).take(7).map(char::from).collect();
            client_id += "_SmolTcppingrespTest";
            let options = ConnectOptions::new(client_id, true);

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
                            Ok(crate::smol::NetworkStatus::Active) => continue,
                            Ok(crate::smol::NetworkStatus::OutgoingDisconnect) => return Ok(pingresp),
                            Ok(crate::smol::NetworkStatus::NoPingResp) => panic!(),
                            Ok(crate::smol::NetworkStatus::IncomingDisconnect) => panic!(),
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
            let pingresp = n.unwrap();
            assert_eq!(2, pingresp.ping_resp_received.load(std::sync::atomic::Ordering::Acquire));
        });
    }

    #[cfg(all(feature = "smol", target_family = "windows"))]
    #[test]
    fn test_close_write_tcp_stream_smol() {
        use crate::error::ConnectionError;
        use std::io::ErrorKind;

        smol::block_on(async {
            let mut client_id: String = rand::thread_rng().sample_iter(&rand::distributions::Alphanumeric).take(7).map(char::from).collect();
            client_id += "_SmolTcppingrespTest";
            let options = ConnectOptions::new(client_id, true);

            let address = "127.0.0.1";
            let port = 2001;

            let (n, _) = futures::join!(
                async {
                    let (mut network, client) = new_smol(options);
                    let stream = smol::net::TcpStream::connect((address, port)).await.unwrap();
                    let mut pingresp = PingResp::new(client.clone());
                    network.connect(stream, &mut pingresp).await
                },
                async {
                    let listener = smol::net::TcpListener::bind((address, port)).await.unwrap();
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
