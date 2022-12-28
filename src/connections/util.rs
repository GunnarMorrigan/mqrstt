use std::{io::{BufReader, Cursor}, sync::Arc};

use async_rustls::webpki;
use rustls::{RootCertStore, OwnedTrustAnchor, ClientConfig, Certificate};

use crate::error::TlsError;
use super::transport::PrivateKey;

pub fn native(){

}


pub fn simple_rust_tls(ca: Vec<u8>, alpn: Option<Vec<Vec<u8>>>, client_auth: Option<(Vec<u8>, PrivateKey)>) -> Result<Arc<ClientConfig>, TlsError>{
    let mut root_cert_store = RootCertStore::empty();
    
    let ca_certs = rustls_pemfile::certs(&mut BufReader::new(Cursor::new(ca)))?;


    let trust_anchors = ca_certs.iter().map_while(|cert| {
        if let Ok(ta) = webpki::TrustAnchor::try_from_cert_der(&cert[..]) {
            Some(OwnedTrustAnchor::from_subject_spki_name_constraints(
                ta.subject,
                ta.spki,
                ta.name_constraints,
            ))
        } else {
            None
        }
    });
    root_cert_store.add_server_trust_anchors(trust_anchors);

    if root_cert_store.is_empty() {
        return Err(TlsError::NoValidRootCertInChain);
    }

    let config = ClientConfig::builder()
    .with_safe_defaults()
    .with_root_certificates(root_cert_store);

    let mut config = match client_auth {
        Some((client_cert_info, client_private_info)) => {
            let read_private_keys = match client_private_info{
                PrivateKey::RSA(rsa) => {
                    rustls_pemfile::rsa_private_keys(&mut BufReader::new(Cursor::new(rsa)))                    
                },
                PrivateKey::ECC(ecc) => {
                    rustls_pemfile::pkcs8_private_keys(&mut BufReader::new(Cursor::new(ecc)))
                },
            }.map_err(|_| TlsError::NoValidPrivateKey)?;

            let key = read_private_keys.into_iter().next().ok_or(TlsError::NoValidPrivateKey)?;

            let client_certs = rustls_pemfile::certs(&mut BufReader::new(Cursor::new(client_cert_info)))?;
            let client_cert_chain = client_certs.into_iter().map(Certificate).collect();

            config.with_single_cert(client_cert_chain, rustls::PrivateKey(key))?
        },
        None => config.with_no_client_auth(),
    };

    if let Some(alpn) = alpn {
        config.alpn_protocols.extend(alpn)
    }
    // config.enable_sni = false;

    Ok(Arc::new(config))
}