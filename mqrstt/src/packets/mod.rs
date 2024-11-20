pub mod error;
pub mod mqtt_trait;

mod macros;

mod auth;
mod connack;
mod connect;
mod disconnect;
mod puback;
mod pubcomp;
mod publish;
mod pubrec;
mod pubrel;
mod suback;
mod subscribe;
mod unsuback;
mod unsubscribe;

mod primitive;
use error::ReadError;
use mqtt_trait::PacketAsyncRead;
pub use primitive::*;

pub use auth::*;
pub use connack::*;
pub use connect::*;
pub use disconnect::*;
pub use puback::*;
pub use pubcomp::*;
pub use publish::*;
pub use pubrec::*;
pub use pubrel::*;
pub use suback::*;
pub use subscribe::*;
pub use unsuback::*;
pub use unsubscribe::*;

use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::fmt::Display;

use self::error::{DeserializeError, ReadBytes, SerializeError};
use self::mqtt_trait::{PacketRead, PacketWrite, WireLength};

// ==================== Packets ====================
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Packet {
    Connect(Connect),
    ConnAck(ConnAck),
    Publish(Publish),
    PubAck(PubAck),
    PubRec(PubRec),
    PubRel(PubRel),
    PubComp(PubComp),
    Subscribe(Subscribe),
    SubAck(SubAck),
    Unsubscribe(Unsubscribe),
    UnsubAck(UnsubAck),
    PingReq,
    PingResp,
    Disconnect(Disconnect),
    Auth(Auth),
}

impl Packet {
    pub fn packet_type(&self) -> PacketType {
        match self {
            Packet::Connect(_) => PacketType::Connect,
            Packet::ConnAck(_) => PacketType::ConnAck,
            Packet::Publish(_) => PacketType::Publish,
            Packet::PubAck(_) => PacketType::PubAck,
            Packet::PubRec(_) => PacketType::PubRec,
            Packet::PubRel(_) => PacketType::PubRel,
            Packet::PubComp(_) => PacketType::PubComp,
            Packet::Subscribe(_) => PacketType::Subscribe,
            Packet::SubAck(_) => PacketType::SubAck,
            Packet::Unsubscribe(_) => PacketType::Unsubscribe,
            Packet::UnsubAck(_) => PacketType::UnsubAck,
            Packet::PingReq => PacketType::PingReq,
            Packet::PingResp => PacketType::PingResp,
            Packet::Disconnect(_) => PacketType::Disconnect,
            Packet::Auth(_) => PacketType::Auth,
        }
    }

    pub(crate) fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError> {
        match self {
            Packet::Connect(p) => {
                buf.put_u8(0b0001_0000);
                p.wire_len().write_variable_integer(buf)?;

                p.write(buf)?;
            }
            Packet::ConnAck(p) => {
                buf.put_u8(0b0010_0000);
                p.wire_len().write_variable_integer(buf)?;
                p.write(buf)?;
            }
            Packet::Publish(p) => {
                let mut first_byte = 0b0011_0000u8;
                if p.dup {
                    first_byte |= 0b1000;
                }

                first_byte |= p.qos.into_u8() << 1;

                if p.retain {
                    first_byte |= 0b0001;
                }
                buf.put_u8(first_byte);
                p.wire_len().write_variable_integer(buf)?;
                p.write(buf)?;
            }
            Packet::PubAck(p) => {
                buf.put_u8(0b0100_0000);
                p.wire_len().write_variable_integer(buf)?;
                p.write(buf)?;
            }
            Packet::PubRec(p) => {
                buf.put_u8(0b0101_0000);
                p.wire_len().write_variable_integer(buf)?;
                p.write(buf)?;
            }
            Packet::PubRel(p) => {
                buf.put_u8(0b0110_0010);
                p.wire_len().write_variable_integer(buf)?;
                p.write(buf)?;
            }
            Packet::PubComp(p) => {
                buf.put_u8(0b0111_0000);
                p.wire_len().write_variable_integer(buf)?;
                p.write(buf)?;
            }
            Packet::Subscribe(p) => {
                buf.put_u8(0b1000_0010);
                p.wire_len().write_variable_integer(buf)?;
                p.write(buf)?;
            }
            Packet::SubAck(_) => {
                unreachable!()
            }
            Packet::Unsubscribe(p) => {
                buf.put_u8(0b1010_0010);
                p.wire_len().write_variable_integer(buf)?;
                p.write(buf)?;
            }
            Packet::UnsubAck(_) => {
                unreachable!();
                // buf.put_u8(0b1011_0000);
            }
            Packet::PingReq => {
                buf.put_u8(0b1100_0000);
                buf.put_u8(0); // Variable header length.
            }
            Packet::PingResp => {
                buf.put_u8(0b1101_0000);
                buf.put_u8(0); // Variable header length.
            }
            Packet::Disconnect(p) => {
                buf.put_u8(0b1110_0000);
                p.wire_len().write_variable_integer(buf)?;
                p.write(buf)?;
            }
            Packet::Auth(p) => {
                buf.put_u8(0b1111_0000);
                p.wire_len().write_variable_integer(buf)?;
                p.write(buf)?;
            }
        }
        Ok(())
    }

