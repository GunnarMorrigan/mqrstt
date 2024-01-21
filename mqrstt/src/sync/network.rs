use async_channel::Receiver;

use std::io::{Read, Write, ErrorKind};
use std::time::{Duration, Instant};

use crate::available_packet_ids::AvailablePacketIds;
use crate::connect_options::ConnectOptions;
use crate::error::ConnectionError;
use crate::packets::error::ReadBytes;
use crate::packets::reason_codes::DisconnectReasonCode;
use crate::packets::{Disconnect, Packet, PacketType};
use crate::NetworkStatus;
use crate::{EventHandler, StateHandler};

use super::stream::Stream;

/// [`Network`] reads and writes to the network based on tokios [`AsyncReadExt`] [`AsyncWriteExt`].
/// This way you can provide the `connect` function with a TLS and TCP stream of your choosing.
/// The most import thing to remember is that you have to provide a new stream after the previous has failed.
/// (i.e. you need to reconnect after any expected or unexpected disconnect).
pub struct Network<S> {
    network: Option<Stream<S>>,

    /// Options of the current mqtt connection
    keep_alive_interval: Duration,
    options: ConnectOptions,

    last_network_action: Instant,
    await_pingresp: Option<Instant>,
    perform_keep_alive: bool,

    state_handler: StateHandler,
    outgoing_packet_buffer: Vec<Packet>,
    incoming_packet_buffer: Vec<Packet>,

    to_network_r: Receiver<Packet>,
}

impl<S> Network<S> {
    pub fn new(options: ConnectOptions, to_network_r: Receiver<Packet>, apkids: AvailablePacketIds) -> Self {
        let state_handler = StateHandler::new(&options, apkids);
        Self {
            network: None,

            keep_alive_interval: options.keep_alive_interval,
            options,

            last_network_action: Instant::now(),
            await_pingresp: None,
            perform_keep_alive: true,

            state_handler,
            outgoing_packet_buffer: Vec::new(),
            incoming_packet_buffer: Vec::new(),

            to_network_r,
        }
    }
}

impl<S> Network<S>
where
    S: Read + Write + Sized + Unpin,
{
    /// Initializes an MQTT connection with the provided configuration an stream
    pub fn connect<H>(&mut self, stream: S, handler: &mut H) -> Result<(), ConnectionError>
    where
        H: EventHandler,
    {
        let (mut network, conn_ack) = Stream::connect(&self.options, stream)?;

        if let Some(keep_alive_interval) = conn_ack.connack_properties.server_keep_alive {
            self.keep_alive_interval = Duration::from_secs(keep_alive_interval as u64);
        }
        if self.keep_alive_interval.is_zero() {
            self.perform_keep_alive = false;
        }

        let packets = self.state_handler.handle_incoming_connack(&conn_ack)?;
        handler.handle(Packet::ConnAck(conn_ack));
        if let Some(mut packets) = packets {
            network.write_all_packets(&mut packets)?;
        }

        self.network = Some(network);

        Ok(())
    }

    /// A single call to poll continue to loop over the following tasks:
    /// - Read from the stream and parse the bytes to packets for the user to handle
    /// - Write user packets to stream
    /// - Perform keepalive if necessary
    /// - 
    ///
    /// This function can produce an indication of the state of the network or an error.
    /// When the network is still active (i.e. stream is not closed and no disconnect packet has been processed) the network will return [`NetworkStatus::Active`]
    ///
    /// In all other cases the network is unusable anymore.
    /// The stream will be dropped and the internal buffers will be cleared.
    pub fn poll<H>(&mut self, handler: &mut H) -> Result<NetworkStatus, ConnectionError>
    where
        H: EventHandler,
    {   
        loop {
            match self.run(handler) {
                Ok(None) => continue,
                otherwise => {
                    self.network = None;
                    self.await_pingresp = None;
                    self.outgoing_packet_buffer.clear();
                    self.incoming_packet_buffer.clear();
                    
                    return otherwise.map(Option::unwrap)
                }
            }
        }
    }

    fn run<H>(&mut self, handler: &mut H) -> Result<NetworkStatus, ConnectionError>
    where
        H: EventHandler,
    {
        let Network {
            network,
            keep_alive_interval,
            options: _,
            last_network_action,
            await_pingresp,
            perform_keep_alive,
            state_handler,
            outgoing_packet_buffer,
            incoming_packet_buffer,
            to_network_r,
        } = self;

        if let Some(stream) = network {            
            // Read segment of stream
            match stream.read_bytes() {
                Ok(bytes_read) => {
                    if bytes_read > 0 {
                        match stream.parse_messages(incoming_packet_buffer) {
                            Err(ReadBytes::Err(err)) => return Err(err),
                            Err(ReadBytes::InsufficientBytes(_)) => (),
                            Ok(_) => (),
                        }
                    }

                    let needs_flush = !incoming_packet_buffer.is_empty();

                    for packet in incoming_packet_buffer.drain(0..) {
                        use Packet::*;
                        match packet {
                            PingResp => {
                                handler.handle(packet);
                                *await_pingresp = None;
                            }
                            Disconnect(_) => {
                                handler.handle(packet);
                                return Ok(Some(NetworkStatus::IncomingDisconnect));
                            }
                            packet => match state_handler.handle_incoming_packet(&packet)? {
                                (maybe_reply_packet, true) => {
                                    if let Some(reply_packet) = maybe_reply_packet {
                                        outgoing_packet_buffer.push(reply_packet);
                                    }
                                }
                                (Some(reply_packet), false) => {
                                    outgoing_packet_buffer.push(reply_packet);
                                }
                                (None, false) => (),
                            },
                        }
                    }
                    if needs_flush {
                        stream.write_all_packets(outgoing_packet_buffer)?;
                        *last_network_action = Instant::now();
                    }
                },
                Err(err) if err.kind() == ErrorKind::Interrupted => (),
                remaining_err => {
                    remaining_err?;
                },
            }

            // Write segment
            let mut flushed = false;
            while let Ok(packet) = to_network_r.try_recv() {
                flushed = stream.extend_write_buffer(&packet)?;
                let packet_type = packet.packet_type();
                state_handler.handle_outgoing_packet(packet)?;

                if packet_type == PacketType::Disconnect {
                    if !flushed {
                        stream.flush_whole_buffer()?;
                        *last_network_action = Instant::now();
                    }
                    return Ok(Some(NetworkStatus::OutgoingDisconnect));
                }
            }
            if !flushed {
                stream.flush_whole_buffer()?;
            }

            if *perform_keep_alive {
                if let Some(instant) = await_pingresp {
                    if *instant + *keep_alive_interval <= Instant::now() {
                        let disconnect = Disconnect {
                            reason_code: DisconnectReasonCode::KeepAliveTimeout,
                            properties: Default::default(),
                        };
                        stream.write_packet(&Packet::Disconnect(disconnect))?;
                        return Ok(Some(NetworkStatus::KeepAliveTimeout));
                    }
                } else if *last_network_action + *keep_alive_interval <= Instant::now() {
                    stream.write_packet(&Packet::PingReq)?;
                    *last_network_action = Instant::now();
                    *await_pingresp = Some(Instant::now());
                }
            }

            Ok(None)
        } else {
            Err(ConnectionError::NoNetwork)
        }
    }
}
