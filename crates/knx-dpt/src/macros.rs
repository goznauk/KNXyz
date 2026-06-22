//! Internal codegen for the pure fixed-width integer DPT codecs.
//!
//! dpt6/dpt7/dpt8/dpt12/dpt13 were byte-for-byte clones differing only in
//! the DPT tag, the `DptValue` variant, the integer type, and the byte
//! width. `impl_int_dpt!` generates their `encode`/`decode` so the single
//! source of truth is the one-line invocation in each module.
//!
//! Behavior is identical to the previous hand-written modules:
//! - `encode` returns `<int>::to_be_bytes().to_vec()` (for the 1-byte
//!   signed case this is bit-identical to the old `value as u8`).
//! - `decode` length-checks via `common::be_array::<N>` (identical
//!   `DptError::InvalidLength { expected: N, actual }`) and reconstructs
//!   with `<int>::from_be_bytes` (bit-identical to the old manual
//!   `[bytes[0], ..]` indexing / `bytes[0] as i8`).
//! - wrong `DptValue` yields the exact `DptError::TypeMismatch { dpt }`.

/// Generate the `encode`/`decode` pair for a pure fixed-width integer DPT.
///
/// `impl_int_dpt!("7.xxx", U16, u16, 2);`
macro_rules! impl_int_dpt {
    ($tag:literal, $variant:ident, $int:ty, $len:literal) => {
        pub fn encode(value: $crate::DptValue) -> $crate::Result<std::vec::Vec<u8>> {
            match value {
                $crate::DptValue::$variant(value) => Ok(value.to_be_bytes().to_vec()),
                _ => Err($crate::DptError::TypeMismatch { dpt: $tag }),
            }
        }

        pub fn decode(bytes: &[u8]) -> $crate::Result<$crate::DptValue> {
            let bytes = $crate::common::be_array::<$len>(bytes)?;
            Ok($crate::DptValue::$variant(<$int>::from_be_bytes(bytes)))
        }
    };
}
