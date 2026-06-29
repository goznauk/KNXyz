//! DPT 22.xxx — KNX B16, a 2-octet raw bitset (e.g. 22.100 DHW Controller
//! Status, 22.101 RHCC Status, 22.1000 Media).
//!
//! Decoding is supported; encoding is not. The two octets are decoded big-endian
//! to a [`DptValue::Bitset16`] raw `u16` mask. The per-bit meaning is carried by
//! the DPT id and is not interpreted here, so every 22.xxx sub shares this
//! sub-agnostic codec. Main 22 is intentionally absent from the uniform codec
//! table, so `encode("22.xxx", …)` returns [`DptError::UnsupportedDpt`], and
//! the `Bitset16` variant is rejected by `knx-ip` write-path inference.
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
