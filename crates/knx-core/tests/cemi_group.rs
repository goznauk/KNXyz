#![cfg(feature = "std")]

use knx_core::{
    Apci, CemiFrame, CemiMessageCode, GroupAddress, GroupTelegram, IndividualAddress, KnxError,
};

#[test]
fn group_value_write_bool_frame_roundtrips() {
    let source: IndividualAddress = "1.1.4".parse().unwrap();
    let destination: GroupAddress = "1/2/3".parse().unwrap();
    let frame = CemiFrame::group_value_write(source, destination, &[0x01]).unwrap();

    let mut encoded = Vec::new();
    frame.encode(&mut encoded).unwrap();

    assert_eq!(
        encoded,
        [0x11, 0x00, 0xbc, 0xe0, 0x11, 0x04, 0x0a, 0x03, 0x01, 0x00, 0x81]
    );

    let (decoded, remaining) = CemiFrame::decode(&encoded).unwrap();
    assert!(remaining.is_empty());
    assert_eq!(decoded, frame);
    assert_eq!(decoded.telegram().apci(), Apci::GroupValueWrite);
    assert_eq!(decoded.telegram().payload(), &[0x01]);
}

#[test]
fn group_value_read_frame_roundtrips() {
    let source: IndividualAddress = "1.1.4".parse().unwrap();
    let destination: GroupAddress = "1/2/3".parse().unwrap();
    let frame = CemiFrame::group_value_read(source, destination).unwrap();

    let mut encoded = Vec::new();
    frame.encode(&mut encoded).unwrap();

    assert_eq!(
        encoded,
        [0x11, 0x00, 0xbc, 0xe0, 0x11, 0x04, 0x0a, 0x03, 0x01, 0x00, 0x00]
    );

    let (decoded, remaining) = CemiFrame::decode(&encoded).unwrap();
    assert!(remaining.is_empty());
    assert_eq!(decoded.telegram().apci(), Apci::GroupValueRead);
    assert!(decoded.telegram().payload().is_empty());
}

#[test]
fn group_value_response_bool_frame_roundtrips() {
    let source: IndividualAddress = "1.1.4".parse().unwrap();
    let destination: GroupAddress = "1/2/3".parse().unwrap();
    let frame = CemiFrame::group_value_response(source, destination, &[0x01]).unwrap();

    let mut encoded = Vec::new();
    frame.encode(&mut encoded).unwrap();

    assert_eq!(
        encoded,
        [0x11, 0x00, 0xbc, 0xe0, 0x11, 0x04, 0x0a, 0x03, 0x01, 0x00, 0x41]
    );

    let (decoded, remaining) = CemiFrame::decode(&encoded).unwrap();
    assert!(remaining.is_empty());
    assert_eq!(decoded.telegram().apci(), Apci::GroupValueResponse);
    assert_eq!(decoded.telegram().payload(), &[0x01]);
}

#[test]
fn group_value_write_multibyte_payload_roundtrips() {
    let source: IndividualAddress = "1.1.4".parse().unwrap();
    let destination: GroupAddress = "1/2/3".parse().unwrap();
    let telegram =
        GroupTelegram::new(source, destination, Apci::GroupValueWrite, &[0x12, 0x34]).unwrap();
    let frame = CemiFrame::new(CemiMessageCode::LDataRequest, telegram);

    let mut encoded = Vec::new();
    frame.encode(&mut encoded).unwrap();

    assert_eq!(
        encoded,
        [0x11, 0x00, 0xbc, 0xe0, 0x11, 0x04, 0x0a, 0x03, 0x03, 0x00, 0x80, 0x12, 0x34]
    );

    let (decoded, remaining) = CemiFrame::decode(&encoded).unwrap();
    assert!(remaining.is_empty());
    assert_eq!(decoded.telegram().payload(), &[0x12, 0x34]);
}

#[test]
fn malformed_cemi_frames_are_rejected() {
    assert_eq!(
        CemiFrame::decode(&[0xff, 0x00, 0xbc, 0xe0, 0x11, 0x04, 0x0a, 0x03, 0x01, 0x00, 0x81]),
        Err(KnxError::InvalidFrame("unsupported cEMI message code"))
    );
    assert_eq!(
        CemiFrame::decode(&[0x11, 0x00, 0xbc, 0xe0, 0x11]),
        Err(KnxError::BufferTooShort {
            needed: 9,
            actual: 5
        })
    );
    assert_eq!(
        CemiFrame::decode(&[0x11, 0x00, 0xbc, 0xe0, 0x11, 0x04, 0x0a, 0x03, 0x03, 0x00, 0x80]),
        Err(KnxError::BufferTooShort {
            needed: 13,
            actual: 11
        })
    );
}

