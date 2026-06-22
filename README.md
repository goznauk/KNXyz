# KNXyz

KNXyz is a KNX library for Rust, Python, and Node.js.

KNXyz is early software and unstable.

## Usage

KNXyz provides KNX datapoint (DPT) codecs and KNXnet/IP client building blocks
for Rust, Python, and Node.js.

### Read a group value over KNXnet/IP

Connect to a KNXnet/IP interface on your network and read a group value. Use your
own interface address in place of the placeholder host.

#### Python

```python
import asyncio
from knxyz import connect_tunnel

async def main():
    client = await connect_tunnel(host="knxip.example")   # your KNXnet/IP interface
    try:
        value = await client.read("1/0/0", "9.001")        # read a temperature
        print(value)
    finally:
        await client.close()

asyncio.run(main())
```

#### Node.js

```javascript
import { connectTunnel } from "@knxyz/knx";

const client = await connectTunnel({ host: "knxip.example" });   // your KNXnet/IP interface
try {
  const value = await client.read("1/0/0", "9.001");   // read a temperature
  console.log(value);
} finally {
  await client.close();
}
```

#### Rust

```rust
use knxyz::ip::TunnelClient;
use knxyz::GroupAddress;
use std::net::ToSocketAddrs;
use std::time::Duration;

let addr = "knxip.example:3671".to_socket_addrs()?.next().expect("resolve interface");
let mut client = TunnelClient::connect(addr).await?;
let value = client
    .group_read("1/0/0".parse::<GroupAddress>()?, "9.001", Duration::from_secs(3))
    .await?;
println!("{value:?}");
client.disconnect().await?;
```

`discover_gateways()` finds KNXnet/IP interfaces on the local network.

### Datapoint values

Encode and decode KNX datapoint values directly:

```python
from knxyz import dpt

payload = dpt.encode("9.001", 21.0)    # temperature, 2-byte float
print(payload.hex())                    # 0c1a
print(dpt.decode("9.001", payload))     # 21.0
```

```javascript
import { encodeDpt, decodeDpt } from "@knxyz/knx";

const payload = encodeDpt("9.001", 21.0);
console.log(Buffer.from(payload).toString("hex"));   // 0c1a
console.log(decodeDpt("9.001", payload));            // 21
```

```rust
use knxyz::{dpt, DptValue};

let payload = dpt::encode("9.001", DptValue::Temperature(21.0))?;  // -> [0c, 1a]
let value = dpt::decode("9.001", &payload)?;                       // -> Temperature(21.0)
```

See [examples/](examples/README.md) for the full runnable examples in each language.

## Documentation

- [Architecture](docs/architecture.md)
- [Examples](examples/README.md)

## License

KNXyz is released under the MIT license. See [LICENSE](LICENSE).
