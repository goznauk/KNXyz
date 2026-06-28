# cython: language_level=3
"""Cython cimport smoke for the KNXyz PyCapsule C API."""

from libc.math cimport NAN

from knxyz import _knxyz as _native
from knxyz.capi cimport (
    KNXYZ_STATUS_INVALID_VALUE,
    KNXYZ_STATUS_OK,
    knxyz_capi_v1,
    knxyz_dpt9_001_decode_result_t,
    knxyz_dpt9_001_encode_result_t,
    knxyz_import_capi,
)


cdef knxyz_capi_v1* _api():
    cdef knxyz_capi_v1* api = knxyz_import_capi(_native._C_API)
    if api == NULL:
        raise ImportError("KNXyz PyCapsule C API is unavailable")
    return api


def round_trip_temperature():
    cdef knxyz_capi_v1* api = _api()
    cdef knxyz_dpt9_001_encode_result_t encoded = api.dpt9_001_encode_f32(21.0)
    if encoded.status != KNXYZ_STATUS_OK:
        raise AssertionError(encoded.status)
    if encoded.payload.bytes[0] != 0x0c or encoded.payload.bytes[1] != 0x1a:
        raise AssertionError((encoded.payload.bytes[0], encoded.payload.bytes[1]))

    cdef knxyz_dpt9_001_decode_result_t decoded = api.dpt9_001_decode_f32(
        encoded.payload.bytes[0],
        encoded.payload.bytes[1],
    )
    if decoded.status != KNXYZ_STATUS_OK:
        raise AssertionError(decoded.status)
    if decoded.value != 21.0:
        raise AssertionError(decoded.value)

    return f"{encoded.payload.bytes[0]:02x}{encoded.payload.bytes[1]:02x}", decoded.value


def negative_temperature_payload():
    cdef knxyz_capi_v1* api = _api()
    cdef knxyz_dpt9_001_encode_result_t encoded = api.dpt9_001_encode_f32(-5.0)
    if encoded.status != KNXYZ_STATUS_OK:
        raise AssertionError(encoded.status)
    return f"{encoded.payload.bytes[0]:02x}{encoded.payload.bytes[1]:02x}"


def nan_encode_status():
    cdef knxyz_capi_v1* api = _api()
    cdef knxyz_dpt9_001_encode_result_t encoded = api.dpt9_001_encode_f32(NAN)
    return encoded.status


def smoke_summary():
    assert round_trip_temperature() == ("0c1a", 21.0)
    assert negative_temperature_payload() == "860c"
    assert nan_encode_status() == KNXYZ_STATUS_INVALID_VALUE
    return "capi-ok"
