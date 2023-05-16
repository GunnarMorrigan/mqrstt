use async_channel::{Receiver, Sender};
use bytes::Bytes;

#[cfg(feature = "logs")]
use tracing::info;

use crate::{
    error::ClientError,
    packets::{
        mqtt_traits::PacketValidation,
        reason_codes::DisconnectReasonCode,
        Packet, QoS, {Disconnect, DisconnectProperties}, {Publish, PublishProperties}, {Subscribe, SubscribeProperties, Subscription}, {Unsubscribe, UnsubscribeProperties, UnsubscribeTopics},
    },
    util::constants::DEFAULT_MAX_PACKET_SIZE,
};

#[derive(Debug, Clone)]
pub struct MqttClient {
    /// Provides this client with an available packet id or waits on it.
    available_packet_ids: Receiver<u16>,

    /// Sends Publish, Subscribe, Unsubscribe to the event handler to handle later.
    to_network_s: Sender<Packet>,

    /// MQTT packet size limit
    max_packet_size: usize,
}

impl MqttClient {
    pub fn new(available_packet_ids: Receiver<u16>, to_network_s: Sender<Packet>, max_packet_size: Option<u32>) -> Self {
        Self {
            available_packet_ids,
            to_network_s,
            max_packet_size: max_packet_size.unwrap_or(DEFAULT_MAX_PACKET_SIZE) as usize,
        }
    }
}

/// Async functions to perform MQTT operations
#[cfg(any(feature = "tokio", feature = "smol", feature = "quic"))]
impl MqttClient {
    /// Creates a subscribe packet that is then asynchronously transferred to the Network stack for transmission
    ///
    /// Can be called with anything that can be converted into [`Subscription`]
    ///
    /// # Examples
    /// ```
    /// # let options = mqrstt::ConnectOptions::new("example".to_string());
    /// # let (network, mqtt_client) = mqrstt::new_smol::<smol::net::TcpStream>(options);
    /// # smol::block_on(async {
    ///
    /// use mqrstt::packets::QoS;
    /// use mqrstt::packets::{SubscriptionOptions, RetainHandling};
    ///
    /// // retain_handling: RetainHandling::ZERO, retain_as_publish: false, no_local: false, qos: QoS::AtMostOnce,
    /// mqtt_client.subscribe("test/topic").await;
    ///
    /// // retain_handling: RetainHandling::ZERO, retain_as_publish: false, no_local: false, qos: QoS::ExactlyOnce,
    /// mqtt_client.subscribe(("test/topic", QoS::ExactlyOnce)).await;
    ///
    /// let vec = vec![("test/topic1", QoS::ExactlyOnce), ("test/topic2", QoS::AtMostOnce)];
    /// mqtt_client.subscribe(vec).await;
    ///
    /// let vec = [("test/topic1", QoS::ExactlyOnce), ("test/topic2", QoS::AtLeastOnce)];
    /// mqtt_client.subscribe(vec.as_slice()).await;
    ///
    /// let sub_options = SubscriptionOptions{
    ///   retain_handling: RetainHandling::TWO,
    ///   retain_as_publish: false,
    ///   no_local: false,
    ///   qos: QoS::AtLeastOnce,
    /// };
    /// mqtt_client.subscribe(("final/test/topic", sub_options)).await;
    /// # });
    /// ```
    pub async fn subscribe<A: Into<Subscription>>(&self, into_subscribtions: A) -> Result<(), ClientError> {
        let pkid = self.available_packet_ids.recv().await.map_err(|_| ClientError::NoNetworkChannel)?;
        let subscription: Subscription = into_subscribtions.into();
        let sub = Subscribe::new(pkid, subscription.0);

        sub.validate(self.max_packet_size)?;
        self.to_network_s.send(Packet::Subscribe(sub)).await.map_err(|_| ClientError::NoNetworkChannel)?;
        Ok(())
    }

    /// Creates a subscribe packet with additional subscribe packet properties.
    /// The packet is then asynchronously transferred to the Network stack for transmission.
    ///
    /// Can be called with anything that can be converted into [`Subscription`]
    ///
    /// # Examples
    /// ```
    /// # let options = mqrstt::ConnectOptions::new("example".to_string());
    /// # let (network, mqtt_client) = mqrstt::new_smol::<smol::net::TcpStream>(options);
    /// # smol::block_on(async {
    ///
    /// use mqrstt::packets::QoS;
    /// use mqrstt::packets::{SubscribeProperties, SubscriptionOptions, RetainHandling};
    ///
    /// let sub_properties = SubscribeProperties{
    ///   subscription_id: Some(1),
    ///   user_properties: vec![],
    /// };
    ///
    /// // retain_handling: RetainHandling::ZERO, retain_as_publish: false, no_local: false, qos: QoS::AtMostOnce,
    /// mqtt_client.subscribe_with_properties("test/topic", sub_properties).await;
    ///
    /// # let sub_properties = SubscribeProperties{
    /// #   subscription_id: Some(1),
    /// #   user_properties: vec![],
    /// # };
    ///
    /// // retain_handling: RetainHandling::ZERO, retain_as_publish: false, no_local: false, qos: QoS::ExactlyOnce,
    /// mqtt_client.subscribe_with_properties(("test/topic", QoS::ExactlyOnce), sub_properties).await;
    ///
    /// # let sub_properties = SubscribeProperties{
    /// #   subscription_id: Some(1),
    /// #   user_properties: vec![],
    /// # };
    ///
    /// let vec = vec![("test/topic1", QoS::ExactlyOnce), ("test/topic2", QoS::ExactlyOnce)];
    /// mqtt_client.subscribe_with_properties(vec, sub_properties).await;
    ///
    /// # let sub_properties = SubscribeProperties{
    /// #   subscription_id: Some(1),
    /// #   user_properties: vec![],
    /// # };
    ///  
    /// let vec = [("test/topic1", QoS::ExactlyOnce), ("test/topic2", QoS::ExactlyOnce)];
    /// mqtt_client.subscribe_with_properties(vec.as_slice(), sub_properties).await;
    ///
    /// # let sub_properties = SubscribeProperties{
    /// #   subscription_id: Some(1),
    /// #   user_properties: vec![],
    /// # };
    ///  
    /// let sub_options = SubscriptionOptions{
    ///   retain_handling: RetainHandling::ZERO,
    ///   retain_as_publish: false,
    ///   no_local: false,
    ///   qos: QoS::AtLeastOnce,
    /// };
    /// mqtt_client.subscribe_with_properties(("final/test/topic", sub_options), sub_properties).await;
    /// # });
    /// ```
    pub async fn subscribe_with_properties<S: Into<Subscription>>(&self, into_sub: S, properties: SubscribeProperties) -> Result<(), ClientError> {
        let pkid = self.available_packet_ids.recv().await.map_err(|_| ClientError::NoNetworkChannel)?;
        let sub = Subscribe {
            packet_identifier: pkid,
            properties,
            topics: into_sub.into().0,
        };
        sub.validate(self.max_packet_size)?;
        self.to_network_s.send(Packet::Subscribe(sub)).await.map_err(|_| ClientError::NoNetworkChannel)?;
        Ok(())
    }

