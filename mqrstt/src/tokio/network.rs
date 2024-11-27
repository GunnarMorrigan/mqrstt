use async_channel::Receiver;

use std::marker::PhantomData;

use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::available_packet_ids::AvailablePacketIds;
use crate::connect_options::ConnectOptions;
use crate::error::ConnectionError;
use crate::packets::DisconnectReasonCode;
use crate::packets::{Disconnect, Packet, PacketType};

use crate::{AsyncEventHandler, NetworkStatus, StateHandler};

use super::stream::StreamExt;

/// [`Network`] reads and writes to the network based on tokios [`::tokio::io::AsyncReadExt`] [`::tokio::io::AsyncWriteExt`].
/// This way you can provide the `connect` function with a TLS and TCP stream of your choosing.
/// The most import thing to remember is that you have to provide a new stream after the previous has failed.
/// (i.e. you need to reconnect after any expected or unexpected disconnect).
pub struct Network<H, S> {
    handler: PhantomData<H>,
    network: Option<S>,

    /// Options of the current mqtt connection
    options: ConnectOptions,
    last_network_action: Instant,
    perform_keep_alive: bool,
    state_handler: Arc<StateHandler>,
    to_network_r: Receiver<Packet>,
}

impl<H, S> Network<H, S> {
    pub(crate) fn new(options: ConnectOptions, to_network_r: Receiver<Packet>, apkids: AvailablePacketIds) -> Self {
        Self {
            handler: PhantomData,
            network: None,

            last_network_action: Instant::now(),
            perform_keep_alive: true,

            state_handler: Arc::new(StateHandler::new(&options, apkids)),

            options,

            to_network_r,
        }
    }
}

impl<H, S> Network<H, S>
where
    H: AsyncEventHandler,
    S: tokio::io::AsyncReadExt + tokio::io::AsyncWriteExt + Sized + Unpin + Send + 'static,
{
    /// Initializes an MQTT connection with the provided configuration an stream
    pub async fn connect(&mut self, mut stream: S, handler: &mut H) -> Result<(), ConnectionError> {
        let conn_ack = stream.connect(&self.options).await?;
        self.last_network_action = Instant::now();

        if let Some(keep_alive_interval) = conn_ack.connack_properties.server_keep_alive {
            self.options.keep_alive_interval = Duration::from_secs(keep_alive_interval as u64)
        }
        if self.options.keep_alive_interval.is_zero() {
            self.perform_keep_alive = false;
        }

        let packets = self.state_handler.handle_incoming_connack(&conn_ack)?;
        handler.handle(Packet::ConnAck(conn_ack)).await;
        if let Some(packets) = packets {
            stream.write_packets(&packets).await?;
            self.last_network_action = Instant::now();
        }

        self.network = Some(stream);

        Ok(())
    }
}

