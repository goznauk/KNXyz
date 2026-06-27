"""DPT 21.xxx / 22.xxx raw bit sets (B8 / B16) decode — binding mirror.

Python-binding mirror of crates/knx-dpt/tests/dpt21_dpt22_bitset.rs. DPT21 is a
fixed 1-octet "8-bit set" and DPT22 a fixed 2-octet (big-endian) "16-bit set".
The wire width is fixed by the main number, so the raw mask decodes without
external per-bit wire-layout evidence. Per-bit meaning is a separate semantic
layer this codec does not claim; decode is sub-agnostic.
The masks are JS-safe (u16::MAX = 65535 < 2^53), so the binding marshals them as
a plain JSON number ({"type":"bitset8","value":<u8>} / {"type":"bitset16",...}),
unlike DPT29's precision-safe decimal string. Encode is refused (decode-only).
"""

import pytest

import knxyz as ok


def test_dpt21_decodes_to_a_bare_u8_mask():
    # sub-agnostic: every sub of main 21 routes to the same B8 codec
    for dpt in ("21.001", "21.100", "21.105"):
        assert ok.dpt.decode(dpt, bytes([0x00])) == {"type": "bitset8", "value": 0}
        assert ok.dpt.decode(dpt, bytes([0xFF])) == {"type": "bitset8", "value": 255}
        # raw mask preserved bit-for-bit (no interpretation)
        assert ok.dpt.decode(dpt, bytes([0xA5])) == {"type": "bitset8", "value": 165}


def test_dpt22_decodes_to_a_bare_u16_mask_big_endian():
    # sub-agnostic: every sub of main 22 routes to the same B16 codec
    for dpt in ("22.100", "22.101", "22.1000"):
        assert ok.dpt.decode(dpt, bytes([0x00, 0x00])) == {"type": "bitset16", "value": 0}
        assert ok.dpt.decode(dpt, bytes([0xFF, 0xFF])) == {
            "type": "bitset16",
            "value": 65535,
        }
        # big-endian: first octet is the high byte (distinct bytes lock the order)
        assert ok.dpt.decode(dpt, bytes([0x01, 0x02])) == {
            "type": "bitset16",
            "value": 0x0102,
        }
        assert ok.dpt.decode(dpt, bytes([0xA5, 0x5A])) == {
            "type": "bitset16",
            "value": 0xA55A,
        }


def test_dpt21_dpt22_wrong_length_returns_invalid_length():
    # exactly 1 octet for DPT21, exactly 2 for DPT22
    with pytest.raises(ValueError, match="invalid payload length"):
        ok.dpt.decode("21.001", bytes([0x00, 0x00]))  # 2 octets
    with pytest.raises(ValueError, match="invalid payload length"):
        ok.dpt.decode("22.101", bytes([0x00]))  # 1 octet


def test_dpt21_dpt22_are_decode_only_encode_refused():
    # the bitset8/bitset16 JSON shapes parse (from-json arms exist for
    # round-trip/marshal), but the keyed encode still refuses — mains 21/22 are
    # absent from the codec table. The error is "unsupported datapoint type"
    # (parse succeeded), not "unsupported DPT JSON value type" (which would mean
    # the from-json arm is missing).
    with pytest.raises(ValueError, match="unsupported datapoint type"):
        ok.dpt.encode("21.001", {"type": "bitset8", "value": 255})
    with pytest.raises(ValueError, match="unsupported datapoint type"):
        ok.dpt.encode("22.101", {"type": "bitset16", "value": 65535})


def test_dpt21_dpt22_decoded_shape_round_trips_through_from_json():
    # the dict produced by decode is accepted by the from-json parser (so a
    # decoded value can be re-marshalled), then refused at encode — never
    # silently written.
    for dpt, payload in (("21.001", bytes([0xA5])), ("22.101", bytes([0xA5, 0x5A]))):
        decoded = ok.dpt.decode(dpt, payload)
        with pytest.raises(ValueError, match="unsupported datapoint type"):
            ok.dpt.encode(dpt, decoded)


def test_dpt21_dpt22_neighbour_mains_stay_unsupported():
    # mains 23/26/27/30 stay unsupported; only mains 21/22 were added
    for dpt in ("23.001", "26.001", "27.001", "30.001"):
        with pytest.raises(ValueError, match="unsupported datapoint type"):
            ok.dpt.decode(dpt, bytes([0, 0]))
