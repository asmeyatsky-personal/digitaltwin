#!/usr/bin/env python3
"""Enforces §5 coverage floors.

Inputs: cargo-llvm-cov JSON (summary-by-function).
Floors:
  - any crate matching *-domain OR `kernel`: >= 95%
  - any crate matching *-application:        >= 85%
  - workspace overall:                       >= 80%
Fails loudly if any floor is breached.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path


def crate_of(path: str) -> str:
    # llvm-cov filenames look like: crates/identity-domain/src/user.rs
    parts = Path(path).parts
    if len(parts) >= 2 and parts[0] in ("crates", "services"):
        return parts[1]
    return parts[0] if parts else "unknown"


def floor_for(crate: str) -> float:
    if crate == "kernel" or crate.endswith("-domain"):
        return 95.0
    if crate.endswith("-application"):
        return 85.0
    return 0.0  # no per-crate floor; overall 80% handles the rest


def pct(covered: int, total: int) -> float:
    return 100.0 * covered / total if total else 100.0


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: check-coverage.py <coverage.json>", file=sys.stderr)
        return 2

    data = json.loads(Path(sys.argv[1]).read_text())
    per_crate_covered: dict[str, int] = {}
    per_crate_total: dict[str, int] = {}

    # llvm-cov JSON: data[0].files[].summary.lines.{covered,count}
    for export in data.get("data", []):
        for file_entry in export.get("files", []):
            filename = file_entry.get("filename", "")
            summary = file_entry.get("summary", {}).get("lines", {})
            covered = int(summary.get("covered", 0))
            total = int(summary.get("count", 0))
            crate = crate_of(filename.removeprefix(str(Path.cwd()) + "/"))
            per_crate_covered[crate] = per_crate_covered.get(crate, 0) + covered
            per_crate_total[crate]   = per_crate_total.get(crate, 0)   + total

    overall_covered = sum(per_crate_covered.values())
    overall_total   = sum(per_crate_total.values())
    overall = pct(overall_covered, overall_total)

    failures: list[str] = []

    print(f"{'crate':<40} {'covered':>10} {'total':>10} {'pct':>7} {'floor':>7}")
    for crate in sorted(per_crate_total):
        total = per_crate_total[crate]
        if total == 0:
            continue
        covered = per_crate_covered[crate]
        p = pct(covered, total)
        floor = floor_for(crate)
        mark = "OK" if p >= floor else "FAIL"
        print(f"{crate:<40} {covered:>10} {total:>10} {p:>6.2f}% {floor:>6.1f}% {mark}")
        if p < floor:
            failures.append(f"{crate}: {p:.2f}% < {floor:.1f}%")

    print(f"{'overall':<40} {overall_covered:>10} {overall_total:>10} {overall:>6.2f}% {80.0:>6.1f}%")
    if overall < 80.0:
        failures.append(f"overall: {overall:.2f}% < 80.0%")

    if failures:
        print("\nCoverage floor failures:", file=sys.stderr)
        for f in failures:
            print(f"  - {f}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
