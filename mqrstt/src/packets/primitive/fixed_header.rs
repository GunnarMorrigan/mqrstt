use core::slice::Iter;
use crate::packets::{error::{DeserializeError, ReadBytes}, PacketType};

use super::read_fixed_header_rem_len;

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
pub(crate) struct FixedHeader {
    pub packet_type: PacketType,
    pub flags: u8,
    pub remaining_length: usize,
}

impl FixedHeader {
    pub fn read_fixed_header(mut header: Iter<u8>) -> Result<(Self, usize), ReadBytes<DeserializeError>> {
        if header.len() < 2 {
            return Err(ReadBytes::InsufficientBytes(2 - header.len()));
        }

        let mut header_length = 1;
        let first_byte = header.next().unwrap();

        let (packet_type, flags) = PacketType::from_first_byte(*first_byte).map_err(ReadBytes::Err)?;

        let (remaining_length, length) = read_fixed_header_rem_len(header)?;
        header_length += length;

        Ok((Self { packet_type, flags, remaining_length }, header_length))
    }
}