use async_channel::Receiver;

use std::io::{Write, Read};
use std::time::{Duration, Instant};

use crate::connect_options::ConnectOptions;
use crate::stream::sync::Stream;
use crate::error::ConnectionError;
use crate::packets::error::ReadBytes;
use crate::packets::reason_codes::DisconnectReasonCode;
use crate::packets::{Disconnect, Packet, PacketType};
use crate::{EventHandler, MqttHandler, NetworkStatus};

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

impl<S> Network<S> {
    pub fn new(options: ConnectOptions, mqtt_handler: MqttHandler, to_network_r: Receiver<Packet>) -> Self {
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
where
    S: Read + Write + Sized + Unpin,
{
    pub fn connect(&mut self, stream: S) -> Result<(), ConnectionError> {
        let (network, connack) = Stream::connect(&self.options, stream)?;

        self.network = Some(network);

        self.last_network_action = Instant::now();
        if self.options.keep_alive_interval_s == 0 {
            self.perform_keep_alive = false;
        }

        self.mqtt_handler.handle_incoming_packet(&connack, &mut self.outgoing_packet_buffer)?;

        Ok(())
    }

    pub fn poll<H>(&mut self, handler: &mut H) -> Result<NetworkStatus, ConnectionError>
    where
        H: EventHandler,
    {
        match self.select(handler) {
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

    fn select<H>(&mut self, handler: &mut H) -> Result<NetworkStatus, ConnectionError>
    where
        H: EventHandler,
    {
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

        if let Some(stream) = network {
            // Read segment of stream
            {
                if stream.read_bytes()? > 0 {
                    match stream.parse_messages(incoming_packet_buffer) {
                        Err(ReadBytes::Err(err)) => return Err(err),
                        Err(ReadBytes::InsufficientBytes(_)) => (),
                        Ok(_) => (),
                    }
                }

                for packet in incoming_packet_buffer.drain(0..){
                    use Packet::*;
                    match packet{
                        PingResp => {
                            handler.handle(packet);
                            *await_pingresp = None;
                        },
                        Disconnect(_) => {
                            handler.handle(packet);
                            return Ok(NetworkStatus::IncomingDisconnect);
                        }
                        packet => {
                            if mqtt_handler.handle_incoming_packet(&packet, outgoing_packet_buffer)?{
                                handler.handle(packet);
                            }
                        }
                    }
                }

                stream.write_all_packets(outgoing_packet_buffer)?;
                *last_network_action = Instant::now();
            }

            // Write segment
            let mut flushed = false;
            while let Ok(packet) = to_network_r.try_recv(){
                flushed = stream.extend_write_buffer(&packet)?;
                let packet_type = packet.packet_type();
                println!("Handling outgoing packet: {:?}", packet.packet_type());
                mqtt_handler.handle_outgoing_packet(packet)?;

                if packet_type == PacketType::Disconnect{
                    if !flushed{
                        stream.flush_whole_buffer()?;
                        *last_network_action = Instant::now();
                    }
                    return Ok(NetworkStatus::OutgoingDisconnect);   
                }
            }
            if !flushed{
                stream.flush_whole_buffer()?;
            }


            // Keepalive process
            if *perform_keep_alive {
                if let Some(instant) = await_pingresp {
                    if *instant + Duration::from_secs(self.options.keep_alive_interval_s) < Instant::now(){
                        let disconnect = Disconnect{ reason_code: DisconnectReasonCode::KeepAliveTimeout, properties: Default::default() };
                        stream.write_packet(&Packet::Disconnect(disconnect))?;
                        return Ok(NetworkStatus::NoPingResp);
                    }
                } 
                else if *last_network_action + Duration::from_secs(self.options.keep_alive_interval_s) < Instant::now(){
                    stream.write_packet(&Packet::PingReq)?;
                    *last_network_action = Instant::now();
                    *await_pingresp = Some(Instant::now());
                }
            }

            
            Ok(NetworkStatus::Active)
        }
        else{
            Err(ConnectionError::NoNetwork)
        }
    }
}
