# Cython example

This example builds a small Cython module that cimports `knxyz.capi` from the
installed KNXyz Python package.

It uses the Python package's PyCapsule C API for DPT `9.001` temperature
encode/decode. Value `21.0` encodes to `0c1a`, and the example checks invalid
input handling.

The Python package ships:

- `knxyz/capi.pxd`
- `knxyz/include/knxyz/capi.h`

The Python wheel's PyCapsule path is for Cython extensions running inside
Python. C and C++ consumers that build and link `libknxyz` from source use the
separate raw C ABI.

The linked C ABI smoke test lives in `crates/knxyz/tests/capi_smoke.c`.

Both native interfaces currently cover DPT `9.001` temperature encode/decode.

The example imports the package capsule object from `knxyz._knxyz` and passes it
to the cimported `knxyz_import_capi(...)` helper.

Build it after installing the local KNXyz Python package:

```sh
python -m pip install --no-build-isolation --no-deps \
  --target /tmp/knxyz-cython-smoke examples/python/cython
PYTHONPATH=/tmp/knxyz-cython-smoke python - <<'PY'
import knxyz_cython_smoke as s
assert s.smoke_summary() == "capi-ok"
PY
```
