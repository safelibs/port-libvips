#!/usr/bin/env python3

from __future__ import annotations

import argparse
import re
import shlex
import sys
from pathlib import Path

FIELDS = [
    "Name",
    "Description",
    "Version",
    "Requires",
    "Requires.private",
    "Libs",
    "Libs.private",
    "Cflags",
]

VAR_RE = re.compile(r"\$\{([^}]+)\}")


def parse_pc(path: Path) -> tuple[dict[str, str], dict[str, str]]:
    variables: dict[str, str] = {}
    fields: dict[str, str] = {}

    for raw_line in path.read_text().splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        if re.match(r"^[A-Za-z0-9_.+-]+=", line):
            key, value = line.split("=", 1)
            variables[key] = value
        elif ":" in line:
            key, value = line.split(":", 1)
            fields[key.strip()] = value.strip()
        else:
            raise SystemExit(f"unrecognised pkg-config line in {path}: {raw_line}")

    return variables, fields


def expand(value: str, variables: dict[str, str], stack: tuple[str, ...] = ()) -> str:
    def replace(match: re.Match[str]) -> str:
        name = match.group(1)
        if name in stack:
            raise SystemExit(f"recursive variable expansion for {' -> '.join(stack + (name,))}")
        return expand(variables.get(name, ""), variables, stack + (name,))

    while True:
        expanded = VAR_RE.sub(replace, value)
        if expanded == value:
            return expanded
        value = expanded


def normalize_requires(value: str) -> str:
    items = [item.strip() for item in value.split(",") if item.strip()]
    return ", ".join(items)


def normalize_token(token: str, variables: dict[str, str]) -> str:
    prefix = expand(variables.get("prefix", ""), variables)
    includedir = expand(variables.get("includedir", ""), variables)
    libdir = expand(variables.get("libdir", ""), variables)

    replacements = [
        (includedir, "${includedir}"),
        (libdir, "${libdir}"),
        (prefix, "${prefix}"),
    ]

    for source, placeholder in replacements:
        if not source:
            continue
        if token == source:
            return placeholder
        if token.startswith("-I" + source):
            return "-I" + placeholder + token[len("-I" + source) :]
        if token.startswith("-L" + source):
            return "-L" + placeholder + token[len("-L" + source) :]
        if source in token:
            token = token.replace(source, placeholder)
    return token


def normalize_flags(value: str, variables: dict[str, str]) -> str:
    tokens = [normalize_token(token, variables) for token in shlex.split(value)]
    return " ".join(tokens)


def normalized_fields(path: Path) -> dict[str, str]:
    variables, fields = parse_pc(path)
    expanded_vars = {key: expand(value, variables) for key, value in variables.items()}

    normalized: dict[str, str] = {}
    for field in FIELDS:
        value = fields.get(field, "")
        expanded = expand(value, {**variables, **expanded_vars})
        if field.startswith("Requires"):
            normalized[field] = normalize_requires(expanded)
        elif field in {"Libs", "Libs.private", "Cflags"}:
            normalized[field] = normalize_flags(expanded, {**variables, **expanded_vars})
        else:
            normalized[field] = " ".join(expanded.split())
    return normalized


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Compare semantic pkg-config contents while ignoring relocatable prefixes.",
    )
    parser.add_argument("reference", type=Path)
    parser.add_argument("candidate", type=Path)
    args = parser.parse_args()

    expected = normalized_fields(args.reference)
    actual = normalized_fields(args.candidate)

    mismatches = []
    for field in FIELDS:
        if expected.get(field, "") != actual.get(field, ""):
            mismatches.append(field)

    if mismatches:
        for field in mismatches:
            print(f"{field} mismatch:", file=sys.stderr)
            print(f"  expected: {expected.get(field, '')}", file=sys.stderr)
            print(f"  actual:   {actual.get(field, '')}", file=sys.stderr)
        return 1

    print(f"matched pkg-config metadata for {args.candidate.name}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
