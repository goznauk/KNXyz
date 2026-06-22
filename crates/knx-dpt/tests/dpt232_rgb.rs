//! DPT 232.600 — KNX 3-octet RGB colour, symmetric pure codec.
//!
//! The bus form (3 octets R, G, B; each 0..=255) round-trips with
//! `DptValue::Rgb { red, green, blue }`. The encode side is the OFFLINE pure
//! codec (`knx_dpt::encode`) only — it does NOT actuate the bus; live colour
//! writes stay refused at `knx-ip`'s `encode_value` (pinned in that crate's
//! tests). These tests pin the decode field order, the boundary bytes, the
//! length contract, the symmetric encode + round-trip, and the wrong-variant
//! encode rejection.

use knx_dpt::{decode, encode, DptError, DptValue};

fn rgb(bytes: &[u8]) -> (u8, u8, u8) {
    match decode("232.600", bytes).unwrap() {
        DptValue::Rgb { red, green, blue } => (red, green, blue),
        other => panic!("232.600 expected Rgb, got {other:?}"),
    }
}

#[test]
fn dpt232_600_decodes_three_octets_in_rgb_order() {
    // distinct R/G/B values confirm the byte order is R, G, B (not reversed).
    assert_eq!(rgb(&[0x0A, 0x14, 0x1E]), (10, 20, 30));
    // boundary bytes: all-min and all-max are both valid (no range check).
    assert_eq!(rgb(&[0x00, 0x00, 0x00]), (0, 0, 0));
    assert_eq!(rgb(&[0xFF, 0xFF, 0xFF]), (255, 255, 255));
}

#[test]
fn dpt232_600_encodes_three_octets_in_rgb_order() {
    // encode is the matching 3-octet identity (R, G, B). This is the OFFLINE
    // pure codec — it does not write to the bus (colour actuation stays refused
    // in knx-ip's encode_value, pinned there).
    assert_eq!(
        encode(
            "232.600",
            DptValue::Rgb {
                red: 1,
                green: 2,
                blue: 3,
            },
        ),
        Ok(std::vec![1, 2, 3]),
    );
    // distinct + boundary values
    assert_eq!(
        encode(
            "232.600",
            DptValue::Rgb {
                red: 10,
                green: 20,
                blue: 30,
            },
        ),
        Ok(std::vec![0x0A, 0x14, 0x1E]),
    );
}

#[test]
fn dpt232_600_round_trips_encode_decode() {
    for (r, g, b) in [(10u8, 20u8, 30u8), (0, 0, 0), (255, 255, 255), (1, 2, 3)] {
        let bytes = encode(
            "232.600",
            DptValue::Rgb {
                red: r,
                green: g,
                blue: b,
            },
        )
        .unwrap();
        assert_eq!(bytes, std::vec![r, g, b]);
        assert_eq!(rgb(&bytes), (r, g, b));
    }
}

#[test]
fn dpt232_600_encode_rejects_wrong_variant() {
    // a non-Rgb variant loud-fails TypeMismatch (the id parses, so it is NOT
    // UnsupportedDpt). RGBW is not RGB.
    assert_eq!(
        encode(
            "232.600",
            DptValue::Rgbw {
                red: 1,
                green: 2,
                blue: 3,
                white: 4,
            },
        ),
        Err(DptError::TypeMismatch { dpt: "232.600" }),
    );
    assert_eq!(
        encode("232.600", DptValue::U8(7)),
        Err(DptError::TypeMismatch { dpt: "232.600" }),
    );
}

#[test]
fn dpt232_codec_is_main_keyed_sub_agnostic() {
    // The dpt232 codec is keyed by main 232, not by the sub: `dpt232.rs` has no
    // sub guard and `lib.rs` dispatches `232 =>` for any sub. 232.600 is the only
    // assigned sub, so a non-600 `232.xxx` also round-trips and the wrong-variant
    // TypeMismatch tag stays the fixed literal "232.600". It does not affect the
    // live-write refusal, which is variant-keyed in knx-ip's `encode_value`.
    assert_eq!(
        encode(
            "232.500",
            DptValue::Rgb {
                red: 1,
                green: 2,
                blue: 3,
            },
        ),
        Ok(std::vec![1, 2, 3]),
        "the 232 codec is main-keyed: a non-600 sub encodes too",
    );
    assert!(
        matches!(decode("232.601", &[1, 2, 3]), Ok(DptValue::Rgb { .. })),
        "the 232 codec is main-keyed: a non-600 sub decodes too",
    );
    // the wrong-variant tag is the fixed literal "232.600" regardless of the sub
    assert_eq!(
        encode("232.500", DptValue::U8(7)),
        Err(DptError::TypeMismatch { dpt: "232.600" }),
    );
}

#[test]
fn dpt232_600_wrong_length_loud_fails() {
    // exactly 3 octets are required; short and long payloads are InvalidLength.
    for bytes in [&[][..], &[0x00], &[0x00, 0x00], &[0x00, 0x00, 0x00, 0x00]] {
        assert!(
            matches!(
                decode("232.600", bytes),
                Err(DptError::InvalidLength { .. })
            ),
            "232.600 with {} bytes must be InvalidLength",
            bytes.len()
        );
    }
}