impl<H, S> Network<H, S>
where
    H: AsyncEventHandler,
    S: tokio::io::AsyncReadExt + tokio::io::AsyncWriteExt + Sized + Unpin + Send + 'static,
{
    /// A single call to run will perform one of three tasks:
    /// - Read from the stream and parse the bytes to packets for the user to handle
    /// - Write user packets to stream
    /// - Perform keepalive if necessary
    ///
    /// In all other cases the network is unusable anymore.
    /// The stream will be dropped and the internal buffers will be cleared.
    pub async fn run(&mut self, handler: &mut H) -> Result<NetworkStatus, ConnectionError> {
        if self.network.is_none() {
            return Err(ConnectionError::NoNetwork);
        }

        match self.tokio_select(handler).await {
            otherwise => {
                self.network = None;

                otherwise
            }
        }
    }

    async fn tokio_select(&mut self, handler: &mut H) -> Result<NetworkStatus, ConnectionError> {
        let Network {
            network,
            options,
            last_network_action,
            perform_keep_alive,
            to_network_r,
            handler: _,
            state_handler,
        } = self;

        let mut await_pingresp = None;

        loop {
            let sleep;
            if let Some(instant) = await_pingresp {
                sleep = instant + options.get_keep_alive_interval() - Instant::now();
            } else {
                sleep = *last_network_action + options.get_keep_alive_interval() - Instant::now();
            }

            if let Some(stream) = network {
                tokio::select! {
                    res = stream.read_packet() => {
                        #[cfg(feature = "logs")]
                        tracing::trace!("Received incoming packet {:?}", &res);

                        let packet = res?;
                        match packet{
                            Packet::PingResp => {
                                handler.handle(packet).await;
                                await_pingresp = None;
                            },
                            Packet::Disconnect(_) => {
                                handler.handle(packet).await;
                                return Ok(NetworkStatus::IncomingDisconnect);
                            }
                            packet => {
                                match state_handler.handle_incoming_packet(&packet)? {
                                    (maybe_reply_packet, true) => {
                                        handler.handle(packet).await;
                                        if let Some(reply_packet) = maybe_reply_packet {
                                            stream.write_packet(&reply_packet).await?;
                                            *last_network_action = Instant::now();
                                        }
                                    },
                                    (Some(reply_packet), false) => {
                                        stream.write_packet(&reply_packet).await?;
                                        *last_network_action = Instant::now();
                                    },
                                    (None, false) => (),
                                }
                            }
                        }
                    },
                    outgoing = to_network_r.recv() => {
                        #[cfg(feature = "logs")]
                        tracing::trace!("Received outgoing item {:?}", &outgoing);

                        let packet = outgoing?;

                        #[cfg(feature = "logs")]
                        tracing::trace!("Sending packet {}", packet);

                        stream.write_packet(&packet).await?;
                        let disconnect = packet.packet_type() == PacketType::Disconnect;

                        state_handler.handle_outgoing_packet(packet)?;
                        *last_network_action = Instant::now();


                        if disconnect{
                            return Ok(NetworkStatus::OutgoingDisconnect);
                        }
                    },
                    _ = tokio::time::sleep(sleep), if await_pingresp.is_none() && *perform_keep_alive => {
                        let packet = Packet::PingReq;
                        stream.write_packet(&packet).await?;
                        *last_network_action = Instant::now();
                        await_pingresp = Some(Instant::now());
                    },
                    _ = tokio::time::sleep(sleep), if await_pingresp.is_some() => {
                        let disconnect = Disconnect{ reason_code: DisconnectReasonCode::KeepAliveTimeout, properties: Default::default() };
                        stream.write_packet(&Packet::Disconnect(disconnect)).await?;
                        return Ok(NetworkStatus::KeepAliveTimeout);
                    }
                }
            } else {
                return Err(ConnectionError::NoNetwork);
            }
        }
    }

    // async fn concurrent_tokio_select(&mut self, handler: &mut H) -> Result<NetworkStatus, ConnectionError> {
    //     let Network {
    //         network,
    //         options,
    //         last_network_action,
    //         perform_keep_alive,
    //         to_network_r,
    //         handler: _,
    //         state_handler,
    //     } = self;

    //     let mut await_pingresp = None;

    //     loop {
    //         let sleep;
    //         if let Some(instant) = await_pingresp {
    //             sleep = instant + options.get_keep_alive_interval() - Instant::now();
    //         } else {
    //             sleep = *last_network_action + options.get_keep_alive_interval() - Instant::now();
    //         }

    //         if let Some(stream) = network {
    //             tokio::select! {
    //                 res = stream.read_packet() => {
    //                     let packet = res?;
    //                     match packet{
    //                         Packet::PingResp => {
    //                             handler.handle(packet).await;
    //                             await_pingresp = None;
    //                         },
    //                         Packet::Disconnect(_) => {
    //                             handler.handle(packet).await;
    //                             return Ok(NetworkStatus::IncomingDisconnect);
    //                         }
    //                         packet => {
    //                             match state_handler.handle_incoming_packet(&packet)? {
    //                                 (maybe_reply_packet, true) => {
    //                                     handler.handle(packet).await;
    //                                     if let Some(reply_packet) = maybe_reply_packet {
    //                                         stream.write_packet(&reply_packet).await?;
    //                                         *last_network_action = Instant::now();
    //                                     }
    //                                 },
    //                                 (Some(reply_packet), false) => {
    //                                     stream.write_packet(&reply_packet).await?;
    //                                     *last_network_action = Instant::now();
    //                                 },
    //                                 (None, false) => (),
    //                             }
    //                         }
    //                     }
    //                 },
    //                 outgoing = to_network_r.recv() => {
    //                     let packet = outgoing?;
    //                     stream.write_packet(&packet).await?;
    //                     let disconnect = packet.packet_type() == PacketType::Disconnect;

    //                     state_handler.handle_outgoing_packet(packet)?;
    //                     *last_network_action = Instant::now();

    //                     if disconnect{
    //                         return Ok(NetworkStatus::OutgoingDisconnect);
    //                     }
    //                 },
    //                 _ = tokio::time::sleep(sleep), if await_pingresp.is_none() && *perform_keep_alive => {
    //                     let packet = Packet::PingReq;
    //                     stream.write_packet(&packet).await?;
    //                     *last_network_action = Instant::now();
    //                     await_pingresp = Some(Instant::now());
    //                 },
    //                 _ = tokio::time::sleep(sleep), if await_pingresp.is_some() => {
    //                     let disconnect = Disconnect{ reason_code: DisconnectReasonCode::KeepAliveTimeout, properties: Default::default() };
    //                     stream.write_packet(&Packet::Disconnect(disconnect)).await?;
    //                     return Ok(NetworkStatus::KeepAliveTimeout);
    //                 }
    //             }
    //         } else {
    //             return Err(ConnectionError::NoNetwork);
    //         }
    //     }
    // }
}
