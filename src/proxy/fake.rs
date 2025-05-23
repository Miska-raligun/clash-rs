use super::outbound::{OutboundHandler, AnyStream};
use async_trait::async_trait;
use tokio::net::TcpStream;

pub struct FakeProxy;

#[async_trait]
impl OutboundHandler for FakeProxy {
    async fn connect(&self, address: &str, port: u16) -> std::io::Result<AnyStream> {
        println!("[FakeProxy] Connecting to {}:{}", address, port);
        let stream = TcpStream::connect(format!("{}:{}", address, port)).await?;
        Ok(Box::new(stream))
    }
}
