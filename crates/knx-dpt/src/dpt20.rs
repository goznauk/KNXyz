//! DPT 20.xxx — 1-octet HVAC enumerations. This module models two
//! distinct DPT 20 sub-types:
//!
//! - **20.102** (HVAC operating mode): 0 = Auto, 1 = Comfort, 2 = Standby,
//!   3 = Economy, 4 = Building Protection. Values outside `0..=4` are
//!   rejected (matching the pinned DPT 20.102 transcoder, which raises on
//!   an unknown byte). Carried by [`DptValue::HvacMode`].
//! - **20.105** (HVAC controller mode): a wider enumeration — 0..=17
//!   (Auto, Heat, Morning Warmup, Cool, Night Purge, Precool, Off, Test,
//!   Emergency Heat, Fan Only, Free Cool, Ice, Maximum Heating Mode,
//!   Economy Heat/Cool Mode, Dehumidification, Calibration Mode, Emergency
//!   Cool Mode, Emergency Steam Mode) plus 20 (NoDem). Bytes 18 and 19 are
//!   KNX-reserved and rejected, as are bytes > 20 — matching both the KNX
//!   spec and the pinned DPT 20.105 transcoder, which raise on an
//!   undefined byte. Carried by [`DptValue::HvacControllerMode`].
//!
//! The crate dispatches codecs by MAIN number, so every `20.xxx` sub-type
//! routes here. Decode is SUB-AWARE: `lib.rs` peels `20.105` off to
//! [`decode_controller_mode`] before the uniform-table `(20, …)` entry
//! (which serves [`decode`], the 20.102 path). Encode discriminates by the
//! `DptValue` variant ([`encode`] accepts both `HvacMode` and
//! `HvacControllerMode`), since the encode dispatch sees only the value,
//! not the sub-number.

use crate::{common, DptError, DptValue, Result};

/// Highest valid DPT 20.102 HVAC operating-mode value (Building Protection).
const HVAC_OPERATION_MODE_MAX: u8 = 4;

/// Highest CONTIGUOUS valid DPT 20.105 controller-mode value (Emergency
/// Steam Mode); 0..=17 are defined.
const HVAC_CONTROLLER_MODE_CONTIGUOUS_MAX: u8 = 17;
/// DPT 20.105 "no demand" — the single defined value above the contiguous
/// run (18 and 19 are KNX-reserved/undefined).
const HVAC_CONTROLLER_MODE_NODEM: u8 = 20;

/// True iff `value` is a defined DPT 20.105 controller mode (0..=17 or 20).
fn is_valid_controller_mode(value: u8) -> bool {
    value <= HVAC_CONTROLLER_MODE_CONTIGUOUS_MAX || value == HVAC_CONTROLLER_MODE_NODEM
}

pub fn encode(value: DptValue) -> Result<std::vec::Vec<u8>> {
    match value {
        DptValue::HvacMode(value) => {
            if value > HVAC_OPERATION_MODE_MAX {
                return Err(DptError::InvalidValue {
                    dpt: "20.102",
                    reason: "HVAC operating mode must be between 0 and 4",
                });
            }
            Ok(std::vec![value])
        }
        DptValue::HvacControllerMode(value) => {
            if !is_valid_controller_mode(value) {
                return Err(DptError::InvalidValue {
                    dpt: "20.105",
                    reason: "HVAC controller mode must be 0..=17 or 20 (18/19 reserved)",
                });
            }
            Ok(std::vec![value])
        }
        _ => Err(DptError::TypeMismatch { dpt: "20.xxx" }),
    }
}

pub fn decode(bytes: &[u8]) -> Result<DptValue> {
    let bytes = common::expect_len(bytes, 1)?;
    if bytes[0] > HVAC_OPERATION_MODE_MAX {
        return Err(DptError::InvalidValue {
            dpt: "20.102",
            reason: "HVAC operating mode must be between 0 and 4",
        });
    }

    Ok(DptValue::HvacMode(bytes[0]))
}

/// Decode a DPT 20.105 HVAC controller-mode byte (sub-aware path).
pub fn decode_controller_mode(bytes: &[u8]) -> Result<DptValue> {
    let bytes = common::expect_len(bytes, 1)?;
    if !is_valid_controller_mode(bytes[0]) {
        return Err(DptError::InvalidValue {
            dpt: "20.105",
            reason: "HVAC controller mode must be 0..=17 or 20 (18/19 reserved)",
        });
    }

    Ok(DptValue::HvacControllerMode(bytes[0]))
}
