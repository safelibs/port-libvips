#!/usr/bin/env python3

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


API_DECL_RE = re.compile(r"(?ms)\bVIPS_API\b\s*(.*?;)")
BLOCK_COMMENT_RE = re.compile(r"(?s)/\*.*?\*/")


def normalize_space(value: str) -> str:
    return " ".join(value.split())


def strip_comments(text: str) -> str:
    text = BLOCK_COMMENT_RE.sub("", text)
    return re.sub(r"//.*", "", text)


def find_header_dir(root: Path) -> Path:
    root = root.resolve()
    candidates = []

    if (root / "include" / "vips" / "vips.h").is_file():
        candidates.append(root / "include" / "vips")
    if (root / "vips.h").is_file() and root.name == "vips":
        candidates.append(root)

    for candidate in root.rglob("include/vips/vips.h"):
        candidates.append(candidate.parent)

    deduped = sorted({candidate.resolve() for candidate in candidates})
    if not deduped:
        raise SystemExit(f"unable to locate installed vips headers under {root}")
    return deduped[0]


def header_files(header_dir: Path) -> list[str]:
    return sorted(
        path.name
        for path in header_dir.iterdir()
        if path.is_file() or path.is_symlink()
    )


def api_declarations(header_dir: Path) -> list[str]:
    decls: list[str] = []
    for path in sorted(header_dir.iterdir(), key=lambda item: item.name):
        if not (path.is_file() or path.is_symlink()):
            continue
        text = strip_comments(path.read_text())
        for match in API_DECL_RE.finditer(text):
            decls.append(f"{path.name}: {normalize_space(match.group(1))}")
    return sorted(decls)


def read_reference_lines(path: Path) -> list[str]:
    return [
        line.strip()
        for line in path.read_text().splitlines()
        if line.strip() and not line.startswith("#")
    ]


def compare(name: str, expected: list[str], actual: list[str]) -> None:
    expected_set = set(expected)
    actual_set = set(actual)

    missing = sorted(expected_set - actual_set)
    unexpected = sorted(actual_set - expected_set)
    if missing or unexpected:
        if missing:
            print(f"missing {name}:", file=sys.stderr)
            for item in missing:
                print(f"  {item}", file=sys.stderr)
        if unexpected:
            print(f"unexpected {name}:", file=sys.stderr)
            for item in unexpected:
                print(f"  {item}", file=sys.stderr)
        raise SystemExit(1)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Compare installed vips headers against committed file and declaration manifests.",
    )
    parser.add_argument("--files", required=True, type=Path)
    parser.add_argument("--decls", required=True, type=Path)
    parser.add_argument("candidate_root", type=Path)
    args = parser.parse_args()

    header_dir = find_header_dir(args.candidate_root)
    compare("header files", read_reference_lines(args.files), header_files(header_dir))
    compare("API declarations", read_reference_lines(args.decls), api_declarations(header_dir))

    print(f"matched headers in {header_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

