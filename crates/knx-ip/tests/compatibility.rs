use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use std::time::Duration;

use knx_core::{Apci, GroupAddress, HostProtocol, Hpai};
use knx_dpt::DptValue;
use knx_ip::{
    ConnectionEvent, HeartbeatOptions, KnxIpError, ReconnectPolicy, TunnelClient, TunnelOptions,
};
use knx_sim::{SimEvent, SimGateway};

#[test]
fn tunnel_options_default_bind_is_ipv4_unspecified_ephemeral() {
    let target = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 3671));
    let options = TunnelOptions::new(target);

    assert_eq!(
        options.bind,
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))
    );
    assert_eq!(options.target, target);
    assert_eq!(options.control_endpoint, None);
    assert_eq!(options.data_endpoint, None);
    assert_eq!(options.ack_timeout, Duration::from_secs(1));
}

#[tokio::test]
async fn tunnel_options_advertise_nat_control_and_data_endpoints() {
    let mut gateway = SimGateway::bind_localhost().await.unwrap();
    let gateway_addr = gateway.local_addr();
    let advertised_control = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(192, 0, 2, 10), 1111));
    let advertised_data = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(192, 0, 2, 11), 2222));

    let gateway_task = tokio::spawn(async move {
        let event = gateway.expect_connect_request().await.unwrap();
        let SimEvent::ConnectRequest {
            control_endpoint,
            data_endpoint,
            ..
        } = event
        else {
            panic!("expected connect request");
        };

        assert_eq!(
            control_endpoint,
            Hpai::new(HostProtocol::Ipv4Udp, [192, 0, 2, 10], 1111)
        );
        assert_eq!(
            data_endpoint,
            Hpai::new(HostProtocol::Ipv4Udp, [192, 0, 2, 11], 2222)
        );
        gateway.reply_connect_response(0x51).await.unwrap();
        gateway.shutdown().await.unwrap();
    });

    let options = TunnelOptions {
        target: gateway_addr,
        control_endpoint: Some(advertised_control),
        data_endpoint: Some(advertised_data),
        ..TunnelOptions::new(gateway_addr)
    };
    let _client = TunnelClient::connect_with_options(options).await.unwrap();

    gateway_task.await.unwrap();
}

#[tokio::test]
async fn tunnel_options_reject_ipv6_target_with_typed_error() {
    let target = SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::LOCALHOST, 3671, 0, 0));
    let error = TunnelClient::connect_with_options(TunnelOptions::new(target))
        .await
        .unwrap_err();

    assert!(matches!(error, KnxIpError::UnsupportedIpv6));
}

#[tokio::test]
async fn reconnect_policy_retries_initial_connect_and_resets_sequence() {
    let mut gateway = SimGateway::bind_localhost().await.unwrap();
    let gateway_addr = gateway.local_addr();
    let group: GroupAddress = "1/2/3".parse().unwrap();

    let gateway_task = tokio::spawn(async move {
        gateway.expect_connect_request().await.unwrap();

        gateway.expect_connect_request().await.unwrap();
        gateway.reply_connect_response(0x52).await.unwrap();

        let write_frame = gateway.expect_tunnelling_request(0).await.unwrap();
        assert_eq!(write_frame.telegram().destination(), group);
        assert_eq!(write_frame.telegram().apci(), Apci::GroupValueWrite);
        gateway.reply_tunnelling_ack(0x52, 0).await.unwrap();
        gateway.shutdown().await.unwrap();
    });

    let mut options = TunnelOptions::new(gateway_addr);
    options.ack_timeout = Duration::from_millis(20);
    options.reconnect_policy = Some(ReconnectPolicy::bounded(
        2,
        Duration::from_millis(1),
        Duration::from_millis(2),
    ));

    let mut client = TunnelClient::connect_with_options(options).await.unwrap();
    assert_eq!(
        client.lifecycle_events(),
        &[
            ConnectionEvent::Reconnecting {
                attempt: 2,
                delay: Duration::from_millis(1),
            },
            ConnectionEvent::Reconnected {
                attempt: 2,
                channel_id: 0x52,
            },
        ]
    );
    drop(client.monitor());

    client
        .group_write(group, DptValue::Bool(true))
        .await
        .unwrap();
    gateway_task.await.unwrap();
}

