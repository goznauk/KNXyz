//! DPT 9.xxx — KNX 2-octet float (4-bit exponent · 11-bit **two's-complement**
//! signed mantissa, resolution 0.01): `value = 0.01 · mantissa · 2^exponent`,
//! with the mantissa in `[-2048, 2047]` (bit 15 is the sign extension of the
//! 11-bit field). The format is **quantized**: encode picks the smallest
//! exponent whose mantissa fits, so `encode` then `decode` is only
//! approximate (tests use a magnitude-scaled tolerance). Non-finite inputs
//! are rejected on encode; values that need a mantissa beyond the signed
//! range yield `ValueOutOfRange`.
//!
//! `9.001` (temperature) encodes/decodes via [`encode`]/[`decode`] using the
//! [`DptValue::Temperature`] variant. The other 2-octet floats
//! (`9.002` temperature difference K, `9.003` temperature gradient K/h,
//! `9.004` lux, `9.005` wind speed, `9.006` pressure, `9.007` humidity,
//! `9.008` CO2/air-quality ppm, `9.009` air flow m³/h, `9.010` time period s,
//! `9.011` time period ms, `9.020` voltage mV, `9.021` current mA,
//! `9.022` power density W/m², `9.023` kelvin/percent K/%, `9.024` power kW,
//! `9.025` volume flow l/h, `9.026` rain amount l/m², `9.027` temperature °F,
//! `9.028` wind speed km/h, `9.029` absolute humidity g/m³,
//! `9.030` concentration µg/m³) — i.e. every defined `9.xxx` other than
//! `9.001` — share the identical wire codec but are DECODE-ONLY via
//! [`decode_weather`] and use the unit-agnostic [`DptValue::Float16`] variant,
//! so they are never misrepresented as temperature.

use crate::{common, DptError, DptValue, Result};

const MANTISSA_MAX: u16 = 0x07ff;
const SIGN_BIT: u16 = 0x8000;
const EXPONENT_SHIFT: u16 = 11;
const EXPONENT_MASK: u16 = 0x0f;
const RESOLUTION: f32 = 0.01;
const MAX_EXPONENT: u16 = 15;
// the signed (two's-complement) mantissa span: a set sign bit makes the 11-bit
// field negative by subtracting 2048, so the mantissa ranges over [-2048, 2047].
const MANTISSA_SPAN: i32 = 0x0800;
const MANTISSA_SIGNED_MIN: i32 = -0x0800;
const MANTISSA_SIGNED_MAX: i32 = 0x07ff;

pub fn encode(value: DptValue) -> Result<std::vec::Vec<u8>> {
    let DptValue::Temperature(value) = value else {
        return Err(DptError::TypeMismatch { dpt: "9.001" });
    };
    if !value.is_finite() {
        return Err(DptError::InvalidValue {
            dpt: "9.001",
            reason: "temperature must be finite",
        });
    }

    for exponent in 0..=MAX_EXPONENT {
        let scale = (1u32 << exponent) as f32 * RESOLUTION;
        // signed two's-complement mantissa: keep the sign here (NOT abs) so the
        // sign bit is driven by the mantissa, which also makes -0.0 encode to
        // [0x00, 0x00] (mantissa 0, sign clear) rather than a spurious -2048.
        let mantissa = (value / scale).round() as i32;
        if (MANTISSA_SIGNED_MIN..=MANTISSA_SIGNED_MAX).contains(&mantissa) {
            let sign = if mantissa < 0 { SIGN_BIT } else { 0x0000 };
            let field = (mantissa & (MANTISSA_MAX as i32)) as u16;
            let word = sign | (exponent << EXPONENT_SHIFT) | field;
            return Ok(std::vec![(word >> 8) as u8, word as u8]);
        }
    }

    Err(DptError::ValueOutOfRange {
        dpt: "9.001",
        value,
    })
}

/// Decode the shared KNX 2-octet float wire form to an `f32`. Used by both
/// the 9.001 temperature codec and the generic weather-float decoder.
fn decode_float(bytes: &[u8]) -> Result<f32> {
    let bytes = common::be_array::<2>(bytes)?;

    let word = u16::from_be_bytes(bytes);
    let exponent = ((word >> EXPONENT_SHIFT) & EXPONENT_MASK) as u32;
    // 11-bit two's-complement mantissa in [-2048, 2047]: bit 15 is the sign
    // extension of the 11-bit field, so a set sign bit subtracts 2048.
    let mut mantissa = (word & MANTISSA_MAX) as i32;
    if word & SIGN_BIT != 0 {
        mantissa -= MANTISSA_SPAN;
    }
    Ok(mantissa as f32 * RESOLUTION * (1u32 << exponent) as f32)
}

pub fn decode(bytes: &[u8]) -> Result<DptValue> {
    Ok(DptValue::Temperature(decode_float(bytes)?))
}

/// Decode a non-temperature 2-octet float (9.002..=9.011 + 9.020..=9.030:
/// Δ-temp/gradient/lux/wind/pressure/humidity/CO2-ppm/air-flow/time-period-s/
/// time-period-ms/voltage-mV/current-mA/power-density/K%/power-kW/volume-flow/
/// rain/°F/wind-km·h/abs-humidity/concentration). Same two's-complement wire
/// codec as 9.001, but the decoded value is the unit-agnostic
/// [`DptValue::Float16`] (the DPT id carries the unit). Decode-only; the signed
/// members (e.g. 9.002/9.003 Δ-temp/gradient, 9.023 K/%, 9.027 °F, voltage/
/// current) decode correctly because the shared codec is two's-complement.
pub fn decode_weather(bytes: &[u8]) -> Result<DptValue> {
    Ok(DptValue::Float16(decode_float(bytes)?))
}
