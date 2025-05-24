use std::io;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::{
    rustls::{self, OwnedTrustAnchor, RootCertStore, ServerName},
    TlsConnector,
};
use webpki_roots::TLS_SERVER_ROOTS;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, AsyncReadExt};
use async_trait::async_trait;

use crate::proxy::outbound::{OutboundHandler, AnyStream};

pub struct TrojanProxy {
    pub name: String,
    pub server: String,
    pub port: u16,
    pub password: String,
    pub sni: Option<String>,
}

impl TrojanProxy {
    pub fn new(
        name: String,
        server: String,
        port: u16,
        password: String,
        sni: Option<String>,
    ) -> Self {
        Self {
            name,
            server,
            port,
            password,
            sni,
        }
    }
}

#[async_trait]
impl OutboundHandler for TrojanProxy {
    async fn connect(&self, address: &str, port: u16) -> io::Result<AnyStream> {
        println!("[Trojan] Connecting to {}:{} via Trojan", address, port);

        let stream = TcpStream::connect(format!("{}:{}", self.server, self.port)).await?;
        println!("[Trojan] Connected to Trojan server {}:{}", self.server, self.port);

        let mut root_cert_store = RootCertStore::empty();
        root_cert_store.add_trust_anchors(
            TLS_SERVER_ROOTS.iter().map(|ta| {
                OwnedTrustAnchor::from_subject_spki_name_constraints(
                    ta.subject,
                    ta.spki,
                    ta.name_constraints,
                )
            }),
        );

        let config = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_cert_store)
            .with_no_client_auth();

        let connector = TlsConnector::from(Arc::new(config));
        let server_name = ServerName::try_from(self.sni.as_deref().unwrap_or(&self.server))
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid SNI"))?;

        let mut tls_stream = connector.connect(server_name, stream).await?;
        println!("[Trojan] TLS handshake successful");

        let connect_header = format!(
            "{password}\r\n\
            CONNECT {host}:{port} HTTP/1.1\r\n\
            Host: {host}:{port}\r\n\r\n",
            password = self.password,
            host = address,
            port = port
        );
        println!("[Trojan] Sending handshake:\n{}", connect_header);

        tls_stream.write_all(connect_header.as_bytes()).await?;
        tls_stream.flush().await?;

        let mut response_buf = [0u8; 1024];
        let n = tls_stream.read(&mut response_buf).await?;
        let response = String::from_utf8_lossy(&response_buf[..n]);
        println!("[Trojan] Received server response:\n{}", response);

        /*
        if !response.contains("200") {
            return Err(io::Error::new(io::ErrorKind::Other, "Trojan handshake failed"));
        }
        */

        Ok(Box::new(tls_stream))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
