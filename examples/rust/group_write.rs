//! KNXyz example: boolean group write, DPT 1.001.
//!
//! The default run is a dry run. Live mode requires `--live`,
//! `KNXYZ_EXAMPLE_ALLOW_LIVE_WRITE=1`, the confirmation flag, and all telegram
//! arguments. Placeholder hosts are rejected. See examples/README.md.

use std::net::SocketAddr;

use knxyz::ip::TunnelClient;
use knxyz::{DptValue, GroupAddress};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Example-only argument parsing.
    let args: Vec<String> = std::env::args().collect();
    let has = |name: &str| args.iter().any(|a| a == name);
    let value_of = |name: &str| -> Option<String> {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };

    // Dry-run output uses a documentation placeholder host.
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

    // Live writes require every telegram parameter explicitly; otherwise the
    // example stays on the dry-run path.
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
             For a live write, pass:\n  \
             --live --confirm ISOLATED_TEST_BUS_ONLY --gateway-host <host:port> \
             --group-address <ga> --dpt 1.001 --value true\n  \
             and set KNXYZ_EXAMPLE_ALLOW_LIVE_WRITE=1."
        );
        return Ok(());
    }

    let dpt_value = parse_group_write_value(&dpt, &value)?;

    // Live path: require explicit arguments, confirmation, environment variable, and
    // non-placeholder host.
    if is_ci() {
        return Err("refusing a live write under CI".into());
    }
    if is_placeholder_host(&host) {
        return Err(
            "refusing a live write to a documentation/placeholder host; pass \
             --gateway-host for your gateway"
                .into(),
        );
    }
    let gateway: SocketAddr = host.parse()?;
    let group: GroupAddress = group_address.parse()?;
    println!("LIVE: writing boolean {value} as DPT={dpt} to GA={group_address} on {host}...");
    let mut client = TunnelClient::connect(gateway).await?;
    let result = client.group_write(group, dpt_value).await;
    let _ = client.disconnect().await; // close the tunnel regardless of the write result
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
            "this example demonstrates DPT 1.001 boolean writes; got {dpt}"
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
    let normalized = host.trim_start_matches('[');
    if normalized.starts_with("2001:db8") {
        return true;
    }

    let h = normalized
        .split(']')
        .next()
        .unwrap_or(normalized)
        .split(':')
        .next()
        .unwrap_or(normalized);

    h.starts_with("192.0.2.")
        || h.starts_with("198.51.100.")
        || h.starts_with("203.0.113.")
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
        assert!(err.contains("demonstrates DPT 1.001 boolean writes"));
    }

    #[test]
    fn rejects_invalid_boolean_literals() {
        let err = parse_group_write_value("1.001", "21.0").unwrap_err();
        assert!(err.contains("--value true or --value false"));
    }

    #[test]
    fn rejects_documentation_and_placeholder_hosts() {
        assert!(is_placeholder_host("[2001:db8::1]:3671"));
        assert!(is_placeholder_host("2001:db8::1"));
        assert!(is_placeholder_host("192.0.2.10:3671"));
        assert!(is_placeholder_host("example-gateway.local"));
        assert!(is_placeholder_host("<gateway-host>"));
    }

    #[test]
    fn accepts_private_ipv4_hosts() {
        assert!(!is_placeholder_host("10.0.0.5:3671"));
    }
}
