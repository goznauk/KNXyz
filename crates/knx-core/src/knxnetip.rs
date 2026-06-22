use core::convert::TryFrom;

use crate::{KnxError, Result};

pub const HEADER_LENGTH: u8 = 0x06;
pub const PROTOCOL_VERSION_1_0: u8 = 0x10;

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum ServiceType {
    SearchRequest = 0x0201,
    SearchResponse = 0x0202,
    DescriptionRequest = 0x0203,
    DescriptionResponse = 0x0204,
    ConnectRequest = 0x0205,
    ConnectResponse = 0x0206,
    ConnectionStateRequest = 0x0207,
    ConnectionStateResponse = 0x0208,
    DisconnectRequest = 0x0209,
    DisconnectResponse = 0x020a,
    TunnellingRequest = 0x0420,
    TunnellingAck = 0x0421,
    RoutingIndication = 0x0530,
    RoutingLostMessage = 0x0531,
    RoutingBusy = 0x0532,
}

impl ServiceType {
    /// All `ServiceType` variants in declaration order; the canonical single
    /// source for iteration/decoding.
    pub const ALL: [ServiceType; 15] = [
        ServiceType::SearchRequest,
        ServiceType::SearchResponse,
        ServiceType::DescriptionRequest,
        ServiceType::DescriptionResponse,
        ServiceType::ConnectRequest,
        ServiceType::ConnectResponse,
        ServiceType::ConnectionStateRequest,
        ServiceType::ConnectionStateResponse,
        ServiceType::DisconnectRequest,
        ServiceType::DisconnectResponse,
        ServiceType::TunnellingRequest,
        ServiceType::TunnellingAck,
        ServiceType::RoutingIndication,
        ServiceType::RoutingLostMessage,
        ServiceType::RoutingBusy,
    ];

    pub const fn as_u16(self) -> u16 {
        self as u16
    }
}

// `ServiceType::ALL` together with `as_u16` is now the single source of truth
// for the variant <-> discriminant mapping; decoding scans `ALL` rather than
// mirroring the discriminants in a separate table.
impl TryFrom<u16> for ServiceType {
    type Error = KnxError;

