#[cfg(all(feature = "tokio", feature = "tcp"))]
pub mod tcp;

#[cfg(all(feature = "smol", feature = "tcp"))]
pub mod tcp_smol;

#[cfg(all(feature = "smol", feature = "native-tls"))]
pub mod async_native_tls;

#[cfg(all(feature = "smol", feature = "smol-rustls"))]
pub mod async_rustls;

#[cfg(all(feature = "quic"))]
pub mod quic;

#[cfg(all(feature = "tokio", feature = "tokio-rustls"))]
pub mod tokio_rustls;

pub mod transport;
#[cfg(any(feature = "smol-rustls", feature = "tokio-rustls"))]
mod util;

use std::future::Future;

use async_channel::{Receiver, Sender};
use bytes::BytesMut;

use crate::connect_options::ConnectOptions;
use crate::error::ConnectionError;
use crate::packets::Connect;
use crate::packets::Packet;

pub fn create_connect_from_options(options: &ConnectOptions) -> Packet {
    let mut connect = Connect::default();

    connect.client_id = options.client_id.clone();
    connect.clean_session = options.clean_session;
    connect.keep_alive = options.keep_alive_interval_s as u16;
    connect.connect_properties.request_problem_information = Some(1u8);
    connect.connect_properties.request_response_information = Some(1u8);

    Packet::Connect(connect)
}

pub trait AsyncMqttNetwork: Sized + Sync + 'static {
    fn connect(
        options: &ConnectOptions,
    ) -> impl Future<Output = Result<(Self, Packet), ConnectionError>> + Send + '_;

    async fn read(&self) -> Result<Packet, ConnectionError>;

    async fn read_many(&self, receiver: &Sender<Packet>) -> Result<(), ConnectionError>;

    async fn write(&self, write_buf: &mut BytesMut) -> Result<(), ConnectionError>;
}

pub trait AsyncMqttNetworkRead: Sized + Sync {
    type W;

    fn connect(
        options: &ConnectOptions,
    ) -> impl Future<Output = Result<(Self, Self::W, Packet), ConnectionError>> + Send + '_;

    async fn read(&mut self) -> Result<Packet, ConnectionError>;

    async fn read_direct(&mut self, sender: &Sender<Packet>) -> Result<bool, ConnectionError>;
}

pub trait AsyncMqttNetworkWrite: Sized + Sync {
    async fn write_buffer(&mut self, buffer: &mut BytesMut) -> Result<(), ConnectionError>;

    async fn write(&mut self, outgoing: &Receiver<Packet>) -> Result<bool, ConnectionError>;
}
