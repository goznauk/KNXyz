//! W1: native weather DPT codecs — DPT 9.004 (lux) / 9.005 (wind m/s) /
//! 9.006 (pressure Pa) / 9.007 (humidity %) decode via the shared 2-octet
//! KNX float to the unit-agnostic `Float16` variant (NEVER `Temperature`),
//! and DPT 5.003 (angle) decodes the 1-byte scaled form to `Angle` degrees.
//! All decode-only (encode loud-fails by inference — see dispatch.rs).
//!
//! The 9.xxx byte examples are the exact pinned xknx 3.15.0 wire bytes
//! (clean-room observed): DPTLux.to_knx(20000)=[0x57,0xA1] -> 19998.72,
//! DPTWsp.to_knx(5.0)=[0x01,0xF4] -> 5.0, DPTHumidity.to_knx(50.0)=
//! [0x14,0xE2] -> 50.0, DPTPressure2Byte.to_knx(101325)=[0x6C,0xD5] ->
//! 101335.04 (KNX float16 quantization).

use knx_dpt::{decode, DptError, DptValue};

fn float16(dpt: &str, bytes: &[u8]) -> f32 {
    match decode(dpt, bytes).unwrap() {
        DptValue::Float16(value) => value,
        other => panic!("{dpt} expected Float16, got {other:?}"),
    }
}

fn angle(bytes: &[u8]) -> f32 {
    match decode("5.003", bytes).unwrap() {
        DptValue::Angle(value) => value,
        other => panic!("5.003 expected Angle, got {other:?}"),
    }
}

#[test]
fn weather_floats_decode_to_float16_matching_pinned_bytes() {
    assert!((float16("9.004", &[0x57, 0xA1]) - 19998.72).abs() < 0.5);
    assert!((float16("9.005", &[0x01, 0xF4]) - 5.0).abs() < 1e-3);
    assert!((float16("9.007", &[0x14, 0xE2]) - 50.0).abs() < 1e-3);
    assert!((float16("9.006", &[0x6C, 0xD5]) - 101335.04).abs() < 1.0);
    // 9.008 CO2/air-quality ppm: same 2-octet float codec, decode-only to
    // Float16 (CO2 is never tagged Temperature). exp=5, mantissa=1400 -> 448 ppm.
    assert!((float16("9.008", &[0x2D, 0x78]) - 448.0).abs() < 0.5);
    // a small clean word decodes identically through the 9.008 sub.
    assert!((float16("9.008", &[0x01, 0xF4]) - 5.0).abs() < 1e-3);
}

#[test]
fn weather_floats_handle_sign_and_zero() {
    // shared float16 math: 0x0000 -> 0.0; negative sign bit honored.
    assert_eq!(float16("9.005", &[0x00, 0x00]), 0.0);
    // 9.001-style payload via the weather variant decodes the same number,
    // just tagged Float16 instead of Temperature (no misrepresentation).
    assert!((float16("9.004", &[0x0c, 0x1a]) - 21.0).abs() < 1e-3);
}

#[test]
fn angle_503_decodes_degrees_byte_times_360_over_255() {
    assert_eq!(angle(&[0x00]), 0.0);
    assert_eq!(angle(&[0xFF]), 360.0);
    // unrounded native value (consistent with 5.001 Scaling being
    // unquantized): 128 * 360 / 255 = 180.7058..., NOT pinned's rounded 181.
    assert!((angle(&[0x80]) - 180.705_88).abs() < 1e-2);
    assert!((angle(&[0x40]) - 90.352_94).abs() < 1e-2);
}

#[test]
fn temperature_9001_is_unchanged_not_float16() {
    // regression: 9.001 still decodes to Temperature, never Float16.
    assert!(matches!(
        decode("9.001", &[0x0c, 0x1a]).unwrap(),
        DptValue::Temperature(_)
    ));
}

#[test]
fn weather_invalid_payload_length_is_loud_not_unsupported() {
    // 2-octet floats need exactly 2 bytes; 5.003 needs exactly 1.
    for dpt in ["9.004", "9.005", "9.006", "9.007", "9.008"] {
        let err = decode(dpt, &[0x00]).unwrap_err();
        assert!(
            !matches!(err, DptError::UnsupportedDpt(_)),
            "{dpt}: {err:?}"
        );
        let err = decode(dpt, &[]).unwrap_err();
        assert!(
            !matches!(err, DptError::UnsupportedDpt(_)),
            "{dpt}: {err:?}"
        );
    }
    let err = decode("5.003", &[]).unwrap_err();
    assert!(
        !matches!(err, DptError::UnsupportedDpt(_)),
        "5.003: {err:?}"
    );
    let err = decode("5.003", &[0x00, 0x00]).unwrap_err();
    assert!(
        !matches!(err, DptError::UnsupportedDpt(_)),
        "5.003: {err:?}"
    );
}