    /// Creates a Publish packet which is then asynchronously transferred to the Network stack for transmission.
    ///
    /// Can be called with any payload that can be converted into [`Bytes`]
    ///
    /// # Examples
    /// ```
    /// # let options = mqrstt::ConnectOptions::new("example".to_string());
    /// # let (network, mqtt_client) = mqrstt::new_smol::<smol::net::TcpStream>(options);
    /// # smol::block_on(async {
    ///
    /// use mqrstt::packets::QoS;
    /// use bytes::Bytes;
    ///
    /// // publish a message with QoS 0, without a packet identifier
    /// mqtt_client.publish("test/topic", QoS::AtMostOnce, false, "Hello world").await;
    ///
    /// // publish a message with QoS 1, with a packet identifier
    /// mqtt_client.publish("test/topic", QoS::AtLeastOnce, false, Bytes::from("Hello world")).await;
    ///
    /// // publish a message with QoS 2, with a packet identifier
    /// mqtt_client.publish("test/topic", QoS::ExactlyOnce, false, Bytes::from("Hello world")).await;
    ///
    /// // publish a message with QoS 1, with a packet identifier, and the "retain" flag set
    /// let payload: &[u8] = "Hello World!".as_bytes();
    /// mqtt_client.publish("test/topic", QoS::AtLeastOnce, true, payload).await;
    ///
    /// // publish a message with QoS 1, with a packet identifier, and the "retain" flag set
    /// let payload: Vec<u8> = "Hello World!".as_bytes().to_vec();
    /// mqtt_client.publish("test/topic", QoS::AtMostOnce, true, payload).await;
    ///
    /// # });
    /// ```
    pub async fn publish<T: Into<String>, P: Into<Bytes>>(&self, topic: T, qos: QoS, retain: bool, payload: P) -> Result<(), ClientError> {
        let pkid = match qos {
            QoS::AtMostOnce => None,
            _ => Some(self.available_packet_ids.recv().await.map_err(|_| ClientError::NoNetworkChannel)?),
        };
        #[cfg(feature = "logs")]
        info!("Published message with ID: {:?}", pkid);
        let publish = Publish {
            dup: false,
            qos,
            retain,
            topic: topic.into(),
            packet_identifier: pkid,
            publish_properties: PublishProperties::default(),
            payload: payload.into(),
        };

        publish.validate(self.max_packet_size)?;
        self.to_network_s.send(Packet::Publish(publish)).await.map_err(|_| ClientError::NoNetworkChannel)?;
        Ok(())
    }

    /// Creates a Publish packet with additional publish properties.
    /// The packet is then asynchronously transferred to the Network stack for transmission.
    ///
    /// Can be called with any payload that can be converted into [`Bytes`]
    ///
    /// # Examples
    /// ```
    /// # let options = mqrstt::ConnectOptions::new("example".to_string());
    /// # let (network, mqtt_client) = mqrstt::new_smol::<smol::net::TcpStream>(options);
    /// # smol::block_on(async {
    ///
    /// use mqrstt::packets::QoS;
    /// use mqrstt::packets::PublishProperties;
    /// use bytes::Bytes;
    ///
    /// let properties = PublishProperties{
    ///     response_topic: Some("response/topic".to_string()),
    ///     correlation_data: Some("correlation_data".into()),
    ///     ..Default::default()
    /// };
    ///
    /// // publish a message with QoS 0, without a packet identifier
    /// mqtt_client.publish_with_properties("test/topic", QoS::AtMostOnce, false, Bytes::from("Hello world"), properties).await;
    ///
    /// # let properties = PublishProperties{
    /// #     response_topic: Some("response/topic".to_string()),
    /// #     correlation_data: Some("correlation_data".into()),
    /// #     ..Default::default()
    /// # };
    ///
    /// // publish a message with QoS 1, with a packet identifier
    /// mqtt_client.publish_with_properties("test/topic", QoS::AtLeastOnce, false, Bytes::from("Hello world"), properties).await;
    ///
    /// # let properties = PublishProperties{
    /// #     response_topic: Some("response/topic".to_string()),
    /// #     correlation_data: Some("correlation_data".into()),
    /// #     ..Default::default()
    /// # };
    ///
    /// // publish a message with QoS 2, with a packet identifier
    /// mqtt_client.publish_with_properties("test/topic", QoS::ExactlyOnce, false, Bytes::from("Hello world"), properties).await;
    ///
    /// # let properties = PublishProperties{
    /// #     response_topic: Some("response/topic".to_string()),
    /// #     correlation_data: Some("correlation_data".into()),
    /// #     ..Default::default()
    /// # };
    ///
    /// // publish a message with QoS 1, with a packet identifier, and the "retain" flag set
    /// let payload = "Hello World!".as_bytes();
    /// mqtt_client.publish_with_properties("test/topic", QoS::AtLeastOnce, true, payload, properties).await;
    ///
    /// # let properties = PublishProperties{
    /// #     response_topic: Some("response/topic".to_string()),
    /// #     correlation_data: Some("correlation_data".into()),
    /// #     ..Default::default()
    /// # };
    ///
    /// // publish a message with QoS 1, with a packet identifier, and the "retain" flag set
    /// let payload = "Hello World!".as_bytes().to_vec();
    /// mqtt_client.publish_with_properties("test/topic", QoS::AtMostOnce, true, payload, properties).await;
    ///
    /// # });
    /// ```
    pub async fn publish_with_properties<T: Into<String>, P: Into<Bytes>>(&self, topic: T, qos: QoS, retain: bool, payload: P, properties: PublishProperties) -> Result<(), ClientError> {
        let pkid = match qos {
            QoS::AtMostOnce => None,
            _ => Some(self.available_packet_ids.recv().await.map_err(|_| ClientError::NoNetworkChannel)?),
        };
        let publish = Publish {
            dup: false,
            qos,
            retain,
            topic: topic.into(),
            packet_identifier: pkid,
            publish_properties: properties,
            payload: payload.into(),
        };

        publish.validate(self.max_packet_size)?;
        self.to_network_s.send(Packet::Publish(publish)).await.map_err(|_| ClientError::NoNetworkChannel)?;
        Ok(())
    }

