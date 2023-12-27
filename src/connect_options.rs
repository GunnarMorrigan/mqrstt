use std::time::Duration;

use bytes::Bytes;

use crate::{packets::{LastWill, ConnectProperties}, util::constants::DEFAULT_MAX_PACKET_SIZE};
#[cfg(any(feature = "smol-rustls", feature = "tokio-rustls"))]
use crate::stream::transport::TlsConfig;
use crate::util::constants::DEFAULT_RECEIVE_MAXIMUM;

#[derive(Debug, Clone)]
pub struct ConnectOptions {
    /// keep alive time to send pingreq to broker when the connection is idle
    pub(crate) keep_alive_interval: Duration,
    /// clean or persistent session indicator
    pub(crate) clean_start: bool,
    /// client identifier
    client_id: String,
    /// username and password
    username: Option<String>,
    password: Option<String>,

    // MQTT v5 Connect Properties:
    session_expiry_interval: Option<u32>,
    
    /// The maximum number of packets that will be inflight from the broker to this client.
    receive_maximum: Option<u16>,

    /// The maximum number of packets that can be inflight from this client to the broker.
    send_maximum: Option<u16>,

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
    pub fn new<S: AsRef<str>>(client_id: S, clean_start: bool) -> Self {
        Self {
            keep_alive_interval: Duration::from_secs(60),
            clean_start: clean_start,
            client_id: client_id.as_ref().to_string(),
            username: None,
            password: None,

            session_expiry_interval: None,
            receive_maximum: None,
            send_maximum: None,
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

    pub(crate) fn create_connect_from_options(&self) -> crate::packets::Packet {
        let connect_properties = ConnectProperties {
            session_expiry_interval: self.session_expiry_interval,
            receive_maximum: self.receive_maximum,
            maximum_packet_size: self.maximum_packet_size,
            topic_alias_maximum: self.topic_alias_maximum,
            request_response_information: self.request_response_information,
            request_problem_information: self.request_response_information,
            user_properties: self.user_properties.clone(),
            authentication_method: self.authentication_method.clone(),
            authentication_data: self.authentication_data.clone(),
        };
    
        let connect = crate::packets::Connect {
            client_id: self.client_id.clone(),
            clean_start: self.clean_start,
            keep_alive: self.keep_alive_interval.as_secs() as u16,
            username: self.username.clone(),
            password: self.password.clone(),
            connect_properties,
            protocol_version: crate::packets::ProtocolVersion::V5,
            last_will: self.last_will.clone(),
        };
    
        crate::packets::Packet::Connect(connect)
    }

    pub fn set_keep_alive_interval(&mut self, keep_alive_interval: Duration) {
        self.keep_alive_interval = keep_alive_interval;
    }
    pub fn get_keep_alive_interval(&self) -> Duration {
        self.keep_alive_interval
    }
   
    pub fn get_clean_start(&self) -> bool {
        self.clean_start
    }

    pub fn get_client_id(&self) -> &str {
        &self.client_id
    }

    pub fn set_last_will(&mut self, last_will: LastWill) {
        self.last_will = Some(last_will);
    }
    pub fn get_last_will(&self) -> Option<&LastWill> {
        self.last_will.as_ref()
    }

    pub fn set_receive_maximum(&mut self, receive_maximum: u16) {
        self.receive_maximum = Some(receive_maximum)
    }
    pub fn receive_maximum(&self) -> u16 {
        self.receive_maximum.unwrap_or(DEFAULT_RECEIVE_MAXIMUM)
    }

    pub fn maximum_packet_size(&self) -> usize {
        self.maximum_packet_size.unwrap_or(DEFAULT_MAX_PACKET_SIZE) as usize
    }

    pub fn set_send_maximum(&mut self, send_maximum: u16) {
        self.send_maximum = Some(send_maximum)
    }
    pub fn send_maximum(&self) -> u16 {
        self.send_maximum.unwrap_or(DEFAULT_RECEIVE_MAXIMUM)
    }
}
