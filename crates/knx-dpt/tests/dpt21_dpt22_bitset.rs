//! DPT 21.xxx / 22.xxx — KNX B8 / B16 raw bit sets, decode-only.
//!
//! DPT21 is a fixed 1-octet "8-bit set" and DPT22 a fixed 2-octet (big-endian)
//! "16-bit set". The wire width is fixed by the main number, so the raw mask is
//! decoded as a raw mask. Per-bit meaning is a separate semantic layer outside
//! this codec. Decode is therefore sub-agnostic (every sub of a main shares the
//! mask), routing to `DptValue::Bitset8` / `DptValue::Bitset16`.
//!
//! Encode is intentionally not provided: mains 21/22 are absent from the
//! uniform table (so `encode("21.xxx"/"22.xxx", …)` stays `UnsupportedDpt`), and
//! the `Bitset8`/`Bitset16` variants are also rejected in `knx-ip`'s
//! `encode_value`. Reusing the writable `U8` [→ "5.010"] or `U16` [→ "7.001"]
//! variants would make decoded bit masks writable through the wrong DPT.
//!
//! These tests pin the decode vectors (incl. byte-order on DPT22), the length
//! contract, the unsupported encode, and the unsupported neighbouring mains.

use knx_dpt::{decode, encode, DptError, DptValue};

fn bitset8_of(dpt: &str, bytes: &[u8]) -> u8 {
    match decode(dpt, bytes).unwrap() {
        DptValue::Bitset8(value) => value,
        other => panic!("{dpt} expected Bitset8, got {other:?}"),
    }
}

fn bitset16_of(dpt: &str, bytes: &[u8]) -> u16 {
    match decode(dpt, bytes).unwrap() {
        DptValue::Bitset16(value) => value,
        other => panic!("{dpt} expected Bitset16, got {other:?}"),
    }
}

#[test]
fn dpt21_decodes_one_octet_raw_mask() {
    // sub-agnostic: every sub of main 21 routes to the same B8 codec.
    for dpt in ["21.001", "21.100", "21.105"] {
        assert_eq!(bitset8_of(dpt, &[0x00]), 0x00, "{dpt} all clear");
        assert_eq!(bitset8_of(dpt, &[0xFF]), 0xFF, "{dpt} all set");
        // raw mask is preserved bit-for-bit (no interpretation, no masking off)
        assert_eq!(bitset8_of(dpt, &[0b1010_0101]), 0xA5, "{dpt} pattern");
        assert_eq!(bitset8_of(dpt, &[0x01]), 0x01, "{dpt} low bit");
        assert_eq!(bitset8_of(dpt, &[0x80]), 0x80, "{dpt} high bit");
    }
}

#[test]
fn dpt22_decodes_two_octet_big_endian_raw_mask() {
    // sub-agnostic: every sub of main 22 routes to the same B16 codec.
    for dpt in ["22.100", "22.101", "22.1000"] {
        assert_eq!(bitset16_of(dpt, &[0x00, 0x00]), 0x0000, "{dpt} all clear");
        assert_eq!(bitset16_of(dpt, &[0xFF, 0xFF]), 0xFFFF, "{dpt} all set");
        // big-endian: first octet is the high byte (distinct bytes catch a
        // byte-order bug).
        assert_eq!(bitset16_of(dpt, &[0x01, 0x02]), 0x0102, "{dpt} 0x0102");
        assert_eq!(bitset16_of(dpt, &[0xA5, 0x5A]), 0xA55A, "{dpt} 0xA55A");
        assert_eq!(bitset16_of(dpt, &[0x80, 0x00]), 0x8000, "{dpt} high bit");
        assert_eq!(bitset16_of(dpt, &[0x00, 0x01]), 0x0001, "{dpt} low bit");
    }
}

#[test]
fn dpt21_dpt22_wrong_length_loud_fails() {
    // exactly 1 octet for DPT21, exactly 2 for DPT22; short/empty/long payloads
    // are InvalidLength (never UnsupportedDpt — that is reserved for an unknown
    // main/sub).
    for bytes in [&[][..], &[0u8; 2][..], &[0u8; 3][..]] {
        assert!(
            matches!(
                decode("21.001", bytes),
                Err(DptError::InvalidLength { expected: 1, .. })
            ),
            "21.001 with {} bytes must be InvalidLength{{expected:1}}",
            bytes.len()
        );
    }
    for bytes in [&[][..], &[0u8; 1][..], &[0u8; 3][..]] {
        assert!(
            matches!(
                decode("22.101", bytes),
                Err(DptError::InvalidLength { expected: 2, .. })
            ),
            "22.101 with {} bytes must be InvalidLength{{expected:2}}",
            bytes.len()
        );
    }
}

#[test]
fn dpt21_dpt22_are_decode_only_encode_unsupported() {
    // decode-only: keyed encode returns an error because mains 21/22 are not in
    // the uniform table and have no encode arm. The Bitset8/Bitset16 variants are also
    // refused by knx-ip encode_value (covered by a knx-ip test), so a decoded
    // value is never silently written.
    for dpt in ["21.001", "21.100"] {
        assert_eq!(
            encode(dpt, DptValue::Bitset8(0xFF)),
            Err(DptError::UnsupportedDpt(dpt.to_owned())),
            "{dpt} encode must stay UnsupportedDpt (decode-only)",
        );
    }
    for dpt in ["22.100", "22.101"] {
        assert_eq!(
            encode(dpt, DptValue::Bitset16(0xFFFF)),
            Err(DptError::UnsupportedDpt(dpt.to_owned())),
            "{dpt} encode must stay UnsupportedDpt (decode-only)",
        );
    }
}

#[test]
fn dpt21_dpt22_unsupported_neighbour_mains_stay_unsupported() {
    // mains 23/26/27/30 stay unsupported in both directions; only mains 21/22
    // were added (24 is a variable-length string, also unsupported).
    for dpt in ["23.001", "26.001", "27.001", "30.001"] {
        assert_eq!(
            decode(dpt, &[0u8; 2]),
            Err(DptError::UnsupportedDpt(dpt.to_owned())),
            "{dpt} decode must be UnsupportedDpt",
        );
        assert_eq!(
            encode(dpt, DptValue::Bitset8(1)),
            Err(DptError::UnsupportedDpt(dpt.to_owned())),
            "{dpt} encode must be UnsupportedDpt",
        );
    }
}
