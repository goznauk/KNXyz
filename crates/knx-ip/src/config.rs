use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::Duration;

use crate::{HeartbeatOptions, ReconnectPolicy};

const DEFAULT_ACK_TIMEOUT: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TunnelOptions {
    pub target: SocketAddr,
    pub bind: SocketAddr,
    pub control_endpoint: Option<SocketAddr>,
    pub data_endpoint: Option<SocketAddr>,
    pub ack_timeout: Duration,
    pub reconnect_policy: Option<ReconnectPolicy>,
    pub heartbeat: Option<HeartbeatOptions>,
}

impl TunnelOptions {
    pub fn new(target: SocketAddr) -> Self {
        Self {
            target,
            bind: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)),
            control_endpoint: None,
            data_endpoint: None,
            ack_timeout: DEFAULT_ACK_TIMEOUT,
            reconnect_policy: None,
            heartbeat: None,
        }
    }
}