fn src() -> IndividualAddress {
    "1.1.4".parse().unwrap()
}

fn dst() -> GroupAddress {
    "1/2/3".parse().unwrap()
}

fn encode(frame: &CemiFrame) -> Vec<u8> {
    let mut out = Vec::new();
    frame.encode(&mut out).unwrap();
    out
}

#[test]
fn write_short_payload_compacts_into_apci_byte() {
    // 0x3f fits in the low 6 bits, so it is folded into the APCI byte.
    let frame = CemiFrame::group_value_write(src(), dst(), &[0x3f]).unwrap();
    let encoded = encode(&frame);
    assert_eq!(&encoded[encoded.len() - 2..], &[0x00, 0xBF]);

    let (decoded, remaining) = CemiFrame::decode(&encoded).unwrap();
    assert!(remaining.is_empty());
    assert_eq!(decoded.telegram().payload(), &[0x3f]);
}

#[test]
fn write_payloads_above_threshold_use_three_byte_apdu() {
    for value in [0x40u8, 0xff] {
        let frame = CemiFrame::group_value_write(src(), dst(), &[value]).unwrap();
        let encoded = encode(&frame);
        assert_eq!(&encoded[encoded.len() - 3..], &[0x00, 0x80, value]);

        let (decoded, remaining) = CemiFrame::decode(&encoded).unwrap();
        assert!(remaining.is_empty());
        assert_eq!(decoded.telegram().payload(), &[value]);
    }
}

#[test]
fn response_short_payload_compacts_into_apci_byte() {
    let frame = CemiFrame::group_value_response(src(), dst(), &[0x3f]).unwrap();
    let encoded = encode(&frame);
    assert_eq!(&encoded[encoded.len() - 2..], &[0x00, 0x7F]);

    let (decoded, remaining) = CemiFrame::decode(&encoded).unwrap();
    assert!(remaining.is_empty());
    assert_eq!(decoded.telegram().payload(), &[0x3f]);
}

#[test]
fn response_payloads_above_threshold_use_three_byte_apdu() {
    for value in [0x40u8, 0xff] {
        let frame = CemiFrame::group_value_response(src(), dst(), &[value]).unwrap();
        let encoded = encode(&frame);
        assert_eq!(&encoded[encoded.len() - 3..], &[0x00, 0x40, value]);

        let (decoded, remaining) = CemiFrame::decode(&encoded).unwrap();
        assert!(remaining.is_empty());
        assert_eq!(decoded.telegram().payload(), &[value]);
    }
}

#[test]
fn empty_write_and_response_payloads_are_rejected() {
    // Payload rule: empty Write/Response payloads are ambiguous with the compact
    // one-byte zero on APDU decode (a 2-byte compact APDU always decodes back
    // to [0x00], never to empty), so they cannot round-trip distinctly and
    // are rejected at construction. [0x00] remains a valid compact payload.
    // `&'static str`; each assertion infers its own Result<_> from the LHS
    // (GroupTelegram::new -> Result<GroupTelegram>, the convenience
    // constructors -> Result<CemiFrame>), so the error is stated inline.
    let msg = "group write/response must carry payload";
    assert_eq!(
        GroupTelegram::new(src(), dst(), Apci::GroupValueWrite, &[]),
        Err(KnxError::InvalidFrame(msg))
    );
    assert_eq!(
        GroupTelegram::new(src(), dst(), Apci::GroupValueResponse, &[]),
        Err(KnxError::InvalidFrame(msg))
    );
    // The convenience constructors propagate the same rejection.
    assert_eq!(
        CemiFrame::group_value_write(src(), dst(), &[]),
        Err(KnxError::InvalidFrame(msg))
    );
    assert_eq!(
        CemiFrame::group_value_response(src(), dst(), &[]),
        Err(KnxError::InvalidFrame(msg))
    );
}

#[test]
fn read_empty_is_accepted_and_read_with_payload_is_rejected() {
    // GroupValueRead remains the only payload-less group operation.
    assert!(GroupTelegram::new(src(), dst(), Apci::GroupValueRead, &[]).is_ok());
    assert_eq!(
        GroupTelegram::new(src(), dst(), Apci::GroupValueRead, &[0x00]),
        Err(KnxError::InvalidFrame("group read cannot carry payload"))
    );
}

