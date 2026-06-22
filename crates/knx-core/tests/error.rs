use knx_core::KnxError;

#[test]
fn error_display_messages_are_stable() {
    let cases = [
        (
            KnxError::InvalidAddress("group main out of range"),
            "invalid address: group main out of range",
        ),
        (
            KnxError::BufferTooShort {
                needed: 6,
                actual: 5,
            },
            "buffer too short: needed 6 bytes, got 5",
        ),
        (
            KnxError::InvalidFrame("invalid KNXnet/IP header length"),
            "invalid frame: invalid KNXnet/IP header length",
        ),
        (
            KnxError::UnsupportedServiceType(0x0200),
            "unsupported service type: 0x0200",
        ),
    ];

    for (error, expected) in cases {
        assert_eq!(error.to_string(), expected);
    }
}

#[test]
fn result_alias_uses_knx_error() {
    let result: knx_core::Result<()> = Err(KnxError::InvalidFrame("demo"));

    assert_eq!(result, Err(KnxError::InvalidFrame("demo")));
}
