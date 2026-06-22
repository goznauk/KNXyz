use crate::{common, DptError, DptValue, Result};

pub fn encode(value: DptValue) -> Result<std::vec::Vec<u8>> {
    match value {
        DptValue::Bool(value) => Ok(std::vec![u8::from(value)]),
        _ => Err(DptError::TypeMismatch { dpt: "1.xxx" }),
    }
}

pub fn decode(bytes: &[u8]) -> Result<DptValue> {
    let bytes = common::expect_len(bytes, 1)?;

    match bytes[0] {
        0x00 => Ok(DptValue::Bool(false)),
        0x01 => Ok(DptValue::Bool(true)),
        _ => Err(DptError::InvalidValue {
            dpt: "1.xxx",
            reason: "boolean payload must be 0 or 1",
        }),
    }
}
