use core::slice::Iter;
use std::fmt::Display;

use bytes::{BufMut, Bytes, BytesMut, Buf};

use super::error::{DeserializeError, ReadBytes, SerializeError};
use super::mqtt_traits::{VariableHeaderRead, VariableHeaderWrite, WireLength};
use super::{read_fixed_header_rem_len, write_variable_integer};

use super::auth::Auth;
use super::connack::ConnAck;
use super::connect::Connect;
use super::disconnect::Disconnect;
use super::puback::PubAck;
use super::pubcomp::PubComp;
use super::publish::Publish;
use super::pubrec::PubRec;
use super::pubrel::PubRel;
use super::suback::SubAck;
use super::subscribe::Subscribe;
use super::unsuback::UnsubAck;
use super::unsubscribe::Unsubscribe;

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

    pub fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError> {
        match self {
            Packet::Connect(p) => {
                buf.put_u8(0b0001_0000);
                write_variable_integer(buf, p.wire_len())?;

                p.write(buf)?;
            }
            Packet::ConnAck(_) => {
                buf.put_u8(0b0010_0000);
                todo!();
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
                write_variable_integer(buf, p.wire_len())?;
                p.write(buf)?;
            }
            Packet::PubAck(p) => {
                buf.put_u8(0b0100_0000);
                write_variable_integer(buf, p.wire_len())?;
                p.write(buf)?;
            }
            Packet::PubRec(p) => {
                buf.put_u8(0b0101_0000);
                write_variable_integer(buf, p.wire_len())?;
                p.write(buf)?;
            }
            Packet::PubRel(p) => {
                buf.put_u8(0b0110_0010);
                write_variable_integer(buf, p.wire_len())?;
                p.write(buf)?;
            }
            Packet::PubComp(p) => {
                buf.put_u8(0b0111_0000);
                write_variable_integer(buf, p.wire_len())?;
                p.write(buf)?;
            }
            Packet::Subscribe(p) => {
                buf.put_u8(0b1000_0010);
                write_variable_integer(buf, p.wire_len())?;
                p.write(buf)?;
            }
            Packet::SubAck(_) => {
                buf.put_u8(0b1001_0000);
                // write_variable_integer(buf, p.wire_len())?;
                todo!()
            }
            Packet::Unsubscribe(p) => {
                buf.put_u8(0b1010_0010);
                write_variable_integer(buf, p.wire_len())?;
                p.write(buf)?;
            }
            Packet::UnsubAck(_) => {
                buf.put_u8(0b1011_0000);
                // write_variable_integer(buf, p.wire_len())?;
                todo!()
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
                write_variable_integer(buf, p.wire_len())?;
                p.write(buf)?;
            }
            Packet::Auth(p) => {
                buf.put_u8(0b1111_0000);
                write_variable_integer(buf, p.wire_len())?;
                p.write(buf)?;
            }
        }
        Ok(())
    }

    pub fn read(header: FixedHeader, buf: Bytes) -> Result<Packet, DeserializeError> {
        let packet = match header.packet_type {
            PacketType::Connect => {
                Packet::Connect(Connect::read(header.flags, header.remaining_length, buf)?)
            }
            PacketType::ConnAck => {
                Packet::ConnAck(ConnAck::read(header.flags, header.remaining_length, buf)?)
            }
            PacketType::Publish => {
                Packet::Publish(Publish::read(header.flags, header.remaining_length, buf)?)
            }
            PacketType::PubAck => {
                Packet::PubAck(PubAck::read(header.flags, header.remaining_length, buf)?)
            }
            PacketType::PubRec => {
                Packet::PubRec(PubRec::read(header.flags, header.remaining_length, buf)?)
            }
            PacketType::PubRel => {
                Packet::PubRel(PubRel::read(header.flags, header.remaining_length, buf)?)
            }
            PacketType::PubComp => {
                Packet::PubComp(PubComp::read(header.flags, header.remaining_length, buf)?)
            }
            PacketType::Subscribe => {
                Packet::Subscribe(Subscribe::read(header.flags, header.remaining_length, buf)?)
            }
            PacketType::SubAck => {
                Packet::SubAck(SubAck::read(header.flags, header.remaining_length, buf)?)
            }
            PacketType::Unsubscribe => Packet::Unsubscribe(Unsubscribe::read(
                header.flags,
                header.remaining_length,
                buf,
            )?),
            PacketType::UnsubAck => {
                Packet::UnsubAck(UnsubAck::read(header.flags, header.remaining_length, buf)?)
            }
            PacketType::PingReq => Packet::PingReq,
            PacketType::PingResp => Packet::PingResp,
            PacketType::Disconnect => Packet::Disconnect(Disconnect::read(
                header.flags,
                header.remaining_length,
                buf,
            )?),
            PacketType::Auth => {
                Packet::Auth(Auth::read(header.flags, header.remaining_length, buf)?)
            }
        };
        Ok(packet)
    }

    pub fn read_from_buffer(buffer: &mut BytesMut) -> Result<Packet, ReadBytes<DeserializeError>> {
        let (header, header_length) = FixedHeader::read_fixed_header(buffer.iter())?;

        if header.remaining_length + header_length > buffer.len() {
            return Err(ReadBytes::InsufficientBytes(
                header.remaining_length - buffer.len(),
            ));
        }
        buffer.advance(header_length);

        let buf = buffer.split_to(header.remaining_length);

        return Ok(Packet::read(header, buf.into())?);
    }
}

