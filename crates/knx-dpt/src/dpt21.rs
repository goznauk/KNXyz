//! DPT 21.xxx — KNX B8, a 1-octet raw bitset (e.g. 21.001 General Status,
//! 21.002 Device Control, 21.100 Forced Heating).
//!
//! DECODE-ONLY: the single octet is decoded to a [`DptValue::Bitset8`] raw `u8`
//! mask. This is the raw mask ONLY — the per-bit meaning is carried by the DPT
//! id and is NOT interpreted here (no named-bit semantics). Every 21.xxx sub is
//! the same 1-octet B8, so they all share this one sub-agnostic codec. No encode
//! is provided — main 21 is intentionally absent from the uniform codec table,
//! so `encode("21.xxx", …)` stays [`DptError::UnsupportedDpt`], and the
//! `Bitset8` variant additionally loud-fails in `knx-ip`'s `encode_value`
//! write-path inference, so a decoded mask can never be silently written.
//!
//! [`DptError::UnsupportedDpt`]: crate::DptError::UnsupportedDpt

use crate::{common, DptValue, Result};

/// Decode a 1-octet DPT 21 (B8) payload into [`DptValue::Bitset8`]. Every bit
/// pattern is a valid raw mask, so the only failure mode is a wrong payload
/// length (`DptError::InvalidLength`).
pub fn decode(bytes: &[u8]) -> Result<DptValue> {
    let [byte] = common::be_array::<1>(bytes)?;
    Ok(DptValue::Bitset8(byte))
}
