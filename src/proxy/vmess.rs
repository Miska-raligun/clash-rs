use std::collections::HashMap;
use std::io;
use std::net::Ipv4Addr;
use uuid::Uuid;
use async_trait::async_trait;
use rand::{RngCore, thread_rng};
use sha2::{Sha256, Digest};
use chrono::Utc;
use md5::{Md5, Digest as Md5Digest};
use futures_util::SinkExt;
use futures_util::StreamExt;
use bytes::{BufMut, BytesMut};

use aes_gcm::{Aes128Gcm, KeyInit, Nonce};
use aes_gcm::aead::{Aead, Payload};

use tokio::net::TcpStream;
use tokio::io::{AsyncWriteExt};
use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream, MaybeTlsStream};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

use crate::proxy::outbound::{OutboundHandler, AnyStream};
use crate::proxy::ws_wrapper::WsStreamWrapper;

pub struct VmessProxy {
    pub name: String,
    pub server: String,
    pub port: u16,
    pub uuid: Uuid,
    pub alter_id: u32,
    pub network: Option<String>,
    pub ws_path: Option<String>,
    pub ws_host: Option<String>,
}

impl VmessProxy {
    pub fn new(
        name: String,
        server: String,
        port: u16,
        uuid: Uuid,
        alter_id: u32,
        network: Option<String>,
        ws_path: Option<String>,
        ws_headers: Option<HashMap<String, String>>,
    ) -> Self {
        let ws_host = ws_headers.as_ref().and_then(|h| h.get("Host").cloned());
        Self {
            name,
            server,
            port,
            uuid,
            alter_id,
            network,
            ws_path,
            ws_host,
        }
    }

    async fn connect_ws(&self, address: &str, port: u16) -> io::Result<AnyStream> {
        println!("[VMess] connect_ws_stream called: {} -> {}:{}", self.name, address, port);

        let raw_url = format!(
            "ws://{}:{}{}",
            self.ws_host.as_deref().unwrap_or(&self.server),
            self.port,
            self.ws_path.as_deref().unwrap_or(&"/".to_string())
        );

        let req_url = url::Url::parse(&raw_url).unwrap();
        let mut request = req_url.into_client_request().unwrap();
        if let Some(host) = &self.ws_host {
            request.headers_mut().insert("Host", host.parse().unwrap());
        }

        let (mut ws_stream, _) = connect_async(request).await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("WebSocket connect failed: {}", e)))?;

        if self.alter_id == 0 {
            send_vmess_aead_handshake(&mut ws_stream, &self.uuid, address, port).await?;
        } else {
            let mut raw_stream = WsStreamWrapper::new(ws_stream);
            send_vmess_legacy_handshake(&mut raw_stream, &self.uuid, address, port).await?;
            return Ok(Box::new(raw_stream));
        }

        // ✅ 读取服务器回应调试
        if let Some(msg) = ws_stream.next().await {
            println!("[VMess WS] response = {:?}", msg);
        }

        Ok(Box::new(WsStreamWrapper::new(ws_stream)))
    }
}

#[async_trait]
impl OutboundHandler for VmessProxy {
    async fn connect(&self, address: &str, port: u16) -> io::Result<AnyStream> {
        match self.network.as_deref() {
            Some("ws") => self.connect_ws(address, port).await,
            _ => Err(io::Error::new(io::ErrorKind::Other, "Only ws supported in this impl")),
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

async fn send_vmess_aead_handshake(
    stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    uuid: &Uuid,
    target_host: &str,
    target_port: u16,
) -> io::Result<()> {
    let payload = build_vmess_aead_request(uuid, target_host, target_port)?;
    println!("[VMess AEAD] payload len = {}", payload.len());
    stream.send(Message::Binary(payload)).await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("WS send failed: {}", e)))
}

async fn send_vmess_legacy_handshake<S: tokio::io::AsyncWrite + Unpin + Send>(
    stream: &mut S,
    uuid: &Uuid,
    target_host: &str,
    target_port: u16,
) -> io::Result<()> {
    let payload = build_vmess_legacy_request(uuid, target_host, target_port)?;
    println!("[VMess Legacy] payload len = {}", payload.len());
    stream.write_all(&payload).await?;
    Ok(())
}

fn build_vmess_aead_request(uuid: &Uuid, target_host: &str, target_port: u16) -> io::Result<Vec<u8>> {
    let timestamp = Utc::now().timestamp() as u32;
    let mut hash_input = uuid.as_bytes().to_vec();
    hash_input.extend_from_slice(&timestamp.to_be_bytes());
    let id = &Sha256::digest(&hash_input)[..16];

    let mut body = Vec::new();
    body.push(0x01);
    body.push(0x01);
    body.push(0x00);
    body.push(0x03);
    body.push(target_host.len() as u8);
    body.extend_from_slice(target_host.as_bytes());
    body.extend_from_slice(&target_port.to_be_bytes());

    let padding_len = 16;
    body.push(padding_len);
    let mut padding = vec![0u8; padding_len as usize];
    thread_rng().fill_bytes(&mut padding);
    body.extend_from_slice(&padding);

    let mut key = [0u8; 16];
    let mut iv = [0u8; 12];
    thread_rng().fill_bytes(&mut key);
    thread_rng().fill_bytes(&mut iv);

    println!("[VMess AEAD] id: {:x?}", id);
    println!("[VMess AEAD] key: {:x?}", key);
    println!("[VMess AEAD] iv: {:x?}", iv);

    let cipher = Aes128Gcm::new_from_slice(&key).unwrap();
    let nonce = Nonce::from_slice(&iv);
    let encrypted = cipher.encrypt(nonce, Payload { msg: &body, aad: &[] })
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "encrypt failed"))?;

    println!("[VMess AEAD] encrypted len: {}", encrypted.len());

    let mut out = Vec::new();
    out.extend_from_slice(id);
    out.extend_from_slice(&key);
    out.extend_from_slice(&iv);
    out.extend_from_slice(&(encrypted.len() as u16).to_be_bytes());
    out.extend_from_slice(&encrypted);
    Ok(out)
}

fn build_vmess_legacy_request(uuid: &Uuid, target_host: &str, target_port: u16) -> io::Result<Vec<u8>> {
    let id = generate_legacy_id(uuid);

    println!("[VMess Legacy] id: {:x?}", id);

    let mut buf = BytesMut::with_capacity(512);
    buf.put_slice(&id);
    buf.put_u8(0x01);
    buf.put_u8(0x01);
    buf.put_u8(0x00);
    buf.put_u8(0x00);

    let padding_len = 16;
    buf.put_u8(padding_len);
    let mut padding = [0u8; 16];
    thread_rng().fill_bytes(&mut padding);
    buf.put_slice(&padding);

    buf.put_u8(0x03); // domain type
    buf.put_u8(target_host.len() as u8);
    buf.put_slice(target_host.as_bytes());
    buf.put_u16(target_port);

    buf.put_u8(0x00); // extra_id_len
    buf.put_u8(0x01); // chunk_stream_mask

    println!("[VMess Legacy] full payload = {:?}", buf.to_vec());
    println!("[VMess Legacy] hex payload = {}", hex::encode(buf.to_vec()));

    Ok(buf.to_vec())
}

fn generate_legacy_id(uuid: &Uuid) -> [u8; 16] {
    let timestamp = (Utc::now().timestamp() / 60) as u64;
    let mut hasher = Md5::new();
    hasher.update(uuid.as_bytes());
    hasher.update(&timestamp.to_be_bytes());
    let result = hasher.finalize();
    let mut id = [0u8; 16];
    id.copy_from_slice(&result[..16]);
    id
}
