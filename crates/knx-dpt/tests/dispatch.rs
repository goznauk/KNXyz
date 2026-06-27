//! Characterizes the public `encode`/`decode` dispatch contract in
//! `knx_dpt::lib` — the exact routing the (uniform-codec-table) refactor
//! must preserve, including the dpt5 (sub-typed) and dpt9 (only `.001`)
//! special cases and the unsupported/parse-failure error behavior, on
//! both the encode and decode sides.

use knx_dpt::{decode, encode, DptError, DptValue};

#[test]
fn uniform_dpt_routes_to_its_module_both_directions() {
    // A representative uniform DPT (7.xxx = u16) round-trips by main number.
    let bytes = encode("7.001", DptValue::U16(0x1234)).unwrap();
    assert_eq!(bytes, vec![0x12, 0x34]);
    assert_eq!(decode("7.001", &bytes).unwrap(), DptValue::U16(0x1234));
}

#[test]
fn dpt5_is_sub_typed_scaling_vs_raw_u8_both_directions() {
    // 5.001 = Scaling (lossy %); any other 5.xxx = raw U8 passthrough.
    assert_eq!(encode("5.001", DptValue::Scaling(50.0)).unwrap(), vec![128]);
    assert!(matches!(
        decode("5.001", &[128]).unwrap(),
        DptValue::Scaling(_)
    ));
    assert_eq!(encode("5.010", DptValue::U8(7)).unwrap(), vec![7]);
    assert_eq!(decode("5.010", &[7]).unwrap(), DptValue::U8(7));
}

#[test]
fn dpt9_001_temperature_encode_decode_and_weather_floats_decode_only() {
    // 9.001 temperature: full encode + decode via the Temperature variant.
    assert_eq!(
        encode("9.001", DptValue::Temperature(21.0)).unwrap(),
        vec![0x0c, 0x1a]
    );
    assert!(matches!(
        decode("9.001", &[0x0c, 0x1a]).unwrap(),
        DptValue::Temperature(_)
    ));
    // 9.002/9.003 Δ-temp/gradient + 9.004-9.008 weather/CO2 + 9.009-9.011
    // air-flow/time-period + 9.020/9.021 voltage/current + 9.022-9.030
    // power-density/K%/power-kW/volume-flow/rain/°F/wind-km·h/abs-humidity/
    // concentration: decode-only to the unit-agnostic Float16 variant (same
    // 2-octet two's-complement codec, never Temperature).
    for dpt in [
        "9.002", "9.003", "9.004", "9.005", "9.006", "9.007", "9.008", "9.009", "9.010", "9.011",
        "9.020", "9.021", "9.022", "9.023", "9.024", "9.025", "9.026", "9.027", "9.028", "9.029",
        "9.030",
    ] {
        assert!(
            matches!(decode(dpt, &[0x0c, 0x1a]).unwrap(), DptValue::Float16(_)),
            "{dpt} must decode to Float16, not Temperature"
        );
        // not writable via inference: a generic Float16 cannot select one
        // 9.xxx sub-type, so encode returns an error.
        assert_eq!(
            encode(dpt, DptValue::Float16(21.0)),
            Err(DptError::UnsupportedDpt(dpt.to_owned()))
        );
    }
    // A DPT9 sub with no codec (e.g. 9.019, in the reserved 9.012-9.019 gap)
    // still falls through to the unsupported wildcard in both directions.
    assert_eq!(
        encode("9.019", DptValue::Temperature(21.0)),
        Err(DptError::UnsupportedDpt("9.019".to_owned()))
    );
    assert_eq!(
        decode("9.019", &[0x00, 0x00]),
        Err(DptError::UnsupportedDpt("9.019".to_owned()))
    );
}

#[test]
fn dpt5_003_decodes_to_angle_degrees() {
    // 5.003 angle: 1-byte scaled decode (degrees = byte * 360 / 255),
    // distinct from 5.001 Scaling (percent). Decode-only.
    assert!(matches!(
        decode("5.003", &[128]).unwrap(),
        DptValue::Angle(_)
    ));
    assert_eq!(decode("5.003", &[0]).unwrap(), DptValue::Angle(0.0));
    assert_eq!(decode("5.003", &[255]).unwrap(), DptValue::Angle(360.0));
}

#[test]
fn unsupported_main_group_is_rejected_both_directions() {
    assert_eq!(
        encode("99.999", DptValue::Bool(true)),
        Err(DptError::UnsupportedDpt("99.999".to_owned()))
    );
    assert_eq!(
        decode("99.999", &[0x00]),
        Err(DptError::UnsupportedDpt("99.999".to_owned()))
    );
}

