#ifndef KNXYZ_CAPI_H
#define KNXYZ_CAPI_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define KNXYZ_CAPI_ABI_VERSION 1u

typedef enum knxyz_status_t {
  KNXYZ_STATUS_OK = 0,
  KNXYZ_STATUS_INVALID_VALUE = 1,
  KNXYZ_STATUS_OUT_OF_RANGE = 2,
  KNXYZ_STATUS_INTERNAL_ERROR = 255
} knxyz_status_t;

typedef struct {
  uint8_t bytes[2];
} knxyz_dpt9_001_payload_t;

typedef struct {
  knxyz_status_t status;
  knxyz_dpt9_001_payload_t payload;
} knxyz_dpt9_001_encode_result_t;

typedef struct {
  knxyz_status_t status;
  float value;
} knxyz_dpt9_001_decode_result_t;

uint32_t knxyz_capi_abi_version(void);
knxyz_dpt9_001_encode_result_t knxyz_dpt9_001_encode_f32(float value);
knxyz_dpt9_001_decode_result_t knxyz_dpt9_001_decode_f32(uint8_t b0, uint8_t b1);

typedef knxyz_dpt9_001_encode_result_t (*knxyz_dpt9_001_encode_f32_fn)(
    float value);
typedef knxyz_dpt9_001_decode_result_t (*knxyz_dpt9_001_decode_f32_fn)(
    uint8_t b0, uint8_t b1);

typedef struct {
  uint32_t abi_version;
  size_t struct_size;
  knxyz_dpt9_001_encode_f32_fn dpt9_001_encode_f32;
  knxyz_dpt9_001_decode_f32_fn dpt9_001_decode_f32;
} knxyz_capi_v1;

#ifdef __cplusplus
}
#endif

#endif
