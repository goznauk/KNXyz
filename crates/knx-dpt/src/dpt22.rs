//! DPT 22.xxx — KNX B16, a 2-octet raw bitset (e.g. 22.100 DHW Controller
//! Status, 22.101 RHCC Status, 22.1000 Media).
//!
//! DECODE-ONLY: the two octets are decoded big-endian to a [`DptValue::Bitset16`]
//! raw `u16` mask. This is the raw mask ONLY — the per-bit meaning is carried by
//! the DPT id and is NOT interpreted here (no named-bit semantics). Every 22.xxx
//! sub is the same 2-octet B16, so they all share this one sub-agnostic codec. No
//! encode is provided — main 22 is intentionally absent from the uniform codec
//! table, so `encode("22.xxx", …)` stays [`DptError::UnsupportedDpt`], and the
//! `Bitset16` variant additionally loud-fails in `knx-ip`'s `encode_value`
//! write-path inference, so a decoded mask can never be silently written.
//!
//! [`DptError::UnsupportedDpt`]: crate::DptError::UnsupportedDpt

use crate::{common, DptValue, Result};

/// Decode a 2-octet DPT 22 (B16) payload into [`DptValue::Bitset16`]. The two
/// octets are big-endian (most-significant first), so `u16::from_be_bytes` is
/// exact. Every bit pattern is a valid raw mask, so the only failure mode is a
/// wrong payload length (`DptError::InvalidLength`).
pub fn decode(bytes: &[u8]) -> Result<DptValue> {
    let bytes = common::be_array::<2>(bytes)?;
    Ok(DptValue::Bitset16(u16::from_be_bytes(bytes)))
}
