use std::sync::Arc;

use async_channel::{Sender, Receiver};
use async_mutex::Mutex;
use bytes::{BytesMut};
#[cfg(feature = "smol")]
use smol::net::TcpStream;
use std::time::Instant;
use tracing::{trace};
use futures_concurrency::future::Join;

use crate::connect_options::ConnectOptions;
use crate::connections::tcp_tokio::{AsyncMqttNetworkRead, AsyncMqttNetworkWrite};
use crate::connections::{AsyncMqttNetwork};
use crate::error::ConnectionError;
use crate::packets::packets::Packet;
use crate::util::timeout;



pub type Incoming = Packet;
pub type Outgoing = Packet;

pub struct MqttNetwork<R,W>{
    network: Option<(R,W)>,

    // write_buffer: BytesMut,

    /// Options of the current mqtt connection
    options: ConnectOptions,

    last_network_action: Arc<Mutex<Instant>>,
    
    network_to_handler_s: Sender<Packet>,
    // incoming_packet_receiver: Receiver<Packet>,
    // outgoing_packet_sender: Sender<Packet>,
    to_network_r: Receiver<Packet>,
}

impl<R,W> MqttNetwork<R,W> 
    where R: AsyncMqttNetworkRead<W = W>, W: AsyncMqttNetworkWrite{

    pub fn new(options: ConnectOptions, network_to_handler_s: Sender<Incoming>, to_network_r: Receiver<Outgoing>, last_network_action: Arc<Mutex<Instant>>) -> Self{
        Self{
            network: None,

            options,

            last_network_action,

            network_to_handler_s,
            to_network_r,
        }
    }

    pub fn reset(&mut self){
        self.network = None;
    }

    // pub async fn run(&mut self) -> Result<(), ConnectionError>{
    //     self.run_with_shutdown_signal(smol::future::pending()).await
    // }

    pub async fn run_with_shutdown_signal(&mut self) -> Result<(), ConnectionError>{
        if self.network.is_none(){
            trace!("Creating network");

            let (reader, writer, connack) = timeout::timeout(R::connect(&self.options), self.options.connection_timeout_s).await??;

            trace!("Succesfully created network");
            self.network = Some((reader, writer));
            self.network_to_handler_s.send(connack).await?;
        }
        // let shutdown_process = shutdown_handler(shutdown_signal);

        if let Some((reader, writer)) = self.network.as_mut(){
            let to_network_r = self.to_network_r.clone();
            let network_to_handler_s = self.network_to_handler_s.clone();
            
            let a = async move{
                loop{
                    match reader.read_many(&network_to_handler_s).await{
                        Ok(_) => (),
                        Err(err) => {
                            return Err(err);
                        },
                    }
                }
            };
            
            let b = async move {
                loop{
                    trace!("There are {} messages in ougoing channel", to_network_r.len());
                    writer.write(&to_network_r).await?;

                    // let mut lock = self.last_network_action.lock().await;
                    // *lock = Instant::now();
                }
            };

            let res: (Result<(), ConnectionError>, Result<(), ConnectionError>) = (a,b).join().await;
        }
        Ok(())
    }

}