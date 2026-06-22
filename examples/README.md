# Examples

Runnable KNXyz examples, from a source checkout. KNXyz covers KNX datapoint
codecs and KNXnet/IP client building blocks; the datapoint examples need no KNX
hardware, and the KNXnet/IP examples are default-safe (dry-run) until you opt in.

## Offline examples

The offline DPT examples encode and decode datapoint values from local bytes
only - no network, no KNX hardware. Each one prints the value, the encoded bytes
(hex), and the decoded value, and verifies the round-trip locally.

- Python: `python examples/python/offline_dpt.py`
- Node.js: `examples/node/offline-dpt.ts` (build the native addon first - see
  Language notes)
- Rust: `examples/rust/offline_dpt.rs`
  (`cargo run --manifest-path examples/rust/Cargo.toml --bin offline-dpt`)

## Running from source

From a source checkout, build the relevant binding or crate before running the
examples:

- Python: `cd bindings/python && maturin develop`, then run the example.
- Node.js: `cd bindings/node && npm ci && npm run build:native`, then run the
  example. The native addon build is exercised in CI.
- Rust: `examples/rust` is a small standalone Cargo package that depends on the
  public `knxyz` facade by path. Run it with
  `cargo run --manifest-path examples/rust/Cargo.toml --bin offline-dpt`.

## Language notes

- Python: `from knxyz import dpt`.
- Node.js: `import { encodeDpt, decodeDpt } from "@knxyz/knx";`. A few datapoint
  types decode to a slightly different shape than Python (for example DPT 17.001
  decodes to a tagged object in Node and a bare integer in Python); the example
  handles this.
- Rust: `use knxyz::{dpt, DptValue};`.

## KNXnet/IP examples

KNXyz includes a KNXnet/IP client in all three languages. Connect a tunnel to an
interface and read a group value with `connect_tunnel(host=...)` /
`client.read(...)` (Python), `connectTunnel({ host })` / `client.read(...)`
(Node.js), or `knxyz::ip::TunnelClient::connect(...)` / `group_read(...)` (Rust).
`discover_gateways()` finds interfaces on the local network.

These examples are default-safe: a plain run performs no bus I/O - it prints the
telegram it would send (a dry run) and opens no socket. A live read or write is
opt-in, requires an explicit host you control, group address, and DPT, and
refuses documentation placeholder hosts; writes also require an explicit value.
The exact flags are in each example's source.

Read a group value:

- Python: `examples/python/group_read.py`
- Node.js: `examples/node/group-read.ts`
- Rust: `examples/rust/group_read.rs`
  (`cargo run --manifest-path examples/rust/Cargo.toml --bin group-read -- --dry-run`)

Write a group value (opt-in, isolated test bus only). The Rust write example is
a boolean DPT 1.001 write:

- Python: `examples/python/group_write.py`
- Node.js: `examples/node/group-write.ts`
- Rust: `examples/rust/group_write.rs`
  (`cargo run --manifest-path examples/rust/Cargo.toml --bin group-write -- --dry-run`)
