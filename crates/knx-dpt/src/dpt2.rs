use crate::{common, DptError, DptValue, Result};

pub fn encode(value: DptValue) -> Result<std::vec::Vec<u8>> {
    match value {
        DptValue::ControlBool { control, value } => {
            Ok(std::vec![(u8::from(control) << 1) | u8::from(value)])
        }
        _ => Err(DptError::TypeMismatch { dpt: "2.xxx" }),
    }
}

pub fn decode(bytes: &[u8]) -> Result<DptValue> {
    let bytes = common::expect_len(bytes, 1)?;
    if bytes[0] & !0x03 != 0 {
        return Err(DptError::InvalidValue {
            dpt: "2.xxx",
            reason: "controlled boolean payload must fit in two bits",
        });
    }

    Ok(DptValue::ControlBool {
        control: bytes[0] & 0x02 != 0,
        value: bytes[0] & 0x01 != 0,
    })
}