    /// Creates a Unsubscribe packet which is then asynchronously transferred to the Network stack for transmission.
    ///
    /// Can be called with anything that can be converted into [`UnsubscribeTopics`]
    ///
    /// # Examples
    ///
    /// ```
    /// # let options = mqrstt::ConnectOptions::new("example".to_string());
    /// # let (network, mqtt_client) = mqrstt::new_smol::<smol::net::TcpStream>(options);
    /// # smol::block_on(async {
    ///
    /// // Unsubscribe from a single topic specified as a string:
    /// let topic = "test/topic";
    /// mqtt_client.unsubscribe(topic).await;
    ///
    /// // Unsubscribe from multiple topics specified as an array of string slices:
    /// let topics = &["test/topic1", "test/topic2"];
    /// mqtt_client.unsubscribe(topics.as_slice()).await;
    ///
    /// // Unsubscribe from a single topic specified as a String:
    /// let topic = String::from("test/topic");
    /// mqtt_client.unsubscribe(topic).await;
    ///
    /// // Unsubscribe from multiple topics specified as a Vec<String>:
    /// let topics = vec![String::from("test/topic1"), String::from("test/topic2")];
    /// mqtt_client.unsubscribe(topics).await;
    ///
    /// // Unsubscribe from multiple topics specified as an array of String:
    /// let topics = &[String::from("test/topic1"), String::from("test/topic2")];
    /// mqtt_client.unsubscribe(topics.as_slice()).await;
    ///
    /// # });
    /// ```
    pub async fn unsubscribe<T: Into<UnsubscribeTopics>>(&self, into_topics: T) -> Result<(), ClientError> {
        let pkid = self.available_packet_ids.recv().await.map_err(|_| ClientError::NoNetworkChannel)?;
        let unsub = Unsubscribe {
            packet_identifier: pkid,
            properties: UnsubscribeProperties::default(),
            topics: into_topics.into().0,
        };

        unsub.validate(self.max_packet_size)?;
        self.to_network_s.send(Packet::Unsubscribe(unsub)).await.map_err(|_| ClientError::NoNetworkChannel)?;
        Ok(())
    }

    /// Creates a Unsubscribe packet with additional unsubscribe properties [`UnsubscribeProperties`].
    /// The packet is then asynchronously transferred to the Network stack for transmission.
    ///
    /// Can be called with anything that can be converted into [`UnsubscribeTopics`]
    ///
    /// # Examples
    ///
    /// ```
    /// # let options = mqrstt::ConnectOptions::new("example".to_string());
    /// # let (network, mqtt_client) = mqrstt::new_smol::<smol::net::TcpStream>(options);
    /// # smol::block_on(async {
    ///
    /// use mqrstt::packets::UnsubscribeProperties;
    ///
    /// let properties = UnsubscribeProperties{
    ///     user_properties: vec![("property".to_string(), "value".to_string())],
    /// };
    ///
    /// // Unsubscribe from a single topic specified as a string:
    /// let topic = "test/topic";
    /// mqtt_client.unsubscribe_with_properties(topic, properties).await;
    ///
    /// # let properties = UnsubscribeProperties{
    /// #     user_properties: vec![("property".to_string(), "value".to_string())],
    /// # };
    ///
    /// // Unsubscribe from multiple topics specified as an array of string slices:
    /// let topics = &["test/topic1", "test/topic2"];
    /// mqtt_client.unsubscribe_with_properties(topics.as_slice(), properties).await;
    ///
    /// # let properties = UnsubscribeProperties{
    /// #     user_properties: vec![("property".to_string(), "value".to_string())],
    /// # };
    ///  
    /// // Unsubscribe from a single topic specified as a String:
    /// let topic = String::from("test/topic");
    /// mqtt_client.unsubscribe_with_properties(topic, properties).await;
    ///
    /// # let properties = UnsubscribeProperties{
    /// #     user_properties: vec![("property".to_string(), "value".to_string())],
    /// # };
    ///  
    /// // Unsubscribe from multiple topics specified as a Vec<String>:
    /// let topics = vec![String::from("test/topic1"), String::from("test/topic2")];
    /// mqtt_client.unsubscribe_with_properties(topics, properties).await;
    ///
    /// # let properties = UnsubscribeProperties{
    /// #     user_properties: vec![("property".to_string(), "value".to_string())],
    /// # };
    ///  
    /// // Unsubscribe from multiple topics specified as an array of String:
    /// let topics = &[String::from("test/topic1"), String::from("test/topic2")];
    /// mqtt_client.unsubscribe_with_properties(topics.as_slice(), properties).await;
    ///
    /// # });
    /// ```
    pub async fn unsubscribe_with_properties<T: Into<UnsubscribeTopics>>(&self, into_topics: T, properties: UnsubscribeProperties) -> Result<(), ClientError> {
        let pkid = self.available_packet_ids.recv().await.map_err(|_| ClientError::NoNetworkChannel)?;
        let unsub = Unsubscribe {
            packet_identifier: pkid,
            properties,
            topics: into_topics.into().0,
        };

        unsub.validate(self.max_packet_size)?;
        self.to_network_s.send(Packet::Unsubscribe(unsub)).await.map_err(|_| ClientError::NoNetworkChannel)?;
        Ok(())
    }

