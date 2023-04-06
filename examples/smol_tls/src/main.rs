use std::{io::{BufReader, Cursor}, sync::Arc};

use async_trait::async_trait;
use mqrstt::{MqttClient, AsyncEventHandler, packets::{self, Packet}, ConnectOptions, new_smol, NetworkStatus};
use rustls::{RootCertStore, OwnedTrustAnchor, ClientConfig, Certificate, ServerName};


pub const EMQX_CERT: &[u8] = include_bytes!("broker.emqx.io-ca.crt");


pub struct PingPong {
    pub client: MqttClient,
}

#[async_trait]
impl AsyncEventHandler for PingPong {
    // Handlers only get INCOMING packets. This can change later.
    async fn handle(&mut self, event: packets::Packet) -> () {
        match event {
            Packet::Publish(p) => {
                if let Ok(payload) = String::from_utf8(p.payload.to_vec()) {
                    if payload.to_lowercase().contains("ping") {
                        self.client
                            .publish(
                                p.topic.clone(),
                                p.qos,
                                p.retain,
                                "pong",
                            )
                            .await
                            .unwrap();
                        println!("Received Ping, Send pong!");
                    }
                }
            },
            Packet::ConnAck(_) => { println!("Connected!") },
            _ => (),
        }
    }
}

#[derive(Debug, Clone)]
pub enum PrivateKey {
    RSA(Vec<u8>),
    ECC(Vec<u8>),
}

pub fn simple_rust_tls(ca: Vec<u8>, alpn: Option<Vec<Vec<u8>>>, client_auth: Option<(Vec<u8>, PrivateKey)>) -> Result<Arc<ClientConfig>, rustls::Error> {
    let mut root_cert_store = RootCertStore::empty();

    let ca_certs = rustls_pemfile::certs(&mut BufReader::new(Cursor::new(ca))).unwrap();

    let trust_anchors = ca_certs.iter().map_while(|cert| {
        if let Ok(ta) = webpki::TrustAnchor::try_from_cert_der(&cert[..]) {
            Some(OwnedTrustAnchor::from_subject_spki_name_constraints(ta.subject, ta.spki, ta.name_constraints))
        } else {
            None
        }
    });
    root_cert_store.add_server_trust_anchors(trust_anchors);

    assert!(!root_cert_store.is_empty());

    let config = ClientConfig::builder().with_safe_defaults().with_root_certificates(root_cert_store);

    let mut config = match client_auth {
        Some((client_cert_info, client_private_info)) => {
            let read_private_keys = match client_private_info {
                PrivateKey::RSA(rsa) => rustls_pemfile::rsa_private_keys(&mut BufReader::new(Cursor::new(rsa))),
                PrivateKey::ECC(ecc) => rustls_pemfile::pkcs8_private_keys(&mut BufReader::new(Cursor::new(ecc))),
            }
            .unwrap();

            let key = read_private_keys.into_iter().next().unwrap();

            let client_certs = rustls_pemfile::certs(&mut BufReader::new(Cursor::new(client_cert_info))).unwrap();
            let client_cert_chain = client_certs.into_iter().map(Certificate).collect();

            config.with_single_cert(client_cert_chain, rustls::PrivateKey(key))?
        }
        None => config.with_no_client_auth(),
    };

    if let Some(alpn) = alpn {
        config.alpn_protocols.extend(alpn)
    }

    Ok(Arc::new(config))
}

fn main() {
    smol::block_on(async {
        let client_id = "SmolTls_MQrsTT_Example".to_string();
        let options = ConnectOptions::new(client_id);

        let address = "broker.emqx.io";
        let port = 8883;

        let (mut network, client) = new_smol(options);

        let arc_client_config = simple_rust_tls(EMQX_CERT.to_vec(), None, None).unwrap();

        let domain = ServerName::try_from(address).unwrap();
        let connector = async_rustls::TlsConnector::from(arc_client_config);

        let stream = smol::net::TcpStream::connect((address, port)).await.unwrap();
        let connection = connector.connect(domain, stream).await.unwrap();

        let mut pingpong = PingPong { client: client.clone() };

        network.connect(connection, &mut pingpong).await.unwrap();

        client.subscribe("mqrstt").await.unwrap();

        let (n, _) = futures::join!(
            async {
                loop {
                    return match network.poll(&mut pingpong).await {
                        Ok(NetworkStatus::Active) => continue,
                        otherwise => otherwise,
                    };
                }
            },
            async {
                smol::Timer::after(std::time::Duration::from_secs(30)).await;
                client.disconnect().await.unwrap();
            }
        );
        assert!(n.is_ok());
    });
}
