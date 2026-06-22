//! Pins the exact `TypeMismatch`/`InvalidLength` errors for the fixed-width
//! integer DPT codecs. Prior to this, only dpt9 had exact-field error
//! assertions; the integer codecs' tags/lengths were unprotected, so the
//! `impl_int_dpt!` macro (and `be_array`) could have drifted silently.

use knx_dpt::{decode, encode, DptError, DptValue};

/// (dpt string accepted by the dispatcher, exact TypeMismatch tag, byte width)
const INT_DPTS: &[(&str, &str, usize)] = &[
    ("6.010", "6.xxx", 1),
    ("7.001", "7.xxx", 2),
    ("8.001", "8.xxx", 2),
    ("12.001", "12.xxx", 4),
    ("13.001", "13.xxx", 4),
    ("14.000", "14.xxx", 4),
];

#[test]
fn int_dpt_wrong_value_type_returns_exact_type_mismatch() {
    for &(dpt, tag, _) in INT_DPTS {
        // `Bool` matches none of I8/U16/I16/U32/I32/F32.
        let err = encode(dpt, DptValue::Bool(true)).unwrap_err();
        assert_eq!(
            err,
            DptError::TypeMismatch { dpt: tag },
            "wrong-type encode for {dpt} must report exact tag {tag}"
        );
    }
}

#[test]
fn int_dpt_wrong_length_returns_exact_invalid_length() {
    for &(dpt, _, n) in INT_DPTS {
        let too_long = vec![0u8; n + 1];
        assert_eq!(
            decode(dpt, &too_long).unwrap_err(),
            DptError::InvalidLength {
                expected: n,
                actual: n + 1,
            },
            "over-long decode for {dpt}"
        );

        let short_len = n - 1; // n >= 1 for all entries; n == 1 -> 0
        let too_short = vec![0u8; short_len];
        assert_eq!(
            decode(dpt, &too_short).unwrap_err(),
            DptError::InvalidLength {
                expected: n,
                actual: short_len,
            },
            "too-short decode for {dpt}"
        );
    }
}
