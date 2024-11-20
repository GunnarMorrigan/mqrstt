use async_channel::{Receiver, Sender};
use tokio::task::JoinSet;

use std::marker::PhantomData;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::available_packet_ids::AvailablePacketIds;
use crate::connect_options::ConnectOptions;
use crate::error::ConnectionError;
use crate::packets::error::ReadBytes;
use crate::packets::DisconnectReasonCode;
use crate::packets::{Disconnect, Packet, PacketType};

use crate::{AsyncEventHandlerMut, NetworkStatus, StateHandler};

use super::stream::Stream;
use super::{HandlerExt, SequentialHandler};

/// [`Network`] reads and writes to the network based on tokios [`::tokio::io::AsyncReadExt`] [`::tokio::io::AsyncWriteExt`].
/// This way you can provide the `connect` function with a TLS and TCP stream of your choosing.
/// The most import thing to remember is that you have to provide a new stream after the previous has failed.
/// (i.e. you need to reconnect after any expected or unexpected disconnect).
pub struct Network<N, H, S> {
    handler_helper: PhantomData<N>,
    handler: PhantomData<H>,
    network: Option<Stream<S>>,

    /// Options of the current mqtt connection
    options: ConnectOptions,
    last_network_action: Instant,
    perform_keep_alive: bool,
    state_handler: Arc<StateHandler>,
    to_network_r: Receiver<Packet>,
}

