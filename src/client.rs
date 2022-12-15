use async_channel::{Receiver, Sender};
use bytes::Bytes;
use tracing::info;

use crate::{
    error::ClientError,
    packets::{
        packets::Packet,
        publish::{Publish, PublishProperties},
        subscribe::{Subscribe, SubscribeProperties, Subscription},
        unsubscribe::{Unsubscribe, UnsubscribeProperties, UnsubscribeTopics},
        QoS,
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

    pub async fn subscribe_options<S: Into<Subscription>>(
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
        } else {
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
        } else {
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
}
