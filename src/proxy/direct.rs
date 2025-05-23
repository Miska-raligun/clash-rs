use async_trait::async_trait;
use tokio::net::TcpStream;
use crate::proxy::outbound::{OutboundHandler, AnyStream};

pub struct DirectProxy;

#[async_trait]
impl OutboundHandler for DirectProxy {
    async fn connect(&self, address: &str, port: u16) -> std::io::Result<AnyStream> {
        let target = format!("{}:{}", address, port);
        println!("[DirectProxy] Connecting to {}", target);
        let stream = TcpStream::connect(target).await?;
        Ok(Box::new(stream))
    }
}
