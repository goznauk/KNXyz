"""KNXyz example - group READ (DEFAULT-SAFE: dry-run).

SAFETY: a ``GroupValueRead`` is an ACTIVE bus telegram (it solicits the bus), so
by default this performs NO bus I/O. It prints the read it WOULD perform and
exits WITHOUT opening a socket (it does not even import the client on the default
path). A real live read is advanced-only and ISOLATED-BUS-ONLY: it requires
``--live``, ``KNXYZ_EXAMPLE_ALLOW_LIVE=1``, and explicit
``--gateway-host``/``--group-address``/``--dpt``; it refuses
documentation/placeholder hosts and refuses to run under CI. The live path is
read-only and closes the tunnel in a ``finally`` block. NEVER point this at a
production or shared KNX bus. See examples/README.md.
"""

import argparse
import asyncio
import os
import sys

# RFC 5737 TEST-NET / RFC 3849 documentation ranges + symbolic tokens; refused live
_PLACEHOLDER_PREFIXES = ("192.0.2.", "198.51.100.", "203.0.113.", "2001:db8")


def _is_placeholder(host: str) -> bool:
    return host.startswith(_PLACEHOLDER_PREFIXES) or "example" in host or host.startswith("<")


def _is_ci() -> bool:
    return os.environ.get("CI", "") not in ("", "0", "false")


async def main() -> None:
    parser = argparse.ArgumentParser(
        description="KNXyz group-read example (dry-run by default; live = isolated bus only)"
    )
    parser.add_argument("--live", action="store_true")
    # live-required params default to None so the live path can require them
    # EXPLICITLY (no silent default GA/DPT); dry-run falls back to a documentation
    # placeholder host (RFC 5737 TEST-NET, refused in live mode).
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
            "For a REAL read on an ISOLATED test bus ONLY, pass --live "
            "--gateway-host <host> --group-address <ga> --dpt 9.001 and set "
            "KNXYZ_EXAMPLE_ALLOW_LIVE=1. NEVER target a production/shared bus."
        )
        return

    # live path: advanced, isolated-bus-only, fail-closed
    if _is_ci():
        sys.exit("refusing a live read under CI")
    if _is_placeholder(args.gateway_host):
        sys.exit(
            "refusing a live read against a documentation/placeholder host; "
            "pass --gateway-host for your own isolated test gateway"
        )

    from knxyz import connect_tunnel  # imported only on the armed live path

    print(
        f"LIVE: reading GA={args.group_address} as DPT={args.dpt} from "
        f"{args.gateway_host} (ISOLATED test bus)..."
    )
    client = await connect_tunnel(host=args.gateway_host, port=args.port)
    try:
        value = await client.read(args.group_address, args.dpt)
        print(value)
    finally:
        await client.close()  # deterministic teardown


if __name__ == "__main__":
    asyncio.run(main())
