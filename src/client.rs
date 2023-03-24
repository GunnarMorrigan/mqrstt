use async_channel::{Receiver, Sender};
use bytes::Bytes;
use tracing::info;

use crate::{
    error::ClientError,
    packets::{
        reason_codes::DisconnectReasonCode,
        Packet, QoS, {Disconnect, DisconnectProperties}, {Publish, PublishProperties}, {Subscribe, SubscribeProperties, Subscription}, {Unsubscribe, UnsubscribeProperties, UnsubscribeTopics},
    },
};

#[derive(Debug, Clone)]
pub struct MqttClient {
    /// Provides this client with an available packet id or waits on it.
    available_packet_ids: Receiver<u16>,

    /// Sends Publish, Subscribe, Unsubscribe to the event handler to handle later.
    to_network_s: Sender<Packet>,
}

/// Async functions to perform MQTT operations
#[cfg(any(feature = "tokio", feature = "smol"))]
impl MqttClient {
    pub fn new(available_packet_ids: Receiver<u16>, to_network_s: Sender<Packet>) -> Self {
        Self { available_packet_ids, to_network_s }
    }

    pub async fn subscribe<A: Into<Subscription>>(&self, into_subscribtions: A) -> Result<(), ClientError> {
        let pkid = self.available_packet_ids.recv().await.map_err(|_| ClientError::NoNetwork)?;
        let subscription: Subscription = into_subscribtions.into();
        let sub = Subscribe::new(pkid, subscription.0);
        self.to_network_s.send(Packet::Subscribe(sub)).await.map_err(|_| ClientError::NoNetwork)?;
        Ok(())
    }

    pub async fn subscribe_with_properties<S: Into<Subscription>>(&self, properties: SubscribeProperties, into_sub: S) -> Result<(), ClientError> {
        let pkid = self.available_packet_ids.recv().await.map_err(|_| ClientError::NoNetwork)?;
        let sub = Subscribe {
            packet_identifier: pkid,
            properties,
            topics: into_sub.into().0,
        };
        self.to_network_s.send(Packet::Subscribe(sub)).await.map_err(|_| ClientError::NoNetwork)?;
        Ok(())
    }

    pub async fn publish<P: Into<Bytes>>(&self, qos: QoS, retain: bool, topic: String, payload: P) -> Result<(), ClientError> {
        let pkid = match qos {
            QoS::AtMostOnce => None,
            _ => Some(self.available_packet_ids.recv().await.map_err(|_| ClientError::NoNetwork)?),
        };
        info!("Published message with ID: {:?}", pkid);
        let publish = Publish {
            dup: false,
            qos,
            retain,
            topic,
            packet_identifier: pkid,
            publish_properties: PublishProperties::default(),
            payload: payload.into(),
        };
        self.to_network_s.send(Packet::Publish(publish)).await.map_err(|_| ClientError::NoNetwork)?;
        info!("Published message into handler_packet_sender: len {}", self.to_network_s.len());
        Ok(())
    }

    pub async fn publish_with_properties<P: Into<Bytes>>(&self, qos: QoS, retain: bool, topic: String, payload: P, properties: PublishProperties) -> Result<(), ClientError> {
        let pkid = match qos {
            QoS::AtMostOnce => None,
            _ => Some(self.available_packet_ids.recv().await.map_err(|_| ClientError::NoNetwork)?),
        };
        let publish = Publish {
            dup: false,
            qos,
            retain,
            topic,
            packet_identifier: pkid,
            publish_properties: properties,
            payload: payload.into(),
        };
        self.to_network_s.send(Packet::Publish(publish)).await.map_err(|_| ClientError::NoNetwork)?;
        Ok(())
    }

    pub async fn unsubscribe<T: Into<UnsubscribeTopics>>(&self, into_topics: T) -> Result<(), ClientError> {
        let pkid = self.available_packet_ids.recv().await.map_err(|_| ClientError::NoNetwork)?;
        let unsub = Unsubscribe {
            packet_identifier: pkid,
            properties: UnsubscribeProperties::default(),
            topics: into_topics.into().0,
        };
        self.to_network_s.send(Packet::Unsubscribe(unsub)).await.map_err(|_| ClientError::NoNetwork)?;
        Ok(())
    }

    pub async fn unsubscribe_with_properties<T: Into<UnsubscribeTopics>>(&self, into_topics: T, properties: UnsubscribeProperties) -> Result<(), ClientError> {
        let pkid = self.available_packet_ids.recv().await.map_err(|_| ClientError::NoNetwork)?;
        let unsub = Unsubscribe {
            packet_identifier: pkid,
            properties,
            topics: into_topics.into().0,
        };
        self.to_network_s.send(Packet::Unsubscribe(unsub)).await.map_err(|_| ClientError::NoNetwork)?;
        Ok(())
    }

    pub async fn disconnect(&self) -> Result<(), ClientError> {
        let disconnect = Disconnect {
            reason_code: DisconnectReasonCode::NormalDisconnection,
            properties: DisconnectProperties::default(),
        };
        self.to_network_s.send(Packet::Disconnect(disconnect)).await.map_err(|_| ClientError::NoNetwork)?;
        Ok(())
    }

    pub async fn disconnect_with_properties(&self, reason_code: DisconnectReasonCode, properties: DisconnectProperties) -> Result<(), ClientError> {
        let disconnect = Disconnect { reason_code, properties };
        self.to_network_s.send(Packet::Disconnect(disconnect)).await.map_err(|_| ClientError::NoNetwork)?;
        Ok(())
    }
}

