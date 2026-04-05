#!/usr/bin/env python3

from __future__ import annotations

import argparse
import ctypes
import ctypes.util
import json
import os
import stat
import sys
from pathlib import Path


def is_executable(path: Path) -> bool:
    try:
        mode = path.stat().st_mode
    except FileNotFoundError:
        return False
    return path.is_file() and bool(mode & stat.S_IXUSR)


def locate_vips_binary(root: Path) -> Path:
    preferred = [
        root / "bin" / "vips",
        root / "usr" / "bin" / "vips",
        root / "tools" / "vips",
    ]
    for candidate in preferred:
        if is_executable(candidate):
            return candidate.resolve()

    matches = sorted(path.resolve() for path in root.rglob("vips") if is_executable(path))
    if not matches:
        raise SystemExit(f"unable to locate a usable vips binary under {root}")
    return matches[0]


def locate_library(root: Path) -> Path:
    patterns = ["libvips.so.42.17.1", "libvips.so.42", "libvips.so"]
    for pattern in patterns:
        matches = sorted(path.resolve() for path in root.rglob(pattern) if path.is_file())
        if matches:
            return matches[0]
    raise SystemExit(f"unable to locate libvips under {root}")


def infer_prefix_root(root: Path, module_dir_basename: str) -> Path:
    candidates = [root, root / "usr"]
    for candidate in candidates:
        if not candidate.exists():
            continue
        if any(path.is_dir() for path in candidate.rglob(module_dir_basename)):
            return candidate.resolve()
    return root.resolve()


def read_module_dir_basename(reference_manifest: Path) -> str:
    module_dir_manifest = reference_manifest.with_name("module-dir.txt")
    if not module_dir_manifest.is_file():
        return "vips-modules-8.15"

    lines = [
        line.strip()
        for line in module_dir_manifest.read_text().splitlines()
        if line.strip() and not line.startswith("#")
    ]
    if len(lines) != 1:
        raise SystemExit(
            f"{module_dir_manifest} should contain exactly one module directory basename"
        )
    return lines[0]


def load_manifest(path: Path) -> dict[str, dict[str, object]]:
    return json.loads(path.read_text())


def load_cdll(path: str, *, global_symbols: bool = False) -> ctypes.CDLL:
    mode = getattr(ctypes, "RTLD_GLOBAL", 0) if global_symbols else getattr(ctypes, "RTLD_LOCAL", 0)
    return ctypes.CDLL(path, mode=mode)


