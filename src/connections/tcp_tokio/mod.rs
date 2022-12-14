pub mod tcp_writer;
pub mod tcp_reader;

use std::future::{Future};

use async_channel::{Sender, Receiver};
use bytes::BytesMut;

use crate::error::ConnectionError;
use crate::{packets::packets::Packet};
use crate::connect_options::ConnectOptions;

pub trait AsyncMqttNetworkRead: Sized + Sync{
    type W;

    fn connect(options: &ConnectOptions) -> impl Future<Output = Result<(Self, Self::W, Packet), ConnectionError>> + Send  + '_;
    
    async fn read(&mut self) -> Result<Packet, ConnectionError>;

    async fn read_many(&mut self, sender: &Sender<Packet>) -> Result<(), ConnectionError>;
}

pub trait AsyncMqttNetworkWrite: Sized + Sync{
    async fn write_buffer(&mut self, buffer: &mut BytesMut) -> Result<(), ConnectionError>;

    async fn write(&mut self, outgoing: &Receiver<Packet>) -> Result<(), ConnectionError>;
}