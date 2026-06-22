//! DPT 232.600 — KNX 3-octet RGB colour (R, G, B; each a plain `0..=255`
//! byte, no scaling or sign).
//!
//! Decode + encode are the symmetric 3-octet identity transform (R, G, B), so
//! `232.600` round-trips as the pure [`crate::encode`]/[`crate::decode`] codec.
//! This is the OFFLINE byte transform only — it is NOT a live-write path. Colour
//! **actuation** stays refused: `DptValue::Rgb` is kept in `knx-ip`'s
//! `encode_value` refusal arm (the variant-keyed write-path inference), and main
//! 232 is intentionally absent from the uniform codec table, so the only encode
//! is this explicit dpt-id-keyed arm. So `knx_dpt::encode("232.600", Rgb)`
//! produces the 3 bytes for round-trip/offline use, while every bus write
//! (`group_write`/`group_response`/`send_group_write`) keeps refusing RGB.
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
/// This is the pure offline codec — it does NOT actuate the bus (colour writes
/// stay refused at `knx-ip`'s `encode_value`).
pub fn encode(value: DptValue) -> Result<std::vec::Vec<u8>> {
    let DptValue::Rgb { red, green, blue } = value else {
        return Err(DptError::TypeMismatch { dpt: "232.600" });
    };
    Ok(std::vec![red, green, blue])
}
