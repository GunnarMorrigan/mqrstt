use core::slice::Iter;

use tokio::io::AsyncReadExt;

use crate::packets::{
    error::{DeserializeError, ReadBytes},
    PacketType,
};

use super::read_fixed_header_rem_len;

/// 2.1.1 Fixed Header
///
/// The fixed header indicates the pakcet type in the first four bits [7 - 4] and for some packets it also contains some flags in the second four bits [3 - 0].
/// The remaining length encodes the length of the variable header and the payload.
///
/// | Bit      | 7 - 4                      | 3 - 0                      |
/// |----------|----------------------------|----------------------------|
/// | byte 1   | MQTT Control Packet Type   | Flags for Packet type      |
/// |          |                            |                            |
/// | byte 2+  |                   Remaining Length                      |
/// |          |---------------------------------------------------------|
///
/// [MQTT v5.0 Specification](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901021)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub(crate) struct FixedHeader {
    pub packet_type: PacketType,
    pub flags: u8,
    pub remaining_length: usize,
}

impl FixedHeader {
    pub(crate) fn read_fixed_header(mut header: Iter<u8>) -> Result<(Self, usize), ReadBytes<DeserializeError>> {
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

    pub(crate) async fn async_read<S>(stream: &mut S) -> Result<(Self, usize), crate::packets::error::ReadError>
    where
        S: tokio::io::AsyncRead + Unpin,
    {
        let first_byte = stream.read_u8().await?;

        let (packet_type, flags) = PacketType::from_first_byte(first_byte)?;

        let (remaining_length, length) = super::async_read_fixed_header_rem_len(stream).await?;
        Ok((Self { packet_type, flags, remaining_length }, 1 + length))
    }
}
