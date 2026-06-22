use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KnxError {
    InvalidAddress(&'static str),
    BufferTooShort { needed: usize, actual: usize },
    InvalidFrame(&'static str),
    UnsupportedServiceType(u16),
}

impl fmt::Display for KnxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAddress(reason) => write!(f, "invalid address: {reason}"),
            Self::BufferTooShort { needed, actual } => {
                write!(f, "buffer too short: needed {needed} bytes, got {actual}")
            }
            Self::InvalidFrame(reason) => write!(f, "invalid frame: {reason}"),
            Self::UnsupportedServiceType(service_type) => {
                write!(f, "unsupported service type: 0x{service_type:04x}")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for KnxError {}

pub type Result<T> = core::result::Result<T, KnxError>;
