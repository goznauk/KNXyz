use knx_sim::SimGateway;

#[tokio::test]
async fn lifecycle_starts_reports_local_addr_and_shuts_down() {
    let gateway = SimGateway::bind_localhost().await.unwrap();

    assert!(gateway.local_addr().port() > 0);
    assert_eq!(gateway.handle().local_addr(), gateway.local_addr());

    gateway.shutdown().await.unwrap();
}
