//! DPT13 energy semantics for 13.010/13.013/13.014/13.015.
//!
//! The four energy subs - 13.010 Wh / 13.013 kWh / 13.014 VAh / 13.015 VARh -
//! decode to `DptValue::EnergyI32` and have a dedicated symmetric payload encode
//! arm (`dpt13::decode_energy`/`encode_energy`) selected before the uniform table
//! in `lib.rs` via a sub-agnostic `matches!(id.sub(), 10 | 13 | 14 | 15)` guard.
//! Every other `13.xxx` (including the 13.001 counter and adjacent unselected
//! subs like 13.011/13.012) keeps decoding to the generic `I32` and keeps
//! refusing `EnergyI32` on offline encode.
//!
//! This only changes the value's type tag (`I32` -> `EnergyI32`); the raw
//! 4-octet signed payload is identical and the unit is still discarded. The
//! payload encode arm accepts both `EnergyI32` (symmetric with decode) and `I32`
//! (backward compatibility); both produce identical bytes. `EnergyU32` and every
//! other variant are refused (`TypeMismatch { dpt: "13.xxx" }`). This is a
//! payload codec: live energy writes stay refused by knx-ip's
//! `encode_value` variant-keyed write inference.
//!
//! These tests cover both halves so any added energy sub updates the selected
//! set deliberately.

use knx_dpt::{decode, encode, DptError, DptValue};

// 1000 = 0x0000_03E8 ; -1000 = 0xFFFF_FC18 (two's-complement i32).
const POS_BYTES: [u8; 4] = [0x00, 0x00, 0x03, 0xE8];
const NEG_BYTES: [u8; 4] = [0xFF, 0xFF, 0xFC, 0x18];

// The four switched energy subs (Wh/kWh/VAh/VARh).
const SELECTED_SUBS: [&str; 4] = ["13.010", "13.013", "13.014", "13.015"];
// Non-energy / non-selected 13.xxx that must stay generic I32: the 13.001
// counter plus adjacent unselected subs straddling the set (13.011/13.012 in the
// 10..13 gap, 13.016 just above 15) that prove `matches!(..., 10 | 13 | 14 | 15)`
// is an exact closed set, with no range below or above.
const NON_ENERGY_SUBS: [&str; 4] = ["13.001", "13.011", "13.012", "13.016"];

#[test]
fn dpt13_selected_energy_subs_decode_to_energy_i32() {
    // every selected energy sub decodes to the energy-tagged EnergyI32 (same
    // signed i32 value; only the type tag differs from the generic I32).
    for dpt in SELECTED_SUBS {
        assert_eq!(
            decode(dpt, &POS_BYTES),
            Ok(DptValue::EnergyI32(1000)),
            "{dpt} must decode to EnergyI32(1000) after the switch",
        );
        assert_eq!(
            decode(dpt, &NEG_BYTES),
            Ok(DptValue::EnergyI32(-1000)),
            "{dpt} must decode the signed value as EnergyI32(-1000)",
        );
        assert!(
            !matches!(decode(dpt, &POS_BYTES).unwrap(), DptValue::I32(_)),
            "{dpt} must no longer decode to the generic I32",
        );
    }
}

#[test]
fn dpt13_non_energy_subs_still_decode_to_i32() {
    // the 13.001 counter + adjacent unselected subs stay generic I32 — proves
    // EnergyI32 does not leak past the exact {10,13,14,15} set.
    for dpt in NON_ENERGY_SUBS {
        assert_eq!(
            decode(dpt, &POS_BYTES),
            Ok(DptValue::I32(1000)),
            "{dpt} must still decode to I32(1000) (uniform DPT13 codec)",
        );
        assert_eq!(
            decode(dpt, &NEG_BYTES),
            Ok(DptValue::I32(-1000)),
            "{dpt} must still decode the signed value as I32(-1000)",
        );
        assert!(
            !matches!(decode(dpt, &POS_BYTES).unwrap(), DptValue::EnergyI32(_)),
            "{dpt} must not decode to EnergyI32 (not a selected sub)",
        );
    }
}

