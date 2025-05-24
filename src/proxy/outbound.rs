
use std::any::Any;
use std::io;
use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};

pub trait AsyncStream: AsyncRead + AsyncWrite + Unpin + Send + Sync {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send + Sync> AsyncStream for T {}
pub type AnyStream = Box<dyn AsyncStream>;

#[async_trait]
pub trait OutboundHandler: Send + Sync {
    async fn connect(&self, address: &str, port: u16) -> io::Result<AnyStream>;
    fn as_any(&self) -> &dyn Any;
}
