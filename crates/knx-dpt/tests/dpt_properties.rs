use knx_dpt::{decode, encode, DptValue};
use proptest::prelude::*;

proptest! {
    #[test]
    fn dpt6_i8_roundtrips(value in any::<i8>()) {
        let encoded = encode("6.010", DptValue::I8(value)).unwrap();
        prop_assert_eq!(decode("6.010", &encoded).unwrap(), DptValue::I8(value));
    }

    #[test]
    fn dpt7_u16_roundtrips(value in any::<u16>()) {
        let encoded = encode("7.001", DptValue::U16(value)).unwrap();
        prop_assert_eq!(decode("7.001", &encoded).unwrap(), DptValue::U16(value));
    }

    #[test]
    fn dpt8_i16_roundtrips(value in any::<i16>()) {
        let encoded = encode("8.001", DptValue::I16(value)).unwrap();
        prop_assert_eq!(decode("8.001", &encoded).unwrap(), DptValue::I16(value));
    }

    #[test]
    fn dpt12_u32_roundtrips(value in any::<u32>()) {
        let encoded = encode("12.001", DptValue::U32(value)).unwrap();
        prop_assert_eq!(decode("12.001", &encoded).unwrap(), DptValue::U32(value));
    }

    #[test]
    fn dpt13_i32_roundtrips(value in any::<i32>()) {
        let encoded = encode("13.001", DptValue::I32(value)).unwrap();
        prop_assert_eq!(decode("13.001", &encoded).unwrap(), DptValue::I32(value));
    }

    #[test]
    fn dpt14_finite_f32_roundtrips(bits in any::<u32>()) {
        let value = f32::from_bits(bits);
        prop_assume!(value.is_finite());

        let encoded = encode("14.000", DptValue::F32(value)).unwrap();
        let decoded = decode("14.000", &encoded).unwrap();

        let DptValue::F32(actual) = decoded else {
            panic!("expected f32 value");
        };
        prop_assert_eq!(actual.to_bits(), value.to_bits());
    }
}
