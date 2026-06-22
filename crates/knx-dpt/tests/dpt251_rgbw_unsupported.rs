//! DPT 251.600 RGBW values are not implemented yet.
//!
//! The existing `DptValue::Rgbw` value has no validity-mask field, while the
//! 251.600 payload shape includes mask information. Until the 6-octet RGBW
//! layout is represented explicitly and covered by reference bytes, the codec
//! rejects `251.600` rather than decoding a masked value into an unmasked type.
//!
//! These tests verify the current unsupported status in both directions and
//! keep the implemented RGB codec independent from the unsupported RGBW id.

use knx_dpt::{decode, encode, DptError, DptValue};

#[test]
fn dpt251_600_decode_is_unsupported() {
    // representative 6-octet RGBW-shaped payload + the empty slice: both must
    // be rejected at dispatch (main 251 has no codec), NOT silently decoded
    // onto a mask-less Rgbw.
    for bytes in [&[0x0A, 0x14, 0x1E, 0x28, 0x00, 0x0F][..], &[]] {
        assert_eq!(
            decode("251.600", bytes),
            Err(DptError::UnsupportedDpt("251.600".to_owned())),
            "251.600 decode must stay UnsupportedDpt until the masked RGBW layout is implemented",
        );
    }
}

#[test]
fn dpt251_600_encode_is_unsupported() {
    // encode must stay refused too: main 251 is absent from the uniform codec
    // table and has no explicit arm, so colour writes remain unavailable.
    assert_eq!(
        encode(
            "251.600",
            DptValue::Rgbw {
                red: 1,
                green: 2,
                blue: 3,
                white: 4,
            },
        ),
        Err(DptError::UnsupportedDpt("251.600".to_owned())),
    );
}

#[test]
fn dpt251_and_colour_neighbours_stay_unsupported() {
    // dispatch is by MAIN, so guard that no neighbouring sub/main was made
    // supported by accident: other 251.xxx subs, and the adjacent 250/252 mains.
    for dpt in ["251.601", "250.600", "252.600"] {
        assert_eq!(
            decode(dpt, &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
            Err(DptError::UnsupportedDpt(dpt.to_owned())),
            "{dpt} must be UnsupportedDpt",
        );
    }
}

#[test]
fn dpt232_600_rgb_decode_still_works() {
    // regression: the shipped 232.600 RGB decode is unaffected by the
    // unsupported 251.600 RGBW id.
    assert_eq!(
        decode("232.600", &[0x0A, 0x14, 0x1E]).unwrap(),
        DptValue::Rgb {
            red: 10,
            green: 20,
            blue: 30,
        },
    );
}
