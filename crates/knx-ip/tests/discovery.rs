use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::Duration;

use knx_ip::{discover_gateways, DiscoveryOptions, ServiceFamily};
use knx_sim::SimGateway;

#[tokio::test]
async fn discovery_uses_sim_gateway() {
    let mut gateway = SimGateway::bind_localhost().await.unwrap();
    let gateway_addr = gateway.local_addr();

    let gateway_task = tokio::spawn(async move {
        gateway.expect_search_request().await.unwrap();
        gateway
            .reply_search_response(
                SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 3671)),
                &[(0x02, 1), (0x04, 1)],
            )
            .await
            .unwrap();
        gateway.shutdown().await.unwrap();
    });

    let gateways = discover_gateways(DiscoveryOptions {
        bind: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)),
        target: gateway_addr,
        timeout: Duration::from_millis(500),
    })
    .await
    .unwrap();

    gateway_task.await.unwrap();

    assert_eq!(gateways.len(), 1);
    assert_eq!(
        gateways[0].control_endpoint,
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 3671))
    );
    assert_eq!(
        gateways[0].service_families,
        vec![
            ServiceFamily {
                id: 0x02,
                version: 1
            },
            ServiceFamily {
                id: 0x04,
                version: 1
            },
        ]
    );
}

#[tokio::test]
async fn discovery_returns_empty_list_on_timeout() {
    let gateway = SimGateway::bind_localhost().await.unwrap();

    let gateways = discover_gateways(DiscoveryOptions {
        bind: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)),
        target: gateway.local_addr(),
        timeout: Duration::from_millis(20),
    })
    .await
    .unwrap();

    assert!(gateways.is_empty());
    gateway.shutdown().await.unwrap();
}
