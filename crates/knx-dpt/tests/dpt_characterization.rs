//! Characterization + regression tests for `knx-dpt` boundary / edge / error
//! behavior of DPT9 (2-byte float), DPT10 (time) and DPT11 (date).
//!
//! These tests assert the current behavior of the crate. They do not
//! change any production logic. Existing happy-path coverage (9.001 21.0 /
//! -5.5 / 0.0, the shared fixture dates/times, generic length rejects) lives
//! in `dpt_mvp.rs` / `dpt_core_fixtures.rs` / `dpt_properties.rs` and is NOT
//! duplicated here. The value-add here is: signed-zero, near-max round-trip
//! quantization, out-of-range, non-finite, type-mismatch, the KNX two-digit
//! year pivot, year validation ordering, and bounded round-trip proptests.

use knx_dpt::{decode, encode, DptError, DptValue};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// DPT9 (9.001 Temperature) — pure characterization, no known bug.
// ---------------------------------------------------------------------------

#[test]
fn dpt9_encode_positive_zero() {
    assert_eq!(
        encode("9.001", DptValue::Temperature(0.0)).unwrap(),
        vec![0x00, 0x00]
    );
    let DptValue::Temperature(d) = decode("9.001", &[0x00, 0x00]).unwrap() else {
        panic!("expected Temperature");
    };
    assert!(d.abs() < 1e-6, "decoded ~0.0, got {d}");
}

#[test]
fn dpt9_encode_negative_zero_is_canonical_zero_twos_complement() {
    // Two's-complement: the sign bit is driven by the MANTISSA, not the f32
    // sign, so -0.0 (mantissa 0) encodes to the canonical [0x00, 0x00] (no
    // spurious sign bit) and decodes back to +0.0.
    assert_eq!(
        encode("9.001", DptValue::Temperature(-0.0)).unwrap(),
        vec![0x00, 0x00]
    );
    let DptValue::Temperature(d) = decode("9.001", &[0x00, 0x00]).unwrap() else {
        panic!("expected Temperature");
    };
    assert_eq!(d, 0.0);
    // [0x80, 0x00] is now the most-negative mantissa at exp 0 (-2048 -> -20.48),
    // not a "negative zero" bit pattern.
    let DptValue::Temperature(neg) = decode("9.001", &[0x80, 0x00]).unwrap() else {
        panic!("expected Temperature");
    };
    assert!((neg - (-20.48)).abs() < 1e-3, "got {neg}");
}

#[test]
fn dpt9_encode_small_positive() {
    assert_eq!(
        encode("9.001", DptValue::Temperature(0.01)).unwrap(),
        vec![0x00, 0x01]
    );
}

#[test]
fn dpt9_encode_max_decodable_magnitude_round_trip_quantizes() {
    // 670924.75 is the largest value that still encodes; it maps to the
    // all-ones-mantissa max code 0x7fff. Decoding 0x7fff yields a smaller
    // value (~670760.9375): this documents the near-max round-trip
    // quantization loss inherent to the 11-bit mantissa.
    assert_eq!(
        encode("9.001", DptValue::Temperature(670924.75)).unwrap(),
        vec![0x7f, 0xff]
    );
    let DptValue::Temperature(d) = decode("9.001", &[0x7f, 0xff]).unwrap() else {
        panic!("expected Temperature");
    };
    // True decoded value is ~670760.9375; assert within 1.0 (literal kept
    // short for clippy::excessive_precision — f32 cannot resolve more here).
    assert!(
        (d - 670_761.0).abs() < 1.0,
        "decoded near-max ~670760.94, got {d}"
    );
}

#[test]
fn dpt9_encode_just_over_max_is_out_of_range() {
    // 670_924.8f32 is bit-identical to 670924.8125f32 (f32 ulp ~0.0625 here);
    // it is the first value above the encodable max, so it is rejected.
    let err = encode("9.001", DptValue::Temperature(670_924.8)).unwrap_err();
    assert_eq!(
        err,
        DptError::ValueOutOfRange {
            dpt: "9.001",
            value: 670_924.8,
        }
    );
}

#[test]
fn dpt9_encode_far_over_max_is_out_of_range() {
    let err = encode("9.001", DptValue::Temperature(671000.0)).unwrap_err();
    assert_eq!(
        err,
        DptError::ValueOutOfRange {
            dpt: "9.001",
            value: 671000.0,
        }
    );
}

#[test]
fn dpt9_encode_nan_is_invalid_value() {
    let err = encode("9.001", DptValue::Temperature(f32::NAN)).unwrap_err();
    assert_eq!(
        err,
        DptError::InvalidValue {
            dpt: "9.001",
            reason: "temperature must be finite",
        }
    );
}

