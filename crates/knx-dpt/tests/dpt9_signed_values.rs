//! DPT 9 signed two's-complement codec plus 9.002 and 9.003 decoding.
//!
//! The KNX 2-octet float uses an 11-bit TWO'S-COMPLEMENT signed mantissa
//! (`value = 0.01 · mantissa · 2^exp`, mantissa in [-2048, 2047]).
//! 9.002 (Δ-temp K) / 9.003 (gradient K/h) are signed and decode-only via the
//! unit-agnostic `Float16` path; the tail widen (2..=8 → 2..=11) extends the
//! same decode-only path to 9.009 (air flow) / 9.010 / 9.011 (time period s/ms).
//! These tests cover negative values, subtypes that decode through `Float16`,
//! unsupported encode paths, neighbouring subtypes, and unchanged positive values.

use knx_dpt::{decode, encode, DptError, DptValue};

fn temperature(bytes: &[u8]) -> f32 {
    match decode("9.001", bytes).unwrap() {
        DptValue::Temperature(value) => value,
        other => panic!("9.001 expected Temperature, got {other:?}"),
    }
}

fn float16(dpt: &str, bytes: &[u8]) -> f32 {
    match decode(dpt, bytes).unwrap() {
        DptValue::Float16(value) => value,
        other => panic!("{dpt} expected Float16, got {other:?}"),
    }
}

#[test]
fn dpt9_002_003_decode_via_float16_encode_stays_unsupported() {
    // 9.002/9.003 now DECODE (decode-only) to the unit-agnostic Float16 - never
    // Temperature - and decode correctly for POSITIVE and NEGATIVE values.
    for dpt in ["9.002", "9.003"] {
        assert!(
            (float16(dpt, &[0x01, 0xF4]) - 5.0).abs() < 1e-3,
            "{dpt} +5.0"
        );
        assert!(
            (float16(dpt, &[0x86, 0x0C]) - (-5.0)).abs() < 1e-3,
            "{dpt} -5.0"
        );
        assert!(
            !matches!(
                decode(dpt, &[0x86, 0x0C]).unwrap(),
                DptValue::Temperature(_)
            ),
            "{dpt} must never decode as Temperature",
        );
        // encode stays UnsupportedDpt (decode-only, like 9.004-9.008)
        assert_eq!(
            encode(dpt, DptValue::Float16(5.0)),
            Err(DptError::UnsupportedDpt(dpt.to_owned())),
            "{dpt} encode must stay UnsupportedDpt (decode-only)",
        );
        assert_eq!(
            encode(dpt, DptValue::Temperature(5.0)),
            Err(DptError::UnsupportedDpt(dpt.to_owned())),
        );
        // invalid payload length returns InvalidLength
        assert!(matches!(
            decode(dpt, &[0x00]),
            Err(DptError::InvalidLength { .. })
        ));
    }
}

#[test]
fn dpt9_float16_tail_decodes_via_float16_encode_stays_unsupported() {
    // tail widen 2..=8 -> 2..=11: 9.009 (air flow m³/h), 9.010 (time period s),
    // 9.011 (time period ms) now decode-only to the unit-agnostic Float16 (same
    // two's-complement wire codec; never Temperature). Encode stays unsupported.
    for dpt in ["9.009", "9.010", "9.011"] {
        assert!(
            (float16(dpt, &[0x01, 0xF4]) - 5.0).abs() < 1e-3,
            "{dpt} +5.0"
        );
        // Float16 is unit-agnostic, so negatives decode by the same signed rule
        assert!(
            (float16(dpt, &[0x86, 0x0C]) - (-5.0)).abs() < 1e-3,
            "{dpt} -5.0"
        );
        assert!(
            !matches!(
                decode(dpt, &[0x01, 0xF4]).unwrap(),
                DptValue::Temperature(_)
            ),
            "{dpt} must never decode as Temperature",
        );
        assert_eq!(
            encode(dpt, DptValue::Float16(5.0)),
            Err(DptError::UnsupportedDpt(dpt.to_owned())),
            "{dpt} encode must stay UnsupportedDpt (decode-only)",
        );
        assert!(matches!(
            decode(dpt, &[0x00]),
            Err(DptError::InvalidLength { .. })
        ));
    }
}