#[test]
fn compact_zero_write_and_response_still_round_trip() {
    // [0x00] is a valid one-byte value payload (not empty) and must still
    // encode (compact 2-byte APDU) and decode back to [0x00].
    for frame in [
        CemiFrame::group_value_write(src(), dst(), &[0x00]).unwrap(),
        CemiFrame::group_value_response(src(), dst(), &[0x00]).unwrap(),
    ] {
        let encoded = encode(&frame);
        let (decoded, remaining) = CemiFrame::decode(&encoded).unwrap();
        assert!(remaining.is_empty());
        assert_eq!(decoded.telegram().payload(), &[0x00]);
    }
}

#[test]
fn group_read_rejects_payload() {
    assert_eq!(
        GroupTelegram::new(src(), dst(), Apci::GroupValueRead, &[0x01]),
        Err(KnxError::InvalidFrame("group read cannot carry payload"))
    );
}

#[test]
fn payload_length_boundary_at_254_bytes() {
    let ok_payload = vec![0xABu8; 254];
    let telegram = GroupTelegram::new(src(), dst(), Apci::GroupValueWrite, &ok_payload).unwrap();
    let frame = CemiFrame::new(CemiMessageCode::LDataRequest, telegram);
    let encoded = encode(&frame);
    let (decoded, remaining) = CemiFrame::decode(&encoded).unwrap();
    assert!(remaining.is_empty());
    assert_eq!(decoded.telegram().payload(), ok_payload.as_slice());

    let too_long = vec![0xABu8; 255];
    assert_eq!(
        GroupTelegram::new(src(), dst(), Apci::GroupValueWrite, &too_long),
        Err(KnxError::InvalidFrame("group payload too long"))
    );
}

#[test]
fn decode_rejects_unsupported_tpci_field() {
    // Take a valid frame and corrupt the TPCI byte (first APDU byte).
    let frame = CemiFrame::group_value_write(src(), dst(), &[0x12, 0x34]).unwrap();
    let mut bytes = encode(&frame);
    // APDU starts at offset 9 (TPCI byte), set TPCI != 0x00.
    bytes[9] = 0x01;
    assert_eq!(
        CemiFrame::decode(&bytes),
        Err(KnxError::InvalidFrame("unsupported TPCI field"))
    );
}

#[test]
fn decode_rejects_extended_apdu_with_inline_data_bits() {
    // Multibyte APDU where the low-6 APCI bits are non-zero.
    let frame = CemiFrame::group_value_write(src(), dst(), &[0x12, 0x34]).unwrap();
    let mut bytes = encode(&frame);
    // APDU is [0x00, 0x80, 0x12, 0x34] at offset 9; set low-6 bits in apdu[1].
    bytes[10] = 0x81;
    assert_eq!(
        CemiFrame::decode(&bytes),
        Err(KnxError::InvalidFrame("extended APDU has inline data bits"))
    );
}

#[test]
fn decode_rejects_apdu_shorter_than_two_bytes() {
    // Hand-build a frame whose declared APDU length is 1 (apdu_len byte=0).
    // Layout: code, ail, c1, c2, src_hi, src_lo, dst_hi, dst_lo, npdu_len, apdu...
    let bytes = [0x11, 0x00, 0xbc, 0xe0, 0x11, 0x04, 0x0a, 0x03, 0x00, 0x00];
    assert_eq!(
        CemiFrame::decode(&bytes),
        Err(KnxError::BufferTooShort {
            needed: 2,
            actual: 1
        })
    );
}

fn splice_additional_info(original: &[u8], info: &[u8]) -> Vec<u8> {
    // Set byte[1] = info.len() and insert the info bytes right after it.
    let mut spliced = Vec::new();
    spliced.push(original[0]);
    spliced.push(info.len() as u8);
    spliced.extend_from_slice(info);
    spliced.extend_from_slice(&original[2..]);
    spliced
}

#[test]
fn constructor_frames_have_empty_additional_info() {
    let frame = CemiFrame::group_value_write(src(), dst(), &[0x01]).unwrap();
    assert!(frame.additional_info().is_empty());
    // And the empty-additional-info encoding is byte-identical to before
    // (additional-info length byte 0x00, no extra bytes).
    assert_eq!(
        encode(&frame),
        [0x11, 0x00, 0xbc, 0xe0, 0x11, 0x04, 0x0a, 0x03, 0x01, 0x00, 0x81]
    );
}

