use std::net::SocketAddr;

use knx_core::{CemiFrame, Hpai, KnxNetIpHeader};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimEvent {
    SearchRequest {
        peer: SocketAddr,
        client_endpoint: Hpai,
    },
    ConnectRequest {
        peer: SocketAddr,
        control_endpoint: Hpai,
        data_endpoint: Hpai,
    },
    ConnectionStateRequest {
        peer: SocketAddr,
        channel_id: u8,
        control_endpoint: Hpai,
    },
    TunnellingRequest {
        peer: SocketAddr,
        channel_id: u8,
        sequence: u8,
        frame: CemiFrame,
    },
    DisconnectRequest {
        peer: SocketAddr,
        channel_id: u8,
        control_endpoint: Hpai,
    },
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReceivedFrame {
    pub(crate) header: KnxNetIpHeader,
    pub(crate) payload: Vec<u8>,
    pub(crate) peer: SocketAddr,
}