impl Display for Packet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self{
            Packet::Connect(c) => write!(f, "Connect(version: {:?}, clean: {}, username: {:?}, password: {:?}, keep_alive: {}, client_id: {})", c.protocol_version, c.clean_session, c.username, c.password, c.keep_alive, c.client_id),
            Packet::ConnAck(c) => write!(f, "ConnAck(session:{:?}, reason code{:?})", c.connack_flags, c.reason_code),
            Packet::Publish(p) => write!(f, "Publish(topic: {}, qos: {:?}, dup: {:?}, retain: {:?}, packet id: {:?})", &p.topic, p.qos, p.dup, p.retain, p.packet_identifier),
            Packet::PubAck(p) => write!(f, "PubAck(id:{:?}, reason code{:?})", p.packet_identifier, p.reason_code),
            Packet::PubRec(p) => write!(f, "PubRec(id: {}, reason code: {:?})", p.packet_identifier, p.reason_code),
            Packet::PubRel(p) => write!(f, "PubRel(id: {}, reason code: {:?})", p.packet_identifier, p.reason_code),
            Packet::PubComp(p) => write!(f, "PubComp(id: {}, reason code: {:?})", p.packet_identifier, p.reason_code),
            Packet::Subscribe(_) => write!(f, "Subscribe()"),
            Packet::SubAck(_) => write!(f, "SubAck()"),
            Packet::Unsubscribe(_) => write!(f, "Unsubscribe()"),
            Packet::UnsubAck(_) => write!(f, "UnsubAck()"),
            Packet::PingReq => write!(f, "PingReq"),
            Packet::PingResp => write!(f, "PingResp"),
            Packet::Disconnect(_) => write!(f, "Disconnect()"),
            Packet::Auth(_) => write!(f, "Auth()"),
        }
    }
}

// 2.1.1 Fixed Header
// ```
//          7                          3                          0
//          +--------------------------+--------------------------+
// byte 1   | MQTT Control Packet Type | Flags for Packet type    |
//          +--------------------------+--------------------------+
//          |                   Remaining Length                  |
//          +-----------------------------------------------------+
//
// https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901021
// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub struct FixedHeader {
    pub packet_type: PacketType,
    pub flags: u8,
    pub remaining_length: usize,
}