#[test]
fn dpt9_encode_infinity_is_invalid_value() {
    let err = encode("9.001", DptValue::Temperature(f32::INFINITY)).unwrap_err();
    assert_eq!(
        err,
        DptError::InvalidValue {
            dpt: "9.001",
            reason: "temperature must be finite",
        }
    );
}

#[test]
fn dpt9_encode_neg_infinity_is_invalid_value() {
    let err = encode("9.001", DptValue::Temperature(f32::NEG_INFINITY)).unwrap_err();
    assert_eq!(
        err,
        DptError::InvalidValue {
            dpt: "9.001",
            reason: "temperature must be finite",
        }
    );
}

#[test]
fn dpt9_encode_wrong_value_type_is_type_mismatch() {
    let err = encode("9.001", DptValue::U8(0)).unwrap_err();
    assert_eq!(err, DptError::TypeMismatch { dpt: "9.001" });
}

// ---------------------------------------------------------------------------
// DPT11 (11.001 Date) — KNX two-digit year pivot characterization.
// ---------------------------------------------------------------------------

// The KNX 1-byte year code pivots on a two-digit window:
//   years 1990..=1999 -> codes 90..=99   (decoded as 1900 + code)
//   years 2000..=2089 -> codes  0..=89   (decoded as 2000 + code)
fn dpt11_round_trip(year: u16, month: u8, day: u8, expected_bytes: [u8; 3]) {
    let date = DptValue::Date { year, month, day };
    let encoded = encode("11.001", date.clone()).unwrap();
    assert_eq!(encoded, expected_bytes.to_vec(), "wire bytes for {year}");
    assert_eq!(
        decode("11.001", &encoded).unwrap(),
        date,
        "round-trip {year}"
    );
}

#[test]
fn dpt11_year_pivot_1990_lower_window_boundary() {
    dpt11_round_trip(1990, 1, 1, [1, 1, 90]);
}

#[test]
fn dpt11_year_pivot_1999_upper_1900s() {
    dpt11_round_trip(1999, 6, 15, [15, 6, 99]);
}

#[test]
fn dpt11_year_pivot_2000_lower_2000s() {
    dpt11_round_trip(2000, 1, 1, [1, 1, 0]);
}

#[test]
fn dpt11_year_pivot_2089_upper_window_boundary() {
    dpt11_round_trip(2089, 12, 31, [31, 12, 89]);
}

#[test]
fn dpt11_year_below_range_is_invalid_value() {
    // validate_date runs before encode_year; the reason dpt label is "11.xxx".
    let err = encode(
        "11.001",
        DptValue::Date {
            year: 1989,
            month: 1,
            day: 1,
        },
    )
    .unwrap_err();
    assert_eq!(
        err,
        DptError::InvalidValue {
            dpt: "11.xxx",
            reason: "year must be between 1990 and 2089",
        }
    );
}

#[test]
fn dpt11_year_above_range_is_invalid_value() {
    let err = encode(
        "11.001",
        DptValue::Date {
            year: 2090,
            month: 1,
            day: 1,
        },
    )
    .unwrap_err();
    assert_eq!(
        err,
        DptError::InvalidValue {
            dpt: "11.xxx",
            reason: "year must be between 1990 and 2089",
        }
    );
}

// ---------------------------------------------------------------------------
// Bounded, deterministic round-trip proptests for DPT9 / DPT10 / DPT11.
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn dpt9_temperature_round_trip_within_magnitude_tolerance(
        value in -273.0f32..=2000.0f32,
    ) {
        let encoded = encode("9.001", DptValue::Temperature(value)).unwrap();
        let decoded = decode("9.001", &encoded).unwrap();
        let DptValue::Temperature(actual) = decoded else {
            panic!("expected Temperature");
        };
        // DPT9 has an 11-bit mantissa; tolerance scales with magnitude.
        let tol = f32::max(0.01, value.abs() / 1500.0);
        prop_assert!(
            (actual - value).abs() <= tol,
            "value {value} decoded as {actual}, tol {tol}"
        );
    }

    #[test]
    fn dpt10_time_round_trips(
        weekday in 0u8..=7,
        hour in 0u8..=23,
        minute in 0u8..=59,
        second in 0u8..=59,
    ) {
        let time = DptValue::Time { weekday, hour, minute, second };
        let encoded = encode("10.001", time.clone()).unwrap();
        prop_assert_eq!(decode("10.001", &encoded).unwrap(), time);
    }

    #[test]
    fn dpt11_date_round_trips(
        year in 1990u16..=2089,
        month in 1u8..=12,
        day in 1u8..=31,
    ) {
        let date = DptValue::Date { year, month, day };
        let encoded = encode("11.001", date.clone()).unwrap();
        prop_assert_eq!(decode("11.001", &encoded).unwrap(), date);
    }
}