    /// Creates a Disconnect packet which is then asynchronously transferred to the Network stack for transmission.
    ///
    /// This function awaits/yields until the packet is queued for transmission
    /// After a disconnect has been transmitted no other packets will be transmitted.
    /// Packets that remain in the channel to the network will be published in a later connection.
    ///
    /// # Example
    ///
    /// ```
    /// # let options = mqrstt::ConnectOptions::new("example".to_string());
    /// # let (network, mqtt_client) = mqrstt::new_smol::<smol::net::TcpStream>(options);
    /// # smol::block_on(async {
    ///
    /// mqtt_client.disconnect().await.unwrap();
    ///
    /// # });
    /// ```
    pub async fn disconnect(&self) -> Result<(), ClientError> {
        let disconnect = Disconnect {
            reason_code: DisconnectReasonCode::NormalDisconnection,
            properties: DisconnectProperties::default(),
        };
        self.to_network_s.send(Packet::Disconnect(disconnect)).await.map_err(|_| ClientError::NoNetworkChannel)?;
        Ok(())
    }

    /// Creates a Disconnect packet with additional [`DisconnectReasonCode`] and [`DisconnectProperties`].
    /// The packet is then asynchronously transferred to the Network stack for transmission.
    ///
    /// After a disconnect has been transmitted no other packets will be transmitted.
    /// Packets that remain in the channel to the network will be published in a later connection.
    ///
    /// # Example
    ///
    /// ```
    /// # let options = mqrstt::ConnectOptions::new("example".to_string());
    /// # let (network, mqtt_client) = mqrstt::new_smol::<smol::net::TcpStream>(options);
    /// # smol::block_on(async {
    ///
    /// use mqrstt::packets::DisconnectProperties;
    /// use mqrstt::packets::reason_codes::DisconnectReasonCode;
    ///
    /// let properties = DisconnectProperties {
    ///     reason_string: Some("Reason here".to_string()),
    ///     ..Default::default()
    /// };
    ///
    /// mqtt_client.disconnect_with_properties(DisconnectReasonCode::NormalDisconnection, properties).await.unwrap();
    ///
    /// # });
    /// ```
    pub async fn disconnect_with_properties(&self, reason_code: DisconnectReasonCode, properties: DisconnectProperties) -> Result<(), ClientError> {
        let disconnect = Disconnect { reason_code, properties };
        self.to_network_s.send(Packet::Disconnect(disconnect)).await.map_err(|_| ClientError::NoNetworkChannel)?;
        Ok(())
    }
}

/// Sync functions to perform MQTT operations
#[cfg(feature = "sync")]
impl MqttClient {
    /// Creates a subscribe packet that is then transferred to the Network stack for transmission
    ///
    /// Can be called with anything that can be converted into [`Subscription`]
    ///
    /// This function blocks until the packet is queued for transmission
    /// Creates a subscribe packet that is then asynchronously transferred to the Network stack for transmission
    ///
    /// Can be called with anything that can be converted into [`Subscription`]
    ///
    /// # Examples
    /// ```
    /// # let options = mqrstt::ConnectOptions::new("example".to_string());
    /// # let (network, mqtt_client) = mqrstt::new_smol::<smol::net::TcpStream>(options);
    /// # smol::block_on(async {
    ///
    /// use mqrstt::packets::QoS;
    /// use mqrstt::packets::{SubscriptionOptions, RetainHandling};
    ///
    /// // retain_handling: RetainHandling::ZERO, retain_as_publish: false, no_local: false, qos: QoS::AtMostOnce,
    /// mqtt_client.subscribe_blocking("test/topic");
    ///
    /// // retain_handling: RetainHandling::ZERO, retain_as_publish: false, no_local: false, qos: QoS::ExactlyOnce,
    /// mqtt_client.subscribe_blocking(("test/topic", QoS::ExactlyOnce));
    ///
    /// let vec = vec![("test/topic1", QoS::ExactlyOnce), ("test/topic2", QoS::AtMostOnce)];
    /// mqtt_client.subscribe_blocking(vec);
    ///
    /// let vec = [("test/topic1", QoS::ExactlyOnce), ("test/topic2", QoS::AtLeastOnce)];
    /// mqtt_client.subscribe_blocking(vec.as_slice());
    ///
    /// let sub_options = SubscriptionOptions{
    ///   retain_handling: RetainHandling::TWO,
    ///   retain_as_publish: false,
    ///   no_local: false,
    ///   qos: QoS::AtLeastOnce,
    /// };
    /// mqtt_client.subscribe_blocking(("final/test/topic", sub_options));
    /// # });
    /// ```
    pub fn subscribe_blocking<A: Into<Subscription>>(&self, into_subscribtions: A) -> Result<(), ClientError> {
        let pkid = self.available_packet_ids.recv_blocking().map_err(|_| ClientError::NoNetworkChannel)?;
        let subscription: Subscription = into_subscribtions.into();
        let sub = Subscribe::new(pkid, subscription.0);

        sub.validate(self.max_packet_size)?;
        self.to_network_s.send_blocking(Packet::Subscribe(sub)).map_err(|_| ClientError::NoNetworkChannel)?;
        Ok(())
    }

