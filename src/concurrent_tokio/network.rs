use async_channel::{Receiver, Sender};
use futures::Future;
use tokio::join;


use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::available_packet_ids::AvailablePacketIds;
use crate::connect_options::ConnectOptions;
use crate::error::ConnectionError;
use crate::packets::error::ReadBytes;
use crate::packets::reason_codes::DisconnectReasonCode;
use crate::packets::{Disconnect, Packet, PacketType};

use crate::{AsyncEventHandler, StateHandler, NetworkStatus};

use super::stream::read_half::ReadStream;
use super::stream::write_half::WriteStream;
use super::stream::Stream;

// type StreamType = tokio::io::AsyncReadExt + tokio::io::AsyncWriteExt + Sized + Unpin + Send + 'static;

/// [`Network`] reads and writes to the network based on tokios [`AsyncReadExt`] [`AsyncWriteExt`].
/// This way you can provide the `connect` function with a TLS and TCP stream of your choosing.
/// The most import thing to remember is that you have to provide a new stream after the previous has failed.
/// (i.e. you need to reconnect after any expected or unexpected disconnect).
pub struct Network<H, S> {
    handler: Option<H>,

    network: Option<Stream<S>>,

    /// Options of the current mqtt connection
    options: ConnectOptions,

    last_network_action: Instant,

    await_pingresp_atomic: Arc<AtomicBool>,
    perform_keep_alive: bool,

    state_handler: Arc<StateHandler>,

    to_network_r: Receiver<Packet>,
}

impl<H, S> Network<H, S> {
    pub fn new(options: ConnectOptions, to_network_r: Receiver<Packet>, apkids: AvailablePacketIds) -> Self {
        Self {
            handler: None,
            network: None,

            last_network_action: Instant::now(),
            await_pingresp_atomic: Arc::new(AtomicBool::new(false)),
            perform_keep_alive: true,

            state_handler: Arc::new(StateHandler::new(&options, apkids)),

            options,

            to_network_r,
        }
    }
}

/// Tokio impl
impl<H, S> Network<H, S>
where
    H: AsyncEventHandler + Clone + Send + Sync + 'static,
    S: tokio::io::AsyncReadExt + tokio::io::AsyncWriteExt + Sized + Unpin + Send + 'static,
{
    /// Initializes an MQTT connection with the provided configuration an stream
    pub async fn connect(&mut self, stream: S, handler: H) -> Result<(), ConnectionError> {
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
            self.last_network_action = Instant::now();
        }

        self.network = Some(network);
        self.handler = Some(handler);

        Ok(())
    }


    #[cfg(feature = "concurrent_tokio")]
    /// Creates both read and write tasks to run this them in parallel.
    /// If you want to run concurrently (not parallel) the [`Self::run`] method is a better aproach!
    pub fn read_write_tasks(&mut self) -> Result<(NetworkReader<H, S>, NetworkWriter<S>), ConnectionError> {
        if self.network.is_none() {
            return Err(ConnectionError::NoNetwork)?;
        }

        match self.network.take() {
            Some(network) => {
                let (read_stream, write_stream) = network.split();
                let run_signal = Arc::new(AtomicBool::new(true));
                let (to_writer_s, to_writer_r) = async_channel::bounded(100);

                let read_network = NetworkReader {
                    run_signal: run_signal.clone(),
                    handler: self.handler.as_ref().unwrap().clone(),
                    read_stream,
                    await_pingresp_atomic: self.await_pingresp_atomic.clone(),
                    state_handler: self.state_handler.clone(),
                    to_writer_s,
                };

                let write_network = NetworkWriter {
                    run_signal: run_signal.clone(),
                    write_stream,
                    keep_alive_interval: self.options.keep_alive_interval,
                    last_network_action: self.last_network_action,
                    await_pingresp_bool: self.await_pingresp_atomic.clone(),
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

    /// A single call to run will perform one of three tasks:
    /// - Read from the stream and parse the bytes to packets for the user to handle
    /// - Write user packets to stream
    /// - Perform keepalive if necessary
    ///
    /// This function can produce an indication of the state of the network or an error.
    /// When the network is still active (i.e. stream is not closed and no disconnect packet has been processed) the network will return [`NetworkStatus::Active`]
    ///
    /// In all other cases the network is unusable anymore.
    /// The stream will be dropped and the internal buffers will be cleared.
    pub fn run() {

    }

}

#[cfg(feature = "concurrent_tokio")]
pub struct NetworkReader<H, S> {
    run_signal: Arc<AtomicBool>,

    handler: H,
    read_stream: ReadStream<S>,
    await_pingresp_atomic: Arc<AtomicBool>,
    state_handler: Arc<StateHandler>,
    to_writer_s: Sender<Packet>,
}

#[cfg(feature = "concurrent_tokio")]
impl<H, S> NetworkReader<H, S>
where
    H: AsyncEventHandler + Clone + Send + Sync + 'static,
    S: tokio::io::AsyncReadExt + Sized + Unpin + Send + 'static,
{   
    /// Runs the read half of the concurrent read & write tokio client.
    /// Continuously loops until disconnect or error.
    /// 
    /// # Return
    ///     - Ok(None) in the case that the write task requested shutdown.
    ///     - Ok(Some(reason)) in the case that this task initiates a shutdown.
    ///     - Err in the case of IO, or protocol errors.
    pub async fn run(mut self) -> Result<Option<NetworkStatus>, ConnectionError> {
        let ret = self.read().await;
        self.run_signal.store(false, std::sync::atomic::Ordering::Release);
        ret
    }
    async fn read(&mut self) -> Result<Option<NetworkStatus>, ConnectionError> {
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
                        self.handler.handle(packet).await;
                        #[cfg(feature = "logs")]
                        if !self.await_pingresp_atomic.fetch_and(false, std::sync::atomic::Ordering::SeqCst) {
                            tracing::warn!("Received PingResp but did not expect it");
                        }
                        #[cfg(not(feature = "logs"))]
                        self.await_pingresp_atomic.store(false, std::sync::atomic::Ordering::SeqCst);
                    }
                    Packet::Disconnect(_) => {
                        self.handler.handle(packet).await;
                        return Ok(Some(NetworkStatus::IncomingDisconnect));
                    }
                    Packet::ConnAck(conn_ack) => {
                        if let Some(retransmit_packets) = self.state_handler.handle_incoming_connack(&conn_ack)? {
                            for packet in retransmit_packets.into_iter() {
                                self.to_writer_s.send(packet).await?;
                            }
                        }
                        self.handler.handle(Packet::ConnAck(conn_ack)).await;
                    }
                    packet => match self.state_handler.handle_incoming_packet(&packet)? {
                        (maybe_reply_packet, true) => {
                            self.handler.handle(packet).await;
                            if let Some(reply_packet) = maybe_reply_packet {
                                let _ = self.to_writer_s.send(reply_packet).await?;
                            }
                        }
                        (Some(reply_packet), false) => {
                            self.to_writer_s.send(reply_packet).await?;
                        }
                        (None, false) => (),
                    },
                }
            }
        }
        Ok(None)
    }
}

