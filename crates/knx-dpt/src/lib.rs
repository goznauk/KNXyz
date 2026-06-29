#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
//! KNX Datapoint Type (DPT) value codecs.
//!
//! # Public API
//!
//! - [`encode`]`(dpt: &str, value: DptValue) -> Result<Vec<u8>>` — encode a
//!   [`DptValue`] to its on-bus byte payload for the given DPT id (e.g.
//!   `"1.001"`, `"9.001"`).
//! - [`decode`]`(dpt: &str, bytes: &[u8]) -> Result<DptValue>` — decode a
//!   byte payload back into a [`DptValue`].
//!
//! The `dpt` string is parsed as `main.sub`; dispatch is by **main** number.
//!
//! # Supported DPT main groups
//!
//! `1`, `2`, `3`, `4` (decode-only), `5`, `6`, `7`, `8`, `9`, `10`, `11`, `12`,
//! `13`, `14`, `16`, `17`, `18`, `19`, `20`, `21` (decode-only raw bitset),
//! `22` (decode-only raw bitset), `29` (decode-only),
//! `232` (RGB; DPT payload encode + decode, live colour writes refused).
//! Groups `1,2,3,6,7,8,10,11,12,13,14,16,17,18,19`
//! dispatch uniformly (sub number is not inspected — any sub of a supported
//! main routes to that codec). Eight groups are intentionally **non-uniform**
//! (a sub is inspected, or the whole main is decode-only, before the uniform
//! table):
//!
//! - **DPT 4**: `4.001` (ASCII, 7-bit range-checked) and `4.002` (ISO-8859-1 /
//!   Latin-1, all bytes) decode (decode-only) to `Char`; the character set is
//!   carried by the DPT id. Any other `4.xxx` is unsupported. Encode is
//!   unsupported (main 4 is absent from the uniform table and `Char` is rejected
//!   by write-path inference).
//! - **DPT 5**: `5.001` is `Scaling` (0..=100 % ⇄ a single 0..=255 byte,
//!   lossy); `5.003` decodes to `Angle` (degrees, decode-only); every other
//!   `5.xxx` is raw `U8` passthrough.
//! - **DPT 9**: `9.001` (2-octet KNX two's-complement float) round-trips as
//!   `Temperature`; `9.002`..=`9.011` + `9.020`..=`9.030` decode (decode-only)
//!   to the unit-agnostic `Float16` (Δ-temp/gradient/lux/wind/pressure/
//!   humidity/CO2-ppm/air-flow/time-period-s/time-period-ms/voltage-mV/
//!   current-mA/power-density/K%/power-kW/volume-flow/rain/°F/wind-km·h/
//!   abs-humidity/concentration) — every defined `9.xxx` decodes. Encode stays
//!   `9.001`-only. The `9.012`-`9.019` reserved gap and `9.031`+ (unassigned)
//!   are unsupported.
//! - **DPT 20**: `20.102` is the HVAC operating mode (`HvacMode`, 0..=4);
//!   `20.105` is the HVAC controller mode (`HvacControllerMode`, 0..=17|20),
//!   sub-dispatched; every other `20.xxx` routes to the `20.102` codec.
//! - **DPT 21 / 22**: `21.xxx` (1-octet B8) decodes (decode-only) to a raw
//!   `Bitset8(u8)` mask; `22.xxx` (2-octet B16, big-endian) to a raw
//!   `Bitset16(u16)` mask. Sub-agnostic (the mask is the same for every sub; the
//!   per-bit meaning is carried by the DPT id and is not interpreted — no
//!   named-bit semantics). Encode is unsupported (mains 21/22 are absent from the
//!   uniform table and `Bitset8`/`Bitset16` are rejected by write-path inference
//!   — dedicated variants, not a `U8`/`U16` reuse, which would infer to the
//!   writable `5.010`/`7.001`).
//! - **DPT 29**: `29.xxx` (8-octet V64 two's-complement signed integer —
//!   `29.010` active energy Wh, `29.011` apparent VAh, `29.012` reactive VARh)
//!   decodes (decode-only) to `I64`; the unit is carried by the DPT id. Encode
//!   is unsupported (main 29 is absent from the uniform table and `I64` is
//!   rejected by write-path inference). Bindings serialise `I64` as a decimal
//!   string (the value exceeds the JS safe-integer range).
//! - **DPT 232**: `232.600` (3-octet RGB colour) decodes to / encodes from
//!   `Rgb { red, green, blue }` — the symmetric pure codec round-trips. Main 232
//!   is absent from the uniform table (an explicit dpt-id-keyed arm). Encode
//!   here is the DPT byte transform; colour writes stay refused (the `Rgb`
//!   variant is in `knx-ip`'s `encode_value` refusal arm).
//!
//! # Unsupported / malformed
//!
//! Any unsupported main group, an unsupported DPT 9 sub, or a `dpt` string
//! that does not parse as `main.sub` digits yields
//! [`DptError::UnsupportedDpt`] carrying the original `dpt` string. Wrong
//! [`DptValue`] variants yield [`DptError::TypeMismatch`]; wrong byte
//! lengths yield [`DptError::InvalidLength`].
//!
//! This crate does **not** claim complete KNX DPT coverage — only the main
//! groups listed above are implemented. The exact runtime dispatch contract
//! is pinned by `tests/dispatch.rs`.

