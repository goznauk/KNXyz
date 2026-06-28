from cpython.pycapsule cimport PyCapsule_GetPointer
from libc.stddef cimport size_t
from libc.stdint cimport uint8_t, uint32_t


cdef extern from "knxyz/capi.h":
    cdef enum:
        KNXYZ_CAPI_ABI_VERSION

    cdef enum knxyz_status_t:
        KNXYZ_STATUS_OK
        KNXYZ_STATUS_INVALID_VALUE
        KNXYZ_STATUS_OUT_OF_RANGE
        KNXYZ_STATUS_INTERNAL_ERROR

    ctypedef struct knxyz_dpt9_001_payload_t:
        uint8_t bytes[2]

    ctypedef struct knxyz_dpt9_001_encode_result_t:
        knxyz_status_t status
        knxyz_dpt9_001_payload_t payload

    ctypedef struct knxyz_dpt9_001_decode_result_t:
        knxyz_status_t status
        float value

    uint32_t knxyz_capi_abi_version()
    knxyz_dpt9_001_encode_result_t knxyz_dpt9_001_encode_f32(float value)
    knxyz_dpt9_001_decode_result_t knxyz_dpt9_001_decode_f32(uint8_t b0, uint8_t b1)

    ctypedef knxyz_dpt9_001_encode_result_t (*knxyz_dpt9_001_encode_f32_fn)(
        float value
    )
    ctypedef knxyz_dpt9_001_decode_result_t (*knxyz_dpt9_001_decode_f32_fn)(
        uint8_t b0,
        uint8_t b1,
    )

    ctypedef struct knxyz_capi_v1:
        uint32_t abi_version
        size_t struct_size
        knxyz_dpt9_001_encode_f32_fn dpt9_001_encode_f32
        knxyz_dpt9_001_decode_f32_fn dpt9_001_decode_f32


cdef inline knxyz_capi_v1* knxyz_import_capi(object capsule) except NULL:
    cdef knxyz_capi_v1* api = <knxyz_capi_v1*>PyCapsule_GetPointer(
        capsule,
        b"knxyz._knxyz._C_API",
    )
    if api == NULL:
        return NULL
    if api.abi_version != KNXYZ_CAPI_ABI_VERSION:
        raise ImportError("unsupported KNXyz PyCapsule C API version")
    if api.struct_size < sizeof(knxyz_capi_v1):
        raise ImportError("incomplete KNXyz PyCapsule C API table")
    return api
