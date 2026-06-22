"""Offline DPT encode/decode example (default-safe, print-only).

This example does not connect to a KNX bus. It encodes and decodes KNX datapoint
values from local bytes only, so it needs no network and no KNX hardware. Run it
from a source checkout with the locally-built package:
  python examples/python/offline_dpt.py
"""

import sys

from knxyz import dpt


def _hex(payload: bytes) -> str:
    return " ".join("%02x" % b for b in payload)


def main() -> int:
    # Scalar DPTs round-trip to a bare Python value (bool / int / float).
    cases = [
        ("1.001", "switch (boolean)", True),
        ("9.001", "temperature (degC, Float16)", 21.0),
        ("9.001", "temperature negative (degC)", -5.5),
        ("5.010", "counter (raw 0-255)", 128),
        ("17.001", "scene number", 7),
    ]
    ok = True
    for dpt_id, label, value in cases:
        payload = dpt.encode(dpt_id, value)
        decoded = dpt.decode(dpt_id, payload)
        matched = decoded == value
        ok = ok and matched
        print(
            "DPT %-7s %-28s value=%-6r -> bytes=[%s] -> decoded=%r %s"
            % (dpt_id, label, value, _hex(payload), decoded, "OK" if matched else "MISMATCH")
        )
    print("offline round-trip:", "all OK" if ok else "FAILED")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
