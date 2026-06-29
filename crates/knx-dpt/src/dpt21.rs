//! DPT 21.xxx — KNX B8, a 1-octet raw bitset (e.g. 21.001 General Status,
//! 21.002 Device Control, 21.100 Forced Heating).
//!
//! Decoding is supported; encoding is not. The single octet is decoded to a
//! [`DptValue::Bitset8`] raw `u8` mask. The per-bit meaning is carried by the
//! DPT id and is not interpreted here, so every 21.xxx sub shares this
//! sub-agnostic codec. Main 21 is intentionally absent from the uniform codec
//! table, so `encode("21.xxx", …)` returns [`DptError::UnsupportedDpt`], and
//! the `Bitset8` variant is rejected by `knx-ip` write-path inference.
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
