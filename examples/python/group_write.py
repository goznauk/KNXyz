"""KNXyz example - group WRITE (DEFAULT-SAFE: dry-run).

SAFETY: by default this performs NO bus I/O. It prints the telegram it WOULD
write and exits WITHOUT opening a socket (it does not even import the client on
the default path). A real live write is advanced-only and ISOLATED-BUS-ONLY: it
requires ALL of ``--live``, ``KNXYZ_EXAMPLE_ALLOW_LIVE_WRITE=1``,
``--confirm ISOLATED_TEST_BUS_ONLY``, and explicit
``--gateway-host``/``--group-address``/``--dpt``/``--value``; it refuses
documentation/placeholder hosts and refuses to run under CI. NEVER point this at
a production or shared KNX bus. See examples/README.md.
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
        description="KNXyz group-write example (dry-run by default; live = isolated bus only)"
    )
    parser.add_argument("--live", action="store_true")
    parser.add_argument("--confirm")
    # live-required telegram params default to None so the live path can require
    # them EXPLICITLY (no silent default GA/DPT/value); dry-run falls back to a
    # documentation placeholder host (RFC 5737 TEST-NET, refused in live mode).
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
            "For a REAL write on an ISOLATED test bus ONLY, pass --live "
            "--confirm ISOLATED_TEST_BUS_ONLY --gateway-host <host> --group-address <ga> "
            "--dpt 1.001 --value true and set KNXYZ_EXAMPLE_ALLOW_LIVE_WRITE=1. "
            "NEVER target a production/shared bus."
        )
        return

    # live path: advanced, isolated-bus-only, fail-closed
    if _is_ci():
        sys.exit("refusing a live write under CI")
    if _is_placeholder(args.gateway_host):
        sys.exit(
            "refusing a live write to a documentation/placeholder host; "
            "pass --gateway-host for your own isolated test gateway"
        )

    from knxyz import connect_tunnel  # imported only on the armed live path

    print(
        f"LIVE: writing {args.value} to GA={args.group_address} on "
        f"{args.gateway_host} (ISOLATED test bus)..."
    )
    client = await connect_tunnel(host=args.gateway_host, port=args.port)
    try:
        await client.write(args.group_address, args.value == "true", args.dpt)
        print("done")
    finally:
        await client.close()  # deterministic teardown


if __name__ == "__main__":
    asyncio.run(main())
