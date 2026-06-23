#!/usr/bin/env python3
"""Verify KNXyz package artifacts before any registry publish.

The script is intentionally read-only. It inspects already-built Python
sdist/wheel artifacts and npm pack dry-run JSON output, then fails if known
internal or build-only paths would ship.
"""

from __future__ import annotations

import argparse
import json
import sys
import tarfile
import zipfile
from pathlib import Path


PYTHON_FORBIDDEN = (
    "/bindings/python/openknx/",
    "/bindings/python/tests/",
    "/crates/knx-core/tests/",
    "/crates/knx-dpt/tests/",
    "/crates/knx-ip/tests/",
    "/crates/knx-sim/tests/",
    "/docs/",
    "/apps/",
    "/fuzz/",
    "/refs/",
    "/tools/",
    "/target/",
    "/node_modules/",
    ".pytest_cache",
    ".DS_Store",
)

PYTHON_SDIST_REQUIRED_SUFFIXES = (
    "/PKG-INFO",
    "/Cargo.lock",
    "/Cargo.toml",
    "/pyproject.toml",
    "/bindings/python/Cargo.toml",
    "/bindings/python/build.rs",
    "/bindings/python/src/lib.rs",
    "/crates/knx-core/Cargo.toml",
    "/crates/knx-core/src/lib.rs",
    "/crates/knx-dpt/Cargo.toml",
    "/crates/knx-dpt/src/lib.rs",
    "/crates/knx-ip/Cargo.toml",
    "/crates/knx-ip/src/lib.rs",
    "/knxyz/__init__.py",
    "/knxyz/dpt.py",
    "/knxyz/py.typed",
)

PYTHON_WHEEL_REQUIRED = (
    "knxyz/__init__.py",
    "knxyz/dpt.py",
    "knxyz/py.typed",
)

NODE_ALLOWED_EXACT = {"package.json", "src/index.ts"}
NODE_ALLOWED_PREFIXES = ("index.", "knxyz-node.")
NODE_ALLOWED_SUFFIXES = (".node",)
NODE_FORBIDDEN_PREFIXES = ("test/", "scripts/", "node_modules/")
NODE_FORBIDDEN_EXACT = {
    "Cargo.toml",
    "Cargo.lock",
    "build.rs",
    "package-lock.json",
    "src/lib.rs",
    "index.d.ts",
}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--python-dist", type=Path)
    parser.add_argument("--node-pack-json", type=Path)
    parser.add_argument("--node-package-json", type=Path)
    args = parser.parse_args()

    failures: list[str] = []
    if args.python_dist:
        failures.extend(check_python_dist(args.python_dist))
    if args.node_pack_json:
        failures.extend(check_node_pack_json(args.node_pack_json, args.node_package_json))
    if not args.python_dist and not args.node_pack_json:
        failures.append("no artifact inputs supplied")

    if failures:
        for failure in failures:
            print(f"FAIL: {failure}", file=sys.stderr)
        return 1

    print("package artifact checks passed")
    return 0


def check_python_dist(dist_dir: Path) -> list[str]:
    failures: list[str] = []
    sdists = sorted(dist_dir.glob("*.tar.gz"))
    wheels = sorted(dist_dir.glob("*.whl"))

    if len(sdists) != 1:
        failures.append(f"expected exactly one Python sdist in {dist_dir}, found {len(sdists)}")
    if len(wheels) != 1:
        failures.append(f"expected exactly one Python wheel in {dist_dir}, found {len(wheels)}")

    for sdist in sdists:
        with tarfile.open(sdist, "r:gz") as archive:
            names = archive.getnames()
        failures.extend(check_forbidden("Python sdist", names, PYTHON_FORBIDDEN))
        failures.extend(require_suffixes("Python sdist", names, PYTHON_SDIST_REQUIRED_SUFFIXES))

    for wheel in wheels:
        with zipfile.ZipFile(wheel) as archive:
            names = archive.namelist()
        failures.extend(check_forbidden("Python wheel", names, PYTHON_FORBIDDEN))
        failures.extend(require_exact("Python wheel", names, PYTHON_WHEEL_REQUIRED))
        if not any(name.startswith("knxyz/_knxyz") and name.endswith(".so") for name in names):
            failures.append("Python wheel missing native knxyz/_knxyz*.so extension")

    return failures


