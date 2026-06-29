//! Raw C ABI for KNXyz native consumers.
//!
//! ABI v1 exposes DPT `9.001` temperature encode/decode with plain C data
//! types. It is intended for C and C++ callers that build and link the `knxyz`
//! facade crate output.

#![allow(non_camel_case_types)]

use std::panic::{catch_unwind, AssertUnwindSafe};

use knx_dpt::{DptError, DptValue};

/// Current raw C ABI version.
pub const KNXYZ_CAPI_ABI_VERSION: u32 = 1;

/// Stable status codes returned by the raw C ABI.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum knxyz_status_t {
    KNXYZ_STATUS_OK = 0,
    KNXYZ_STATUS_INVALID_VALUE = 1,
    KNXYZ_STATUS_OUT_OF_RANGE = 2,
    KNXYZ_STATUS_INTERNAL_ERROR = 255,
}

/// Two-byte DPT `9.001` payload.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct knxyz_dpt9_001_payload_t {
    pub bytes: [u8; 2],
}

/// Result for DPT `9.001` encode.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct knxyz_dpt9_001_encode_result_t {
    pub status: knxyz_status_t,
    pub payload: knxyz_dpt9_001_payload_t,
}

/// Result for DPT `9.001` decode.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct knxyz_dpt9_001_decode_result_t {
    pub status: knxyz_status_t,
    pub value: f32,
}

pub type knxyz_dpt9_001_encode_f32_fn = extern "C" fn(f32) -> knxyz_dpt9_001_encode_result_t;
pub type knxyz_dpt9_001_decode_f32_fn = extern "C" fn(u8, u8) -> knxyz_dpt9_001_decode_result_t;

/// Function table used by the Python PyCapsule.
///
/// C and C++ callers can use the exported raw C ABI functions directly.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct knxyz_capi_v1 {
    pub abi_version: u32,
    pub struct_size: usize,
    pub dpt9_001_encode_f32: knxyz_dpt9_001_encode_f32_fn,
    pub dpt9_001_decode_f32: knxyz_dpt9_001_decode_f32_fn,
}

const ZERO_PAYLOAD: knxyz_dpt9_001_payload_t = knxyz_dpt9_001_payload_t { bytes: [0, 0] };

/// Return the raw C ABI version.
#[no_mangle]
pub extern "C" fn knxyz_capi_abi_version() -> u32 {
    KNXYZ_CAPI_ABI_VERSION
}

/// Encode a DPT `9.001` temperature value to the two-byte KNX payload.
#[no_mangle]
pub extern "C" fn knxyz_dpt9_001_encode_f32(value: f32) -> knxyz_dpt9_001_encode_result_t {
    match catch_unwind(AssertUnwindSafe(|| encode_dpt9_001(value))) {
        Ok(result) => result,
        Err(_) => encode_result(knxyz_status_t::KNXYZ_STATUS_INTERNAL_ERROR, ZERO_PAYLOAD),
    }
}

/// Decode a DPT `9.001` two-byte KNX payload to a temperature value.
#[no_mangle]
pub extern "C" fn knxyz_dpt9_001_decode_f32(b0: u8, b1: u8) -> knxyz_dpt9_001_decode_result_t {
    match catch_unwind(AssertUnwindSafe(|| decode_dpt9_001(b0, b1))) {
        Ok(result) => result,
        Err(_) => decode_result(knxyz_status_t::KNXYZ_STATUS_INTERNAL_ERROR, 0.0),
    }
}

/// Return the PyCapsule function table for CPython/Cython consumers.
pub fn capi_v1() -> knxyz_capi_v1 {
    knxyz_capi_v1 {
        abi_version: KNXYZ_CAPI_ABI_VERSION,
        struct_size: std::mem::size_of::<knxyz_capi_v1>(),
        dpt9_001_encode_f32: knxyz_dpt9_001_encode_f32,
        dpt9_001_decode_f32: knxyz_dpt9_001_decode_f32,
    }
}

fn encode_dpt9_001(value: f32) -> knxyz_dpt9_001_encode_result_t {
    match knx_dpt::encode("9.001", DptValue::Temperature(value)) {
        Ok(bytes) if bytes.len() == 2 => encode_result(
            knxyz_status_t::KNXYZ_STATUS_OK,
            knxyz_dpt9_001_payload_t {
                bytes: [bytes[0], bytes[1]],
            },
        ),
        Err(DptError::InvalidValue { .. }) => {
            encode_result(knxyz_status_t::KNXYZ_STATUS_INVALID_VALUE, ZERO_PAYLOAD)
        }
        Err(DptError::ValueOutOfRange { .. }) => {
            encode_result(knxyz_status_t::KNXYZ_STATUS_OUT_OF_RANGE, ZERO_PAYLOAD)
        }
        _ => encode_result(knxyz_status_t::KNXYZ_STATUS_INTERNAL_ERROR, ZERO_PAYLOAD),
    }
}

fn decode_dpt9_001(b0: u8, b1: u8) -> knxyz_dpt9_001_decode_result_t {
    match knx_dpt::decode("9.001", &[b0, b1]) {
        Ok(DptValue::Temperature(value)) => decode_result(knxyz_status_t::KNXYZ_STATUS_OK, value),
        _ => decode_result(knxyz_status_t::KNXYZ_STATUS_INTERNAL_ERROR, 0.0),
    }
}

const fn encode_result(
    status: knxyz_status_t,
    payload: knxyz_dpt9_001_payload_t,
) -> knxyz_dpt9_001_encode_result_t {
    knxyz_dpt9_001_encode_result_t { status, payload }
}

const fn decode_result(status: knxyz_status_t, value: f32) -> knxyz_dpt9_001_decode_result_t {
    knxyz_dpt9_001_decode_result_t { status, value }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abi_version_is_v1() {
        assert_eq!(knxyz_capi_abi_version(), KNXYZ_CAPI_ABI_VERSION);
    }

    #[test]
    fn dpt9_001_round_trips() {
        let encoded = knxyz_dpt9_001_encode_f32(21.0);
        assert_eq!(encoded.status, knxyz_status_t::KNXYZ_STATUS_OK);
        assert_eq!(encoded.payload.bytes, [0x0c, 0x1a]);

        let decoded = knxyz_dpt9_001_decode_f32(encoded.payload.bytes[0], encoded.payload.bytes[1]);
        assert_eq!(decoded.status, knxyz_status_t::KNXYZ_STATUS_OK);
        assert!((decoded.value - 21.0).abs() < 0.01);
    }

    #[test]
    fn dpt9_001_negative_temperature_matches_wire_value() {
        let encoded = knxyz_dpt9_001_encode_f32(-5.0);
        assert_eq!(encoded.status, knxyz_status_t::KNXYZ_STATUS_OK);
        assert_eq!(encoded.payload.bytes, [0x86, 0x0c]);
    }

    #[test]
    fn dpt9_001_nan_is_invalid() {
        let encoded = knxyz_dpt9_001_encode_f32(f32::NAN);
        assert_eq!(encoded.status, knxyz_status_t::KNXYZ_STATUS_INVALID_VALUE);
    }
}
