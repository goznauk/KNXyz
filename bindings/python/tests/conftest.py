"""Shared fixtures: the loopback knx-sim gateway fixture binary.

Spawns the test-only Rust binary ``knx_sim_lifecycle_fixture``
(crates/knx-sim/src/bin/), which binds an EPHEMERAL localhost UDP port
(no collision risk), prints ``PORT=<port>``, accepts exactly one tunnel
CONNECT_REQUEST, and then either waits for stdin EOF (``lifecycle``
mode), answers one SEARCH_REQUEST without any tunnel handshake
(``discovery`` mode), or serves scripted tunnelling requests (``group``
mode, echoing
each observed frame as a ``FRAME={...}`` stdout line and answering
GroupValueRead with a GroupValueResponse indication). A 60s safety
timeout guarantees no orphaned process. Loopback only — no real KNX,
no external automation systems, no external network. Tests are skipped only when
cargo (and a prebuilt binary) is genuinely unavailable.
"""

import json
import queue
import shutil
import subprocess
import threading
from pathlib import Path

import pytest

_REPO_ROOT = Path(__file__).resolve().parents[3]
_FIXTURE_BIN = _REPO_ROOT / "target" / "debug" / "knx_sim_lifecycle_fixture"


def pytest_configure(config):
    # Register the opt-in real-device marker so it does not warn. Tests carrying
    # this marker are env-gated and skip by default; they never run in CI.
    config.addinivalue_line(
        "markers",
        "reference_device: opt-in real-device validation against a real KNX gateway"
        " (env-gated; never runs in CI)",
    )


@pytest.fixture(scope="session")
def sim_fixture_binary() -> Path:
    if shutil.which("cargo") is not None:
        build = subprocess.run(
            ["cargo", "build", "-p", "knx-sim", "--bin", "knx_sim_lifecycle_fixture"],
            cwd=_REPO_ROOT,
            capture_output=True,
            text=True,
            timeout=600,
        )
        if build.returncode != 0:
            pytest.fail(f"failed to build knx_sim_lifecycle_fixture:\n{build.stderr}")
    elif not _FIXTURE_BIN.exists():
        pytest.skip("cargo unavailable and no prebuilt knx_sim_lifecycle_fixture")
    return _FIXTURE_BIN


class SimLifecycleGateway:
    def __init__(self, process: subprocess.Popen):
        self._process = process
        self._lines: queue.Queue[str] = queue.Queue()
        self._reader = threading.Thread(target=self._pump_stdout, daemon=True)
        self._reader.start()
        self.port = self._read_port()

    def _pump_stdout(self) -> None:
        assert self._process.stdout is not None
        for line in self._process.stdout:
            self._lines.put(line)

    def _next_line(self, timeout: float) -> str:
        try:
            return self._lines.get(timeout=timeout)
        except queue.Empty:
            raise RuntimeError("sim fixture produced no output in time") from None

    def _read_port(self) -> int:
        try:
            line = self._next_line(timeout=10)
        except RuntimeError:
            self._process.kill()
            self._process.wait(timeout=5)
            raise
        if not line.startswith("PORT="):
            self._process.kill()
            self._process.wait(timeout=5)
            raise RuntimeError(f"sim fixture did not report a port: {line!r}")
        return int(line.strip().removeprefix("PORT="))

    def read_frame(self, timeout: float = 5.0) -> dict:
        """Return the next FRAME line (group mode) as a parsed dict."""
        while True:
            line = self._next_line(timeout=timeout)
            if line.startswith("FRAME="):
                return json.loads(line.strip().removeprefix("FRAME="))

    def read_disconnect(self, timeout: float = 5.0) -> dict:
        """Return the next DISCONNECT line (orderly teardown) as a dict.

        The fixture emits this when the client sends a real KNXnet/IP
        DISCONNECT_REQUEST; skips any FRAME lines in between.
        """
        while True:
            line = self._next_line(timeout=timeout)
            if line.startswith("DISCONNECT="):
                return json.loads(line.strip().removeprefix("DISCONNECT="))

    def read_response_or_silence(self, timeout: float = 5.0):
        """Expose mode: return the next FRAME dict, or None on NO_RESPONSE.

        The `expose` fixture sends a GroupValueRead to the client and then
        either records the client's GroupValueResponse as a FRAME line
        (respond_to_read answered) or prints NO_RESPONSE when the client
        stays silent (respond_to_read=False or no value observed yet).
        """
        while True:
            line = self._next_line(timeout=timeout)
            if line.startswith("FRAME="):
                return json.loads(line.strip().removeprefix("FRAME="))
            if line.startswith("NO_RESPONSE"):
                return None

    def close(self) -> None:
        if self._process.poll() is None:
            assert self._process.stdin is not None
            try:
                self._process.stdin.close()
                self._process.wait(timeout=5)
            except (OSError, subprocess.TimeoutExpired):
                self._process.kill()
                self._process.wait(timeout=5)


@pytest.fixture
def make_sim_gateway(sim_fixture_binary: Path):
    gateways: list[SimLifecycleGateway] = []

    def factory(*args: str) -> SimLifecycleGateway:
        process = subprocess.Popen(
            [str(sim_fixture_binary), *args],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        gateway = SimLifecycleGateway(process)
        gateways.append(gateway)
        return gateway

    try:
        yield factory
    finally:
        for gateway in gateways:
            gateway.close()


@pytest.fixture
def sim_gateway(make_sim_gateway):
    return make_sim_gateway()


@pytest.fixture
def sim_group_gateway(make_sim_gateway):
    return make_sim_gateway("group")