#[test]
fn dpt9_electrical_tail_decodes_via_float16_encode_stays_unsupported() {
    // electrical widen 20..=21: 9.020 (voltage mV), 9.021 (current mA) now
    // decode-only to the unit-agnostic Float16 (same two's-complement wire
    // codec; never Temperature). Both are signed; encode stays unsupported.
    for dpt in ["9.020", "9.021"] {
        assert!(
            (float16(dpt, &[0x01, 0xF4]) - 5.0).abs() < 1e-3,
            "{dpt} +5.0"
        );
        assert!(
            (float16(dpt, &[0x86, 0x0C]) - (-5.0)).abs() < 1e-3,
            "{dpt} -5.0"
        );
        assert!(
            !matches!(
                decode(dpt, &[0x01, 0xF4]).unwrap(),
                DptValue::Temperature(_)
            ),
            "{dpt} must never decode as Temperature",
        );
        assert_eq!(
            encode(dpt, DptValue::Float16(5.0)),
            Err(DptError::UnsupportedDpt(dpt.to_owned())),
            "{dpt} encode must stay UnsupportedDpt (decode-only)",
        );
        assert!(matches!(
            decode(dpt, &[0x00]),
            Err(DptError::InvalidLength { .. })
        ));
    }
}

#[test]
fn dpt9_remaining_tail_decodes_via_float16_encode_stays_unsupported() {
    // tail widen 20..=21 -> 20..=30: the full remaining defined DPT9 tail
    // 9.022 (power density W/m²), 9.023 (kelvin/percent K/%), 9.024 (power kW),
    // 9.025 (volume flow l/h), 9.026 (rain l/m²), 9.027 (temperature °F),
    // 9.028 (wind km/h), 9.029 (abs humidity g/m³), 9.030 (concentration µg/m³)
    // now decode-only to the unit-agnostic Float16 (same two's-complement wire
    // codec; never Temperature). The signed members (9.023 K/%, 9.027 °F)
    // decode correctly because the codec is two's-complement. Encode stays
    // unsupported for all.
    for dpt in [
        "9.022", "9.023", "9.024", "9.025", "9.026", "9.027", "9.028", "9.029", "9.030",
    ] {
        assert!(
            (float16(dpt, &[0x01, 0xF4]) - 5.0).abs() < 1e-3,
            "{dpt} +5.0"
        );
        // a two's-complement negative (signed subs like 9.023/9.027 need this)
        assert!(
            (float16(dpt, &[0x86, 0x0C]) - (-5.0)).abs() < 1e-3,
            "{dpt} -5.0"
        );
        assert!(
            !matches!(
                decode(dpt, &[0x01, 0xF4]).unwrap(),
                DptValue::Temperature(_)
            ),
            "{dpt} must never decode as Temperature",
        );
        // encode stays UnsupportedDpt in both directions (decode-only)
        assert_eq!(
            encode(dpt, DptValue::Float16(5.0)),
            Err(DptError::UnsupportedDpt(dpt.to_owned())),
            "{dpt} encode (Float16) must stay UnsupportedDpt",
        );
        assert_eq!(
            encode(dpt, DptValue::Temperature(5.0)),
            Err(DptError::UnsupportedDpt(dpt.to_owned())),
            "{dpt} encode (Temperature) must stay UnsupportedDpt",
        );
        // invalid payload length returns InvalidLength per sub
        assert!(matches!(
            decode(dpt, &[0x00]),
            Err(DptError::InvalidLength { .. })
        ));
    }
}

