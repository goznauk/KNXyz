use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

use knx_core::{
    Apci, CemiFrame, CemiMessageCode, GroupAddress, GroupTelegram, IndividualAddress,
    KnxNetIpHeader, ServiceType,
};
use knx_dpt::DptValue;
use knx_ip::{
    bind_routing_socket, KnxIpError, RouteMonitor, RouteSender, RoutingOptions, RoutingSendOptions,
    DEFAULT_ROUTING_MULTICAST, DEFAULT_ROUTING_PORT,
};
use tokio::net::UdpSocket;
use tokio_stream::StreamExt;

#[test]
fn routing_options_default_uses_standard_multicast_endpoint() {
    let options = RoutingOptions::default();

    assert_eq!(
        options.bind,
        SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, DEFAULT_ROUTING_PORT)
    );
    assert_eq!(options.multicast, DEFAULT_ROUTING_MULTICAST);
    assert_eq!(options.port, DEFAULT_ROUTING_PORT);
    assert_eq!(options.interface, None);
    assert!(!options.loopback);
}

#[tokio::test]
async fn routing_options_accept_loopback_interface_for_tests() {
    let options = RoutingOptions {
        bind: SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0),
        interface: Some(Ipv4Addr::LOCALHOST),
        loopback: true,
        ..RoutingOptions::default()
    };

    let socket = bind_routing_socket(options).await.unwrap();

    assert!(socket.local_addr().unwrap().port() > 0);
}

#[test]
fn routing_options_reject_non_ipv4_multicast_target() {
    let bind = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0));
    let target = SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::LOCALHOST, 3671, 0, 0));

    let error = RoutingOptions::from_target(bind, target, None, false).unwrap_err();

    assert!(matches!(
        error,
        KnxIpError::InvalidResponse("routing requires IPv4 multicast target")
    ));
}

#[test]
fn routing_send_options_default_uses_standard_multicast_endpoint() {
    let options = RoutingSendOptions::default();

    assert_eq!(options.bind, SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0));
    assert_eq!(
        options.target,
        SocketAddrV4::new(DEFAULT_ROUTING_MULTICAST, DEFAULT_ROUTING_PORT)
    );
    assert_eq!(options.interface, None);
    assert!(!options.loopback);
    assert_eq!(options.ttl, 1);
}

#[test]
fn routing_send_options_reject_non_ipv4_multicast_target() {
    let bind = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0));
    let target = SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::LOCALHOST, 3671, 0, 0));

    let error = RoutingSendOptions::from_target(bind, target, None, false, 1).unwrap_err();

    assert!(matches!(
        error,
        KnxIpError::InvalidResponse("routing requires IPv4 multicast target")
    ));
}

#[tokio::test]
#[ignore = "requires KNXYZ_ROUTING_INTERFACE with a local IPv4 interface"]
async fn joins_real_routing_multicast_when_hardware_network_is_configured() {
    let interface = std::env::var("KNXYZ_ROUTING_INTERFACE")
        .expect("KNXYZ_ROUTING_INTERFACE must name a local IPv4 interface");
    let interface = interface
        .parse::<Ipv4Addr>()
        .expect("KNXYZ_ROUTING_INTERFACE must be an IPv4 address");

    let options = RoutingOptions {
        interface: Some(interface),
        ..RoutingOptions::default()
    };

    let socket = bind_routing_socket(options).await.unwrap();
    assert_eq!(socket.local_addr().unwrap().port(), DEFAULT_ROUTING_PORT);
}

#[tokio::test]
async fn route_sender_sends_loopback_indication_to_monitor() {
    let port = unused_loopback_port();
    let receive_options = RoutingOptions {
        bind: SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port),
        port,
        interface: Some(Ipv4Addr::LOCALHOST),
        loopback: true,
        ..RoutingOptions::default()
    };
    let send_options = RoutingSendOptions {
        target: SocketAddrV4::new(DEFAULT_ROUTING_MULTICAST, port),
        interface: Some(Ipv4Addr::LOCALHOST),
        loopback: true,
        ..RoutingSendOptions::default()
    };
    let monitor = RouteMonitor::bind(receive_options).await.unwrap();
    let mut events = monitor.events();
    let sender = RouteSender::bind(send_options).await.unwrap();
    let source: IndividualAddress = "1.1.10".parse().unwrap();
    let destination: GroupAddress = "1/2/3".parse().unwrap();
    let telegram = GroupTelegram::new(source, destination, Apci::GroupValueWrite, &[0x01]).unwrap();
    let frame = CemiFrame::new(CemiMessageCode::LDataIndication, telegram);

    sender.send_frame(&frame).await.unwrap();

    let event = tokio::time::timeout(std::time::Duration::from_secs(1), events.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(event.source, source);
    assert_eq!(event.destination, destination);
    assert_eq!(event.apci, Apci::GroupValueWrite);
    assert_eq!(event.payload, [0x01]);
}

