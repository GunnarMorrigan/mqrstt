use std::io::{self, Error, ErrorKind};

use async_channel::Receiver;
use bytes::{Buf, BytesMut};
use smol::io::{AsyncReadExt, AsyncWriteExt};

use futures::{AsyncRead, AsyncWrite};

use tracing::trace;

use crate::packets::{
	error::ReadBytes,
	reason_codes::ConnAckReasonCode,
	{FixedHeader, Packet, PacketType},
};
use crate::{
	connect_options::ConnectOptions, connections::create_connect_from_options,
	error::ConnectionError,
};

#[derive(Debug)]
pub struct SmolStream<S> {
	pub stream: S,

	/// Input buffer
	const_buffer: [u8; 1000],
	/// Buffered reads
	buffer: BytesMut,
}

impl<S> SmolStream<S>
where
	S: AsyncRead + AsyncWrite + Sized + Unpin,
{
	pub async fn connect(
		options: &ConnectOptions,
		stream: S,
	) -> Result<(Self, Packet), ConnectionError> {
		let mut s = Self {
			stream,
			const_buffer: [0; 1000],
			buffer: BytesMut::new(),
		};

		let mut buf_out = BytesMut::new();

		create_connect_from_options(options).write(&mut buf_out)?;

		s.write_buffer(&mut buf_out).await?;

		let packet = s.read().await?;
		if let Packet::ConnAck(con) = packet {
			if con.reason_code == ConnAckReasonCode::Success {
				Ok((s, Packet::ConnAck(con)))
			}
			else {
				Err(ConnectionError::ConnectionRefused(con.reason_code))
			}
		}
		else {
			Err(ConnectionError::NotConnAck(packet))
		}
	}

	pub async fn parse_messages(
		&mut self,
		incoming_packet_sender: &async_channel::Sender<Packet>,
	) -> Result<Option<PacketType>, ReadBytes<ConnectionError>> {
		let mut ret_packet_type = None;
		loop {
			if self.buffer.is_empty(){
				return Ok(ret_packet_type)
			}
			let (header, header_length) = FixedHeader::read_fixed_header(self.buffer.iter())?;

			if header.remaining_length > self.buffer.len() {
				return Err(ReadBytes::InsufficientBytes(
					header.remaining_length - self.buffer.len(),
				));
			}

			self.buffer.advance(header_length);

			let buf = self.buffer.split_to(header.remaining_length);
			let read_packet = Packet::read(header, buf.into())?;
			tracing::trace!("Read packet from network {}", read_packet);
			let packet_type = read_packet.packet_type();
			incoming_packet_sender.send(read_packet).await?;

			match packet_type{
				PacketType::Disconnect => return Ok(Some(PacketType::Disconnect)),
				PacketType::PingResp => return Ok(Some(PacketType::PingResp)),
				packet_type => ret_packet_type = Some(packet_type),
			}
		}
	}

	pub async fn read(&mut self) -> io::Result<Packet> {
		loop {
			let (header, header_length) = match FixedHeader::read_fixed_header(self.buffer.iter()) {
				Ok(header) => header,
				Err(ReadBytes::InsufficientBytes(required_len)) => {
					self.read_required_bytes(required_len).await?;
					continue;
				}
				Err(ReadBytes::Err(err)) => return Err(Error::new(ErrorKind::InvalidData, err)),
			};

			self.buffer.advance(header_length);

			if header.remaining_length > self.buffer.len() {
				self.read_required_bytes(header.remaining_length - self.buffer.len())
					.await?;
			}

			let buf = self.buffer.split_to(header.remaining_length);

			return Packet::read(header, buf.into())
				.map_err(|err| Error::new(ErrorKind::InvalidData, err));
		}
	}

	pub async fn read_bytes(&mut self) -> io::Result<usize> {
		let read = self.stream.read(&mut self.const_buffer).await?;
		if 0 == read {
			return if self.buffer.is_empty() {
				Err(io::Error::new(
					io::ErrorKind::ConnectionAborted,
					"Connection closed by peer",
				))
			}
			else {
				Err(io::Error::new(
					io::ErrorKind::ConnectionReset,
					"Connection reset by peer",
				))
			};
		}
		else {
			self.buffer.extend_from_slice(&self.const_buffer[0..read]);
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

	pub async fn write_buffer(&mut self, buffer: &mut BytesMut) -> Result<(), ConnectionError> {
		if buffer.is_empty() {
			return Ok(());
		}

		self.stream.write_all(&buffer[..]).await?;
		buffer.clear();
		Ok(())
	}

	pub async fn write(&mut self, packet: &Packet) -> Result<(), ConnectionError> {
		packet.write(&mut self.buffer)?;
		trace!("Sending packet {}", packet);

		self.stream.write_all(&self.buffer[..]).await?;
		self.stream.flush().await?;
		self.buffer.clear();
		Ok(())
	}
}
