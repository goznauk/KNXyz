//! DPT 29.xxx — KNX V64, an 8-octet two's-complement signed integer
//! (29.010 active energy Wh, 29.011 apparent energy VAh, 29.012 reactive
//! energy VARh).
//!
//! Decoding is supported; encoding is not. The 8 octets are decoded big-endian
//! to [`DptValue::I64`]. DPT main 29 is intentionally absent from the uniform
//! codec table, so `encode("29.xxx", …)` returns
//! [`DptError::UnsupportedDpt`], and the `I64` variant is rejected by `knx-ip`
//! write-path inference. The unit (Wh / VAh / VARh) is carried by the DPT id,
//! never the value, so every 29.xxx sub shares this one codec.
//!
//! [`DptError::UnsupportedDpt`]: crate::DptError::UnsupportedDpt

use crate::{common, DptValue, Result};

/// Decode an 8-octet DPT 29 (V64) payload into [`DptValue::I64`]. The bytes are
/// big-endian two's-complement, so `i64::from_be_bytes` is exact over the full
/// signed range (no manual sign handling). The only failure mode is a wrong
/// payload length (`DptError::InvalidLength`); every 8-byte pattern is a valid
/// signed `i64`.
pub fn decode(bytes: &[u8]) -> Result<DptValue> {
    let bytes = common::be_array::<8>(bytes)?;
    Ok(DptValue::I64(i64::from_be_bytes(bytes)))
}
