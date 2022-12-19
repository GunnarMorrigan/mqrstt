pub enum Transport{
    TCP,
    TLS(TlsConfig),
    QUIC(),
}   

pub enum TlsConfig{
    TwoWay {
        ca: Vec<u8>,
        alpn: Option<Vec<Vec<u8>>>,
        client_auth: Option<(Vec<u8>, PrivateKey)>,
    },
    // SimpleNative {
    //     ca: Vec<u8>,
    //     der: Vec<u8>,
    //     password: String,
    // },
    // Rustls(Arc<ClientConfig>),
}

pub enum PrivateKey {
    RSA(Vec<u8>),
    ECC(Vec<u8>),
}