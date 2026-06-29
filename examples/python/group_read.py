"""KNXyz example: group read.

The default run is a dry run. Live mode requires ``--live``,
``KNXYZ_EXAMPLE_ALLOW_LIVE=1``, and explicit gateway, group address, and DPT
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


async def main() -> None:
    parser = argparse.ArgumentParser(
        description="KNXyz group-read example (dry-run by default; use --live to connect)"
    )
    parser.add_argument("--live", action="store_true")
    # Live-required params default to None so live mode can require explicit
    # gateway, group address, and DPT arguments.
    parser.add_argument("--gateway-host", default=None)
    parser.add_argument("--port", type=int, default=3671)
    parser.add_argument("--group-address", default=None)
    parser.add_argument("--dpt", default=None)
    args = parser.parse_args()

    armed = (
        args.live
        and os.environ.get("KNXYZ_EXAMPLE_ALLOW_LIVE") == "1"
        and args.gateway_host is not None
        and args.group_address is not None
        and args.dpt is not None
    )
    if not armed:
        host = args.gateway_host or "knxip.example"
        group_address = args.group_address or "1/0/0"
        dpt = args.dpt or "9.001"
        print(
            f"DRY-RUN: would read GA={group_address} DPT={dpt} "
            f"from {host}:{args.port} (no connection made).\n"
            "For a live read, pass --live "
            "--gateway-host <host> --group-address <ga> --dpt 9.001 and set "
            "KNXYZ_EXAMPLE_ALLOW_LIVE=1."
        )
        return

    # Live path: require explicit arguments, environment variable, and non-placeholder host.
    if _is_ci():
        sys.exit("refusing a live read under CI")
    if _is_placeholder(args.gateway_host):
        sys.exit(
            "refusing a live read against a documentation/placeholder host; "
            "pass --gateway-host for your gateway"
        )

    from knxyz import connect_tunnel  # imported only on the armed live path

    print(
        f"LIVE: reading GA={args.group_address} as DPT={args.dpt} from "
        f"{args.gateway_host}..."
    )
    client = await connect_tunnel(host=args.gateway_host, port=args.port)
    try:
        value = await client.read(args.group_address, args.dpt)
        print(value)
    finally:
        await client.close()  # close the tunnel regardless of the read result


if __name__ == "__main__":
    asyncio.run(main())
