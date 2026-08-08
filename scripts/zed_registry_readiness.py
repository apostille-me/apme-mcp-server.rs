#!/usr/bin/env python3
"""Probe the exact Apostille Me Zed package graph without credentials.

HTTP 404 and a package with no compatible 0.1.x release are publication-readiness
states, not transport failures. The script records those states, emits a
GitHub Actions boolean output, and exits successfully. Unexpected HTTP status,
malformed registry data, or network failure remain hard errors.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import re
import sys
import tomllib
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Any

EXPECTED_DEPENDENCIES = {
    "apostille-me/apme-cli",
    "apostille-me/apme-clients",
    "apostille-me/apme-interfaces",
    "apostille-me/apme-libs",
    "apostille-me/apme-sync",
    "shared-auth/shared-auth-clients",
}
COMPATIBLE_VERSION = re.compile(r"^0\.1\.\d+$")
CREDENTIAL_PREFIXES = (
    "gh" + "p_",
    "github" + "_pat_",
    "cf" + "at_",
    "lin" + "_api_",
    "AKIA",
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--registry", required=True)
    parser.add_argument("--zed-cli-revision", required=True)
    parser.add_argument("--evidence", type=Path, required=True)
    return parser.parse_args()


def append_line(path: str | None, line: str) -> None:
    if not path:
        return
    with Path(path).open("a", encoding="utf-8") as stream:
        stream.write(line)
        stream.write("\n")


def fail(message: str) -> "NoReturn":
    raise RuntimeError(message)


def probe_package(registry: str, coordinate: str, requirement: str) -> tuple[dict[str, Any], str | None]:
    org, name = coordinate.split("/", 1)
    url = (
        f"{registry.rstrip('/')}/v1/packages/"
        f"{urllib.parse.quote(org, safe='')}/{urllib.parse.quote(name, safe='')}"
    )
    request = urllib.request.Request(
        url,
        headers={
            "Accept": "application/json",
            "User-Agent": "apme-zed-registry-readiness/1",
        },
    )
    item: dict[str, Any] = {
        "coordinate": coordinate,
        "requirement": requirement,
        "url": url,
    }

    try:
        with urllib.request.urlopen(request, timeout=20) as response:
            item["httpStatus"] = response.status
            if response.status != 200:
                fail(f"{coordinate}: unexpected HTTP {response.status}")
            payload = json.load(response)
    except urllib.error.HTTPError as error:
        item["httpStatus"] = error.code
        if error.code == 404:
            item["state"] = "not-published"
            item["compatibleVersions"] = []
            return item, "not-published"
        fail(f"{coordinate}: unexpected HTTP {error.code}")
    except (urllib.error.URLError, TimeoutError, OSError) as error:
        fail(f"{coordinate}: registry transport failed ({type(error).__name__})")
    except json.JSONDecodeError:
        fail(f"{coordinate}: registry returned malformed JSON")

    if not isinstance(payload, dict):
        fail(f"{coordinate}: registry payload must be an object")
    versions = payload.get("versions", [])
    if not isinstance(versions, list) or not all(isinstance(value, str) for value in versions):
        fail(f"{coordinate}: registry versions must be a string array")

    compatible = sorted(value for value in versions if COMPATIBLE_VERSION.fullmatch(value))
    item.update(
        {
            "state": "ready" if compatible else "no-compatible-version",
            "latest": payload.get("latest"),
            "versions": sorted(versions),
            "compatibleVersions": compatible,
        }
    )
    return item, None if compatible else "no-compatible-version"


def write_summary(evidence: dict[str, Any]) -> None:
    summary_path = os.environ.get("GITHUB_STEP_SUMMARY")
    if not summary_path:
        return

    lines = [
        "## Apostille Me Zed registry readiness",
        "",
        f"Registry: `{evidence['registry']}`",
        "",
        "| Package | Requirement | HTTP | State | Compatible versions |",
        "|---|---:|---:|---|---|",
    ]
    for package in evidence["packages"]:
        versions = ", ".join(package.get("compatibleVersions", [])) or "—"
        lines.append(
            f"| `{package['coordinate']}` | `{package['requirement']}` | "
            f"{package.get('httpStatus', '—')} | `{package['state']}` | {versions} |"
        )
    lines.extend(
        [
            "",
            f"**Ready for resolver-generated lock:** `{str(evidence['allReady']).lower()}`",
            "",
            "A non-ready result is publication-readiness evidence only. The resolver and frozen-replay job remains skipped until every exact coordinate has a compatible published `0.1.x` artifact.",
        ]
    )
    append_line(summary_path, "\n".join(lines))


def main() -> int:
    args = parse_args()
    registry = args.registry.rstrip("/")
    manifest = tomllib.loads(args.manifest.read_text(encoding="utf-8"))
    dependencies = manifest.get("dependencies", {})
    if not isinstance(dependencies, dict):
        fail("manifest dependencies must be a table")
    if set(dependencies) != EXPECTED_DEPENDENCIES:
        fail(f"unexpected dependency set: {sorted(dependencies)}")
    if not all(isinstance(value, str) and value == "^0.1.0" for value in dependencies.values()):
        fail("every wave-2 dependency must retain requirement ^0.1.0")

    evidence: dict[str, Any] = {
        "schemaVersion": 1,
        "observedAt": dt.datetime.now(dt.timezone.utc).isoformat(),
        "registry": registry,
        "zedCliRevision": args.zed_cli_revision,
        "consumerManifest": str(args.manifest),
        "allReady": False,
        "packages": [],
        "blocked": [],
    }

    for coordinate, requirement in sorted(dependencies.items()):
        item, blocked_reason = probe_package(registry, coordinate, requirement)
        evidence["packages"].append(item)
        if blocked_reason:
            evidence["blocked"].append(
                {"coordinate": coordinate, "reason": blocked_reason}
            )

    evidence["allReady"] = not evidence["blocked"]
    args.evidence.write_text(
        json.dumps(evidence, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )

    serialized = args.evidence.read_text(encoding="utf-8")
    for prefix in CREDENTIAL_PREFIXES:
        if prefix in serialized:
            fail(f"credential-shaped value found in {args.evidence}")

    append_line(os.environ.get("GITHUB_OUTPUT"), f"ready={str(evidence['allReady']).lower()}")
    append_line(os.environ.get("GITHUB_OUTPUT"), f"blocked_count={len(evidence['blocked'])}")
    write_summary(evidence)

    if evidence["allReady"]:
        print(f"all {len(evidence['packages'])} package coordinates are ready")
    else:
        print(
            f"registry probe completed: {len(evidence['blocked'])} of "
            f"{len(evidence['packages'])} coordinates are not ready"
        )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (RuntimeError, ValueError, OSError, tomllib.TOMLDecodeError) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
