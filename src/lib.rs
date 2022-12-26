#![feature(async_fn_in_trait)]
#![feature(return_position_impl_trait_in_trait)]

use std::{sync::Arc, time::Instant};

use async_channel::Receiver;
use async_mutex::Mutex;
use client::AsyncClient;
use connect_options::ConnectOptions;
use connections::{AsyncMqttNetworkWrite, AsyncMqttNetworkRead};
#[cfg(feature = "tokio")]
use connections::tcp_tokio::{tcp_reader::TcpReader, tcp_writer::TcpWriter};
#[cfg(all(feature = "smol", feature = "rust-tls"))]
use connections::async_rustls::{TlsReader, TlsWriter};


use event_handler::{EventHandlerTask};
use network::MqttNetwork;
use packets::packets::Packet;

mod available_packet_ids;
pub mod client;
pub mod connect_options;
mod connections;
pub mod error;
pub mod event_handler;
mod network;
mod packets;
mod state;
mod util;

mod tests;

#[cfg(all(feature = "smol", feature = "rust-tls"))]
pub fn create_smol_tls(options: ConnectOptions) -> (
    MqttNetwork<TlsReader, TlsWriter>,
    EventHandlerTask,
    AsyncClient,
){
    new(options)
}

// #[cfg(all(feature = "tokio", feature = "tcp"))]
pub fn create_tokio_tcp(options: ConnectOptions) -> (
    MqttNetwork<TcpReader, TcpWriter>,
    EventHandlerTask,
    AsyncClient,
){
    new(options)
}


pub fn new<R, W>(
    options: ConnectOptions,
) -> (
    MqttNetwork<R, W>,
    EventHandlerTask,
    AsyncClient
)
where
    R: AsyncMqttNetworkRead<W = W>,
    W: AsyncMqttNetworkWrite{
    
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