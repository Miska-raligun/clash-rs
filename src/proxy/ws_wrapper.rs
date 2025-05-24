
use tokio_tungstenite::{WebSocketStream, MaybeTlsStream};
use tokio::net::TcpStream;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use std::pin::Pin;
use std::task::{Context, Poll};
use futures_util::{StreamExt, Sink, SinkExt};
use tungstenite::Message;
use std::io;
use futures_util::Stream;

pub struct WsStreamWrapper {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    read_buf: Vec<u8>,
}

impl WsStreamWrapper {
    pub fn new(ws: WebSocketStream<MaybeTlsStream<TcpStream>>) -> Self {
        Self { ws, read_buf: Vec::new() }
    }
}

impl AsyncRead for WsStreamWrapper {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if !self.read_buf.is_empty() {
            let to_read = std::cmp::min(buf.remaining(), self.read_buf.len());
            buf.put_slice(&self.read_buf[..to_read]);
            self.read_buf.drain(..to_read);
            println!("[WsWrapper] Returning {} buffered bytes to client", to_read);
            return Poll::Ready(Ok(()));
        }

        match Pin::new(&mut self.ws).poll_next(cx) {
            Poll::Ready(Some(Ok(Message::Binary(data)))) => {
                let to_read = std::cmp::min(buf.remaining(), data.len());
                buf.put_slice(&data[..to_read]);
                self.read_buf = data[to_read..].to_vec();
                println!("[WsWrapper] Received {} bytes from remote", data.len());
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Some(Ok(other))) => {
                println!("[WsWrapper] Ignored non-binary WebSocket message: {:?}", other);
                self.poll_read(cx, buf)
            }
            Poll::Ready(Some(Err(e))) => {
                println!("[WsWrapper] Error reading from WebSocket: {}", e);
                Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, e)))
            }
            Poll::Ready(None) => {
                println!("[WsWrapper] Remote closed connection");
                Poll::Ready(Ok(()))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl AsyncWrite for WsStreamWrapper {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        data: &[u8],
    ) -> Poll<io::Result<usize>> {
        let len = data.len();
        println!("[WsWrapper] Sending {} bytes to remote", len);
        let result = futures::executor::block_on(self.ws.send(Message::Binary(data.to_vec())));
        match result {
            Ok(_) => {
                println!("[WsWrapper] Successfully sent {} bytes", len);
                Poll::Ready(Ok(len))
            }
            Err(e) => {
                println!("[WsWrapper] Error sending to remote: {}", e);
                Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, e)))
            }
        }
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.ws)
            .poll_flush(cx)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.ws)
            .poll_close(cx)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }
}
