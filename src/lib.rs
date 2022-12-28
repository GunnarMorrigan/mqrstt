#![feature(async_fn_in_trait)]
#![feature(return_position_impl_trait_in_trait)]

use std::{sync::Arc, time::Instant};

use async_mutex::Mutex;
use client::AsyncClient;
use connect_options::ConnectOptions;

#[cfg(all(feature = "smol", feature = "rust-tls"))]
use connections::async_rustls::{TlsReader, TlsWriter};
#[cfg(all(feature = "tokio", feature = "tcp"))]
use connections::tcp::{TcpReader, TcpWriter};

use connections::{AsyncMqttNetworkRead, AsyncMqttNetworkWrite};

use connections::transport::RustlsConfig;

use event_handler::EventHandlerTask;
use network::MqttNetwork;

mod available_packet_ids;
pub mod client;
pub mod connect_options;
mod connections;
pub mod error;
pub mod event_handler;
mod network;
pub mod packets;
mod state;
mod util;

#[cfg(test)]
mod tests;

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
    MqttNetwork<TcpReader, TcpWriter>,
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
