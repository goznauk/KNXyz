"""Offline Cython cimport smoke for the public knxyz C API."""

import importlib.util
import os
import subprocess
import sys
from pathlib import Path

import pytest

_ROOT = Path(__file__).resolve().parents[3]
_EXAMPLE = _ROOT / "examples" / "python" / "cython"


def test_cython_example_builds_and_round_trips(tmp_path):
    pytest.importorskip("Cython")
    pytest.importorskip("setuptools")
    if importlib.util.find_spec("knxyz") is None:
        pytest.skip("knxyz is not importable in this environment")
    assert (_ROOT / "bindings" / "python" / "knxyz" / "capi.pxd").is_file()
    assert (
        _ROOT / "bindings" / "python" / "knxyz" / "include" / "knxyz" / "capi.h"
    ).is_file()
    smoke_source = (_EXAMPLE / "knxyz_cython_smoke.pyx").read_text(encoding="utf-8")
    assert "from knxyz.capi cimport" in smoke_source
    assert "from knxyz import dpt" not in smoke_source
    assert "knxyz_dpt9_001_encode_result_t" in smoke_source
    assert "payload.bytes" in smoke_source
    assert "knxyz_import_capi(_native._C_API)" in smoke_source

    target = tmp_path / "site"
    build = subprocess.run(
        [
            sys.executable,
            "-m",
            "pip",
            "install",
            "--no-build-isolation",
            "--no-deps",
            "--target",
            str(target),
            str(_EXAMPLE),
        ],
        cwd=str(_ROOT),
        capture_output=True,
        text=True,
        timeout=120,
    )
    assert build.returncode == 0, "Cython example build failed:\n%s\n%s" % (
        build.stdout,
        build.stderr,
    )

    env = os.environ.copy()
    existing = env.get("PYTHONPATH")
    env["PYTHONPATH"] = str(target) if not existing else str(target) + os.pathsep + existing
    smoke = subprocess.run(
        [
            sys.executable,
            "-c",
            (
                "import knxyz_cython_smoke as s; "
                "assert s.round_trip_temperature() == ('0c1a', 21.0); "
                "assert s.negative_temperature_payload() == '860c'; "
                "assert s.smoke_summary() == 'capi-ok'"
            ),
        ],
        cwd=str(_ROOT),
        env=env,
        capture_output=True,
        text=True,
        timeout=60,
    )
    assert smoke.returncode == 0, "Cython example smoke failed:\n%s\n%s" % (
        smoke.stdout,
        smoke.stderr,
    )
