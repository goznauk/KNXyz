"""Python binding coverage for unsupported DPT 251.600 RGBW values.

The public codec rejects DPT 251.600 until the masked RGBW payload layout is
implemented explicitly. This verifies decode and encode both surface the same
unsupported-DPT error while the implemented DPT 232.600 RGB codec still works.
"""

import pytest

import knxyz as ok


def test_dpt251_600_decode_is_unsupported():
    # a 6-octet RGBW-shaped payload must not decode onto a mask-less rgbw value
    with pytest.raises(ValueError, match="unsupported datapoint type"):
        ok.dpt.decode("251.600", bytes([0x0A, 0x14, 0x1E, 0x28, 0x00, 0x0F]))


def test_dpt251_600_encode_is_unsupported():
    # encoding an rgbw value as DPT 251.600 raises an unsupported-DPT error
    with pytest.raises(ValueError, match="unsupported datapoint type"):
        ok.dpt.encode(
            "251.600",
            {"type": "rgbw", "red": 1, "green": 2, "blue": 3, "white": 4},
        )


def test_dpt232_600_rgb_decode_still_works():
    # regression: the shipped sibling 232.600 RGB decode is unaffected
    assert ok.dpt.decode("232.600", bytes([0x0A, 0x14, 0x1E])) == {
        "type": "rgb",
        "red": 10,
        "green": 20,
        "blue": 30,
    }