def check_node_pack_json(pack_json: Path, package_json: Path | None) -> list[str]:
    failures: list[str] = []
    data = json.loads(pack_json.read_text())
    if not isinstance(data, list) or not data:
        return ["npm pack JSON did not contain a package list"]

    files = data[0].get("files", [])
    names = sorted(file_info.get("path", "") for file_info in files)
    if not names:
        failures.append("npm pack JSON listed no files")

    for name in names:
        if name in NODE_FORBIDDEN_EXACT or name.startswith(NODE_FORBIDDEN_PREFIXES):
            failures.append(f"npm package contains forbidden path {name}")
        if is_node_allowed(name):
            continue
        failures.append(f"npm package contains unexpected path {name}")

    if "src/index.ts" not in names:
        failures.append("npm package missing src/index.ts")
    if not any(is_native_node_artifact(name) for name in names):
        failures.append("npm package missing a native .node artifact")
    failures.extend(check_node_entrypoints(pack_json.parent, names, package_json))

    return failures


def is_node_allowed(name: str) -> bool:
    return name in NODE_ALLOWED_EXACT or is_native_node_artifact(name)


def is_native_node_artifact(name: str) -> bool:
    return name.startswith(NODE_ALLOWED_PREFIXES) and name.endswith(NODE_ALLOWED_SUFFIXES)


def check_node_entrypoints(
    pack_json_dir: Path, names: list[str], package_json: Path | None
) -> list[str]:
    package_json_path = package_json or find_node_package_json(pack_json_dir)
    if package_json_path is None:
        return ["could not locate bindings/node/package.json for npm entrypoint checks"]

    package_json = json.loads(package_json_path.read_text())
    required = {
        normalize_npm_path(package_json.get("main")),
        normalize_npm_path(package_json.get("types")),
    }
    exports = package_json.get("exports", {}).get(".", {})
    if isinstance(exports, dict):
        required.add(normalize_npm_path(exports.get("types")))
        required.add(normalize_npm_path(exports.get("import")))
        required.add(normalize_npm_path(exports.get("default")))
    else:
        required.add(normalize_npm_path(exports))

    failures: list[str] = []
    for path in sorted(path for path in required if path):
        if path not in names:
            failures.append(f"npm package entrypoint {path} is not included in tarball")
    return failures


def find_node_package_json(start: Path) -> Path | None:
    for directory in [start, *start.parents]:
        candidate = directory / "bindings" / "node" / "package.json"
        if candidate.exists():
            return candidate
        candidate = directory / "package.json"
        if candidate.exists() and (directory / "src" / "index.ts").exists():
            return candidate
    return None


def normalize_npm_path(value: object) -> str | None:
    if not isinstance(value, str) or not value:
        return None
    return value[2:] if value.startswith("./") else value


def check_forbidden(label: str, names: list[str], forbidden: tuple[str, ...]) -> list[str]:
    failures: list[str] = []
    normalized = [f"/{name}" for name in names]
    for needle in forbidden:
        for name in normalized:
            if needle in name:
                failures.append(f"{label} contains forbidden path fragment {needle}: {name[1:]}")
    return failures


def require_suffixes(label: str, names: list[str], suffixes: tuple[str, ...]) -> list[str]:
    failures: list[str] = []
    for suffix in suffixes:
        if not any(name.endswith(suffix) for name in names):
            failures.append(f"{label} missing required path ending in {suffix}")
    return failures


def require_exact(label: str, names: list[str], required: tuple[str, ...]) -> list[str]:
    failures: list[str] = []
    for name in required:
        if name not in names:
            failures.append(f"{label} missing required path {name}")
    return failures


if __name__ == "__main__":
    raise SystemExit(main())
