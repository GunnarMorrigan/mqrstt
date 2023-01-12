use async_channel::{Receiver, Sender};
use bytes::Bytes;
use tracing::info;

use crate::{
    error::ClientError,
    packets::{
        reason_codes::DisconnectReasonCode,
        Packet, QoS, {Disconnect, DisconnectProperties}, {Publish, PublishProperties},
        {Subscribe, SubscribeProperties, Subscription},
        {Unsubscribe, UnsubscribeProperties, UnsubscribeTopics},
    },
};

#[derive(Debug, Clone)]
pub struct AsyncClient {
    // Provides this client with an available packet id or waits on it.
    available_packet_ids: Receiver<u16>,

    // Sends Publish, Subscribe, Unsubscribe to the event handler to handle later.
    client_to_handler_s: Sender<Packet>,

    // Sends Publish with QoS 0
    to_network_s: Sender<Packet>,
}

impl AsyncClient {
    pub fn new(
        available_packet_ids: Receiver<u16>,
        client_to_handler_s: Sender<Packet>,
        to_network_s: Sender<Packet>,
    ) -> Self {
        Self {
            available_packet_ids,
            client_to_handler_s,
            to_network_s,
        }
    }

    pub async fn subscribe<A: Into<Subscription>>(
        &self,
        into_subscribtions: A,
    ) -> Result<(), ClientError> {
        let pkid = self
            .available_packet_ids
            .recv()
            .await
            .map_err(|_| ClientError::NoHandler)?;
        let subscription: Subscription = into_subscribtions.into();
        let sub = Subscribe::new(pkid, subscription.0);
        self.client_to_handler_s
            .send(Packet::Subscribe(sub))
            .await
            .map_err(|_| ClientError::NoHandler)?;
        Ok(())
    }

    pub async fn subscribe_with_properties<S: Into<Subscription>>(
        &self,
        properties: SubscribeProperties,
        into_sub: S,
    ) -> Result<(), ClientError> {
        let pkid = self
            .available_packet_ids
            .recv()
            .await
            .map_err(|_| ClientError::NoHandler)?;
        let sub = Subscribe {
            packet_identifier: pkid,
            properties,
            topics: into_sub.into().0,
        };
        self.client_to_handler_s
            .send(Packet::Subscribe(sub))
            .await
            .map_err(|_| ClientError::NoHandler)?;
        Ok(())
    }

