use async_channel::Receiver;

use std::io::{Read, Write};
use std::time::{Duration, Instant};

use crate::connect_options::ConnectOptions;
use crate::error::ConnectionError;
use crate::packets::error::ReadBytes;
use crate::packets::reason_codes::DisconnectReasonCode;
use crate::packets::{Disconnect, Packet, PacketType};
use crate::sync::NetworkStatus;
use crate::{EventHandler, StateHandler};

use super::stream::Stream;

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

    mqtt_handler: StateHandler,
    outgoing_packet_buffer: Vec<Packet>,
    incoming_packet_buffer: Vec<Packet>,

    to_network_r: Receiver<Packet>,
}

impl<S> Network<S> {
    pub fn new(options: ConnectOptions, mqtt_handler: StateHandler, to_network_r: Receiver<Packet>) -> Self {
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

impl<S> Network<S>
where
    S: Read + Write + Sized + Unpin,
{
    /// Initializes an MQTT connection with the provided configuration an stream
    pub fn connect<H>(&mut self, stream: S, handler: &mut H) -> Result<(), ConnectionError>
    where
        H: EventHandler,
    {
        let (network, connack) = Stream::connect(&self.options, stream)?;

        self.network = Some(network);

        if let Some(keep_alive_interval) = connack.connack_properties.server_keep_alive {
            self.keep_alive_interval_s = keep_alive_interval as u64;
        }
        if self.keep_alive_interval_s == 0 {
            self.perform_keep_alive = false;
        }

        let packet = Packet::ConnAck(connack);

        // self.mqtt_handler.handle_incoming_packet(&packet, &mut self.outgoing_packet_buffer)?;
        // handler.handle(packet);

        Ok(())
    }

    /// A single call to poll will perform all of the following tasks:
    /// - Read from the stream and parse the bytes to packets for the user to handle
    /// - Write user packets to stream
    /// - Perform keepalive if necessary
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
        match self.select(handler) {
            Ok(NetworkStatus::ActivePending) => Ok(NetworkStatus::ActivePending),
            Ok(NetworkStatus::ActiveReady) => Ok(NetworkStatus::ActiveReady),
            otherwise => {
                self.network = None;
                self.await_pingresp = None;
                self.outgoing_packet_buffer.clear();
                self.incoming_packet_buffer.clear();

                otherwise
            }
        }
    }

    fn select<H>(&mut self, handler: &mut H) -> Result<NetworkStatus, ConnectionError>
    where
        H: EventHandler,
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

        if let Some(stream) = network {
            let mut ret_status = NetworkStatus::ActivePending;
            // Read segment of stream
            {
                if stream.read_bytes()? > 0 {
                    match stream.parse_messages(incoming_packet_buffer) {
                        Err(ReadBytes::Err(err)) => return Err(err),
                        Err(ReadBytes::InsufficientBytes(_)) => ret_status = NetworkStatus::ActiveReady,
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
                            return Ok(NetworkStatus::IncomingDisconnect);
                        }
                        packet => {
                            todo!()
                            // if mqtt_handler.handle_incoming_packet(&packet, outgoing_packet_buffer)? {
                            //     handler.handle(packet);
                            // }
                        }
                    }
                }
                if needs_flush {
                    stream.write_all_packets(outgoing_packet_buffer)?;
                    *last_network_action = Instant::now();
                }
            }

            // Write segment
            let mut flushed = false;
            while let Ok(packet) = to_network_r.try_recv() {
                flushed = stream.extend_write_buffer(&packet)?;
                let packet_type = packet.packet_type();
                println!("Handling outgoing packet: {:?}", packet.packet_type());
                mqtt_handler.handle_outgoing_packet(packet)?;

                if packet_type == PacketType::Disconnect {
                    if !flushed {
                        stream.flush_whole_buffer()?;
                        *last_network_action = Instant::now();
                    }
                    return Ok(NetworkStatus::OutgoingDisconnect);
                }
            }
            if !flushed {
                stream.flush_whole_buffer()?;
            }

            if *perform_keep_alive {
                if let Some(instant) = await_pingresp {
                    if *instant + Duration::from_secs(*keep_alive_interval_s) <= Instant::now() {
                        let disconnect = Disconnect {
                            reason_code: DisconnectReasonCode::KeepAliveTimeout,
                            properties: Default::default(),
                        };
                        stream.write_packet(&Packet::Disconnect(disconnect))?;
                        return Ok(NetworkStatus::NoPingResp);
                    }
                } else if *last_network_action + Duration::from_secs(*keep_alive_interval_s) <= Instant::now() {
                    stream.write_packet(&Packet::PingReq)?;
                    *last_network_action = Instant::now();
                    *await_pingresp = Some(Instant::now());
                }
            }

            Ok(ret_status)
        } else {
            Err(ConnectionError::NoNetwork)
        }
    }
}