#[tokio::test]
async fn send_group_write_round_trips_a_dpt_value_on_loopback() {
    // exercises the exact send path the Python binding uses
    // (RouteSender::send_group_write: DPT-id inference -> encode ->
    // L_Data.indication frame), egress-free via IP_MULTICAST_LOOP.
    let port = unused_loopback_port();
    let receive_options = RoutingOptions {
        bind: SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port),
        port,
        interface: Some(Ipv4Addr::LOCALHOST),
        loopback: true,
        ..RoutingOptions::default()
    };
    let send_options = RoutingSendOptions {
        target: SocketAddrV4::new(DEFAULT_ROUTING_MULTICAST, port),
        interface: Some(Ipv4Addr::LOCALHOST),
        loopback: true,
        ..RoutingSendOptions::default()
    };
    let monitor = RouteMonitor::bind(receive_options).await.unwrap();
    let mut events = monitor.events();
    let sender = RouteSender::bind(send_options).await.unwrap();
    let source: IndividualAddress = "1.1.10".parse().unwrap();
    let destination: GroupAddress = "1/2/3".parse().unwrap();

    // DPT 1.001 true (inferred from DptValue::Bool) -> wire byte [0x01]
    sender
        .send_group_write(source, destination, DptValue::Bool(true))
        .await
        .unwrap();

    let event = tokio::time::timeout(std::time::Duration::from_secs(1), events.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(event.source, source);
    assert_eq!(event.destination, destination);
    assert_eq!(event.apci, Apci::GroupValueWrite);
    assert_eq!(event.payload, [0x01]);
}

#[tokio::test]
async fn route_monitor_receives_loopback_indication() {
    let port = unused_loopback_port();
    let options = RoutingOptions {
        bind: SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port),
        port,
        interface: Some(Ipv4Addr::LOCALHOST),
        loopback: true,
        ..RoutingOptions::default()
    };
    let monitor = RouteMonitor::bind(options).await.unwrap();
    let mut events = monitor.events();

    let peer = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let source: IndividualAddress = "1.1.10".parse().unwrap();
    let destination: GroupAddress = "1/2/3".parse().unwrap();
    let packet = routing_indication_packet(source, destination, Apci::GroupValueResponse, &[0x01]);
    peer.send_to(&packet, SocketAddrV4::new(Ipv4Addr::LOCALHOST, port))
        .await
        .unwrap();

    let event = tokio::time::timeout(std::time::Duration::from_secs(1), events.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    assert_eq!(event.source, source);
    assert_eq!(event.destination, destination);
    assert_eq!(event.apci, Apci::GroupValueResponse);
    assert_eq!(event.payload, [0x01]);
}

#[tokio::test]
async fn route_monitor_ignores_non_routing_and_reports_malformed_routing() {
    let port = unused_loopback_port();
    let options = RoutingOptions {
        bind: SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port),
        port,
        interface: Some(Ipv4Addr::LOCALHOST),
        loopback: true,
        ..RoutingOptions::default()
    };
    let monitor = RouteMonitor::bind(options).await.unwrap();
    let mut events = monitor.events();
    let peer = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();

    let mut non_routing = Vec::new();
    KnxNetIpHeader::new(ServiceType::SearchRequest, 6)
        .unwrap()
        .encode(&mut non_routing)
        .unwrap();
    peer.send_to(&non_routing, SocketAddrV4::new(Ipv4Addr::LOCALHOST, port))
        .await
        .unwrap();

    let mut malformed_routing = Vec::new();
    KnxNetIpHeader::new(ServiceType::RoutingIndication, 6)
        .unwrap()
        .encode(&mut malformed_routing)
        .unwrap();
    peer.send_to(
        &malformed_routing,
        SocketAddrV4::new(Ipv4Addr::LOCALHOST, port),
    )
    .await
    .unwrap();

    let error = tokio::time::timeout(std::time::Duration::from_secs(1), events.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert!(matches!(error, KnxIpError::Core(_)));
}

fn unused_loopback_port() -> u16 {
    std::net::UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
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
