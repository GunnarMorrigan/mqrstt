#[cfg(all(feature = "quic"))]
pub mod quic;
#[cfg(feature = "smol")]
pub mod smol_stream;
#[cfg(feature = "tokio")]
pub mod tokio_stream;

use crate::connect_options::ConnectOptions;
use crate::packets::Connect;
use crate::packets::Packet;

pub fn create_connect_from_options(options: &ConnectOptions) -> Packet {
	let mut connect = Connect::default();

	connect.client_id = options.client_id.clone();
	connect.clean_session = options.clean_session;
	connect.keep_alive = options.keep_alive_interval_s as u16;
	connect.connect_properties.request_problem_information = Some(1u8);
	connect.connect_properties.request_response_information = Some(1u8);
	connect.username = options.username.clone();
	connect.password = options.password.clone();

	Packet::Connect(connect)
}