#[test]
fn unparseable_dpt_id_is_rejected_before_dispatch() {
    // DptId::parse rejects non-numeric sub and carries the raw input.
    assert_eq!(
        encode("7.not-a-number", DptValue::U16(0)),
        Err(DptError::UnsupportedDpt("7.not-a-number".to_owned()))
    );
    assert_eq!(
        decode("7.not-a-number", &[0x00, 0x00]),
        Err(DptError::UnsupportedDpt("7.not-a-number".to_owned()))
    );
}

/// Every documented supported main group (crate rustdoc) must dispatch
/// into a codec, and every unsupported one must not. Value-agnostic:
/// `decode(dpt, &[])` for a supported DPT fails inside its codec
/// (`InvalidLength`, never `UnsupportedDpt`), whereas an unsupported
/// main / DPT9 sub yields exactly `UnsupportedDpt(dpt)`. This pins the
/// full support matrix without coupling to per-codec byte formats.
#[test]
fn support_matrix_every_supported_main_dispatches() {
    const SUPPORTED: &[&str] = &[
        "1.001", "2.001", "3.007", "4.001", "4.002", "5.001", "5.003", "5.010", "6.010", "7.001",
        "8.001", "9.001", "9.002", "9.003", "9.004", "9.005", "9.006", "9.007", "9.008", "9.009",
        "9.010", "9.011", "9.020", "9.021", "9.022", "9.023", "9.024", "9.025", "9.026", "9.027",
        "9.028", "9.029", "9.030", "10.001", "11.001", "12.001", "13.001", "14.000", "16.000",
        "17.001", "18.001", "19.001", "20.102", "20.105", "21.001", "22.101", "29.010", "29.011",
        "29.012", "232.600",
    ];
    for dpt in SUPPORTED {
        let err = decode(dpt, &[]).unwrap_err();
        assert!(
            !matches!(err, DptError::UnsupportedDpt(_)),
            "{dpt} must dispatch into a codec, got {err:?}"
        );
    }

    // Unsupported main groups (incl. DPT29's neighbours 28/30), an unsupported
    // DPT4 sub (4.003 — only 4.001/4.002 are defined), and an unsupported DPT9
    // sub (9.019, reserved gap).
    for dpt in ["4.003", "9.019", "15.000", "28.001", "30.001", "99.999"] {
        assert_eq!(
            decode(dpt, &[]),
            Err(DptError::UnsupportedDpt(dpt.to_owned())),
            "{dpt} must be UnsupportedDpt"
        );
    }
}

/// DPT 20.102 (operating mode) and 20.105 (controller mode) share main 20
/// but are distinct: the sub-aware decode arm routes 20.105 to its own
/// codec/variant with the wider value set, while 20.102 keeps its 0..=4
/// range. A 20.105 byte must never decode as a 20.102 value.
#[test]
fn dpt20_operating_and_controller_modes_are_distinct() {
    // 20.105 valid values round-trip into HvacControllerMode.
    for byte in [0u8, 5, 17, 20] {
        assert_eq!(
            decode("20.105", &[byte]),
            Ok(DptValue::HvacControllerMode(byte)),
            "20.105 byte {byte} decodes to a controller mode"
        );
        assert_eq!(
            encode("20.105", DptValue::HvacControllerMode(byte)).unwrap(),
            vec![byte]
        );
    }

    // 20.105 rejects the KNX-reserved 18/19 and bytes above 20.
    for byte in [18u8, 19, 21, 255] {
        assert!(
            matches!(
                decode("20.105", &[byte]),
                Err(DptError::InvalidValue { .. })
            ),
            "20.105 byte {byte} must be rejected (reserved/out-of-range)"
        );
    }

    // byte 5 is a valid controller mode (Precool) but not a valid 20.102
    // operating mode (max 4) - the two sub-types do not alias.
    assert_eq!(decode("20.105", &[5]), Ok(DptValue::HvacControllerMode(5)));
    assert!(matches!(
        decode("20.102", &[5]),
        Err(DptError::InvalidValue { .. })
    ));

    // encode discriminates by variant: an HvacMode value still encodes as
    // 20.102 even when the requested id is 20.105 (variant is the truth),
    // and HvacControllerMode never encodes through the 20.102 range check.
    assert_eq!(encode("20.102", DptValue::HvacMode(4)).unwrap(), vec![4]);
    assert!(matches!(
        encode("20.102", DptValue::HvacMode(5)),
        Err(DptError::InvalidValue { .. })
    ));
}
