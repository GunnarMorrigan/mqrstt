use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use async_channel::{Receiver, Sender};
use async_mutex::Mutex;

use futures_concurrency::future::Join;
#[cfg(feature = "smol")]
use smol::net::TcpStream;
use std::time::Instant;
use tracing::trace;

use crate::connect_options::ConnectOptions;
use crate::connections::tcp_tokio::{AsyncMqttNetworkRead, AsyncMqttNetworkWrite};
use crate::error::ConnectionError;
use crate::packets::packets::Packet;
use crate::util::timeout;

pub type Incoming = Packet;
pub type Outgoing = Packet;

pub struct MqttNetwork<R, W> {
    network: Option<(R, W)>,

    // write_buffer: BytesMut,
    /// Options of the current mqtt connection
    options: ConnectOptions,

    last_network_action: Arc<Mutex<Instant>>,

    network_to_handler_s: Sender<Packet>,
    // incoming_packet_receiver: Receiver<Packet>,
    // outgoing_packet_sender: Sender<Packet>,
    to_network_r: Receiver<Packet>,
}

impl<R, W> MqttNetwork<R, W>
where
    R: AsyncMqttNetworkRead<W = W>,
    W: AsyncMqttNetworkWrite,
{
    pub fn new(
        options: ConnectOptions,
        network_to_handler_s: Sender<Incoming>,
        to_network_r: Receiver<Outgoing>,
        last_network_action: Arc<Mutex<Instant>>,
    ) -> Self {
        Self {
            network: None,

            options,

            last_network_action,

            network_to_handler_s,
            to_network_r,
        }
    }

    pub fn reset(&mut self) {
        self.network = None;
    }

    pub async fn run(&mut self) -> Result<(), ConnectionError> {
        if self.network.is_none() {
            trace!("Creating network");

            let (reader, writer, connack) =
                timeout::timeout(R::connect(&self.options), self.options.connection_timeout_s)
                    .await??;

            trace!("Succesfully created network");
            self.network = Some((reader, writer));
            self.network_to_handler_s.send(connack).await?;
        }
        let MqttNetwork {
            network,
            options: _,
            last_network_action,
            network_to_handler_s,
            to_network_r,
        } = self;


        if let Some((reader, writer)) = network {
            let disconnect = AtomicBool::new(false);

            let incoming = async {
                loop {
                    let local_disconnect = reader.read_direct(network_to_handler_s).await?;
                    *(last_network_action.lock().await) = std::time::Instant::now();
                    if local_disconnect{
                        disconnect.store(true, Ordering::Release);
                        return Ok(());
                    }
                    else if disconnect.load(Ordering::Acquire){
                        return Ok(());
                    }
                }
            };

            let outgoing = async {
                loop {
                    let local_disconnect = writer.write(to_network_r).await?;
                    *(last_network_action.lock().await) = std::time::Instant::now();
                    if local_disconnect{
                        disconnect.store(true, Ordering::Release);
                        return Ok(());
                    }
                    else if disconnect.load(Ordering::Acquire){
                        return Ok(());
                    }
                }
            };

            let res: (Result<(), ConnectionError>, Result<(), ConnectionError>) = (incoming, outgoing).join().await;
            res.0?;
            res.1
        }
        else{
            Ok(())
        }
    }
}
