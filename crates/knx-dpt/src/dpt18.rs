use crate::{common, DptError, DptValue, Result};

/// Highest valid scene number (6-bit field); shared bound with DPT 17.
const SCENE_MAX: u8 = 63;

pub fn encode(value: DptValue) -> Result<std::vec::Vec<u8>> {
    let DptValue::SceneControl { learn, scene } = value else {
        return Err(DptError::TypeMismatch { dpt: "18.xxx" });
    };
    if scene > SCENE_MAX {
        return Err(DptError::InvalidValue {
            dpt: "18.xxx",
            reason: "scene number must be between 0 and 63",
        });
    }

    Ok(std::vec![(u8::from(learn) << 7) | scene])
}

pub fn decode(bytes: &[u8]) -> Result<DptValue> {
    let bytes = common::expect_len(bytes, 1)?;
    if bytes[0] & 0x40 != 0 {
        return Err(DptError::InvalidValue {
            dpt: "18.xxx",
            reason: "reserved scene control bit must be unset",
        });
    }

    Ok(DptValue::SceneControl {
        learn: bytes[0] & 0x80 != 0,
        scene: bytes[0] & 0x3f,
    })
}