#[cfg(feature = "concurrent_tokio")]
pub struct NetworkWriter<S> {
    run_signal: Arc<AtomicBool>,

    write_stream: WriteStream<S>,

    keep_alive_interval: Duration,

    last_network_action: Instant,
    await_pingresp_bool: Arc<AtomicBool>,
    await_pingresp_time: Option<Instant>,
    perform_keep_alive: bool,

    state_handler: Arc<StateHandler>,

    to_writer_r: Receiver<Packet>,
    to_network_r: Receiver<Packet>,
}

#[cfg(feature = "concurrent_tokio")]
impl<S> NetworkWriter<S>
where
    S: tokio::io::AsyncWriteExt + Sized + Unpin,
{   
    /// Runs the write half of the concurrent read & write tokio client
    /// Continuously loops until disconnect or error.
    /// 
    /// # Return
    ///     - Ok(None) in the case that the read task requested shutdown
    ///     - Ok(Some(reason)) in the case that this task initiates a shutdown
    ///     - Err in the case of IO, or protocol errors.
    pub async fn run(mut self) -> Result<Option<NetworkStatus>, ConnectionError> {
        let ret = self.write().await;
        self.run_signal.store(false, std::sync::atomic::Ordering::Release);
        ret
    }
    async fn write(&mut self) -> Result<Option<NetworkStatus>, ConnectionError> {
        while self.run_signal.load(std::sync::atomic::Ordering::Acquire) {
            if self.await_pingresp_time.is_some() && !self.await_pingresp_bool.load(std::sync::atomic::Ordering::Acquire) {
                self.await_pingresp_time = None;
            }

            let sleep;
            if let Some(instant) = &self.await_pingresp_time {
                sleep = *instant + self.keep_alive_interval - Instant::now();
            } else {
                sleep = self.last_network_action + self.keep_alive_interval - Instant::now();
            }

            tokio::select! {
                outgoing = self.to_network_r.recv() => {
                    let packet = outgoing?;
                    self.write_stream.write(&packet).await?;

                    let disconnect = if packet.packet_type() == PacketType::Disconnect { true } else { false };

                    self.state_handler.handle_outgoing_packet(packet)?;
                    self.last_network_action = Instant::now();

                    if disconnect{
                        return Ok(Some(NetworkStatus::OutgoingDisconnect))
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
                        return Ok(Some(NetworkStatus::KeepAliveTimeout))
                    }
                }
            }
        }
        Ok(None)
    }
}