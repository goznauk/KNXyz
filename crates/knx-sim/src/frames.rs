use std::net::SocketAddr;

use knx_core::{
    Apci, CemiFrame, CemiMessageCode, ConnectionHeader, GroupAddress, GroupTelegram,
    IndividualAddress, KnxNetIpHeader, ServiceType,
};

use crate::gateway::{ipv4_hpai_for, SimError};

pub(crate) fn search_response(
    control_endpoint: SocketAddr,
    service_families: &[(u8, u8)],
) -> Result<Vec<u8>, SimError> {
    let service_family_len = 2 + service_families.len() * 2;
    let service_family_len = u8::try_from(service_family_len)
        .map_err(|_| SimError::InvalidScript("too many service families"))?;

    let mut payload = Vec::new();
    ipv4_hpai_for(control_endpoint).encode(&mut payload)?;
    payload.push(service_family_len);
    payload.push(0x02);
    for (id, version) in service_families {
        payload.push(*id);
        payload.push(*version);
    }

    knxnetip(ServiceType::SearchResponse, &payload)
}

pub(crate) fn connect_response(
    channel_id: u8,
    data_endpoint: SocketAddr,
) -> Result<Vec<u8>, SimError> {
    let mut payload = vec![channel_id, 0x00];
    ipv4_hpai_for(data_endpoint).encode(&mut payload)?;
    payload.extend_from_slice(&[0x04, 0x04, 0x02, 0x00]);

    knxnetip(ServiceType::ConnectResponse, &payload)
}

pub(crate) fn tunnelling_ack(channel_id: u8, sequence: u8) -> Result<Vec<u8>, SimError> {
    let mut payload = Vec::new();
    ConnectionHeader::new(channel_id, sequence, 0x00).encode(&mut payload)?;

    knxnetip(ServiceType::TunnellingAck, &payload)
}

pub(crate) fn connection_state_response(channel_id: u8, status: u8) -> Result<Vec<u8>, SimError> {
    knxnetip(ServiceType::ConnectionStateResponse, &[channel_id, status])
}

pub(crate) fn disconnect_response(channel_id: u8, status: u8) -> Result<Vec<u8>, SimError> {
    knxnetip(ServiceType::DisconnectResponse, &[channel_id, status])
}

pub(crate) fn tunnelling_indication(
    channel_id: u8,
    sequence: u8,
    group: GroupAddress,
    apci: Apci,
    payload: &[u8],
) -> Result<Vec<u8>, SimError> {
    let source = IndividualAddress::from_raw(0x110a);
    let telegram = GroupTelegram::new(source, group, apci, payload)?;
    let frame = CemiFrame::new(CemiMessageCode::LDataIndication, telegram);

    let mut body = Vec::new();
    ConnectionHeader::new(channel_id, sequence, 0x00).encode(&mut body)?;
    frame.encode(&mut body)?;

    knxnetip(ServiceType::TunnellingRequest, &body)
}

pub(crate) fn tunnelling_request_with_cemi(
    channel_id: u8,
    sequence: u8,
    cemi: &[u8],
) -> Result<Vec<u8>, SimError> {
    let mut body = Vec::new();
    ConnectionHeader::new(channel_id, sequence, 0x00).encode(&mut body)?;
    body.extend_from_slice(cemi);

    knxnetip(ServiceType::TunnellingRequest, &body)
}

pub(crate) fn knxnetip(service_type: ServiceType, payload: &[u8]) -> Result<Vec<u8>, SimError> {
    let total_length = u16::try_from(6 + payload.len())
        .map_err(|_| SimError::InvalidScript("KNXnet/IP frame too long"))?;
    let mut frame = Vec::new();
    KnxNetIpHeader::new(service_type, total_length)?.encode(&mut frame)?;
    frame.extend_from_slice(payload);

    Ok(frame)
}
