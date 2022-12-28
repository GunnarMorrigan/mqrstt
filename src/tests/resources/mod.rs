pub mod test_bytes_packet;
pub mod test_packets;

pub use test_bytes_packet::*;

pub const google: &[u8] = include_bytes!("google_certs/www-google-com.pem");
pub const google_chain: &[u8] = include_bytes!("google_certs/www-google-com-chain.pem");

pub const EMQX: &[u8] = include_bytes!("broker.emqx.io-ca.crt");