    pub(crate) fn read(header: FixedHeader, buf: Bytes) -> Result<Packet, DeserializeError> {
        let packet = match header.packet_type {
            PacketType::Connect => Packet::Connect(Connect::read(header.flags, header.remaining_length, buf)?),
            PacketType::ConnAck => Packet::ConnAck(ConnAck::read(header.flags, header.remaining_length, buf)?),
            PacketType::Publish => Packet::Publish(Publish::read(header.flags, header.remaining_length, buf)?),
            PacketType::PubAck => Packet::PubAck(PubAck::read(header.flags, header.remaining_length, buf)?),
            PacketType::PubRec => Packet::PubRec(PubRec::read(header.flags, header.remaining_length, buf)?),
            PacketType::PubRel => Packet::PubRel(PubRel::read(header.flags, header.remaining_length, buf)?),
            PacketType::PubComp => Packet::PubComp(PubComp::read(header.flags, header.remaining_length, buf)?),
            PacketType::Subscribe => Packet::Subscribe(Subscribe::read(header.flags, header.remaining_length, buf)?),
            PacketType::SubAck => Packet::SubAck(SubAck::read(header.flags, header.remaining_length, buf)?),
            PacketType::Unsubscribe => Packet::Unsubscribe(Unsubscribe::read(header.flags, header.remaining_length, buf)?),
            PacketType::UnsubAck => Packet::UnsubAck(UnsubAck::read(header.flags, header.remaining_length, buf)?),
            PacketType::PingReq => Packet::PingReq,
            PacketType::PingResp => Packet::PingResp,
            PacketType::Disconnect => Packet::Disconnect(Disconnect::read(header.flags, header.remaining_length, buf)?),
            PacketType::Auth => Packet::Auth(Auth::read(header.flags, header.remaining_length, buf)?),
        };
        Ok(packet)
    }

