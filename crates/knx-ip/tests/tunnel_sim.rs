use std::time::Duration;

use knx_core::{Apci, GroupAddress};
use knx_dpt::DptValue;
use knx_ip::{KnxIpError, TunnelClient};
use knx_sim::{SimEvent, SimGateway};
use tokio_stream::StreamExt;

#[tokio::test]
async fn tunnel_client_uses_sim_gateway() {
    let mut gateway = SimGateway::bind_localhost().await.unwrap();
    let gateway_addr = gateway.local_addr();
    let channel_id = 0x31;
    let group: GroupAddress = "1/2/3".parse().unwrap();

    let gateway_task = tokio::spawn(async move {
        gateway.expect_connect_request().await.unwrap();
        gateway.reply_connect_response(channel_id).await.unwrap();

        let write_frame = gateway.expect_tunnelling_request(0).await.unwrap();
        assert_eq!(write_frame.telegram().destination(), group);
        assert_eq!(write_frame.telegram().apci(), Apci::GroupValueWrite);
        assert_eq!(write_frame.telegram().payload(), &[0x01]);
        gateway.reply_tunnelling_ack(channel_id, 0).await.unwrap();

        let read_frame = gateway.expect_tunnelling_request(1).await.unwrap();
        assert_eq!(read_frame.telegram().destination(), group);
        assert_eq!(read_frame.telegram().apci(), Apci::GroupValueRead);
        gateway.reply_tunnelling_ack(channel_id, 1).await.unwrap();

        gateway
            .send_tunnelling_indication(channel_id, 0, group, Apci::GroupValueResponse, &[0x01])
            .await
            .unwrap();
        gateway.shutdown().await.unwrap();
    });

    let mut client = TunnelClient::connect(gateway_addr).await.unwrap();
    client
        .group_write(group, DptValue::Bool(true))
        .await
        .unwrap();

    let read_value = client
        .group_read(group, "1.001", Duration::from_secs(1))
        .await
        .unwrap();
    assert_eq!(read_value, DptValue::Bool(true));

    gateway_task.await.unwrap();
}

