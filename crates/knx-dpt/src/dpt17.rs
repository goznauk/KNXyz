use crate::{common, DptError, DptValue, Result};

/// Highest valid scene number (6-bit field); shared bound with DPT 18.
const SCENE_MAX: u8 = 63;

pub fn encode(value: DptValue) -> Result<std::vec::Vec<u8>> {
    let DptValue::SceneNumber(value) = value else {
        return Err(DptError::TypeMismatch { dpt: "17.xxx" });
    };
    if value > SCENE_MAX {
        return Err(DptError::InvalidValue {
            dpt: "17.xxx",
            reason: "scene number must be between 0 and 63",
        });
    }

    Ok(std::vec![value])
}

pub fn decode(bytes: &[u8]) -> Result<DptValue> {
    let bytes = common::expect_len(bytes, 1)?;
    if bytes[0] > SCENE_MAX {
        return Err(DptError::InvalidValue {
            dpt: "17.xxx",
            reason: "scene number must be between 0 and 63",
        });
    }

    Ok(DptValue::SceneNumber(bytes[0]))
}
