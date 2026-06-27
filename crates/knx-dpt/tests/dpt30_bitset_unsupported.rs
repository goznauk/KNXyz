//! DPT 30 raw bit-set values are not implemented yet.
//!
//! The codec currently has no confirmed DPT30 payload contract: width, byte
//! order, and whether the bits form an unstructured raw mask are all unspecified
//! in this implementation. Until those details are represented explicitly, main
//! 30 values return `UnsupportedDpt` instead of assuming a 4-octet layout.
//!
//! These tests verify the current unsupported status in both directions,
//! regardless of payload width. The 4-octet payload is included because it is the
//! shape a raw 32-bit mask codec would otherwise accept.

use knx_dpt::{decode, encode, DptError, DptValue};

#[test]
fn dpt30_decode_is_unsupported_regardless_of_payload_width() {
    // Every payload width - including the 4 octets a B32 codec would consume -
    // must be rejected at dispatch (main 30 has no codec), not decoded. The
    // 4-octet case is the load-bearing pin: it proves no accidental Bitset32
    // codec exists.
    let payloads: [&[u8]; 6] = [
        &[],                             // empty
        &[0x00],                         // 1 octet
        &[0x00, 0x00],                   // 2 octets
        &[0x00, 0x00, 0x00],             // 3 octets
        &[0xFF, 0xFF, 0xFF, 0xFF],       // 4 octets: the B32 width an implementation would use
        &[0x00, 0x00, 0x00, 0x00, 0x00], // 5 octets
    ];
    for bytes in payloads {
        assert_eq!(
            decode("30.001", bytes),
            Err(DptError::UnsupportedDpt("30.001".to_owned())),
            "30.001 decode must stay UnsupportedDpt while the wire format is unconfirmed \
             (payload was {} bytes)",
            bytes.len(),
        );
    }
}

#[test]
fn dpt30_encode_is_unsupported() {
    // encode stays refused too: main 30 is absent from the uniform codec table
    // and has no explicit arm, so a raw bit-set value cannot be written as 30.xxx.
    // (Bitset16 is used only as a stand-in raw-mask-carrying value; there is no
    // Bitset32 variant yet.)
    assert_eq!(
        encode("30.001", DptValue::Bitset16(0xFFFF)),
        Err(DptError::UnsupportedDpt("30.001".to_owned())),
    );
}

#[test]
fn dpt30_neighbour_subs_stay_unsupported() {
    // dispatch is by MAIN, so guard that no neighbouring 30.xxx sub was made
    // supported by accident.
    for dpt in ["30.000", "30.100"] {
        assert_eq!(
            decode(dpt, &[0xFF, 0xFF, 0xFF, 0xFF]),
            Err(DptError::UnsupportedDpt(dpt.to_owned())),
            "{dpt} must be UnsupportedDpt",
        );
    }
}

#[test]
fn dpt30_bitset_family_neighbours_stay_unsupported() {
    // the other structured / bit-set-adjacent mains the repo deliberately leaves
    // unsupported (23/26/27) must stay refused in both directions — only 21/22
    // were ever added.
    for dpt in ["23.001", "26.001", "27.001"] {
        assert_eq!(
            decode(dpt, &[0xFF, 0xFF, 0xFF, 0xFF]),
            Err(DptError::UnsupportedDpt(dpt.to_owned())),
            "{dpt} decode must be UnsupportedDpt",
        );
        assert_eq!(
            encode(dpt, DptValue::Bitset16(1)),
            Err(DptError::UnsupportedDpt(dpt.to_owned())),
            "{dpt} encode must be UnsupportedDpt",
        );
    }
}

#[test]
fn dpt22_bitset16_decode_still_works() {
    // regression: the shipped sibling raw-bit-set codec (DPT22 -> Bitset16) is
    // unaffected by the unsupported DPT30 id.
    assert_eq!(
        decode("22.101", &[0xA5, 0x5A]).unwrap(),
        DptValue::Bitset16(0xA55A),
    );
}
