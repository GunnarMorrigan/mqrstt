use async_channel::{Receiver, Sender};

#[cfg(feature = "logs")]
use tracing::info;

use crate::{
    error::ClientError,
    packets::{
        mqtt_trait::PacketValidation,
        DisconnectReasonCode,
        Packet, QoS, 
        // disconnect::{Disconnect, DisconnectProperties}, 
        // publish::{Publish, PublishProperties}, 
        // subscribe::{Subscribe, SubscribeProperties, Subscription}, 
        // unsubscribe::{Unsubscribe, UnsubscribeProperties, UnsubscribeTopics},

        {Disconnect, DisconnectProperties}, 
        {Publish, PublishProperties}, 
        {Subscribe, SubscribeProperties, Subscription}, 
        {Unsubscribe, UnsubscribeProperties, UnsubscribeTopics},
    },
};

#[derive(Debug, Clone)]
/// A Clonable client that can be used to perform MQTT operations
/// 
/// This object is never self constructed but is a obtained by calling the builder functions on [`crate::NetworkBuilder`]
pub struct MqttClient {
    /// Provides this client with an available packet id or waits on it.
    available_packet_ids_r: Receiver<u16>,

    /// Sends Publish, Subscribe, Unsubscribe to the event handler to handle later.
    to_network_s: Sender<Packet>,

    /// MQTT packet size limit
    max_packet_size: usize,
}

impl MqttClient {
    pub(crate) fn new(available_packet_ids_r: Receiver<u16>, to_network_s: Sender<Packet>, max_packet_size: usize) -> Self {
        Self {
            available_packet_ids_r,
            to_network_s,
            max_packet_size,
        }
    }

    /// This function is only here for you to use during testing of for example your handler
    /// For a simple client look at [`MqttClient::test_client`]
    #[cfg(feature = "test")]
    pub fn test_custom_client(available_packet_ids_r: Receiver<u16>, to_network_s: Sender<Packet>, max_packet_size: usize) -> Self {
        Self {
            available_packet_ids_r,
            to_network_s,
            max_packet_size,
        }
    }



    /// This function is only here for you to use during testing of for example your handler
    /// For control over the input of this type look at [`MqttClient::test_custom_client`]
    ///
    /// The returned values should not be dropped otherwise the client won't be able to operate normally.
    ///
    /// # Example
    /// ```ignore
    /// let (
    ///     client, // An instance of this client
    ///     ids, // Allows you to indicate which packet IDs have become available again.
    ///     network_receiver // Messages send through the `client` will be dispatched through this channel
    /// ) = MqttClient::test_client();
    ///
    /// // perform testing
    ///
    /// // Make sure to not drop these before the test is done!
    /// std::hint::black_box((ids, network_receiver));
    /// ```
    #[cfg(feature = "test")]
    pub fn test_client() -> (Self, crate::available_packet_ids::AvailablePacketIds, Receiver<Packet>) {
        use async_channel::unbounded;

        use crate::{available_packet_ids::AvailablePacketIds, util::constants::MAXIMUM_PACKET_SIZE};

        let (available_packet_ids, available_packet_ids_r) = AvailablePacketIds::new(u16::MAX);

        let (s, r) = unbounded();

        (
            Self {
                available_packet_ids_r,
                to_network_s: s,
                max_packet_size: MAXIMUM_PACKET_SIZE as usize,
            },
            available_packet_ids,
            r,
        )
    }
}

