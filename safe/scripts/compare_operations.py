#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def load(path: Path) -> dict[str, object]:
    return json.loads(path.read_text())


def compare_lists(reference: dict[str, object], candidate: dict[str, object], key: str) -> list[str]:
    ref_value = reference.get(key)
    cand_value = candidate.get(key)
    if ref_value != cand_value:
        return [f"mismatch for {key}"]
    return []


def main() -> int:
    parser = argparse.ArgumentParser(
        description=(
            "Compare the committed phase-1 operation tree against the generated "
            "phase-4 operation manifest."
        )
    )
    parser.add_argument("reference", type=Path)
    parser.add_argument("candidate", type=Path)
    args = parser.parse_args()

    reference = load(args.reference)
    candidate = load(args.candidate)

    failures: list[str] = []
    for key in ["count", "entries", "nicknames", "type_names"]:
        failures.extend(compare_lists(reference, candidate, key))

    if failures:
        for failure in failures:
            print(failure, file=sys.stderr)
        return 1

    print(
        f"matched operation tree contract for {args.reference.resolve()} "
        f"and {args.candidate.resolve()}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
