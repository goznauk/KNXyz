use crate::{common, DptError, DptValue, Result};

const TEXT_LEN: usize = 14;

pub fn encode(value: DptValue) -> Result<std::vec::Vec<u8>> {
    let DptValue::Text14(value) = value else {
        return Err(DptError::TypeMismatch { dpt: "16.xxx" });
    };

    if !value.is_ascii() {
        return Err(DptError::InvalidValue {
            dpt: "16.xxx",
            reason: "text must be ASCII",
        });
    }

    let bytes = value.as_bytes();
    if bytes.len() > TEXT_LEN {
        return Err(DptError::InvalidValue {
            dpt: "16.xxx",
            reason: "text must fit in 14 bytes",
        });
    }

    let mut encoded = std::vec![0; TEXT_LEN];
    encoded[..bytes.len()].copy_from_slice(bytes);
    Ok(encoded)
}

pub fn decode(bytes: &[u8]) -> Result<DptValue> {
    let bytes = common::expect_len(bytes, TEXT_LEN)?;
    if bytes.iter().any(|byte| !byte.is_ascii()) {
        return Err(DptError::InvalidValue {
            dpt: "16.xxx",
            reason: "text payload must be ASCII",
        });
    }

    let end = bytes
        .iter()
        .rposition(|byte| *byte != 0)
        .map_or(0, |index| index + 1);
    let value = std::string::String::from_utf8(bytes[..end].to_vec()).map_err(|_| {
        DptError::InvalidValue {
            dpt: "16.xxx",
            reason: "text payload must be valid UTF-8",
        }
    })?;

    Ok(DptValue::Text14(value))
}
