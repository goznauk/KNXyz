//! DPT 29.xxx — KNX V64, 8-octet two's-complement signed integer, decode-only.
//!
//! 29.010 (active energy Wh), 29.011 (apparent energy VAh), 29.012 (reactive
//! energy VARh) all share one wire codec (the unit is carried by the DPT id),
//! decoding the 8 big-endian octets to `DptValue::I64`. Encode is intentionally
//! not provided — main 29 is absent from the uniform table (so
//! `encode("29.xxx", …)` stays `UnsupportedDpt`), and `I64` additionally
//! is rejected in `knx-ip`'s `encode_value`.
//!
//! These tests pin the decode vectors (incl. negatives, i64::MIN/MAX, and a
//! value beyond the JS 2^53 safe-integer range), the length contract, the
//! unsupported encode, and the unsupported neighbouring mains.

use knx_dpt::{decode, encode, DptError, DptValue};

fn i64_of(dpt: &str, bytes: &[u8]) -> i64 {
    match decode(dpt, bytes).unwrap() {
        DptValue::I64(value) => value,
        other => panic!("{dpt} expected I64, got {other:?}"),
    }
}

#[test]
fn dpt29_decodes_8_octet_twos_complement_big_endian() {
    // all three energy subs route to the same codec
    for dpt in ["29.010", "29.011", "29.012"] {
        assert_eq!(i64_of(dpt, &[0, 0, 0, 0, 0, 0, 0, 0]), 0, "{dpt} zero");
        assert_eq!(i64_of(dpt, &[0, 0, 0, 0, 0, 0, 0, 1]), 1, "{dpt} +1");
        // all-ones two's-complement -> -1
        assert_eq!(
            i64_of(dpt, &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]),
            -1,
            "{dpt} -1",
        );
        // big-endian ordering: low octet carries the small magnitude
        assert_eq!(
            i64_of(dpt, &[0, 0, 0, 0, 0, 0, 0x03, 0xE8]),
            1000,
            "{dpt} 1000"
        );
    }
}

#[test]
fn dpt29_decodes_distinct_byte_order_and_large_values() {
    // fully distinct ascending bytes catch a byte-order bug; this value
    // (~7.26e16) is also beyond 2^53, exercising the large-magnitude path.
    assert_eq!(
        i64_of("29.010", &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]),
        72_623_859_790_382_856,
    );
    // i64::MAX and i64::MIN both decode exactly (full domain, both beyond 2^53)
    assert_eq!(
        i64_of("29.010", &[0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]),
        i64::MAX,
    );
    assert_eq!(
        i64_of("29.010", &[0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
        i64::MIN,
    );
}

#[test]
fn dpt29_wrong_length_loud_fails() {
    // exactly 8 octets required; short and long payloads are InvalidLength
    // (never UnsupportedDpt — that is reserved for an unknown main/sub).
    for bytes in [&[][..], &[0u8; 7][..], &[0u8; 9][..]] {
        assert!(
            matches!(
                decode("29.010", bytes),
                Err(DptError::InvalidLength { expected: 8, .. })
            ),
            "29.010 with {} bytes must be InvalidLength{{expected:8}}",
            bytes.len()
        );
    }
}

#[test]
fn dpt29_is_decode_only_encode_unsupported() {
    // decode-only: encode returns an error (main 29 is not in the uniform table and
    // has no encode arm). The I64 variant is also refused by knx-ip encode_value
    // (covered by a knx-ip test), so a decoded value is never silently written.
    for dpt in ["29.010", "29.011", "29.012"] {
        assert_eq!(
            encode(dpt, DptValue::I64(1000)),
            Err(DptError::UnsupportedDpt(dpt.to_owned())),
            "{dpt} encode must stay UnsupportedDpt (decode-only)",
        );
    }
}

#[test]
fn dpt29_unsupported_neighbour_mains_stay_unsupported() {
    // mains 28 (UTF-8 string) and 30 stay unsupported in both directions; only
    // main 29 was added.
    for dpt in ["28.001", "30.001"] {
        assert_eq!(
            decode(dpt, &[0u8; 8]),
            Err(DptError::UnsupportedDpt(dpt.to_owned())),
            "{dpt} decode must be UnsupportedDpt",
        );
        assert_eq!(
            encode(dpt, DptValue::I64(1)),
            Err(DptError::UnsupportedDpt(dpt.to_owned())),
            "{dpt} encode must be UnsupportedDpt",
        );
    }
}
