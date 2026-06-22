use crate::{common, DptError, DptValue, Result};

pub fn encode(value: DptValue) -> Result<std::vec::Vec<u8>> {
    let DptValue::StepControl {
        increase,
        step_code,
    } = value
    else {
        return Err(DptError::TypeMismatch { dpt: "3.xxx" });
    };

    if step_code > 7 {
        return Err(DptError::InvalidValue {
            dpt: "3.xxx",
            reason: "step code must be between 0 and 7",
        });
    }

    Ok(std::vec![(u8::from(increase) << 3) | step_code])
}

pub fn decode(bytes: &[u8]) -> Result<DptValue> {
    let bytes = common::expect_len(bytes, 1)?;
    if bytes[0] & !0x0f != 0 {
        return Err(DptError::InvalidValue {
            dpt: "3.xxx",
            reason: "step control payload must fit in four bits",
        });
    }

    Ok(DptValue::StepControl {
        increase: bytes[0] & 0x08 != 0,
        step_code: bytes[0] & 0x07,
    })
}
