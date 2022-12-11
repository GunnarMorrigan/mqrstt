use std::future::{Future};

use async_channel::{Sender};
use bytes::BytesMut;

use crate::error::ConnectionError;
use crate::{packets::packets::Packet};
use crate::connect_options::ConnectOptions;

pub mod tcp;

pub trait AsyncMqttNetwork: Sized + Sync + 'static{
    fn connect(options: &ConnectOptions) -> impl Future<Output = Result<(Self, Packet), ConnectionError>> + Send + '_;
    
    async fn read(&mut self) -> Result<Packet, ConnectionError>;

    async fn read_many(&mut self, receiver: &mut Sender<Packet>) -> Result<(), ConnectionError>;

    async fn write(&mut self, write_buf: &mut BytesMut) -> Result<(), ConnectionError>;
}