    /// Creates a subscribe packet with additional subscribe packet properties.
    /// The packet is then transferred to the Network stack for transmission.
    ///
    /// Can be called with anything that can be converted into [`Subscription`]
    ///
    /// This function blocks until the packet is queued for transmission
    /// # Examples
    /// ```
    /// # let options = mqrstt::ConnectOptions::new("example".to_string());
    /// # let (network, mqtt_client) = mqrstt::new_sync::<std::net::TcpStream>(options);
    ///
    /// use mqrstt::packets::QoS;
    /// use mqrstt::packets::{SubscribeProperties, SubscriptionOptions, RetainHandling};
    ///
    /// let sub_properties = SubscribeProperties{
    ///   subscription_id: Some(1),
    ///   user_properties: vec![],
    /// };
    ///
    /// // retain_handling: RetainHandling::ZERO, retain_as_publish: false, no_local: false, qos: QoS::AtMostOnce,
    /// mqtt_client.subscribe_with_properties_blocking("test/topic", sub_properties);
    ///
    /// # let sub_properties = SubscribeProperties{
    /// #   subscription_id: Some(1),
    /// #   user_properties: vec![],
    /// # };
    ///
    /// // retain_handling: RetainHandling::ZERO, retain_as_publish: false, no_local: false, qos: QoS::ExactlyOnce,
    /// mqtt_client.subscribe_with_properties_blocking(("test/topic", QoS::ExactlyOnce), sub_properties);
    ///
    /// # let sub_properties = SubscribeProperties{
    /// #   subscription_id: Some(1),
    /// #   user_properties: vec![],
    /// # };
    ///
    /// let vec = vec![("test/topic1", QoS::ExactlyOnce), ("test/topic2", QoS::ExactlyOnce)];
    /// mqtt_client.subscribe_with_properties_blocking(vec, sub_properties);
    ///
    /// # let sub_properties = SubscribeProperties{
    /// #   subscription_id: Some(1),
    /// #   user_properties: vec![],
    /// # };
    ///  
    /// let vec = [("test/topic1", QoS::ExactlyOnce), ("test/topic2", QoS::ExactlyOnce)];
    /// mqtt_client.subscribe_with_properties_blocking(vec.as_slice(), sub_properties);
    ///
    /// # let sub_properties = SubscribeProperties{
    /// #   subscription_id: Some(1),
    /// #   user_properties: vec![],
    /// # };
    ///  
    /// let sub_options = SubscriptionOptions{
    ///   retain_handling: RetainHandling::ZERO,
    ///   retain_as_publish: false,
    ///   no_local: false,
    ///   qos: QoS::AtLeastOnce,
    /// };
    /// mqtt_client.subscribe_with_properties_blocking(("final/test/topic", sub_options), sub_properties);
    /// ```
    pub fn subscribe_with_properties_blocking<S: Into<Subscription>>(&self, into_subscribtions: S, properties: SubscribeProperties) -> Result<(), ClientError> {
        let pkid = self.available_packet_ids.recv_blocking().map_err(|_| ClientError::NoNetworkChannel)?;
        let sub = Subscribe {
            packet_identifier: pkid,
            properties,
            topics: into_subscribtions.into().0,
        };

        sub.validate(self.max_packet_size)?;
        self.to_network_s.send_blocking(Packet::Subscribe(sub)).map_err(|_| ClientError::NoNetworkChannel)?;
        Ok(())
    }

    /// Creates a Publish packet which is then transferred to the Network stack for transmission.
    ///
    /// Can be called with any payload that can be converted into [`Bytes`]
    ///
    /// This function blocks until the packet is queued for transmission
    /// # Examples
    /// ```
    /// # let options = mqrstt::ConnectOptions::new("example".to_string());
    /// # let (network, mqtt_client) = mqrstt::new_sync::<std::net::TcpStream>(options);
    ///
    /// use mqrstt::packets::QoS;
    /// use bytes::Bytes;
    ///
    /// // publish a message with QoS 0, without a packet identifier
    /// mqtt_client.publish("test/topic", QoS::AtMostOnce, false, "Hello world");
    ///
    /// // publish a message with QoS 1, with a packet identifier
    /// mqtt_client.publish("test/topic", QoS::AtLeastOnce, false, Bytes::from("Hello world"));
    ///
    /// // publish a message with QoS 2, with a packet identifier
    /// mqtt_client.publish("test/topic", QoS::ExactlyOnce, false, Bytes::from("Hello world"));
    ///
    /// // publish a message with QoS 1, with a packet identifier, and the "retain" flag set
    /// let payload: &[u8] = "Hello World!".as_bytes();
    /// mqtt_client.publish("test/topic", QoS::AtLeastOnce, true, payload);
    ///
    /// // publish a message with QoS 1, with a packet identifier, and the "retain" flag set
    /// let payload: Vec<u8> = "Hello World!".as_bytes().to_vec();
    /// mqtt_client.publish("test/topic", QoS::AtMostOnce, true, payload);
    ///
    /// ```
    pub fn publish_blocking<T: Into<String>, P: Into<Bytes>>(&self, topic: T, qos: QoS, retain: bool, payload: P) -> Result<(), ClientError> {
        let pkid = match qos {
            QoS::AtMostOnce => None,
            _ => Some(self.available_packet_ids.recv_blocking().map_err(|_| ClientError::NoNetworkChannel)?),
        };
        #[cfg(feature = "logs")]
        info!("Published message with ID: {:?}", pkid);
        let publish = Publish {
            dup: false,
            qos,
            retain,
            topic: topic.into(),
            packet_identifier: pkid,
            publish_properties: PublishProperties::default(),
            payload: payload.into(),
        };

        publish.validate(self.max_packet_size)?;
        self.to_network_s.send_blocking(Packet::Publish(publish)).map_err(|_| ClientError::NoNetworkChannel)?;
        Ok(())
    }

