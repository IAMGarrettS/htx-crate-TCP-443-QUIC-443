use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_rustls::TlsAcceptor;
use tokio_rustls::rustls::{ServerConfig, Certificate, PrivateKey};
use std::fs::File;
use std::io::BufReader;
use tracing::{info, error};

fn load_certs(path: &str) -> Vec<Certificate> {
    let certfile = File::open(path).expect("cannot open certificate file");
    let mut reader = BufReader::new(certfile);
    rustls_pemfile::certs(&mut reader)
        .expect("failed to read certificates")
        .into_iter()
        .map(Certificate)
        .collect()
}

fn load_private_key(path: &str) -> PrivateKey {
    let keyfile = File::open(path).expect("cannot open private key");
    let mut reader = BufReader::new(keyfile);
    let keys = rustls_pemfile::pkcs8_private_keys(&mut reader)
        .expect("failed to read private keys");
    assert!(!keys.is_empty(), "no private keys found");
    PrivateKey(keys[0].clone())
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();
    let addr = "127.0.0.1:8443";
    let listener = TcpListener::bind(addr).await.expect("failed to bind");

    info!("Echo server listening on {}", addr);

    // Load certs
    let certs = load_certs("certs/server.crt");
    let key = load_private_key("certs/server.key");
    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .expect("invalid certificate or key");

    let acceptor = TlsAcceptor::from(Arc::new(config));

    loop {
        let (stream, peer) = listener.accept().await.expect("failed to accept");
        let acceptor = acceptor.clone();

        tokio::spawn(async move {
            match acceptor.accept(stream).await {
                Ok(mut tls_stream) => {
                    info!("TLS connection established from {}", peer);
                    let mut buf = vec![0u8; 4096];
                    loop {
                        match tls_stream.read(&mut buf).await {
                            Ok(0) => break, // connection closed
                            Ok(n) => {
                                if let Err(e) = tls_stream.write_all(&buf[..n]).await {
                                    error!("Write error to {}: {:?}", peer, e);
                                    break;
                                }
                            }
                            Err(e) => {
                                error!("Read error from {}: {:?}", peer, e);
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("TLS handshake failed from {}: {:?}", peer, e);
                }
            }
        });
    }
}
