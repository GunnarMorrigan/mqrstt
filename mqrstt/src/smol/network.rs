use async_channel::Receiver;

use futures::FutureExt;

use std::marker::PhantomData;
use std::time::{Duration, Instant};

use crate::available_packet_ids::AvailablePacketIds;
use crate::connect_options::ConnectOptions;
use crate::error::ConnectionError;
use crate::packets::error::ReadBytes;
use crate::packets::reason_codes::DisconnectReasonCode;
use crate::packets::{Disconnect, Packet, PacketType};
use crate::NetworkStatus;
use crate::{AsyncEventHandlerMut, StateHandler};

use super::stream::Stream;

/// [`Network`] reads and writes to the network based on tokios [`::smol::io::AsyncReadExt`] [`::smol::io::AsyncWriteExt`].
/// This way you can provide the `connect` function with a TLS and TCP stream of your choosing.
/// The most import thing to remember is that you have to provide a new stream after the previous has failed.
/// (i.e. you need to reconnect after any expected or unexpected disconnect).
pub struct Network<H, S> {
    handler: PhantomData<H>,
    network: Option<Stream<S>>,

    /// Options of the current mqtt connection
    keep_alive_interval: Duration,
    options: ConnectOptions,

    last_network_action: Instant,
    await_pingresp: Option<Instant>,
    perform_keep_alive: bool,

    state_handler: StateHandler,
    outgoing_packet_buffer: Vec<Packet>,

    to_network_r: Receiver<Packet>,
}

impl<H, S> Network<H, S> {
    pub fn new(options: ConnectOptions, to_network_r: Receiver<Packet>, apkids: AvailablePacketIds) -> Self {
        let state_handler = StateHandler::new(&options, apkids);
        Self {
            handler: PhantomData,
            network: None,

            keep_alive_interval: options.keep_alive_interval,
            options,

            last_network_action: Instant::now(),
            await_pingresp: None,
            perform_keep_alive: true,

            state_handler,
            outgoing_packet_buffer: Vec::new(),

            to_network_r,
        }
    }
}

impl<H, S> Network<H, S>
where
    H: AsyncEventHandlerMut,
    S: smol::io::AsyncReadExt + smol::io::AsyncWriteExt + Sized + Unpin,
{
    /// Initializes an MQTT connection with the provided configuration an stream
    pub async fn connect(&mut self, stream: S, handler: &mut H) -> Result<(), ConnectionError> {
        let (mut network, conn_ack) = Stream::connect(&self.options, stream).await?;
        self.last_network_action = Instant::now();

        if let Some(keep_alive_interval) = conn_ack.connack_properties.server_keep_alive {
            self.keep_alive_interval = Duration::from_secs(keep_alive_interval as u64);
        }
        if self.keep_alive_interval.is_zero() {
            self.perform_keep_alive = false;
        }

        let packets = self.state_handler.handle_incoming_connack(&conn_ack)?;
        handler.handle(Packet::ConnAck(conn_ack)).await;
        if let Some(mut packets) = packets {
            network.write_all(&mut packets).await?;
            self.last_network_action = Instant::now();
        }

        self.network = Some(network);

        Ok(())
    }

    /// A single call to [`Network::run`] will continiously perform the tasks mentioned below until an error is encountered.
    /// 1. Read from the stream and parse the bytes to packets for the user to handle
    /// 2. Write user packets to stream
    /// 3. Perform keepalive if necessary
    ///
    /// This function can produce an indication of the state of the network or an error.
    ///
    /// In all other cases the network is unusable anymore.
    /// The stream will be dropped and the internal buffers will be cleared.
    pub async fn run(&mut self, handler: &mut H) -> Result<NetworkStatus, ConnectionError> {
        if self.network.is_none() {
            return Err(ConnectionError::NoNetwork);
        }
        loop {
            match self.smol_select(handler).await {
                Ok(None) => continue,
                otherwise => {
                    self.network = None;
                    self.await_pingresp = None;
                    self.outgoing_packet_buffer.clear();

                    // This is safe as inside the Ok it is not possible to have a None due to the above Ok(None) pattern.
                    return otherwise.map(|ok| ok.unwrap());
                }
            }
        }
    }

    async fn smol_select(&mut self, handler: &mut H) -> Result<Option<NetworkStatus>, ConnectionError> {
        let Network {
            handler: _,
            network,
            options: _,
            keep_alive_interval,
            last_network_action,
            await_pingresp,
            perform_keep_alive,
            state_handler,
            outgoing_packet_buffer,
            to_network_r,
        } = self;

        let sleep;
        if !(*perform_keep_alive) {
            sleep = Duration::new(3600, 0);
        } else if let Some(instant) = await_pingresp {
            sleep = *instant + *keep_alive_interval - Instant::now();
        } else {
            sleep = *last_network_action + *keep_alive_interval - Instant::now();
        }

        if let Some(stream) = network {
            futures::select! {
                res = stream.read_bytes().fuse() => {
                    res?;
                    match stream.parse_message().await {
                        Err(ReadBytes::Err(err)) => return Err(err),
                        Err(ReadBytes::InsufficientBytes(_)) => return Ok(None),
                        Ok(packet) => {
                            match packet{
                                Packet::PingResp => {
                                    handler.handle(packet).await;
                                    *await_pingresp = None;
                                },
                                Packet::Disconnect(_) => {
                                    handler.handle(packet).await;
                                    return Ok(Some(NetworkStatus::IncomingDisconnect));
                                }
                                packet => {
                                    match state_handler.handle_incoming_packet(&packet)? {
                                        (maybe_reply_packet, true) => {
                                            handler.handle(packet).await;
                                            if let Some(reply_packet) = maybe_reply_packet {
                                                outgoing_packet_buffer.push(reply_packet);
                                            }
                                        },
                                        (Some(reply_packet), false) => {
                                            outgoing_packet_buffer.push(reply_packet);
                                        },
                                        (None, false) => (),
                                    }
                                }
                            }
                            stream.write_all(outgoing_packet_buffer).await?;
                            *last_network_action = Instant::now();
                        },
                    }

                    Ok(None)
                },
                outgoing = to_network_r.recv().fuse() => {
                    let packet = outgoing?;
                    stream.write(&packet).await?;

                    let disconnect = packet.packet_type() == PacketType::Disconnect;

                    state_handler.handle_outgoing_packet(packet)?;
                    *last_network_action = Instant::now();


                    if disconnect{
                        Ok(Some(NetworkStatus::OutgoingDisconnect))
                    }
                    else{
                        Ok(None)
                    }
                },
                _ = smol::Timer::after(sleep).fuse() => {
                    if await_pingresp.is_none() && *perform_keep_alive{
                        let packet = Packet::PingReq;
                        stream.write(&packet).await?;
                        *last_network_action = Instant::now();
                        *await_pingresp = Some(Instant::now());
                        Ok(None)
                    }
                    else{
                        let disconnect = Disconnect{ reason_code: DisconnectReasonCode::KeepAliveTimeout, properties: Default::default() };
                        stream.write(&Packet::Disconnect(disconnect)).await?;
                        Ok(Some(NetworkStatus::KeepAliveTimeout))
                    }
                },
            }
        } else {
            Err(ConnectionError::NoNetwork)
        }
    }
}
