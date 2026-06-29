"""KNXyz example: boolean group write, DPT 1.001.

The default run is a dry run. Live mode requires ``--live``,
``KNXYZ_EXAMPLE_ALLOW_LIVE_WRITE=1``, the confirmation flag, and all telegram
arguments. Placeholder hosts are rejected, and the live tunnel is closed in a
``finally`` block. See examples/README.md.
"""

import argparse
import asyncio
import os
import sys

# RFC 5737 TEST-NET / RFC 3849 documentation ranges and placeholder tokens.
_PLACEHOLDER_PREFIXES = ("192.0.2.", "198.51.100.", "203.0.113.", "2001:db8")


def _is_placeholder(host: str) -> bool:
    normalized = host[1:] if host.startswith("[") else host
    return (
        normalized.startswith(_PLACEHOLDER_PREFIXES)
        or "example" in normalized
        or normalized.startswith("<")
    )


def _is_ci() -> bool:
    return os.environ.get("CI", "") not in ("", "0", "false")


def _parse_boolean_write_value(dpt: str, value: str) -> bool:
    if dpt != "1.001":
        raise ValueError(f"this example demonstrates DPT 1.001 boolean writes; got {dpt}")

    lowered = value.lower()
    if lowered == "true":
        return True
    if lowered == "false":
        return False

    raise ValueError(f"this example accepts only --value true or --value false; got {value}")


async def main() -> None:
    parser = argparse.ArgumentParser(
        description="KNXyz group-write example (dry-run by default; use --live to connect)"
    )
    parser.add_argument("--live", action="store_true")
    parser.add_argument("--confirm")
    # Live-required params default to None so live mode can require every
    # telegram argument explicitly.
    parser.add_argument("--gateway-host", default=None)
    parser.add_argument("--port", type=int, default=3671)
    parser.add_argument("--group-address", default=None)
    parser.add_argument("--dpt", default=None)
    parser.add_argument("--value", default=None)
    args = parser.parse_args()

    armed = (
        args.live
        and os.environ.get("KNXYZ_EXAMPLE_ALLOW_LIVE_WRITE") == "1"
        and args.confirm == "ISOLATED_TEST_BUS_ONLY"
        and args.gateway_host is not None
        and args.group_address is not None
        and args.dpt is not None
        and args.value is not None
    )
    if not armed:
        host = args.gateway_host or "203.0.113.10"
        group_address = args.group_address or "1/2/3"
        dpt = args.dpt or "1.001"
        value = args.value or "true"
        print(
            f"DRY-RUN: would write GA={group_address} DPT={dpt} "
            f"value={value} to {host}:{args.port} (no connection made).\n"
            "For a live write, pass --live "
            "--confirm ISOLATED_TEST_BUS_ONLY --gateway-host <host> --group-address <ga> "
            "--dpt 1.001 --value true and set KNXYZ_EXAMPLE_ALLOW_LIVE_WRITE=1. "
            "The confirmation flag is required for write examples."
        )
        return

    # Live path: require explicit arguments, confirmation, environment variable, and
    # non-placeholder host.
    if _is_ci():
        sys.exit("refusing a live write under CI")
    if _is_placeholder(args.gateway_host):
        sys.exit(
            "refusing a live write to a documentation/placeholder host; "
            "pass --gateway-host for your gateway"
        )

    try:
        parsed_value = _parse_boolean_write_value(args.dpt, args.value)
    except ValueError as exc:
        sys.exit(str(exc))

    from knxyz import connect_tunnel  # imported only on the armed live path

    print(
        f"LIVE: writing {parsed_value} as DPT={args.dpt} to GA={args.group_address} on "
        f"{args.gateway_host}..."
    )
    client = await connect_tunnel(host=args.gateway_host, port=args.port)
    try:
        await client.write(args.group_address, parsed_value, args.dpt)
        print("done")
    finally:
        await client.close()  # close the tunnel regardless of the write result


if __name__ == "__main__":
    asyncio.run(main())
