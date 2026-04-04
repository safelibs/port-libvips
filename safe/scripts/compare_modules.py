#!/usr/bin/env python3

from __future__ import annotations

import argparse
import sys
from pathlib import Path


def read_manifest_lines(path: Path) -> list[str]:
    return [
        line.strip()
        for line in path.read_text().splitlines()
        if line.strip() and not line.startswith("#")
    ]


def find_module_dirs(root: Path, basename: str) -> list[Path]:
    candidates = []
    if root.name == basename and root.is_dir():
        candidates.append(root.resolve())
    candidates.extend(path.resolve() for path in root.rglob(basename) if path.is_dir())
    return sorted(set(candidates))


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Validate that the expected module directory and shared-module basenames are present.",
    )
    parser.add_argument("reference_modules", type=Path)
    parser.add_argument("candidate_root", type=Path)
    args = parser.parse_args()

    expected_dir = read_manifest_lines(args.reference_modules / "module-dir.txt")
    expected_modules = read_manifest_lines(args.reference_modules / "installed-modules.txt")
    if len(expected_dir) != 1:
        raise SystemExit(f"module-dir.txt should contain exactly one basename, got: {expected_dir}")
    module_dir_basename = expected_dir[0]

    module_dirs = find_module_dirs(args.candidate_root.resolve(), module_dir_basename)
    if not module_dirs:
        print(
            f"missing module directory {module_dir_basename!r} under {args.candidate_root}",
            file=sys.stderr,
        )
        return 1

    missing = []
    for module in expected_modules:
        found = any(
            list(module_dir.glob(f"{module}.so")) or list(module_dir.glob(f"{module}.so.*"))
            for module_dir in module_dirs
        )
        if not found:
            missing.append(module)

    if missing:
        print("missing module payloads:", file=sys.stderr)
        for module in missing:
            print(f"  {module}", file=sys.stderr)
        print("searched:", file=sys.stderr)
        for module_dir in module_dirs:
            print(f"  {module_dir}", file=sys.stderr)
        return 1

    print(f"matched module directory {module_dir_basename} in {args.candidate_root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
