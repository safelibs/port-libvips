#!/usr/bin/env python3

from __future__ import annotations

import argparse
import hashlib
import sys
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class BinaryIdentity:
    path: Path
    size: int
    sha256: str


def describe(path: Path) -> BinaryIdentity:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return BinaryIdentity(
        path=path,
        size=path.stat().st_size,
        sha256=digest.hexdigest(),
    )


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Fail if a candidate binary matches the committed reference binary.",
    )
    parser.add_argument("reference", type=Path)
    parser.add_argument("candidate", type=Path)
    args = parser.parse_args()

    reference = describe(args.reference)
    candidate = describe(args.candidate)

    if reference.size == candidate.size and reference.sha256 == candidate.sha256:
        print(
            (
                "error: candidate binary matches the reference binary\n"
                f"  reference: {reference.path}\n"
                f"  candidate: {candidate.path}\n"
                f"  size: {candidate.size} bytes\n"
                f"  sha256: {candidate.sha256}"
            ),
            file=sys.stderr,
        )
        return 1

    print(
        (
            "verified candidate differs from reference\n"
            f"  reference: {reference.path}\n"
            f"  candidate: {candidate.path}"
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
