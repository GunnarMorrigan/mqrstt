use std::sync::Arc;

use async_channel::{Sender, Receiver};
use async_mutex::Mutex;
use bytes::{BytesMut};
#[cfg(feature = "smol")]
use smol::net::TcpStream;
use std::time::Instant;
use tracing::debug;
use futures_concurrency::future::Join;

use crate::connect_options::ConnectOptions;
use crate::connections::{AsyncMqttNetwork};
use crate::error::ConnectionError;
use crate::packets::packets::Packet;
use crate::util::timeout;



pub type Incoming = Packet;
pub type Outgoing = Packet;

pub struct MqttNetwork<N>{
    network: Option<N>,
    // writer: Option<S>,

    // write_buffer: BytesMut,

    /// Options of the current mqtt connection
    options: ConnectOptions,

    last_network_action: Arc<Mutex<Instant>>,
    
    incoming_packet_sender: Sender<Packet>,
    // incoming_packet_receiver: Receiver<Packet>,
    // outgoing_packet_sender: Sender<Packet>,
    outgoing_packet_receiver: Receiver<Packet>,
}

impl<N> MqttNetwork<N> 
    where N: AsyncMqttNetwork {

    pub fn new(options: ConnectOptions) -> (Self, Sender<Incoming>, Receiver<Outgoing>, Arc<Mutex<Instant>>){
        let (incoming_packet_sender, incoming_packet_receiver) = async_channel::bounded(100);
        let (outgoing_packet_sender, outgoing_packet_receiver) = async_channel::bounded(100);

        let last_network_action = Arc::new(Mutex::new(Instant::now()));

        let network = Self{
            network: None,
            // reader: None,
            // writer: None,

            options,

            last_network_action: last_network_action.clone(),

            incoming_packet_sender,
            outgoing_packet_receiver,
        };

        (network, outgoing_packet_sender, incoming_packet_receiver, last_network_action)
    }

    pub fn reset(&mut self){
        self.reader = None;
    }

    // pub async fn run(&mut self) -> Result<(), ConnectionError>{
    //     self.run_with_shutdown_signal(smol::future::pending()).await
    // }

    pub async fn run_with_shutdown_signal(&mut self, shutdown_signal: impl core::future::Future<Output = ()>) -> Result<(), ConnectionError>{
        if self.reader.is_none(){
            debug!("Creating network");

            let (network, connack) = timeout::timeout(N::connect(&self.options), self.options.connection_timeout_s).await??;

            debug!("Succesfully created network");
            self.network = Some(network);
            // self.reader = Some(reader);
            // self.writer = Some(writer);
            self.incoming_packet_sender.send(connack).await?;
        }
        // let shutdown_process = shutdown_handler(shutdown_signal);

        if let (Some(reader),Some(writer)) = (self.reader.as_mut(), self.writer.as_mut()){
            let incoming_packet_sender = self.incoming_packet_sender.clone();
            let outgoing_packet_receiver = self.outgoing_packet_receiver.clone();
            
            let a = async move{
                loop{
                    match reader.read_many(&incoming_packet_sender).await{
                        Ok(_) => (),
                        Err(err) => {
                            // self.reset();
                            return Err(err);
                        },
                    }
                }
            };
            
            let b = async move {
                let mut write_buffer = BytesMut::with_capacity(20 * 1024);
                loop{
                    let outgoing_packet = outgoing_packet_receiver.recv().await;
                    let packet = outgoing_packet?;
                    tracing::trace!("Writing packet to network {:?}", packet);
                    packet.write(&mut write_buffer)?;
                    writer.write(&mut write_buffer).await?;
                    // let mut lock = self.last_network_action.lock().await;
                    // *lock = Instant::now();
                }
            };

            let res: (Result<(), ConnectionError>, Result<(), ConnectionError>) = (a,b).join().await;
        }
        Ok(())
    }

}