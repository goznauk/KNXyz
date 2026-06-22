//! KNXyz example - boolean group WRITE, DPT 1.001 (DEFAULT-SAFE: dry-run).
//!
//! SAFETY: by default this performs NO bus I/O. It parses a planned telegram,
//! prints what it WOULD write, and exits WITHOUT opening a socket. A real live
//! write is advanced-only and ISOLATED-BUS-ONLY: it requires ALL of `--live`,
//! `KNXYZ_EXAMPLE_ALLOW_LIVE_WRITE=1`, `--confirm ISOLATED_TEST_BUS_ONLY`, and
//! explicit `--gateway-host`/`--group-address`/`--dpt 1.001`/`--value true|false`;
//! it refuses documentation/placeholder hosts and refuses to run under CI. NEVER
//! point this at a production or shared KNX bus. See examples/README.md.

use std::net::SocketAddr;

use knxyz::ip::TunnelClient;
use knxyz::{DptValue, GroupAddress};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // example-only argument parsing (no library involvement on the default path)
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
    let value_arg = value_of("--value");

    let host = gateway_host
        .clone()
        .unwrap_or_else(|| "203.0.113.10:3671".to_string());
    let group_address = group_address_arg
        .clone()
        .unwrap_or_else(|| "1/2/3".to_string());
    let dpt = dpt_arg.clone().unwrap_or_else(|| "1.001".to_string());
    let value = value_arg.clone().unwrap_or_else(|| "true".to_string());

    // a live write also requires EVERY telegram parameter to be passed
    // explicitly (no silent default GA/DPT/value on the live path); any missing
    // factor falls back to dry-run (fail-closed).
    let armed = has("--live")
        && std::env::var("KNXYZ_EXAMPLE_ALLOW_LIVE_WRITE").as_deref() == Ok("1")
        && value_of("--confirm").as_deref() == Some("ISOLATED_TEST_BUS_ONLY")
        && gateway_host.is_some()
        && group_address_arg.is_some()
        && dpt_arg.is_some()
        && value_arg.is_some();

    if !armed {
        println!(
            "DRY-RUN: would write boolean GA={group_address} DPT={dpt} value={value} to {host} \
             (no connection made).\n\
             For a REAL write on an ISOLATED test bus ONLY, pass:\n  \
             --live --confirm ISOLATED_TEST_BUS_ONLY --gateway-host <host:port> \
             --group-address <ga> --dpt 1.001 --value true\n  \
             and set KNXYZ_EXAMPLE_ALLOW_LIVE_WRITE=1. NEVER target a production/shared bus."
        );
        return Ok(());
    }

    let dpt_value = parse_group_write_value(&dpt, &value)?;

    // live path: advanced, isolated-bus-only, fail-closed
    if is_ci() {
        return Err("refusing a live write under CI".into());
    }
    if is_placeholder_host(&host) {
        return Err(
            "refusing a live write to a documentation/placeholder host; pass \
             --gateway-host for your own isolated test gateway"
                .into(),
        );
    }
    let gateway: SocketAddr = host.parse()?;
    let group: GroupAddress = group_address.parse()?;
    println!("LIVE: writing boolean {value} as DPT={dpt} to GA={group_address} on {host} (ISOLATED test bus)...");
    let mut client = TunnelClient::connect(gateway).await?;
    let result = client.group_write(group, dpt_value).await;
    let _ = client.disconnect().await; // best-effort teardown regardless of outcome
    result?;
    println!("done");
    Ok(())
}

fn is_ci() -> bool {
    std::env::var("CI")
        .map(|v| !v.is_empty() && v != "0" && v != "false")
        .unwrap_or(false)
}

fn parse_group_write_value(dpt: &str, value: &str) -> Result<DptValue, String> {
    if dpt != "1.001" {
        return Err(format!(
            "this boolean write example supports only DPT 1.001; got {dpt}"
        ));
    }

    parse_bool(value).map(DptValue::Bool)
}

fn parse_bool(value: &str) -> Result<bool, String> {
    if value.eq_ignore_ascii_case("true") {
        Ok(true)
    } else if value.eq_ignore_ascii_case("false") {
        Ok(false)
    } else {
        Err(format!(
            "this boolean write example accepts only --value true or --value false; got {value}"
        ))
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_supported_boolean_dpt_values() {
        assert_eq!(
            parse_group_write_value("1.001", "true").unwrap(),
            DptValue::Bool(true)
        );
        assert_eq!(
            parse_group_write_value("1.001", "false").unwrap(),
            DptValue::Bool(false)
        );
    }

    #[test]
    fn rejects_unsupported_dpt_before_live_write() {
        let err = parse_group_write_value("9.001", "21.0").unwrap_err();
        assert!(err.contains("supports only DPT 1.001"));
    }

    #[test]
    fn rejects_invalid_boolean_literals() {
        let err = parse_group_write_value("1.001", "21.0").unwrap_err();
        assert!(err.contains("--value true or --value false"));
    }
}