    pub async fn publish<P: Into<Bytes>>(
        &self,
        qos: QoS,
        retain: bool,
        topic: String,
        payload: P,
    ) -> Result<(), ClientError> {
        let pkid = match qos {
            QoS::AtMostOnce => None,
            _ => Some(
                self.available_packet_ids
                    .recv()
                    .await
                    .map_err(|_| ClientError::NoHandler)?,
            ),
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
        if qos == QoS::AtMostOnce {
            self.to_network_s
                .send(Packet::Publish(publish))
                .await
                .map_err(|_| ClientError::NoHandler)?;
            info!(
                "Published message into network_packet_sender. len {}",
                self.to_network_s.len()
            );
        }
        else {
            self.client_to_handler_s
                .send(Packet::Publish(publish))
                .await
                .map_err(|_| ClientError::NoHandler)?;
            info!(
                "Published message into handler_packet_sender: len {}",
                self.client_to_handler_s.len()
            );
        }
        Ok(())
    }

    pub async fn publish_with_properties<P: Into<Bytes>>(
        &self,
        qos: QoS,
        retain: bool,
        topic: String,
        payload: P,
        properties: PublishProperties,
    ) -> Result<(), ClientError> {
        let pkid = match qos {
            QoS::AtMostOnce => None,
            _ => Some(
                self.available_packet_ids
                    .recv()
                    .await
                    .map_err(|_| ClientError::NoHandler)?,
            ),
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
        if qos == QoS::AtMostOnce {
            self.to_network_s
                .send(Packet::Publish(publish))
                .await
                .map_err(|_| ClientError::NoHandler)?;
        }
        else {
            self.client_to_handler_s
                .send(Packet::Publish(publish))
                .await
                .map_err(|_| ClientError::NoHandler)?;
        }
        Ok(())
    }

    pub async fn unsubscribe<T: Into<UnsubscribeTopics>>(
        &self,
        into_topics: T,
    ) -> Result<(), ClientError> {
        let pkid = self
            .available_packet_ids
            .recv()
            .await
            .map_err(|_| ClientError::NoHandler)?;
        let unsub = Unsubscribe {
            packet_identifier: pkid,
            properties: UnsubscribeProperties::default(),
            topics: into_topics.into().0,
        };
        self.client_to_handler_s
            .send(Packet::Unsubscribe(unsub))
            .await
            .map_err(|_| ClientError::NoHandler)?;
        Ok(())
    }

    pub async fn unsubscribe_with_properties<T: Into<UnsubscribeTopics>>(
        &self,
        into_topics: T,
        properties: UnsubscribeProperties,
    ) -> Result<(), ClientError> {
        let pkid = self
            .available_packet_ids
            .recv()
            .await
            .map_err(|_| ClientError::NoHandler)?;
        let unsub = Unsubscribe {
            packet_identifier: pkid,
            properties,
            topics: into_topics.into().0,
        };
        self.client_to_handler_s
            .send(Packet::Unsubscribe(unsub))
            .await
            .map_err(|_| ClientError::NoHandler)?;
        Ok(())
    }

    pub async fn disconnect(&self) -> Result<(), ClientError> {
        let disconnect = Disconnect {
            reason_code: DisconnectReasonCode::NormalDisconnection,
            properties: DisconnectProperties::default(),
        };
        self.client_to_handler_s
            .send(Packet::Disconnect(disconnect))
            .await
            .map_err(|_| ClientError::NoHandler)?;
        Ok(())
    }

    pub async fn disconnect_with_properties(
        &self,
        reason_code: DisconnectReasonCode,
        properties: DisconnectProperties,
    ) -> Result<(), ClientError> {
        let disconnect = Disconnect {
            reason_code,
            properties,
        };
        self.client_to_handler_s
            .send(Packet::Disconnect(disconnect))
            .await
            .map_err(|_| ClientError::NoHandler)?;
        Ok(())
    }
}


#[cfg(test)]
mod tests{
    use async_channel::Receiver;

    use crate::packets::{Packet, reason_codes::DisconnectReasonCode, PacketType, DisconnectProperties, UnsubscribeProperties};

    use super::AsyncClient;

    fn create_new_test_client() -> (AsyncClient, Receiver<Packet>, Receiver<Packet>) {
        let (s, r) = async_channel::bounded(100);

        for i in 1..=100{
            s.send_blocking(i).unwrap();
        }

        let (client_to_handler_s, client_to_handler_r) = async_channel::bounded(100);
        let (to_network_s, to_network_r) = async_channel::bounded(100);

        let client = AsyncClient::new(r, client_to_handler_s, to_network_s);

        (client, client_to_handler_r, to_network_r)
    }

    #[tokio::test]
    async fn unsubscribe_with_properties_test() {
        let (client, client_to_handler_r, _) = create_new_test_client();

        let mut prop = UnsubscribeProperties::default();
        prop.user_properties = vec![("A".to_string(), "B".to_string())];

        client.unsubscribe_with_properties("Topic", prop.clone()).await.unwrap();
        let unsubscribe = client_to_handler_r.recv().await.unwrap();

        assert_eq!(PacketType::Unsubscribe, unsubscribe.packet_type());

        if let Packet::Unsubscribe(unsub) = unsubscribe{
            assert_eq!(1, unsub.packet_identifier);
            assert_eq!(vec!["Topic"], unsub.topics);
            assert_eq!(prop, unsub.properties);
        }
        else{
            // To make sure we did the if branch
            unreachable!();
        }

    }

    #[tokio::test]
    async fn disconnect_test(){
        let (client, client_to_handler_r, _) = create_new_test_client();
        client.disconnect().await.unwrap();
        let disconnect = client_to_handler_r.recv().await.unwrap();
        assert_eq!(PacketType::Disconnect, disconnect.packet_type());

        if let Packet::Disconnect(res) = disconnect{
            assert_eq!(DisconnectReasonCode::NormalDisconnection, res.reason_code);
            assert_eq!(DisconnectProperties::default(), res.properties);
        }
        else{
            // To make sure we did the if branch
            unreachable!();
        }
    }

    #[tokio::test]
    async fn disconnect_with_properties_test(){
        let (client, client_to_handler_r, _) = create_new_test_client();
        client.disconnect_with_properties(DisconnectReasonCode::KeepAliveTimeout, Default::default()).await.unwrap();
        let disconnect = client_to_handler_r.recv().await.unwrap();
        assert_eq!(PacketType::Disconnect, disconnect.packet_type());

        if let Packet::Disconnect(res) = disconnect{
            assert_eq!(DisconnectReasonCode::KeepAliveTimeout, res.reason_code);
            assert_eq!(DisconnectProperties::default(), res.properties);
        }
        else{
            // To make sure we did the if branch
            unreachable!();
        }
    }

    #[tokio::test]
    async fn disconnect_with_properties_test2(){
        let (client, client_to_handler_r, _) = create_new_test_client();
        let mut properties = DisconnectProperties::default();
        properties.reason_string = Some("TestString".to_string());

        client.disconnect_with_properties(DisconnectReasonCode::KeepAliveTimeout, properties.clone()).await.unwrap();
        let disconnect = client_to_handler_r.recv().await.unwrap();
        assert_eq!(PacketType::Disconnect, disconnect.packet_type());

        if let Packet::Disconnect(res) = disconnect{
            assert_eq!(DisconnectReasonCode::KeepAliveTimeout, res.reason_code);
            assert_eq!(properties, res.properties);
        }
        else{
            // To make sure we did the if branch
            unreachable!();
        }
    }
}