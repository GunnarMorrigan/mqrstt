use async_channel::Receiver;
use tokio::task::JoinSet;

use std::time::{Duration, Instant};

use crate::connect_options::ConnectOptions;
use crate::error::ConnectionError;
use crate::packets::error::ReadBytes;
use crate::packets::reason_codes::DisconnectReasonCode;
use crate::packets::{Disconnect, Packet, PacketType};
use crate::{AsyncEventHandler, StateHandler};

use super::stream::Stream;
use super::NetworkStatus;
use super::stream::read_half::ReadStream;
use super::stream::write_half::WriteStream;

/// [`Network`] reads and writes to the network based on tokios [`AsyncReadExt`] [`AsyncWriteExt`].
/// This way you can provide the `connect` function with a TLS and TCP stream of your choosing.
/// The most import thing to remember is that you have to provide a new stream after the previous has failed.
/// (i.e. you need to reconnect after any expected or unexpected disconnect).
pub struct Network<S> {
    network: Option<(ReadStream<S>, WriteStream<S>)>,

    /// Options of the current mqtt connection
    keep_alive_interval_s: u64,
    options: ConnectOptions,

    last_network_action: Instant,
    await_pingresp: Option<Instant>,
    perform_keep_alive: bool,

    state_handler: StateHandler,
    outgoing_packet_buffer: Vec<Packet>,
    incoming_packet_buffer: Vec<Packet>,

    join_set: JoinSet<Option<Packet>>,

    to_network_r: Receiver<Packet>,
}

impl<S> Network<S> {
    pub fn new(options: ConnectOptions, state_handler: StateHandler, to_network_r: Receiver<Packet>) -> Self {
        Self {
            network: None,

            keep_alive_interval_s: options.keep_alive_interval_s,
            options,

            last_network_action: Instant::now(),
            await_pingresp: None,
            perform_keep_alive: true,

            state_handler,
            outgoing_packet_buffer: Vec::new(),
            incoming_packet_buffer: Vec::new(),

            join_set: JoinSet::new(),

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
        H: AsyncEventHandler
    {
        let (mut network, conn_ack) = Stream::connect(&self.options, stream).await?;
        self.last_network_action = Instant::now();

        if let Some(keep_alive_interval) = conn_ack.connack_properties.server_keep_alive {
            self.keep_alive_interval_s = keep_alive_interval as u64;
        }
        if self.keep_alive_interval_s == 0 {
            self.perform_keep_alive = false;
        }

        if let Some(mut retransmit_packets) = self.state_handler.handle_incoming_connack(&conn_ack)? {
            self.outgoing_packet_buffer.append(&mut retransmit_packets)
        }
        handler.handle(Packet::ConnAck(conn_ack)).await;

        network.write_all(&mut self.outgoing_packet_buffer).await?;
        self.last_network_action = Instant::now();

        self.network = Some(network.split());

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
    pub async fn poll<H>(self, handler: &mut H) -> Result<Self, ConnectionError>
    where
        H: AsyncEventHandler + Clone + Send + Sync + 'static
    {
        if self.network.is_none() {
            return Err(ConnectionError::NoNetwork);
        }

        match self.tokio_select(handler).await {
            _ => todo!()
            // Ok(NetworkStatus::Active) => Ok(NetworkStatus::Active),
            // Err(ConnectionError::JoinError(err)) => Err(ConnectionError::JoinError(err)),
            // otherwise => {
            //     self.network = None;
            //     self.await_pingresp = None;
            //     self.outgoing_packet_buffer.clear();
            //     self.incoming_packet_buffer.clear();
            //     self.join_set.abort_all();

            //     otherwise
            // }
        }
    }

    async fn tokio_select<H>(&mut self, handler: &mut H) -> Result<Self, ConnectionError>
    where
        H: AsyncEventHandler + Clone + Send + Sync + 'static
    {
        let Network {
            network,
            keep_alive_interval_s,
            options: _,
            last_network_action,
            await_pingresp,
            perform_keep_alive,
            state_handler: mqtt_handler,
            outgoing_packet_buffer,
            incoming_packet_buffer,
            join_set,
            to_network_r,
        } = self;

        let sleep;
        if let Some(instant) = await_pingresp {
            sleep = *instant + Duration::from_secs(*keep_alive_interval_s) - Instant::now();
        } else {
            sleep = *last_network_action + Duration::from_secs(*keep_alive_interval_s) - Instant::now();
        }
        
        if let Some((read_stream, write_stream)) = network {

            tokio::select! {
                res = read_stream.read_bytes() => {
                    res?;
                    loop {
                        let packet = match stream.parse_message().await {
                            Err(ReadBytes::Err(err)) => return Err(err),
                            Err(ReadBytes::InsufficientBytes(_)) => {
                                break;
                            },
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
                            Packet::ConnAck(conn_ack) => {
                                if let Some(mut retransmit_packets) = mqtt_handler.handle_incoming_connack(&conn_ack)? {
                                    outgoing_packet_buffer.append(&mut retransmit_packets)
                                }
                                handler.handle(Packet::ConnAck(conn_ack)).await;
                            }
                            packet => {
                                match mqtt_handler.handle_incoming_packet(&packet)? {
                                    (reply_packet, true) => {
                                        let handler_clone = handler.clone();
                                        join_set.spawn(async move {
                                            handler_clone.handle(packet).await;
                                            reply_packet
                                        });
                                    },
                                    (Some(reply_packet), false) => {
                                        outgoing_packet_buffer.push(reply_packet);
                                    },
                                    (None, false) => (),
                                }
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
                joined_res = join_set.join_next(), if !join_set.is_empty() => {
                    if let Some(res) = joined_res {
                        if let Some(packet) = res? {
                            outgoing_packet_buffer.push(packet);

                            stream.write_all(outgoing_packet_buffer).await?;
                            *last_network_action = Instant::now();
        
                            Ok(NetworkStatus::Active)

                        } else {
                            Ok(NetworkStatus::Active)
                        }
                    } else {
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