    pub(crate) async fn async_read<S>(header: FixedHeader, stream: &mut S) -> Result<Packet, ReadError>
    where
        S: tokio::io::AsyncRead + Unpin,
    {
        let packet = match header.packet_type {
            PacketType::Connect => Packet::Connect(Connect::async_read(header.flags, header.remaining_length, stream).await?.0),
            PacketType::ConnAck => Packet::ConnAck(ConnAck::async_read(header.flags, header.remaining_length, stream).await?.0),
            PacketType::Publish => Packet::Publish(Publish::async_read(header.flags, header.remaining_length, stream).await?.0),
            PacketType::PubAck => Packet::PubAck(PubAck::async_read(header.flags, header.remaining_length, stream).await?.0),
            PacketType::PubRec => Packet::PubRec(PubRec::async_read(header.flags, header.remaining_length, stream).await?.0),
            PacketType::PubRel => Packet::PubRel(PubRel::async_read(header.flags, header.remaining_length, stream).await?.0),
            PacketType::PubComp => Packet::PubComp(PubComp::async_read(header.flags, header.remaining_length, stream).await?.0),
            PacketType::Subscribe => Packet::Subscribe(Subscribe::async_read(header.flags, header.remaining_length, stream).await?.0),
            PacketType::SubAck => Packet::SubAck(SubAck::async_read(header.flags, header.remaining_length, stream).await?.0),
            PacketType::Unsubscribe => Packet::Unsubscribe(Unsubscribe::async_read(header.flags, header.remaining_length, stream).await?.0),
            PacketType::UnsubAck => Packet::UnsubAck(UnsubAck::async_read(header.flags, header.remaining_length, stream).await?.0),
            PacketType::PingReq => Packet::PingReq,
            PacketType::PingResp => Packet::PingResp,
            PacketType::Disconnect => Packet::Disconnect(Disconnect::async_read(header.flags, header.remaining_length, stream).await?.0),
            PacketType::Auth => Packet::Auth(Auth::async_read(header.flags, header.remaining_length, stream).await?.0),
        };
        Ok(packet)
    }

    #[cfg(test)]
    pub(crate) async fn async_read_from_buffer<S>(stream: &mut S) -> Result<Packet, ReadError>
    where
        S: tokio::io::AsyncRead + Unpin,
    {
        let (header, _) = FixedHeader::async_read(stream).await?;

        Ok(Packet::async_read(header, stream).await?)
    }

    #[cfg(test)]
    pub(crate) fn read_from_buffer(buffer: &mut BytesMut) -> Result<Packet, ReadBytes<DeserializeError>> {
        let (header, header_length) = FixedHeader::read_fixed_header(buffer.iter())?;
        if header.remaining_length + header_length > buffer.len() {
            return Err(ReadBytes::InsufficientBytes(header.remaining_length + header_length - buffer.len()));
        }
        buffer.advance(header_length);

        let buf = buffer.split_to(header.remaining_length);

        Ok(Packet::read(header, buf.into())?)
    }
}

impl Display for Packet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Packet::Connect(c) => write!(
                f,
                "Connect(version: {:?}, clean: {}, username: {:?}, password: {:?}, keep_alive: {}, client_id: {})",
                c.protocol_version, c.clean_start, c.username, c.password, c.keep_alive, c.client_id
            ),
            Packet::ConnAck(c) => write!(f, "ConnAck(session:{:?}, reason code{:?})", c.connack_flags, c.reason_code),
            Packet::Publish(p) => write!(
                f,
                "Publish(topic: {}, qos: {:?}, dup: {:?}, retain: {:?}, packet id: {:?})",
                &p.topic, p.qos, p.dup, p.retain, p.packet_identifier
            ),
            Packet::PubAck(p) => write!(f, "PubAck(id:{:?}, reason code: {:?})", p.packet_identifier, p.reason_code),
            Packet::PubRec(p) => write!(f, "PubRec(id: {}, reason code: {:?})", p.packet_identifier, p.reason_code),
            Packet::PubRel(p) => write!(f, "PubRel(id: {}, reason code: {:?})", p.packet_identifier, p.reason_code),
            Packet::PubComp(p) => write!(f, "PubComp(id: {}, reason code: {:?})", p.packet_identifier, p.reason_code),
            Packet::Subscribe(_) => write!(f, "Subscribe()"),
            Packet::SubAck(_) => write!(f, "SubAck()"),
            Packet::Unsubscribe(_) => write!(f, "Unsubscribe()"),
            Packet::UnsubAck(_) => write!(f, "UnsubAck()"),
            Packet::PingReq => write!(f, "PingReq"),
            Packet::PingResp => write!(f, "PingResp"),
            Packet::Disconnect(d) => write!(f, "Disconnect(reason code: {:?})", d.reason_code),
            Packet::Auth(_) => write!(f, "Auth()"),
        }
    }
}

