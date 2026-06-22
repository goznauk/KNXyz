use knx_dpt::{decode, encode, DptError, DptValue};

#[test]
fn dpt_1_bool_encodes_as_one_bit_value() {
    assert_eq!(encode("1.001", DptValue::Bool(true)).unwrap(), [0x01]);
    assert_eq!(encode("1.001", DptValue::Bool(false)).unwrap(), [0x00]);
    assert_eq!(decode("1.001", &[0x01]).unwrap(), DptValue::Bool(true));
    assert_eq!(decode("1.001", &[0x00]).unwrap(), DptValue::Bool(false));
}

#[test]
fn dpt_5_raw_unsigned_roundtrips() {
    for value in [0, 128, 255] {
        assert_eq!(encode("5.010", DptValue::U8(value)).unwrap(), [value]);
        assert_eq!(decode("5.010", &[value]).unwrap(), DptValue::U8(value));
    }
}

#[test]
fn dpt_5_001_scaling_validates_and_roundtrips_common_values() {
    assert_eq!(encode("5.001", DptValue::Scaling(0.0)).unwrap(), [0]);
    assert_eq!(encode("5.001", DptValue::Scaling(50.0)).unwrap(), [128]);
    assert_eq!(encode("5.001", DptValue::Scaling(100.0)).unwrap(), [255]);

    assert_scaling(decode("5.001", &[0]).unwrap(), 0.0);
    assert_scaling(decode("5.001", &[128]).unwrap(), 50.196_08);
    assert_scaling(decode("5.001", &[255]).unwrap(), 100.0);

    assert!(matches!(
        encode("5.001", DptValue::Scaling(100.1)),
        Err(DptError::ValueOutOfRange { .. })
    ));
}

#[test]
fn dpt_9_001_temperature_encodes_self_authored_reference_value() {
    assert_eq!(
        encode("9.001", DptValue::Temperature(21.0)).unwrap(),
        [0x0c, 0x1a]
    );
    assert_temperature(decode("9.001", &[0x0c, 0x1a]).unwrap(), 21.0);
}

#[test]
fn invalid_dpt_ids_and_lengths_return_typed_errors() {
    assert!(matches!(
        encode("99.999", DptValue::Bool(true)),
        Err(DptError::UnsupportedDpt(_))
    ));
    assert!(matches!(
        encode("7.not-a-number", DptValue::U16(1)),
        Err(DptError::UnsupportedDpt(_))
    ));
    assert!(matches!(
        decode("14.float", &[0, 0, 0, 0]),
        Err(DptError::UnsupportedDpt(_))
    ));
    assert!(matches!(
        encode("9.002", DptValue::Temperature(21.0)),
        Err(DptError::UnsupportedDpt(_))
    ));
    assert!(matches!(
        decode("9.001", &[0x00]),
        Err(DptError::InvalidLength {
            expected: 2,
            actual: 1
        })
    ));
}

fn assert_scaling(value: DptValue, expected: f32) {
    match value {
        DptValue::Scaling(actual) => assert!((actual - expected).abs() < 0.001),
        other => panic!("unexpected value: {other:?}"),
    }
}

fn assert_temperature(value: DptValue, expected: f32) {
    match value {
        DptValue::Temperature(actual) => assert!((actual - expected).abs() < 0.01),
        other => panic!("unexpected value: {other:?}"),
    }
}
