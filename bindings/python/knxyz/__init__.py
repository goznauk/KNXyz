import asyncio
import json
from pathlib import Path

from . import _knxyz
from . import dpt


def get_include() -> str:
    """Return the installed include directory for Cython C API consumers."""
    return str(Path(__file__).with_name("include"))


def parse_individual_address(value: str) -> str:
    return _knxyz.parse_individual_address(value)


def format_individual_address(value: str) -> str:
    return _knxyz.format_individual_address(value)


def parse_group_address(value: str) -> str:
    return _knxyz.parse_group_address(value)


def format_group_address(value: str) -> str:
    return _knxyz.format_group_address(value)


def group_address_to_raw(value: str) -> int:
    """Validate ``value`` and return its raw 16-bit group address."""
    return _knxyz.group_address_to_raw(value)


def group_address_from_raw(raw: int) -> str:
    """Canonical 3-level group address string for a raw 16-bit value."""
    return _knxyz.group_address_from_raw(raw)


def individual_address_to_raw(value: str) -> int:
    """Validate ``value`` and return its raw 16-bit individual address."""
    return _knxyz.individual_address_to_raw(value)


def individual_address_from_raw(raw: int) -> str:
    """Canonical ``a.l.d`` individual address string for a raw value."""
    return _knxyz.individual_address_from_raw(raw)


async def discover_gateways(**options):
    gateways_json = await asyncio.to_thread(
        _knxyz.discover_gateways_json, json.dumps(options)
    )
    return json.loads(gateways_json)


async def connect_tunnel(**options):
    native = await asyncio.to_thread(
        _knxyz.NativeTunnelClient.connect, json.dumps(options)
    )
    return TunnelClient(native)


async def connect_routing(**options):
    """Open a native KNXnet/IP ROUTING client (connectionless multicast).

    Routing is fire-and-forget UDP multicast — there is no tunnelling
    connection, channel, or ack. Options:

    - ``multicast_group`` (str, default ``"224.0.23.12"``) and
      ``multicast_port`` (int, default ``3671``): the routing group.
    - ``local_ip`` / ``interface`` (str): the local IPv4 interface to
      send/join on. Set this to ``"127.0.0.1"`` (with ``loopback=True``)
      to keep traffic on the loopback NIC for local tests.
    - ``loopback`` (bool, default ``False``): IP_MULTICAST_LOOP — deliver
      our own sends back to local listeners (needed for on-host
      round-trips without any LAN egress).
    - ``ttl`` (int, default ``1``): multicast TTL; the default keeps a
      real send on the local segment.
    - ``source`` / ``individual_address`` (str, default ``"0.0.0"``): the
      source individual address stamped on sent telegrams.
    - ``bind`` (str): explicit ``ip:port`` for the receive socket
      (defaults to ``0.0.0.0:<multicast_port>``).

    This exposes the native KNXnet/IP routing transport.
    """
    native = await asyncio.to_thread(
        _knxyz.NativeRouteClient.connect, json.dumps(options)
    )
    return RouteClient(native)


class RouteClient:
    """Thin async wrapper over the native KNXnet/IP routing client.

    Routing is connectionless: ``send`` multicasts a GroupValueWrite
    RoutingIndication (no ack), and ``close`` simply drops the sockets
    (there is no disconnect frame). State is never inferred; incoming
    telegrams are observed via the monitor stream exactly as on the tunnel.
    """

    def __init__(self, native):
        self._native = native

    async def send(self, group_address: str, value, dpt: str) -> int:
        """Multicast a GroupValueWrite for ``value`` encoded as ``dpt``.

        Returns the number of bytes sent. Fire-and-forget: routing has no
        ack, so a successful send does not imply any device received or
        acted on the telegram, and no local state is mutated.
        """
        return await asyncio.to_thread(
            self._native.send, group_address, dpt, json.dumps(value)
        )

    async def start_monitor(self) -> None:
        """Join the multicast group and start receiving indications.

        Indications arriving before this call are not delivered: start the
        monitor before triggering the traffic you want to observe. Raises
        if the client is closed or a monitor is already started.
        """
        await asyncio.to_thread(self._native.monitor_start)

    def monitor_started(self) -> bool:
        return self._native.monitor_started()

    async def monitor_next(self, timeout_ms: int = 3000):
        """Wait for the next incoming RoutingIndication.

        Returns a dict with source, destination, APCI, raw payload bytes, and
        peer address. Decode payloads with
        ``knxyz.dpt.decode(dpt, bytes(payload))``. Raises on timeout, a closed
        client, a missing monitor, or monitor lag.
        """
        event_json = await asyncio.to_thread(
            self._native.monitor_next_json, timeout_ms
        )
        return json.loads(event_json)

    async def close(self) -> None:
        """Release the routing sockets (idempotent).

        Routing is connectionless, so there is no disconnect frame to
        send; this just drops the send/receive sockets. Later
        send/monitor calls raise "routing client is closed".
        """
        await asyncio.to_thread(self._native.close)

    def is_closed(self) -> bool:
        return self._native.is_closed()


