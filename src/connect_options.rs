use bytes::Bytes;

#[cfg(any(feature = "smol-rustls", feature = "tokio-rustls"))]
use crate::connections::transport::TlsConfig;
use crate::packets::LastWill;
use crate::util::constants::RECEIVE_MAXIMUM_DEFAULT;

#[derive(Debug, Clone)]
pub struct ConnectOptions {
	/// keep alive time to send pingreq to broker when the connection is idle
	pub keep_alive_interval_s: u64,
	pub connection_timeout_s: u64,
	/// clean (or) persistent session
	pub clean_session: bool,
	/// client identifier
	pub client_id: String,
	/// username and password
	pub username: Option<String>,
	pub password: Option<String>,
	/// request (publish, subscribe) channel capacity
	channel_capacity: usize,

	/// Minimum delay time between consecutive outgoing packets
	/// while retransmitting pending packets
	// TODO! IMPLEMENT THIS!
	pending_throttle_s: u64,

	send_reason_messages: bool,

	// MQTT v5 Connect Properties:
	session_expiry_interval: Option<u32>,
	pub(crate) receive_maximum: Option<u16>,
	maximum_packet_size: Option<u32>,
	topic_alias_maximum: Option<u16>,
	request_response_information: Option<u8>,
	request_problem_information: Option<u8>,
	user_properties: Vec<(String, String)>,
	authentication_method: Option<String>,
	authentication_data: Bytes,

	/// Last will that will be issued on unexpected disconnect
	last_will: Option<LastWill>,
}

impl ConnectOptions {
	pub fn new(client_id: String) -> Self {
		Self {
			keep_alive_interval_s: 60,
			connection_timeout_s: 30,
			clean_session: false,
			client_id,
			username: None,
			password: None,
			channel_capacity: 100,
			pending_throttle_s: 30,
			send_reason_messages: false,

			session_expiry_interval: None,
			receive_maximum: None,
			maximum_packet_size: None,
			topic_alias_maximum: None,
			request_response_information: None,
			request_problem_information: None,
			user_properties: vec![],
			authentication_method: None,
			authentication_data: Bytes::new(),
			last_will: None,
		}
	}

	pub fn set_keep_alive_interval_s(&mut self, keep_alive_interval_s: u64) {
		self.keep_alive_interval_s = keep_alive_interval_s
	}
	pub fn set_connection_timeout_s(&mut self, connection_timeout_s: u64) {
		self.connection_timeout_s = connection_timeout_s
	}
	pub fn set_clean_session(&mut self, clean_session: bool) {
		self.clean_session = clean_session
	}
	pub fn set_client_id(&mut self, client_id: String) {
		self.client_id = client_id
	}
	pub fn set_channel_capacity(&mut self, channel_capacity: usize) {
		self.channel_capacity = channel_capacity
	}
	pub fn set_pending_throttle_s(&mut self, pending_throttle_s: u64) {
		self.pending_throttle_s = pending_throttle_s
	}
	pub fn set_send_reason_messages(&mut self, send_reason_messages: bool) {
		self.send_reason_messages = send_reason_messages
	}

	pub fn set_session_expiry_interval(&mut self, session_expiry_interval: u32) {
		self.session_expiry_interval = Some(session_expiry_interval)
	}
	pub fn clear_session_expiry_interval(&mut self) {
		self.session_expiry_interval = None
	}

	pub fn set_receive_maximum(&mut self, receive_maximum: u16) {
		self.receive_maximum = Some(receive_maximum)
	}
	pub fn clear_receive_maximum(&mut self) {
		self.receive_maximum = None
	}
	pub fn receive_maximum(&self) -> u16 {
		self.receive_maximum.unwrap_or(RECEIVE_MAXIMUM_DEFAULT)
	}

	pub fn set_maximum_packet_size(&mut self, maximum_packet_size: u32) {
		self.maximum_packet_size = Some(maximum_packet_size)
	}
	pub fn clear_maximum_packet_size(&mut self) {
		self.maximum_packet_size = None
	}
	pub fn set_topic_alias_maximum(&mut self, topic_alias_maximum: u16) {
		self.topic_alias_maximum = Some(topic_alias_maximum)
	}
	pub fn clear_topic_alias_maximum(&mut self) {
		self.topic_alias_maximum = None
	}
	pub fn set_request_response_information(&mut self, request_response_information: bool) {
		self.request_response_information = Some(if request_response_information { 1 } else { 0 })
	}
	pub fn clear_request_response_information(&mut self) {
		self.request_response_information = None
	}
	pub fn set_request_problem_information(&mut self, request_problem_information: bool) {
		self.request_problem_information = Some(if request_problem_information { 1 } else { 0 })
	}
	pub fn clear_request_problem_information(&mut self) {
		self.request_problem_information = None
	}
	pub fn add_user_properties(&mut self, user_properties: &[(String, String)]) {
		self.user_properties.extend_from_slice(user_properties)
	}
	pub fn clear_user_properties(&mut self) {
		self.user_properties.clear()
	}
	pub fn set_authentication_method(&mut self, authentication_method: String) {
		self.authentication_method = Some(authentication_method)
	}
	pub fn clear_authentication_method(&mut self) {
		self.authentication_method = None
	}
	pub fn set_authentication_data(&mut self, authentication_data: Bytes) {
		self.authentication_data = authentication_data
	}
	pub fn clear_authentication_data(&mut self) {
		self.authentication_data.clear()
	}
	pub fn set_last_will(&mut self, last_will: LastWill) {
		self.last_will = Some(last_will)
	}
	pub fn clear_last_will(&mut self) {
		self.last_will = None
	}
}