#[test]
fn dpt9_unsupported_neighbours_stay_unsupported() {
    // decode now covers 9.002..=9.011 + 9.020..=9.030 (the full defined tail).
    // The 9.012-9.019 reserved gap stays unsupported, and 9.031 (the next sub
    // above the defined range, unassigned) stays unsupported. 9.019 (a
    // permanently-reserved gap sub) is the stable lower sentinel; 9.031 is the
    // upper sentinel that catches an accidental over-widen past 9.030.
    for dpt in ["9.012", "9.019", "9.031"] {
        assert_eq!(
            decode(dpt, &[0x00, 0x00]),
            Err(DptError::UnsupportedDpt(dpt.to_owned())),
        );
    }
}

#[test]
fn dpt9_decode_positive_is_unchanged() {
    // POSITIVE values are identical under the old and new conventions.
    assert!((temperature(&[0x01, 0xF4]) - 5.0).abs() < 1e-3); // +5.0
    assert!((temperature(&[0x0C, 0x1A]) - 21.0).abs() < 1e-3); // +21.0
    assert!((temperature(&[0x0C, 0x4E]) - 22.04).abs() < 0.05); // +22.04
    assert!((temperature(&[0x00, 0x00]) - 0.0).abs() < 1e-6); // 0.0
}

#[test]
fn dpt9_decode_negative_is_knx_twos_complement() {
    // KNX two's-complement negatives decode to the signed value. These
    // distinct bytes previously separated the intended behavior from a
    // sign-magnitude interpretation.
    assert!((temperature(&[0x86, 0x0C]) - (-5.0)).abs() < 1e-3);
    assert!((temperature(&[0x84, 0x18]) - (-10.0)).abs() < 1e-3);
    assert!((temperature(&[0x8A, 0x24]) - (-30.0)).abs() < 1e-3);
    assert!((temperature(&[0x85, 0xDA]) - (-5.5)).abs() < 1e-3);
    // most-negative mantissa at exp 0: m = -2048 -> -20.48
    assert!((temperature(&[0x80, 0x00]) - (-20.48)).abs() < 1e-3);
}

#[test]
fn dpt9_001_negative_encode_is_twos_complement_and_round_trips() {
    // negative 9.001 encode now emits KNX two's-complement bytes (was sign-mag).
    assert_eq!(
        encode("9.001", DptValue::Temperature(-5.0)).unwrap(),
        vec![0x86, 0x0C]
    );
    assert_eq!(
        encode("9.001", DptValue::Temperature(-10.0)).unwrap(),
        vec![0x84, 0x18]
    );
    assert_eq!(
        encode("9.001", DptValue::Temperature(-30.0)).unwrap(),
        vec![0x8A, 0x24]
    );
    assert_eq!(
        encode("9.001", DptValue::Temperature(-5.5)).unwrap(),
        vec![0x85, 0xDA]
    );
    // positive encode unchanged
    assert_eq!(
        encode("9.001", DptValue::Temperature(21.0)).unwrap(),
        vec![0x0C, 0x1A]
    );
    // round-trip a spread of positive AND negative values
    for v in [-30.0f32, -10.0, -5.5, -0.5, 0.0, 0.5, 5.0, 21.0, 22.04] {
        let bytes = encode("9.001", DptValue::Temperature(v)).unwrap();
        let back = temperature(&bytes);
        assert!((back - v).abs() < 0.02, "round-trip {v} -> {back}");
    }
}

#[test]
fn dpt9_existing_decode_only_floats_unaffected() {
    // regression: the shipped decode-only weather subs are untouched.
    for (dpt, bytes, expected) in [
        ("9.004", [0x57u8, 0xA1u8], 19998.72f32),
        ("9.007", [0x14, 0xE2], 50.0),
        ("9.008", [0x2D, 0x78], 448.0),
    ] {
        assert!((float16(dpt, &bytes) - expected).abs() < 0.5, "{dpt}");
    }
}
