use std::io::{self, Error, ErrorKind, Read, Write};

use bytes::{Buf, BytesMut};

#[cfg(feature = "logs")]
use tracing::trace;

use crate::packets::ConnAck;
use crate::{connect_options::ConnectOptions, error::ConnectionError};
use crate::{
    create_connect_from_options,
    packets::{
        error::ReadBytes,
        reason_codes::ConnAckReasonCode,
        {FixedHeader, Packet, PacketType},
    },
};

#[derive(Debug)]
pub struct Stream<S> {
    pub stream: S,

    /// Input buffer
    const_buffer: [u8; 50000],

    /// Write buffer
    read_buffer: BytesMut,

    /// Write buffer
    write_buffer: BytesMut,
}

impl<S> Stream<S> {
    pub fn parse_messages(&mut self, incoming_packet_buffer: &mut Vec<Packet>) -> Result<(), ReadBytes<ConnectionError>> {
        loop {
            if self.read_buffer.is_empty() {
                return Ok(());
            }
            let (header, header_length) = FixedHeader::read_fixed_header(self.read_buffer.iter())?;

            if header.remaining_length + header_length > self.read_buffer.len() {
                return Err(ReadBytes::InsufficientBytes(header.remaining_length - self.read_buffer.len()));
            }

            self.read_buffer.advance(header_length);

            let buf = self.read_buffer.split_to(header.remaining_length);
            let read_packet = Packet::read(header, buf.into())?;

            #[cfg(feature = "logs")]
            trace!("Read packet from network {}", read_packet);
            let packet_type = read_packet.packet_type();
            incoming_packet_buffer.push(read_packet);

            if packet_type == PacketType::Disconnect {
                return Ok(());
            }
        }
    }
}

impl<S> Stream<S>
where
    S: Read + Write + Sized + Unpin,
{
    pub fn connect(options: &ConnectOptions, stream: S) -> Result<(Self, ConnAck), ConnectionError> {
        let mut s = Self {
            stream,
            const_buffer: [0; 1000],
            read_buffer: BytesMut::new(),
            write_buffer: BytesMut::new(),
        };

        let connect = create_connect_from_options(options);

        s.write_packet(&connect)?;

        let packet = s.read()?;
        if let Packet::ConnAck(con) = packet {
            if con.reason_code == ConnAckReasonCode::Success {
                Ok((s, con))
            } else {
                Err(ConnectionError::ConnectionRefused(con.reason_code))
            }
        } else {
            Err(ConnectionError::NotConnAck(packet))
        }
    }

    pub fn read(&mut self) -> io::Result<Packet> {
        loop {
            let (header, header_length) = match FixedHeader::read_fixed_header(self.read_buffer.iter()) {
                Ok(header) => header,
                Err(ReadBytes::InsufficientBytes(required_len)) => {
                    self.read_required_bytes(required_len)?;
                    continue;
                }
                Err(ReadBytes::Err(err)) => return Err(Error::new(ErrorKind::InvalidData, err)),
            };

            self.read_buffer.advance(header_length);

            if header.remaining_length > self.read_buffer.len() {
                self.read_required_bytes(header.remaining_length - self.read_buffer.len())?;
            }

            let buf = self.read_buffer.split_to(header.remaining_length);

            return Packet::read(header, buf.into()).map_err(|err| Error::new(ErrorKind::InvalidData, err));
        }
    }

    pub fn read_bytes(&mut self) -> io::Result<usize> {
        match self.stream.read(&mut self.const_buffer) {
            Ok(read) => {
                if read == 0 {
                    Err(io::Error::new(io::ErrorKind::ConnectionReset, "Connection reset by peer"))
                } else {
                    self.read_buffer.extend_from_slice(&self.const_buffer[0..read]);
                    Ok(read)
                }
            }
            Err(err) => match err.kind() {
                ErrorKind::WouldBlock => Ok(0),
                _ => Err(err),
            },
        }
    }

    /// Reads more than 'required' bytes to frame a packet into self.read buffer
    pub fn read_required_bytes(&mut self, required: usize) -> io::Result<usize> {
        let mut total_read = 0;

        loop {
            let read = self.read_bytes()?;
            total_read += read;
            if total_read >= required {
                return Ok(total_read);
            }
        }
    }

    pub fn extend_write_buffer(&mut self, packet: &Packet) -> Result<bool, ConnectionError> {
        packet.write(&mut self.write_buffer)?;

        #[cfg(feature = "logs")]
        trace!("Wrote packet {} to write buffer", packet);

        if self.write_buffer.len() >= 1000 {
            self.flush_whole_buffer()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn flush_whole_buffer(&mut self) -> Result<(), ConnectionError> {
        self.stream.write_all(&self.write_buffer[..])?;
        self.stream.flush()?;
        self.write_buffer.clear();
        Ok(())
    }

    pub fn write_packet(&mut self, packet: &Packet) -> Result<(), ConnectionError> {
        packet.write(&mut self.write_buffer)?;

        #[cfg(feature = "logs")]
        trace!("Sending packet {}", packet);

        self.stream.write_all(&self.write_buffer[..])?;
        self.stream.flush()?;
        self.write_buffer.clear();
        Ok(())
    }

    pub fn write_all_packets(&mut self, packets: &mut Vec<Packet>) -> Result<(), ConnectionError> {
        let writes = packets.drain(0..).map(|packet| {
            packet.write(&mut self.write_buffer)?;

            #[cfg(feature = "logs")]
            trace!("Sending packet {}", packet);

            Ok::<(), ConnectionError>(())
        });

        for write in writes {
            write?;
        }

        self.stream.write_all(&self.write_buffer[..])?;
        self.stream.flush()?;
        self.write_buffer.clear();
        Ok(())
    }
}
