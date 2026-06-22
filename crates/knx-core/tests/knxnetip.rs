use knx_core::{ConnectionHeader, HostProtocol, Hpai, KnxError, KnxNetIpHeader, ServiceType};
use proptest::prelude::*;

#[test]
#[cfg(feature = "std")]
fn self_authored_search_request_header_roundtrips_with_payload_length() {
    let header = KnxNetIpHeader::new(ServiceType::SearchRequest, 8).unwrap();
    let mut encoded = Vec::new();
    header.encode(&mut encoded).unwrap();

    assert_eq!(encoded, [0x06, 0x10, 0x02, 0x01, 0x00, 0x08]);

    let mut bytes = encoded.clone();
    bytes.extend_from_slice(&[0xaa, 0xbb]);
    let (decoded, remaining) = KnxNetIpHeader::decode(&bytes).unwrap();

    assert_eq!(decoded, header);
    assert_eq!(remaining, &[0xaa, 0xbb]);
}

#[test]
fn self_authored_service_type_values_roundtrip() {
    let services = [
        (ServiceType::SearchRequest, 0x0201),
        (ServiceType::SearchResponse, 0x0202),
        (ServiceType::DescriptionRequest, 0x0203),
        (ServiceType::DescriptionResponse, 0x0204),
        (ServiceType::ConnectRequest, 0x0205),
        (ServiceType::ConnectResponse, 0x0206),
        (ServiceType::ConnectionStateRequest, 0x0207),
        (ServiceType::ConnectionStateResponse, 0x0208),
        (ServiceType::DisconnectRequest, 0x0209),
        (ServiceType::DisconnectResponse, 0x020a),
        (ServiceType::TunnellingRequest, 0x0420),
        (ServiceType::TunnellingAck, 0x0421),
        (ServiceType::RoutingIndication, 0x0530),
        (ServiceType::RoutingLostMessage, 0x0531),
        (ServiceType::RoutingBusy, 0x0532),
    ];

    for (service, value) in services {
        assert_eq!(service.as_u16(), value);
        assert_eq!(ServiceType::try_from(value).unwrap(), service);
    }
}

#[test]
fn invalid_header_fields_are_rejected_without_panics() {
    assert_eq!(
        KnxNetIpHeader::decode(&[0x05, 0x10, 0x02, 0x01, 0x00, 0x06]),
        Err(KnxError::InvalidFrame("invalid KNXnet/IP header length"))
    );
    assert_eq!(
        KnxNetIpHeader::decode(&[0x06, 0x11, 0x02, 0x01, 0x00, 0x06]),
        Err(KnxError::InvalidFrame("invalid KNXnet/IP protocol version"))
    );
    assert_eq!(
        KnxNetIpHeader::decode(&[0x06, 0x10, 0x02, 0x01, 0x00]),
        Err(KnxError::BufferTooShort {
            needed: 6,
            actual: 5
        })
    );
    assert_eq!(
        KnxNetIpHeader::decode(&[0x06, 0x10, 0x02, 0x01, 0x00, 0x05]),
        Err(KnxError::InvalidFrame("total length shorter than header"))
    );
    assert_eq!(
        ServiceType::try_from(0xffff),
        Err(KnxError::UnsupportedServiceType(0xffff))
    );
}

#[test]
#[cfg(feature = "std")]
fn hpai_ipv4_udp_roundtrips() {
    let hpai = Hpai::new(HostProtocol::Ipv4Udp, [192, 168, 1, 10], 3671);
    let mut encoded = Vec::new();
    hpai.encode(&mut encoded).unwrap();

    assert_eq!(encoded, [0x08, 0x01, 192, 168, 1, 10, 0x0e, 0x57]);
    assert_eq!(Hpai::decode(&encoded), Ok((hpai, [].as_slice())));
}

#[test]
#[cfg(feature = "std")]
fn connection_header_roundtrips() {
    let header = ConnectionHeader::new(0x21, 0x7f, 0x00);
    let mut encoded = Vec::new();
    header.encode(&mut encoded).unwrap();

    assert_eq!(encoded, [0x04, 0x21, 0x7f, 0x00]);
    assert_eq!(
        ConnectionHeader::decode(&encoded),
        Ok((header, [].as_slice()))
    );
}

#[test]
#[cfg(feature = "std")]
fn hpai_ipv4_tcp_roundtrips() {
    let hpai = Hpai::new(HostProtocol::Ipv4Tcp, [10, 0, 0, 1], 3671);
    let mut encoded = Vec::new();
    hpai.encode(&mut encoded).unwrap();

    assert_eq!(encoded, [0x08, 0x02, 10, 0, 0, 1, 0x0e, 0x57]);
    let (decoded, remaining) = Hpai::decode(&encoded).unwrap();
    assert_eq!(decoded, hpai);
    assert!(remaining.is_empty());
}