#[test]
fn dpt13_selected_energy_subs_offline_encode_is_symmetric() {
    for dpt in SELECTED_SUBS {
        // payload encode accepts EnergyI32 (symmetric with decode_energy)
        assert_eq!(
            encode(dpt, DptValue::EnergyI32(1000)),
            Ok(POS_BYTES.to_vec()),
            "{dpt} must encode EnergyI32 to its 4-octet payload",
        );
        // and still accepts a generic I32 for callers that provide plain signed
        // DPT13 values.
        assert_eq!(
            encode(dpt, DptValue::I32(1000)),
            Ok(POS_BYTES.to_vec()),
            "{dpt} must still accept a generic I32 on payload encode (backward compat)",
        );
        // EnergyU32 is refused even for the selected subs because DPT13 is signed;
        // the dedicated arm's tag is the sub-agnostic "13.xxx".
        assert_eq!(
            encode(dpt, DptValue::EnergyU32(1000)),
            Err(DptError::TypeMismatch { dpt: "13.xxx" }),
            "{dpt} must refuse EnergyU32 (not a DPT13 candidate)",
        );
        // round-trip: EnergyI32 -> bytes -> EnergyI32 (symmetric); and the benign
        // asymmetry: a generic I32 encodes to the same bytes but decodes back to
        // EnergyI32 (the switched sub's canonical decode type).
        assert_eq!(
            decode(dpt, &encode(dpt, DptValue::EnergyI32(-1000)).unwrap()),
            Ok(DptValue::EnergyI32(-1000)),
            "{dpt} EnergyI32 must round-trip",
        );
        assert_eq!(
            decode(dpt, &encode(dpt, DptValue::I32(-1000)).unwrap()),
            Ok(DptValue::EnergyI32(-1000)),
            "{dpt} I32-encode decodes back as EnergyI32 (benign asymmetry)",
        );
    }
}

#[test]
fn dpt13_non_energy_subs_encode_only_accept_i32() {
    for dpt in NON_ENERGY_SUBS {
        // I32 encodes (the sub is writable via the unchanged uniform codec)
        assert_eq!(
            encode(dpt, DptValue::I32(1000)),
            Ok(POS_BYTES.to_vec()),
            "{dpt} must still encode an I32 value",
        );
        // EnergyI32 remains rejected on payload encode for the non-selected subs:
        // the uniform DPT13 macro codec accepts only I32, so a non-I32 variant
        // yields TypeMismatch with the macro's "13.xxx" tag. Proves EnergyI32
        // does not leak into 13.001 or any adjacent unselected 13.xxx.
        assert_eq!(
            encode(dpt, DptValue::EnergyI32(1000)),
            Err(DptError::TypeMismatch { dpt: "13.xxx" }),
            "{dpt} encode of EnergyI32 must stay refused (not a selected sub)",
        );
        // EnergyU32 is likewise refused (never a DPT13 candidate).
        assert_eq!(
            encode(dpt, DptValue::EnergyU32(1000)),
            Err(DptError::TypeMismatch { dpt: "13.xxx" }),
            "{dpt} encode of EnergyU32 must stay refused",
        );
    }
}

#[test]
fn dpt13_energy_subs_invalid_length_returns_invalid_length() {
    // the switched subs (decode_energy) length-check via be_array::<4>.
    for dpt in SELECTED_SUBS {
        assert!(
            matches!(decode(dpt, &[0x00]), Err(DptError::InvalidLength { .. })),
            "{dpt} must reject a short payload",
        );
    }
}

#[test]
fn dpt12_001_refuses_energy_u32_and_accepts_u32() {
    // EnergyU32's only positive "real home" would be the unsigned 4-octet main 12
    // — and even there it is refused: main 12 is the uniform U32 codec
    // (`dpt12.rs`), so a non-U32 variant yields TypeMismatch with the macro's
    // "12.xxx" tag, while a plain U32 still encodes. This pins in knx-dpt (not
    // just the bindings) that EnergyU32 has no positive encode home anywhere.
    assert_eq!(
        encode("12.001", DptValue::EnergyU32(1000)),
        Err(DptError::TypeMismatch { dpt: "12.xxx" }),
        "main 12 must refuse EnergyU32 (uniform U32 codec)",
    );
    assert_eq!(
        encode("12.001", DptValue::U32(1000)),
        Ok(1000u32.to_be_bytes().to_vec()),
        "main 12 must still encode a plain U32",
    );
}

#[test]
fn energy_u32_has_no_decode_producer() {
    // EnergyU32 is dead-on-the-wire: NO codec anywhere decodes to it (its sole
    // definition is the `value.rs` variant). Main 12 decodes to U32; the DPT13
    // energy subs decode to EnergyI32 — never EnergyU32. Pin the absence of a
    // producer so a future stray arm emitting EnergyU32 is caught loudly.
    assert_eq!(
        decode("12.001", &[0x00, 0x00, 0x00, 0x01]),
        Ok(DptValue::U32(1)),
        "main 12 decodes to U32, never EnergyU32",
    );
    for dpt in SELECTED_SUBS {
        assert!(
            !matches!(decode(dpt, &POS_BYTES), Ok(DptValue::EnergyU32(_))),
            "{dpt} must never decode to EnergyU32 (it decodes to EnergyI32)",
        );
    }
    for dpt in NON_ENERGY_SUBS {
        assert!(
            !matches!(decode(dpt, &POS_BYTES), Ok(DptValue::EnergyU32(_))),
            "{dpt} must never decode to EnergyU32",
        );
    }
}