    /// Creates a Publish packet with additional publish properties.
    /// The packet is then transferred to the Network stack for transmission.
    ///
    /// Can be called with any payload that can be converted into [`Bytes`]
    ///
    /// This function blocks until the packet is queued for transmission
    ///
    /// # Examples
    /// ```
    /// # let options = mqrstt::ConnectOptions::new("example".to_string());
    /// # let (network, mqtt_client) = mqrstt::new_sync::<std::net::TcpStream>(options);
    ///
    /// use mqrstt::packets::QoS;
    /// use mqrstt::packets::PublishProperties;
    /// use bytes::Bytes;
    ///
    /// let properties = PublishProperties{
    ///     response_topic: Some("response/topic".to_string()),
    ///     correlation_data: Some("correlation_data".into()),
    ///     ..Default::default()
    /// };
    ///
    /// // publish a message with QoS 0, without a packet identifier
    /// mqtt_client.publish_with_properties_blocking("test/topic", QoS::AtMostOnce, false, Bytes::from("Hello world"), properties);
    ///
    /// # let properties = PublishProperties{
    /// #     response_topic: Some("response/topic".to_string()),
    /// #     correlation_data: Some("correlation_data".into()),
    /// #     ..Default::default()
    /// # };
    ///
    /// // publish a message with QoS 1, with a packet identifier
    /// mqtt_client.publish_with_properties_blocking("test/topic", QoS::AtLeastOnce, false, Bytes::from("Hello world"), properties);
    ///
    /// # let properties = PublishProperties{
    /// #     response_topic: Some("response/topic".to_string()),
    /// #     correlation_data: Some("correlation_data".into()),
    /// #     ..Default::default()
    /// # };
    ///
    /// // publish a message with QoS 2, with a packet identifier
    /// mqtt_client.publish_with_properties_blocking("test/topic", QoS::ExactlyOnce, false, Bytes::from("Hello world"), properties);
    ///
    /// # let properties = PublishProperties{
    /// #     response_topic: Some("response/topic".to_string()),
    /// #     correlation_data: Some("correlation_data".into()),
    /// #     ..Default::default()
    /// # };
    ///
    /// // publish a message with QoS 1, with a packet identifier, and the "retain" flag set
    /// let payload = "Hello World!".as_bytes();
    /// mqtt_client.publish_with_properties_blocking("test/topic", QoS::AtLeastOnce, true, payload, properties);
    ///
    /// # let properties = PublishProperties{
    /// #     response_topic: Some("response/topic".to_string()),
    /// #     correlation_data: Some("correlation_data".into()),
    /// #     ..Default::default()
    /// # };
    ///
    /// // publish a message with QoS 1, with a packet identifier, and the "retain" flag set
    /// let payload = "Hello World!".as_bytes().to_vec();
    /// mqtt_client.publish_with_properties_blocking("test/topic", QoS::AtMostOnce, true, payload, properties);
    ///
    /// ```
    pub fn publish_with_properties_blocking<T: Into<String>, P: Into<Bytes>>(&self, topic: T, qos: QoS, retain: bool, payload: P, properties: PublishProperties) -> Result<(), ClientError> {
        let pkid = match qos {
            QoS::AtMostOnce => None,
            _ => Some(self.available_packet_ids.recv_blocking().map_err(|_| ClientError::NoNetworkChannel)?),
        };
        let publish = Publish {
            dup: false,
            qos,
            retain,
            topic: topic.into(),
            packet_identifier: pkid,
            publish_properties: properties,
            payload: payload.into(),
        };

        publish.validate(self.max_packet_size)?;
        self.to_network_s.send_blocking(Packet::Publish(publish)).map_err(|_| ClientError::NoNetworkChannel)?;
        Ok(())
    }

    /// Creates a Unsubscribe packet which is then transferred to the Network stack for transmission.
    ///
    /// Can be called with anything that can be converted into [`UnsubscribeTopics`]
    ///
    /// This function blocks until the packet is queued for transmission
    ///
    /// # Examples
    /// ```
    /// # let options = mqrstt::ConnectOptions::new("example".to_string());
    /// # let (network, mqtt_client) = mqrstt::new_sync::<std::net::TcpStream>(options);
    ///
    /// // Unsubscribe from a single topic specified as a string:
    /// let topic = "test/topic";
    /// mqtt_client.unsubscribe(topic);
    ///
    /// // Unsubscribe from multiple topics specified as an array of string slices:
    /// let topics = &["test/topic1", "test/topic2"];
    /// mqtt_client.unsubscribe(topics.as_slice());
    ///
    /// // Unsubscribe from a single topic specified as a String:
    /// let topic = String::from("test/topic");
    /// mqtt_client.unsubscribe(topic);
    ///
    /// // Unsubscribe from multiple topics specified as a Vec<String>:
    /// let topics = vec![String::from("test/topic1"), String::from("test/topic2")];
    /// mqtt_client.unsubscribe(topics);
    ///
    /// // Unsubscribe from multiple topics specified as an array of String:
    /// let topics = &[String::from("test/topic1"), String::from("test/topic2")];
    /// mqtt_client.unsubscribe(topics.as_slice());
    ///
    /// ```
    pub fn unsubscribe_blocking<T: Into<UnsubscribeTopics>>(&self, into_topics: T) -> Result<(), ClientError> {
        let pkid = self.available_packet_ids.recv_blocking().map_err(|_| ClientError::NoNetworkChannel)?;
        let unsub = Unsubscribe {
            packet_identifier: pkid,
            properties: UnsubscribeProperties::default(),
            topics: into_topics.into().0,
        };

        unsub.validate(self.max_packet_size)?;
        self.to_network_s.send_blocking(Packet::Unsubscribe(unsub)).map_err(|_| ClientError::NoNetworkChannel)?;
        Ok(())
    }

