use std::sync::Arc;
use std::fs;
use std::time::SystemTime;

use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::{
    rustls::{self, ClientConfig, RootCertStore, ServerName},
    TlsConnector,
};
use tracing::info;
//use webpki_roots::TLS_SERVER_ROOTS;

pub mod frame; // for Frame struct + serialization
pub mod mux;   // for multiplexed stream handling (next step)


/// HTX TLS client wrapper
pub struct HtxClient {
    connector: TlsConnector,
}

impl HtxClient {
    /// Create a client from a given TLS configuration
    pub fn with_config(config: Arc<ClientConfig>) -> Result<Self> {
        Ok(Self {
            connector: TlsConnector::from(config),
        })
    }

    /// Perform an echo roundtrip: connect, send message, read response
    pub async fn echo_roundtrip(
        &self,
        addr: &str,
        server_name: &str,
        msg: &[u8],
    ) -> Result<Vec<u8>> {
        info!("connecting to {}", addr);
        let tcp = TcpStream::connect(addr).await?;

        // Correct TryFrom usage
        let server_name = ServerName::try_from(server_name.as_ref())?;
        let mut tls = self.connector.connect(server_name, tcp).await?;

        tls.write_all(msg).await?;
        tls.flush().await?;

        let mut buf = vec![0u8; 4096];
        let n = tls.read(&mut buf).await?;
        buf.truncate(n);

        Ok(buf)
    }
}

/// Create a TLS client configuration
pub fn make_tls_config(insecure: bool) -> Result<Arc<ClientConfig>> {
    if insecure {
        let cfg = ClientConfig::builder()
            .with_safe_defaults()
            .with_custom_certificate_verifier(Arc::new(NoVerifier))
            .with_no_client_auth();
        Ok(Arc::new(cfg))
    } else {
        let mut root_store = RootCertStore::empty();
        // Load your CA certificate
        let ca_cert = fs::read("certs/ca.crt")?;
        root_store.add(&rustls::Certificate(ca_cert))?;

        let mut cfg = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_store)
            .with_no_client_auth();
        cfg.resumption = rustls::client::Resumption::default();
        Ok(Arc::new(cfg))
    }
}

/// Custom certificate verifier that disables verification
struct NoVerifier;

impl rustls::client::ServerCertVerifier for NoVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::Certificate,
        _intermediates: &[rustls::Certificate],
        _server_name: &ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: SystemTime,
    ) -> std::result::Result<rustls::client::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::ServerCertVerified::assertion())
    }
}
