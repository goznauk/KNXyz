use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use knx_core::{
    Apci, CemiFrame, CemiMessageCode, GroupAddress, GroupTelegram, IndividualAddress,
    KnxNetIpHeader, ServiceType,
};
use knx_ip::{decode_routing_packet, encode_routing_packet, KnxIpError, RouteEvent};

#[test]
fn routing_packet_decode_returns_route_event() {
    let peer = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 3671));
    let source: IndividualAddress = "1.1.10".parse().unwrap();
    let destination: GroupAddress = "1/2/3".parse().unwrap();
    let packet = routing_indication_packet(source, destination, Apci::GroupValueWrite, &[0x01]);

    let event = decode_routing_packet(&packet, peer).unwrap().unwrap();

    assert_eq!(
        event,
        RouteEvent {
            source,
            destination,
            apci: Apci::GroupValueWrite,
            payload: vec![0x01],
            peer,
        }
    );
}

#[test]
fn routing_packet_decode_returns_typed_error_for_malformed_header() {
    let peer = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 3671));
    let error = decode_routing_packet(&[0x01, 0x02, 0x03], peer).unwrap_err();

    assert!(matches!(error, KnxIpError::Core(_)));
}

#[test]
fn routing_packet_decode_ignores_non_routing_service_types() {
    let peer = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 3671));
    let mut packet = Vec::new();
    KnxNetIpHeader::new(ServiceType::SearchRequest, 6)
        .unwrap()
        .encode(&mut packet)
        .unwrap();

    assert_eq!(decode_routing_packet(&packet, peer).unwrap(), None);
}

#[test]
fn routing_packet_encode_wraps_cemi_in_routing_indication() {
    let source: IndividualAddress = "1.1.10".parse().unwrap();
    let destination: GroupAddress = "1/2/3".parse().unwrap();
    let telegram = GroupTelegram::new(source, destination, Apci::GroupValueWrite, &[0x01]).unwrap();
    let frame = CemiFrame::new(CemiMessageCode::LDataIndication, telegram);

    let packet = encode_routing_packet(&frame).unwrap();

    assert_eq!(&packet[..6], &[0x06, 0x10, 0x05, 0x30, 0x00, 0x11]);
    assert_eq!(
        &packet[6..],
        &[0x29, 0x00, 0xbc, 0xe0, 0x11, 0x0a, 0x0a, 0x03, 0x01, 0x00, 0x81]
    );
}

#[test]
fn encoded_routing_packet_decodes_as_route_event() {
    let peer = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 3671));
    let source: IndividualAddress = "1.1.10".parse().unwrap();
    let destination: GroupAddress = "1/2/3".parse().unwrap();
    let telegram =
        GroupTelegram::new(source, destination, Apci::GroupValueResponse, &[0x01]).unwrap();
    let frame = CemiFrame::new(CemiMessageCode::LDataIndication, telegram);

    let packet = encode_routing_packet(&frame).unwrap();
    let event = decode_routing_packet(&packet, peer).unwrap().unwrap();

    assert_eq!(event.source, source);
    assert_eq!(event.destination, destination);
    assert_eq!(event.apci, Apci::GroupValueResponse);
    assert_eq!(event.payload, [0x01]);
    assert_eq!(event.peer, peer);
}

fn routing_indication_packet(
    source: IndividualAddress,
    destination: GroupAddress,
    apci: Apci,
    payload: &[u8],
) -> Vec<u8> {
    let telegram = GroupTelegram::new(source, destination, apci, payload).unwrap();
    let frame = CemiFrame::new(CemiMessageCode::LDataIndication, telegram);
    let mut cemi = Vec::new();
    frame.encode(&mut cemi).unwrap();

    let mut packet = Vec::new();
    KnxNetIpHeader::new(ServiceType::RoutingIndication, (6 + cemi.len()) as u16)
        .unwrap()
        .encode(&mut packet)
        .unwrap();
    packet.extend_from_slice(&cemi);
    packet
}
