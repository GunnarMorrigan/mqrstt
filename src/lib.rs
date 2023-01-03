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
//! - Keep alive sending of PingReq and PingResp.
//! - Enforce size of outbound messages (e.g. Publish)
//! 
//! A few questions still remain:
//! - This crate uses async channels to perform communication across its parts. Is there a better approach?
//!   These channels do allow the user to decouple this crate and the network in the future but comes at a cost of more copies
//! - This crate provides network implementation which hinder sync and async agnosticism.
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
//! 
//! ```
//! pub struct PingPong {
//! pub client: AsyncClient,
//! }
//! 
//! impl EventHandler for PingPong{
//!     fn handle<'a>(&'a mut self, event: &'a packets::Packet) -> impl std::future::Future<Output = ()> + Send + 'a {
//!         async move{
//!             match event{
//!                 Packet::Publish(p) => {
//!                     if let Ok(payload) = String::from_utf8(p.payload.to_vec()){
//!                         if payload.to_lowercase().contains("ping"){
//!                             self.client.publish(p.qos, p.retain, p.topic.clone(), Bytes::from_static(b"pong")).await;
//!                             println!("Received Ping, Send pong!");
//!                         }
//!                     }
//!                 },
//!                 Packet::ConnAck(_) => {
//!                     println!("Connected!");
//!                 }
//!                 _ => (),
//!             }
//!         }
//!     }
//! }
//! 
//! fn main(){
//!     let options = ConnectOptions::new("broker.emqx.io".to_string(), 8883, "mqrstt".to_string());
//!         
//!     let tls_config = RustlsConfig::Simple {
//!         ca: crate::tests::resources::EMQX_CERT.to_vec(),
//!         alpn: None,
//!         client_auth: None,
//!     };
//! 
//!     let (mut network, handler, client) = create_smol_rustls(options, tls_config);
//!     smol::block_on(async{
//!         client.subscribe("mqrstt").await.unwrap();
//!         
//!         let mut pingpong = PingPong{ client };
//! 
//!         join!(network.run(), handler.handle(&mut pingpong));
//!     });
//! }
//! ```
//! 


#![feature(async_fn_in_trait)]
#![feature(return_position_impl_trait_in_trait)]

use std::{sync::Arc, time::Instant};

use async_mutex::Mutex;
use client::AsyncClient;
use connect_options::ConnectOptions;

use connections::*;

// #[cfg(all(feature = "tokio", feature = "tcp"))]
// use connections::tokio_tcp::{TcpReader, TcpWriter};

use connections::{AsyncMqttNetworkRead, AsyncMqttNetworkWrite};

use connections::transport::RustlsConfig;

use event_handler::EventHandlerTask;
use network::MqttNetwork;

mod available_packet_ids;
pub mod client;
pub mod connect_options;
pub mod connections;
pub mod error;
pub mod event_handler;
mod network;
pub mod packets;
mod state;
mod util;

#[cfg(test)]
pub mod tests;

#[cfg(all(feature = "smol", feature = "smol-rustls"))]
pub fn create_smol_rustls(
    mut options: ConnectOptions,
    tls_config: RustlsConfig,
) -> (
    MqttNetwork<connections::async_rustls::TlsReader, connections::async_rustls::TlsWriter>,
    EventHandlerTask,
    AsyncClient,
) {
    use connections::transport::TlsConfig;

    options.tls_config = Some(TlsConfig::Rustls(tls_config));
    new(options)
}

#[cfg(all(feature = "tokio", feature = "tokio-rustls"))]
pub fn create_tokio_rustls(
    mut options: ConnectOptions,
    tls_config: RustlsConfig,
) -> (
    MqttNetwork<connections::tokio_rustls::TlsReader, connections::tokio_rustls::TlsWriter>,
    EventHandlerTask,
    AsyncClient,
) {
    use connections::transport::TlsConfig;

    options.tls_config = Some(TlsConfig::Rustls(tls_config));
    new(options)
}

#[cfg(all(feature = "tokio", feature = "tcp"))]
pub fn create_tokio_tcp(
    options: ConnectOptions,
) -> (
    MqttNetwork<tokio_tcp::TcpReader, tokio_tcp::TcpWriter>,
    EventHandlerTask,
    AsyncClient,
) {
    new(options)
}

#[cfg(all(feature = "smol", feature = "tcp"))]
pub fn create_smol_tcp(
    options: ConnectOptions,
) -> (
    MqttNetwork<smol_tcp::TcpReader, smol_tcp::TcpWriter>,
    EventHandlerTask,
    AsyncClient,
) {
    new(options)
}

pub fn new<R, W>(options: ConnectOptions) -> (MqttNetwork<R, W>, EventHandlerTask, AsyncClient)
where
    R: AsyncMqttNetworkRead<W = W>,
    W: AsyncMqttNetworkWrite,
{
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
        client_to_handler_r.clone(),
        last_network_action.clone(),
    );

    let network = MqttNetwork::<R, W>::new(
        options,
        network_to_handler_s,
        to_network_r,
        last_network_action,
    );

    let client = AsyncClient::new(packet_ids, client_to_handler_s, to_network_s);

    (network, handler, client)
}

#[cfg(test)]
mod lib_test{
    use bytes::Bytes;
    use futures::join;

    use crate::{client::AsyncClient, packets::{self, Packet}};
    use crate::event_handler::AsyncEventHandler;
    use crate::create_smol_rustls;
    use crate::connections::transport::RustlsConfig;
    use crate::connect_options::ConnectOptions;

    pub struct PingPong {
        pub client: AsyncClient,
    }
    
    impl AsyncEventHandler for PingPong{
        fn handle<'a>(&'a mut self, event: &'a packets::Packet) -> impl std::future::Future<Output = ()> + Send + 'a {
            async move{
                match event{
                    Packet::Publish(p) => {
                        if let Ok(payload) = String::from_utf8(p.payload.to_vec()){
                            if payload.to_lowercase().contains("ping"){
                                self.client.publish(p.qos, p.retain, p.topic.clone(), Bytes::from_static(b"pong")).await;
                                println!("Received Ping, Send pong!");
                            }
                        }
                    },
                    Packet::ConnAck(_) => {
                        println!("Connected!");
                    }
                    _ => (),
                }
            }
        }
    }
    
    // #[test]
    fn test_smol(){


        let filter = tracing_subscriber::filter::EnvFilter::new("none,mqrstt=trace");

        let subscriber = tracing_subscriber::FmtSubscriber::builder()
            .with_env_filter(filter)
            .with_max_level(tracing::Level::TRACE)
            .with_line_number(true)
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");

        smol::block_on(async{
            let options = ConnectOptions::new("broker.emqx.io".to_string(), 8883, "mqrstt".to_string());
            
            let tls_config = RustlsConfig::Simple {
                ca: crate::tests::resources::EMQX_CERT.to_vec(),
                alpn: None,
                client_auth: None,
            };
    
            let (mut network, mut handler, client) = create_smol_rustls(options, tls_config);
    
            client.subscribe("mqrstt").await.unwrap();
            
    
            let mut pingpong = PingPong{ client };

            join!(network.run(), 
                async {
                    loop{
                        handler.handle(&mut pingpong).await.unwrap();
                    }
                }
            ).0.unwrap();
        });
    }
}