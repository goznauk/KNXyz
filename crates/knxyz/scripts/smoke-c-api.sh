#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
crate_dir=$(CDPATH= cd -- "$script_dir/.." && pwd)
repo_root=$(CDPATH= cd -- "$crate_dir/../.." && pwd)
target_dir=${CARGO_TARGET_DIR:-"$repo_root/target"}
profile_dir="$target_dir/debug"
tmp_dir=$(mktemp -d)

cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

cargo build -p knxyz --lib

case "$(uname -s)" in
  Darwin)
    lib_name="libknxyz.dylib"
    path_var="DYLD_LIBRARY_PATH"
    ;;
  Linux)
    lib_name="libknxyz.so"
    path_var="LD_LIBRARY_PATH"
    ;;
  *)
    echo "unsupported platform for raw C ABI smoke: $(uname -s)" >&2
    exit 2
    ;;
esac

lib_path="$profile_dir/$lib_name"
if [ ! -f "$lib_path" ]; then
  echo "raw C ABI library not found: $lib_path" >&2
  exit 3
fi

cc -std=c99 -Wall -Wextra -I "$crate_dir/include" \
  "$crate_dir/tests/capi_smoke.c" "$lib_path" -lm -o "$tmp_dir/capi_smoke"

if [ "$path_var" = "DYLD_LIBRARY_PATH" ]; then
  DYLD_LIBRARY_PATH="$profile_dir${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}" \
    "$tmp_dir/capi_smoke"
else
  LD_LIBRARY_PATH="$profile_dir${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" \
    "$tmp_dir/capi_smoke"
fi
