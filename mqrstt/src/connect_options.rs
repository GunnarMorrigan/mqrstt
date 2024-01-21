use std::time::Duration;

use bytes::Bytes;

use crate::util::constants::DEFAULT_RECEIVE_MAXIMUM;
use crate::{
    packets::{ConnectProperties, LastWill},
    util::constants::MAXIMUM_PACKET_SIZE,
};

#[derive(Debug, thiserror::Error)]
pub enum ConnectOptionsError {
    #[error("Maximum packet size is exceeded. Maximum is {MAXIMUM_PACKET_SIZE}, was provided {0}")]
    MaximumPacketSize(u32),
}

#[derive(Debug, Clone)]
pub struct ConnectOptions {
    /// client identifier
    client_id: Box<str>,
    /// keep alive time to send pingreq to broker when the connection is idle
    pub(crate) keep_alive_interval: Duration,
    /// clean or persistent session indicator
    pub(crate) clean_start: bool,
    /// username and password
    username: Option<Box<str>>,
    password: Option<Box<str>>,

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
    user_properties: Vec<(Box<str>, Box<str>)>,
    authentication_method: Option<Box<str>>,
    authentication_data: Bytes,

    /// Last will that will be issued on unexpected disconnect
    last_will: Option<LastWill>,
}

impl Default for ConnectOptions {
    fn default() -> Self {
        Self {
            keep_alive_interval: Duration::from_secs(60),
            clean_start: true,
            client_id: Box::from("ChangeClientId_MQRSTT"),
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
}

impl ConnectOptions {
    /// Create a new [`ConnectOptions`]
    /// ClientId recommendation:
    ///     - 1 to 23 bytes UTF-8 bytes
    ///     - Contains [a-zA-Z0-9] characters only.
    ///
    /// Some brokers accept longer client ids with different characters
    pub fn new<S: AsRef<str>>(client_id: S) -> Self {
        Self {
            keep_alive_interval: Duration::from_secs(60),
            clean_start: true,
            client_id: client_id.as_ref().into(),
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
            request_problem_information: self.request_problem_information,
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

    /// The Client Identifier (ClientID) identifies the Client to the Server. Each Client connecting to the Server has a unique ClientID.
    /// The ClientID MUST be used by Clients and by Servers to identify state that they hold relating to this MQTT Session between the Client and the Server [MQTT-3.1.3-2].
    /// More info here: <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901059>
    ///
    /// Non unique client ids often result in connect, disconnect loops due to auto reconnects and forced disconnects
    pub fn get_client_id(&self) -> &str {
        &self.client_id
    }
    /// The Client Identifier (ClientID) identifies the Client to the Server. Each Client connecting to the Server has a unique ClientID.
    /// The ClientID MUST be used by Clients and by Servers to identify state that they hold relating to this MQTT Session between the Client and the Server [MQTT-3.1.3-2].
    /// More info here: <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901059>
    ///
    /// Non unique client ids often result in connect, disconnect loops due to auto reconnects and forced disconnects
    pub fn set_client_id<S: AsRef<str>>(&mut self, client_id: S) -> &mut Self {
        self.client_id = client_id.as_ref().into();
        self
    }

    /// This specifies whether the Connection starts a new Session or is a continuation of an existing Session.
    /// More info here: <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901039>
    pub fn get_clean_start(&self) -> bool {
        self.clean_start
    }
    /// This specifies whether the Connection starts a new Session or is a continuation of an existing Session.
    /// More info here: <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901039>
    pub fn set_clean_start(&mut self, clean_start: bool) -> &mut Self {
        self.clean_start = clean_start;
        self
    }

    pub fn get_username(&self) -> Option<&str> {
        self.username.as_ref().map(Box::<str>::as_ref)
    }
    pub fn set_username<S: AsRef<str>>(&mut self, username: S) -> &mut Self {
        self.username = Some(username.as_ref().into());
        self
    }
    pub fn get_password(&self) -> Option<&str> {
        self.password.as_ref().map(Box::<str>::as_ref)
    }
    pub fn set_password<S: AsRef<str>>(&mut self, password: S) -> &mut Self {
        self.password = Some(password.as_ref().into());
        self
    }

    /// Get the Session Expiry Interval in seconds.
    /// If the Session Expiry Interval is absent the value 0 is used. If it is set to 0, or is absent, the Session ends when the Network Connection is closed.
    /// If the Session Expiry Interval is 0xFFFFFFFF (UINT_MAX), the Session does not expire.
    /// More info here: <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901211>
    pub fn get_session_expiry_interval(&self) -> Option<u32> {
        self.session_expiry_interval
    }
    /// Set the Session Expiry Interval in seconds.
    /// If the Session Expiry Interval is absent the value 0 is used. If it is set to 0, or is absent, the Session ends when the Network Connection is closed.
    /// If the Session Expiry Interval is 0xFFFFFFFF (UINT_MAX), the Session does not expire.
    /// More info here: <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901211>
    pub fn set_session_expiry_interval(&mut self, session_expiry_interval: u32) -> &mut Self {
        self.session_expiry_interval = Some(session_expiry_interval);
        self
    }

    /// Get the current keep alive interval for the MQTT protocol
    /// This time is used in Ping Pong when natural traffic is absent
    /// The granularity is in seconds!
    pub fn get_keep_alive_interval(&self) -> Duration {
        self.keep_alive_interval
    }
    /// Set the keep alive interval for the MQTT protocol
    /// This time is used in Ping Pong when natural traffic is absent
    /// The granularity is in seconds!
    pub fn set_keep_alive_interval(&mut self, keep_alive_interval: Duration) -> &mut Self {
        self.keep_alive_interval = Duration::from_secs(keep_alive_interval.as_secs());
        self
    }

    pub fn set_last_will(&mut self, last_will: LastWill) -> &mut Self {
        self.last_will = Some(last_will);
        self
    }
    pub fn get_last_will(&self) -> Option<&LastWill> {
        self.last_will.as_ref()
    }

    pub fn set_receive_maximum(&mut self, receive_maximum: u16) -> &mut Self {
        self.receive_maximum = Some(receive_maximum);
        self
    }
    pub fn receive_maximum(&self) -> u16 {
        self.receive_maximum.unwrap_or(DEFAULT_RECEIVE_MAXIMUM)
    }

    pub fn set_maximum_packet_size(&mut self, maximum_packet_size: u32) -> Result<&mut Self, ConnectOptionsError> {
        if maximum_packet_size > MAXIMUM_PACKET_SIZE {
            Err(ConnectOptionsError::MaximumPacketSize(maximum_packet_size))
        } else {
            self.maximum_packet_size = Some(maximum_packet_size);
            Ok(self)
        }
    }
    pub fn maximum_packet_size(&self) -> usize {
        self.maximum_packet_size.unwrap_or(MAXIMUM_PACKET_SIZE) as usize
    }

    pub fn set_send_maximum(&mut self, send_maximum: u16) -> &mut Self {
        self.send_maximum = Some(send_maximum);
        self
    }
    pub fn send_maximum(&self) -> u16 {
        self.send_maximum.unwrap_or(DEFAULT_RECEIVE_MAXIMUM)
    }
}
