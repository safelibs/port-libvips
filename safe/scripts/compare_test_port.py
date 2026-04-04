#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


TEXT_MANIFESTS = {
    "meson_tests": "meson-tests.txt",
    "standalone_shell_tests": "standalone-shell-tests.txt",
    "python_files": "python-files.txt",
    "fuzz_targets": "fuzz-targets.txt",
}

REFERENCE_ONLY_MANIFESTS = {
    "python_requirements": "python-requirements.txt",
    "tools": "tools.txt",
    "examples": "examples.txt",
}

EXPECTED_WRAPPERS = {
    "meson": "run-meson-suite.sh",
    "shell": "run-shell-suite.sh",
    "pytest": "run-pytest-suite.sh",
    "fuzz": "run-fuzz-suite.sh",
}


def read_manifest_lines(path: Path) -> list[str]:
    return [
        line.strip()
        for line in path.read_text().splitlines()
        if line.strip() and not line.startswith("#")
    ]


def fail(errors: list[str]) -> int:
    for error in errors:
        print(error, file=sys.stderr)
    return 1


def main() -> int:
    parser = argparse.ArgumentParser(
        description=(
            "Compare the safe-local upstream wrapper manifest against the committed "
            "reference coverage manifests."
        )
    )
    parser.add_argument("reference_dir", type=Path)
    parser.add_argument("candidate_dir", type=Path)
    args = parser.parse_args()

    manifest_path = args.candidate_dir / "manifest.json"
    manifest = json.loads(manifest_path.read_text())
    errors: list[str] = []

    if manifest.get("schema_version") != 1:
        errors.append(
            f"{manifest_path}: expected schema_version=1, found {manifest.get('schema_version')!r}"
        )

    if manifest.get("upstream_root") != "original":
        errors.append(
            f"{manifest_path}: expected upstream_root='original', found "
            f"{manifest.get('upstream_root')!r}"
        )

    wrappers = manifest.get("wrappers")
    if not isinstance(wrappers, dict):
        errors.append(f"{manifest_path}: wrappers must be a JSON object")
    else:
        for key, expected_path in EXPECTED_WRAPPERS.items():
            actual_path = wrappers.get(key)
            if actual_path != expected_path:
                errors.append(
                    f"{manifest_path}: wrappers.{key} expected {expected_path!r}, "
                    f"found {actual_path!r}"
                )
            elif not (args.candidate_dir / actual_path).is_file():
                errors.append(
                    f"{manifest_path}: wrappers.{key} points to missing file {actual_path!r}"
                )

    for manifest_key, filename in TEXT_MANIFESTS.items():
        reference_entries = read_manifest_lines(args.reference_dir / filename)
        candidate_entries = read_manifest_lines(args.candidate_dir / filename)
        if candidate_entries != reference_entries:
            errors.append(
                f"{args.candidate_dir / filename}: expected {reference_entries!r}, "
                f"found {candidate_entries!r}"
            )
        manifest_entries = manifest.get(manifest_key)
        if manifest_entries != reference_entries:
            errors.append(
                f"{manifest_path}: {manifest_key} expected {reference_entries!r}, "
                f"found {manifest_entries!r}"
            )

    for manifest_key, filename in REFERENCE_ONLY_MANIFESTS.items():
        reference_entries = read_manifest_lines(args.reference_dir / filename)
        manifest_entries = manifest.get(manifest_key)
        if manifest_entries != reference_entries:
            errors.append(
                f"{manifest_path}: {manifest_key} expected {reference_entries!r}, "
                f"found {manifest_entries!r}"
            )

    standalone_shell_tests = manifest.get("standalone_shell_tests", [])
    if "original/test/test_thumbnail.sh" not in standalone_shell_tests:
        errors.append(
            f"{manifest_path}: standalone_shell_tests must explicitly include "
            f"'original/test/test_thumbnail.sh'"
        )

    if errors:
        return fail(errors)

    print(f"matched safe-local upstream test manifest in {args.candidate_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
