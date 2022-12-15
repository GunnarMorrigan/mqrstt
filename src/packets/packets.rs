use core::slice::Iter;


use bytes::{BytesMut, BufMut, Bytes};

use super::error::{SerializeError, DeserializeError, ReadBytes};
use super::mqtt_traits::{VariableHeaderWrite, WireLength, VariableHeaderRead};
use super::{write_variable_integer, read_fixed_header_rem_len};

use super::connect::Connect;
use super::auth::Auth;
use super::connack::ConnAck;
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

impl Packet{
    pub fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError>{
        match self{
            Packet::Connect(p) => {
                buf.put_u8(0b0001_0000);
                write_variable_integer(buf, p.wire_len())?;
                
                p.write(buf)?;
            },
            Packet::ConnAck(_) => {
                buf.put_u8(0b0010_0000);
                // write_variable_integer(buf, c.wire_len())?;
                todo!();
            },
            Packet::Publish(p) => {
                let mut first_byte = 0b0011_0000u8;
                if p.dup{
                    first_byte |= 0b1000;
                }
                
                first_byte |= p.qos.into_u8() << 1;

                if p.retain{
                    first_byte |= 0b0001;
                }
                buf.put_u8(first_byte);
                write_variable_integer(buf, p.wire_len())?;
                p.write(buf)?;
            },
            Packet::PubAck(p) => {
                buf.put_u8(0b0100_0000);
                write_variable_integer(buf, p.wire_len())?;
                p.write(buf)?;
            },
            Packet::PubRec(p) => {
                buf.put_u8(0b0101_0000);
                write_variable_integer(buf, p.wire_len())?;
                p.write(buf)?;
            },
            Packet::PubRel(p) => {
                buf.put_u8(0b0110_0010);
                write_variable_integer(buf, p.wire_len())?;
                p.write(buf)?;
            },
            Packet::PubComp(p) => {
                buf.put_u8(0b0111_0000);
                write_variable_integer(buf, p.wire_len())?;
                p.write(buf)?;
            },
            Packet::Subscribe(p) => {
                buf.put_u8(0b1000_0010);
                write_variable_integer(buf, p.wire_len())?;
                p.write(buf)?;
            },
            Packet::SubAck(_) => {
                buf.put_u8(0b1001_0000);
                // write_variable_integer(buf, p.wire_len())?;
                todo!()
            },
            Packet::Unsubscribe(p) => {
                buf.put_u8(0b1010_0010);
                write_variable_integer(buf, p.wire_len())?;
                p.write(buf)?;
            },
            Packet::UnsubAck(_) => {
                buf.put_u8(0b1011_0000);
                // write_variable_integer(buf, p.wire_len())?;
                todo!()
            },
            Packet::PingReq => {
                buf.put_u8(0b1100_0000);
            },
            Packet::PingResp => {
                buf.put_u8(0b1101_0000);
            },
            Packet::Disconnect(p) => {
                buf.put_u8(0b1110_0000);
                write_variable_integer(buf, p.wire_len())?;
                p.write(buf)?;
            },
            Packet::Auth(p) => {
                buf.put_u8(0b1111_0000);
                write_variable_integer(buf, p.wire_len())?;
                p.write(buf)?;
            },
        }
        Ok(())
    }

    pub fn read(header: FixedHeader, buf: Bytes) -> Result<Packet, DeserializeError>{
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
    pub fn is_connack(&self) -> bool{
        match self{
            Packet::ConnAck(_) => true,
            _ => false,
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
pub struct FixedHeader{
    pub packet_type: PacketType,
    pub flags: u8,
    pub remaining_length: usize,
}

impl FixedHeader{
    pub fn read_fixed_header(mut header: Iter<u8>) -> Result<(Self, usize), ReadBytes<DeserializeError>> {
        if header.len() < 2 {
            return Err(ReadBytes::InsufficientBytes(2 - header.len()))
        }

        let first_byte = header.next().unwrap();

        let (packet_type, flags) = PacketType::from_first_byte(*first_byte).map_err(|err| ReadBytes::Err(err))?;

        let (remaining_length, length) = read_fixed_header_rem_len(header)?;

        Ok(
            (
                Self{
                    packet_type,
                    flags,
                    remaining_length,
                },
                1 + length
            )
        )
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
impl PacketType{
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
            (_, _) => Err(DeserializeError::UnknownFixedHeader(value))
        }
    }
}


#[cfg(test)]
mod tests{
    

    #[test]
    fn connect_read_write(){
        #[test]
        fn read_and_write_connect2(){
            let packet = [0x10, 0x1d, 0x00, 0x04, 0x4d, 0x51, 0x54, 0x54, 0x05, 0x80, 0x00, 0x3c, 0x05, 0x11, 0xff, 0xff,
            0xff, 0xff, 0x00, 0x05, 0x39, 0x2e, 0x30, 0x2e, 0x31, 0x00, 0x04, 0x54, 0x65, 0x73, 0x74];
    
            let mut buf = bytes::BytesMut::new();
            buf.extend_from_slice(&packet);
    
            // let c = Packet::read(buf.into()).unwrap();
        
            // let mut write_buf = bytes::BytesMut::new();

            // Packet::Connect(c).write(&mut write_buf).unwrap();
            
            // assert_eq!(packet.to_vec(), write_buf.to_vec());
    
        }
    }
}