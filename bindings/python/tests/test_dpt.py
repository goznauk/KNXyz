import json
from pathlib import Path

import pytest

from knxyz import (
    dpt,
    format_group_address,
    format_individual_address,
    parse_group_address,
    parse_individual_address,
)


def test_addresses_match_rust_parity_fixtures():
    assert parse_individual_address("1.1.4") == "1.1.4"
    assert format_individual_address("1.1.4") == "1.1.4"
    assert parse_group_address("1/2/3") == "1/2/3"
    assert format_group_address("1/2/3") == "1/2/3"


def test_dpt_bool_matches_rust_parity_fixture():
    encoded = dpt.encode("1.001", True)

    assert encoded == bytes([0x01])
    assert dpt.decode("1.001", encoded) is True


def test_dpt_5_raw_matches_rust_parity_fixture():
    encoded = dpt.encode("5.010", 128)

    assert encoded == bytes([0x80])
    assert dpt.decode("5.010", encoded) == 128


def test_dpt_9_001_temperature_matches_rust_parity_fixture():
    encoded = dpt.encode("9.001", 21.0)

    assert encoded == bytes([0x0C, 0x1A])
    assert dpt.decode("9.001", encoded) == 21.0


def test_dpt_9_008_co2_ppm_decodes_via_the_2octet_float():
    # a related DPT-9 datapoint: 9.008 (CO2/air-quality
    # ppm) shares the DPT-9 2-octet float codec and is DECODE-ONLY (the unit
    # is carried by the DPT id; CO2 is never tagged as temperature).
    assert dpt.decode("9.008", bytes([0x2D, 0x78])) == pytest.approx(448.0, abs=0.5)
    assert dpt.decode("9.008", bytes([0x01, 0xF4])) == pytest.approx(5.0, abs=1e-3)
    # decode-only: encode raises (a generic float cannot infer the sub)
    with pytest.raises(ValueError):
        dpt.encode("9.008", 448.0)
    # invalid length is loud, not a silent/garbage value
    with pytest.raises(ValueError):
        dpt.decode("9.008", bytes([0x00]))


def test_dpt_17_001_scene_number_untyped_round_trips():
    # The Python binding exposes DPT 17.001 directly as the 0-63 wire scene value.
    for wire in (0, 4, 63):
        encoded = dpt.encode("17.001", wire)
        assert encoded == bytes([wire])
        assert dpt.decode("17.001", encoded) == wire


def test_dpt_17_001_rejects_out_of_range():
    # the native codec validates the 6-bit scene field (0-63)
    for bad in (64, 100, 255):
        with pytest.raises(ValueError):
            dpt.encode("17.001", bad)
    for bad in (-1, 256):
        with pytest.raises(ValueError):
            dpt.encode("17.001", bad)
    with pytest.raises(ValueError):
        dpt.decode("17.001", bytes([64]))


def test_dpt_17_001_untyped_and_tagged_encode_are_byte_identical():
    # both accepted input shapes (bare int and the tagged scene_number object)
    # produce the same wire byte. Node accepts both too.
    assert dpt.encode("17.001", 7) == bytes([7])
    assert dpt.encode("17.001", {"type": "scene_number", "value": 7}) == bytes([7])
    assert dpt.encode("17.001", 7) == dpt.encode(
        "17.001", {"type": "scene_number", "value": 7}
    )


def test_dpt_17_001_decode_output_is_a_bare_int():
    # Python returns a bare int for SceneNumber decode. Node returns a tagged
    # object, so this test keeps the Python output shape explicit.
    decoded = dpt.decode("17.001", bytes([7]))
    assert decoded == 7
    assert isinstance(decoded, int) and not isinstance(decoded, dict)


def test_dpt_19_001_datetime_round_trips():
    # Native DPT 19.001 datetime codec. weekday 0 -> no-day-of-week status.
    value = {
        "type": "datetime", "year": 2024, "month": 6, "day": 15,
        "weekday": 0, "hour": 13, "minute": 45, "second": 30,
    }
    encoded = dpt.encode("19.001", value)
    assert encoded == bytes([124, 6, 15, 13, 45, 30, 36, 0])
    assert dpt.decode("19.001", encoded) == value
    # a carried weekday packs into the hour byte's high bits
    sat = {**value, "weekday": 6}
    assert dpt.encode("19.001", sat) == bytes([124, 6, 15, 205, 45, 30, 32, 0])
    assert dpt.decode("19.001", dpt.encode("19.001", sat)) == sat


def test_dpt_19_001_rejects_out_of_range_and_partial():
    base = {
        "type": "datetime", "year": 2024, "month": 6, "day": 15,
        "weekday": 0, "hour": 13, "minute": 45, "second": 30,
    }
    for bad in (
        {**base, "year": 1899},
        {**base, "year": 2156},
        {**base, "month": 13},
        {**base, "hour": 24, "minute": 30},  # hour 24 only with 0 min/sec
    ):
        with pytest.raises(ValueError):
            dpt.encode("19.001", bad)
    # a partial datetime (no-time validity bit, status 0x26) is rejected
    # rather than decoded as a fabricated midnight
    with pytest.raises(ValueError):
        dpt.decode("19.001", bytes([124, 6, 15, 0, 0, 0, 0x26, 0]))
    with pytest.raises(ValueError):
        dpt.decode("19.001", bytes([124, 6, 15]))  # wrong length