    /// Creates a Unsubscribe packet with additional unsubscribe properties [`UnsubscribeProperties`].
    /// The packet is then transferred to the Network stack for transmission.
    ///
    /// Can be called with anything that can be converted into [`UnsubscribeTopics`]
    ///
    /// This function blocks until the packet is queued for transmission
    /// # Examples
    ///
    /// ```
    /// # let options = mqrstt::ConnectOptions::new("example".to_string());
    /// # let (network, mqtt_client) = mqrstt::new_sync::<std::net::TcpStream>(options);
    ///
    /// use mqrstt::packets::UnsubscribeProperties;
    ///
    /// let properties = UnsubscribeProperties{
    ///     user_properties: vec![("property".to_string(), "value".to_string())],
    /// };
    ///
    /// // Unsubscribe from a single topic specified as a string:
    /// let topic = "test/topic";
    /// mqtt_client.unsubscribe_with_properties_blocking(topic, properties);
    ///
    /// # let properties = UnsubscribeProperties{
    /// #     user_properties: vec![("property".to_string(), "value".to_string())],
    /// # };
    ///
    /// // Unsubscribe from multiple topics specified as an array of string slices:
    /// let topics = &["test/topic1", "test/topic2"];
    /// mqtt_client.unsubscribe_with_properties_blocking(topics.as_slice(), properties);
    ///
    /// # let properties = UnsubscribeProperties{
    /// #     user_properties: vec![("property".to_string(), "value".to_string())],
    /// # };
    ///  
    /// // Unsubscribe from a single topic specified as a String:
    /// let topic = String::from("test/topic");
    /// mqtt_client.unsubscribe_with_properties_blocking(topic, properties);
    ///
    /// # let properties = UnsubscribeProperties{
    /// #     user_properties: vec![("property".to_string(), "value".to_string())],
    /// # };
    ///  
    /// // Unsubscribe from multiple topics specified as a Vec<String>:
    /// let topics = vec![String::from("test/topic1"), String::from("test/topic2")];
    /// mqtt_client.unsubscribe_with_properties_blocking(topics, properties);
    ///
    /// # let properties = UnsubscribeProperties{
    /// #     user_properties: vec![("property".to_string(), "value".to_string())],
    /// # };
    ///  
    /// // Unsubscribe from multiple topics specified as an array of String:
    /// let topics = &[String::from("test/topic1"), String::from("test/topic2")];
    /// mqtt_client.unsubscribe_with_properties_blocking(topics.as_slice(), properties);
    /// ```
    pub fn unsubscribe_with_properties_blocking<T: Into<UnsubscribeTopics>>(&self, into_topics: T, properties: UnsubscribeProperties) -> Result<(), ClientError> {
        let pkid = self.available_packet_ids.recv_blocking().map_err(|_| ClientError::NoNetworkChannel)?;
        let unsub = Unsubscribe {
            packet_identifier: pkid,
            properties,
            topics: into_topics.into().0,
        };

        unsub.validate(self.max_packet_size)?;
        self.to_network_s.send_blocking(Packet::Unsubscribe(unsub)).map_err(|_| ClientError::NoNetworkChannel)?;
        Ok(())
    }

    /// Creates a Disconnect packet which is then transferred to the Network stack for transmission.
    ///
    /// This function blocks until the packet is queued for transmission
    ///
    /// # Example
    ///
    /// ```
    /// # let options = mqrstt::ConnectOptions::new("example".to_string());
    /// # let (network, mqtt_client) = mqrstt::new_sync::<std::net::TcpStream>(options);
    ///
    /// mqtt_client.disconnect_blocking().unwrap();
    ///
    /// ```
    pub fn disconnect_blocking(&self) -> Result<(), ClientError> {
        let disconnect = Disconnect {
            reason_code: DisconnectReasonCode::NormalDisconnection,
            properties: DisconnectProperties::default(),
        };

        self.to_network_s.send_blocking(Packet::Disconnect(disconnect)).map_err(|_| ClientError::NoNetworkChannel)?;
        Ok(())
    }

    /// Creates a Disconnect packet with additional [`DisconnectReasonCode`] and [`DisconnectProperties`].
    /// The packet is then transferred to the Network stack for transmission.
    ///
    /// This function blocks until the packet is queued for transmission
    ///
    /// # Example
    ///
    /// ```
    /// # let options = mqrstt::ConnectOptions::new("example".to_string());
    /// # let (network, mqtt_client) = mqrstt::new_sync::<std::net::TcpStream>(options);
    ///
    /// use mqrstt::packets::DisconnectProperties;
    /// use mqrstt::packets::reason_codes::DisconnectReasonCode;
    ///
    /// let properties = DisconnectProperties {
    ///     reason_string: Some("Reason here".to_string()),
    ///     ..Default::default()
    /// };
    ///
    /// mqtt_client.disconnect_with_properties_blocking(DisconnectReasonCode::NormalDisconnection, properties).unwrap();
    ///
    /// ```
    pub fn disconnect_with_properties_blocking(&self, reason_code: DisconnectReasonCode, properties: DisconnectProperties) -> Result<(), ClientError> {
        let disconnect = Disconnect { reason_code, properties };
        self.to_network_s.send_blocking(Packet::Disconnect(disconnect)).map_err(|_| ClientError::NoNetworkChannel)?;
        Ok(())
    }
}

