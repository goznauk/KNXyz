//! DPT 232.600 — KNX 3-octet RGB colour (R, G, B; each a plain `0..=255`
//! byte, no scaling or sign).
//!
//! Decode + encode are the symmetric 3-octet identity transform (R, G, B), so
//! `232.600` round-trips as the pure [`crate::encode`]/[`crate::decode`] codec.
//! `DptValue::Rgb` is kept in `knx-ip`'s `encode_value` refusal arm, and main
//! 232 is intentionally absent from the uniform codec table, so the only encode
//! path is this explicit dpt-id-keyed arm.
//!
//! [`DptError::UnsupportedDpt`]: crate::DptError::UnsupportedDpt

use crate::{common, DptError, DptValue, Result};

/// Decode a 3-octet DPT 232.600 RGB payload (byte order R, G, B) into
/// [`DptValue::Rgb`]. Every byte value is in range (`0..=255`), so the only
/// failure mode is a wrong payload length (`DptError::InvalidLength`).
pub fn decode(bytes: &[u8]) -> Result<DptValue> {
    let [red, green, blue] = common::be_array::<3>(bytes)?;
    Ok(DptValue::Rgb { red, green, blue })
}

/// Encode a [`DptValue::Rgb`] to its 3-octet payload (R, G, B). Every channel is
/// a plain `0..=255` byte (no scaling/sign), so this cannot fail on range; the
/// only failure is a wrong [`DptValue`] variant ([`DptError::TypeMismatch`]).
/// Colour writes stay refused at `knx-ip`'s `encode_value`.
pub fn encode(value: DptValue) -> Result<std::vec::Vec<u8>> {
    let DptValue::Rgb { red, green, blue } = value else {
        return Err(DptError::TypeMismatch { dpt: "232.600" });
    };
    Ok(std::vec![red, green, blue])
}