def test_dpt_20_102_hvac_operating_mode_round_trips():
    # Native DPT 20.102 codec (HVAC operating mode, 0..4).
    for wire in (0, 1, 2, 3, 4):
        encoded = dpt.encode("20.102", {"type": "hvac_mode", "value": wire})
        assert encoded == bytes([wire])
        assert dpt.decode("20.102", encoded) == {"type": "hvac_mode", "value": wire}


def test_dpt_20_102_rejects_out_of_range():
    # the native codec validates the 5 HVAC operating modes (0..4)
    for bad in (5, 6, 255):
        with pytest.raises(ValueError):
            dpt.encode("20.102", {"type": "hvac_mode", "value": bad})
    with pytest.raises(ValueError):
        dpt.decode("20.102", bytes([5]))
    with pytest.raises(ValueError):
        dpt.decode("20.102", bytes([1, 2]))  # wrong length


def test_dpt_20_105_hvac_controller_mode_round_trips():
    # C1: native DPT 20.105 codec (HVAC controller mode), distinct from
    # 20.102. Valid set is 0..=17 plus 20 (NoDem).
    for wire in (0, 1, 5, 17, 20):
        encoded = dpt.encode(
            "20.105", {"type": "hvac_controller_mode", "value": wire}
        )
        assert encoded == bytes([wire])
        assert dpt.decode("20.105", encoded) == {
            "type": "hvac_controller_mode",
            "value": wire,
        }


def test_dpt_20_105_rejects_reserved_and_out_of_range():
    # 18/19 are KNX-reserved; 21+ are undefined - all loud-fail.
    for bad in (18, 19, 21, 255):
        with pytest.raises(ValueError):
            dpt.encode("20.105", {"type": "hvac_controller_mode", "value": bad})
        with pytest.raises(ValueError):
            dpt.decode("20.105", bytes([bad]))
    with pytest.raises(ValueError):
        dpt.decode("20.105", bytes([1, 2]))  # wrong length


def test_dpt_20_102_and_20_105_do_not_alias():
    # byte 5 is a valid 20.105 controller mode but an invalid 20.102
    # operating mode - the two sub-types must not be confused.
    assert dpt.decode("20.105", bytes([5])) == {
        "type": "hvac_controller_mode",
        "value": 5,
    }
    with pytest.raises(ValueError):
        dpt.decode("20.102", bytes([5]))


def test_shared_dpt_fixtures_match_rust_parity():
    for fixture in load_dpt_core_fixtures():
        encoded = dpt.encode(fixture["dpt"], fixture["value"])

        assert encoded == bytes(fixture["bytes"]), fixture["name"]
        assert dpt.decode(fixture["dpt"], encoded) == expected_decoded_value(
            fixture
        ), fixture["name"]


def test_unsupported_dpt_ids_are_rejected_by_rust_bindings():
    # encode of main 21 is unsupported (decode-only: 21.xxx decodes a raw B8
    # mask but has no encode arm; a bool value is also the wrong shape)
    with pytest.raises(ValueError, match="unsupported datapoint type"):
        dpt.encode("21.001", {"type": "bool", "value": True})
    with pytest.raises(ValueError, match="unsupported datapoint type"):
        dpt.encode("9.002", {"type": "temperature", "value": 21.0})
    with pytest.raises(ValueError, match="unsupported datapoint type"):
        dpt.decode("14.float", bytes([0, 0, 0, 0]))


def test_malformed_dpt_values_are_rejected_by_rust_bindings():
    with pytest.raises(ValueError, match="invalid value"):
        dpt.encode("3.007", {"type": "step_control", "increase": True, "step_code": 8})
    with pytest.raises(ValueError, match="invalid value"):
        dpt.encode("17.001", {"type": "scene_number", "value": 64})
    with pytest.raises(ValueError, match="out of i8 range"):
        dpt.encode("6.010", {"type": "i8", "value": 128})
    with pytest.raises(ValueError, match="invalid value"):
        dpt.encode("16.000", {"type": "text14", "value": "abcdefghijklmn!"})


def test_malformed_dpt_payloads_are_rejected_by_rust_bindings():
    with pytest.raises(ValueError, match="invalid value"):
        dpt.decode("1.001", bytes([0x02]))
    with pytest.raises(ValueError, match="invalid payload length"):
        dpt.decode("9.001", bytes([0x00]))
    with pytest.raises(ValueError, match="invalid value"):
        dpt.decode("16.000", bytes([0xFF, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]))
    with pytest.raises(ValueError, match="invalid value"):
        dpt.decode("18.001", bytes([0x40]))


def load_dpt_core_fixtures():
    path = (
        Path(__file__).resolve().parents[3]
        / "crates"
        / "knx-dpt"
        / "tests"
        / "fixtures"
        / "dpt_core_fixtures.json"
    )
    return json.loads(path.read_text())


def expected_decoded_value(fixture):
    value = fixture["value"]
    # These scalar DPTs decode to a plain Python value (no typed
    # envelope). scene_number (DPT 17.001) is a plain int too, symmetric
    # with its untyped encode path.
    if isinstance(value, dict) and value.get("type") in {
        "bool",
        "u8",
        "scaling",
        "temperature",
        "scene_number",
    }:
        return value["value"]
    return value
