use crate::{common, DptError, DptValue, Result};

const WEEKDAY_SHIFT: u8 = 5;
const HOUR_MASK: u8 = 0x1f;
// Field upper bounds. weekday 0 means "no day" (1..=7 = Mon..=Sun), so 0 is
// intentionally accepted; only the upper bound is range-checked.
const WEEKDAY_MAX: u8 = 7;
const HOUR_MAX: u8 = 23;
const MINUTE_MAX: u8 = 59;
const SECOND_MAX: u8 = 59;

pub fn encode(value: DptValue) -> Result<std::vec::Vec<u8>> {
    let DptValue::Time {
        weekday,
        hour,
        minute,
        second,
    } = value
    else {
        return Err(DptError::TypeMismatch { dpt: "10.xxx" });
    };

    validate_time(weekday, hour, minute, second)?;
    Ok(std::vec![(weekday << WEEKDAY_SHIFT) | hour, minute, second])
}

pub fn decode(bytes: &[u8]) -> Result<DptValue> {
    let bytes = common::expect_len(bytes, 3)?;

    let weekday = bytes[0] >> WEEKDAY_SHIFT;
    let hour = bytes[0] & HOUR_MASK;
    let minute = bytes[1];
    let second = bytes[2];
    validate_time(weekday, hour, minute, second)?;

    Ok(DptValue::Time {
        weekday,
        hour,
        minute,
        second,
    })
}

fn validate_time(weekday: u8, hour: u8, minute: u8, second: u8) -> Result<()> {
    if weekday > WEEKDAY_MAX {
        return Err(DptError::InvalidValue {
            dpt: "10.xxx",
            reason: "weekday must be between 0 and 7",
        });
    }
    if hour > HOUR_MAX {
        return Err(DptError::InvalidValue {
            dpt: "10.xxx",
            reason: "hour must be between 0 and 23",
        });
    }
    if minute > MINUTE_MAX {
        return Err(DptError::InvalidValue {
            dpt: "10.xxx",
            reason: "minute must be between 0 and 59",
        });
    }
    if second > SECOND_MAX {
        return Err(DptError::InvalidValue {
            dpt: "10.xxx",
            reason: "second must be between 0 and 59",
        });
    }

    Ok(())
}
