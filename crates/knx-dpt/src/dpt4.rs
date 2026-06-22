//! DPT 4.xxx — KNX single character, DECODE-ONLY.
//!
//! `4.001` is ASCII (a 7-bit code point, 0x00..=0x7F); `4.002` is ISO-8859-1
//! (Latin-1, the full octet 0x00..=0xFF). Both are a single octet decoded to
//! [`DptValue::Char`]. The character set is carried by the DPT id, never the
//! value — so the two subs use separate decode paths: `4.001` range-checks the
//! 7-bit constraint, `4.002` accepts every byte via the Latin-1 → Unicode
//! identity (`char::from(u8)` maps a byte to U+0000..U+00FF, exactly Latin-1).
//!
//! No encode is provided — DPT main 4 is intentionally absent from the uniform
//! codec table (so `encode("4.xxx", …)` stays [`DptError::UnsupportedDpt`]), and
//! [`DptValue::Char`] returns an explicit error in `knx-ip`'s `encode_value` write-path
//! inference, so a decoded character can never be silently written to a wrong
//! main (a U8 reuse could not provide that isolation).
//!
//! [`DptError::UnsupportedDpt`]: crate::DptError::UnsupportedDpt

use crate::{common, DptError, DptValue, Result};

/// Decode a 1-octet DPT 4.001 ASCII character. Bytes above `0x7F` are not valid
/// 7-bit ASCII and loud-fail with [`DptError::InvalidValue`] (the length is
/// fine, so it is NOT `InvalidLength`; the DPT is supported, so NOT
/// `UnsupportedDpt`).
pub fn decode_ascii(bytes: &[u8]) -> Result<DptValue> {
    let [byte] = common::be_array::<1>(bytes)?;
    if byte > 0x7F {
        return Err(DptError::InvalidValue {
            dpt: "4.001",
            reason: "character must be 7-bit ASCII (0x00..=0x7F)",
        });
    }
    Ok(DptValue::Char(char::from(byte)))
}

/// Decode a 1-octet DPT 4.002 ISO-8859-1 (Latin-1) character. Every byte is a
/// valid Latin-1 code point (`char::from(u8)` is total), so the only failure is
/// a wrong payload length.
pub fn decode_latin1(bytes: &[u8]) -> Result<DptValue> {
    let [byte] = common::be_array::<1>(bytes)?;
    Ok(DptValue::Char(char::from(byte)))
}
