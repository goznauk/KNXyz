"""DPT 29.xxx V64 (8-octet signed) decode — binding mirror.

Python-binding mirror of crates/knx-dpt/tests/dpt29_v64.rs. DPT29 (29.010 Wh /
29.011 VAh / 29.012 VARh) decodes (decode-only) to a new I64 value. Because an
i64 exceeds the JS safe-integer range (2^53), the binding marshals it as a
DECIMAL STRING ({"type":"i64","value":"<decimal>"}), not a bare JSON number — so
values up to i64::MAX survive losslessly. Encode is refused (decode-only).
"""

import pytest

import knxyz as ok

I64_MAX = 9223372036854775807  # 0x7FFF_FFFF_FFFF_FFFF
I64_MIN = -9223372036854775808  # 0x8000_0000_0000_0000


def test_dpt29_decodes_to_i64_decimal_string():
    # value is a STRING (precision-safe); all three energy subs decode identically
    for dpt in ("29.010", "29.011", "29.012"):
        assert ok.dpt.decode(dpt, bytes([0, 0, 0, 0, 0, 0, 0, 1])) == {
            "type": "i64",
            "value": "1",
        }
        assert ok.dpt.decode(dpt, bytes([0xFF] * 8)) == {"type": "i64", "value": "-1"}


def test_dpt29_large_values_survive_losslessly():
    # i64::MAX would be corrupted as a bare JS number; as a decimal string it is
    # exact, and int(...) reconstructs it precisely.
    out = ok.dpt.decode("29.010", bytes([0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]))
    assert out == {"type": "i64", "value": "9223372036854775807"}
    assert int(out["value"]) == I64_MAX

    out_min = ok.dpt.decode("29.010", bytes([0x80, 0, 0, 0, 0, 0, 0, 0]))
    assert int(out_min["value"]) == I64_MIN

    out_distinct = ok.dpt.decode("29.010", bytes([0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]))
    assert int(out_distinct["value"]) == 72623859790382856


def test_dpt29_is_decode_only_encode_refused():
    with pytest.raises(ValueError):
        ok.dpt.encode("29.010", {"type": "i64", "value": "1000"})


def test_dpt29_wrong_length_returns_invalid_length():
    with pytest.raises(ValueError, match="invalid payload length"):
        ok.dpt.decode("29.010", bytes([0, 0, 0, 0, 0, 0, 0]))  # 7 octets


def test_dpt29_neighbour_mains_stay_unsupported():
    for dpt in ("28.001", "30.001"):
        with pytest.raises(ValueError, match="unsupported datapoint type"):
            ok.dpt.decode(dpt, bytes([0] * 8))
