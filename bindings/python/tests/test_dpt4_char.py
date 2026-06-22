"""DPT 4.xxx character decode -- binding mirror.

Python-binding mirror of crates/knx-dpt/tests/dpt4_char.rs. 4.001 (ASCII, 7-bit)
and 4.002 (ISO-8859-1) decode (decode-only) to a new Char value, marshalled as a
1-char string ({"type":"char","value":"A"}). Encode is refused because a
character value cannot infer a writable DPT main. Expected non-printable /
high-Latin-1 chars use chr(...) so this source stays pure ASCII.
"""

import pytest

import knxyz as ok


def test_dpt4_001_ascii_decodes_to_char_string():
    assert ok.dpt.decode("4.001", bytes([0x41])) == {"type": "char", "value": "A"}
    assert ok.dpt.decode("4.001", bytes([0x00])) == {"type": "char", "value": chr(0x00)}  # NUL
    assert ok.dpt.decode("4.001", bytes([0x7F])) == {"type": "char", "value": chr(0x7F)}  # DEL


def test_dpt4_001_ascii_rejects_high_bit():
    for byte in (0x80, 0xA0, 0xFF):
        with pytest.raises(ValueError, match="7-bit ASCII"):
            ok.dpt.decode("4.001", bytes([byte]))


def test_dpt4_002_latin1_decodes_every_byte():
    # Latin-1 byte b -> Unicode U+00b (char::from(b) on the Rust side)
    assert ok.dpt.decode("4.002", bytes([0xE4])) == {"type": "char", "value": chr(0xE4)}  # a-umlaut
    assert ok.dpt.decode("4.002", bytes([0xFF])) == {"type": "char", "value": chr(0xFF)}  # y-umlaut
    # 0x80 is rejected by 4.001 but accepted by 4.002 (Latin-1 C1 control, U+0080)
    assert ok.dpt.decode("4.002", bytes([0x80])) == {"type": "char", "value": chr(0x80)}


def test_dpt4_wrong_length_returns_invalid_length():
    for dpt in ("4.001", "4.002"):
        with pytest.raises(ValueError, match="invalid payload length"):
            ok.dpt.decode(dpt, bytes([0x41, 0x42]))


def test_dpt4_is_decode_only_encode_refused():
    with pytest.raises(ValueError):
        ok.dpt.encode("4.001", {"type": "char", "value": "A"})


def test_dpt4_undefined_subs_stay_unsupported():
    for dpt in ("4.003", "4.000"):
        with pytest.raises(ValueError, match="unsupported datapoint type"):
            ok.dpt.decode(dpt, bytes([0x41]))


def test_dpt5_raw_u8_passthrough_still_works():
    # regression: the unrelated DPT5 raw-U8 passthrough is unaffected
    assert ok.dpt.decode("5.010", bytes([0x41])) == 65
