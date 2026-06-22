#[derive(Debug, thiserror::Error, Clone, PartialEq)]
pub enum DptError {
    #[error("unsupported datapoint type: {0}")]
    UnsupportedDpt(std::string::String),
    #[error("invalid value for {dpt}: {reason}")]
    InvalidValue {
        dpt: &'static str,
        reason: &'static str,
    },
    #[error("value out of range for {dpt}: {value}")]
    ValueOutOfRange { dpt: &'static str, value: f32 },
    #[error("invalid payload length: expected {expected}, got {actual}")]
    InvalidLength { expected: usize, actual: usize },
    #[error("datapoint value type does not match {dpt}")]
    TypeMismatch { dpt: &'static str },
}

pub type Result<T> = core::result::Result<T, DptError>;
