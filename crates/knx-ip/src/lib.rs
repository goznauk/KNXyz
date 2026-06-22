#![forbid(unsafe_code)]

mod config;
mod discovery;
mod error;
mod heartbeat;
mod reconnect;
mod routing;
mod telegram;
mod transport;
mod tunnel;

pub use config::TunnelOptions;
pub use discovery::{discover_gateways, DiscoveryOptions, Gateway, ServiceFamily, KNXNET_IP_PORT};
pub use error::{KnxIpError, Result};
pub use heartbeat::HeartbeatOptions;
pub use reconnect::{ConnectionEvent, ReconnectPolicy};
pub use routing::{
    bind_routing_socket, decode_routing_packet, encode_routing_packet, RouteEvent, RouteMonitor,
    RouteSender, RoutingOptions, RoutingSendOptions, DEFAULT_ROUTING_MULTICAST,
    DEFAULT_ROUTING_PORT,
};
pub use telegram::GroupEvent;
pub use transport::UdpTransport;
pub use tunnel::TunnelClient;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
