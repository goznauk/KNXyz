//! KNXyz - a KNX library for Rust, Python, and Node.js.
//!
//! `knxyz` is the primary public crate. It re-exports the stable KNX data point
//! type (DPT) encode/decode API ([`dpt`]) and the KNXnet/IP client building
//! blocks ([`ip`]) so applications depend on a single `knxyz` crate rather than
//! the internal `knx-*` building blocks. The facade is intentionally small.
//!
//! # Example
//!
//! Encode and decode a KNX datapoint value:
//!
//! ```
//! use knxyz::{dpt, DptValue};
//!
//! let payload = dpt::encode("9.001", DptValue::Temperature(21.0))?;
//! assert_eq!(payload, vec![0x0c, 0x1a]);
//!
//! let value = dpt::decode("9.001", &payload)?;
//! assert_eq!(value, DptValue::Temperature(21.0));
//! # Ok::<(), knxyz::dpt::DptError>(())
//! ```

#![forbid(unsafe_code)]

/// KNX data point type (DPT) encoding and decoding.
///
/// Re-exported from the `knx-dpt` crate: [`encode`](knx_dpt::encode) /
/// [`decode`](knx_dpt::decode) are pure payload byte transforms.
pub mod dpt {
    pub use knx_dpt::{decode, encode, DptError, DptValue, Result};
}

/// The KNX datapoint value type, re-exported at the crate root for convenience.
pub use knx_dpt::DptValue;

/// KNXnet/IP client building blocks.
///
/// Re-exported from the `knx-ip` crate: connect a
/// [`TunnelClient`](knx_ip::TunnelClient) to a KNXnet/IP interface to read
/// group values, or use [`discover_gateways`](knx_ip::discover_gateways) to find
/// interfaces on the local network.
///
/// ```no_run
/// # async fn read_one() -> Result<(), Box<dyn std::error::Error>> {
/// use knxyz::ip::TunnelClient;
/// use knxyz::GroupAddress;
/// use std::net::ToSocketAddrs;
/// use std::time::Duration;
///
/// // resolve your KNXnet/IP interface (host or IP, default port 3671)
/// let addr = "knxip.example:3671".to_socket_addrs()?.next().expect("resolve interface");
/// let mut client = TunnelClient::connect(addr).await?;
/// let value = client
///     .group_read("1/0/0".parse::<GroupAddress>()?, "9.001", Duration::from_secs(3))
///     .await?;
/// println!("{value:?}");
/// client.disconnect().await?;
/// # Ok(()) }
/// ```
pub mod ip {
    pub use knx_ip::{
        discover_gateways, DiscoveryOptions, Gateway, KnxIpError, Result, TunnelClient,
        TunnelOptions, KNXNET_IP_PORT,
    };
}

/// The KNX group address type, re-exported at the crate root for convenience.
pub use knx_core::GroupAddress;

/// Returns the `knxyz` crate version.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::{dpt, DptValue};

    #[test]
    fn dpt_round_trip_offline() {
        // DPT 9.001 (2-byte float, degC): 21.0 -> 0x0c 0x1a -> 21.0
        let payload = dpt::encode("9.001", DptValue::Temperature(21.0)).unwrap();
        assert_eq!(payload, vec![0x0c, 0x1a]);
        assert_eq!(
            dpt::decode("9.001", &payload).unwrap(),
            DptValue::Temperature(21.0)
        );
    }

    #[test]
    fn root_dpt_value_reexport_matches() {
        // the crate-root DptValue is the same type as dpt::DptValue
        let v: DptValue = dpt::DptValue::Bool(true);
        assert_eq!(v, DptValue::Bool(true));
    }

    #[test]
    fn version_is_non_empty() {
        assert!(!super::version().is_empty());
    }
}
