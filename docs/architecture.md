# Architecture

KNXyz is an independent Rust workspace: protocol primitives at the bottom, an
async KNXnet/IP transport above them, and thin Python and Node.js bindings over
the Rust APIs. `knxyz` is the primary public crate.

## Workspace layout

- `crates/knxyz`: the primary public crate; a small facade that re-exports the
  DPT codec (`knxyz::dpt`) and the KNXnet/IP client (`knxyz::ip`).
- `crates/knx-core`: KNX address types, cEMI group telegrams, and KNXnet/IP
  framing primitives. Kept small and no-std-friendly where practical.
- `crates/knx-dpt`: datapoint type (DPT) encoding and decoding, with shared
  Node/Python parity fixtures.
- `crates/knx-ip`: a Tokio-based KNXnet/IP client for discovery, tunnelling, and
  routing.
- `crates/knx-sim`: an unpublished in-process simulated gateway for transport tests.
- `bindings/python`: a PyO3 native module exposed as the `knxyz` Python package.
- `bindings/node`: a napi-rs native module exposed as the `@knxyz/knx` package.

## Runtime boundaries

`knx-core` and `knx-dpt` are runtime-independent: they parse, validate, and
encode data structures without opening sockets or depending on Tokio.

`knx-ip` is the async boundary; it uses Tokio for UDP sockets, timeouts, and
monitor streams. Higher layers depend on `knx-ip` rather than re-implementing
KNXnet/IP behavior.

The bindings are thin adapters: they pass values into the Rust APIs and decode
the results, and do not re-implement frame encoding, cEMI/APCI packing, DPT
conversion, sequence counters, or ACK handling.

## DPT values

`knx-dpt` owns binary datapoint conversion. The bindings pass JSON-shaped values
into Rust and return the same shapes back. DPT coverage is partial: many
datapoint families encode and decode, while several (for example DPT 4, parts of
the DPT 9 family, DPT 21/22, and DPT 29) are decode-only for now.

## KNXnet/IP transport

`knx-ip` provides KNXnet/IP discovery, UDP tunnelling (connect, group
read/write, timeouts, ACKs, and a monitor stream), and routing multicast
(receive, plus experimental transmit). TCP tunnelling, KNX Secure, and serial
transports are planned and reuse the same core primitives.