#[tokio::test]
async fn tunnel_client_connects_writes_reads_and_monitors_group_telegrams() {
    let mut gateway = SimGateway::bind_localhost().await.unwrap();
    let gateway_addr = gateway.local_addr();
    let channel_id = 0x21;
    let group: GroupAddress = "1/2/3".parse().unwrap();

    let gateway_task = tokio::spawn(async move {
        gateway.expect_connect_request().await.unwrap();
        gateway.reply_connect_response(channel_id).await.unwrap();

        let write_frame = gateway.expect_tunnelling_request(0).await.unwrap();
        assert_eq!(write_frame.telegram().destination(), group);
        assert_eq!(write_frame.telegram().apci(), Apci::GroupValueWrite);
        assert_eq!(write_frame.telegram().payload(), &[0x01]);
        gateway.reply_tunnelling_ack(channel_id, 0).await.unwrap();

        let read_frame = gateway.expect_tunnelling_request(1).await.unwrap();
        assert_eq!(read_frame.telegram().destination(), group);
        assert_eq!(read_frame.telegram().apci(), Apci::GroupValueRead);
        gateway.reply_tunnelling_ack(channel_id, 1).await.unwrap();

        gateway
            .send_tunnelling_indication(channel_id, 0, group, Apci::GroupValueResponse, &[0x01])
            .await
            .unwrap();
        gateway
            .send_tunnelling_indication(channel_id, 1, group, Apci::GroupValueResponse, &[0x00])
            .await
            .unwrap();
        gateway.shutdown().await.unwrap();
    });

    let mut client = TunnelClient::connect(gateway_addr).await.unwrap();
    client
        .group_write(group, DptValue::Bool(true))
        .await
        .unwrap();

    let mut monitor = client.monitor();
    let read_value = client
        .group_read(group, "1.001", Duration::from_secs(1))
        .await
        .unwrap();
    assert_eq!(read_value, DptValue::Bool(true));

    let event = tokio::time::timeout(Duration::from_secs(1), monitor.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(event.destination, group);
    assert_eq!(event.apci, Apci::GroupValueResponse);
    assert_eq!(event.payload, [0x01]);

    gateway_task.await.unwrap();
}

#[tokio::test]
async fn group_read_returns_typed_timeout_when_gateway_does_not_respond() {
    let mut gateway = SimGateway::bind_localhost().await.unwrap();
    let gateway_addr = gateway.local_addr();
    let channel_id = 0x22;
    let group: GroupAddress = "1/2/3".parse().unwrap();

    let gateway_task = tokio::spawn(async move {
        gateway.expect_connect_request().await.unwrap();
        gateway.reply_connect_response(channel_id).await.unwrap();

        let read_frame = gateway.expect_tunnelling_request(0).await.unwrap();
        assert_eq!(read_frame.telegram().apci(), Apci::GroupValueRead);
        gateway.reply_tunnelling_ack(channel_id, 0).await.unwrap();
        gateway.shutdown().await.unwrap();
    });

    let mut client = TunnelClient::connect(gateway_addr).await.unwrap();
    let error = client
        .group_read(group, "1.001", Duration::from_millis(25))
        .await
        .unwrap_err();

    assert!(matches!(error, KnxIpError::Timeout));
    gateway_task.await.unwrap();
}

#[tokio::test]
async fn simulator_negative_cases_malformed_knxnetip_header_is_ignored_by_client() {
    let mut gateway = SimGateway::bind_localhost().await.unwrap();
    let gateway_addr = gateway.local_addr();
    let channel_id = 0x41;
    let group: GroupAddress = "1/2/3".parse().unwrap();

    let gateway_task = tokio::spawn(async move {
        gateway.expect_connect_request().await.unwrap();
        gateway.reply_connect_response(channel_id).await.unwrap();

        gateway.send_raw(&[0x01, 0x02, 0x03]).await.unwrap();

        let write_frame = gateway.expect_tunnelling_request(0).await.unwrap();
        assert_eq!(write_frame.telegram().apci(), Apci::GroupValueWrite);
        gateway.reply_tunnelling_ack(channel_id, 0).await.unwrap();
        gateway.shutdown().await.unwrap();
    });

    let mut client =
        TunnelClient::connect_with_ack_timeout(gateway_addr, Duration::from_millis(20))
            .await
            .unwrap();
    client
        .group_write(group, DptValue::Bool(true))
        .await
        .unwrap();

    gateway_task.await.unwrap();
}

#[tokio::test]
async fn simulator_negative_cases_wrong_channel_ack_times_out() {
    let mut gateway = SimGateway::bind_localhost().await.unwrap();
    let gateway_addr = gateway.local_addr();
    let channel_id = 0x42;
    let group: GroupAddress = "1/2/3".parse().unwrap();

    let gateway_task = tokio::spawn(async move {
        gateway.expect_connect_request().await.unwrap();
        gateway.reply_connect_response(channel_id).await.unwrap();

        gateway.expect_tunnelling_request(0).await.unwrap();
        gateway
            .reply_wrong_channel_ack(channel_id, 0)
            .await
            .unwrap();
        gateway.expect_timeout().await.unwrap();
        gateway.shutdown().await.unwrap();
    });

    let mut client =
        TunnelClient::connect_with_ack_timeout(gateway_addr, Duration::from_millis(20))
            .await
            .unwrap();
    let error = client
        .group_write(group, DptValue::Bool(true))
        .await
        .unwrap_err();
    assert!(matches!(error, KnxIpError::AckTimeout));

    gateway_task.await.unwrap();
}

#[tokio::test]
async fn simulator_negative_cases_wrong_sequence_ack_times_out() {
    let mut gateway = SimGateway::bind_localhost().await.unwrap();
    let gateway_addr = gateway.local_addr();
    let channel_id = 0x43;
    let group: GroupAddress = "1/2/3".parse().unwrap();

    let gateway_task = tokio::spawn(async move {
        gateway.expect_connect_request().await.unwrap();
        gateway.reply_connect_response(channel_id).await.unwrap();

        gateway.expect_tunnelling_request(0).await.unwrap();
        gateway
            .reply_wrong_sequence_ack(channel_id, 0)
            .await
            .unwrap();
        gateway.expect_timeout().await.unwrap();
        gateway.shutdown().await.unwrap();
    });

    let mut client =
        TunnelClient::connect_with_ack_timeout(gateway_addr, Duration::from_millis(20))
            .await
            .unwrap();
    let error = client
        .group_write(group, DptValue::Bool(true))
        .await
        .unwrap_err();
    assert!(matches!(error, KnxIpError::AckTimeout));

    gateway_task.await.unwrap();
}

#[tokio::test]
async fn simulator_negative_cases_delayed_ack_produces_typed_timeout() {
    let mut gateway = SimGateway::bind_localhost().await.unwrap();
    let gateway_addr = gateway.local_addr();
    let channel_id = 0x44;
    let group: GroupAddress = "1/2/3".parse().unwrap();

    let gateway_task = tokio::spawn(async move {
        gateway.expect_connect_request().await.unwrap();
        gateway.reply_connect_response(channel_id).await.unwrap();

        gateway.expect_tunnelling_request(0).await.unwrap();
        gateway.delay(Duration::from_millis(50)).await;
        gateway.reply_tunnelling_ack(channel_id, 0).await.unwrap();
        gateway.shutdown().await.unwrap();
    });

    let mut client =
        TunnelClient::connect_with_ack_timeout(gateway_addr, Duration::from_millis(20))
            .await
            .unwrap();
    let error = client
        .group_write(group, DptValue::Bool(true))
        .await
        .unwrap_err();
    assert!(matches!(error, KnxIpError::AckTimeout));

    gateway_task.await.unwrap();
}

#[tokio::test]
async fn simulator_negative_cases_malformed_cemi_indication_is_ignored() {
    let mut gateway = SimGateway::bind_localhost().await.unwrap();
    let gateway_addr = gateway.local_addr();
    let channel_id = 0x45;
    let group: GroupAddress = "1/2/3".parse().unwrap();

    let gateway_task = tokio::spawn(async move {
        gateway.expect_connect_request().await.unwrap();
        gateway.reply_connect_response(channel_id).await.unwrap();

        let read_frame = gateway.expect_tunnelling_request(0).await.unwrap();
        assert_eq!(read_frame.telegram().apci(), Apci::GroupValueRead);
        gateway.reply_tunnelling_ack(channel_id, 0).await.unwrap();

        gateway
            .send_malformed_tunnelling_indication(channel_id, 0, &[0xff, 0x00])
            .await
            .unwrap();
        gateway
            .send_tunnelling_indication(channel_id, 1, group, Apci::GroupValueResponse, &[0x01])
            .await
            .unwrap();
        gateway.shutdown().await.unwrap();
    });

    let mut client =
        TunnelClient::connect_with_ack_timeout(gateway_addr, Duration::from_millis(20))
            .await
            .unwrap();
    let read_value = client
        .group_read(group, "1.001", Duration::from_secs(1))
        .await
        .unwrap();
    assert_eq!(read_value, DptValue::Bool(true));

    gateway_task.await.unwrap();
}

#[tokio::test]
async fn tunnel_client_sends_group_value_response() {
    let mut gateway = SimGateway::bind_localhost().await.unwrap();
    let gateway_addr = gateway.local_addr();
    let channel_id = 0x41;
    let group: GroupAddress = "2/3/4".parse().unwrap();

    let gateway_task = tokio::spawn(async move {
        gateway.expect_connect_request().await.unwrap();
        gateway.reply_connect_response(channel_id).await.unwrap();

        let frame = gateway.expect_tunnelling_request(0).await.unwrap();
        assert_eq!(frame.telegram().destination(), group);
        assert_eq!(frame.telegram().apci(), Apci::GroupValueResponse);
        assert_eq!(frame.telegram().payload(), &[0x01]);
        gateway.reply_tunnelling_ack(channel_id, 0).await.unwrap();
        gateway.shutdown().await.unwrap();
    });

    let mut client = TunnelClient::connect(gateway_addr).await.unwrap();
    client
        .group_response(group, DptValue::Bool(true))
        .await
        .unwrap();

    gateway_task.await.unwrap();
}

#[tokio::test]
async fn tunnel_client_sends_fire_and_forget_read_request() {
    let mut gateway = SimGateway::bind_localhost().await.unwrap();
    let gateway_addr = gateway.local_addr();
    let channel_id = 0x51;
    let group: GroupAddress = "3/4/5".parse().unwrap();

    let gateway_task = tokio::spawn(async move {
        gateway.expect_connect_request().await.unwrap();
        gateway.reply_connect_response(channel_id).await.unwrap();

        let frame = gateway.expect_tunnelling_request(0).await.unwrap();
        assert_eq!(frame.telegram().destination(), group);
        assert_eq!(frame.telegram().apci(), Apci::GroupValueRead);
        assert!(frame.telegram().payload().is_empty());
        gateway.reply_tunnelling_ack(channel_id, 0).await.unwrap();
        gateway.shutdown().await.unwrap();
    });

    let mut client = TunnelClient::connect(gateway_addr).await.unwrap();
    client.group_read_request(group).await.unwrap();

    gateway_task.await.unwrap();
}

#[tokio::test]
async fn tunnel_client_disconnect_sends_disconnect_request() {
    let mut gateway = SimGateway::bind_localhost().await.unwrap();
    let gateway_addr = gateway.local_addr();
    let channel_id = 0x42;

    let gateway_task = tokio::spawn(async move {
        gateway.expect_connect_request().await.unwrap();
        gateway.reply_connect_response(channel_id).await.unwrap();

        let event = gateway.expect_disconnect_request(channel_id).await.unwrap();
        assert!(matches!(
            event,
            SimEvent::DisconnectRequest { channel_id: cid, .. } if cid == channel_id
        ));
        gateway.reply_disconnect_response(channel_id).await.unwrap();
        gateway.shutdown().await.unwrap();
    });

    let client = TunnelClient::connect(gateway_addr).await.unwrap();
    // Orderly disconnect: sends DISCONNECT_REQUEST and resolves on the
    // matching DISCONNECT_RESPONSE.
    client.disconnect().await.unwrap();

    gateway_task.await.unwrap();
}

#[tokio::test]
async fn tunnel_client_disconnect_times_out_when_gateway_is_silent() {
    let mut gateway = SimGateway::bind_localhost().await.unwrap();
    let gateway_addr = gateway.local_addr();
    let channel_id = 0x42;

    let gateway_task = tokio::spawn(async move {
        gateway.expect_connect_request().await.unwrap();
        gateway.reply_connect_response(channel_id).await.unwrap();

        // Receive the DISCONNECT_REQUEST but deliberately never answer it,
        // then hold the socket past the client's ACK timeout (1s default).
        let _ = gateway.expect_disconnect_request(channel_id).await.unwrap();
        gateway.delay(Duration::from_millis(1200)).await;
        gateway.shutdown().await.unwrap();
    });

    let client = TunnelClient::connect(gateway_addr).await.unwrap();
    // Timeout-safe: a silent gateway yields DisconnectTimeout, never a hang.
    let result = client.disconnect().await;
    assert!(matches!(result, Err(KnxIpError::DisconnectTimeout)));

    gateway_task.await.unwrap();
}
