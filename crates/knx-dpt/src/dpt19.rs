//! DPT 19.xxx — Date+Time. A fixed 8-octet struct:
//!
//! - byte0: year offset from 1900 (`year - 1900`; calendar years
//!   1900..=2155).
//! - byte1: month (1..=12).
//! - byte2: day of month (1..=31).
//! - byte3: `(weekday << 5) | hour` — weekday in the top 3 bits
//!   (0 = "no day", 1=Mon..=7=Sun), hour in the low 5 bits (0..=24; 24
//!   only with minute=second=0), exactly like DPT 10.001's first octet.
//! - byte4: minutes (0..=59).
//! - byte5: seconds (0..=59).
//! - byte6: status/validity flags.
//! - byte7: clock-quality flags.
//!
//! This codec models the SEVEN core temporal fields. Validation is
//! calendar-naive (per-field range checks only), matching the wire
//! format. On `encode` the status byte is fixed to "core fields valid,
//! working-day unknown (no-working-day bit), no fault, no DST, no
//! clock-quality"; the no-day-of-week bit is set when `weekday == 0`.
//! On `decode` a partial value — any of the no-year / no-date / no-time
//! validity bits set — is rejected loudly (only complete datetimes are
//! representable here); the working-day, DST and clock-quality flags do
//! not affect the core fields and are ignored.

use crate::{common, DptError, DptValue, Result};

const YEAR_BASE_1900: u16 = 1900;
const YEAR_MIN: u16 = 1900;
const YEAR_MAX: u16 = 2155;
const MONTH_MIN: u8 = 1;
const MONTH_MAX: u8 = 12;
const DAY_MIN: u8 = 1;
const DAY_MAX: u8 = 31;
const WEEKDAY_MAX: u8 = 7;
const HOUR_MAX: u8 = 24;
const MINUTE_MAX: u8 = 59;
const SECOND_MAX: u8 = 59;

const WEEKDAY_SHIFT: u8 = 5;
const HOUR_MASK: u8 = 0x1f;

// Status byte (byte6) bits.
const STATUS_NO_WORKING_DAY: u8 = 0x20;
const STATUS_NO_DAY_OF_WEEK: u8 = 0x04;
// Validity bits that make the core datetime incomplete.
const STATUS_NO_YEAR: u8 = 0x10;
const STATUS_NO_DATE: u8 = 0x08;
const STATUS_NO_TIME: u8 = 0x02;
const STATUS_INCOMPLETE: u8 = STATUS_NO_YEAR | STATUS_NO_DATE | STATUS_NO_TIME;

pub fn encode(value: DptValue) -> Result<std::vec::Vec<u8>> {
    let DptValue::DateTime {
        year,
        month,
        day,
        weekday,
        hour,
        minute,
        second,
    } = value
    else {
        return Err(DptError::TypeMismatch { dpt: "19.xxx" });
    };

    validate(year, month, day, weekday, hour, minute, second)?;

    let year_byte = (year - YEAR_BASE_1900) as u8;
    let dow_hour = (weekday << WEEKDAY_SHIFT) | hour;
    // working-day is not modeled; mark it unknown. The day-of-week field
    // is only valid when a concrete weekday (1..=7) is carried.
    let mut status = STATUS_NO_WORKING_DAY;
    if weekday == 0 {
        status |= STATUS_NO_DAY_OF_WEEK;
    }

    Ok(std::vec![
        year_byte, month, day, dow_hour, minute, second, status, 0x00,
    ])
}

pub fn decode(bytes: &[u8]) -> Result<DptValue> {
    let bytes = common::expect_len(bytes, 8)?;

    let status = bytes[6];
    if status & STATUS_INCOMPLETE != 0 {
        return Err(DptError::InvalidValue {
            dpt: "19.xxx",
            reason: "partial datetime (no-year/no-date/no-time) is not supported",
        });
    }

    let year = YEAR_BASE_1900 + u16::from(bytes[0]);
    let month = bytes[1];
    let day = bytes[2];
    let weekday = if status & STATUS_NO_DAY_OF_WEEK != 0 {
        0
    } else {
        bytes[3] >> WEEKDAY_SHIFT
    };
    let hour = bytes[3] & HOUR_MASK;
    let minute = bytes[4];
    let second = bytes[5];
    validate(year, month, day, weekday, hour, minute, second)?;

    Ok(DptValue::DateTime {
        year,
        month,
        day,
        weekday,
        hour,
        minute,
        second,
    })
}

#[allow(clippy::too_many_arguments)]
fn validate(
    year: u16,
    month: u8,
    day: u8,
    weekday: u8,
    hour: u8,
    minute: u8,
    second: u8,
) -> Result<()> {
    if !(YEAR_MIN..=YEAR_MAX).contains(&year) {
        return Err(DptError::InvalidValue {
            dpt: "19.xxx",
            reason: "year must be between 1900 and 2155",
        });
    }
    if !(MONTH_MIN..=MONTH_MAX).contains(&month) {
        return Err(DptError::InvalidValue {
            dpt: "19.xxx",
            reason: "month must be between 1 and 12",
        });
    }
    if !(DAY_MIN..=DAY_MAX).contains(&day) {
        return Err(DptError::InvalidValue {
            dpt: "19.xxx",
            reason: "day must be between 1 and 31",
        });
    }
    if weekday > WEEKDAY_MAX {
        return Err(DptError::InvalidValue {
            dpt: "19.xxx",
            reason: "weekday must be between 0 and 7",
        });
    }
    if hour > HOUR_MAX {
        return Err(DptError::InvalidValue {
            dpt: "19.xxx",
            reason: "hour must be between 0 and 24",
        });
    }
    if minute > MINUTE_MAX {
        return Err(DptError::InvalidValue {
            dpt: "19.xxx",
            reason: "minute must be between 0 and 59",
        });
    }
    if second > SECOND_MAX {
        return Err(DptError::InvalidValue {
            dpt: "19.xxx",
            reason: "second must be between 0 and 59",
        });
    }
    if hour == HOUR_MAX && (minute != 0 || second != 0) {
        return Err(DptError::InvalidValue {
            dpt: "19.xxx",
            reason: "hour 24 requires minute and second to be 0",
        });
    }

    Ok(())
}
