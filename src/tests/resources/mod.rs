pub mod test_bytes_packet;
pub mod test_packets;

pub use test_bytes_packet::*;

pub const EMQX_CERT: &[u8] = include_bytes!("broker.emqx.io-ca.crt");