use async_channel::Receiver;

use std::time::{Duration, Instant};

use crate::connect_options::ConnectOptions;
use crate::error::ConnectionError;
use crate::packets::error::ReadBytes;
use crate::packets::reason_codes::DisconnectReasonCode;
use crate::packets::{Disconnect, Packet, PacketType};
use crate::{AsyncEventHandler, MqttHandler};

use super::stream::Stream;
use super::NetworkStatus;

/// [`Network`] reads and writes to the network based on tokios [`AsyncReadExt`] [`AsyncWriteExt`].
/// This way you can provide the `connect` function with a TLS and TCP stream of your choosing.
/// The most import thing to remember is that you have to provide a new stream after the previous has failed.
/// (i.e. you need to reconnect after any expected or unexpected disconnect).
pub struct Network<S> {
    network: Option<Stream<S>>,

    /// Options of the current mqtt connection
    keep_alive_interval_s: u64,
    options: ConnectOptions,

    last_network_action: Instant,
    await_pingresp: Option<Instant>,
    perform_keep_alive: bool,

    mqtt_handler: MqttHandler,
    outgoing_packet_buffer: Vec<Packet>,
    incoming_packet_buffer: Vec<Packet>,

    to_network_r: Receiver<Packet>,
}

impl<S> Network<S> {
    pub fn new(options: ConnectOptions, mqtt_handler: MqttHandler, to_network_r: Receiver<Packet>) -> Self {
        Self {
            network: None,

            keep_alive_interval_s: options.keep_alive_interval_s,
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

/// Tokio impl
#[cfg(feature = "tokio")]
impl<S> Network<S>
where
    S: tokio::io::AsyncReadExt + tokio::io::AsyncWriteExt + Sized + Unpin,
{
    /// Initializes an MQTT connection with the provided configuration an stream
    pub async fn connect<H>(&mut self, stream: S, handler: &mut H) -> Result<(), ConnectionError>
    where
        H: AsyncEventHandler,
    {
        let (network, connack) = Stream::connect(&self.options, stream).await?;
        self.last_network_action = Instant::now();

        self.network = Some(network);

        if let Some(keep_alive_interval) = connack.connack_properties.server_keep_alive {
            self.keep_alive_interval_s = keep_alive_interval as u64;
        }
        if self.keep_alive_interval_s == 0 {
            self.perform_keep_alive = false;
        }

        let packet = Packet::ConnAck(connack);

        self.mqtt_handler.handle_incoming_packet(&packet, &mut self.outgoing_packet_buffer)?;
        handler.handle(packet).await;

        Ok(())
    }

    /// A single call to poll will perform one of three tasks:
    /// - Read from the stream and parse the bytes to packets for the user to handle
    /// - Write user packets to stream
    /// - Perform keepalive if necessary
    ///
    /// This function can produce an indication of the state of the network or an error.
    /// When the network is still active (i.e. stream is not closed and no disconnect packet has been processed) the network will return [`NetworkStatus::Active`]
    ///
    /// In all other cases the network is unusable anymore.
    /// The stream will be dropped and the internal buffers will be cleared.
    pub async fn poll<H>(&mut self, handler: &mut H) -> Result<NetworkStatus, ConnectionError>
    where
        H: AsyncEventHandler,
    {
        if self.network.is_none() {
            return Err(ConnectionError::NoNetwork);
        }

        match self.tokio_select(handler).await {
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

    async fn tokio_select<H>(&mut self, handler: &mut H) -> Result<NetworkStatus, ConnectionError>
    where
        H: AsyncEventHandler,
    {
        let Network {
            network,
            keep_alive_interval_s,
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
        if let Some(instant) = await_pingresp {
            sleep = *instant + Duration::from_secs(*keep_alive_interval_s) - Instant::now();
        } else {
            sleep = *last_network_action + Duration::from_secs(*keep_alive_interval_s) - Instant::now();
        }

        if let Some(stream) = network {
            tokio::select! {
                res = stream.read_bytes() => {
                    res?;
                    let packet = match stream.parse_message().await {
                        Err(ReadBytes::Err(err)) => return Err(err),
                        Err(ReadBytes::InsufficientBytes(_)) => return Ok(NetworkStatus::Active),
                        Ok(packet) => packet,
                    };

                    match packet{
                        Packet::PingResp => {
                            handler.handle(packet).await;
                            *await_pingresp = None;
                        },
                        Packet::Disconnect(_) => {
                            handler.handle(packet).await;
                            return Ok(NetworkStatus::IncomingDisconnect);
                        }
                        packet => {
                            if mqtt_handler.handle_incoming_packet(&packet, outgoing_packet_buffer)?{
                                handler.handle(packet).await;
                            }
                        }
                    }

                    stream.write_all(outgoing_packet_buffer).await?;
                    *last_network_action = Instant::now();

                    Ok(NetworkStatus::Active)
                },
                outgoing = to_network_r.recv() => {
                    let packet = outgoing?;
                    stream.write(&packet).await?;
                    let mut disconnect = false;

                    if packet.packet_type() == PacketType::Disconnect{
                        disconnect = true;
                    }

                    mqtt_handler.handle_outgoing_packet(packet)?;
                    *last_network_action = Instant::now();


                    if disconnect{
                        Ok(NetworkStatus::OutgoingDisconnect)
                    }
                    else{
                        Ok(NetworkStatus::Active)
                    }
                },
                _ = tokio::time::sleep(sleep), if await_pingresp.is_none() && *perform_keep_alive => {
                    let packet = Packet::PingReq;
                    stream.write(&packet).await?;
                    *last_network_action = Instant::now();
                    *await_pingresp = Some(Instant::now());
                    Ok(NetworkStatus::Active)
                },
                _ = tokio::time::sleep(sleep), if await_pingresp.is_some() => {
                    let disconnect = Disconnect{ reason_code: DisconnectReasonCode::KeepAliveTimeout, properties: Default::default() };
                    stream.write(&Packet::Disconnect(disconnect)).await?;
                    Ok(NetworkStatus::NoPingResp)
                }
            }
        } else {
            Err(ConnectionError::NoNetwork)
        }
    }
}
