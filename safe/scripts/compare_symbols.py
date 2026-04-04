#!/usr/bin/env python3

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path


def read_reference(path: Path) -> set[str]:
    return {
        line.strip()
        for line in path.read_text().splitlines()
        if line.strip() and not line.startswith("#")
    }


def read_library_symbols(path: Path) -> set[str]:
    output = subprocess.check_output(
        ["nm", "-D", "--defined-only", str(path)],
        text=True,
    )
    symbols: set[str] = set()
    for line in output.splitlines():
        parts = line.split()
        if parts:
            symbols.add(parts[-1])
    return symbols


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Compare a committed symbol manifest against a shared library.",
    )
    parser.add_argument("reference", type=Path)
    parser.add_argument("candidate", type=Path)
    args = parser.parse_args()

    expected = read_reference(args.reference)
    actual = read_library_symbols(args.candidate)

    missing = sorted(expected - actual)
    unexpected = sorted(actual - expected)

    if missing or unexpected:
        if missing:
            print("missing symbols:", file=sys.stderr)
            for symbol in missing:
                print(f"  {symbol}", file=sys.stderr)
        if unexpected:
            print("unexpected symbols:", file=sys.stderr)
            for symbol in unexpected:
                print(f"  {symbol}", file=sys.stderr)
        return 1

    print(f"matched {len(expected)} symbols")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

