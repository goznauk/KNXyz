use std::net::SocketAddr;

use tokio::net::UdpSocket;

use crate::Result;

#[derive(Debug)]
pub struct UdpTransport {
    socket: UdpSocket,
}

impl UdpTransport {
    pub async fn bind(bind: SocketAddr) -> Result<Self> {
        Ok(Self {
            socket: UdpSocket::bind(bind).await?,
        })
    }

    pub fn set_broadcast(&self, broadcast: bool) -> Result<()> {
        Ok(self.socket.set_broadcast(broadcast)?)
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.socket.local_addr()?)
    }

    pub async fn send_to(&self, bytes: &[u8], target: SocketAddr) -> Result<usize> {
        Ok(self.socket.send_to(bytes, target).await?)
    }

    pub async fn recv_from(&self, buffer: &mut [u8]) -> Result<(usize, SocketAddr)> {
        Ok(self.socket.recv_from(buffer).await?)
    }
}
