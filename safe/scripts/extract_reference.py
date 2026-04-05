#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
from pathlib import Path

from extract_introspection import (
    ASSIGNMENT_RE,
    TypeDefinition,
    parse_define_macros,
    strip_translation,
    type_name_from_parent_expr,
)


BLOCK_COMMENT_RE = re.compile(r"(?s)/\*.*?\*/")
LINE_COMMENT_RE = re.compile(r"//.*")
API_DECL_RE = re.compile(r"(?ms)\bVIPS_API\b\s*(.*?;)")
TYPE_DEFINE_RE = re.compile(
    r"(?ms)G_DEFINE(?:_(?:ABSTRACT|FINAL))?_TYPE(?:_WITH_CODE)?\s*\(\s*([A-Za-z0-9_]+)\s*,\s*([a-z0-9_]+)\s*,"
)
NICKNAME_RE = re.compile(r'nickname\s*=\s*"([^"]+)"')
GET_TYPE_CALL_RE = re.compile(r"\b([a-z0-9_]+_get_type)\s*\(")

BOOTSTRAP_SYMBOLS = {
    "_vips__argument_id",
    "vips_argument_get_id",
    "vips_get_argv0",
    "vips_get_prgname",
    "vips_init",
    "vips_shutdown",
    "vips_thread_shutdown",
    "vips_version",
    "vips_version_string",
}

MODULE_SOURCE_LISTS = {
    "vips-heif": "heif_module_sources",
    "vips-jxl": "jpeg_xl_module_sources",
    "vips-magick": "magick_module_sources",
    "vips-openslide": "openslide_module_sources",
    "vips-poppler": "poppler_module_sources",
}
MODULE_TYPE_RE = re.compile(
    r'module_type!\(\s*'
    r'[^,]+,\s*'
    r'(?P<parent_expr>[^,]+),\s*'
    r'[^,]+,\s*'
    r'[^,]+,\s*'
    r'"(?P<type_name>[^"]+)"\s*,\s*'
    r'"(?P<nickname>[^"]+)"\s*,\s*'
    r'"(?P<description>[^"]+)"\s*'
    r'\);',
    re.S,
)


def normalize_space(value: str) -> str:
    return " ".join(value.split())


def strip_comments(text: str) -> str:
    text = BLOCK_COMMENT_RE.sub("", text)
    return LINE_COMMENT_RE.sub("", text)


def parse_meson_list(text: str, name: str) -> list[str]:
    match = re.search(rf"(?ms){re.escape(name)}\s*=\s*\[(.*?)\]", text)
    if not match:
        match = re.search(rf"(?ms){re.escape(name)}\s*=\s*files\((.*?)\)", text)
    if not match:
        raise ValueError(f"unable to find Meson list {name!r}")
    return re.findall(r"'([^']+)'", match.group(1))


def run_command(command: list[str], *, env: dict[str, str] | None = None) -> str:
    return subprocess.check_output(command, text=True, env=env)


def read_symbols(library: Path) -> list[str]:
    output = run_command(["nm", "-D", "--defined-only", str(library)])
    return sorted({line.split()[-1] for line in output.splitlines() if line.split()})


def write_lines(path: Path, lines: list[str]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines) + "\n")


