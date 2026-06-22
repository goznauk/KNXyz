use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use knx_core::{
    Apci, CemiFrame, ConnectionHeader, GroupAddress, HostProtocol, Hpai, KnxError, KnxNetIpHeader,
    ServiceType,
};
use thiserror::Error;
use tokio::net::UdpSocket;
use tokio::time::{self, Duration};

use crate::frames;
use crate::script::{ReceivedFrame, SimEvent};

#[derive(Debug, Error)]
pub enum SimError {
    #[error("simulated gateway socket error: {0}")]
    Io(#[from] std::io::Error),
    #[error("simulated gateway frame error: {0}")]
    Frame(#[from] KnxError),
    #[error("expected {expected:?}, received {actual:?}")]
    UnexpectedServiceType {
        expected: ServiceType,
        actual: ServiceType,
    },
    #[error("expected sequence {expected}, received {actual}")]
    UnexpectedSequence { expected: u8, actual: u8 },
    #[error("simulated gateway has not received a client packet yet")]
    MissingPeer,
    #[error("invalid simulator script: {0}")]
    InvalidScript(&'static str),
    #[error("received an unexpected packet while waiting for timeout")]
    UnexpectedPacket,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimGatewayHandle {
    local_addr: SocketAddr,
}

impl SimGatewayHandle {
    pub const fn local_addr(self) -> SocketAddr {
        self.local_addr
    }
}

#[derive(Debug)]
pub struct SimGateway {
    socket: UdpSocket,
    local_addr: SocketAddr,
    last_peer: Option<SocketAddr>,
}

impl SimGateway {
    pub async fn bind_localhost() -> Result<Self, SimError> {
        let socket =
            UdpSocket::bind(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))).await?;
        let local_addr = socket.local_addr()?;

        Ok(Self {
            socket,
            local_addr,
            last_peer: None,
        })
    }

    pub const fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub const fn handle(&self) -> SimGatewayHandle {
        SimGatewayHandle {
            local_addr: self.local_addr,
        }
    }

    pub async fn shutdown(self) -> Result<(), SimError> {
        Ok(())
    }

    pub async fn expect_search_request(&mut self) -> Result<SimEvent, SimError> {
        let received = self.recv_knxnetip().await?;
        expect_service_type(&received, ServiceType::SearchRequest)?;

        let (client_endpoint, remaining) = Hpai::decode(&received.payload)?;
        if !remaining.is_empty() {
            return Err(SimError::InvalidScript("search request has trailing bytes"));
        }

        Ok(SimEvent::SearchRequest {
            peer: received.peer,
            client_endpoint,
        })
    }

    pub async fn reply_search_response(
        &self,
        control_endpoint: SocketAddr,
        service_families: &[(u8, u8)],
    ) -> Result<(), SimError> {
        let packet = frames::search_response(control_endpoint, service_families)?;
        self.send_to_last_peer(&packet).await
    }

    pub async fn expect_connect_request(&mut self) -> Result<SimEvent, SimError> {
        let received = self.recv_knxnetip().await?;
        expect_service_type(&received, ServiceType::ConnectRequest)?;

        let (control_endpoint, payload) = Hpai::decode(&received.payload)?;
        let (data_endpoint, payload) = Hpai::decode(payload)?;
        if payload != [0x04, 0x04, 0x02, 0x00] {
            return Err(SimError::InvalidScript("connect request CRI did not match"));
        }

        Ok(SimEvent::ConnectRequest {
            peer: received.peer,
            control_endpoint,
            data_endpoint,
        })
    }

    pub async fn reply_connect_response(&self, channel_id: u8) -> Result<(), SimError> {
        let packet = frames::connect_response(channel_id, self.local_addr)?;
        self.send_to_last_peer(&packet).await
    }

    pub async fn expect_connection_state_request(
        &mut self,
        expected_channel_id: u8,
    ) -> Result<SimEvent, SimError> {
        let received = self.recv_knxnetip().await?;
        expect_service_type(&received, ServiceType::ConnectionStateRequest)?;

        if received.payload.len() < 2 {
            return Err(SimError::InvalidScript(
                "connection state request missing channel header",
            ));
        }
        let channel_id = received.payload[0];
        if channel_id != expected_channel_id {
            return Err(SimError::UnexpectedSequence {
                expected: expected_channel_id,
                actual: channel_id,
            });
        }
        let (control_endpoint, remaining) = Hpai::decode(&received.payload[2..])?;
        if !remaining.is_empty() {
            return Err(SimError::InvalidScript(
                "connection state request has trailing bytes",
            ));
        }

        Ok(SimEvent::ConnectionStateRequest {
            peer: received.peer,
            channel_id,
            control_endpoint,
        })
    }

    pub async fn reply_connection_state_response(
        &self,
        channel_id: u8,
        status: u8,
    ) -> Result<(), SimError> {
        let packet = frames::connection_state_response(channel_id, status)?;
        self.send_to_last_peer(&packet).await
    }

    pub async fn expect_disconnect_request(
        &mut self,
        expected_channel_id: u8,
    ) -> Result<SimEvent, SimError> {
        let received = self.recv_knxnetip().await?;
        expect_service_type(&received, ServiceType::DisconnectRequest)?;

        let channel_id = disconnect_channel_id(&received.payload)?;
        if channel_id != expected_channel_id {
            return Err(SimError::UnexpectedSequence {
                expected: expected_channel_id,
                actual: channel_id,
            });
        }
        let (control_endpoint, remaining) = Hpai::decode(&received.payload[2..])?;
        if !remaining.is_empty() {
            return Err(SimError::InvalidScript(
                "disconnect request has trailing bytes",
            ));
        }

        Ok(SimEvent::DisconnectRequest {
            peer: received.peer,
            channel_id,
            control_endpoint,
        })
    }

    pub async fn reply_disconnect_response(&self, channel_id: u8) -> Result<(), SimError> {
        let packet = frames::disconnect_response(channel_id, 0x00)?;
        self.send_to_last_peer(&packet).await
    }

    /// Receive the next tunnel frame, which may be a TUNNELLING_REQUEST
    /// (the steady-state path) or a DISCONNECT_REQUEST (orderly teardown).
    /// Lets a serving loop branch on whichever the client sends next.
    pub async fn expect_tunnelling_or_disconnect(
        &mut self,
        expected_sequence: u8,
    ) -> Result<TunnelInbound, SimError> {
        let received = self.recv_knxnetip().await?;
        match received.header.service_type() {
            ServiceType::TunnellingRequest => {
                let (connection, payload) = ConnectionHeader::decode(&received.payload)?;
                if connection.sequence_counter() != expected_sequence {
                    return Err(SimError::UnexpectedSequence {
                        expected: expected_sequence,
                        actual: connection.sequence_counter(),
                    });
                }
                let (frame, remaining) = CemiFrame::decode(payload)?;
                if !remaining.is_empty() {
                    return Err(SimError::InvalidScript(
                        "tunnelling request cEMI has trailing bytes",
                    ));
                }
                Ok(TunnelInbound::Tunnelling(frame))
            }
            ServiceType::DisconnectRequest => {
                let channel_id = disconnect_channel_id(&received.payload)?;
                Ok(TunnelInbound::Disconnect { channel_id })
            }
            actual => Err(SimError::UnexpectedServiceType {
                expected: ServiceType::TunnellingRequest,
                actual,
            }),
        }
    }

    pub async fn expect_tunnelling_request(
        &mut self,
        expected_sequence: u8,
    ) -> Result<CemiFrame, SimError> {
        let received = self.recv_knxnetip().await?;
        expect_service_type(&received, ServiceType::TunnellingRequest)?;

        let (connection, payload) = ConnectionHeader::decode(&received.payload)?;
        if connection.sequence_counter() != expected_sequence {
            return Err(SimError::UnexpectedSequence {
                expected: expected_sequence,
                actual: connection.sequence_counter(),
            });
        }

        let (frame, remaining) = CemiFrame::decode(payload)?;
        if !remaining.is_empty() {
            return Err(SimError::InvalidScript(
                "tunnelling request cEMI has trailing bytes",
            ));
        }

        Ok(frame)
    }

    pub async fn expect_tunnelling_ack(&mut self, expected_sequence: u8) -> Result<(), SimError> {
        let received = self.recv_knxnetip().await?;
        expect_service_type(&received, ServiceType::TunnellingAck)?;

        let (connection, _) = ConnectionHeader::decode(&received.payload)?;
        if connection.sequence_counter() != expected_sequence {
            return Err(SimError::UnexpectedSequence {
                expected: expected_sequence,
                actual: connection.sequence_counter(),
            });
        }

        Ok(())
    }

    pub async fn reply_tunnelling_ack(&self, channel_id: u8, sequence: u8) -> Result<(), SimError> {
        let packet = frames::tunnelling_ack(channel_id, sequence)?;
        self.send_to_last_peer(&packet).await
    }

    pub async fn reply_wrong_channel_ack(
        &self,
        channel_id: u8,
        sequence: u8,
    ) -> Result<(), SimError> {
        self.reply_tunnelling_ack(channel_id.wrapping_add(1), sequence)
            .await
    }

    pub async fn reply_wrong_sequence_ack(
        &self,
        channel_id: u8,
        sequence: u8,
    ) -> Result<(), SimError> {
        self.reply_tunnelling_ack(channel_id, sequence.wrapping_add(1))
            .await
    }

    pub async fn send_tunnelling_indication(
        &self,
        channel_id: u8,
        sequence: u8,
        group: GroupAddress,
        apci: Apci,
        payload: &[u8],
    ) -> Result<(), SimError> {
        let packet = frames::tunnelling_indication(channel_id, sequence, group, apci, payload)?;
        self.send_to_last_peer(&packet).await
    }

    pub async fn send_malformed_tunnelling_indication(
        &self,
        channel_id: u8,
        sequence: u8,
        cemi: &[u8],
    ) -> Result<(), SimError> {
        let packet = frames::tunnelling_request_with_cemi(channel_id, sequence, cemi)?;
        self.send_to_last_peer(&packet).await
    }

    pub async fn send_raw(&self, bytes: &[u8]) -> Result<(), SimError> {
        self.send_to_last_peer(bytes).await
    }

    pub async fn delay(&self, duration: Duration) {
        time::sleep(duration).await;
    }

    pub async fn expect_timeout(&mut self) -> Result<(), SimError> {
        let mut buffer = [0_u8; 1500];
        match time::timeout(
            Duration::from_millis(25),
            self.socket.recv_from(&mut buffer),
        )
        .await
        {
            Err(_) => Ok(()),
            Ok(Ok(_)) => Err(SimError::UnexpectedPacket),
            Ok(Err(error)) => Err(SimError::Io(error)),
        }
    }

    async fn recv_knxnetip(&mut self) -> Result<ReceivedFrame, SimError> {
        let mut buffer = [0_u8; 1500];
        let (len, peer) = self.socket.recv_from(&mut buffer).await?;
        let (header, payload) = KnxNetIpHeader::decode(&buffer[..len])?;
        self.last_peer = Some(peer);

        Ok(ReceivedFrame {
            header,
            payload: payload.to_vec(),
            peer,
        })
    }

    async fn send_to_last_peer(&self, packet: &[u8]) -> Result<(), SimError> {
        let peer = self.last_peer.ok_or(SimError::MissingPeer)?;
        self.socket.send_to(packet, peer).await?;
        Ok(())
    }
}

/// The next tunnel frame a serving loop received: either steady-state
/// tunnelling traffic or an orderly DISCONNECT_REQUEST.
#[derive(Debug)]
pub enum TunnelInbound {
    Tunnelling(CemiFrame),
    Disconnect { channel_id: u8 },
}

fn expect_service_type(received: &ReceivedFrame, expected: ServiceType) -> Result<(), SimError> {
    let actual = received.header.service_type();
    if actual != expected {
        return Err(SimError::UnexpectedServiceType { expected, actual });
    }
    Ok(())
}

fn disconnect_channel_id(payload: &[u8]) -> Result<u8, SimError> {
    if payload.len() < 2 {
        return Err(SimError::InvalidScript(
            "disconnect request missing channel header",
        ));
    }
    Ok(payload[0])
}

pub(crate) fn ipv4_hpai_for(endpoint: SocketAddr) -> Hpai {
    match endpoint {
        SocketAddr::V4(endpoint) => Hpai::new(
            HostProtocol::Ipv4Udp,
            endpoint.ip().octets(),
            endpoint.port(),
        ),
        SocketAddr::V6(endpoint) => Hpai::new(
            HostProtocol::Ipv4Udp,
            Ipv4Addr::UNSPECIFIED.octets(),
            endpoint.port(),
        ),
    }
}
