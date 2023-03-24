#[cfg(all(feature = "quic"))]
pub mod quic;
#[cfg(feature = "smol")]
pub mod smol;
#[cfg(feature = "tokio")]
pub mod tokio;
#[cfg(feature = "sync")]
pub mod sync;

use crate::connect_options::ConnectOptions;
use crate::packets::Connect;
use crate::packets::Packet;

pub fn create_connect_from_options(options: &ConnectOptions) -> Packet {
    let mut connect = Connect {
        client_id: options.client_id.clone(),
        clean_start: options.clean_start,
        keep_alive: options.keep_alive_interval_s as u16,
        username: options.username.clone(),
        password: options.password.clone(),
        ..Default::default()
    };

    connect.connect_properties.request_problem_information = options.request_problem_information;
    connect.connect_properties.request_response_information = options.request_response_information;

    Packet::Connect(connect)
}
