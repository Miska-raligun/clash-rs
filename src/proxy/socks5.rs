
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt, copy_bidirectional};
use crate::proxy::proxy_manager::ProxyManager;
use crate::proxy::runtime::ProxyRuntime;
use crate::proxy::outbound::OutboundHandler;

pub async fn start_socks5_server(
    addr: &str,
    manager: Arc<ProxyManager>,
    runtime: Arc<ProxyRuntime>,
) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    println!("[SOCKS5] Listening on {}", addr);

    loop {
        let (stream, peer_addr) = listener.accept().await?;
        let manager = manager.clone();
        let runtime = runtime.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_client(stream, manager, runtime).await {
                eprintln!("[SOCKS5] Error from {}: {:?}", peer_addr, e);
            }
        });
    }
}

async fn handle_client(
    mut client: TcpStream,
    manager: Arc<ProxyManager>,
    runtime: Arc<ProxyRuntime>,
) -> std::io::Result<()> {
    let mut buf = [0u8; 262];
    client.read_exact(&mut buf[..2]).await?;
    let nmethods = buf[1] as usize;
    client.read_exact(&mut buf[..nmethods]).await?;
    client.write_all(&[0x05, 0x00]).await?;

    client.read_exact(&mut buf[..4]).await?;
    if buf[1] != 0x01 {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "Only CONNECT supported"));
    }

    let addr = match buf[3] {
        0x01 => {
            client.read_exact(&mut buf[..4]).await?;
            std::net::Ipv4Addr::from(<[u8; 4]>::try_from(&buf[..4]).unwrap()).to_string()
        }
        0x03 => {
            client.read_exact(&mut buf[..1]).await?;
            let len = buf[0] as usize;
            client.read_exact(&mut buf[..len]).await?;
            String::from_utf8_lossy(&buf[..len]).to_string()
        }
        _ => return Err(std::io::Error::new(std::io::ErrorKind::Other, "Address type not supported")),
    };

    client.read_exact(&mut buf[..2]).await?;
    let port = u16::from_be_bytes(buf[..2].try_into().unwrap());

    let current_group = runtime
        .get_group("🔰国外流量")
        .expect("[socks5] runtime missing Proxy group");
    println!("[SOCKS5] Received request to connect to {}:{}", addr, port);
    let current_name = current_group.get();
    println!("[SOCKS5] 当前使用节点：{}", current_name);
    let handler = manager.get(&current_name)
        .unwrap_or_else(|| panic!("[SOCKS5] No handler found for proxy: {}", current_name));
    let mut remote = handler.connect(&addr, port).await?;

    client.write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).await?;

    let (n1, n2) = tokio::io::copy_bidirectional(&mut client, &mut remote).await?;
    println!("[SOCKS5] Relay complete: client → remote = {} bytes, remote → client = {} bytes", n1, n2);

    Ok(())
}
