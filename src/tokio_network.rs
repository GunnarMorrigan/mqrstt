use async_channel::{Receiver, Sender};

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::connect_options::ConnectOptions;
use crate::connections::tokio_stream::TokioStream;
use crate::error::ConnectionError;
use crate::packets::error::ReadBytes;
use crate::packets::reason_codes::DisconnectReasonCode;
use crate::packets::{Disconnect, Packet, PacketType};
use crate::{NetworkStatus, MqttHandler};

/// [`TokioNetwork`] reads and writes to the network based on tokios [`AsyncReadExt`] [`AsyncWriteExt`]. 
/// This way you can provide the `connect` function with a TLS and TCP stream of your choosing.
/// The most import thing to remember is that you have to provide a new stream after the previous has failed.
/// (i.e. you need to reconnect after any expected or unexpected disconnect).
pub struct TokioNetwork<S> {
    network: Option<TokioStream<S>>,

    /// Options of the current mqtt connection
    options: ConnectOptions,

    connected: Arc<AtomicBool>,
    last_network_action: Instant,
    await_pingresp: Option<Instant>,
    perform_keep_alive: bool,

    mqtt_handler: MqttHandler,

    to_network_r: Receiver<Packet>,
}

impl<S> TokioNetwork<S>
where
    S: AsyncReadExt + AsyncWriteExt + Sized + Unpin,
{
    pub fn new(
        options: ConnectOptions,
        network_to_handler_s: Sender<Packet>,
        to_network_r: Receiver<Packet>,
        connected: Arc<AtomicBool>,
    ) -> Self {

        let (mqtt_handler, apkid) = MqttHandler::new(&options);
        Self {
            network: None,

            options,

            connected,
            last_network_action: Instant::now(),
            await_pingresp: None,
            perform_keep_alive: true,

            mqtt_handler,

            to_network_r,
        }
    }


    /// There is a possibility that when a disconnect and connect (aka reconnect) occurs that there are
    /// still messages in the channel that were supposed to be send on the previous connection.
    pub async fn connect(&mut self, stream: S) -> Result<(), ConnectionError> {
        let (network, connack) = TokioStream::connect(&self.options, stream).await?;

        self.network = Some(network);

        
        // self.network_to_handler_s.send(connack).await?;

        self.last_network_action = Instant::now();
        if self.options.keep_alive_interval_s == 0 {
            self.perform_keep_alive = false;
        }
        self.clear_channels().await;
        Ok(())
    }

    async fn clear_channels(&mut self){
        match self.to_network_r.try_recv() {
            Ok(packet) => {
                match packet{
                    Packet::ConnAck(_) => todo!(),
                    _ => (),
                }
            },
            Err(_) => {},
        }
    }

    pub async fn run(&mut self) -> Result<NetworkStatus, ConnectionError> {
        if self.network.is_none() {
            return Err(ConnectionError::NoNetwork);
        }

        match self.select().await {
            Ok(NetworkStatus::Active) => Ok(NetworkStatus::Active),
            otherwise => {
                
                self.connected.store(false, Ordering::Release);
                self.network = None;
                self.await_pingresp = None;

                otherwise
            }
        }
    }

    async fn select(&mut self) -> Result<NetworkStatus, ConnectionError> {
        let TokioNetwork {
            network,
            options,
            connected,
            last_network_action,
            await_pingresp,
            perform_keep_alive,
            mqtt_handler,
            to_network_r,
        } = self;

        let sleep;
        if let Some(instant) = await_pingresp {
            sleep =
                *instant + Duration::from_secs(self.options.keep_alive_interval_s) - Instant::now();
        } else {
            sleep = *last_network_action + Duration::from_secs(self.options.keep_alive_interval_s)
                - Instant::now();
        }

        if let Some(stream) = network {
            loop {
                tokio::select! {
                    _ = stream.read_bytes() => {
                        match stream.parse_message().await {
                            Err(ReadBytes::Err(err)) => return Err(err),
                            Err(ReadBytes::InsufficientBytes(_)) => continue,
                            Ok(Packet::PingResp) => {
                                *await_pingresp = None;
                                return Ok(NetworkStatus::Active)
                            },
                            Ok(packet) => {
                                mqtt_handler.handle_incoming_packet();
                            },
                        };
                    },
                    outgoing = to_network_r.recv() => {
                        let packet = outgoing?;
                        stream.write(&packet).await?;
                        *last_network_action = Instant::now();
                        if packet.packet_type() == PacketType::Disconnect{
                            return Ok(NetworkStatus::OutgoingDisconnect);
                        }
                        return Ok(NetworkStatus::Active);
                    },
                    _ = tokio::time::sleep(sleep), if await_pingresp.is_none() && *perform_keep_alive => {
                        let packet = Packet::PingReq;
                        stream.write(&packet).await?;
                        *last_network_action = Instant::now();
                        *await_pingresp = Some(Instant::now());
                        return Ok(NetworkStatus::Active);
                    },
                    _ = tokio::time::sleep(sleep), if await_pingresp.is_some() => {
                        let disconnect = Disconnect{ reason_code: DisconnectReasonCode::KeepAliveTimeout, properties: Default::default() };
                        stream.write(&Packet::Disconnect(disconnect)).await?;
                        return Ok(NetworkStatus::NoPingResp);
                    }
                }
            }
        } else {
            Err(ConnectionError::NoNetwork)
        }
    }
}