def main() -> int:
    parser = argparse.ArgumentParser(
        description=(
            "Trigger module-backed operation loading and compare the live VipsOperation/"
            "VipsObject registries against the committed module registry manifest."
        ),
    )
    parser.add_argument("reference_manifest", type=Path)
    parser.add_argument("candidate_root", type=Path)
    args = parser.parse_args()

    manifest = load_manifest(args.reference_manifest)
    candidate_root = args.candidate_root.resolve()
    vips_binary = locate_vips_binary(candidate_root)
    module_dir_basename = read_module_dir_basename(args.reference_manifest.resolve())
    prefix_root = infer_prefix_root(candidate_root, module_dir_basename)
    library_path = locate_library(candidate_root)

    os.environ["VIPSHOME"] = str(prefix_root)
    os.environ["LD_LIBRARY_PATH"] = (
        f"{library_path.parent}{os.pathsep}{os.environ['LD_LIBRARY_PATH']}"
        if os.environ.get("LD_LIBRARY_PATH")
        else str(library_path.parent)
    )

    gobject_name = ctypes.util.find_library("gobject-2.0")
    if not gobject_name:
        raise SystemExit("unable to locate gobject-2.0")
    gobject = load_cdll(gobject_name)
    libvips = load_cdll(str(library_path), global_symbols=True)

    guint64 = ctypes.c_size_t
    gpointer = ctypes.c_void_p

    libvips.vips_init.argtypes = [ctypes.c_char_p]
    libvips.vips_init.restype = ctypes.c_int
    libvips.vips_shutdown.argtypes = []
    libvips.vips_shutdown.restype = None
    libvips.vips_operation_new.argtypes = [ctypes.c_char_p]
    libvips.vips_operation_new.restype = gpointer
    libvips.vips_type_map_all.argtypes = [guint64, ctypes.c_void_p, gpointer]
    libvips.vips_type_map_all.restype = gpointer
    libvips.vips_nickname_find.argtypes = [guint64]
    libvips.vips_nickname_find.restype = ctypes.c_char_p
    libvips.vips_error_buffer.argtypes = []
    libvips.vips_error_buffer.restype = ctypes.c_char_p
    libvips.vips_error_clear.argtypes = []
    libvips.vips_error_clear.restype = None

    gobject.g_type_from_name.argtypes = [ctypes.c_char_p]
    gobject.g_type_from_name.restype = guint64
    gobject.g_type_name.argtypes = [guint64]
    gobject.g_type_name.restype = ctypes.c_char_p
    gobject.g_object_unref.argtypes = [gpointer]
    gobject.g_object_unref.restype = None

    if libvips.vips_init(str(vips_binary).encode()) != 0:
        message = libvips.vips_error_buffer()
        raise SystemExit(
            f"vips_init failed for {library_path}: {message.decode() if message else 'unknown error'}"
        )

    try:
        probe_failures = []
        for module_name in sorted(manifest):
            probe = str(manifest[module_name]["probe_operation"])
            instance = libvips.vips_operation_new(probe.encode())
            if instance:
                gobject.g_object_unref(instance)
                continue

            message = libvips.vips_error_buffer()
            probe_failures.append(
                f"{module_name}: unable to instantiate probe operation {probe!r}: "
                f"{message.decode() if message else 'unknown error'}"
            )
            libvips.vips_error_clear()

        if probe_failures:
            for failure in probe_failures:
                print(failure, file=sys.stderr)
            return 1

        operation_base = gobject.g_type_from_name(b"VipsOperation")
        object_base = gobject.g_type_from_name(b"VipsObject")
        if not operation_base or not object_base:
            raise SystemExit("failed to resolve VipsOperation or VipsObject base types")

        operations_seen: set[str] = set()
        object_types_seen: set[str] = set()

        @ctypes.CFUNCTYPE(gpointer, guint64, gpointer)
        def collect_operations(gtype: int, _userdata: int) -> int:
            nickname = libvips.vips_nickname_find(gtype)
            if nickname:
                operations_seen.add(nickname.decode())
            return 0

        @ctypes.CFUNCTYPE(gpointer, guint64, gpointer)
        def collect_objects(gtype: int, _userdata: int) -> int:
            type_name = gobject.g_type_name(gtype)
            if type_name:
                object_types_seen.add(type_name.decode())
            return 0

        libvips.vips_type_map_all(operation_base, collect_operations, None)
        libvips.vips_type_map_all(object_base, collect_objects, None)

        missing_operations: list[str] = []
        missing_types: list[str] = []

        for module_name in sorted(manifest):
            entry = manifest[module_name]
            for operation in entry.get("operations", []):
                if operation not in operations_seen:
                    missing_operations.append(f"{module_name}: {operation}")
            for type_name in entry.get("types", []):
                if type_name not in object_types_seen:
                    missing_types.append(f"{module_name}: {type_name}")

        if missing_operations or missing_types:
            if missing_operations:
                print("missing module-backed operations:", file=sys.stderr)
                for item in missing_operations:
                    print(f"  {item}", file=sys.stderr)
            if missing_types:
                print("missing module-backed GTypes:", file=sys.stderr)
                for item in missing_types:
                    print(f"  {item}", file=sys.stderr)
            return 1

        print(
            f"matched module-backed registry contract via {vips_binary} and {library_path}"
        )
        return 0
    finally:
        libvips.vips_shutdown()


if __name__ == "__main__":
    raise SystemExit(main())