def write_json(path: Path, payload: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")


def sync_headers(reference_install: Path, safe_root: Path) -> None:
    source_dir = reference_install / "include" / "vips"
    target_dir = safe_root / "include" / "vips"
    target_dir.mkdir(parents=True, exist_ok=True)

    source_names = {path.name for path in source_dir.iterdir() if path.is_file() or path.is_symlink()}
    for path in list(target_dir.iterdir()):
        if path.name not in source_names:
            if path.is_dir():
                shutil.rmtree(path)
            else:
                path.unlink()

    for path in sorted(source_dir.iterdir(), key=lambda item: item.name):
        if not (path.is_file() or path.is_symlink()):
            continue
        shutil.copy2(path, target_dir / path.name)


def header_files(header_dir: Path) -> list[str]:
    return sorted(
        path.name
        for path in header_dir.iterdir()
        if path.is_file() or path.is_symlink()
    )


def public_api_decls(header_dir: Path) -> list[str]:
    decls: list[str] = []
    for path in sorted(header_dir.iterdir(), key=lambda item: item.name):
        if not (path.is_file() or path.is_symlink()):
            continue
        text = strip_comments(path.read_text())
        for match in API_DECL_RE.finditer(text):
            decls.append(f"{path.name}: {normalize_space(match.group(1))}")
    return sorted(decls)


def extract_declared_symbols(text: str) -> set[str]:
    text = strip_comments(text)
    lines = [line for line in text.splitlines() if not line.strip().startswith("#")]
    declarations = [chunk.strip() for chunk in "\n".join(lines).split(";") if chunk.strip()]

    symbols: set[str] = set()
    for declaration in declarations:
        if declaration.startswith("typedef"):
            continue
        candidate = declaration
        if "(" in candidate:
            before_paren = candidate.split("(", 1)[0].strip()
            match = re.search(r"([A-Za-z_][A-Za-z0-9_]*)$", before_paren)
        else:
            match = re.search(r"([A-Za-z_][A-Za-z0-9_]*)\s*(?:\[[^]]*\])?$", candidate)
        if match:
            symbols.add(match.group(1))
    return symbols


def parse_vips_tree(output: str, source: str) -> dict[str, object]:
    entries: list[dict[str, object]] = []
    stack: list[dict[str, object]] = []
    base_depth: int | None = None

    for raw_line in output.splitlines():
        if not raw_line.strip():
            continue
        indent = len(raw_line) - len(raw_line.lstrip(" "))
        depth = indent // 2
        if base_depth is None:
            base_depth = depth
        logical_depth = depth - base_depth
        line = raw_line.strip()
        match = re.match(r"^(.*?) \((.*?)\), (.*)$", line)
        if not match:
            raise ValueError(f"unable to parse vips tree line: {raw_line!r}")

        type_name, nickname, description = match.groups()
        while len(stack) > logical_depth:
            stack.pop()

        parent = stack[-1]["type_name"] if stack else None
        entry = {
            "depth": depth,
            "type_name": type_name,
            "nickname": nickname,
            "description": description,
            "parent": parent,
        }
        entries.append(entry)
        stack.append(entry)

    return {
        "source": source,
        "count": len(entries),
        "nicknames": sorted({entry["nickname"] for entry in entries}),
        "type_names": sorted({entry["type_name"] for entry in entries}),
        "entries": entries,
    }


def load_type_definitions(original_root: Path) -> dict[str, TypeDefinition]:
    definitions: dict[str, TypeDefinition] = {}
    for source_path in sorted(original_root.rglob("*.c")):
        source_text = source_path.read_text(errors="replace")
        for definition in parse_define_macros(source_text, str(source_path.relative_to(original_root.parent))):
            definitions.setdefault(definition.type_name, definition)
    return definitions


def definition_fields(definition: TypeDefinition) -> dict[str, str]:
    fields: dict[str, str] = {}
    for match in ASSIGNMENT_RE.finditer(definition.body):
        fields[match.group(1)] = strip_translation(match.group(2))
    return fields


def load_module_type_metadata(safe_root: Path) -> dict[str, dict[str, str]]:
    text = (safe_root / "src" / "foreign" / "modules.rs").read_text()
    metadata: dict[str, dict[str, str | None]] = {}
    for match in MODULE_TYPE_RE.finditer(text):
        parent_expr = match.group("parent_expr").strip().split("::")[-1]
        metadata[match.group("type_name")] = {
            "nickname": match.group("nickname"),
            "description": match.group("description"),
            "parent_type_name": type_name_from_parent_expr(f"{parent_expr}()"),
        }
    return metadata


def repair_tree_manifest(
    manifest: dict[str, object],
    *,
    definitions: dict[str, TypeDefinition],
    extra_type_names: set[str],
    module_type_metadata: dict[str, dict[str, str | None]],
) -> dict[str, object]:
    base_entries = {
        str(entry["type_name"]): dict(entry)
        for entry in manifest["entries"]  # type: ignore[index]
    }

    wanted_type_names = set(base_entries)
    pending = set(extra_type_names)
    while pending:
        type_name = pending.pop()
        if type_name in wanted_type_names:
            continue
        wanted_type_names.add(type_name)
        definition = definitions.get(type_name)
        if definition and definition.parent_type_name and definition.parent_type_name not in wanted_type_names:
            pending.add(definition.parent_type_name)

    field_cache: dict[str, dict[str, str]] = {}
    built: dict[str, dict[str, object]] = {}

    def build_entry(type_name: str) -> dict[str, object]:
        cached = built.get(type_name)
        if cached is not None:
            return cached

        base_entry = dict(base_entries.get(type_name, {}))
        definition = definitions.get(type_name)
        fields = field_cache.setdefault(type_name, definition_fields(definition)) if definition else {}
        module_fields = module_type_metadata.get(type_name, {})

        parent = base_entry.get("parent")
        if definition and definition.parent_type_name:
            parent = definition.parent_type_name
        elif module_fields.get("parent_type_name"):
            parent = module_fields["parent_type_name"]

        if isinstance(parent, str) and parent in wanted_type_names:
            parent_entry = build_entry(parent)
            depth = int(parent_entry["depth"]) + 1
        else:
            parent = None if parent is None else parent
            depth = int(base_entry.get("depth", 0))

        nickname = base_entry.get("nickname") or fields.get("nickname") or module_fields.get("nickname")
        description = (
            base_entry.get("description")
            or fields.get("description")
            or module_fields.get("description")
        )
        if nickname is None or description is None:
            raise ValueError(f"missing nickname/description for {type_name}")

        entry = {
            "depth": depth,
            "type_name": type_name,
            "nickname": nickname,
            "description": description,
            "parent": parent,
        }
        built[type_name] = entry
        return entry

    ordered_type_names = [
        str(entry["type_name"]) for entry in manifest["entries"]  # type: ignore[index]
    ]
    for type_name in sorted(wanted_type_names):
        if type_name not in ordered_type_names:
            ordered_type_names.append(type_name)

    entries = [build_entry(type_name) for type_name in ordered_type_names if type_name in wanted_type_names]
    return {
        "source": manifest["source"],
        "count": len(entries),
        "nicknames": sorted({str(entry["nickname"]) for entry in entries}),
        "type_names": sorted({str(entry["type_name"]) for entry in entries}),
        "entries": entries,
    }


def select_probe_operation(operations: set[str]) -> str:
    preferred = sorted(
        op
        for op in operations
        if not any(tag in op for tag in ("_base", "_buffer", "_source", "_target", "_mime"))
    )
    load_preferred = [op for op in preferred if "load" in op]
    if load_preferred:
        return sorted(load_preferred)[0]
    if preferred:
        return preferred[0]
    return sorted(operations)[0]


def add_canonical_operation_aliases(operations: set[str]) -> set[str]:
    expanded = set(operations)
    for operation in list(operations):
        for suffix in ("_buffer", "_source", "_target"):
            if operation.endswith(suffix):
                expanded.add(operation[: -len(suffix)])
    return expanded


def build_module_registry(original_root: Path) -> dict[str, dict[str, object]]:
    foreign_meson = (original_root / "libvips" / "foreign" / "meson.build").read_text()
    registry: dict[str, dict[str, object]] = {}

    for module_name, source_list_name in MODULE_SOURCE_LISTS.items():
        short_name = module_name.removeprefix("vips-")
        module_path = original_root / "libvips" / "module" / f"{short_name}.c"
        module_text = strip_comments(module_path.read_text())
        entrypoint_functions = sorted(set(GET_TYPE_CALL_RE.findall(module_text)))

        source_files = [
            original_root / "libvips" / "foreign" / source_name
            for source_name in parse_meson_list(foreign_meson, source_list_name)
        ]

        function_to_type: dict[str, str] = {}
        operations: set[str] = set()
        types: set[str] = set()
        for source_file in source_files:
            source_text = source_file.read_text()
            for type_name, function_name in TYPE_DEFINE_RE.findall(source_text):
                function_to_type[function_name] = type_name
                types.add(type_name)
            operations.update(NICKNAME_RE.findall(source_text))

        operations = add_canonical_operation_aliases(operations)

        direct_types = {
            function_to_type[function_name.removesuffix("_get_type")]
            for function_name in entrypoint_functions
            if function_name.removesuffix("_get_type") in function_to_type
        }
        if direct_types:
            types.update(direct_types)

        registry[module_name] = {
            "probe_operation": select_probe_operation(operations),
            "operations": sorted(operations),
            "types": sorted(types),
            "source_files": [str(path.relative_to(original_root)) for path in source_files],
            "entrypoints": entrypoint_functions,
        }

    return registry


def extract_tests(
    original_root: Path,
    meson_test_log: Path,
    pytest_log: Path,
) -> dict[str, list[str]]:
    test_meson = (original_root / "test" / "meson.build").read_text()
    fuzz_meson = (original_root / "fuzz" / "meson.build").read_text()
    tools_meson = (original_root / "tools" / "meson.build").read_text()
    examples_meson = (original_root / "examples" / "meson.build").read_text()
    conftest = (original_root / "test" / "test-suite" / "conftest.py").read_text()

    meson_tests = parse_meson_list(test_meson, "script_tests") + [
        "connections",
        "descriptors",
        "webpsave_timeout",
    ]
    meson_tests.append("fuzz")

    fuzz_targets = parse_meson_list(fuzz_meson, "fuzz_progs")
    tools = parse_meson_list(tools_meson, "tools")
    if (original_root / "tools" / "vipsprofile").is_file():
        tools.append("vipsprofile")
    python_files = sorted(
        str(path.relative_to(original_root))
        for path in (original_root / "test" / "test-suite").rglob("*.py")
        if "__pycache__" not in path.parts
    )

    examples = parse_meson_list(examples_meson, "examples")

    meson_summary = re.search(r"Ok:\s+(\d+).*?Fail:\s+(\d+)", meson_test_log.read_text(), re.S)
    pytest_summary = re.search(
        r"(?P<passed>\d+) passed, (?P<skipped>\d+) skipped",
        pytest_log.read_text(),
    )
    py_requirement = re.search(r'PYVIPS_REQUIREMENT = "([^"]+)"', conftest)

    return {
        "meson_tests": meson_tests,
        "meson_comments": [
            f"# baseline_ok={meson_summary.group(1) if meson_summary else 'unknown'}",
            f"# baseline_fail={meson_summary.group(2) if meson_summary else 'unknown'}",
            f"# total={len(meson_tests)}",
        ],
        "standalone_shell_tests": [
            "# total=1",
            "original/test/test_thumbnail.sh",
        ],
        "python_files": [
            f"# baseline_passed={pytest_summary.group('passed') if pytest_summary else 'unknown'}",
            f"# baseline_skipped={pytest_summary.group('skipped') if pytest_summary else 'unknown'}",
            f"# total={len(python_files)}",
            *python_files,
        ],
        "python_requirements": [py_requirement.group(1) if py_requirement else "pyvips==3.1.1"],
        "fuzz_targets": [f"# total={len(fuzz_targets)}", *fuzz_targets],
        "tools": [f"# total={len(tools)}", *tools],
        "examples": [f"# total={len(examples)}", *examples],
    }


def module_contract(original_root: Path) -> tuple[str, list[str]]:
    root_meson = (original_root / "meson.build").read_text()
    libvips_meson = (original_root / "libvips" / "meson.build").read_text()

    version_match = re.search(r"version:\s*'(\d+)\.(\d+)\.(\d+)'", root_meson)
    if not version_match:
        raise ValueError("unable to determine project version from original/meson.build")
    module_dir = f"vips-modules-{version_match.group(1)}.{version_match.group(2)}"

    modules = re.findall(r"shared_module\('([^']+)'", libvips_meson)
    return module_dir, sorted(modules)


def bootstrap_symbols(public_decl_lines: list[str], exported: set[str]) -> list[str]:
    decl_symbols = {
        match.group(1)
        for line in public_decl_lines
        for match in [re.search(r"([A-Za-z_][A-Za-z0-9_]*)\s*\(", line)]
        if match
    }
    selected = {
        symbol
        for symbol in exported
        if symbol in BOOTSTRAP_SYMBOLS or symbol.startswith("vips_error") or (symbol.endswith("_get_type") and symbol in decl_symbols)
    }
    return sorted(selected)


def deprecated_symbols(header_dir: Path, exported: set[str]) -> list[str]:
    compat_symbols = extract_declared_symbols((header_dir / "vips7compat.h").read_text())
    selected = {symbol for symbol in compat_symbols if symbol in exported}
    selected.update(symbol for symbol in exported if symbol.startswith("im_"))
    return sorted(selected)


def copy_pkgconfig(reference_install: Path, safe_root: Path) -> None:
    src_dir = reference_install / "lib" / "pkgconfig"
    dst_dir = safe_root / "reference" / "pkgconfig"
    dst_dir.mkdir(parents=True, exist_ok=True)
    for name in ("vips.pc", "vips-cpp.pc"):
        shutil.copy2(src_dir / name, dst_dir / name)


def relative_link_inputs(target: dict[str, object]) -> list[str]:
    linker = next((entry for entry in target["target_sources"] if "linker" in entry), None)
    if not linker:
        return []
    inputs = []
    for value in linker.get("parameters", []):
        if value.startswith("/") or value.startswith("-Wl") or value.startswith("-l") or value in {"-shared", "-fPIC", "-pthread", "-lm"}:
            continue
        if value.startswith("libvips/libvips.so") or value == "fuzz/libstandalone_engine.a":
            continue
        if value.endswith((".a", ".so", ".o")):
            inputs.append(value)
    return sorted(set(inputs))


def manifest_case(
    *,
    category: str,
    name: str,
    objects: list[Path],
    link_lang: str,
    output: str,
    run: dict[str, object],
    extra_link_inputs: list[str],
    support_objects: list[Path] | None = None,
    pkg_config: str = "vips",
) -> dict[str, object]:
    case = {
        "category": category,
        "name": name,
        "objects": [str(path.resolve()) for path in objects],
        "link_lang": link_lang,
        "output": output,
        "pkg_config": pkg_config,
        "extra_link_inputs": extra_link_inputs,
        "run": run,
    }
    if support_objects:
        case["support_objects"] = [str(path.resolve()) for path in support_objects]
    return case


def link_compat_manifest(
    *,
    build_check: Path,
    original_root: Path,
    dependents: Path,
    cves: Path,
) -> dict[str, object]:
    targets = {
        target["name"]: target
        for target in json.loads((build_check / "meson-info" / "intro-targets.json").read_text())
    }
    sample_dir = original_root / "test" / "test-suite" / "images"
    corpus_glob = str((original_root / "fuzz" / "common_fuzzer_corpus" / "*").resolve())
    standalone_engine = build_check / "fuzz" / "libstandalone_engine.a.p" / "StandaloneFuzzTargetMain.c.o"

    cases = [
        manifest_case(
            category="test",
            name="test_connections",
            objects=[build_check / "test" / "test_connections.p" / "test_connections.c.o"],
            link_lang="c",
            output="test_connections",
            extra_link_inputs=relative_link_inputs(targets["test_connections"]),
            run={
                "argv": [
                    "@output@",
                    str((sample_dir / "sample.jpg").resolve()),
                    "@workdir@/test_connections.png",
                ],
                "artifacts": ["@workdir@/test_connections.png"],
            },
        ),
        manifest_case(
            category="test",
            name="test_descriptors",
            objects=[build_check / "test" / "test_descriptors.p" / "test_descriptors.c.o"],
            link_lang="c",
            output="test_descriptors",
            extra_link_inputs=relative_link_inputs(targets["test_descriptors"]),
            run={
                "argv": [
                    "@output@",
                    str((sample_dir / "sample.jpg").resolve()),
                ],
            },
        ),
        manifest_case(
            category="test",
            name="test_timeout_webpsave",
            objects=[build_check / "test" / "test_timeout_webpsave.p" / "test_timeout_webpsave.c.o"],
            link_lang="c",
            output="test_timeout_webpsave",
            extra_link_inputs=relative_link_inputs(targets["test_timeout_webpsave"]),
            run={"argv": ["@output@"]},
        ),
        manifest_case(
            category="tool",
            name="vips",
            objects=[build_check / "tools" / "vips.p" / "vips.c.o"],
            link_lang="c",
            output="vips",
            extra_link_inputs=relative_link_inputs(targets["vips"]),
            run={
                "argv": [
                    "@output@",
                    "avg",
                    str((sample_dir / "sample.jpg").resolve()),
                ],
            },
        ),
        manifest_case(
            category="tool",
            name="vipsedit",
            objects=[build_check / "tools" / "vipsedit.p" / "vipsedit.c.o"],
            link_lang="c",
            output="vipsedit",
            extra_link_inputs=relative_link_inputs(targets["vipsedit"]),
            run={
                "prepare": [
                    {
                        "argv": [
                            "@safe_prefix@/bin/vips",
                            "copy",
                            str((sample_dir / "sample.jpg").resolve()),
                            "@workdir@/vipsedit-input.v",
                        ],
                    }
                ],
                "argv": [
                    "@output@",
                    "--width",
                    "123",
                    "@workdir@/vipsedit-input.v",
                ],
                "post_check": {
                    "argv": [
                        "@safe_prefix@/bin/vipsheader",
                        "-f",
                        "width",
                        "@workdir@/vipsedit-input.v",
                    ],
                    "equals": "123",
                },
            },
        ),
        manifest_case(
            category="tool",
            name="vipsheader",
            objects=[build_check / "tools" / "vipsheader.p" / "vipsheader.c.o"],
            link_lang="c",
            output="vipsheader",
            extra_link_inputs=relative_link_inputs(targets["vipsheader"]),
            run={
                "argv": [
                    "@output@",
                    "-f",
                    "width",
                    str((sample_dir / "sample.jpg").resolve()),
                ],
            },
        ),
        manifest_case(
            category="tool",
            name="vipsthumbnail",
            objects=[build_check / "tools" / "vipsthumbnail.p" / "vipsthumbnail.c.o"],
            link_lang="c",
            output="vipsthumbnail",
            extra_link_inputs=relative_link_inputs(targets["vipsthumbnail"]),
            run={
                "argv": [
                    "@output@",
                    str((sample_dir / "sample.jpg").resolve()),
                    "-s",
                    "10",
                    "-o",
                    "@workdir@/thumb.jpg",
                ],
                "artifacts": ["@workdir@/thumb.jpg"],
            },
        ),
        manifest_case(
            category="example",
            name="annotate-animated",
            objects=[build_check / "examples" / "annotate-animated.p" / "annotate-animated.c.o"],
            link_lang="c",
            output="annotate-animated",
            extra_link_inputs=relative_link_inputs(targets["annotate-animated"]),
            run={
                "argv": [
                    "@output@",
                    str((sample_dir / "garden.gif").resolve()) + "[n=-1]",
                    "@workdir@/annotated.webp",
                ],
                "artifacts": ["@workdir@/annotated.webp"],
            },
        ),
        manifest_case(
            category="example",
            name="new-from-buffer",
            objects=[build_check / "examples" / "new-from-buffer.p" / "new-from-buffer.c.o"],
            link_lang="c",
            output="new-from-buffer",
            extra_link_inputs=relative_link_inputs(targets["new-from-buffer"]),
            run={
                "argv": [
                    "@output@",
                    str((sample_dir / "sample.jpg").resolve()),
                ],
            },
        ),
        manifest_case(
            category="example",
            name="progress-cancel",
            objects=[build_check / "examples" / "progress-cancel.p" / "progress-cancel.c.o"],
            link_lang="c",
            output="progress-cancel",
            extra_link_inputs=relative_link_inputs(targets["progress-cancel"]),
            run={
                "argv": [
                    "@output@",
                    str((sample_dir / "sample.jpg").resolve()),
                    ".jpg",
                ],
            },
        ),
        manifest_case(
            category="example",
            name="use-vips-func",
            objects=[build_check / "examples" / "use-vips-func.p" / "use-vips-func.c.o"],
            link_lang="c",
            output="use-vips-func",
            extra_link_inputs=relative_link_inputs(targets["use-vips-func"]),
            run={
                "argv": [
                    "@output@",
                    str((sample_dir / "sample.jpg").resolve()),
                    "@workdir@/use-vips-func-out.jpg",
                ],
                "artifacts": ["@workdir@/use-vips-func-out.jpg"],
            },
        ),
    ]

    for target_name in [
        "gifsave_buffer_fuzzer",
        "jpegsave_buffer_fuzzer",
        "jpegsave_file_fuzzer",
        "mosaic_fuzzer",
        "pngsave_buffer_fuzzer",
        "sharpen_fuzzer",
        "smartcrop_fuzzer",
        "thumbnail_fuzzer",
        "webpsave_buffer_fuzzer",
    ]:
        cases.append(
            manifest_case(
                category="fuzz",
                name=target_name,
                objects=[build_check / "fuzz" / f"{target_name}.p" / f"{target_name}.cc.o"],
                support_objects=[standalone_engine],
                link_lang="cpp",
                output=target_name,
                extra_link_inputs=relative_link_inputs(targets[target_name]),
                run={
                    "mode": "corpus",
                    "corpus_glob": corpus_glob,
                    "argv": ["@output@", "@corpus@"],
                },
            )
        )

    cpp_objects = [
        build_check / "cplusplus" / "libvips-cpp.so.42.17.1.p" / name
        for name in [
            "VConnection.cpp.o",
            "VError.cpp.o",
            "VImage.cpp.o",
            "VInterpolate.cpp.o",
            "VRegion.cpp.o",
        ]
    ]
    cases.append(
        manifest_case(
            category="cplusplus",
            name="libvips-cpp",
            objects=cpp_objects,
            link_lang="cpp",
            output="libvips-cpp.so.42.17.1",
            pkg_config="vips-cpp",
            extra_link_inputs=relative_link_inputs(targets["vips-cpp"]),
            run={
                "mode": "cplusplus_smoke",
                "smoke_source": "tests/link_compat/vips_cpp_smoke.cpp",
                "argv": [
                    "@workdir@/vips_cpp_smoke",
                    str((sample_dir / "sample.jpg").resolve()),
                ],
            },
        )
    )

    return {
        "metadata": {
            "build_check": str(build_check.resolve()),
            "dependents": str(dependents.resolve()),
            "dependents_count": len(json.loads(dependents.read_text())),
            "cves": str(cves.resolve()),
            "cve_count": len(json.loads(cves.read_text())),
        },
        "cases": cases,
    }


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Capture committed libvips reference contracts from prepared local artifacts.",
    )
    parser.add_argument("--original-root", required=True, type=Path)
    parser.add_argument("--build-check", required=True, type=Path)
    parser.add_argument("--reference-install", required=True, type=Path)
    parser.add_argument("--compile-log", required=True, type=Path)
    parser.add_argument("--meson-test-log", required=True, type=Path)
    parser.add_argument("--pytest-log", required=True, type=Path)
    parser.add_argument("--dependents", required=True, type=Path)
    parser.add_argument("--cves", required=True, type=Path)
    args = parser.parse_args()

    safe_root = Path(__file__).resolve().parent.parent
    header_dir = args.reference_install / "include" / "vips"
    sync_headers(args.reference_install, safe_root)

    libvips = args.reference_install / "lib" / "libvips.so.42.17.1"
    libvips_cpp = args.reference_install / "lib" / "libvips-cpp.so.42.17.1"
    exported_vips = read_symbols(libvips)
    exported_vips_set = set(exported_vips)
    exported_cpp = read_symbols(libvips_cpp)

    public_files = header_files(header_dir)
    public_decl_lines = public_api_decls(header_dir)
    tests = extract_tests(args.original_root, args.meson_test_log, args.pytest_log)
    module_dir, installed_modules = module_contract(args.original_root)
    module_registry = build_module_registry(args.original_root)
    definitions = load_type_definitions(args.original_root)
    module_type_metadata = load_module_type_metadata(safe_root)
    module_type_names = {
        type_name
        for entry in module_registry.values()
        for type_name in entry["types"]  # type: ignore[index]
    }
    module_type_names.update(module_type_metadata)

    env = dict(os.environ, LD_LIBRARY_PATH=str((args.reference_install / "lib").resolve()))
    operations_output = parse_vips_tree(
        run_command(
        [str(args.reference_install / "bin" / "vips"), "-l", "VipsOperation"],
        env=env,
        ),
        f"LD_LIBRARY_PATH={args.reference_install / 'lib'} {(args.reference_install / 'bin' / 'vips')} -l VipsOperation",
    )
    operations_output = repair_tree_manifest(
        operations_output,
        definitions=definitions,
        extra_type_names=module_type_names,
        module_type_metadata=module_type_metadata,
    )
    types_output = parse_vips_tree(
        run_command(
        [str(args.reference_install / "bin" / "vips"), "-l", "VipsObject"],
        env=env,
        ),
        f"LD_LIBRARY_PATH={args.reference_install / 'lib'} {(args.reference_install / 'bin' / 'vips')} -l VipsObject",
    )
    types_output = repair_tree_manifest(
        types_output,
        definitions=definitions,
        extra_type_names=module_type_names,
        module_type_metadata=module_type_metadata,
    )

    write_lines(
        safe_root / "reference" / "abi" / "libvips.symbols",
        exported_vips,
    )
    write_lines(
        safe_root / "reference" / "abi" / "libvips-cpp.symbols",
        exported_cpp,
    )
    write_lines(
        safe_root / "reference" / "abi" / "deprecated-im.symbols",
        deprecated_symbols(header_dir, exported_vips_set),
    )
    write_lines(
        safe_root / "reference" / "abi" / "core-bootstrap.symbols",
        bootstrap_symbols(public_decl_lines, exported_vips_set),
    )
    write_lines(
        safe_root / "reference" / "headers" / "public-files.txt",
        [f"# total={len(public_files)}", *public_files],
    )
    write_lines(
        safe_root / "reference" / "headers" / "public-api-decls.txt",
        [f"# total={len(public_decl_lines)}", *public_decl_lines],
    )
    write_lines(
        safe_root / "reference" / "tests" / "meson-tests.txt",
        [*tests["meson_comments"], *tests["meson_tests"]],
    )
    write_lines(
        safe_root / "reference" / "tests" / "standalone-shell-tests.txt",
        tests["standalone_shell_tests"],
    )
    write_lines(
        safe_root / "reference" / "tests" / "python-files.txt",
        tests["python_files"],
    )
    write_lines(
        safe_root / "reference" / "tests" / "python-requirements.txt",
        tests["python_requirements"],
    )
    write_lines(
        safe_root / "reference" / "tests" / "fuzz-targets.txt",
        tests["fuzz_targets"],
    )
    write_lines(
        safe_root / "reference" / "tests" / "tools.txt",
        tests["tools"],
    )
    write_lines(
        safe_root / "reference" / "tests" / "examples.txt",
        tests["examples"],
    )
    write_lines(
        safe_root / "reference" / "modules" / "module-dir.txt",
        [module_dir],
    )
    write_lines(
        safe_root / "reference" / "modules" / "installed-modules.txt",
        installed_modules,
    )
    write_json(
        safe_root / "reference" / "modules" / "module-registry.json",
        module_registry,
    )
    write_json(
        safe_root / "reference" / "operations.json",
        operations_output,
    )
    write_json(
        safe_root / "reference" / "types.json",
        types_output,
    )
    write_json(
        safe_root / "reference" / "objects" / "link-compat-manifest.json",
        link_compat_manifest(
            build_check=args.build_check,
            original_root=args.original_root,
            dependents=args.dependents,
            cves=args.cves,
        ),
    )
    copy_pkgconfig(args.reference_install, safe_root)

    print(f"wrote reference artifacts under {safe_root / 'reference'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
