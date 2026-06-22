#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum DptValue {
    Bool(bool),
    U8(u8),
    Scaling(f32),
    Temperature(f32),
    /// A decoded KNX 2-octet float (DPT 9.xxx) that is NOT temperature —
    /// e.g. illuminance (9.004), wind speed (9.005), pressure (9.006),
    /// humidity (9.007). The unit is carried by the DPT id, never by this
    /// variant, so non-temperature 9.xxx values are not misrepresented as
    /// `Temperature`. Decode-only: it is not writable via DPT-id inference
    /// (a single variant cannot select one 9.xxx sub-type).
    Float16(f32),
    /// A decoded DPT 5.003 angle in degrees (0..=360), the 1-byte scaled
    /// form (`degrees = byte * 360 / 255`). Distinct from `Scaling`
    /// (5.001 percent). Decode-only.
    Angle(f32),
    ControlBool {
        control: bool,
        value: bool,
    },
    StepControl {
        increase: bool,
        step_code: u8,
    },
    I8(i8),
    U16(u16),
    I16(i16),
    Time {
        weekday: u8,
        hour: u8,
        minute: u8,
        second: u8,
    },
    Date {
        year: u16,
        month: u8,
        day: u8,
    },
    DateTime {
        year: u16,
        month: u8,
        day: u8,
        weekday: u8,
        hour: u8,
        minute: u8,
        second: u8,
    },
    U32(u32),
    I32(i32),
    F32(f32),
    Text14(std::string::String),
    SceneNumber(u8),
    SceneControl {
        learn: bool,
        scene: u8,
    },
    Rgb {
        red: u8,
        green: u8,
        blue: u8,
    },
    Rgbw {
        red: u8,
        green: u8,
        blue: u8,
        white: u8,
    },
    HvacMode(u8),
    /// DPT 20.105 HVAC controller mode (a 1-octet enumeration distinct
    /// from the 20.102 operating mode carried by `HvacMode`). Raw byte;
    /// the valid set is 0..=17 plus 20 (18/19 are KNX-reserved). Kept a
    /// separate variant so DPT-id inference maps it to 20.105 (not 20.102)
    /// and so the wider value range is range-checked independently.
    HvacControllerMode(u8),
    EnergyI32(i32),
    EnergyU32(u32),
    /// DPT 29.xxx (V64) 8-octet two's-complement signed integer — active
    /// energy (29.010 Wh), apparent energy (29.011 VAh), reactive energy
    /// (29.012 VARh). DECODE-ONLY: the unit is carried by the DPT id, never the
    /// variant. The i64 range (±9.2e18) exceeds the JS safe-integer range
    /// (2^53), so bindings serialize it as a DECIMAL STRING, never a bare JSON
    /// number. Kept out of `encode_value` inference (loud-fails there) so a
    /// decoded value can never be silently written to a wrong main.
    I64(i64),
    /// DPT 4.xxx single character — `4.001` ASCII (0x00..=0x7F), `4.002`
    /// ISO-8859-1 / Latin-1 (0x00..=0xFF). DECODE-ONLY: the character set is
    /// carried by the DPT id, never the variant. A Rust `char` is a Unicode
    /// scalar value, which losslessly represents both subs (Latin-1 == the first
    /// 256 Unicode code points); bindings serialize it as a 1-char string. Kept
    /// out of `encode_value` inference (loud-fails there) so a decoded value can
    /// never be silently written to a wrong main.
    Char(char),
    /// DPT 21.xxx — KNX B8, a 1-octet RAW BITSET. DECODE-ONLY: the octet is
    /// decoded to a `u8` mask. This is the raw mask ONLY — the per-bit meaning
    /// (e.g. 21.001 General Status bit assignments) is carried by the DPT id and
    /// is NOT interpreted here. Every 21.xxx sub shares this one codec. Kept out
    /// of `encode_value` inference (loud-fails there) so a decoded mask can never
    /// be silently written to a wrong main; reusing `U8` would be unsafe because
    /// `U8` infers to the writable `5.010`.
    Bitset8(u8),
    /// DPT 22.xxx — KNX B16, a 2-octet RAW BITSET. DECODE-ONLY: the two octets
    /// are decoded big-endian to a `u16` mask. Raw mask ONLY — the per-bit
    /// meaning is carried by the DPT id and is NOT interpreted here. Every 22.xxx
    /// sub shares this one codec. Kept out of `encode_value` inference; reusing
    /// `U16` would be unsafe because `U16` infers to the writable `7.001`.
    Bitset16(u16),
}
