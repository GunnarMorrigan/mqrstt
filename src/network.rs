use std::sync::Arc;

use async_channel::{Sender, Receiver};
use async_mutex::Mutex;
use bytes::BytesMut;
use futures::{future, Future};
use std::time::Instant;
use tracing::debug;

use crate::connections::AsyncMqttNetwork;
use crate::connect_options::ConnectOptions;
use crate::error::ConnectionError;
use crate::packets::packets::Packet;

pub type Incoming = Packet;
pub type Outgoing = Packet;

pub struct MqttNetwork<N>{
    network: Option<N>,
    write_buffer: BytesMut,

    /// Options of the current mqtt connection
    options: ConnectOptions,

    last_communication: Arc<Mutex<Instant>>,
    
    incoming_packet_sender: Sender<Packet>,
    // incoming_packet_receiver: Receiver<Packet>,
    // outgoing_packet_sender: Sender<Packet>,
    outgoing_packet_receiver: Receiver<Packet>,
}

impl<N> MqttNetwork<N> 
    where N: AsyncMqttNetwork{

    pub fn new(options: ConnectOptions) -> (Self, Sender<Incoming>, Receiver<Outgoing>){
        let (incoming_packet_sender, incoming_packet_receiver) = async_channel::bounded(100);
        let (outgoing_packet_sender, outgoing_packet_receiver) = async_channel::bounded(100);

        let network = Self{
            network: None,
            write_buffer: BytesMut::with_capacity(20 * 1024),

            options,

            last_communication: Arc::new(Mutex::new(data))

            incoming_packet_sender,
            // incoming_packet_receiver: incoming_packet_receiver.clone(),
            // outgoing_packet_sender: outgoing_packet_sender.clone(),
            outgoing_packet_receiver,
        };

        (network, outgoing_packet_sender, incoming_packet_receiver)
    }


    pub async fn run(&mut self) -> Result<(), ConnectionError>{
        self.run_with_shutdown_signal(future::pending()).await
    }

    pub async fn run_with_shutdown_signal(&mut self, shutdown_signal: impl Future<Output = ()>) -> Result<(), ConnectionError>{
        if self.network.is_none(){
            debug!("Creating network");

            let (network, connack) = tokio::time::timeout(
                tokio::time::Duration::from_secs(self.options.connection_timeout_s),
                N::connect(&self.options),
            ).await??;


            debug!("Succesfully created network");
            self.network = Some(network);
            self.incoming_packet_sender.send(connack).await?;
        }
        // let shutdown_process = shutdown_handler(shutdown_signal);

        if let Some(network) = self.network.as_mut(){
            
            loop{
                tokio::select! {
                    net = network.read_many(&mut self.incoming_packet_sender) => {
                        net?
                    },
                    outgoing_packet = self.outgoing_packet_receiver.recv() => {
                        let packet = outgoing_packet?;
                        tracing::trace!("Writing packet to network {:?}", packet);
                        packet.write(&mut self.write_buffer)?;
                        network.write(&mut self.write_buffer).await?;
                    },
                }
                // network.read_many(&mut self.incoming_packet_sender).await
            }
        }
        Ok(())
    }

}