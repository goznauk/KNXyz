use knx_dpt::{decode, encode, DptError, DptValue};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Fixture {
    name: String,
    dpt: String,
    value: FixtureValue,
    bytes: Vec<u8>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum FixtureValue {
    Bool {
        value: bool,
    },
    U8 {
        value: u8,
    },
    Scaling {
        value: f32,
    },
    Temperature {
        value: f32,
    },
    ControlBool {
        control: bool,
        value: bool,
    },
    StepControl {
        increase: bool,
        step_code: u8,
    },
    I8 {
        value: i8,
    },
    U16 {
        value: u16,
    },
    I16 {
        value: i16,
    },
    Time {
        weekday: u8,
        hour: u8,
        minute: u8,
        second: u8,
    },
    Date {
        year: u16,
        month: u8,
        day: u8,
    },
    #[serde(rename = "datetime")]
    DateTime {
        year: u16,
        month: u8,
        day: u8,
        weekday: u8,
        hour: u8,
        minute: u8,
        second: u8,
    },
    U32 {
        value: u32,
    },
    I32 {
        value: i32,
    },
    F32 {
        value: f32,
    },
    Text14 {
        value: String,
    },
    SceneNumber {
        value: u8,
    },
    SceneControl {
        learn: bool,
        scene: u8,
    },
    HvacMode {
        value: u8,
    },
    HvacControllerMode {
        value: u8,
    },
}

impl From<FixtureValue> for DptValue {
    fn from(value: FixtureValue) -> Self {
        match value {
            FixtureValue::Bool { value } => DptValue::Bool(value),
            FixtureValue::U8 { value } => DptValue::U8(value),
            FixtureValue::Scaling { value } => DptValue::Scaling(value),
            FixtureValue::Temperature { value } => DptValue::Temperature(value),
            FixtureValue::ControlBool { control, value } => {
                DptValue::ControlBool { control, value }
            }
            FixtureValue::StepControl {
                increase,
                step_code,
            } => DptValue::StepControl {
                increase,
                step_code,
            },
            FixtureValue::I8 { value } => DptValue::I8(value),
            FixtureValue::U16 { value } => DptValue::U16(value),
            FixtureValue::I16 { value } => DptValue::I16(value),
            FixtureValue::Time {
                weekday,
                hour,
                minute,
                second,
            } => DptValue::Time {
                weekday,
                hour,
                minute,
                second,
            },
            FixtureValue::Date { year, month, day } => DptValue::Date { year, month, day },
            FixtureValue::DateTime {
                year,
                month,
                day,
                weekday,
                hour,
                minute,
                second,
            } => DptValue::DateTime {
                year,
                month,
                day,
                weekday,
                hour,
                minute,
                second,
            },
            FixtureValue::U32 { value } => DptValue::U32(value),
            FixtureValue::I32 { value } => DptValue::I32(value),
            FixtureValue::F32 { value } => DptValue::F32(value),
            FixtureValue::Text14 { value } => DptValue::Text14(value),
            FixtureValue::SceneNumber { value } => DptValue::SceneNumber(value),
            FixtureValue::SceneControl { learn, scene } => DptValue::SceneControl { learn, scene },
            FixtureValue::HvacMode { value } => DptValue::HvacMode(value),
            FixtureValue::HvacControllerMode { value } => DptValue::HvacControllerMode(value),
        }
    }
}

#[test]
fn core_fixtures_encode_and_decode() {
    for fixture in load_fixtures() {
        let value = DptValue::from(fixture.value);
        let encoded = encode(&fixture.dpt, value.clone())
            .unwrap_or_else(|error| panic!("{} encode failed: {error}", fixture.name));
        assert_eq!(encoded, fixture.bytes, "{} encoded bytes", fixture.name);

        let decoded = decode(&fixture.dpt, &fixture.bytes)
            .unwrap_or_else(|error| panic!("{} decode failed: {error}", fixture.name));
        assert_eq!(decoded, value, "{} decoded value", fixture.name);
    }
}

#[test]
fn invalid_lengths_return_typed_errors() {
    for fixture in load_fixtures() {
        let mut truncated = fixture.bytes.clone();
        truncated.pop();

        assert!(
            matches!(
                decode(&fixture.dpt, &truncated),
                Err(DptError::InvalidLength { .. })
            ),
            "{} should reject truncated payload",
            fixture.name
        );
    }
}

#[test]
fn numeric_and_time_boundaries_roundtrip() {
    for value in [i8::MIN, 0, i8::MAX] {
        let encoded = encode("6.010", DptValue::I8(value)).unwrap();
        assert_eq!(decode("6.010", &encoded).unwrap(), DptValue::I8(value));
    }

    for value in [u16::MIN, u16::MAX] {
        let encoded = encode("7.001", DptValue::U16(value)).unwrap();
        assert_eq!(decode("7.001", &encoded).unwrap(), DptValue::U16(value));
    }

    for value in [i16::MIN, 0, i16::MAX] {
        let encoded = encode("8.001", DptValue::I16(value)).unwrap();
        assert_eq!(decode("8.001", &encoded).unwrap(), DptValue::I16(value));
    }

    let time = DptValue::Time {
        weekday: 7,
        hour: 23,
        minute: 59,
        second: 59,
    };
    assert_eq!(
        decode("10.001", &encode("10.001", time.clone()).unwrap()).unwrap(),
        time
    );

    let date = DptValue::Date {
        year: 2024,
        month: 12,
        day: 31,
    };
    assert_eq!(
        decode("11.001", &encode("11.001", date.clone()).unwrap()).unwrap(),
        date
    );

    for value in [u32::MIN, u32::MAX] {
        let encoded = encode("12.001", DptValue::U32(value)).unwrap();
        assert_eq!(decode("12.001", &encoded).unwrap(), DptValue::U32(value));
    }

    for value in [i32::MIN, 0, i32::MAX] {
        let encoded = encode("13.001", DptValue::I32(value)).unwrap();
        assert_eq!(decode("13.001", &encoded).unwrap(), DptValue::I32(value));
    }

    for value in [1.0_f32, -1.0] {
        let encoded = encode("14.000", DptValue::F32(value)).unwrap();
        assert_float_value(decode("14.000", &encoded).unwrap(), value);
    }
}

#[test]
fn numeric_and_time_validation_rejects_invalid_values() {
    assert!(matches!(
        encode(
            "10.001",
            DptValue::Time {
                weekday: 1,
                hour: 24,
                minute: 0,
                second: 0,
            },
        ),
        Err(DptError::InvalidValue { .. })
    ));
    assert!(matches!(
        encode(
            "10.001",
            DptValue::Time {
                weekday: 1,
                hour: 23,
                minute: 60,
                second: 0,
            },
        ),
        Err(DptError::InvalidValue { .. })
    ));
    assert!(matches!(
        encode(
            "10.001",
            DptValue::Time {
                weekday: 1,
                hour: 23,
                minute: 0,
                second: 60,
            },
        ),
        Err(DptError::InvalidValue { .. })
    ));
    assert!(matches!(
        encode(
            "11.001",
            DptValue::Date {
                year: 2024,
                month: 0,
                day: 1,
            },
        ),
        Err(DptError::InvalidValue { .. })
    ));
    assert!(matches!(
        encode(
            "11.001",
            DptValue::Date {
                year: 2024,
                month: 13,
                day: 1,
            },
        ),
        Err(DptError::InvalidValue { .. })
    ));
    assert!(matches!(
        encode(
            "11.001",
            DptValue::Date {
                year: 2024,
                month: 1,
                day: 0,
            },
        ),
        Err(DptError::InvalidValue { .. })
    ));
    assert!(matches!(
        encode("14.000", DptValue::F32(f32::NAN)),
        Err(DptError::InvalidValue { .. })
    ));
    assert!(matches!(
        encode("14.000", DptValue::F32(f32::INFINITY)),
        Err(DptError::InvalidValue { .. })
    ));
}

#[test]
fn structured_values_encode_decode_and_validate() {
    assert_eq!(
        encode(
            "2.001",
            DptValue::ControlBool {
                control: true,
                value: false,
            },
        )
        .unwrap(),
        [0x02]
    );
    assert_eq!(
        decode("2.001", &[0x02]).unwrap(),
        DptValue::ControlBool {
            control: true,
            value: false,
        }
    );

    assert_eq!(
        encode(
            "3.007",
            DptValue::StepControl {
                increase: true,
                step_code: 3,
            },
        )
        .unwrap(),
        [0x0b]
    );
    assert!(matches!(
        encode(
            "3.007",
            DptValue::StepControl {
                increase: true,
                step_code: 8,
            },
        ),
        Err(DptError::InvalidValue { .. })
    ));

    let encoded = encode("16.000", DptValue::Text14("knxyz".to_owned())).unwrap();
    assert_eq!(encoded.len(), 14);
    assert_eq!(
        decode("16.000", &encoded).unwrap(),
        DptValue::Text14("knxyz".to_owned())
    );
    assert_eq!(
        decode("16.000", &[b'a', 0, b'b', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],).unwrap(),
        DptValue::Text14("a\0b".to_owned())
    );

    for scene in [0, 63] {
        let encoded = encode("17.001", DptValue::SceneNumber(scene)).unwrap();
        assert_eq!(
            decode("17.001", &encoded).unwrap(),
            DptValue::SceneNumber(scene)
        );
    }
    assert!(matches!(
        encode("17.001", DptValue::SceneNumber(64)),
        Err(DptError::InvalidValue { .. })
    ));

    assert_eq!(
        encode(
            "18.001",
            DptValue::SceneControl {
                learn: true,
                scene: 7,
            },
        )
        .unwrap(),
        [0x87]
    );
    assert!(matches!(
        encode(
            "18.001",
            DptValue::SceneControl {
                learn: true,
                scene: 64,
            },
        ),
        Err(DptError::InvalidValue { .. })
    ));
}

#[test]
fn unconfirmed_category_identifiers_stay_unsupported() {
    assert!(matches!(
        encode(
            "rgb.unconfirmed",
            DptValue::Rgb {
                red: 1,
                green: 2,
                blue: 3,
            },
        ),
        Err(DptError::UnsupportedDpt(_))
    ));
    assert!(matches!(
        encode(
            "rgbw.unconfirmed",
            DptValue::Rgbw {
                red: 1,
                green: 2,
                blue: 3,
                white: 4,
            },
        ),
        Err(DptError::UnsupportedDpt(_))
    ));
    assert!(matches!(
        encode("hvac.unconfirmed", DptValue::HvacMode(1)),
        Err(DptError::UnsupportedDpt(_))
    ));
    assert!(matches!(
        encode("energy.unconfirmed", DptValue::EnergyI32(1)),
        Err(DptError::UnsupportedDpt(_))
    ));
    assert!(matches!(
        encode("energy-unsigned.unconfirmed", DptValue::EnergyU32(1)),
        Err(DptError::UnsupportedDpt(_))
    ));
    // DPT29 V64 is decode-only: the I64 variant never encodes because a single
    // variant cannot infer a 29.xxx sub.
    assert!(matches!(
        encode("v64.unconfirmed", DptValue::I64(1)),
        Err(DptError::UnsupportedDpt(_))
    ));
    // DPT4 Char is decode-only: encode returns an error, so a decoded
    // character can never be re-encoded to a wrong main.
    assert!(matches!(
        encode("char.unconfirmed", DptValue::Char('A')),
        Err(DptError::UnsupportedDpt(_))
    ));
}

fn load_fixtures() -> Vec<Fixture> {
    serde_json::from_str(include_str!("fixtures/dpt_core_fixtures.json")).unwrap()
}

fn assert_float_value(value: DptValue, expected: f32) {
    match value {
        DptValue::F32(actual) => assert_eq!(actual.to_bits(), expected.to_bits()),
        other => panic!("unexpected value: {other:?}"),
    }
}
