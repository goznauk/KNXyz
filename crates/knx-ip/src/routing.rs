use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use futures_core::Stream;
use knx_core::{
    Apci, CemiFrame, CemiMessageCode, GroupAddress, GroupTelegram, IndividualAddress,
    KnxNetIpHeader, ServiceType,
};
use knx_dpt::DptValue;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::discovery::KNXNET_IP_PORT;
use crate::telegram::GroupEvent;
use crate::{KnxIpError, Result};

pub const DEFAULT_ROUTING_MULTICAST: Ipv4Addr = Ipv4Addr::new(224, 0, 23, 12);
pub const DEFAULT_ROUTING_PORT: u16 = KNXNET_IP_PORT;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoutingOptions {
    pub bind: SocketAddrV4,
    pub multicast: Ipv4Addr,
    pub port: u16,
    pub interface: Option<Ipv4Addr>,
    pub loopback: bool,
}

impl Default for RoutingOptions {
    fn default() -> Self {
        Self {
            bind: SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, DEFAULT_ROUTING_PORT),
            multicast: DEFAULT_ROUTING_MULTICAST,
            port: DEFAULT_ROUTING_PORT,
            interface: None,
            loopback: false,
        }
    }
}

impl RoutingOptions {
    pub fn from_target(
        bind: SocketAddr,
        target: SocketAddr,
        interface: Option<Ipv4Addr>,
        loopback: bool,
    ) -> Result<Self> {
        let SocketAddr::V4(bind) = bind else {
            return Err(KnxIpError::InvalidResponse(
                "routing requires IPv4 bind address",
            ));
        };
        let SocketAddr::V4(target) = target else {
            return Err(KnxIpError::InvalidResponse(
                "routing requires IPv4 multicast target",
            ));
        };
        if !target.ip().is_multicast() {
            return Err(KnxIpError::InvalidResponse(
                "routing requires IPv4 multicast target",
            ));
        }

        Ok(Self {
            bind,
            multicast: *target.ip(),
            port: target.port(),
            interface,
            loopback,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoutingSendOptions {
    pub bind: SocketAddrV4,
    pub target: SocketAddrV4,
    pub interface: Option<Ipv4Addr>,
    pub loopback: bool,
    pub ttl: u32,
}

impl Default for RoutingSendOptions {
    fn default() -> Self {
        Self {
            bind: SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0),
            target: SocketAddrV4::new(DEFAULT_ROUTING_MULTICAST, DEFAULT_ROUTING_PORT),
            interface: None,
            loopback: false,
            ttl: 1,
        }
    }
}

impl RoutingSendOptions {
    pub fn from_target(
        bind: SocketAddr,
        target: SocketAddr,
        interface: Option<Ipv4Addr>,
        loopback: bool,
        ttl: u32,
    ) -> Result<Self> {
        let SocketAddr::V4(bind) = bind else {
            return Err(KnxIpError::InvalidResponse(
                "routing requires IPv4 bind address",
            ));
        };
        let SocketAddr::V4(target) = target else {
            return Err(KnxIpError::InvalidResponse(
                "routing requires IPv4 multicast target",
            ));
        };
        if !target.ip().is_multicast() {
            return Err(KnxIpError::InvalidResponse(
                "routing requires IPv4 multicast target",
            ));
        }

        Ok(Self {
            bind,
            target,
            interface,
            loopback,
            ttl,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteEvent {
    pub source: IndividualAddress,
    pub destination: GroupAddress,
    pub apci: Apci,
    pub payload: Vec<u8>,
    pub peer: SocketAddr,
}

impl RouteEvent {
    fn from_cemi(frame: &CemiFrame, peer: SocketAddr) -> Self {
        let event = GroupEvent::from_cemi(frame);
        Self {
            source: event.source,
            destination: event.destination,
            apci: event.apci,
            payload: event.payload,
            peer,
        }
    }
}

pub fn decode_routing_packet(input: &[u8], peer: SocketAddr) -> Result<Option<RouteEvent>> {
    let (header, payload) = KnxNetIpHeader::decode(input)?;
    if header.service_type() != ServiceType::RoutingIndication {
        return Ok(None);
    }

    let (frame, remaining) = CemiFrame::decode(payload)?;
    if !remaining.is_empty() {
        return Err(knx_core::KnxError::InvalidFrame("routing cEMI has trailing bytes").into());
    }

    Ok(Some(RouteEvent::from_cemi(&frame, peer)))
}

pub fn encode_routing_packet(frame: &CemiFrame) -> Result<Vec<u8>> {
    let mut payload = Vec::new();
    frame.encode(&mut payload)?;

    let total_length = u16::try_from(6 + payload.len())
        .map_err(|_| KnxIpError::InvalidResponse("KNXnet/IP frame too long"))?;
    let mut packet = Vec::new();
    KnxNetIpHeader::new(ServiceType::RoutingIndication, total_length)?.encode(&mut packet)?;
    packet.extend_from_slice(&payload);

    Ok(packet)
}

pub async fn bind_routing_socket(options: RoutingOptions) -> Result<UdpSocket> {
    let socket = UdpSocket::bind(options.bind).await?;
    socket.set_multicast_loop_v4(options.loopback)?;
    socket.join_multicast_v4(
        options.multicast,
        options.interface.unwrap_or(Ipv4Addr::UNSPECIFIED),
    )?;

    Ok(socket)
}

#[derive(Debug)]
pub struct RouteSender {
    socket: UdpSocket,
    target: SocketAddrV4,
}

impl RouteSender {
    pub async fn bind(options: RoutingSendOptions) -> Result<Self> {
        let socket = bind_routing_send_socket(options)?;

        Ok(Self {
            socket,
            target: options.target,
        })
    }

    pub async fn send_frame(&self, frame: &CemiFrame) -> Result<usize> {
        let packet = encode_routing_packet(frame)?;

        Ok(self.socket.send_to(&packet, self.target).await?)
    }

    /// Multicast a GroupValueWrite RoutingIndication for `value`.
    ///
    /// The DPT id is inferred from the [`DptValue`] variant via the same
    /// encoder the tunnel write path uses (so encoding stays centralized
    /// in knx-ip; the binding does no DPT work). Routing is connectionless
    /// multicast: this is a single fire-and-forget send (no ACK), and the
    /// frame is an L_Data.indication (the routing wire form), NOT the
    /// L_Data.request that `CemiFrame::group_value_write` stamps.
    pub async fn send_group_write(
        &self,
        source: IndividualAddress,
        group: GroupAddress,
        value: DptValue,
    ) -> Result<usize> {
        let payload = crate::tunnel::encode_value(value)?;
        let telegram = GroupTelegram::new(source, group, Apci::GroupValueWrite, &payload)?;
        let frame = CemiFrame::new(CemiMessageCode::LDataIndication, telegram);
        self.send_frame(&frame).await
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.socket.local_addr()?)
    }

    pub const fn target(&self) -> SocketAddrV4 {
        self.target
    }
}

fn bind_routing_send_socket(options: RoutingSendOptions) -> Result<UdpSocket> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_multicast_loop_v4(options.loopback)?;
    socket.set_multicast_ttl_v4(options.ttl)?;
    if let Some(interface) = options.interface {
        socket.set_multicast_if_v4(&interface)?;
    }
    socket.bind(&SockAddr::from(SocketAddr::V4(options.bind)))?;
    socket.set_nonblocking(true)?;

    Ok(UdpSocket::from_std(socket.into())?)
}

#[derive(Debug)]
pub struct RouteMonitor {
    events_rx: mpsc::Receiver<Result<RouteEvent>>,
}

impl RouteMonitor {
    pub async fn bind(options: RoutingOptions) -> Result<Self> {
        let socket = bind_routing_socket(options).await?;
        let (events_tx, events_rx) = mpsc::channel(64);

        tokio::spawn(async move {
            let mut buffer = [0_u8; 1500];
            loop {
                let Ok((len, peer)) = socket.recv_from(&mut buffer).await else {
                    break;
                };

                match decode_routing_packet(&buffer[..len], peer) {
                    Ok(Some(event)) => {
                        if events_tx.send(Ok(event)).await.is_err() {
                            break;
                        }
                    }
                    Ok(None) => {}
                    Err(error) => {
                        if events_tx.send(Err(error)).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Ok(Self { events_rx })
    }

    pub fn events(self) -> impl Stream<Item = Result<RouteEvent>> + Send + 'static {
        ReceiverStream::new(self.events_rx)
    }
}
