use std::sync::Arc;
use async_trait::async_trait;
use tokio::net::TcpStream;
use uuid::Uuid;

use crate::proxy::outbound::{OutboundHandler, AnyStream};

pub struct VmessProxy {
    pub name: String,
    pub server: String,
    pub port: u16,
    pub uuid: Uuid,
}

impl VmessProxy {
    pub fn new(name: String, server: String, port: u16, uuid: Uuid) -> Self {
        Self { name, server, port, uuid }
    }
}

#[async_trait]
impl OutboundHandler for VmessProxy {
    async fn connect(&self, address: &str, port: u16) -> std::io::Result<AnyStream> {
        let target = format!("{}:{}", self.server, self.port);
        println!("[VmessProxy:{}] Connecting to upstream: {}", self.name, target);

        let mut stream = TcpStream::connect(target).await?;

        // Step 1: 构造并发送握手请求帧（简化版本）
        send_vmess_handshake(&mut stream, &self.uuid, address, port).await?;

        // Step 2: 返回加密的连接流（现在直接返回 TCP 流）
        Ok(Box::new(stream))
    }
}

use tokio::io::{AsyncWriteExt, AsyncReadExt};

async fn send_vmess_handshake(
    stream: &mut TcpStream,
    uuid: &Uuid,
    address: &str,
    port: u16,
) -> std::io::Result<()> {
    // 构造 UUID+时间戳+目标地址等 header（此处使用最简格式）
    let mut buf = Vec::new();

    let uuid_bytes = uuid.as_bytes();
    buf.extend_from_slice(uuid_bytes); // 简化处理，仅为演示

    // 添加目标地址信息（这里我们假设是域名）
    buf.push(0x03); // address type: domain
    buf.push(address.len() as u8);
    buf.extend_from_slice(address.as_bytes());

    buf.extend_from_slice(&port.to_be_bytes());

    // 模拟 VMess Request Frame
    stream.write_all(&buf).await?;
    stream.flush().await?;

    Ok(())
}

