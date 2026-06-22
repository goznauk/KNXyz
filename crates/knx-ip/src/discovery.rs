use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use knx_core::{HostProtocol, Hpai, KnxNetIpHeader, ServiceType};
use tokio::time;

use crate::transport::UdpTransport;
use crate::{KnxIpError, Result};

pub const KNXNET_IP_PORT: u16 = 3671;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiscoveryOptions {
    pub bind: SocketAddr,
    pub target: SocketAddr,
    pub timeout: Duration,
}

impl Default for DiscoveryOptions {
    fn default() -> Self {
        Self {
            bind: SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)),
            target: SocketAddr::from((Ipv4Addr::BROADCAST, KNXNET_IP_PORT)),
            timeout: Duration::from_secs(3),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gateway {
    pub control_endpoint: SocketAddr,
    pub service_families: Vec<ServiceFamily>,
    pub received_from: SocketAddr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServiceFamily {
    pub id: u8,
    pub version: u8,
}

pub async fn discover_gateways(options: DiscoveryOptions) -> Result<Vec<Gateway>> {
    let transport = UdpTransport::bind(options.bind).await?;
    if options.target.ip().is_broadcast() {
        transport.set_broadcast(true)?;
    }

    let request = encode_search_request(transport.local_addr()?)?;
    transport.send_to(&request, options.target).await?;

    // Accumulate every SEARCH_RESPONSE that arrives until the timeout
    // elapses (multi-gateway discovery semantics): results are in
    // receive order and duplicates are preserved as received. An empty
    // result means nothing answered. A malformed response still aborts
    // the scan loudly (the pre-accumulation behavior, preserved -
    // never silently skipped).
    let deadline = time::Instant::now() + options.timeout;
    let mut gateways = Vec::new();
    let mut buffer = [0_u8; 1500];
    loop {
        let remaining = deadline.saturating_duration_since(time::Instant::now());
        if remaining.is_zero() {
            return Ok(gateways);
        }
        match time::timeout(remaining, transport.recv_from(&mut buffer)).await {
            Ok(Ok((len, received_from))) => {
                let gateway = decode_search_response(&buffer[..len], received_from)?;
                gateways.push(gateway);
            }
            Ok(Err(error)) => return Err(error),
            Err(_) => return Ok(gateways),
        }
    }
}

fn encode_search_request(local_addr: SocketAddr) -> Result<Vec<u8>> {
    let mut payload = Vec::new();
    Hpai::new(
        HostProtocol::Ipv4Udp,
        ipv4_octets_or_unspecified(local_addr.ip()),
        local_addr.port(),
    )
    .encode(&mut payload)?;

    let mut frame = Vec::new();
    KnxNetIpHeader::new(ServiceType::SearchRequest, (6 + payload.len()) as u16)?
        .encode(&mut frame)?;
    frame.extend_from_slice(&payload);

    Ok(frame)
}

fn decode_search_response(input: &[u8], received_from: SocketAddr) -> Result<Gateway> {
    let (header, payload) = KnxNetIpHeader::decode(input)?;
    if header.service_type() != ServiceType::SearchResponse {
        return Err(KnxIpError::InvalidResponse("expected search response"));
    }

    let (control_endpoint, remaining) = Hpai::decode(payload)?;
    let control_endpoint = SocketAddr::from((
        Ipv4Addr::from(control_endpoint.address()),
        control_endpoint.port(),
    ));
    let service_families = decode_service_families(remaining)?;

    Ok(Gateway {
        control_endpoint,
        service_families,
        received_from,
    })
}

fn decode_service_families(mut input: &[u8]) -> Result<Vec<ServiceFamily>> {
    let mut families = Vec::new();

    while !input.is_empty() {
        if input.len() < 2 {
            return Err(KnxIpError::InvalidResponse("truncated DIB header"));
        }

        let len = input[0] as usize;
        let dib_type = input[1];
        if len < 2 || input.len() < len {
            return Err(KnxIpError::InvalidResponse("invalid DIB length"));
        }

        if dib_type == 0x02 {
            let entries = &input[2..len];
            if entries.len() % 2 != 0 {
                return Err(KnxIpError::InvalidResponse("invalid service family list"));
            }
            for entry in entries.chunks_exact(2) {
                families.push(ServiceFamily {
                    id: entry[0],
                    version: entry[1],
                });
            }
        }

        input = &input[len..];
    }

    Ok(families)
}

fn ipv4_octets_or_unspecified(ip: IpAddr) -> [u8; 4] {
    match ip {
        IpAddr::V4(ip) => ip.octets(),
        IpAddr::V6(_) => Ipv4Addr::UNSPECIFIED.octets(),
    }
}

trait BroadcastCheck {
    fn is_broadcast(&self) -> bool;
}

impl BroadcastCheck for IpAddr {
    fn is_broadcast(&self) -> bool {
        matches!(self, IpAddr::V4(ip) if *ip == Ipv4Addr::BROADCAST)
    }
}
