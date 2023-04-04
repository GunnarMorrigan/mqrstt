use std::io::{self, Error, ErrorKind};

use bytes::{Buf, BytesMut};

use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[cfg(feature = "logs")]
use tracing::trace;

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
    const_buffer: [u8; 1000],

    /// Write buffer
    read_buffer: BytesMut,

    /// Write buffer
    write_buffer: BytesMut,
}

impl<S> Stream<S> {
    pub async fn parse_messages(&mut self, incoming_packet_buffer: &mut Vec<Packet>) -> Result<(), ReadBytes<ConnectionError>> {
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
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Sized + Unpin,
{
    pub async fn connect(options: &ConnectOptions, stream: S) -> Result<(Self, Packet), ConnectionError> {
        let mut s = Self {
            stream,
            const_buffer: [0; 1000],
            read_buffer: BytesMut::new(),
            write_buffer: BytesMut::new(),
        };

        let connect = create_connect_from_options(options);

        s.write(&connect).await?;

        let packet = s.read().await?;
        if let Packet::ConnAck(con) = packet {
            if con.reason_code == ConnAckReasonCode::Success {
                Ok((s, Packet::ConnAck(con)))
            } else {
                Err(ConnectionError::ConnectionRefused(con.reason_code))
            }
        } else {
            Err(ConnectionError::NotConnAck(packet))
        }
    }

    pub async fn read(&mut self) -> io::Result<Packet> {
        loop {
            let (header, header_length) = match FixedHeader::read_fixed_header(self.read_buffer.iter()) {
                Ok(header) => header,
                Err(ReadBytes::InsufficientBytes(required_len)) => {
                    self.read_required_bytes(required_len).await?;
                    continue;
                }
                Err(ReadBytes::Err(err)) => return Err(Error::new(ErrorKind::InvalidData, err)),
            };

            self.read_buffer.advance(header_length);

            if header.remaining_length > self.read_buffer.len() {
                self.read_required_bytes(header.remaining_length - self.read_buffer.len()).await?;
            }

            let buf = self.read_buffer.split_to(header.remaining_length);

            return Packet::read(header, buf.into()).map_err(|err| Error::new(ErrorKind::InvalidData, err));
        }
    }

    pub async fn read_bytes(&mut self) -> io::Result<usize> {
        let read = self.stream.read(&mut self.const_buffer).await?;
        if read == 0 {
            Err(io::Error::new(io::ErrorKind::ConnectionReset, "Connection reset by peer"))
        } else {
            self.read_buffer.extend_from_slice(&self.const_buffer[0..read]);
            Ok(read)
        }
    }

    /// Reads more than 'required' bytes to frame a packet into self.read buffer
    pub async fn read_required_bytes(&mut self, required: usize) -> io::Result<usize> {
        let mut total_read = 0;

        loop {
            let read = self.read_bytes().await?;
            total_read += read;
            if total_read >= required {
                return Ok(total_read);
            }
        }
    }

    pub async fn write(&mut self, packet: &Packet) -> Result<(), ConnectionError> {
        packet.write(&mut self.write_buffer)?;

        #[cfg(feature = "logs")]
        trace!("Sending packet {}", packet);

        self.stream.write_all(&self.write_buffer[..]).await?;
        self.stream.flush().await?;
        self.write_buffer.clear();
        Ok(())
    }

    pub async fn write_all(&mut self, packets: &mut Vec<Packet>) -> Result<(), ConnectionError> {
        let writes = packets.drain(0..).map(|packet| {
            packet.write(&mut self.write_buffer)?;

            #[cfg(feature = "logs")]
            trace!("Sending packet {}", packet);

            Ok::<(), ConnectionError>(())
        });

        for write in writes {
            write?;
        }

        self.stream.write_all(&self.write_buffer[..]).await?;
        self.stream.flush().await?;
        self.write_buffer.clear();
        Ok(())
    }
}