/// 2.1.2 MQTT Control Packet type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum PacketType {
    Connect,
    ConnAck,
    Publish,
    PubAck,
    PubRec,
    PubRel,
    PubComp,
    Subscribe,
    SubAck,
    Unsubscribe,
    UnsubAck,
    PingReq,
    PingResp,
    Disconnect,
    Auth,
}
impl PacketType {
    #[inline]
    const fn from_first_byte(value: u8) -> Result<(Self, u8), DeserializeError> {
        match (value >> 4, value & 0x0f) {
            (0b0001, 0) => Ok((PacketType::Connect, 0)),
            (0b0010, 0) => Ok((PacketType::ConnAck, 0)),
            (0b0011, flags) => Ok((PacketType::Publish, flags)),
            (0b0100, 0) => Ok((PacketType::PubAck, 0)),
            (0b0101, 0) => Ok((PacketType::PubRec, 0)),
            (0b0110, 0b0010) => Ok((PacketType::PubRel, 0)),
            (0b0111, 0) => Ok((PacketType::PubComp, 0)),
            (0b1000, 0b0010) => Ok((PacketType::Subscribe, 0)),
            (0b1001, 0) => Ok((PacketType::SubAck, 0)),
            (0b1010, 0b0010) => Ok((PacketType::Unsubscribe, 0)),
            (0b1011, 0) => Ok((PacketType::UnsubAck, 0)),
            (0b1100, 0) => Ok((PacketType::PingReq, 0)),
            (0b1101, 0) => Ok((PacketType::PingResp, 0)),
            (0b1110, 0) => Ok((PacketType::Disconnect, 0)),
            (0b1111, 0) => Ok((PacketType::Auth, 0)),
            (_, _) => Err(DeserializeError::UnknownFixedHeader(value)),
        }
    }
}

impl std::fmt::Display for PacketType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self, f)
    }
}

#[cfg(test)]
mod tests {
    use bytes::BytesMut;

    use crate::packets::connack::{ConnAck, ConnAckFlags, ConnAckProperties};
    use crate::packets::disconnect::{Disconnect, DisconnectProperties};
    use crate::packets::QoS;

    use crate::packets::connack::ConnAckReasonCode;
    use crate::packets::disconnect::DisconnectReasonCode;
    use crate::packets::publish::{Publish, PublishProperties};
    use crate::packets::pubrel::PubRelReasonCode;
    use crate::packets::pubrel::{PubRel, PubRelProperties};
    use crate::packets::Packet;

    use crate::tests::test_packets::{disconnect_case, ping_req_case, ping_resp_case, publish_case, pubrel_case, pubrel_smallest_case};

    #[rstest::rstest]
    #[case(disconnect_case())]
    #[case(ping_req_case())]
    #[case(ping_resp_case())]
    #[case(publish_case())]
    #[case(pubrel_case())]
    #[case(pubrel_smallest_case())]
    fn test_read_write_cases(#[case] (bytes, expected_packet): (&[u8], Packet)) {
        let mut buffer = BytesMut::from_iter(bytes);

        let res = Packet::read_from_buffer(&mut buffer);

        assert!(res.is_ok());

        let packet = res.unwrap();

        assert_eq!(packet, expected_packet);

        buffer.clear();

        packet.write(&mut buffer).unwrap();

        assert_eq!(buffer.to_vec(), bytes.to_vec())
    }

    #[rstest::rstest]
    #[case(disconnect_case())]
    #[case(ping_req_case())]
    #[case(ping_resp_case())]
    #[case(publish_case())]
    #[case(pubrel_case())]
    #[case(pubrel_smallest_case())]
    #[tokio::test]
    async fn test_async_read_write(#[case] (mut bytes, expected_packet): (&[u8], Packet)) {
        // let mut buffer = BytesMut::from(bytes);

        let res = Packet::async_read_from_buffer(&mut bytes).await;

        dbg!(&res);
        assert!(res.is_ok());

        let packet = res.unwrap();

        assert_eq!(packet, expected_packet);

        // packet.write(&mut buffer).unwrap();

        // assert_eq!()
    }
}
