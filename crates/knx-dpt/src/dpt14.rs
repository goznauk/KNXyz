use crate::{common, DptError, DptValue, Result};

pub fn encode(value: DptValue) -> Result<std::vec::Vec<u8>> {
    let DptValue::F32(value) = value else {
        return Err(DptError::TypeMismatch { dpt: "14.xxx" });
    };

    if !value.is_finite() {
        return Err(DptError::InvalidValue {
            dpt: "14.xxx",
            reason: "float value must be finite",
        });
    }

    Ok(value.to_bits().to_be_bytes().to_vec())
}

pub fn decode(bytes: &[u8]) -> Result<DptValue> {
    let bytes = common::be_array::<4>(bytes)?;

    Ok(DptValue::F32(f32::from_bits(u32::from_be_bytes(bytes))))
}
