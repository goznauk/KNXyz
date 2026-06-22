// KNXyz example: offline DPT encode/decode (default-safe, print-only).
//
// This example does not connect to a KNX bus. It encodes and decodes KNX
// datapoint values from local bytes only, so it needs no network and no KNX
// hardware. The Node binding is a native addon: build it once before running
// this example (see examples/README.md).

import { encodeDpt, decodeDpt } from "@knxyz/knx";

function hex(payload: Uint8Array): string {
  return Array.from(payload, (b) => b.toString(16).padStart(2, "0")).join(" ");
}

// Scalar DPTs decode to a bare value; some (e.g. 17.001 in Node) decode to a
// tagged object { type, value }. Pull the scalar out for the round-trip check.
function scalarOf(decoded: unknown): unknown {
  if (decoded !== null && typeof decoded === "object" && "value" in (decoded as object)) {
    return (decoded as { value: unknown }).value;
  }
  return decoded;
}

const cases: Array<[string, string, unknown]> = [
  ["1.001", "switch (boolean)", true],
  ["9.001", "temperature (degC, Float16)", 21.0],
  ["9.001", "temperature negative (degC)", -5.5],
  ["5.010", "counter (raw 0-255)", 128],
  ["17.001", "scene number", 7],
];

let ok = true;
for (const [dpt, label, value] of cases) {
  const payload = encodeDpt(dpt, value);
  const decoded = decodeDpt(dpt, payload);
  const matched = scalarOf(decoded) === value;
  ok = ok && matched;
  console.log(
    `DPT ${dpt.padEnd(7)} ${label.padEnd(28)} -> bytes=[${hex(payload)}] -> ` +
      `decoded=${JSON.stringify(decoded)} ${matched ? "OK" : "MISMATCH"}`,
  );
}

console.log(`offline round-trip: ${ok ? "all OK" : "FAILED"}`);
if (!ok) {
  process.exit(1);
}
