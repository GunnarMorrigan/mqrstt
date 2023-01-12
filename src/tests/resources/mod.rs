#[cfg(test)]
mod test_bytes_packet;

#[cfg(test)]
pub use test_bytes_packet::*;

pub const EMQX_CERT: &[u8] = include_bytes!("broker.emqx.io-ca.crt");