#[cfg(feature = "std")]
#[macro_use]
mod macros;
#[cfg(feature = "std")]
mod common;
#[cfg(feature = "std")]
mod dpt1;
#[cfg(feature = "std")]
mod dpt10;
#[cfg(feature = "std")]
mod dpt11;
#[cfg(feature = "std")]
mod dpt12;
#[cfg(feature = "std")]
mod dpt13;
#[cfg(feature = "std")]
mod dpt14;
#[cfg(feature = "std")]
mod dpt16;
#[cfg(feature = "std")]
mod dpt17;
#[cfg(feature = "std")]
mod dpt18;
#[cfg(feature = "std")]
mod dpt19;
#[cfg(feature = "std")]
mod dpt2;
#[cfg(feature = "std")]
mod dpt20;
#[cfg(feature = "std")]
mod dpt21;
#[cfg(feature = "std")]
mod dpt22;
#[cfg(feature = "std")]
mod dpt232;
#[cfg(feature = "std")]
mod dpt29;
#[cfg(feature = "std")]
mod dpt3;
#[cfg(feature = "std")]
mod dpt4;
#[cfg(feature = "std")]
mod dpt5;
#[cfg(feature = "std")]
mod dpt6;
#[cfg(feature = "std")]
mod dpt7;
#[cfg(feature = "std")]
mod dpt8;
#[cfg(feature = "std")]
mod dpt9;
#[cfg(feature = "std")]
mod error;
#[cfg(feature = "std")]
mod id;
#[cfg(feature = "std")]
mod value;

#[cfg(feature = "std")]
pub use error::{DptError, Result};
#[cfg(feature = "std")]
pub use value::DptValue;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(feature = "std")]
type EncodeFn = fn(DptValue) -> Result<std::vec::Vec<u8>>;
#[cfg(feature = "std")]
type DecodeFn = fn(&[u8]) -> Result<DptValue>;

/// Uniform DPT codecs keyed by main number: `(main, encode, decode)`.
///
/// DPT 5 (sub-typed: `5.001` scaling, `5.003` angle decode, else raw
/// `U8`) and DPT 9 (`9.001` temperature encode/decode; `9.002`..=`9.011` +
/// `9.020`..=`9.030` 2-octet floats decode-only) are intentionally not in
/// this table — their dispatch is not uniform and is handled explicitly
/// in `encode` / `decode`. Keeping the uniform mapping in one place means
/// the encode and decode sides cannot drift.
#[cfg(feature = "std")]
const UNIFORM_CODECS: &[(u16, EncodeFn, DecodeFn)] = &[
    (1, dpt1::encode, dpt1::decode),
    (2, dpt2::encode, dpt2::decode),
    (3, dpt3::encode, dpt3::decode),
    (6, dpt6::encode, dpt6::decode),
    (7, dpt7::encode, dpt7::decode),
    (8, dpt8::encode, dpt8::decode),
    (10, dpt10::encode, dpt10::decode),
    (11, dpt11::encode, dpt11::decode),
    (12, dpt12::encode, dpt12::decode),
    (13, dpt13::encode, dpt13::decode),
    (14, dpt14::encode, dpt14::decode),
    (16, dpt16::encode, dpt16::decode),
    (17, dpt17::encode, dpt17::decode),
    (18, dpt18::encode, dpt18::decode),
    (19, dpt19::encode, dpt19::decode),
    (20, dpt20::encode, dpt20::decode),
];

#[cfg(feature = "std")]
pub fn encode(dpt: &str, value: DptValue) -> Result<std::vec::Vec<u8>> {
    let id = id::DptId::parse(dpt)?;

    match id.main() {
        // dpt5/dpt9/dpt232 dispatch is not uniform — kept explicit.
        5 => dpt5::encode(dpt, value),
        9 if id.sub() == 1 => dpt9::encode(value),
        // 232.600 RGB — the pure dpt-id-keyed codec (round-trips with decode).
        // Main 232 is not in UNIFORM_CODECS, so this explicit arm is the only
        // encode path. Colour writes stay refused: the `Rgb` variant is kept in
        // knx-ip's `encode_value` (variant-keyed) refusal arm.
        232 => dpt232::encode(value),
        // 13.010/13.013/13.014/13.015 energy (Wh/kWh/VAh/VARh) — the four switched
        // energy subs. The guard is `id.main()==13 && matches!(id.sub(), 10 | 13 |
        // 14 | 15)` (an exact closed set, no ranges); every other 13.xxx (incl the
        // 13.001 counter + adjacent unselected subs) falls through to the uniform
        // (13, …) macro codec below (I32-only). This payload arm accepts EnergyI32
        // (symmetric with dpt13::decode_energy) and I32 (backward compat) as
        // identical 4 bytes; EnergyU32 and all else refuse. Live energy writes
        // stay refused at knx-ip `encode_value` (variant-keyed).
        13 if matches!(id.sub(), 10 | 13 | 14 | 15) => dpt13::encode_energy(value),
        main => match UNIFORM_CODECS.iter().find(|&&(m, _, _)| m == main) {
            Some(&(_, encode_fn, _)) => encode_fn(value),
            None => Err(DptError::UnsupportedDpt(dpt.to_owned())),
        },
    }
}