#[tokio::test]
async fn reconnect_policy_stops_after_max_attempts() {
    let mut gateway = SimGateway::bind_localhost().await.unwrap();
    let gateway_addr = gateway.local_addr();

    let gateway_task = tokio::spawn(async move {
        gateway.expect_connect_request().await.unwrap();
        gateway.expect_connect_request().await.unwrap();
        gateway.shutdown().await.unwrap();
    });

    let mut options = TunnelOptions::new(gateway_addr);
    options.ack_timeout = Duration::from_millis(10);
    options.reconnect_policy = Some(ReconnectPolicy::bounded(
        2,
        Duration::from_millis(1),
        Duration::from_millis(1),
    ));

    let error = TunnelClient::connect_with_options(options)
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        KnxIpError::ReconnectAttemptsExhausted { attempts: 2 }
    ));

    gateway_task.await.unwrap();
}

#[tokio::test]
async fn heartbeat_sends_connection_state_request_and_accepts_success_response() {
    let mut gateway = SimGateway::bind_localhost().await.unwrap();
    let gateway_addr = gateway.local_addr();
    let channel_id = 0x61;

    let gateway_task = tokio::spawn(async move {
        gateway.expect_connect_request().await.unwrap();
        gateway.reply_connect_response(channel_id).await.unwrap();

        gateway
            .expect_connection_state_request(channel_id)
            .await
            .unwrap();
        gateway
            .reply_connection_state_response(channel_id, 0x00)
            .await
            .unwrap();
        gateway.shutdown().await.unwrap();
    });

    let mut options = TunnelOptions::new(gateway_addr);
    options.heartbeat = Some(HeartbeatOptions::new(
        Duration::from_millis(1),
        Duration::from_millis(25),
        1,
    ));

    let client = TunnelClient::connect_with_options(options).await.unwrap();
    client.heartbeat().await.unwrap();
    gateway_task.await.unwrap();
}

#[tokio::test]
async fn heartbeat_timeout_returns_typed_error() {
    let mut gateway = SimGateway::bind_localhost().await.unwrap();
    let gateway_addr = gateway.local_addr();
    let channel_id = 0x62;

    let gateway_task = tokio::spawn(async move {
        gateway.expect_connect_request().await.unwrap();
        gateway.reply_connect_response(channel_id).await.unwrap();

        gateway
            .expect_connection_state_request(channel_id)
            .await
            .unwrap();
        gateway.shutdown().await.unwrap();
    });

    let mut options = TunnelOptions::new(gateway_addr);
    options.heartbeat = Some(HeartbeatOptions::new(
        Duration::from_millis(1),
        Duration::from_millis(10),
        1,
    ));

    let client = TunnelClient::connect_with_options(options).await.unwrap();
    let error = client.heartbeat().await.unwrap_err();
    assert!(matches!(error, KnxIpError::HeartbeatTimeout));

    gateway_task.await.unwrap();
}

#[tokio::test]
async fn heartbeat_gateway_status_returns_typed_error() {
    let mut gateway = SimGateway::bind_localhost().await.unwrap();
    let gateway_addr = gateway.local_addr();
    let channel_id = 0x63;

    let gateway_task = tokio::spawn(async move {
        gateway.expect_connect_request().await.unwrap();
        gateway.reply_connect_response(channel_id).await.unwrap();

        gateway
            .expect_connection_state_request(channel_id)
            .await
            .unwrap();
        gateway
            .reply_connection_state_response(channel_id, 0x21)
            .await
            .unwrap();
        gateway.shutdown().await.unwrap();
    });

    let mut options = TunnelOptions::new(gateway_addr);
    options.heartbeat = Some(HeartbeatOptions::new(
        Duration::from_millis(1),
        Duration::from_millis(25),
        1,
    ));

    let client = TunnelClient::connect_with_options(options).await.unwrap();
    let error = client.heartbeat().await.unwrap_err();
    assert!(matches!(error, KnxIpError::GatewayStatus { status: 0x21 }));

    gateway_task.await.unwrap();
}
