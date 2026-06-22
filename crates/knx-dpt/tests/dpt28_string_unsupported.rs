//! DPT 28.001 UTF-8 string values are not implemented yet.
//!
//! DPT 28 uses variable-length UTF-8 text. The codec currently has no explicit
//! value type or termination policy for this id, so it rejects `28.001` before
//! attempting UTF-8 validation. This avoids treating a guessed length or trailing
//! NUL rule as part of the public API.
//!
//! These tests verify the current unsupported status in both directions,
//! regardless of payload validity.

use knx_dpt::{decode, encode, DptError, DptValue};

#[test]
fn dpt28_001_decode_is_unsupported_regardless_of_payload() {
    // Every payload shape - empty, valid ASCII, valid multi-byte UTF-8, and
    // invalid UTF-8 — must be rejected at DISPATCH (main 28 has no codec), NOT
    // partially decoded. This proves no accidental DPT28 codec exists and that
    // the rejection happens before any UTF-8 validation step.
    let payloads: [&[u8]; 4] = [
        &[],           // empty
        &[0x41],       // "A"
        &[0xC3, 0xA9], // U+00E9 (e-acute), valid UTF-8
        &[0xFF],       // invalid UTF-8 (lone 0xFF)
    ];
    for bytes in payloads {
        assert_eq!(
            decode("28.001", bytes),
            Err(DptError::UnsupportedDpt("28.001".to_owned())),
            "28.001 decode must stay UnsupportedDpt while the wire format is unconfirmed",
        );
    }
}

#[test]
fn dpt28_001_encode_is_unsupported() {
    // encode stays refused too: main 28 is absent from the uniform codec table
    // and has no explicit arm, so a string value cannot be written as 28.001.
    // (Text14 is used only as a stand-in String-carrying value; there is no
    // Utf8String variant yet.)
    assert_eq!(
        encode("28.001", DptValue::Text14("hello".to_owned())),
        Err(DptError::UnsupportedDpt("28.001".to_owned())),
    );
}

#[test]
fn dpt28_neighbour_subs_stay_unsupported() {
    // dispatch is by MAIN, so guard that no neighbouring 28.xxx sub was made
    // supported by accident.
    for dpt in ["28.000", "28.002"] {
        assert_eq!(
            decode(dpt, &[0x41]),
            Err(DptError::UnsupportedDpt(dpt.to_owned())),
            "{dpt} must be UnsupportedDpt",
        );
    }
}

#[test]
fn dpt29_v64_decode_still_works() {
    // regression: the shipped adjacent decode-only codec (DPT29 -> I64) is
    // unaffected by the unsupported DPT28 id.
    assert_eq!(
        decode("29.010", &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01]).unwrap(),
        DptValue::I64(1),
    );
}
