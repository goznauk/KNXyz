//! Regression coverage for the `serde` feature on cEMI payload types.
//!
//! These types embed `Vec<u8>`, whose serde impls require serde's `alloc`
//! feature. This test fails to compile (and thus regresses) if the
//! `knx-core/serde` feature ever stops enabling `serde/alloc`.
//!
//! Gated on `std` as well because the cEMI types are `#[cfg(feature =
//! "std")]` in `knx-core` today.
#![cfg(all(feature = "serde", feature = "std"))]

use knx_core::{Apci, CemiFrame, GroupTelegram};

#[test]
fn group_telegram_serde_json_roundtrips() {
    let source = "1.1.4".parse().unwrap();
    let destination = "1/2/3".parse().unwrap();
    let telegram =
        GroupTelegram::new(source, destination, Apci::GroupValueWrite, &[0x12, 0x34]).unwrap();

    let json = serde_json::to_string(&telegram).unwrap();
    let decoded: GroupTelegram = serde_json::from_str(&json).unwrap();

    assert_eq!(decoded, telegram);
    assert_eq!(decoded.payload(), &[0x12, 0x34]);
}

#[test]
fn cemi_frame_serde_json_roundtrips() {
    let source = "1.1.4".parse().unwrap();
    let destination = "1/2/3".parse().unwrap();
    let frame = CemiFrame::group_value_write(source, destination, &[0xab, 0xcd]).unwrap();

    let json = serde_json::to_string(&frame).unwrap();
    let decoded: CemiFrame = serde_json::from_str(&json).unwrap();

    assert_eq!(decoded, frame);
}
