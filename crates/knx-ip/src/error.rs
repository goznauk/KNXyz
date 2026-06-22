#[derive(Debug, thiserror::Error)]
pub enum KnxIpError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Core(#[from] knx_core::KnxError),
    #[error(transparent)]
    Dpt(#[from] knx_dpt::DptError),
    #[error("operation timed out")]
    Timeout,
    #[error("tunnel ACK timed out")]
    AckTimeout,
    #[error("tunnel heartbeat timed out")]
    HeartbeatTimeout,
    #[error("tunnel disconnect timed out")]
    DisconnectTimeout,
    #[error("gateway returned status 0x{status:02x}")]
    GatewayStatus { status: u8 },
    #[error("tunnel receive loop stopped")]
    ReceiveLoopStopped,
    #[error("IPv6 is not supported for KNXnet/IP tunneling")]
    UnsupportedIpv6,
    #[error("tunnel reconnect attempts exhausted after {attempts} attempts")]
    ReconnectAttemptsExhausted { attempts: usize },
    #[error("invalid KNXnet/IP response: {0}")]
    InvalidResponse(&'static str),
    #[error("monitor event stream lagged")]
    MonitorLagged,
}

pub type Result<T> = std::result::Result<T, KnxIpError>;
