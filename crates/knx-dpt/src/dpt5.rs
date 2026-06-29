//! DPT 5.xxx — 8-bit unsigned. Sub-type `5.001` ("Scaling") maps a
//! 0..=100 % value onto a single 0..=255 byte, and `5.003` ("Angle") maps a
//! 0..=360 ° value onto a single 0..=255 byte (`degrees = byte * 360 / 255`).
//! Both mappings are **lossy**: `encode` then `decode` does not generally
//! return the exact input (the fixture tests use a tolerance). `5.003` is
//! decode-only here (wind bearing is a listener-only weather reading). Other
//! 5.xxx sub-types pass the raw byte through unchanged.

use crate::{common, DptError, DptValue, Result};

const SCALE_MAX: f32 = 255.0;
const PERCENT_MAX: f32 = 100.0;
const PERCENT_MIN: f32 = 0.0;
const ANGLE_MAX: f32 = 360.0;

pub fn encode(dpt: &str, value: DptValue) -> Result<std::vec::Vec<u8>> {
    match (dpt, value) {
        ("5.001", DptValue::Scaling(value)) => {
            if !(PERCENT_MIN..=PERCENT_MAX).contains(&value) {
                return Err(DptError::ValueOutOfRange {
                    dpt: "5.001",
                    value,
                });
            }
            Ok(std::vec![((value * SCALE_MAX) / PERCENT_MAX).round() as u8])
        }
        ("5.001", _) => Err(DptError::TypeMismatch { dpt: "5.001" }),
        (_, DptValue::U8(value)) => Ok(std::vec![value]),
        _ => Err(DptError::TypeMismatch { dpt: "5.xxx" }),
    }
}

pub fn decode(dpt: &str, bytes: &[u8]) -> Result<DptValue> {
    let bytes = common::expect_len(bytes, 1)?;

    if dpt == "5.001" {
        Ok(DptValue::Scaling(
            (bytes[0] as f32 * PERCENT_MAX) / SCALE_MAX,
        ))
    } else if dpt == "5.003" {
        Ok(DptValue::Angle((bytes[0] as f32 * ANGLE_MAX) / SCALE_MAX))
    } else {
        Ok(DptValue::U8(bytes[0]))
    }
}
