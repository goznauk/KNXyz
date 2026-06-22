//! Regression coverage for the `serde` feature on `DptValue`.
//!
//! `DptValue::Text14` embeds a `String`, whose serde impls require serde's
//! `alloc` feature. This test fails to compile (and thus regresses) if the
//! `knx-dpt/serde` feature ever stops enabling `serde/alloc`.
//!
//! Gated on `std` as well because `DptValue` is `#[cfg(feature = "std")]`
//! in `knx-dpt` today.
#![cfg(all(feature = "serde", feature = "std"))]

use knx_dpt::DptValue;

#[test]
fn non_alloc_value_serde_json_roundtrips() {
    let value = DptValue::Bool(true);
    let json = serde_json::to_string(&value).unwrap();
    let decoded: DptValue = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, value);
}

#[test]
fn text_value_serde_json_roundtrips() {
    // The alloc-backed variant that previously failed to compile under the
    // `serde` feature without `serde/alloc`.
    let value = DptValue::Text14("hello".to_owned());
    let json = serde_json::to_string(&value).unwrap();
    let decoded: DptValue = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, value);
}
