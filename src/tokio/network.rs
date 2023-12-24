use async_channel::{Receiver, Sender};
use tokio::task::JoinSet;

use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::available_packet_ids::AvailablePacketIds;
use crate::connect_options::ConnectOptions;
use crate::error::ConnectionError;
use crate::packets::error::ReadBytes;
use crate::packets::reason_codes::DisconnectReasonCode;
use crate::packets::{Disconnect, Packet, PacketType};
use crate::state::State;
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
    options: ConnectOptions,

    last_network_action: Instant,
    await_pingresp: Option<Instant>,
    perform_keep_alive: bool,

    state_handler: Arc<StateHandler>,
    // outgoing_packet_buffer: Vec<Packet>,
    // incoming_packet_buffer: Vec<Packet>,

    join_set: JoinSet<()>,

    to_writer_s: Sender<Packet>,
    to_writer_r: Receiver<Packet>,

    to_network_r: Receiver<Packet>,
}

impl<S> Network<S> {
    pub fn new(options: ConnectOptions, to_network_r: Receiver<Packet>, apkids: AvailablePacketIds) -> Self {
        let (to_writer_s, to_writer_r) = async_channel::bounded(100);

        Self {
            network: None,
            
            last_network_action: Instant::now(),
            await_pingresp: None,
            perform_keep_alive: true,
            
            state_handler: Arc::new(StateHandler::new(&options, apkids)),
            
            options,

            join_set: JoinSet::new(),

            to_writer_s,
            to_writer_r,

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
            self.options.keep_alive_interval = Duration::from_secs(keep_alive_interval as u64)
        }
        if self.options.keep_alive_interval.is_zero() {
            self.perform_keep_alive = false;
        }

        let packets = self.state_handler.handle_incoming_connack(&conn_ack)?;

        handler.handle(Packet::ConnAck(conn_ack)).await;
        if let Some(mut packets) = packets {
            network.write_all(&mut packets).await?;

        }
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
    pub async fn poll<H>(&mut self, handler: &mut H) -> Result<NetworkStatus, ConnectionError>
    where
        H: AsyncEventHandler + Clone + Send + Sync + 'static
    {
        if self.network.is_none() {
            return Err(ConnectionError::NoNetwork);
        }

        match self.tokio_select(handler).await {
            Ok(NetworkStatus::Active) => Ok(NetworkStatus::Active),
            Err(ConnectionError::JoinError(err)) => Err(ConnectionError::JoinError(err)),
            otherwise => {
                self.network = None;
                self.await_pingresp = None;
                self.join_set.abort_all();
                self.clear_write_channl();
                
                

                otherwise
            }
        }
    }

    fn clear_write_channl(&mut self) {
        loop {
            match self.to_writer_r.try_recv() {
                Ok(_) => (),
                Err(_) => return,
            }
        }
    }

    async fn tokio_select<H>(&mut self, handler: &mut H) -> Result<NetworkStatus, ConnectionError>
    where
        H: AsyncEventHandler + Clone + Send + Sync + 'static
    {
        let Network {
            network,
            options: options,
            last_network_action,
            await_pingresp,
            perform_keep_alive,
            state_handler: state_handler,
            // outgoing_packet_buffer,
            join_set,
            to_writer_s,
            to_writer_r,
            to_network_r,
        } = self;

        let sleep;
        if let Some(instant) = await_pingresp {
            sleep = *instant + options.keep_alive_interval - Instant::now();
        } else {
            sleep = *last_network_action + options.keep_alive_interval - Instant::now();
        }
        
        if let Some((read_stream, write_stream)) = network {
            tokio::select! {
                // res = read_stream.read_bytes() => {
                //     res?;
                //     loop {
                //         let packet = match read_stream.parse_message().await {
                //             Err(ReadBytes::Err(err)) => return Err(err),
                //             Err(ReadBytes::InsufficientBytes(_)) => {
                //                 break;
                //             },
                //             Ok(packet) => packet,
                //         };

                //         match packet{
                //             Packet::PingResp => {
                //                 handler.handle(packet).await;
                //                 *await_pingresp = None;
                //             },
                //             Packet::Disconnect(_) => {
                //                 handler.handle(packet).await;
                //                 return Ok(NetworkStatus::IncomingDisconnect);
                //             }
                //             Packet::ConnAck(conn_ack) => {
                //                 if let Some(retransmit_packets) = mqtt_handler.handle_incoming_connack(&conn_ack)? {
                //                     retransmit_packets.into_iter().map(|p| to_network_s.send(p));
                //                     // outgoing_packet_buffer.append(&mut retransmit_packets)
                //                 }
                //                 handler.handle(Packet::ConnAck(conn_ack)).await;
                //             }
                //             packet => {
                //                 match mqtt_handler.handle_incoming_packet(&packet)? {
                //                     (maybe_reply_packet, true) => {
                //                         let handler_clone = handler.clone();
                //                         let sender_clone = to_network_s.clone();
                //                         join_set.spawn(async move {
                //                             handler_clone.handle(packet).await;
                //                             if let Some(reply_packet) = maybe_reply_packet {
                //                                 sender_clone.send(reply_packet).await;
                //                             }
                //                         });
                //                     },
                //                     (Some(reply_packet), false) => {
                //                         to_network_s.send(reply_packet);
                //                     },
                //                     (None, false) => (),
                //                 }
                //             }
                //         }
                //     }
                //     Ok(NetworkStatus::Active)
                // },
                // outgoing = to_network_r.recv() => {
                //     let packet = outgoing?;
                //     write_stream.write(&packet).await?;
                //     let mut disconnect = false;

                //     if packet.packet_type() == PacketType::Disconnect{
                //         disconnect = true;
                //     }

                //     mqtt_handler.handle_outgoing_packet(packet)?;
                //     *last_network_action = Instant::now();

                //     if disconnect{
                //         Ok(NetworkStatus::OutgoingDisconnect)
                //     }
                //     else{
                //         Ok(NetworkStatus::Active)
                //     }
                // },
                _ = tokio::time::sleep(sleep), if await_pingresp.is_none() && *perform_keep_alive => {
                    let packet = Packet::PingReq;
                    write_stream.write(&packet).await?;
                    *last_network_action = Instant::now();
                    *await_pingresp = Some(Instant::now());
                    Ok(NetworkStatus::Active)
                },
                _ = tokio::time::sleep(sleep), if await_pingresp.is_some() => {
                    let disconnect = Disconnect{ reason_code: DisconnectReasonCode::KeepAliveTimeout, properties: Default::default() };
                    write_stream.write(&Packet::Disconnect(disconnect)).await?;
                    Ok(NetworkStatus::NoPingResp)
                }
            }
        } else {
            Err(ConnectionError::NoNetwork)
        }
    }

    
}


pub struct NetworkReader<S> {
    read_stream: ReadStream<S>,