#[cfg(any(feature = "tokio", feature = "smol", feature = "quic"))]
#[cfg(test)]
mod tests {
    use async_channel::Receiver;

    use crate::{
        error::{ClientError, PacketValidationError},
        packets::{reason_codes::DisconnectReasonCode, DisconnectProperties, Packet, PacketType, QoS, UnsubscribeProperties},
    };

    use super::MqttClient;

    fn create_new_test_client() -> (MqttClient, Receiver<Packet>, Receiver<Packet>) {
        let (s, r) = async_channel::bounded(100);

        for i in 1..=100 {
            s.send_blocking(i).unwrap();
        }

        let (client_to_handler_s, client_to_handler_r) = async_channel::bounded(100);
        let (_, to_network_r) = async_channel::bounded(100);

        let client = MqttClient::new(r, client_to_handler_s, Some(500000));

        (client, client_to_handler_r, to_network_r)
    }

    #[tokio::test]
    async fn publish_with_just_right_topic_len() {
        let (client, _client_to_handler_r, _) = create_new_test_client();

        let res = client.publish("way".repeat(21845).to_string(), QoS::ExactlyOnce, false, "hello").await;

        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn publish_with_too_long_topic() {
        let (client, _client_to_handler_r, _) = create_new_test_client();

        let res = client.publish("way".repeat(21846).to_string(), QoS::ExactlyOnce, false, "hello").await;

        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ClientError::ValidationError(PacketValidationError::TopicSize(65538)));
    }

    #[tokio::test]
    async fn subscribe_with_too_long_topic() {
        let (client, _client_to_handler_r, _) = create_new_test_client();

        let topics = ["WAYYY TOOO LONG".repeat(10000), "hello".to_string()];

        let res = client.subscribe(topics.as_slice()).await;

        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ClientError::ValidationError(PacketValidationError::TopicSize(150000)));
    }

    #[tokio::test]
    async fn subscribe_with_just_right_topic_len() {
        let (client, _client_to_handler_r, _) = create_new_test_client();

        let topics = ["way".repeat(21845), "hello".to_string()];

        let res = client.subscribe(topics.as_slice()).await;

        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn publish_with_too_large_mqtt_packet() {
        let (client, _client_to_handler_r, _) = create_new_test_client();

        let res = client.publish("way".repeat(21845).to_string(), QoS::ExactlyOnce, false, "hello".repeat(86893)).await;

        assert!(res.is_err());
        assert_eq!(ClientError::ValidationError(PacketValidationError::MaxPacketSize(500005)), res.unwrap_err())
    }

    #[tokio::test]
    async fn unsubscribe_with_properties_test() {
        let (client, client_to_handler_r, _) = create_new_test_client();

        let prop = UnsubscribeProperties {
            user_properties: vec![("A".to_string(), "B".to_string())],
        };

        client.unsubscribe_with_properties("Topic", prop.clone()).await.unwrap();
        let unsubscribe = client_to_handler_r.recv().await.unwrap();

        assert_eq!(PacketType::Unsubscribe, unsubscribe.packet_type());

        if let Packet::Unsubscribe(unsub) = unsubscribe {
            assert_eq!(1, unsub.packet_identifier);
            assert_eq!(vec!["Topic"], unsub.topics);
            assert_eq!(prop, unsub.properties);
        } else {
            // To make sure we did the if branch
            unreachable!();
        }
    }

    #[tokio::test]
    async fn disconnect_test() {
        let (client, client_to_handler_r, _) = create_new_test_client();
        client.disconnect().await.unwrap();
        let disconnect = client_to_handler_r.recv().await.unwrap();
        assert_eq!(PacketType::Disconnect, disconnect.packet_type());

        if let Packet::Disconnect(res) = disconnect {
            assert_eq!(DisconnectReasonCode::NormalDisconnection, res.reason_code);
            assert_eq!(DisconnectProperties::default(), res.properties);
        } else {
            // To make sure we did the if branch
            unreachable!();
        }
    }

    #[tokio::test]
    async fn disconnect_with_properties_test() {
        let (client, client_to_handler_r, _) = create_new_test_client();
        client.disconnect_with_properties(DisconnectReasonCode::KeepAliveTimeout, Default::default()).await.unwrap();
        let disconnect = client_to_handler_r.recv().await.unwrap();
        assert_eq!(PacketType::Disconnect, disconnect.packet_type());

        if let Packet::Disconnect(res) = disconnect {
            assert_eq!(DisconnectReasonCode::KeepAliveTimeout, res.reason_code);
            assert_eq!(DisconnectProperties::default(), res.properties);
        } else {
            // To make sure we did the if branch
            unreachable!();
        }
    }

    #[tokio::test]
    async fn disconnect_with_properties_test2() {
        let (client, client_to_handler_r, _) = create_new_test_client();
        let properties = DisconnectProperties {
            reason_string: Some("TestString".to_string()),
            ..Default::default()
        };

        client.disconnect_with_properties(DisconnectReasonCode::KeepAliveTimeout, properties.clone()).await.unwrap();
        let disconnect = client_to_handler_r.recv().await.unwrap();
        assert_eq!(PacketType::Disconnect, disconnect.packet_type());

        if let Packet::Disconnect(res) = disconnect {
            assert_eq!(DisconnectReasonCode::KeepAliveTimeout, res.reason_code);
            assert_eq!(properties, res.properties);
        } else {
            // To make sure we did the if branch
            unreachable!();
        }
    }
}