class TunnelClient:
    def __init__(self, native):
        self._native = native

    async def write(self, group_address: str, value, dpt: str) -> None:
        await asyncio.to_thread(
            self._native.write, group_address, dpt, json.dumps(value)
        )

    async def read_request(self, group_address: str) -> None:
        """Send a GroupValueRead request without waiting for a response.

        Fire-and-forget: completes after the tunnelling ack. Any answer
        arrives as a bus indication observable via the monitor stream.
        """
        await asyncio.to_thread(self._native.read_request, group_address)

    async def respond(self, group_address: str, value, dpt: str) -> None:
        """Send a GroupValueResponse telegram (answering a group read)."""
        await asyncio.to_thread(
            self._native.respond, group_address, dpt, json.dumps(value)
        )

    async def read(self, group_address: str, dpt: str, timeout_ms: int = 3000):
        value_json = await asyncio.to_thread(
            self._native.read, group_address, dpt, timeout_ms
        )
        return json.loads(value_json)

    async def close(self) -> None:
        """Release the native tunnel client deterministically.

        When connected, sends ``DISCONNECT_REQUEST`` so the gateway can free the
        tunnel slot. The request is timeout-bounded; a missing or late response
        does not fail teardown. Closing twice is a no-op, and later write, read,
        or lifecycle-event calls raise after close.
        """
        await asyncio.to_thread(self._native.close)

    def is_closed(self) -> bool:
        return self._native.is_closed()

    async def lifecycle_events(self):
        """Recorded connection lifecycle events as a list of dicts.

        Shapes: {"type": "connected", "channel_id": n},
        {"type": "disconnected"},
        {"type": "reconnecting", "attempt": n, "delay_ms": n},
        {"type": "reconnected", "attempt": n, "channel_id": n}.
        """
        events_json = await asyncio.to_thread(self._native.lifecycle_events_json)
        return json.loads(events_json)

    async def start_monitor(self) -> None:
        """Subscribe to incoming group telegrams (bus indications).

        Indications arriving before this call are not delivered
        (broadcast semantics): start the monitor before triggering the
        traffic you want to observe. Raises if the client is closed or
        a monitor is already started.
        """
        await asyncio.to_thread(self._native.monitor_start)

    def monitor_started(self) -> bool:
        return self._native.monitor_started()

    async def monitor_next(self, timeout_ms: int = 3000):
        """Wait for the next incoming group telegram.

        Returns a dict with source, destination, APCI, and raw payload bytes.
        Decode payloads with ``knxyz.dpt.decode(dpt, bytes(payload))``. Raises
        on timeout, a closed client, a missing monitor, or monitor lag.
        """
        event_json = await asyncio.to_thread(
            self._native.monitor_next_json, timeout_ms
        )
        return json.loads(event_json)

    async def monitor(self):
        raise NotImplementedError(
            "use start_monitor() + monitor_next();"
            " an async-iterator monitor() facade is not available"
        )


__all__ = [
    "connect_routing",
    "connect_tunnel",
    "discover_gateways",
    "dpt",
    "format_group_address",
    "format_individual_address",
    "group_address_from_raw",
    "group_address_to_raw",
    "individual_address_from_raw",
    "individual_address_to_raw",
    "parse_group_address",
    "parse_individual_address",
    "RouteClient",
    "TunnelClient",
]
