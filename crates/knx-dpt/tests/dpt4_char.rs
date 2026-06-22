//! DPT 4.xxx — KNX single character, DECODE-ONLY.
//!
//! 4.001 (ASCII, 7-bit) and 4.002 (ISO-8859-1 / Latin-1) decode their single
//! octet to `DptValue::Char` (the character set is carried by the DPT id).
//! Encode is intentionally not provided: main 4 is absent from the uniform
//! table (so `encode("4.xxx", ...)` stays `UnsupportedDpt`), and `Char`
//! is also rejected by `knx-ip`'s `encode_value` write inference. A bare `U8`
//! would lose the DPT4 character semantics and could be inferred as DPT 5.010,
//! so the dedicated `Char` variant range-checks 4.001 and stays write-isolated.

use knx_dpt::{decode, encode, DptError, DptValue};

fn char_of(dpt: &str, bytes: &[u8]) -> char {
    match decode(dpt, bytes).unwrap() {
        DptValue::Char(c) => c,
        other => panic!("{dpt} expected Char, got {other:?}"),
    }
}

#[test]
fn dpt4_001_ascii_decodes_in_range() {
    assert_eq!(char_of("4.001", &[0x41]), 'A'); // printable
    assert_eq!(char_of("4.001", &[0x00]), '\u{0000}'); // NUL (control chars OK)
    assert_eq!(char_of("4.001", &[0x7F]), '\u{007F}'); // DEL = inclusive max
}

#[test]
fn dpt4_001_ascii_rejects_high_bit() {
    // bytes above 0x7F are not valid 7-bit ASCII -> InvalidValue (the
    // length is correct, so NOT InvalidLength; the DPT is supported, so NOT
    // UnsupportedDpt). This is the honesty gap a U8 reuse could not close.
    for byte in [0x80u8, 0xA0, 0xFF] {
        assert!(
            matches!(
                decode("4.001", &[byte]),
                Err(DptError::InvalidValue { dpt: "4.001", .. })
            ),
            "4.001 byte {byte:#04x} must be InvalidValue",
        );
    }
}

#[test]
fn dpt4_002_latin1_decodes_every_byte() {
    assert_eq!(char_of("4.002", &[0x41]), 'A');
    assert_eq!(char_of("4.002", &[0xE4]), 'ä'); // U+00E4
    assert_eq!(char_of("4.002", &[0xFF]), 'ÿ'); // U+00FF, max Latin-1
                                                // the discriminating sub-aware byte: 0x80 is REJECTED by 4.001 but ACCEPTED
                                                // by 4.002 (a valid Latin-1 C1 control, U+0080).
    assert_eq!(char_of("4.002", &[0x80]), '\u{0080}');
}

#[test]
fn dpt4_wrong_length_returns_invalid_length() {
    for dpt in ["4.001", "4.002"] {
        for bytes in [&[][..], &[0x41, 0x42][..]] {
            assert!(
                matches!(
                    decode(dpt, bytes),
                    Err(DptError::InvalidLength { expected: 1, .. })
                ),
                "{dpt} with {} bytes must be InvalidLength{{expected:1}}",
                bytes.len()
            );
        }
    }
}

#[test]
fn dpt4_is_decode_only_encode_unsupported() {
    for dpt in ["4.001", "4.002"] {
        assert_eq!(
            encode(dpt, DptValue::Char('A')),
            Err(DptError::UnsupportedDpt(dpt.to_owned())),
            "{dpt} encode must stay UnsupportedDpt (decode-only)",
        );
    }
}

#[test]
fn dpt4_undefined_subs_stay_unsupported() {
    // only 4.001 and 4.002 are defined; any other 4.xxx stays UnsupportedDpt
    // (the decode arms are sub-aware, not a sub-agnostic accept-all).
    for dpt in ["4.003", "4.000"] {
        assert_eq!(
            decode(dpt, &[0x41]),
            Err(DptError::UnsupportedDpt(dpt.to_owned())),
            "{dpt} must be UnsupportedDpt",
        );
    }
}

#[test]
fn dpt5_raw_u8_passthrough_still_works() {
    // regression: the unrelated DPT5 raw-U8 passthrough (5.010) is unaffected.
    assert_eq!(decode("5.010", &[0x41]), Ok(DptValue::U8(0x41)));
}