#[test]
fn decode_preserves_additional_info_and_round_trips() {
    let frame = CemiFrame::group_value_write(src(), dst(), &[0x12, 0x34]).unwrap();
    let original = encode(&frame);
    let info = [0xEEu8, 0xEE, 0xEE];
    let spliced = splice_additional_info(&original, &info);

    let (decoded, remaining) = CemiFrame::decode(&spliced).unwrap();
    assert!(remaining.is_empty());
    assert_eq!(decoded.telegram(), frame.telegram());
    assert_eq!(decoded.additional_info(), &info);
    // Re-encoding the decoded frame reproduces the spliced bytes exactly.
    assert_eq!(encode(&decoded), spliced);
}

#[test]
fn decode_additional_info_keeps_trailing_remainder() {
    let frame = CemiFrame::group_value_write(src(), dst(), &[0x01]).unwrap();
    let spliced = splice_additional_info(&encode(&frame), &[0x01, 0x02]);
    let mut bytes = spliced.clone();
    bytes.extend_from_slice(&[0xAA, 0xBB]);

    let (decoded, remaining) = CemiFrame::decode(&bytes).unwrap();
    assert_eq!(remaining, &[0xAA, 0xBB]);
    assert_eq!(decoded.additional_info(), &[0x01, 0x02]);
    assert_eq!(decoded.telegram().payload(), &[0x01]);
    assert_eq!(encode(&decoded), spliced);
}

#[test]
fn with_additional_info_emits_length_and_bytes_before_fixed_fields() {
    let frame = CemiFrame::group_value_write(src(), dst(), &[0x01])
        .unwrap()
        .with_additional_info(vec![0xAB, 0xCD])
        .unwrap();
    let encoded = encode(&frame);
    // msg, ail_len=2, [AB CD], control1, control2, src, dst, npdu, apdu
    assert_eq!(
        encoded,
        [0x11, 0x02, 0xAB, 0xCD, 0xbc, 0xe0, 0x11, 0x04, 0x0a, 0x03, 0x01, 0x00, 0x81]
    );

    let (decoded, remaining) = CemiFrame::decode(&encoded).unwrap();
    assert!(remaining.is_empty());
    assert_eq!(decoded.additional_info(), &[0xAB, 0xCD]);
    assert_eq!(decoded.telegram().payload(), &[0x01]);
    assert_eq!(encode(&decoded), encoded);
}

#[test]
fn with_additional_info_length_boundary() {
    let base = CemiFrame::group_value_write(src(), dst(), &[0x01]).unwrap();
    // 255 bytes fits the single-octet additional-info length.
    assert!(base.clone().with_additional_info(vec![0u8; 255]).is_ok());
    // 256 does not.
    assert_eq!(
        base.with_additional_info(vec![0u8; 256]),
        Err(KnxError::InvalidFrame("additional info too long"))
    );
}

#[test]
fn cemi_message_code_direct_mapping_roundtrips() {
    let table: [(CemiMessageCode, u8); 3] = [
        (CemiMessageCode::LDataRequest, 0x11),
        (CemiMessageCode::LDataIndication, 0x29),
        (CemiMessageCode::LDataConfirmation, 0x2e),
    ];

    for (code, byte) in table {
        assert_eq!(code.as_u8(), byte);
        assert_eq!(CemiMessageCode::try_from(byte), Ok(code));
    }
}

#[test]
fn cemi_message_code_unsupported_values_rejected() {
    for v in [0x00u8, 0x10, 0x12, 0x28, 0x2f, 0xff] {
        assert_eq!(
            CemiMessageCode::try_from(v),
            Err(KnxError::InvalidFrame("unsupported cEMI message code"))
        );
    }
}

#[test]
fn decode_returns_trailing_bytes_as_remainder() {
    let frame = CemiFrame::group_value_write(src(), dst(), &[0x01]).unwrap();
    let mut bytes = encode(&frame);
    bytes.extend_from_slice(&[0xAA, 0xBB]);

    let (decoded, remaining) = CemiFrame::decode(&bytes).unwrap();
    assert_eq!(remaining, &[0xAA, 0xBB]);
    assert_eq!(decoded, frame);
}