    // last_network_action: Instant,
    // await_pingresp: Option<Instant>,
    // perform_keep_alive: bool,

    state_handler: Arc<StateHandler>,
    // outgoing_packet_buffer: Vec<Packet>,
    // incoming_packet_buffer: Vec<Packet>,

    join_set: JoinSet<()>,

    to_writer_s: Sender<Packet>,
}

#[cfg(feature = "tokio")]
impl<S> NetworkReader<S>
where
    S: tokio::io::AsyncReadExt + tokio::io::AsyncWriteExt + Sized + Unpin,
{
    async fn read<H>(&mut self, handler: &mut H) -> Result<NetworkStatus, ConnectionError> 
    where
        H: AsyncEventHandler + Clone + Send + Sync + 'static
    {
        loop {
            let _ = self.read_stream.read_bytes().await?;
            loop {
                let packet = match self.read_stream.parse_message().await {
                    Err(ReadBytes::Err(err)) => return Err(err),
                    Err(ReadBytes::InsufficientBytes(_)) => {
                        break;
                    },
                    Ok(packet) => packet,
                };

                match packet{
                    Packet::PingResp => {
                        handler.handle(packet).await;
                        // *await_pingresp = None;
                    },
                    Packet::Disconnect(_) => {
                        handler.handle(packet).await;
                        return Ok(NetworkStatus::IncomingDisconnect);
                    }
                    Packet::ConnAck(conn_ack) => {
                        if let Some(retransmit_packets) = self.state_handler.handle_incoming_connack(&conn_ack)? {
                            retransmit_packets.into_iter().map(|p| self.to_writer_s.send(p));
                        }
                        handler.handle(Packet::ConnAck(conn_ack)).await;
                    }
                    packet => {
                        match self.state_handler.handle_incoming_packet(&packet)? {
                            (maybe_reply_packet, true) => {
                                let handler_clone = handler.clone();
                                let sender_clone = self.to_writer_s.clone();
                                self.join_set.spawn(async move {
                                    handler_clone.handle(packet).await;
                                    if let Some(reply_packet) = maybe_reply_packet {
                                        sender_clone.send(reply_packet).await;
                                    }
                                });
                            },
                            (Some(reply_packet), false) => {
                                self.to_writer_s.send(reply_packet);
                            },
                            (None, false) => (),
                        }
                    }
                }
            }
        }
        Ok(NetworkStatus::Active)
    }

}

pub struct NetworkWriter<S> {
    write_stream: WriteStream<S>,

    // last_network_action: Instant,
    // await_pingresp: Option<Instant>,
    // perform_keep_alive: bool,

    state_handler: Arc<StateHandler>,

    to_writer_r: Receiver<Packet>,
    to_network_r: Receiver<Packet>,
}

#[cfg(feature = "tokio")]
impl<S> NetworkWriter<S>
where
    S: tokio::io::AsyncReadExt + tokio::io::AsyncWriteExt + Sized + Unpin,
{
    async fn write<H>(&mut self) -> Result<NetworkStatus, ConnectionError> 
    where
        H: AsyncEventHandler + Clone + Send + Sync + 'static
    {   
        loop {
            tokio::select!{
                outgoing = self.to_network_r.recv() => {
                    let packet = outgoing?;
                    self.write_stream.write(&packet).await?;
                    
                    let disconnect = if packet.packet_type() == PacketType::Disconnect { true } else { false };
                    
                    self.state_handler.handle_outgoing_packet(packet)?;
                    // *last_network_action = Instant::now();
                    
                    if disconnect{
                        return Ok(NetworkStatus::OutgoingDisconnect)
                    }
                }
            }
        }
    }
}