#[cfg(feature = "std")]
pub fn decode(dpt: &str, bytes: &[u8]) -> Result<DptValue> {
    let id = id::DptId::parse(dpt)?;

    match id.main() {
        // dpt4/dpt5/dpt9 dispatch is not uniform — kept explicit.
        // 4.001 ASCII (7-bit, range-checked) / 4.002 ISO-8859-1 (Latin-1, all
        // bytes) — decode-only to DptValue::Char, sub-aware (the character set
        // is carried by the DPT id). Any other 4.xxx stays UnsupportedDpt. Main
        // 4 is absent from UNIFORM_CODECS so encode stays UnsupportedDpt, and
        // Char is rejected by knx-ip encode_value (decode-only isolation).
        4 if id.sub() == 1 => dpt4::decode_ascii(bytes),
        4 if id.sub() == 2 => dpt4::decode_latin1(bytes),
        5 => dpt5::decode(dpt, bytes),
        9 if id.sub() == 1 => dpt9::decode(bytes),
        // 2-octet floats (Δ-temp/gradient/lux/wind/pressure/humidity/CO2-ppm/
        // air-flow/time-period-s/time-period-ms + voltage-mV/current-mA +
        // 9.022-9.030 power-density/K%/power-kW/volume-flow/rain/°F/wind-km·h/
        // abs-humidity/concentration) — decode-only to the unit-agnostic
        // Float16 (all share the 9.001 two's-complement codec). 9.012-9.019 is
        // a reserved KNX gap and 9.031+ is unassigned; both stay unsupported.
        9 if matches!(id.sub(), 2..=11 | 20..=30) => dpt9::decode_weather(bytes),
        // 20.105 HVAC controller mode — peeled off before the uniform
        // (20, …) entry, which serves 20.102 via dpt20::decode.
        20 if id.sub() == 105 => dpt20::decode_controller_mode(bytes),
        // 29.xxx V64 (8-octet two's-complement signed energy) — decode-only to
        // DptValue::I64. Sub-agnostic (29.010 Wh/29.011 VAh/29.012 VARh share
        // one codec; unit by DPT id). Main 29 is intentionally not in
        // UNIFORM_CODECS, so encode("29.xxx", …) stays UnsupportedDpt, and I64
        // is rejected by knx-ip encode_value (decode-only isolation).
        29 => dpt29::decode(bytes),
        // 21.xxx B8 (1 octet) -> DptValue::Bitset8 raw mask; 22.xxx B16 (2 octets,
        // big-endian) -> DptValue::Bitset16 raw mask. Decoding is supported;
        // encoding is not, and the codec is sub-agnostic by main. Mains 21/22
        // are intentionally not in UNIFORM_CODECS, so encode("21.xxx"/"22.xxx",
        // …) stays UnsupportedDpt, and Bitset8/Bitset16 are rejected by knx-ip
        // encode_value (decode-only isolation, not a U8/U16 reuse).
        21 => dpt21::decode(bytes),
        22 => dpt22::decode(bytes),
        // 232.600 RGB colour (3 octets R/G/B) <-> DptValue::Rgb (symmetric pure
        // codec; the encode side is the matching explicit arm in `encode`).
        // Main 232 is intentionally not in UNIFORM_CODECS. The codec
        // round-trips payload bytes, but colour writes stay refused: `Rgb` is in
        // knx-ip's `encode_value` (variant-keyed write inference) refusal arm.
        232 => dpt232::decode(bytes),
        // 13.010/13.013/13.014/13.015 energy (Wh/kWh/VAh/VARh) — the four switched
        // energy subs: decode to the energy-tagged EnergyI32 rather than the
        // generic I32 that every other 13.xxx (incl. the 13.001 counter + adjacent
        // unselected subs) keeps. Same signed i32 value; only the type tag
        // changes. The guard is `id.main()==13 && matches!(id.sub(), 10 | 13 | 14
        // | 15)` (an exact closed set — no range, no leak into 13.001/13.011/…).
        13 if matches!(id.sub(), 10 | 13 | 14 | 15) => dpt13::decode_energy(bytes),
        main => match UNIFORM_CODECS.iter().find(|&&(m, _, _)| m == main) {
            Some(&(_, _, decode_fn)) => decode_fn(bytes),
            None => Err(DptError::UnsupportedDpt(dpt.to_owned())),
        },
    }
}
