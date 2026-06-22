//! DPT 11.xxx — Date. KNX stores the year as a two-digit code: codes
//! `0..=89` map to `2000..=2089`, codes `90..=99` map to `1990..=1999`
//! (pivot at code `89`), so only years `1990..=2089` are representable.
//! Validation is **calendar-naive**: it only range-checks the individual
//! fields (year 1990..=2089, month 1..=12, day 1..=31). Impossible dates
//! such as 2024-02-31 are accepted — this matches the KNX wire format,
//! which carries raw fields with no calendar rule.

use crate::{common, DptError, DptValue, Result};

const YEAR_PIVOT_CODE: u8 = 89;
const YEAR_BASE_2000: u16 = 2000;
const YEAR_BASE_1900: u16 = 1900;
const YEAR_MIN: u16 = 1990;
const YEAR_MAX: u16 = 2089;
const MONTH_MIN: u8 = 1;
const MONTH_MAX: u8 = 12;
const DAY_MIN: u8 = 1;
const DAY_MAX: u8 = 31;

pub fn encode(value: DptValue) -> Result<std::vec::Vec<u8>> {
    let DptValue::Date { year, month, day } = value else {
        return Err(DptError::TypeMismatch { dpt: "11.xxx" });
    };

    validate_date(year, month, day)?;
    Ok(std::vec![day, month, encode_year(year)])
}

pub fn decode(bytes: &[u8]) -> Result<DptValue> {
    let bytes = common::expect_len(bytes, 3)?;

    let day = bytes[0];
    let month = bytes[1];
    let year = decode_year(bytes[2]);
    validate_date(year, month, day)?;

    Ok(DptValue::Date { year, month, day })
}

fn encode_year(year: u16) -> u8 {
    if year >= YEAR_BASE_2000 {
        (year - YEAR_BASE_2000) as u8
    } else {
        (year - YEAR_BASE_1900) as u8
    }
}

fn decode_year(value: u8) -> u16 {
    if value <= YEAR_PIVOT_CODE {
        YEAR_BASE_2000 + u16::from(value)
    } else {
        YEAR_BASE_1900 + u16::from(value)
    }
}

fn validate_date(year: u16, month: u8, day: u8) -> Result<()> {
    if !(YEAR_MIN..=YEAR_MAX).contains(&year) {
        return Err(DptError::InvalidValue {
            dpt: "11.xxx",
            reason: "year must be between 1990 and 2089",
        });
    }
    if !(MONTH_MIN..=MONTH_MAX).contains(&month) {
        return Err(DptError::InvalidValue {
            dpt: "11.xxx",
            reason: "month must be between 1 and 12",
        });
    }
    if !(DAY_MIN..=DAY_MAX).contains(&day) {
        return Err(DptError::InvalidValue {
            dpt: "11.xxx",
            reason: "day must be between 1 and 31",
        });
    }

    Ok(())
}