/// Sync functions to perform MQTT operations
#[cfg(feature = "sync")]
impl MqttClient {
    pub fn subscribe_blocking<A: Into<Subscription>>(&self, into_subscribtions: A) -> Result<(), ClientError> {
        let pkid = self.available_packet_ids.recv_blocking().map_err(|_| ClientError::NoNetwork)?;
        let subscription: Subscription = into_subscribtions.into();
        let sub = Subscribe::new(pkid, subscription.0);
        self.to_network_s.send_blocking(Packet::Subscribe(sub)).map_err(|_| ClientError::NoNetwork)?;
        Ok(())
    }

    pub fn subscribe_with_properties_blocking<S: Into<Subscription>>(&self, properties: SubscribeProperties, into_sub: S) -> Result<(), ClientError> {
        let pkid = self.available_packet_ids.recv_blocking().map_err(|_| ClientError::NoNetwork)?;
        let sub = Subscribe {
            packet_identifier: pkid,
            properties,
            topics: into_sub.into().0,
        };
        self.to_network_s.send_blocking(Packet::Subscribe(sub)).map_err(|_| ClientError::NoNetwork)?;
        Ok(())
    }

    pub fn publish_blocking<P: Into<Bytes>>(&self, qos: QoS, retain: bool, topic: String, payload: P) -> Result<(), ClientError> {
        let pkid = match qos {
            QoS::AtMostOnce => None,
            _ => Some(self.available_packet_ids.recv_blocking().map_err(|_| ClientError::NoNetwork)?),
        };
        info!("Published message with ID: {:?}", pkid);
        let publish = Publish {
            dup: false,
            qos,
            retain,
            topic,
            packet_identifier: pkid,
            publish_properties: PublishProperties::default(),
            payload: payload.into(),
        };
        self.to_network_s.send_blocking(Packet::Publish(publish)).map_err(|_| ClientError::NoNetwork)?;
        info!("Published message into handler_packet_sender: len {}", self.to_network_s.len());
        Ok(())
    }

    pub fn publish_with_properties_blocking<P: Into<Bytes>>(&self, qos: QoS, retain: bool, topic: String, payload: P, properties: PublishProperties) -> Result<(), ClientError> {
        let pkid = match qos {
            QoS::AtMostOnce => None,
            _ => Some(self.available_packet_ids.recv_blocking().map_err(|_| ClientError::NoNetwork)?),
        };
        let publish = Publish {
            dup: false,
            qos,
            retain,
            topic,
            packet_identifier: pkid,
            publish_properties: properties,
            payload: payload.into(),
        };
        self.to_network_s.send_blocking(Packet::Publish(publish)).map_err(|_| ClientError::NoNetwork)?;
        Ok(())
    }

    pub fn unsubscribe_blocking<T: Into<UnsubscribeTopics>>(&self, into_topics: T) -> Result<(), ClientError> {
        let pkid = self.available_packet_ids.recv_blocking().map_err(|_| ClientError::NoNetwork)?;
        let unsub = Unsubscribe {
            packet_identifier: pkid,
            properties: UnsubscribeProperties::default(),
            topics: into_topics.into().0,
        };
        self.to_network_s.send_blocking(Packet::Unsubscribe(unsub)).map_err(|_| ClientError::NoNetwork)?;
        Ok(())
    }

    pub fn unsubscribe_with_properties_blocking<T: Into<UnsubscribeTopics>>(&self, into_topics: T, properties: UnsubscribeProperties) -> Result<(), ClientError> {
        let pkid = self.available_packet_ids.recv_blocking().map_err(|_| ClientError::NoNetwork)?;
        let unsub = Unsubscribe {
            packet_identifier: pkid,
            properties,
            topics: into_topics.into().0,
        };
        self.to_network_s.send_blocking(Packet::Unsubscribe(unsub)).map_err(|_| ClientError::NoNetwork)?;
        Ok(())
    }

    pub fn disconnect_blocking(&self) -> Result<(), ClientError> {
        let disconnect = Disconnect {
            reason_code: DisconnectReasonCode::NormalDisconnection,
            properties: DisconnectProperties::default(),
        };
        self.to_network_s.send_blocking(Packet::Disconnect(disconnect)).map_err(|_| ClientError::NoNetwork)?;
        Ok(())
    }

    pub fn disconnect_with_properties_blocking(&self, reason_code: DisconnectReasonCode, properties: DisconnectProperties) -> Result<(), ClientError> {
        let disconnect = Disconnect { reason_code, properties };
        self.to_network_s.send_blocking(Packet::Disconnect(disconnect)).map_err(|_| ClientError::NoNetwork)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use async_channel::Receiver;

    use crate::packets::{reason_codes::DisconnectReasonCode, DisconnectProperties, Packet, PacketType, UnsubscribeProperties};

    use super::MqttClient;

    fn create_new_test_client() -> (MqttClient, Receiver<Packet>, Receiver<Packet>) {
        let (s, r) = async_channel::bounded(100);

        for i in 1..=100 {
            s.send_blocking(i).unwrap();
        }

        let (client_to_handler_s, client_to_handler_r) = async_channel::bounded(100);
        let (_, to_network_r) = async_channel::bounded(100);

        let client = MqttClient::new(r, client_to_handler_s);

        (client, client_to_handler_r, to_network_r)
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
