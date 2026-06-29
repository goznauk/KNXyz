#include <math.h>
#include <stdint.h>

#include "knxyz/capi.h"

static int close_enough(float left, float right) {
  return fabsf(left - right) < 0.01f;
}

int main(void) {
  if (knxyz_capi_abi_version() != KNXYZ_CAPI_ABI_VERSION) {
    return 1;
  }

  knxyz_dpt9_001_encode_result_t encoded =
      knxyz_dpt9_001_encode_f32(21.0f);
  if (encoded.status != KNXYZ_STATUS_OK) {
    return 2;
  }
  if (encoded.payload.bytes[0] != 0x0c || encoded.payload.bytes[1] != 0x1a) {
    return 3;
  }

  knxyz_dpt9_001_decode_result_t decoded =
      knxyz_dpt9_001_decode_f32(0x0c, 0x1a);
  if (decoded.status != KNXYZ_STATUS_OK) {
    return 4;
  }
  if (!close_enough(decoded.value, 21.0f)) {
    return 5;
  }

  encoded = knxyz_dpt9_001_encode_f32(-5.0f);
  if (encoded.status != KNXYZ_STATUS_OK) {
    return 6;
  }
  if (encoded.payload.bytes[0] != 0x86 || encoded.payload.bytes[1] != 0x0c) {
    return 7;
  }

  encoded = knxyz_dpt9_001_encode_f32(NAN);
  if (encoded.status != KNXYZ_STATUS_INVALID_VALUE) {
    return 8;
  }

  return 0;
}