/// Async functions to perform MQTT operations
impl MqttClient {
    /// Creates a subscribe packet that is then asynchronously transferred to the Network stack for transmission
    ///
    /// Can be called with anything that can be converted into [`Subscription`]
    ///
    /// # Examples
    /// ```
    /// # let (network, mqtt_client) = mqrstt::NetworkBuilder::<(), smol::net::TcpStream>::new_from_client_id("example_id").smol_sequential_network();
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
        let pkid = self.available_packet_ids_r.recv().await.map_err(|_| ClientError::NoNetworkChannel)?;
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
    /// # let (network, mqtt_client) = mqrstt::NetworkBuilder::<(), smol::net::TcpStream>::new_from_client_id("example_id").smol_sequential_network();
    /// # smol::block_on(async {
    ///
    /// use mqrstt::packets::QoS;
    /// use mqrstt::packets::{SubscribeProperties, SubscriptionOptions, RetainHandling};
    ///
    /// let sub_properties = SubscribeProperties{
    ///   subscription_identifier: Some(1),
    ///   user_properties: vec![],
    /// };
    /// 
    /// let sub_properties_clone = sub_properties.clone();
    ///
    /// // retain_handling: RetainHandling::ZERO, retain_as_publish: false, no_local: false, qos: QoS::AtMostOnce,
    /// mqtt_client.subscribe_with_properties("test/topic", sub_properties).await;
    ///
    /// # let sub_properties = sub_properties_clone.clone();
    ///
    /// // retain_handling: RetainHandling::ZERO, retain_as_publish: false, no_local: false, qos: QoS::ExactlyOnce,
    /// mqtt_client.subscribe_with_properties(("test/topic", QoS::ExactlyOnce), sub_properties).await;
    ///
    /// # let sub_properties = sub_properties_clone.clone();
    ///
    /// let vec = vec![("test/topic1", QoS::ExactlyOnce), ("test/topic2", QoS::ExactlyOnce)];
    /// mqtt_client.subscribe_with_properties(vec, sub_properties).await;
    ///
    /// # let sub_properties = sub_properties_clone.clone();
    ///  
    /// let vec = [("test/topic1", QoS::ExactlyOnce), ("test/topic2", QoS::ExactlyOnce)];
    /// mqtt_client.subscribe_with_properties(vec.as_slice(), sub_properties).await;
    ///
    /// # let sub_properties = sub_properties_clone.clone();
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
        let pkid = self.available_packet_ids_r.recv().await.map_err(|_| ClientError::NoNetworkChannel)?;
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
    /// # let (_, mqtt_client) = mqrstt::NetworkBuilder::<(), smol::net::TcpStream>::new_from_client_id("Example").smol_sequential_network();
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
    pub async fn publish<T: AsRef<str>, P: Into<Vec<u8>>>(&self, topic: T, qos: QoS, retain: bool, payload: P) -> Result<(), ClientError> {
        let pkid = match qos {
            QoS::AtMostOnce => None,
            _ => Some(self.available_packet_ids_r.recv().await.map_err(|_| ClientError::NoNetworkChannel)?),
        };
        #[cfg(feature = "logs")]
        info!("Published message with ID: {:?}", pkid);
        let publish = Publish {
            dup: false,
            qos,
            retain,
            topic: topic.as_ref().into(),
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
    /// # let (network, mqtt_client) = mqrstt::NetworkBuilder::<(), smol::net::TcpStream>::new_from_client_id("Example").smol_sequential_network();
    /// # smol::block_on(async {
    ///
    /// use mqrstt::packets::QoS;
    /// use mqrstt::packets::PublishProperties;
    /// use bytes::Bytes;
    ///
    /// let properties = PublishProperties{
    ///     response_topic: Some("response/topic".into()),
    ///     correlation_data: Some("correlation_data".into()),
    ///     ..Default::default()
    /// };
    /// 
    /// # let properties_clone = properties.clone();
    ///
    /// // publish a message with QoS 0, without a packet identifier
    /// mqtt_client.publish_with_properties("test/topic", QoS::AtMostOnce, false, Bytes::from("Hello world"), properties).await;
    ///
    /// # let properties = properties_clone.clone();
    ///
    /// // publish a message with QoS 1, with a packet identifier
    /// mqtt_client.publish_with_properties("test/topic", QoS::AtLeastOnce, false, Bytes::from("Hello world"), properties).await;
    ///
    /// # let properties = properties_clone.clone();
    ///
    /// // publish a message with QoS 2, with a packet identifier
    /// mqtt_client.publish_with_properties("test/topic", QoS::ExactlyOnce, false, Bytes::from("Hello world"), properties).await;
    ///
    /// # let properties = properties_clone.clone();
    ///
    /// // publish a message with QoS 1, with a packet identifier, and the "retain" flag set
    /// let payload = "Hello World!".as_bytes();
    /// mqtt_client.publish_with_properties("test/topic", QoS::AtLeastOnce, true, payload, properties).await;
    ///
    /// # let properties = properties_clone.clone();
    ///
    /// // publish a message with QoS 1, with a packet identifier, and the "retain" flag set
    /// let payload = "Hello World!".as_bytes().to_vec();
    /// mqtt_client.publish_with_properties("test/topic", QoS::AtMostOnce, true, payload, properties).await;
    ///
    /// # });
    /// # let _network = std::hint::black_box(network);
    /// ```
    pub async fn publish_with_properties<T: AsRef<str>, P: Into<Vec<u8>>>(&self, topic: T, qos: QoS, retain: bool, payload: P, properties: PublishProperties) -> Result<(), ClientError> {
        let pkid = match qos {
            QoS::AtMostOnce => None,
            _ => Some(self.available_packet_ids_r.recv().await.map_err(|_| ClientError::NoNetworkChannel)?),
        };
        let publish = Publish {
            dup: false,
            qos,
            retain,
            topic: topic.as_ref().into(),
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
    /// # let (network, mqtt_client) = mqrstt::NetworkBuilder::<(), smol::net::TcpStream>::new_from_client_id("Example").smol_sequential_network();
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
    /// # let _network = std::hint::black_box(network);
    /// ```
    pub async fn unsubscribe<T: Into<UnsubscribeTopics>>(&self, into_topics: T) -> Result<(), ClientError> {
        let pkid = self.available_packet_ids_r.recv().await.map_err(|_| ClientError::NoNetworkChannel)?;
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
    /// # let (_, mqtt_client) = mqrstt::NetworkBuilder::<(), smol::net::TcpStream>::new_from_client_id("Example").smol_sequential_network();
    /// # smol::block_on(async {
    ///
    /// use mqrstt::packets::UnsubscribeProperties;
    ///
    /// let properties = UnsubscribeProperties{
    ///     user_properties: vec![("property".into(), "value".into())],
    /// };
    ///
    /// // Unsubscribe from a single topic specified as a string:
    /// let topic = "test/topic";
    /// mqtt_client.unsubscribe_with_properties(topic, properties).await;
    ///
    /// # let properties = UnsubscribeProperties{
    /// #     user_properties: vec![("property".into(), "value".into())],
    /// # };
    ///
    /// // Unsubscribe from multiple topics specified as an array of string slices:
    /// let topics = &["test/topic1", "test/topic2"];
    /// mqtt_client.unsubscribe_with_properties(topics.as_slice(), properties).await;
    ///
    /// # let properties = UnsubscribeProperties{
    /// #     user_properties: vec![("property".into(), "value".into())],
    /// # };
    ///  
    /// // Unsubscribe from a single topic specified as a String:
    /// let topic = String::from("test/topic");
    /// mqtt_client.unsubscribe_with_properties(topic, properties).await;
    ///
    /// # let properties = UnsubscribeProperties{
    /// #     user_properties: vec![("property".into(), "value".into())],
    /// # };
    ///  
    /// // Unsubscribe from multiple topics specified as a Vec<String>:
    /// let topics = vec![String::from("test/topic1"), String::from("test/topic2")];
    /// mqtt_client.unsubscribe_with_properties(topics, properties).await;
    ///
    /// # let properties = UnsubscribeProperties{
    /// #     user_properties: vec![("property".into(), "value".into())],
    /// # };
    ///  
    /// // Unsubscribe from multiple topics specified as an array of String:
    /// let topics = &[String::from("test/topic1"), String::from("test/topic2")];
    /// mqtt_client.unsubscribe_with_properties(topics.as_slice(), properties).await;
    ///
    /// # });
    /// ```
    pub async fn unsubscribe_with_properties<T: Into<UnsubscribeTopics>>(&self, into_topics: T, properties: UnsubscribeProperties) -> Result<(), ClientError> {
        let pkid = self.available_packet_ids_r.recv().await.map_err(|_| ClientError::NoNetworkChannel)?;
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
    /// # let (network, mqtt_client) = mqrstt::NetworkBuilder::<(), smol::net::TcpStream>::new_from_client_id("Example").smol_sequential_network();
    /// # smol::block_on(async {
    ///
    /// mqtt_client.disconnect().await.unwrap();
    ///
    /// # });
    /// # let _network = std::hint::black_box(network);
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
    /// # let (network, mqtt_client) = mqrstt::NetworkBuilder::<(), smol::net::TcpStream>::new_from_client_id("Example").smol_sequential_network();
    /// # smol::block_on(async {
    ///
    /// use mqrstt::packets::DisconnectProperties;
    /// use mqrstt::packets::DisconnectReasonCode;
    ///
    /// let properties = DisconnectProperties {
    ///     reason_string: Some("Reason here".into()),
    ///     ..Default::default()
    /// };
    ///
    /// mqtt_client.disconnect_with_properties(DisconnectReasonCode::NormalDisconnection, properties).await.unwrap();
    ///
    /// # });
    /// # let _network = std::hint::black_box(network);
    /// ```
    pub async fn disconnect_with_properties(&self, reason_code: DisconnectReasonCode, properties: DisconnectProperties) -> Result<(), ClientError> {
        let disconnect = Disconnect { reason_code, properties };
        self.to_network_s.send(Packet::Disconnect(disconnect)).await.map_err(|_| ClientError::NoNetworkChannel)?;
        Ok(())
    }
}

/// Sync functions to perform MQTT operations
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
    /// # let (network, mqtt_client) = mqrstt::NetworkBuilder::<(), smol::net::TcpStream>::new_from_client_id("Example").smol_sequential_network();
    /// # smol::block_on(async {
    /// use mqrstt::packets::QoS;
    /// use mqrstt::packets::{SubscriptionOptions, RetainHandling};
    ///
    /// // retain_handling: RetainHandling::ZERO, retain_as_publish: false, no_local: false, qos: QoS::AtMostOnce,
    /// mqtt_client.subscribe_blocking("test/topic").unwrap();
    ///
    /// // retain_handling: RetainHandling::ZERO, retain_as_publish: false, no_local: false, qos: QoS::ExactlyOnce,
    /// mqtt_client.subscribe_blocking(("test/topic", QoS::ExactlyOnce)).unwrap();
    ///
    /// let vec = vec![("test/topic1", QoS::ExactlyOnce), ("test/topic2", QoS::AtMostOnce)];
    /// mqtt_client.subscribe_blocking(vec).unwrap();
    ///
    /// let vec = [("test/topic1", QoS::ExactlyOnce), ("test/topic2", QoS::AtLeastOnce)];
    /// mqtt_client.subscribe_blocking(vec.as_slice()).unwrap();
    ///
    /// let sub_options = SubscriptionOptions{
    ///   retain_handling: RetainHandling::TWO,
    ///   retain_as_publish: false,
    ///   no_local: false,
    ///   qos: QoS::AtLeastOnce,
    /// };
    /// mqtt_client.subscribe_blocking(("final/test/topic", sub_options)).unwrap();
    /// # });
    /// ```
    pub fn subscribe_blocking<A: Into<Subscription>>(&self, into_subscribtions: A) -> Result<(), ClientError> {
        let pkid = self.available_packet_ids_r.recv_blocking().map_err(|_| ClientError::NoNetworkChannel)?;
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
    /// # let (network, mqtt_client) = mqrstt::NetworkBuilder::<(), smol::net::TcpStream>::new_from_client_id("Example").smol_sequential_network();
    /// # smol::block_on(async {
    /// use mqrstt::packets::QoS;
    /// use mqrstt::packets::{SubscribeProperties, SubscriptionOptions, RetainHandling};
    ///
    /// let sub_properties = SubscribeProperties{
    ///   subscription_identifier: Some(1),
    ///   user_properties: vec![],
    /// };
    /// # let sub_properties_clone = sub_properties.clone();
    /// 
    /// // retain_handling: RetainHandling::ZERO, retain_as_publish: false, no_local: false, qos: QoS::AtMostOnce,
    /// mqtt_client.subscribe_with_properties_blocking("test/topic", sub_properties).unwrap();
    ///
    /// # let sub_properties = sub_properties_clone.clone();
    ///
    /// // retain_handling: RetainHandling::ZERO, retain_as_publish: false, no_local: false, qos: QoS::ExactlyOnce,
    /// mqtt_client.subscribe_with_properties_blocking(("test/topic", QoS::ExactlyOnce), sub_properties).unwrap();
    ///
    /// # let sub_properties = sub_properties_clone.clone();
    ///
    /// let vec = vec![("test/topic1", QoS::ExactlyOnce), ("test/topic2", QoS::ExactlyOnce)];
    /// mqtt_client.subscribe_with_properties_blocking(vec, sub_properties).unwrap();
    ///
    /// # let sub_properties = sub_properties_clone.clone();
    ///  
    /// let vec = [("test/topic1", QoS::ExactlyOnce), ("test/topic2", QoS::ExactlyOnce)];
    /// mqtt_client.subscribe_with_properties_blocking(vec.as_slice(), sub_properties).unwrap();
    ///
    /// # let sub_properties = sub_properties_clone.clone();
    ///  
    /// let sub_options = SubscriptionOptions{
    ///   retain_handling: RetainHandling::ZERO,
    ///   retain_as_publish: false,
    ///   no_local: false,
    ///   qos: QoS::AtLeastOnce,
    /// };
    /// mqtt_client.subscribe_with_properties_blocking(("final/test/topic", sub_options), sub_properties).unwrap();
    /// # });
    /// ```
    pub fn subscribe_with_properties_blocking<S: Into<Subscription>>(&self, into_subscribtions: S, properties: SubscribeProperties) -> Result<(), ClientError> {
        let pkid = self.available_packet_ids_r.recv_blocking().map_err(|_| ClientError::NoNetworkChannel)?;
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
    /// # let (network, mqtt_client) = mqrstt::NetworkBuilder::<(), smol::net::TcpStream>::new_from_client_id("Example").smol_sequential_network();
    /// # smol::block_on(async {
    /// 
    /// use mqrstt::packets::QoS;
    /// use bytes::Bytes;
    ///
    /// // publish a message with QoS 0, without a packet identifier
    /// mqtt_client.publish_blocking("test/topic", QoS::AtMostOnce, false, "Hello world").unwrap();
    ///
    /// // publish a message with QoS 1, with a packet identifier
    /// mqtt_client.publish_blocking("test/topic", QoS::AtLeastOnce, false, Bytes::from("Hello world")).unwrap();
    ///
    /// // publish a message with QoS 2, with a packet identifier
    /// mqtt_client.publish_blocking("test/topic", QoS::ExactlyOnce, false, Bytes::from("Hello world")).unwrap();
    ///
    /// // publish a message with QoS 1, with a packet identifier, and the "retain" flag set
    /// let payload: &[u8] = "Hello World!".as_bytes();
    /// mqtt_client.publish_blocking("test/topic", QoS::AtLeastOnce, true, payload).unwrap();
    ///
    /// // publish a message with QoS 1, with a packet identifier, and the "retain" flag set
    /// let payload: Vec<u8> = "Hello World!".as_bytes().to_vec();
    /// mqtt_client.publish_blocking("test/topic", QoS::AtMostOnce, true, payload).unwrap();
    ///
    /// # });
    /// ```
    pub fn publish_blocking<T: AsRef<str>, P: Into<Vec<u8>>>(&self, topic: T, qos: QoS, retain: bool, payload: P) -> Result<(), ClientError> {
        let pkid = match qos {
            QoS::AtMostOnce => None,
            _ => Some(self.available_packet_ids_r.recv_blocking().map_err(|_| ClientError::NoNetworkChannel)?),
        };
        #[cfg(feature = "logs")]
        info!("Published message with ID: {:?}", pkid);
        let publish = Publish {
            dup: false,
            qos,
            retain,
            topic: topic.as_ref().into(),
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
    /// # let (network, mqtt_client) = mqrstt::NetworkBuilder::<(), smol::net::TcpStream>::new_from_client_id("Example").smol_sequential_network();
    /// # smol::block_on(async {
    /// 
    /// use mqrstt::packets::QoS;
    /// use mqrstt::packets::PublishProperties;
    /// use bytes::Bytes;
    ///
    /// let properties = PublishProperties{
    ///     response_topic: Some("response/topic".into()),
    ///     correlation_data: Some("correlation_data".into()),
    ///     ..Default::default()
    /// };
    /// 
    /// # let properties_clone = properties.clone();
    ///
    /// // publish a message with QoS 0, without a packet identifier
    /// mqtt_client.publish_with_properties_blocking("test/topic", QoS::AtMostOnce, false, Bytes::from("Hello world"), properties).unwrap();
    ///
    /// # let properties = properties_clone.clone();
    ///
    /// // publish a message with QoS 1, with a packet identifier
    /// mqtt_client.publish_with_properties_blocking("test/topic", QoS::AtLeastOnce, false, Bytes::from("Hello world"), properties).unwrap();
    ///
    /// # let properties = properties_clone.clone();
    ///
    /// // publish a message with QoS 2, with a packet identifier
    /// mqtt_client.publish_with_properties_blocking("test/topic", QoS::ExactlyOnce, false, Bytes::from("Hello world"), properties).unwrap();
    ///
    /// # let properties = properties_clone.clone();
    ///
    /// // publish a message with QoS 1, with a packet identifier, and the "retain" flag set
    /// let payload = "Hello World!".as_bytes();
    /// mqtt_client.publish_with_properties_blocking("test/topic", QoS::AtLeastOnce, true, payload, properties).unwrap();
    ///
    /// # let properties = properties_clone.clone();
    ///
    /// // publish a message with QoS 1, with a packet identifier, and the "retain" flag set
    /// let payload = "Hello World!".as_bytes().to_vec();
    /// mqtt_client.publish_with_properties_blocking("test/topic", QoS::AtMostOnce, true, payload, properties).unwrap();
    ///
    /// # });
    /// ```
    pub fn publish_with_properties_blocking<T: AsRef<str>, P: Into<Vec<u8>>>(&self, topic: T, qos: QoS, retain: bool, payload: P, properties: PublishProperties) -> Result<(), ClientError> {
        let pkid = match qos {
            QoS::AtMostOnce => None,
            _ => Some(self.available_packet_ids_r.recv_blocking().map_err(|_| ClientError::NoNetworkChannel)?),
        };
        let publish = Publish {
            dup: false,
            qos,
            retain,
            topic: topic.as_ref().into(),
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
    /// # let (network, mqtt_client) = mqrstt::NetworkBuilder::<(), smol::net::TcpStream>::new_from_client_id("Example").smol_sequential_network();
    /// # smol::block_on(async {
    ///
    /// // Unsubscribe from a single topic specified as a string:
    /// let topic = "test/topic";
    /// mqtt_client.unsubscribe_blocking(topic);
    ///
    /// // Unsubscribe from multiple topics specified as an array of string slices:
    /// let topics = ["test/topic1", "test/topic2"];
    /// mqtt_client.unsubscribe_blocking(topics.as_slice());
    ///
    /// // Unsubscribe from a single topic specified as a String:
    /// let topic = String::from("test/topic");
    /// mqtt_client.unsubscribe_blocking(topic);
    ///
    /// // Unsubscribe from multiple topics specified as a Vec<String>:
    /// let topics = vec![String::from("test/topic1"), String::from("test/topic2")];
    /// mqtt_client.unsubscribe_blocking(topics);
    ///
    /// // Unsubscribe from multiple topics specified as an array of String:
    /// let topics = &[String::from("test/topic1"), String::from("test/topic2")];
    /// mqtt_client.unsubscribe_blocking(topics.as_slice());
    /// 
    /// # });
    /// # std::hint::black_box(network);
    /// ```
    pub fn unsubscribe_blocking<T: Into<UnsubscribeTopics>>(&self, into_topics: T) -> Result<(), ClientError> {
        let pkid = self.available_packet_ids_r.recv_blocking().map_err(|_| ClientError::NoNetworkChannel)?;
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
    /// # let (network, mqtt_client) = mqrstt::NetworkBuilder::<(), smol::net::TcpStream>::new_from_client_id("Example").smol_sequential_network();
    /// # smol::block_on(async {
    ///
    /// use mqrstt::packets::UnsubscribeProperties;
    ///
    /// let properties = UnsubscribeProperties{
    ///     user_properties: vec![("property".into(), "value".into())],
    /// };
    /// # let properties_clone = properties.clone();
    ///
    /// // Unsubscribe from a single topic specified as a string:
    /// let topic = "test/topic";
    /// mqtt_client.unsubscribe_with_properties_blocking(topic, properties);
    ///
    /// # let properties = properties_clone.clone();
    ///
    /// // Unsubscribe from multiple topics specified as an array of string slices:
    /// let topics = ["test/topic1", "test/topic2"];
    /// mqtt_client.unsubscribe_with_properties_blocking(topics.as_slice(), properties);
    ///
    /// # let properties = properties_clone.clone();
    ///  
    /// // Unsubscribe from a single topic specified as a String:
    /// let topic = String::from("test/topic");
    /// mqtt_client.unsubscribe_with_properties_blocking(topic, properties);
    ///
    /// # let properties = properties_clone.clone();
    ///  
    /// // Unsubscribe from multiple topics specified as a Vec<String>:
    /// let topics = vec![String::from("test/topic1"), String::from("test/topic2")];
    /// mqtt_client.unsubscribe_with_properties_blocking(topics, properties);
    ///
    /// # let properties = properties_clone.clone();
    ///  
    /// // Unsubscribe from multiple topics specified as an array of String:
    /// let topics = ["test/topic1","test/topic2"];
    /// mqtt_client.unsubscribe_with_properties_blocking(topics.as_slice(), properties);
    /// 
    /// # });
    /// # std::hint::black_box(network);
    /// ```
    pub fn unsubscribe_with_properties_blocking<T: Into<UnsubscribeTopics>>(&self, into_topics: T, properties: UnsubscribeProperties) -> Result<(), ClientError> {
        let pkid = self.available_packet_ids_r.recv_blocking().map_err(|_| ClientError::NoNetworkChannel)?;
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
    /// # let (network, mqtt_client) = mqrstt::NetworkBuilder::<(), smol::net::TcpStream>::new_from_client_id("Example").smol_sequential_network();
    /// # smol::block_on(async {
    /// 
    /// mqtt_client.disconnect_blocking().unwrap();
    ///
    /// # });
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
    ///
    /// # let (network, mqtt_client) = mqrstt::NetworkBuilder::<(), smol::net::TcpStream>::new_from_client_id("Example").smol_sequential_network();
    /// # smol::block_on(async {
    /// 
    /// use mqrstt::packets::DisconnectProperties;
    /// use mqrstt::packets::DisconnectReasonCode;
    ///
    /// let properties = DisconnectProperties {
    ///     reason_string: Some("Reason here".into()),
    ///     ..Default::default()
    /// };
    ///
    /// mqtt_client.disconnect_with_properties_blocking(DisconnectReasonCode::NormalDisconnection, properties).unwrap();
    ///
    /// # });
    /// ```
    pub fn disconnect_with_properties_blocking(&self, reason_code: DisconnectReasonCode, properties: DisconnectProperties) -> Result<(), ClientError> {
        let disconnect = Disconnect { reason_code, properties };
        self.to_network_s.send_blocking(Packet::Disconnect(disconnect)).map_err(|_| ClientError::NoNetworkChannel)?;
        Ok(())
    }
}

#[cfg(any(feature = "tokio", feature = "smol"))]
#[cfg(test)]
mod tests {
    use async_channel::Receiver;

    use crate::{
        error::{ClientError, PacketValidationError},
        packets::{DisconnectProperties, DisconnectReasonCode, Packet, PacketType, Publish, QoS, SubscribeProperties, UnsubscribeProperties},
    };

    use super::MqttClient;

    fn create_new_test_client() -> (MqttClient, Receiver<Packet>, Receiver<Packet>) {
        let (s, r) = async_channel::bounded(100);

        for i in 1..=100 {
            s.send_blocking(i).unwrap();
        }

        let (client_to_handler_s, client_to_handler_r) = async_channel::bounded(100);
        let (_, to_network_r) = async_channel::bounded(100);

        let client = MqttClient::new(r, client_to_handler_s, 500000);

        (client, client_to_handler_r, to_network_r)
    }

    #[tokio::test]
    async fn test_subscribe() {
        let (mqtt_client, _client_to_handler_r, _) = create_new_test_client();

        // retain_handling: RetainHandling::ZERO, retain_as_publish: false, no_local: false, qos: QoS::AtMostOnce,
        let _ = mqtt_client.subscribe("test/topic").await;

        // retain_handling: RetainHandling::ZERO, retain_as_publish: false, no_local: false, qos: QoS::ExactlyOnce,
        let _ = mqtt_client.subscribe(("test/topic", QoS::ExactlyOnce)).await;

        let vec = vec![("test/topic1", QoS::ExactlyOnce), ("test/topic2", QoS::AtMostOnce)];
        let _ = mqtt_client.subscribe(vec).await;

        let vec = [("test/topic1", QoS::ExactlyOnce), ("test/topic2", QoS::AtLeastOnce)];
        let _ = mqtt_client.subscribe(vec.as_slice()).await;

        let sub_options = crate::packets::SubscriptionOptions {
            retain_handling: crate::packets::RetainHandling::TWO,
            retain_as_publish: false,
            no_local: false,
            qos: QoS::AtLeastOnce,
        };
        let _ = mqtt_client.subscribe(("final/test/topic", sub_options)).await;
    }

    #[tokio::test]
    async fn test_subscribe_with_properties() {
        let (mqtt_client, client_to_handler_r, to_network_r) = create_new_test_client();
        
        let sub_properties = SubscribeProperties{
            subscription_identifier: Some(1),
            user_properties: vec![],
        };

        // retain_handling: RetainHandling::ZERO, retain_as_publish: false, no_local: false, qos: QoS::AtMostOnce,
        let res = mqtt_client.subscribe_with_properties("test/topic", sub_properties.clone()).await;
        
        assert!(res.is_ok());
        let packet = client_to_handler_r.recv().await.unwrap();
        // assert!(matches!(packet, Packet::Subscribe(sub) if sub.properties.subscription_id == Some(1)));
        assert!(matches!(packet, Packet::Subscribe(sub) if sub.properties == sub_properties && sub.topics[0].0.as_ref() == "test/topic"));

        std::hint::black_box((mqtt_client, client_to_handler_r, to_network_r));
    }

    #[test]
    
    fn test_subscribe_blocking() {
        let (client, client_to_handler_r, to_network_r) = create_new_test_client();

        // retain_handling: RetainHandling::ZERO, retain_as_publish: false, no_local: false, qos: QoS::AtMostOnce,
        client.subscribe_blocking("test/topic").unwrap();
        let packet = client_to_handler_r.recv_blocking().unwrap();
        assert!(matches!(packet, Packet::Subscribe(sub) if sub.topics[0].0.as_ref() == "test/topic"));

        // retain_handling: RetainHandling::ZERO, retain_as_publish: false, no_local: false, qos: QoS::ExactlyOnce,
        client.subscribe_blocking(("test/topic", QoS::ExactlyOnce)).unwrap();
        let packet = client_to_handler_r.recv_blocking().unwrap();
        assert!(matches!(packet, Packet::Subscribe(sub) if sub.topics[0].0.as_ref() == "test/topic"));

        let vec = vec![("test/topic1", QoS::ExactlyOnce), ("test/topic2", QoS::AtMostOnce)];
        client.subscribe_blocking(vec).unwrap();
        let packet = client_to_handler_r.recv_blocking().unwrap();
        assert!(matches!(packet, Packet::Subscribe(sub) if sub.topics[0].0.as_ref() == "test/topic1" && sub.topics[1].0.as_ref() == "test/topic2"));

        let sub_options = crate::packets::SubscriptionOptions {
            retain_handling: crate::packets::RetainHandling::TWO,
            retain_as_publish: false,
            no_local: false,
            qos: QoS::AtLeastOnce,
        };
        client.subscribe_blocking(("final/test/topic", sub_options.clone())).unwrap();
        let packet = client_to_handler_r.recv_blocking().unwrap();
        assert!(matches!(packet, Packet::Subscribe(sub) if sub.topics[0].0.as_ref() == "final/test/topic" && sub.topics[0].1 == sub_options));

        std::hint::black_box((client, client_to_handler_r, to_network_r));
    }

    #[tokio::test]
    async fn test_unsubscribe() {
        let (client, client_to_handler_r, to_network_r) = create_new_test_client();

        // Unsubscribe from a single topic specified as a string:
        let topic = "test/topic";
        client.unsubscribe(topic).await.unwrap();
        let packet = client_to_handler_r.recv().await.unwrap();
        assert!(matches!(packet, Packet::Unsubscribe(unsub) if unsub.topics[0].as_ref() == "test/topic" ));

        // Unsubscribe from multiple topics specified as an array of String:
        let topics = &[String::from("test/topic1"), String::from("test/topic2")];
        client.unsubscribe(topics.as_slice()).await.unwrap();
        let packet = client_to_handler_r.recv().await.unwrap();
        assert!(matches!(packet, Packet::Unsubscribe(unsub) if unsub.topics[0].as_ref() == "test/topic1" && unsub.topics[1].as_ref() == "test/topic2" ));

        std::hint::black_box((client, client_to_handler_r, to_network_r));
    }


    #[test]
    fn test_unsubscribe_blocking() {
        let (client, client_to_handler_r, to_network_r) = create_new_test_client();

        // Unsubscribe from a single topic specified as a string:
        let topic = "test/topic";
        client.unsubscribe_blocking(topic).unwrap();
        let packet = client_to_handler_r.recv_blocking().unwrap();
        assert!(matches!(packet, Packet::Unsubscribe(unsub) if unsub.topics[0].as_ref() == "test/topic" ));

        // Unsubscribe from a single topic specified as a String:
        let topic = String::from("test/topic");
        client.unsubscribe_blocking(topic).unwrap();
        let packet = client_to_handler_r.recv_blocking().unwrap();
        assert!(matches!(packet, Packet::Unsubscribe(unsub) if unsub.topics[0].as_ref() == "test/topic" ));

        // Unsubscribe from multiple topics specified as an array of String:
        let topics = &[String::from("test/topic1"), String::from("test/topic2")];
        client.unsubscribe_blocking(topics.as_slice()).unwrap();
        let packet = client_to_handler_r.recv_blocking().unwrap();
        assert!(matches!(packet, Packet::Unsubscribe(unsub) if unsub.topics[0].as_ref() == "test/topic1" && unsub.topics[1].as_ref() == "test/topic2" ));

        std::hint::black_box((client, client_to_handler_r, to_network_r));
    }


    #[test]
    fn test_unsubscribe_with_properties_blocking() {
        let (client, client_to_handler_r, to_network_r) = create_new_test_client();

        let properties = UnsubscribeProperties{
            user_properties: vec![("property".into(), "value".into())],
        };

        // Unsubscribe from a single topic specified as a string:
        let topic = "test/topic";
        client.unsubscribe_with_properties_blocking(topic, properties.clone()).unwrap();
        let packet = client_to_handler_r.recv_blocking().unwrap();
        assert!(matches!(packet, Packet::Unsubscribe(unsub) if unsub.topics[0].as_ref() == "test/topic" ));

        // Unsubscribe from multiple topics specified as an array of string slices:
        let topics = ["test/topic1", "test/topic2"];
        client.unsubscribe_with_properties_blocking(topics.as_slice(), properties.clone()).unwrap();
        let packet = client_to_handler_r.recv_blocking().unwrap();
        assert!(matches!(packet, Packet::Unsubscribe(unsub) if unsub.topics[0].as_ref() == "test/topic1" && unsub.topics[1].as_ref() == "test/topic2" ));

        // Unsubscribe from multiple topics specified as a Vec<String>:
        let topics = vec![String::from("test/topic1"), String::from("test/topic2")];
        client.unsubscribe_with_properties_blocking(topics, properties.clone()).unwrap();
        let packet = client_to_handler_r.recv_blocking().unwrap();
        assert!(matches!(packet, Packet::Unsubscribe(unsub) if unsub.topics[0].as_ref() == "test/topic1" && unsub.topics[1].as_ref() == "test/topic2" ));

        std::hint::black_box((client, client_to_handler_r, to_network_r));
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
    async fn publish_with_properties() {
        let (client, client_to_handler_r, to_network_r) = create_new_test_client();

        let properties = crate::packets::PublishProperties{
            response_topic: Some("response/topic".into()),
            correlation_data: Some("correlation_other_data".into()),
            ..Default::default()
        };

        for (id, qos) in [QoS::AtMostOnce, QoS::AtLeastOnce, QoS::ExactlyOnce].iter().enumerate() {
            let res = client.publish_with_properties("way".repeat(21845).to_string(), *qos, false, "hello", properties.clone()).await;

            assert!(res.is_ok());
            let packet = client_to_handler_r.recv().await.unwrap();

            let publ = Publish{
                dup: false,
                qos: *qos,
                retain: false,
                topic: "way".repeat(21845).into(),
                packet_identifier: if *qos == QoS::AtMostOnce { None } else { Some(id as u16) },
                publish_properties: properties.clone(),
                payload: "hello".into(),
            };

            assert_eq!(Packet::Publish(publ), packet);
        }

        std::hint::black_box((client, client_to_handler_r, to_network_r));
    }


    #[tokio::test]
    async fn publish_with_just_right_topic_len_properties() {
        let (client, _client_to_handler_r, _) = create_new_test_client();

        let properties = crate::packets::PublishProperties{
            response_topic: Some("response/topic".into()),
            correlation_data: Some("correlation_data".into()),
            ..Default::default()
        };

        let res = client.publish_with_properties("way".repeat(21845).to_string(), QoS::ExactlyOnce, false, "hello", properties).await;

        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn publish_with_too_long_topic_properties() {
        let (client, _client_to_handler_r, _) = create_new_test_client();

        let properties = crate::packets::PublishProperties{
            response_topic: Some("response/topic".into()),
            correlation_data: Some("correlation_data".into()),
            ..Default::default()
        };

        let res = client.publish_with_properties("way".repeat(21846).to_string(), QoS::ExactlyOnce, false, "hello", properties).await;

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
            user_properties: vec![("A".into(), "B".into())],
        };

        client.unsubscribe_with_properties("Topic", prop.clone()).await.unwrap();
        let unsubscribe = client_to_handler_r.recv().await.unwrap();

        assert_eq!(PacketType::Unsubscribe, unsubscribe.packet_type());

        if let Packet::Unsubscribe(unsub) = unsubscribe {
            assert_eq!(1, unsub.packet_identifier);
            assert_eq!(vec![Box::from("Topic")], unsub.topics);
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

        assert!(
            matches!(disconnect, Packet::Disconnect(res) 
                if res.properties == DisconnectProperties::default() && 
                DisconnectReasonCode::NormalDisconnection == res.reason_code
            )
        );
    }

    #[tokio::test]
    async fn test_disconnect_with_properties() {
        let (client, client_to_handler_r, _) = create_new_test_client();
        client.disconnect_with_properties(DisconnectReasonCode::KeepAliveTimeout, Default::default()).await.unwrap();
        let disconnect = client_to_handler_r.recv().await.unwrap();
        assert_eq!(PacketType::Disconnect, disconnect.packet_type());

        assert!(matches!(disconnect, Packet::Disconnect(res) if res.properties == DisconnectProperties::default() && DisconnectReasonCode::KeepAliveTimeout == res.reason_code));
    }

    #[tokio::test]
    async fn test_disconnect_with_properties_2() {
        let (client, client_to_handler_r, _) = create_new_test_client();
        let properties = DisconnectProperties {
            reason_string: Some("TestString".into()),
            ..Default::default()
        };

        client.disconnect_with_properties(DisconnectReasonCode::KeepAliveTimeout, properties.clone()).await.unwrap();
        let disconnect = client_to_handler_r.recv().await.unwrap();
        assert_eq!(PacketType::Disconnect, disconnect.packet_type());

        assert!(matches!(disconnect, Packet::Disconnect(res) if properties == res.properties && DisconnectReasonCode::KeepAliveTimeout == res.reason_code));
    }


    #[test]
    fn test_disconnect_blocking() {
        let (client, client_to_handler_r, _) = create_new_test_client();

        let result = client.disconnect_blocking();

        // Check that the function returns Ok
        assert!(result.is_ok());

        // Check that the last message sent was a Disconnect
        let last_message = client_to_handler_r.recv_blocking().unwrap();
        assert!(matches!(last_message, Packet::Disconnect(_)));
    }

    #[test]
    fn test_disconnect_blocking_with_properties() {
        let (client, client_to_handler_r, _) = create_new_test_client();
        let properties = DisconnectProperties {
            reason_string: Some("TestString".into()),
            ..Default::default()
        };

        client.disconnect_with_properties_blocking(DisconnectReasonCode::KeepAliveTimeout, properties.clone()).unwrap();
        let disconnect = client_to_handler_r.recv_blocking().unwrap();
        assert_eq!(PacketType::Disconnect, disconnect.packet_type());

        assert!(matches!(disconnect, Packet::Disconnect(res) if properties == res.properties && DisconnectReasonCode::KeepAliveTimeout == res.reason_code));
    }

}
