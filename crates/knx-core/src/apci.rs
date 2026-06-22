use core::convert::TryFrom;

use crate::{KnxError, Result};

const APCI_MASK: u8 = 0xc0;
const GROUP_VALUE_READ_BITS: u8 = 0x00;
const GROUP_VALUE_RESPONSE_BITS: u8 = 0x40;
const GROUP_VALUE_WRITE_BITS: u8 = 0x80;

/// Only group-value services are modeled; all other APCI codes decode as `InvalidFrame`
/// (intentional scope limit).
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Apci {
    GroupValueRead,
    GroupValueResponse,
    GroupValueWrite,
}

impl Apci {
    pub const fn service_bits(self) -> u8 {
        match self {
            Self::GroupValueRead => GROUP_VALUE_READ_BITS,
            Self::GroupValueResponse => GROUP_VALUE_RESPONSE_BITS,
            Self::GroupValueWrite => GROUP_VALUE_WRITE_BITS,
        }
    }
}

impl TryFrom<u8> for Apci {
    type Error = KnxError;

    fn try_from(value: u8) -> Result<Self> {
        match value & APCI_MASK {
            GROUP_VALUE_READ_BITS => Ok(Self::GroupValueRead),
            GROUP_VALUE_RESPONSE_BITS => Ok(Self::GroupValueResponse),
            GROUP_VALUE_WRITE_BITS => Ok(Self::GroupValueWrite),
            _ => Err(KnxError::InvalidFrame("unsupported APCI service")),
        }
    }
}