    fn try_from(value: u16) -> Result<Self> {
        ServiceType::ALL
            .iter()
            .copied()
            .find(|st| st.as_u16() == value)
            .ok_or(KnxError::UnsupportedServiceType(value))
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KnxNetIpHeader {
    service_type: ServiceType,
    total_length: u16,
}

impl KnxNetIpHeader {
    pub const fn new(service_type: ServiceType, total_length: u16) -> Result<Self> {
        if total_length < HEADER_LENGTH as u16 {
            return Err(KnxError::InvalidFrame("total length shorter than header"));
        }

        Ok(Self {
            service_type,
            total_length,
        })
    }

    pub const fn service_type(self) -> ServiceType {
        self.service_type
    }

    pub const fn total_length(self) -> u16 {
        self.total_length
    }

    pub fn decode(input: &[u8]) -> Result<(Self, &[u8])> {
        if input.len() < HEADER_LENGTH as usize {
            return Err(KnxError::BufferTooShort {
                needed: HEADER_LENGTH as usize,
                actual: input.len(),
            });
        }

        if input[0] != HEADER_LENGTH {
            return Err(KnxError::InvalidFrame("invalid KNXnet/IP header length"));
        }
        if input[1] != PROTOCOL_VERSION_1_0 {
            return Err(KnxError::InvalidFrame("invalid KNXnet/IP protocol version"));
        }

        let service_type = ServiceType::try_from(u16::from_be_bytes([input[2], input[3]]))?;
        let total_length = u16::from_be_bytes([input[4], input[5]]);
        let header = Self::new(service_type, total_length)?;

        let total_length = usize::from(total_length);
        if input.len() < total_length {
            return Err(KnxError::BufferTooShort {
                needed: total_length,
                actual: input.len(),
            });
        }

        Ok((header, &input[HEADER_LENGTH as usize..total_length]))
    }

    #[cfg(feature = "std")]
    pub fn encode(self, out: &mut std::vec::Vec<u8>) -> Result<()> {
        out.extend_from_slice(&[
            HEADER_LENGTH,
            PROTOCOL_VERSION_1_0,
            (self.service_type.as_u16() >> 8) as u8,
            self.service_type.as_u16() as u8,
            (self.total_length >> 8) as u8,
            self.total_length as u8,
        ]);
        Ok(())
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum HostProtocol {
    Ipv4Udp = 0x01,
    Ipv4Tcp = 0x02,
}

// All `HostProtocol` variants in declaration order; together with `as_u8`
// this is the single source of truth for the variant <-> byte mapping.
const HOST_PROTOCOL_ALL: [HostProtocol; 2] = [HostProtocol::Ipv4Udp, HostProtocol::Ipv4Tcp];

impl HostProtocol {
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

impl TryFrom<u8> for HostProtocol {
    type Error = KnxError;

    fn try_from(value: u8) -> Result<Self> {
        HOST_PROTOCOL_ALL
            .iter()
            .copied()
            .find(|hp| hp.as_u8() == value)
            .ok_or(KnxError::InvalidFrame("unsupported host protocol"))
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hpai {
    protocol: HostProtocol,
    address: [u8; 4],
    port: u16,
}

impl Hpai {
    pub const LENGTH: u8 = 0x08;

    pub const fn new(protocol: HostProtocol, address: [u8; 4], port: u16) -> Self {
        Self {
            protocol,
            address,
            port,
        }
    }

    pub const fn protocol(self) -> HostProtocol {
        self.protocol
    }

    pub const fn address(self) -> [u8; 4] {
        self.address
    }

    pub const fn port(self) -> u16 {
        self.port
    }

    pub fn decode(input: &[u8]) -> Result<(Self, &[u8])> {
        if input.len() < Self::LENGTH as usize {
            return Err(KnxError::BufferTooShort {
                needed: Self::LENGTH as usize,
                actual: input.len(),
            });
        }
        if input[0] != Self::LENGTH {
            return Err(KnxError::InvalidFrame("invalid HPAI length"));
        }

        let protocol = HostProtocol::try_from(input[1])?;
        let address = [input[2], input[3], input[4], input[5]];
        let port = u16::from_be_bytes([input[6], input[7]]);

        Ok((
            Self::new(protocol, address, port),
            &input[Self::LENGTH as usize..],
        ))
    }

    #[cfg(feature = "std")]
    pub fn encode(self, out: &mut std::vec::Vec<u8>) -> Result<()> {
        out.extend_from_slice(&[
            Self::LENGTH,
            self.protocol.as_u8(),
            self.address[0],
            self.address[1],
            self.address[2],
            self.address[3],
            (self.port >> 8) as u8,
            self.port as u8,
        ]);
        Ok(())
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionHeader {
    channel_id: u8,
    sequence_counter: u8,
    status: u8,
}

impl ConnectionHeader {
    pub const LENGTH: u8 = 0x04;

    pub const fn new(channel_id: u8, sequence_counter: u8, status: u8) -> Self {
        Self {
            channel_id,
            sequence_counter,
            status,
        }
    }

    pub const fn channel_id(self) -> u8 {
        self.channel_id
    }

    pub const fn sequence_counter(self) -> u8 {
        self.sequence_counter
    }

    pub const fn status(self) -> u8 {
        self.status
    }

    pub fn decode(input: &[u8]) -> Result<(Self, &[u8])> {
        if input.len() < Self::LENGTH as usize {
            return Err(KnxError::BufferTooShort {
                needed: Self::LENGTH as usize,
                actual: input.len(),
            });
        }
        if input[0] != Self::LENGTH {
            return Err(KnxError::InvalidFrame("invalid connection header length"));
        }

        Ok((
            Self::new(input[1], input[2], input[3]),
            &input[Self::LENGTH as usize..],
        ))
    }

    #[cfg(feature = "std")]
    pub fn encode(self, out: &mut std::vec::Vec<u8>) -> Result<()> {
        out.extend_from_slice(&[
            Self::LENGTH,
            self.channel_id,
            self.sequence_counter,
            self.status,
        ]);
        Ok(())
    }
}
