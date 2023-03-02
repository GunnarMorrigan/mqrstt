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
    pub clean_start: bool,
    /// client identifier
    pub client_id: String,
    /// username and password
    pub username: Option<String>,
    pub password: Option<String>,
    /// request (publish, subscribe) channel capacity
    pub channel_capacity: usize,

    /// Minimum delay time between consecutive outgoing packets
    /// while retransmitting pending packets
    // TODO! IMPLEMENT THIS!
    pub pending_throttle_s: u64,

    pub send_reason_messages: bool,

    // MQTT v5 Connect Properties:
    pub session_expiry_interval: Option<u32>,
    pub receive_maximum: Option<u16>,
    pub maximum_packet_size: Option<u32>,
    pub topic_alias_maximum: Option<u16>,
    pub request_response_information: Option<u8>,
    pub request_problem_information: Option<u8>,
    pub user_properties: Vec<(String, String)>,
    pub authentication_method: Option<String>,
    pub authentication_data: Bytes,

    /// Last will that will be issued on unexpected disconnect
    pub last_will: Option<LastWill>,
}

impl ConnectOptions {
    pub fn new(client_id: String) -> Self {
        Self {
            keep_alive_interval_s: 60,
            connection_timeout_s: 30,
            clean_start: false,
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

    pub fn receive_maximum(&self) -> u16 {
        self.receive_maximum.unwrap_or(RECEIVE_MAXIMUM_DEFAULT)
    }
}
