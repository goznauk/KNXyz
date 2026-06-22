"""DPT 232.600 RGB colour (binding mirror).

Python-binding mirror of crates/knx-dpt/tests/dpt232_rgb.rs. DPT 232.600 is a
3-octet RGB colour (R, G, B; each 0..=255) that round-trips through the native
codec with the pre-existing ``rgb`` typed value. ``ok.dpt.encode`` is the
OFFLINE pure codec (no bus); live colour writes stay refused at knx-ip's
``encode_value``. The binding already marshals the ``Rgb`` variant to/from a
dict.
"""

import pytest

import knxyz as ok


def test_dpt232_rgb_decodes_to_an_rgb_dict():
    # byte order R, G, B (distinct values lock the order)
    assert ok.dpt.decode("232.600", bytes([0x0A, 0x14, 0x1E])) == {
        "type": "rgb",
        "red": 10,
        "green": 20,
        "blue": 30,
    }
    # boundary bytes both valid (no range check on a colour byte)
    assert ok.dpt.decode("232.600", bytes([0x00, 0x00, 0x00])) == {
        "type": "rgb",
        "red": 0,
        "green": 0,
        "blue": 0,
    }
    assert ok.dpt.decode("232.600", bytes([0xFF, 0xFF, 0xFF])) == {
        "type": "rgb",
        "red": 255,
        "green": 255,
        "blue": 255,
    }


def test_dpt232_rgb_encodes_offline_and_round_trips():
    # the OFFLINE pure codec now encodes RGB -> 3 bytes (no bus contact); this is
    # NOT a live write (colour actuation stays refused at knx-ip encode_value).
    assert ok.dpt.encode("232.600", {"type": "rgb", "red": 1, "green": 2, "blue": 3}) == bytes(
        [1, 2, 3]
    )
    # round-trip: encode then decode is the identity
    assert ok.dpt.decode("232.600", ok.dpt.encode(
        "232.600", {"type": "rgb", "red": 10, "green": 20, "blue": 30}
    )) == {"type": "rgb", "red": 10, "green": 20, "blue": 30}


def test_dpt232_rgb_wrong_length_returns_invalid_length():
    # exactly 3 octets required
    with pytest.raises(ValueError, match="invalid payload length"):
        ok.dpt.decode("232.600", bytes([0x00, 0x00]))
