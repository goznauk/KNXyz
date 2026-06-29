use crate::{DptError, Result};

pub fn expect_len(bytes: &[u8], expected: usize) -> Result<&[u8]> {
    if bytes.len() != expected {
        return Err(DptError::InvalidLength {
            expected,
            actual: bytes.len(),
        });
    }

    Ok(bytes)
}

/// Length-checks `bytes` and copies it into a fixed `[u8; N]`.
///
/// Convenience over [`expect_len`] for fixed-width decoders: on a length
/// mismatch it returns the identical `DptError::InvalidLength { expected: N,
/// actual }`. The copy cannot panic because `expect_len` guarantees
/// `bytes.len() == N` before it runs.
pub fn be_array<const N: usize>(bytes: &[u8]) -> Result<[u8; N]> {
    let bytes = expect_len(bytes, N)?;
    let mut array = [0u8; N];
    array.copy_from_slice(bytes);
    Ok(array)
}

#[cfg(test)]
mod tests {
    //! Characterization unit tests for the crate-internal `expect_len` helper.
    //! These assert current behavior; no production logic is changed.
    use super::{be_array, expect_len};
    use crate::error::DptError;

    #[test]
    fn exact_length_returns_ok_with_same_full_slice() {
        let input = [0x10u8, 0x20, 0x30];
        let out = expect_len(&input, 3).expect("exact length must be Ok");
        // Returns the same full slice unchanged.
        assert_eq!(out, &input[..]);
        assert_eq!(out.len(), 3);
    }

    #[test]
    fn shorter_input_returns_invalid_length() {
        let input = [0x01u8, 0x02];
        let err = expect_len(&input, 4).unwrap_err();
        assert_eq!(
            err,
            DptError::InvalidLength {
                expected: 4,
                actual: 2,
            }
        );
    }

    #[test]
    fn longer_input_returns_invalid_length() {
        let input = [0x01u8, 0x02, 0x03, 0x04, 0x05];
        let err = expect_len(&input, 2).unwrap_err();
        assert_eq!(
            err,
            DptError::InvalidLength {
                expected: 2,
                actual: 5,
            }
        );
    }

    #[test]
    fn empty_input_with_positive_expected_returns_invalid_length() {
        let input: [u8; 0] = [];
        let err = expect_len(&input, 3).unwrap_err();
        assert_eq!(
            err,
            DptError::InvalidLength {
                expected: 3,
                actual: 0,
            }
        );
    }

    #[test]
    fn empty_input_with_zero_expected_returns_ok_empty_slice() {
        let input: [u8; 0] = [];
        let out = expect_len(&input, 0).expect("0-len with expected 0 must be Ok");
        assert_eq!(out, &[] as &[u8]);
        assert_eq!(out.len(), 0);
    }

    #[test]
    fn be_array_exact_length_returns_array() {
        let input = [0x10u8, 0x20, 0x30, 0x40];
        let out = be_array::<4>(&input).expect("exact length must be Ok");
        assert_eq!(out, [0x10, 0x20, 0x30, 0x40]);
    }

    #[test]
    fn be_array_too_short_returns_invalid_length() {
        let input = [0x01u8, 0x02];
        let err = be_array::<4>(&input).unwrap_err();
        assert_eq!(
            err,
            DptError::InvalidLength {
                expected: 4,
                actual: 2,
            }
        );
    }

    #[test]
    fn be_array_too_long_returns_invalid_length() {
        let input = [0x01u8, 0x02, 0x03, 0x04, 0x05];
        let err = be_array::<2>(&input).unwrap_err();
        assert_eq!(
            err,
            DptError::InvalidLength {
                expected: 2,
                actual: 5,
            }
        );
    }

    #[test]
    fn be_array_zero_length_returns_empty_array() {
        let input: [u8; 0] = [];
        let out = be_array::<0>(&input).expect("0-len with expected 0 must be Ok");
        assert_eq!(out, [] as [u8; 0]);
    }
}
