//! DPT 13.xxx — 32-bit signed integer codec.
//!
//! Every `13.xxx` sub uses this uniform `I32` codec except the four switched
//! energy subs `13.010` (active energy, Wh) / `13.013` (active energy, kWh) /
//! `13.014` (apparent energy, VAh) / `13.015` (reactive energy, VARh), which use
//! the dedicated [`decode_energy`]/[`encode_energy`] pair below (peeled off
//! before the uniform table in `lib.rs` via a `matches!(id.sub(), 10 | 13 | 14 |
//! 15)` guard). The switch only changes the value tag
//! (`I32` → `EnergyI32`); the raw 4-octet signed big-endian payload is identical,
//! and the unit (Wh/kWh/VAh/VARh) is still carried by the DPT id and discarded.
//! This lets an energy reading be distinguished from the dimensionless counter
//! `13.001`. The helpers are sub-agnostic (they take bytes/value, not the id),
//! so one pair serves all four subs. `EnergyI32` is kept in knx-ip's
//! `encode_value` variant-keyed refusal arm, so this codec is limited to payload
//! conversion.
impl_int_dpt!("13.xxx", I32, i32, 4);

/// Decode a switched DPT13 energy sub (`13.010`/`13.013`/`13.014`/`13.015`)
/// into [`crate::DptValue::EnergyI32`]. Other `13.xxx` subtypes keep the
/// generic `I32` value tag. The raw value is the identical signed `i32`; only
/// the type tag changes. The sole failure mode is a wrong payload length
/// ([`crate::DptError::InvalidLength`]).
pub fn decode_energy(bytes: &[u8]) -> crate::Result<crate::DptValue> {
    let bytes = crate::common::be_array::<4>(bytes)?;
    Ok(crate::DptValue::EnergyI32(i32::from_be_bytes(bytes)))
}

/// Encode a switched DPT13 energy sub, symmetric with [`decode_energy`].
///
/// Accepts both [`crate::DptValue::EnergyI32`] (the energy-tagged variant,
/// symmetric with the decode side) and [`crate::DptValue::I32`] (backward
/// compatibility: existing callers may still feed a generic `I32` for main 13,
/// and both encode to the identical 4 bytes). Every other variant, including
/// [`crate::DptValue::EnergyU32`] (main 13 is signed `V32`) returns
/// [`crate::DptError::TypeMismatch`]. The refusal tag is the sub-agnostic
/// `"13.xxx"` (this arm serves all four energy subs), matching the uniform
/// macro's tag used by the non-selected subs.
pub fn encode_energy(value: crate::DptValue) -> crate::Result<std::vec::Vec<u8>> {
    match value {
        crate::DptValue::EnergyI32(value) | crate::DptValue::I32(value) => {
            Ok(value.to_be_bytes().to_vec())
        }
        _ => Err(crate::DptError::TypeMismatch { dpt: "13.xxx" }),
    }
}
