# Examples

Runnable KNXyz examples from a source checkout.

The DPT examples work with KNX payload bytes. KNXnet/IP examples include dry-run
commands and can also connect to a gateway when you provide a host, group
address, and DPT.

## DPT codec examples

The DPT examples encode and decode datapoint values. Each one prints the value,
the encoded bytes as hex, and the decoded value, then verifies the round trip.

- Python: `python examples/python/dpt_codec.py`
- Node.js: `examples/node/dpt-codec.ts` (build the native addon first; see
  Language notes)
- Rust: `examples/rust/dpt_codec.rs`
  (`cargo run --manifest-path examples/rust/Cargo.toml --bin dpt-codec`)

## Running from source

From a source checkout, build the relevant binding or crate before running the
examples:

- Python: `cd bindings/python && maturin develop`, then run the example.
- Node.js: `cd bindings/node && npm ci && npm run build:native`, then run the
  example. The native addon build is exercised in CI.
- Rust: `examples/rust` is a small standalone Cargo package that depends on the
  public `knxyz` facade by path. Run it with
  `cargo run --manifest-path examples/rust/Cargo.toml --bin dpt-codec`.

## Language notes

- Python: `from knxyz import dpt`.
- Node.js: `import { encodeDpt, decodeDpt } from "@knxyz/knx";`. A few datapoint
  types decode to a slightly different shape than Python (for example DPT 17.001
  decodes to a tagged object in Node and a bare integer in Python); the example
  handles this.
- Rust: `use knxyz::{dpt, DptValue};`.

### Cython

Cython extensions can cimport `knxyz.capi` and use the Python wheel's PyCapsule
C API. The example in `examples/python/cython/` builds a `.pyx` module, imports
the capsule table, and exercises DPT `9.001` temperature encode/decode.

KNXyz also exposes a small raw C ABI from the Rust `knxyz` facade crate for C
and C++ consumers that build and link `libknxyz` from source. The raw C ABI is
separate from the Python package PyCapsule path.

## KNXnet/IP examples

KNXyz includes a KNXnet/IP client in all three languages. Connect a tunnel to an
interface and read a group value with `connect_tunnel(host=...)` /
`client.read(...)` (Python), `connectTunnel({ host })` / `client.read(...)`
(Node.js), or `knxyz::ip::TunnelClient::connect(...)` / `group_read(...)` (Rust).
`discover_gateways()` finds interfaces on the local network.

The group read and group write examples print dry-run output unless live-mode
arguments are provided. To connect to a gateway, provide the host, group address,
and DPT; writes also take the value to send. The exact flags are in each
example's source.

Read a group value:

- Python: `examples/python/group_read.py`
- Node.js: `examples/node/group-read.ts`
- Rust: `examples/rust/group_read.rs`
  (`cargo run --manifest-path examples/rust/Cargo.toml --bin group-read`)

Write a group value. The runnable write examples demonstrate a DPT 1.001
boolean write:

- Python: `examples/python/group_write.py`
- Node.js: `examples/node/group-write.ts`
- Rust: `examples/rust/group_write.rs`
  (`cargo run --manifest-path examples/rust/Cargo.toml --bin group-write`)
