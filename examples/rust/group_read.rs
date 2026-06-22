//! KNXyz example - group READ (DEFAULT-SAFE: dry-run).
//!
//! SAFETY: a `GroupValueRead` is an ACTIVE bus telegram (it solicits the bus),
//! so this example performs NO bus I/O by default: it prints what it WOULD read
//! and exits WITHOUT opening a socket. A live read is advanced (Tier 2): it
//! requires `--live`, `KNXYZ_EXAMPLE_ALLOW_LIVE=1`, and explicit
//! `--gateway-host`/`--group-address`/`--dpt`, and refuses to run under CI.
//! NEVER point this at a production or shared KNX bus.
//! See examples/README.md.

use std::net::SocketAddr;
use std::time::Duration;

use knxyz::ip::TunnelClient;
use knxyz::GroupAddress;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<String> = std::env::args().collect();
    let has = |name: &str| args.iter().any(|a| a == name);
    let value_of = |name: &str| -> Option<String> {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };

    // a documentation placeholder host (RFC 5737 TEST-NET); refused in live mode
    let gateway_host = value_of("--gateway-host");
    let group_address_arg = value_of("--group-address");
    let dpt_arg = value_of("--dpt");

    let host = gateway_host
        .clone()
        .unwrap_or_else(|| "203.0.113.10:3671".to_string());
    let group_address = group_address_arg
        .clone()
        .unwrap_or_else(|| "1/2/4".to_string());
    let dpt = dpt_arg.clone().unwrap_or_else(|| "9.001".to_string());

    let armed = has("--live")
        && std::env::var("KNXYZ_EXAMPLE_ALLOW_LIVE").as_deref() == Ok("1")
        && gateway_host.is_some()
        && group_address_arg.is_some()
        && dpt_arg.is_some();

    if !armed {
        println!(
            "DRY-RUN: would read GA={group_address} as DPT={dpt} from {host} \
             (no connection made).\n\
             For a REAL read from a gateway you own / are authorized to test, pass:\n  \
             --live --gateway-host <host:port> --group-address <ga> --dpt 9.001\n  \
             and set KNXYZ_EXAMPLE_ALLOW_LIVE=1. Never target a production/shared bus."
        );
        return Ok(());
    }

    if is_ci() {
        return Err("refusing a live read under CI".into());
    }
    if is_placeholder_host(&host) {
        return Err(
            "refusing a live read against a documentation/placeholder host; pass \
             --gateway-host for your own gateway"
                .into(),
        );
    }
    let gateway: SocketAddr = host.parse()?;
    let group: GroupAddress = group_address.parse()?;
    println!("LIVE: reading GA={group_address} as DPT={dpt} from {host}...");
    let mut client = TunnelClient::connect(gateway).await?;
    let result = client.group_read(group, &dpt, Duration::from_secs(3)).await;
    let _ = client.disconnect().await; // best-effort teardown regardless of outcome
    println!("{:?}", result?);
    Ok(())
}

fn is_ci() -> bool {
    std::env::var("CI")
        .map(|v| !v.is_empty() && v != "0" && v != "false")
        .unwrap_or(false)
}

/// Refuse RFC 5737 TEST-NET / RFC 3849 documentation ranges and symbolic
/// `<...>` placeholder tokens as live targets.
fn is_placeholder_host(host: &str) -> bool {
    let h = host.split(':').next().unwrap_or(host);
    h.starts_with("192.0.2.")
        || h.starts_with("198.51.100.")
        || h.starts_with("203.0.113.")
        || h.starts_with("2001:db8")
        || h.contains("example")
        || h.starts_with('<')
}
