use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_rustls::rustls::{ClientConfig, Certificate, RootCertStore, ServerName};
use std::fs::File;
use std::io::BufReader;
use anyhow::Result;
use tracing::info;

fn load_ca(path: &str) -> RootCertStore {
    let mut root_store = RootCertStore::empty();
    let ca_cert = File::open(path).expect("cannot open CA file");
    let mut reader = BufReader::new(ca_cert);
    let certs = rustls_pemfile::certs(&mut reader).expect("failed to read CA");
    for cert in certs {
        root_store.add(&Certificate(cert)).expect("failed to add CA");
    }
    root_store
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().init();

    let addr = "127.0.0.1:8443";
    let server_name = "localhost";
    let msg = b"ping";

    let root_store = load_ca("certs/ca.crt");
    let config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    let connector = TlsConnector::from(Arc::new(config));

    let server_name = ServerName::try_from(server_name)?;
    let mut stream = connector.connect(server_name, TcpStream::connect(addr).await?).await?;

    stream.write_all(msg).await?;
    stream.flush().await?;

    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).await?;
    buf.truncate(n);

    info!("echoed: {}", String::from_utf8_lossy(&buf));
    Ok(())
}
