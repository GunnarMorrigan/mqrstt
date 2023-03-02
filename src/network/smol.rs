use async_channel::{Receiver};

use futures::FutureExt;

use std::time::{Duration, Instant};

use crate::connections::smol::Stream;

use crate::connect_options::ConnectOptions;
use crate::error::ConnectionError;
use crate::packets::error::ReadBytes;
use crate::packets::reason_codes::DisconnectReasonCode;
use crate::packets::{Disconnect, Packet, PacketType};
use crate::{MqttHandler, NetworkStatus, AsyncEventHandler};

/// [`Network`] reads and writes to the network based on tokios [`AsyncReadExt`] [`AsyncWriteExt`]. 
/// This way you can provide the `connect` function with a TLS and TCP stream of your choosing.
/// The most import thing to remember is that you have to provide a new stream after the previous has failed.
/// (i.e. you need to reconnect after any expected or unexpected disconnect).
pub struct Network<S> {
    network: Option<Stream<S>>,

    /// Options of the current mqtt connection
    options: ConnectOptions,

    last_network_action: Instant,
    await_pingresp: Option<Instant>,
    perform_keep_alive: bool,

    mqtt_handler: MqttHandler,
    outgoing_packet_buffer: Vec<Packet>,
    incoming_packet_buffer: Vec<Packet>,

    to_network_r: Receiver<Packet>,
}

impl<S> Network<S>{
    pub fn new(
        options: ConnectOptions,
        mqtt_handler: MqttHandler,
        to_network_r: Receiver<Packet>,
    ) -> Self {
        Self {
            network: None,

            options,

            last_network_action: Instant::now(),
            await_pingresp: None,
            perform_keep_alive: true,

            mqtt_handler,
            outgoing_packet_buffer: Vec::new(),
            incoming_packet_buffer: Vec::new(),

            to_network_r,
        }
    }
}

impl<S> Network<S>
    where S: smol::io::AsyncReadExt + smol::io::AsyncWriteExt + Sized + Unpin {
    
    pub async fn connect(&mut self, stream: S) -> Result<(), ConnectionError> {
        let (network, _) = Stream::connect(&self.options, stream).await?;

        self.network = Some(network);

        self.last_network_action = Instant::now();
        if self.options.keep_alive_interval_s == 0 {
            self.perform_keep_alive = false;
        }
        Ok(())
    }

    pub async fn poll<H>(&mut self, handler: &mut H) -> Result<NetworkStatus, ConnectionError>  where H: AsyncEventHandler {
        if self.network.is_none() {
            return Err(ConnectionError::NoNetwork);
        }

        match self.smol_select(handler).await {
            Ok(NetworkStatus::Active) => Ok(NetworkStatus::Active),
            otherwise => {
                self.network = None;
                self.await_pingresp = None;
                self.outgoing_packet_buffer.clear();
                self.incoming_packet_buffer.clear();

                otherwise
            }
        }
    }

    async fn smol_select<H>(&mut self, handler: &mut H) -> Result<NetworkStatus, ConnectionError> where H: AsyncEventHandler {
        let Network {
            network,
            options: _,
            last_network_action,
            await_pingresp,
            perform_keep_alive,
            mqtt_handler,
            outgoing_packet_buffer,
            incoming_packet_buffer,
            to_network_r,
        } = self;

        let sleep;
        if !(*perform_keep_alive) {
            sleep = Duration::new(3600, 0);
        } else if let Some(instant) = await_pingresp {
            sleep =
                *instant + Duration::from_secs(self.options.keep_alive_interval_s) - Instant::now();
        } else {
            sleep = *last_network_action + Duration::from_secs(self.options.keep_alive_interval_s)
                - Instant::now();
        }

        if let Some(stream) = network {
            futures::select! {
                _ = stream.read_bytes().fuse() => {
                    match stream.parse_messages(incoming_packet_buffer).await {
                        Err(ReadBytes::Err(err)) => return Err(err),
                        Err(ReadBytes::InsufficientBytes(_)) => return Ok(NetworkStatus::Active),
                        Ok(_) => (),
                    }
                    
                    for packet in incoming_packet_buffer.drain(0..){
                        use Packet::*;
                        match packet{
                            PingResp => {
                                handler.handle(packet).await;
                                *await_pingresp = None;
                            },
                            Disconnect(_) => {
                                handler.handle(packet).await;
                                return Ok(NetworkStatus::IncomingDisconnect);
                            }
                            packet => {
                                if mqtt_handler.handle_incoming_packet(&packet, outgoing_packet_buffer).await?{
                                    handler.handle(packet).await;
                                }
                            }
                        }
                    }
                    
                    stream.write_all(outgoing_packet_buffer).await?;
                    *last_network_action = Instant::now();
                    
                    Ok(NetworkStatus::Active)
                },
                outgoing = to_network_r.recv().fuse() => {
                    let packet = outgoing?;
                    stream.write(&packet).await?;
                    let mut disconnect = false;

                    if packet.packet_type() == PacketType::Disconnect{
                        disconnect = true;
                    }

                    mqtt_handler.handle_outgoing_packet(packet).await?;
                    *last_network_action = Instant::now();


                    if disconnect{
                        Ok(NetworkStatus::OutgoingDisconnect)
                    }
                    else{
                        Ok(NetworkStatus::Active)
                    }
                },
                _ = smol::Timer::after(sleep).fuse() => {
                    if await_pingresp.is_none() && *perform_keep_alive{
                        let packet = Packet::PingReq;
                        stream.write(&packet).await?;
                        *last_network_action = Instant::now();
                        *await_pingresp = Some(Instant::now());
                        Ok(NetworkStatus::Active)
                    }
                    else{
                        let disconnect = Disconnect{ reason_code: DisconnectReasonCode::KeepAliveTimeout, properties: Default::default() };
                        stream.write(&Packet::Disconnect(disconnect)).await?;
                        Ok(NetworkStatus::NoPingResp)
                    }
                },
            }
        } else {
            Err(ConnectionError::NoNetwork)
        }
    }
}