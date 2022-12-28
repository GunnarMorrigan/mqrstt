use std::sync::Arc;
use rustls::ClientConfig;

#[derive(Debug, Clone)]
pub enum TlsConfig {
    #[cfg(any(feature = "smol-rustls", feature = "tokio-rustls"))]
    Rustls(RustlsConfig),
    #[cfg(feature = "native-tls")]
    Native {
        ca: Vec<u8>,
        der: Vec<u8>,
        password: String,
    },
}

#[cfg(any(feature = "smol-rustls", feature = "tokio-rustls"))]
#[derive(Debug, Clone)]
pub enum RustlsConfig{
    Simple {
        ca: Vec<u8>,
        alpn: Option<Vec<Vec<u8>>>,
        client_auth: Option<(Vec<u8>, PrivateKey)>,
    },
    Rustls(Arc<ClientConfig>),
}

#[derive(Debug, Clone)]
pub enum PrivateKey {
    RSA(Vec<u8>),
    ECC(Vec<u8>),
}
