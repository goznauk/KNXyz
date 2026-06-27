import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import {
  decodeDpt,
  encodeDpt,
  formatGroupAddress,
  formatIndividualAddress,
  parseGroupAddress,
  parseIndividualAddress,
} from "../src/index.ts";

interface DptFixture {
  name: string;
  dpt: string;
  value: unknown;
  bytes: number[];
}

describe("KNXyz Node parity fixtures", () => {
  it("parses and formats KNX addresses using Rust bindings", () => {
    assert.equal(parseIndividualAddress("1.1.4"), "1.1.4");
    assert.equal(formatIndividualAddress("1.1.4"), "1.1.4");
    assert.equal(parseGroupAddress("1/2/3"), "1/2/3");
    assert.equal(formatGroupAddress("1/2/3"), "1/2/3");
  });

  it("encodes and decodes DPT 1.xxx boolean values using Rust bindings", () => {
    const encoded = encodeDpt("1.001", true);

    assert.deepEqual([...encoded], [0x01]);
    assert.equal(decodeDpt("1.001", encoded), true);
  });

  it("encodes and decodes DPT 5 raw values using Rust bindings", () => {
    const encoded = encodeDpt("5.010", 128);

    assert.deepEqual([...encoded], [0x80]);
    assert.equal(decodeDpt("5.010", encoded), 128);
  });

  it("encodes and decodes DPT 9.001 temperatures using Rust bindings", () => {
    const encoded = encodeDpt("9.001", 21.0);

    assert.deepEqual([...encoded], [0x0c, 0x1a]);
    assert.equal(decodeDpt("9.001", encoded), 21.0);
  });

  it("decodes DPT 29 V64 as a precision-safe i64 decimal string", () => {
    // i64 exceeds JS Number.MAX_SAFE_INTEGER (2^53), so the binding emits a
    // decimal string, not a plain number. BigInt(value) reconstructs it exactly.
    const max = decodeDpt("29.010", Uint8Array.of(0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff));
    assert.deepEqual(max, { type: "i64", value: "9223372036854775807" });
    assert.equal(BigInt((max as { value: string }).value), 9223372036854775807n);

    const negOne = decodeDpt("29.011", Uint8Array.of(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff));
    assert.deepEqual(negOne, { type: "i64", value: "-1" });

    const distinct = decodeDpt("29.012", Uint8Array.of(0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08));
    assert.equal(BigInt((distinct as { value: string }).value), 72623859790382856n);

    // decode-only: encode is refused, and a short payload returns an error
    assert.throws(() => encodeDpt("29.010", { type: "i64", value: "1000" }), /unsupported datapoint type/);
    assert.throws(() => decodeDpt("29.010", Uint8Array.of(0, 0, 0, 0, 0, 0, 0)), /invalid payload length/);
  });

  it("decodes DPT 4 characters as a 1-char string (decode-only)", () => {
    // 4.001 ASCII (7-bit, range-checked) and 4.002 ISO-8859-1 decode to a
    // {type:"char"} 1-char string; the character set is carried by the DPT id.
    assert.deepEqual(decodeDpt("4.001", Uint8Array.of(0x41)), { type: "char", value: "A" });
    assert.deepEqual(decodeDpt("4.002", Uint8Array.of(0xe4)), { type: "char", value: "ä" });
    // 4.001 rejects bytes above 0x7f (not valid 7-bit ASCII)
    assert.throws(() => decodeDpt("4.001", Uint8Array.of(0x80)), /7-bit ASCII/);
    // decode-only: encode is refused
    assert.throws(() => encodeDpt("4.001", { type: "char", value: "A" }), /unsupported datapoint type/);
  });

  it("decodes DPT 21 / 22 raw bit sets as a plain mask (decode-only)", () => {
    // DPT21 = fixed 1-octet B8, DPT22 = fixed 2-octet big-endian B16; the raw
    // mask decodes sub-agnostically (per-bit meaning is not claimed). Masks are
    // JS-safe (<= 65535), so they cross as a plain number, not a string.
    assert.deepEqual(decodeDpt("21.001", Uint8Array.of(0xa5)), { type: "bitset8", value: 165 });
    assert.deepEqual(decodeDpt("21.100", Uint8Array.of(0xff)), { type: "bitset8", value: 255 });
    // big-endian: first octet is the high byte (distinct bytes lock the order)
    assert.deepEqual(decodeDpt("22.101", Uint8Array.of(0xa5, 0x5a)), { type: "bitset16", value: 0xa55a });
    assert.deepEqual(decodeDpt("22.100", Uint8Array.of(0x00, 0x00)), { type: "bitset16", value: 0 });
    // decode-only: keyed encode is refused (parse succeeds, encode has no arm),
    // and a wrong-length payload returns an error
    assert.throws(() => encodeDpt("21.001", { type: "bitset8", value: 255 }), /unsupported datapoint type/);
    assert.throws(() => encodeDpt("22.101", { type: "bitset16", value: 65535 }), /unsupported datapoint type/);
    assert.throws(() => decodeDpt("21.001", Uint8Array.of(0x00, 0x00)), /invalid payload length/);
    assert.throws(() => decodeDpt("22.101", Uint8Array.of(0x00)), /invalid payload length/);
  });

  it("round-trips DPT 232.600 RGB through the payload codec", () => {
    // encode is the pure DPT payload codec; live colour writes stay refused at
    // knx-ip encode_value (pinned in that crate).
    const encoded = encodeDpt("232.600", { type: "rgb", red: 10, green: 20, blue: 30 });
    assert.deepEqual([...encoded], [0x0a, 0x14, 0x1e]);
    assert.deepEqual(decodeDpt("232.600", encoded), { type: "rgb", red: 10, green: 20, blue: 30 });
  });

  it("rejects unsupported DPT 251.600 RGBW both directions and leaves RGB intact", () => {
    // This exercises Node's rgbw from-json arm: the value parses, then the keyed
    // encode refuses it because main 251 has no codec. Decode is rejected too.
    assert.throws(() => decodeDpt("251.600", Uint8Array.of(1, 2, 3, 4, 5, 6)), /unsupported datapoint type/);
    assert.throws(
      () => encodeDpt("251.600", { type: "rgbw", red: 1, green: 2, blue: 3, white: 4 }),
      /unsupported datapoint type/,
    );
    // regression: the shipped RGB 232.600 DPT payload codec is unaffected
    assert.deepEqual(decodeDpt("232.600", Uint8Array.of(0x0a, 0x14, 0x1e)), {
      type: "rgb",
      red: 10,
      green: 20,
      blue: 30,
    });
  });

  it("pins Energy value JSON marshal (from-json parse + encode refusal; decode-only)", () => {
    // EnergyI32 / EnergyU32 are not inferred for writes, so a true from-json ->
    // to-json round-trip is not reachable from the public API. These verify the
    // reachable conversion boundary: the from-json
    // value conversion (sign + range + the bare-number boundary + spelling)
    // followed by the encode refusal. The to-json render shape is pinned by the
    // Studio Rust unit test (a different serializer; nothing decodes to energy
    // here). A DPT13 energy decode change must deliberately update these.
    const I32_MIN = -2147483648;
    const I32_MAX = 2147483647;
    const U32_MAX = 4294967295;

    // energy_i32: a valid i32 (incl. the negative -> sign handling) is parsed by
    // json_i32, then encode refuses it under a signed 13.xxx id. The refusal is
    // TypeMismatch ("datapoint value type does not match"), not the
    // "unsupported datapoint type" message used for unknown ids -- matching it
    // proves the parse succeeded and the refusal came from the encoder.
    // The energy_i32 refusal now uses the non-energy 13.001 counter: the four
    // energy subs (13.010/13.013/13.014/13.015) now payload-encode energy_i32
    // (covered in the switched-subs test below). A valid i32 (incl. the negative
    // -> sign handling) is parsed by json_i32, then encode refuses it.
    assert.throws(() => encodeDpt("13.001", { type: "energy_i32", value: 0 }), /datapoint value type does not match/);
    assert.throws(() => encodeDpt("13.001", { type: "energy_i32", value: I32_MAX }), /datapoint value type does not match/);
    assert.throws(() => encodeDpt("13.001", { type: "energy_i32", value: I32_MIN }), /datapoint value type does not match/);
    // parser-origin guards (distinct from the refusal) prove the energy arm
    // dispatches to json_i32.
    assert.throws(() => encodeDpt("13.010", { type: "energy_i32", value: I32_MAX + 1 }), /out of i32 range/);
    assert.throws(() => encodeDpt("13.010", { type: "energy_i32", value: "x" }), /expected signed DPT value/);

    // energy_u32 is carried by an unsigned main (12.001), not DPT13. u32::MAX
    // crosses as a plain number (4294967295 < Number.MAX_SAFE_INTEGER) -- no
    // decimal string (contrast the i64 case above) -- is parsed by json_u32,
    // then encode refuses it (TypeMismatch dpt 12.xxx).
    assert.throws(() => encodeDpt("12.001", { type: "energy_u32", value: 0 }), /datapoint value type does not match/);
    assert.throws(() => encodeDpt("12.001", { type: "energy_u32", value: 1000 }), /datapoint value type does not match/);
    assert.throws(() => encodeDpt("12.001", { type: "energy_u32", value: U32_MAX }), /datapoint value type does not match/);
    assert.throws(() => encodeDpt("12.001", { type: "energy_u32", value: -1 }), /expected unsigned DPT value/);
    assert.throws(() => encodeDpt("12.001", { type: "energy_u32", value: U32_MAX + 1 }), /out of u32 range/);

    // spelling pin: only "energy_i32" / "energy_u32" are accepted; a near-miss
    // "energy" hits the from-json wildcard.
    assert.throws(() => encodeDpt("13.010", { type: "energy", value: 1 }), /unsupported DPT JSON value type/);
  });

  it("switches DPT13 13.010/13.013/13.014/13.015 to semantic energy_i32 decode + payload encode", () => {
    // The four energy subs (Wh/kWh/VAh/VARh) decode to {type:"energy_i32"} (every
    // other 13.xxx, incl. the 13.001 counter, stays {type:"i32"}). The payload
    // encode arm accepts both energy_i32 (symmetric with decode) and i32 (backward
    // compat) -> identical bytes. energy_u32 stays refused (never a DPT13
    // value). Live energy writes stay refused at knx-ip encode_value (unchanged).
    const POS = Uint8Array.of(0x00, 0x00, 0x03, 0xe8); // 1000
    const NEG = Uint8Array.of(0xff, 0xff, 0xfc, 0x18); // -1000

    for (const dpt of ["13.010", "13.013", "13.014", "13.015"]) {
      // decode -> energy_i32
      assert.deepEqual(decodeDpt(dpt, POS), { type: "energy_i32", value: 1000 });
      assert.deepEqual(decodeDpt(dpt, NEG), { type: "energy_i32", value: -1000 });
      // payload encode: energy_i32 round-trips, and i32 still encodes (same bytes)
      assert.deepEqual([...encodeDpt(dpt, { type: "energy_i32", value: 1000 })], [0x00, 0x00, 0x03, 0xe8]);
      assert.deepEqual([...encodeDpt(dpt, { type: "i32", value: 1000 })], [0x00, 0x00, 0x03, 0xe8]);
      // energy_u32 stays refused even for the switched subs (not a DPT13 candidate)
      assert.throws(() => encodeDpt(dpt, { type: "energy_u32", value: 1000 }), /datapoint value type does not match/);
    }

    // non-selected 13.xxx stay generic i32 (the 13.001 counter + adjacent unselected)
    assert.deepEqual(decodeDpt("13.001", POS), { type: "i32", value: 1000 });
    assert.deepEqual(decodeDpt("13.012", POS), { type: "i32", value: 1000 });
  });

  it("accepts the untyped 17.001 scene-number shorthand (parity with Python)", () => {
    // Node accepts a bare number under "17.001" as a scene number, matching the
    // Python binding's untyped shorthand. This adds an input spelling only; Node
    // could already write a scene via the
    // tagged {type:"scene_number"} form, and the native dpt17 0..63 bound is
    // unchanged (no validation bypass, no new live-write capability).
    for (const wire of [0, 4, 63]) {
      assert.deepEqual([...encodeDpt("17.001", wire)], [wire]);
      // decode output stays the tagged object, while Python returns a bare int.
      // Keep this behavior explicit so an output-shape change updates the tests.
      assert.deepEqual(decodeDpt("17.001", encodeDpt("17.001", wire)), {
        type: "scene_number",
        value: wire,
      });
    }
    // the untyped and tagged input shapes produce identical bytes
    assert.deepEqual(
      [...encodeDpt("17.001", 7)],
      [...encodeDpt("17.001", { type: "scene_number", value: 7 })],
    );
    // out-of-range stays rejected (no validation bypass): 0..63 via the dpt17
    // codec, 0..255 via json_u8
    for (const bad of [64, 100, 255]) {
      assert.throws(() => encodeDpt("17.001", bad), /invalid value/);
    }
    for (const bad of [-1, 256]) {
      assert.throws(() => encodeDpt("17.001", bad), /out of u8 range|expected unsigned DPT value/);
    }
  });

  it("matches shared DPT fixtures using Rust bindings", () => {
    for (const fixture of loadDptCoreFixtures()) {
      const encoded = encodeDpt(fixture.dpt, fixture.value);

      assert.deepEqual([...encoded], fixture.bytes, fixture.name);
      assert.deepEqual(decodeDpt(fixture.dpt, encoded), expectedDecodedValue(fixture), fixture.name);
    }
  });

  it("rejects unsupported DPT IDs through Rust bindings", () => {
    // encode of main 21 is unsupported (decode-only: 21.xxx decodes a raw B8
    // mask but has no encode arm; a bool value is also the wrong shape)
    assert.throws(() => encodeDpt("21.001", { type: "bool", value: true }), /unsupported datapoint type/);
    assert.throws(() => encodeDpt("9.002", { type: "temperature", value: 21.0 }), /unsupported datapoint type/);
    assert.throws(() => decodeDpt("14.float", Uint8Array.of(0, 0, 0, 0)), /unsupported datapoint type/);
  });

  it("rejects malformed DPT values through Rust bindings", () => {
    assert.throws(() => encodeDpt("3.007", { type: "step_control", increase: true, step_code: 8 }), /invalid value/);
    assert.throws(() => encodeDpt("17.001", { type: "scene_number", value: 64 }), /invalid value/);
    assert.throws(() => encodeDpt("6.010", { type: "i8", value: 128 }), /out of i8 range/);
    assert.throws(() => encodeDpt("16.000", { type: "text14", value: "abcdefghijklmn!" }), /invalid value/);
  });

  it("rejects malformed DPT payloads through Rust bindings", () => {
    assert.throws(() => decodeDpt("1.001", Uint8Array.of(0x02)), /invalid value/);
    assert.throws(() => decodeDpt("9.001", Uint8Array.of(0x00)), /invalid payload length/);
    assert.throws(() => decodeDpt("16.000", Uint8Array.of(0xff, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)), /invalid value/);
    assert.throws(() => decodeDpt("18.001", Uint8Array.of(0x40)), /invalid value/);
  });
});

function loadDptCoreFixtures(): DptFixture[] {
  const path = fileURLToPath(
    new URL("../../../crates/knx-dpt/tests/fixtures/dpt_core_fixtures.json", import.meta.url),
  );

  return JSON.parse(readFileSync(path, "utf8")) as DptFixture[];
}

function expectedDecodedValue(fixture: DptFixture): unknown {
  if (isTypedScalarFixture(fixture.value)) {
    return fixture.value.value;
  }

  return fixture.value;
}

function isTypedScalarFixture(value: unknown): value is { type: string; value: unknown } {
  return (
    typeof value === "object" &&
    value !== null &&
    "type" in value &&
    "value" in value &&
    ["bool", "u8", "scaling", "temperature"].includes(String(value.type))
  );
}