impl FixedHeader {
    pub fn read_fixed_header(
        mut header: Iter<u8>,
    ) -> Result<(Self, usize), ReadBytes<DeserializeError>> {
        if header.len() < 2 {
            return Err(ReadBytes::InsufficientBytes(2 - header.len()));
        }

        let first_byte = header.next().unwrap();

        let (packet_type, flags) =
            PacketType::from_first_byte(*first_byte).map_err(ReadBytes::Err)?;

        let (remaining_length, length) = read_fixed_header_rem_len(header)?;

        Ok((
            Self {
                packet_type,
                flags,
                remaining_length,
            },
            1 + length,
        ))
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

#[cfg(test)]
mod tests {
    use bytes::{Buf, Bytes, BytesMut};

    use crate::packets::connack::{ConnAck, ConnAckFlags, ConnAckProperties};
    use crate::packets::disconnect::{Disconnect, DisconnectProperties};
    use crate::packets::QoS;

    use crate::packets::error::{DeserializeError, ReadBytes};
    use crate::packets::packets::{FixedHeader, Packet};
    use crate::packets::publish::{Publish, PublishProperties};
    use crate::packets::pubrel::{PubRel, PubRelProperties};
    use crate::packets::reason_codes::{ConnAckReasonCode, DisconnectReasonCode, PubRelReasonCode};

    #[test]
    fn test_connack_read() {
        let connack = [
            0x20, 0x13, 0x01, 0x00, 0x10, 0x27, 0x00, 0x10, 0x00, 0x00, 0x25, 0x01, 0x2a, 0x01,
            0x29, 0x01, 0x22, 0xff, 0xff, 0x28, 0x01,
        ];
        let mut buf = BytesMut::new();
        buf.extend(connack);

        let res = Packet::read_from_buffer(&mut buf);
        assert!(res.is_ok());
        let res = res.unwrap();

        let expected = ConnAck {
            connack_flags: ConnAckFlags::SESSION_PRESENT,
            reason_code: ConnAckReasonCode::Success,
            connack_properties: ConnAckProperties {
                session_expiry_interval: None,
                receive_maximum: None,
                maximum_qos: None,
                retain_available: Some(true),
                maximum_packet_size: Some(1048576),
                assigned_client_id: None,
                topic_alias_maximum: Some(65535),
                reason_string: None,
                user_properties: vec![],
                wildcards_available: Some(true),
                subscription_ids_available: Some(true),
                shared_subscription_available: Some(true),
                server_keep_alive: None,
                response_info: None,
                server_reference: None,
                authentication_method: None,
                authentication_data: None,
            },
        };

        assert_eq!(Packet::ConnAck(expected), res);
    }

    #[test]
    fn test_disconnect_read() {
        let packet = [0xe0, 0x02, 0x8e, 0x00];
        let mut buf = BytesMut::new();
        buf.extend(packet);

        let res = Packet::read_from_buffer(&mut buf);
        assert!(res.is_ok());
        let res = res.unwrap();

        let expected = Disconnect {
            reason_code: DisconnectReasonCode::SessionTakenOver,
            properties: DisconnectProperties {
                session_expiry_interval: None,
                reason_string: None,
                user_properties: vec![],
                server_reference: None,
            },
        };

        assert_eq!(Packet::Disconnect(expected), res);
    }

    #[test]
    fn test_pingreq_read_write() {
        let packet = [0xc0, 0x00];
        let mut buf = BytesMut::new();
        buf.extend(packet);

        let res = Packet::read_from_buffer(&mut buf);
        assert!(res.is_ok());
        let res = res.unwrap();

        assert_eq!(Packet::PingReq, res);

        buf.clear();
        Packet::PingReq.write(&mut buf).unwrap();
        assert_eq!(buf.to_vec(), packet);
    }

    #[test]
    fn test_pingresp_read_write() {
        let packet = [0xd0, 0x00];
        let mut buf = BytesMut::new();
        buf.extend(packet);

        let res = Packet::read_from_buffer(&mut buf);
        assert!(res.is_ok());
        let res = res.unwrap();

        assert_eq!(Packet::PingResp, res);

        buf.clear();
        Packet::PingResp.write(&mut buf).unwrap();
        assert_eq!(buf.to_vec(), packet);
    }

    #[test]
    fn test_publish_read() {
        let packet = [
            0x35, 0x24, 0x00, 0x14, 0x74, 0x65, 0x73, 0x74, 0x2f, 0x31, 0x32, 0x33, 0x2f, 0x74,
            0x65, 0x73, 0x74, 0x2f, 0x62, 0x6c, 0x61, 0x62, 0x6c, 0x61, 0x35, 0xd3, 0x0b, 0x01,
            0x01, 0x09, 0x00, 0x04, 0x31, 0x32, 0x31, 0x32, 0x0b, 0x01,
        ];

        let mut buf = BytesMut::new();
        buf.extend(packet);

        let res = Packet::read_from_buffer(&mut buf);
        assert!(res.is_ok());
        let res = res.unwrap();

        let expected = Publish {
            dup: false,
            qos: QoS::ExactlyOnce,
            retain: true,
            topic: "test/123/test/blabla".to_string(),
            packet_identifier: Some(13779),
            publish_properties: PublishProperties {
                payload_format_indicator: Some(1),
                message_expiry_interval: None,
                topic_alias: None,
                response_topic: None,
                correlation_data: Some(Bytes::from_static(b"1212")),
                subscription_identifier: vec![1],
                user_properties: vec![],
                content_type: None,
            },
            payload: Bytes::from_static(b""),
        };

        assert_eq!(Packet::Publish(expected), res);
    }

    #[test]
    fn test_pubrel_read_write() {
        let bytes = [0x62, 0x03, 0x35, 0xd3, 0x00];

        let mut buffer = BytesMut::from_iter(&bytes);

        let res = Packet::read_from_buffer(&mut buffer);

        assert!(res.is_ok());

        let packet = res.unwrap();

        let expected = PubRel {
            packet_identifier: 13779,
            reason_code: PubRelReasonCode::Success,
            properties: PubRelProperties {
                reason_string: None,
                user_properties: vec![],
            },
        };

        assert_eq!(packet, Packet::PubRel(expected));

        buffer.clear();

        packet.write(&mut buffer).unwrap();

        // The input is not in the smallest possible format but when writing we do expect it to be in the smallest possible format.
        assert_eq!(buffer.to_vec(), [0x62, 0x02, 0x35, 0xd3].to_vec())
    }

    #[test]
    fn test_pubrel_read_smallest_format() {
        let bytes = [0x62, 0x02, 0x35, 0xd3];

        let mut buffer = BytesMut::from_iter(&bytes);

        let res = Packet::read_from_buffer(&mut buffer);

        assert!(res.is_ok());

        let packet = res.unwrap();

        let expected = PubRel {
            packet_identifier: 13779,
            reason_code: PubRelReasonCode::Success,
            properties: PubRelProperties {
                reason_string: None,
                user_properties: vec![],
            },
        };

        assert_eq!(packet, Packet::PubRel(expected));

        buffer.clear();

        packet.write(&mut buffer).unwrap();

        // The input is not in the smallest possible format but when writing we do expect it to be in the smallest possible format.
        assert_eq!(buffer.to_vec(), bytes.to_vec())
    }
}
