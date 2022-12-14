// pub mod tcp;
pub mod tcp_tokio;

use std::future::{Future};

use async_channel::{Sender};
use bytes::BytesMut;

use crate::error::ConnectionError;
use crate::packets::connect::Connect;
use crate::{packets::packets::Packet};
use crate::connect_options::ConnectOptions;


pub fn create_connect_from_options(options: &ConnectOptions) -> Packet{
    let mut connect = Connect::default();

    connect.client_id = options.client_id.clone();
    connect.clean_session = options.clean_session;
    connect.keep_alive = options.keep_alive_interval_s as u16;
    connect.connect_properties.request_problem_information = Some(1u8);
    connect.connect_properties.request_response_information = Some(1u8);

    Packet::Connect(connect)

}


pub trait AsyncMqttNetwork: Sized + Sync + 'static{
    fn connect(options: &ConnectOptions) -> impl Future<Output = Result<(Self, Packet), ConnectionError>> + Send + '_;
    
    async fn read(&self) -> Result<Packet, ConnectionError>;

    async fn read_many(&self, receiver: &Sender<Packet>) -> Result<(), ConnectionError>;

    async fn write(&self, write_buf: &mut BytesMut) -> Result<(), ConnectionError>;
}