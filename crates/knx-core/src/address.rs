use core::fmt;
use core::str::FromStr;

use crate::{KnxError, Result};

// Individual address bit layout: 4-bit area | 4-bit line | 8-bit device.
const IA_AREA_SHIFT: u16 = 12;
const IA_LINE_SHIFT: u16 = 8;
const IA_FIELD_MAX: u8 = 0x0f;
const IA_NIBBLE_MASK: u16 = 0x0f;
const IA_DEVICE_MASK: u16 = 0xff;

// Group address bit layout: 5-bit main | 3-bit middle | 8-bit sub (three-level),
// or 5-bit main | 11-bit sub (two-level).
const GA_MAIN_SHIFT: u16 = 11;
const GA_MAIN_MAX: u8 = 0x1f;
const GA_MAIN_MASK: u16 = 0x1f;
const GA_MIDDLE_SHIFT: u16 = 8;
const GA_MIDDLE_MAX: u8 = 0x07;
const GA_MIDDLE_MASK: u16 = 0x07;
const GA_SUB_MASK: u16 = 0xff;
const GA_TWO_LEVEL_SUB_MAX: u16 = 0x07ff;
const GA_TWO_LEVEL_SUB_MASK: u16 = 0x07ff;

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndividualAddress(u16);

impl IndividualAddress {
    pub const fn new(area: u8, line: u8, device: u8) -> Result<Self> {
        if area > IA_FIELD_MAX {
            return Err(KnxError::InvalidAddress("individual area out of range"));
        }
        if line > IA_FIELD_MAX {
            return Err(KnxError::InvalidAddress("individual line out of range"));
        }

        Ok(Self(
            ((area as u16) << IA_AREA_SHIFT) | ((line as u16) << IA_LINE_SHIFT) | device as u16,
        ))
    }

    // Infallible: every u16 maps to a structurally valid individual address
    // (4-bit area + 4-bit line + 8-bit device covers the full 16-bit space),
    // so there is no invalid input a checked constructor could reject.
    pub const fn from_raw(raw: u16) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u16 {
        self.0
    }

    pub const fn area(self) -> u8 {
        ((self.0 >> IA_AREA_SHIFT) & IA_NIBBLE_MASK) as u8
    }

    pub const fn line(self) -> u8 {
        ((self.0 >> IA_LINE_SHIFT) & IA_NIBBLE_MASK) as u8
    }

    pub const fn device(self) -> u8 {
        (self.0 & IA_DEVICE_MASK) as u8
    }
}

impl fmt::Display for IndividualAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.area(), self.line(), self.device())
    }
}

impl FromStr for IndividualAddress {
    type Err = KnxError;

    fn from_str(value: &str) -> Result<Self> {
        let mut parts = value.split('.');
        let area = parse_part(parts.next(), "missing individual area")?;
        let line = parse_part(parts.next(), "missing individual line")?;
        let device = parse_part(parts.next(), "missing individual device")?;

        if parts.next().is_some() {
            return Err(KnxError::InvalidAddress("too many individual parts"));
        }

        Self::new(area, line, device)
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroupAddress(u16);

impl GroupAddress {
    pub const fn new_three_level(main: u8, middle: u8, sub: u8) -> Result<Self> {
        if main > GA_MAIN_MAX {
            return Err(KnxError::InvalidAddress("group main out of range"));
        }
        if middle > GA_MIDDLE_MAX {
            return Err(KnxError::InvalidAddress("group middle out of range"));
        }

        Ok(Self(
            ((main as u16) << GA_MAIN_SHIFT) | ((middle as u16) << GA_MIDDLE_SHIFT) | sub as u16,
        ))
    }

    pub const fn new_two_level(main: u8, sub: u16) -> Result<Self> {
        if main > GA_MAIN_MAX {
            return Err(KnxError::InvalidAddress("group main out of range"));
        }
        if sub > GA_TWO_LEVEL_SUB_MAX {
            return Err(KnxError::InvalidAddress("group sub out of range"));
        }

        Ok(Self(((main as u16) << GA_MAIN_SHIFT) | sub))
    }

    // Infallible: every u16 maps to a structurally valid group address
    // (5-bit main + 3-bit middle + 8-bit sub, or 5-bit main + 11-bit sub,
    // both covering the full 16-bit space), so a checked constructor would
    // have no invalid input to reject.
    pub const fn from_raw(raw: u16) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u16 {
        self.0
    }

    pub const fn main(self) -> u8 {
        ((self.0 >> GA_MAIN_SHIFT) & GA_MAIN_MASK) as u8
    }

    pub const fn middle(self) -> u8 {
        ((self.0 >> GA_MIDDLE_SHIFT) & GA_MIDDLE_MASK) as u8
    }

    pub const fn sub(self) -> u8 {
        (self.0 & GA_SUB_MASK) as u8
    }

    pub const fn two_level_sub(self) -> u16 {
        self.0 & GA_TWO_LEVEL_SUB_MASK
    }

    pub fn parse_two_level(value: &str) -> Result<Self> {
        let mut parts = value.split('/');
        let main = parse_part(parts.next(), "missing group main")?;
        let sub = parse_part(parts.next(), "missing group sub")?;

        if parts.next().is_some() {
            return Err(KnxError::InvalidAddress("too many group parts"));
        }

        Self::new_two_level(main, sub)
    }

    pub fn to_two_level_display(self) -> TwoLevelGroupAddressDisplay {
        TwoLevelGroupAddressDisplay(self)
    }

    #[cfg(feature = "std")]
    pub fn to_two_level_string(self) -> std::string::String {
        use std::string::ToString;

        self.to_two_level_display().to_string()
    }
}

impl fmt::Display for GroupAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}/{}", self.main(), self.middle(), self.sub())
    }
}

impl FromStr for GroupAddress {
    type Err = KnxError;

    fn from_str(value: &str) -> Result<Self> {
        let mut parts = value.split('/');
        let main = parse_part(parts.next(), "missing group main")?;
        let middle = parse_part(parts.next(), "missing group middle")?;
        let sub = parse_part(parts.next(), "missing group sub")?;

        if parts.next().is_some() {
            return Err(KnxError::InvalidAddress("too many group parts"));
        }

        Self::new_three_level(main, middle, sub)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TwoLevelGroupAddressDisplay(GroupAddress);

impl fmt::Display for TwoLevelGroupAddressDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.0.main(), self.0.two_level_sub())
    }
}

fn parse_part<T: core::str::FromStr>(value: Option<&str>, missing: &'static str) -> Result<T> {
    let value = value.ok_or(KnxError::InvalidAddress(missing))?;
    value
        .parse()
        .map_err(|_| KnxError::InvalidAddress("invalid numeric address part"))
}