#[test]
fn hpai_decode_negatives() {
    assert_eq!(
        Hpai::decode(&[0x08, 0x01, 192, 168]),
        Err(KnxError::BufferTooShort {
            needed: 8,
            actual: 4
        })
    );
    assert_eq!(
        Hpai::decode(&[0x07, 0x01, 192, 168, 1, 10, 0x0e, 0x57]),
        Err(KnxError::InvalidFrame("invalid HPAI length"))
    );
    assert_eq!(
        Hpai::decode(&[0x08, 0x03, 192, 168, 1, 10, 0x0e, 0x57]),
        Err(KnxError::InvalidFrame("unsupported host protocol"))
    );
}

#[test]
fn connection_header_decode_negatives() {
    assert_eq!(
        ConnectionHeader::decode(&[0x04, 0x21, 0x7f]),
        Err(KnxError::BufferTooShort {
            needed: 4,
            actual: 3
        })
    );
    assert_eq!(
        ConnectionHeader::decode(&[0x05, 0x21, 0x7f, 0x00]),
        Err(KnxError::InvalidFrame("invalid connection header length"))
    );
}

#[test]
fn host_protocol_direct_mapping_roundtrips() {
    let table: [(HostProtocol, u8); 2] =
        [(HostProtocol::Ipv4Udp, 0x01), (HostProtocol::Ipv4Tcp, 0x02)];

    for (protocol, byte) in table {
        assert_eq!(protocol.as_u8(), byte);
        assert_eq!(HostProtocol::try_from(byte), Ok(protocol));
    }
}

#[test]
fn host_protocol_unsupported_values_rejected() {
    for v in [0x00u8, 0x03, 0x04, 0x7f, 0xff] {
        assert_eq!(
            HostProtocol::try_from(v),
            Err(KnxError::InvalidFrame("unsupported host protocol"))
        );
    }
}

#[test]
fn service_type_additional_unsupported_sentinels() {
    for v in [0x0000u16, 0x0200, 0x0210, 0x0422, 0x0533, 0x0a03] {
        assert_eq!(
            ServiceType::try_from(v),
            Err(KnxError::UnsupportedServiceType(v))
        );
    }
}

#[test]
fn service_type_all_is_exhaustive() {
    assert_eq!(ServiceType::ALL.len(), 15);

    // Exhaustive match so adding a variant without updating `ALL` fails to
    // compile (forcing the new variant to be added to the single source).
    let st = ServiceType::SearchRequest;
    match st {
        ServiceType::SearchRequest
        | ServiceType::SearchResponse
        | ServiceType::DescriptionRequest
        | ServiceType::DescriptionResponse
        | ServiceType::ConnectRequest
        | ServiceType::ConnectResponse
        | ServiceType::ConnectionStateRequest
        | ServiceType::ConnectionStateResponse
        | ServiceType::DisconnectRequest
        | ServiceType::DisconnectResponse
        | ServiceType::TunnellingRequest
        | ServiceType::TunnellingAck
        | ServiceType::RoutingIndication
        | ServiceType::RoutingLostMessage
        | ServiceType::RoutingBusy => {}
    }
}

proptest! {
    #[test]
    #[cfg(feature = "std")]
    fn header_roundtrips_for_supported_service_types(
        service in proptest::sample::select(ServiceType::ALL.as_slice()),
        payload in proptest::collection::vec(any::<u8>(), 0..32),
    ) {
        let total_length = 6 + payload.len() as u16;
        let header = KnxNetIpHeader::new(service, total_length).unwrap();
        let mut encoded = Vec::new();
        header.encode(&mut encoded).unwrap();
        encoded.extend_from_slice(&payload);

        let (decoded, remaining) = KnxNetIpHeader::decode(&encoded).unwrap();
        prop_assert_eq!(decoded, header);
        prop_assert_eq!(remaining, payload.as_slice());
    }

    #[test]
    #[cfg(feature = "std")]
    fn hpai_roundtrips_for_ipv4_udp(address in any::<[u8; 4]>(), port in any::<u16>()) {
        let hpai = Hpai::new(HostProtocol::Ipv4Udp, address, port);
        let mut encoded = Vec::new();
        hpai.encode(&mut encoded).unwrap();

        let (decoded, remaining) = Hpai::decode(&encoded).unwrap();
        prop_assert_eq!(decoded, hpai);
        prop_assert!(remaining.is_empty());
    }
}