impl<N, H, S> Network<N, H, S> {
    pub fn new(options: ConnectOptions, to_network_r: Receiver<Packet>, apkids: AvailablePacketIds) -> Self {
        Self {
            handler_helper: PhantomData,
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

/// Tokio impl
impl<N, H, S> Network<N, H, S>
where
    N: HandlerExt<H>,
    S: tokio::io::AsyncReadExt + tokio::io::AsyncWriteExt + Sized + Unpin + Send + 'static,
{
    /// Initializes an MQTT connection with the provided configuration an stream
    pub async fn connect(&mut self, stream: S, handler: &mut H) -> Result<(), ConnectionError> {
        let (mut network, conn_ack) = Stream::connect(&self.options, stream).await?;
        self.last_network_action = Instant::now();

        if let Some(keep_alive_interval) = conn_ack.connack_properties.server_keep_alive {
            self.options.keep_alive_interval = Duration::from_secs(keep_alive_interval as u64)
        }
        if self.options.keep_alive_interval.is_zero() {
            self.perform_keep_alive = false;
        }

        let packets = self.state_handler.handle_incoming_connack(&conn_ack)?;
        N::call_handler_await(handler, Packet::ConnAck(conn_ack)).await;
        if let Some(mut packets) = packets {
            network.write_all(&mut packets).await?;
            self.last_network_action = Instant::now();
        }

        self.network = Some(network);

        Ok(())
    }
}

impl<H, S> Network<SequentialHandler, H, S>
where
    H: AsyncEventHandlerMut,
    SequentialHandler: HandlerExt<H>,
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
            handler_helper: _,
            handler: _,
            state_handler,
        } = self;

        let mut await_pingresp = None;
        let mut outgoing_packet_buffer = Vec::new();

        loop {
            let sleep;
            if let Some(instant) = await_pingresp {
                sleep = instant + options.get_keep_alive_interval() - Instant::now();
            } else {
                sleep = *last_network_action + options.get_keep_alive_interval() - Instant::now();
            }

            if let Some(stream) = network {
                tokio::select! {
                    res = stream.read_bytes() => {
                        res?;
                        loop{
                            let packet = match stream.parse_message().await {
                                Err(ReadBytes::Err(err)) => return Err(err),
                                Err(ReadBytes::InsufficientBytes(_)) => break,
                                Ok(packet) => packet,
                            };
                            match packet{
                                Packet::PingResp => {
                                    SequentialHandler::call_handler_await(handler, packet).await;
                                    await_pingresp = None;
                                },
                                Packet::Disconnect(_) => {
                                    SequentialHandler::call_handler_await(handler, packet).await;
                                    return Ok(NetworkStatus::IncomingDisconnect);
                                }
                                packet => {
                                    match state_handler.handle_incoming_packet(&packet)? {
                                        (maybe_reply_packet, true) => {
                                            SequentialHandler::call_handler_await(handler, packet).await;
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
                            stream.write_all(&mut outgoing_packet_buffer).await?;
                            *last_network_action = Instant::now();
                        }
                    },
                    outgoing = to_network_r.recv() => {
                        let packet = outgoing?;
                        stream.write(&packet).await?;
                        let disconnect = packet.packet_type() == PacketType::Disconnect;

                        state_handler.handle_outgoing_packet(packet)?;
                        *last_network_action = Instant::now();


                        if disconnect{
                            return Ok(NetworkStatus::OutgoingDisconnect);
                        }
                    },
                    _ = tokio::time::sleep(sleep), if await_pingresp.is_none() && *perform_keep_alive => {
                        let packet = Packet::PingReq;
                        stream.write(&packet).await?;
                        *last_network_action = Instant::now();
                        await_pingresp = Some(Instant::now());
                    },
                    _ = tokio::time::sleep(sleep), if await_pingresp.is_some() => {
                        let disconnect = Disconnect{ reason_code: DisconnectReasonCode::KeepAliveTimeout, properties: Default::default() };
                        stream.write(&Packet::Disconnect(disconnect)).await?;
                        return Ok(NetworkStatus::KeepAliveTimeout);
                    }
                }
            } else {
                return Err(ConnectionError::NoNetwork);
            }
        }
    }
}

impl<N, H, S> Network<N, H, S>
where
    S: tokio::io::AsyncReadExt + tokio::io::AsyncWriteExt + Sized + Unpin + Send + 'static,
{
    /// Creates both read and write tasks to run this them in parallel.
    /// If you want to run concurrently (not parallel) the [`Self::run`] method is a better aproach!
    pub fn split(&mut self, handler: H) -> Result<(NetworkReader<N, H, S>, NetworkWriter<S>), ConnectionError> {
        if self.network.is_none() {
            return Err(ConnectionError::NoNetwork)?;
        }

        match self.network.take() {
            Some(network) => {
                let (read_stream, write_stream) = network.split();
                let run_signal = Arc::new(AtomicBool::new(true));
                let (to_writer_s, to_writer_r) = async_channel::bounded(100);
                let await_pingresp_atomic = Arc::new(AtomicBool::new(false));

                let read_network = NetworkReader {
                    run_signal: run_signal.clone(),
                    handler_helper: PhantomData,
                    handler: handler,
                    read_stream,
                    await_pingresp_atomic: await_pingresp_atomic.clone(),
                    state_handler: self.state_handler.clone(),
                    to_writer_s,
                    join_set: JoinSet::new(),
                };

                let write_network = NetworkWriter {
                    run_signal: run_signal.clone(),
                    write_stream,
                    keep_alive_interval: self.options.keep_alive_interval,
                    last_network_action: self.last_network_action,
                    await_pingresp_bool: await_pingresp_atomic.clone(),
                    await_pingresp_time: None,
                    perform_keep_alive: self.perform_keep_alive,
                    state_handler: self.state_handler.clone(),
                    to_writer_r: to_writer_r,
                    to_network_r: self.to_network_r.clone(),
                };

                Ok((read_network, write_network))
            }
            None => Err(ConnectionError::NoNetwork),
        }
    }
}

pub struct NetworkReader<N, H, S> {
    pub(crate) run_signal: Arc<AtomicBool>,

    pub(crate) handler_helper: PhantomData<N>,
    pub handler: H,

    pub(crate) read_stream: super::stream::read_half::ReadStream<S>,
    pub(crate) await_pingresp_atomic: Arc<AtomicBool>,
    pub(crate) state_handler: Arc<StateHandler>,
    pub(crate) to_writer_s: Sender<Packet>,
    pub(crate) join_set: JoinSet<Result<(), ConnectionError>>,
}

impl<N, H, S> NetworkReader<N, H, S>
where
    N: HandlerExt<H>,
    S: tokio::io::AsyncReadExt + Sized + Unpin + Send + 'static,
{
    /// Runs the read half of the mqtt connection.
    /// Continuously loops until disconnect or error.
    ///
    /// # Return
    /// - Ok(None) in the case that the write task requested shutdown.
    /// - Ok(Some(reason)) in the case that this task initiates a shutdown.
    /// - Err in the case of IO, or protocol errors.
    pub async fn run(mut self) -> (Result<NetworkStatus, ConnectionError>, H) {
        let ret = self.read().await;
        self.run_signal.store(false, std::sync::atomic::Ordering::Release);
        while let Some(_) = self.join_set.join_next().await {
            ()
        }
        (ret, self.handler)
    }
    async fn read(&mut self) -> Result<NetworkStatus, ConnectionError> {
        while self.run_signal.load(std::sync::atomic::Ordering::Acquire) {
            let _ = self.read_stream.read_bytes().await?;
            loop {
                let packet = match self.read_stream.parse_message() {
                    Err(ReadBytes::Err(err)) => return Err(err),
                    Err(ReadBytes::InsufficientBytes(_)) => {
                        break;
                    }
                    Ok(packet) => packet,
                };

                match packet {
                    Packet::PingResp => {
                        N::call_handler(&mut self.handler, packet).await;
                        #[cfg(feature = "logs")]
                        if !self.await_pingresp_atomic.fetch_and(false, std::sync::atomic::Ordering::SeqCst) {
                            tracing::warn!("Received PingResp but did not expect it");
                        }
                        #[cfg(not(feature = "logs"))]
                        self.await_pingresp_atomic.store(false, std::sync::atomic::Ordering::SeqCst);
                    }
                    Packet::Disconnect(_) => {
                        N::call_handler(&mut self.handler, packet).await;
                        return Ok(NetworkStatus::IncomingDisconnect);
                    }
                    Packet::ConnAck(conn_ack) => {
                        if let Some(retransmit_packets) = self.state_handler.handle_incoming_connack(&conn_ack)? {
                            for packet in retransmit_packets.into_iter() {
                                self.to_writer_s.send(packet).await?;
                            }
                        }
                        N::call_handler(&mut self.handler, Packet::ConnAck(conn_ack)).await;
                    }
                    packet => match self.state_handler.handle_incoming_packet(&packet)? {
                        (maybe_reply_packet, true) => {
                            N::call_handler_with_reply(self, packet, maybe_reply_packet).await?;
                        }
                        (Some(reply_packet), false) => {
                            self.to_writer_s.send(reply_packet).await?;
                        }
                        (None, false) => (),
                    },
                }
            }
        }
        Ok(NetworkStatus::ShutdownSignal)
    }
}

pub struct NetworkWriter<S> {
    run_signal: Arc<AtomicBool>,

    write_stream: super::stream::write_half::WriteStream<S>,

    keep_alive_interval: Duration,

    last_network_action: Instant,
    await_pingresp_bool: Arc<AtomicBool>,
    await_pingresp_time: Option<Instant>,
    perform_keep_alive: bool,

    state_handler: Arc<StateHandler>,

    to_writer_r: Receiver<Packet>,
    to_network_r: Receiver<Packet>,
}

impl<S> NetworkWriter<S>
where
    S: tokio::io::AsyncWriteExt + Sized + Unpin,
{
    /// Runs the read half of the mqtt connection.
    /// Continuously loops until disconnect or error.
    ///
    /// # Return
    /// - Ok(None) in the case that the read task requested shutdown
    /// - Ok(Some(reason)) in the case that this task initiates a shutdown
    /// - Err in the case of IO, or protocol errors.
    pub async fn run(mut self) -> Result<NetworkStatus, ConnectionError> {
        let ret = self.write().await;
        self.run_signal.store(false, std::sync::atomic::Ordering::Release);
        ret
    }
    async fn write(&mut self) -> Result<NetworkStatus, ConnectionError> {
        while self.run_signal.load(std::sync::atomic::Ordering::Acquire) {
            if self.await_pingresp_time.is_some() && !self.await_pingresp_bool.load(std::sync::atomic::Ordering::Acquire) {
                self.await_pingresp_time = None;
            }

            let sleep;
            if let Some(instant) = &self.await_pingresp_time {
                sleep = *instant + self.keep_alive_interval - Instant::now();
            } else {
                sleep = self.last_network_action + self.keep_alive_interval - Instant::now();
            };
            tokio::select! {
                outgoing = self.to_network_r.recv() => {
                    let packet = outgoing?;
                    self.write_stream.write(&packet).await?;

                    let disconnect = packet.packet_type() == PacketType::Disconnect;

                    self.state_handler.handle_outgoing_packet(packet)?;
                    self.last_network_action = Instant::now();

                    if disconnect{
                        return Ok(NetworkStatus::OutgoingDisconnect);
                    }
                },
                from_reader = self.to_writer_r.recv() => {
                    let packet = from_reader?;
                    self.write_stream.write(&packet).await?;
                    match packet {
                        foo @ (Packet::Publish(_) | Packet::Subscribe(_) | Packet::Unsubscribe(_) | Packet::Disconnect(_)) => {
                            self.state_handler.handle_outgoing_packet(foo)?;
                        },
                        _ => (),
                    }
                    self.last_network_action = Instant::now();
                },
                _ = tokio::time::sleep(sleep), if self.await_pingresp_time.is_none() && self.perform_keep_alive => {
                    let packet = Packet::PingReq;
                    self.write_stream.write(&packet).await?;
                    self.await_pingresp_bool.store(true, std::sync::atomic::Ordering::SeqCst);
                    self.last_network_action = Instant::now();
                    self.await_pingresp_time = Some(Instant::now());
                },
                _ = tokio::time::sleep(sleep), if self.await_pingresp_time.is_some() => {
                    self.await_pingresp_time = None;
                    if self.await_pingresp_bool.load(std::sync::atomic::Ordering::SeqCst){
                        let disconnect = Disconnect{ reason_code: DisconnectReasonCode::KeepAliveTimeout, properties: Default::default() };
                        self.write_stream.write(&Packet::Disconnect(disconnect)).await?;
                        return Ok(NetworkStatus::KeepAliveTimeout);
                    }
                }
            }
        }
        Ok(NetworkStatus::ShutdownSignal)
    }
}
