#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


SCRIPT_DIR = Path(__file__).resolve().parent
SAFE_ROOT = SCRIPT_DIR.parent
REPO_ROOT = SAFE_ROOT.parent
ORIGINAL_ROOT = REPO_ROOT / "original" / "libvips"


ARG_FLAG_ALIASES = {
    "VIPS_ARGUMENT_REQUIRED_INPUT": [
        "VIPS_ARGUMENT_INPUT",
        "VIPS_ARGUMENT_REQUIRED",
        "VIPS_ARGUMENT_CONSTRUCT",
    ],
    "VIPS_ARGUMENT_OPTIONAL_INPUT": [
        "VIPS_ARGUMENT_INPUT",
        "VIPS_ARGUMENT_CONSTRUCT",
    ],
    "VIPS_ARGUMENT_REQUIRED_OUTPUT": [
        "VIPS_ARGUMENT_OUTPUT",
        "VIPS_ARGUMENT_REQUIRED",
        "VIPS_ARGUMENT_CONSTRUCT",
    ],
    "VIPS_ARGUMENT_OPTIONAL_OUTPUT": [
        "VIPS_ARGUMENT_OUTPUT",
        "VIPS_ARGUMENT_CONSTRUCT",
    ],
}


PRIMITIVE_TYPE_INFO = {
    "BOOL": {"gtype": "G_TYPE_BOOLEAN", "value_type": "gboolean"},
    "INT": {"gtype": "G_TYPE_INT", "value_type": "gint"},
    "UINT64": {"gtype": "G_TYPE_UINT64", "value_type": "guint64"},
    "DOUBLE": {"gtype": "G_TYPE_DOUBLE", "value_type": "gdouble"},
    "STRING": {"gtype": "G_TYPE_STRING", "value_type": "gchararray"},
    "POINTER": {"gtype": "G_TYPE_POINTER", "value_type": "gpointer"},
    "IMAGE": {"gtype": "VIPS_TYPE_IMAGE", "value_type": "VipsImage"},
    "INTERPOLATE": {
        "gtype": "VIPS_TYPE_INTERPOLATE",
        "value_type": "VipsInterpolate",
    },
}


TYPE_MACRO_OVERRIDES = {
    "VIPS_TYPE_IMAGE": "VipsImage",
    "VIPS_TYPE_SOURCE": "VipsSource",
    "VIPS_TYPE_TARGET": "VipsTarget",
    "VIPS_TYPE_CONNECTION": "VipsConnection",
    "VIPS_TYPE_INTERPOLATE": "VipsInterpolate",
    "VIPS_TYPE_BLOB": "VipsBlob",
    "VIPS_TYPE_AREA": "VipsArea",
    "VIPS_TYPE_ARRAY_DOUBLE": "VipsArrayDouble",
    "VIPS_TYPE_ARRAY_INT": "VipsArrayInt",
    "VIPS_TYPE_ARRAY_IMAGE": "VipsArrayImage",
    "VIPS_TYPE_REF_STRING": "VipsRefString",
    "VIPS_TYPE_SAVE_STRING": "VipsSaveString",
    "VIPS_TYPE_OPERATION_FLAGS": "VipsOperationFlags",
    "VIPS_TYPE_ARGUMENT_FLAGS": "VipsArgumentFlags",
}

WRAPPER_ALIASES = {
    "crop": "extract_area",
}


PROTOTYPE_RE = re.compile(
    r"(?:VIPS_API\s+)?int\s+(vips_[A-Za-z0-9_]+)\s*\((.*?)\)\s*(?:G_[A-Z0-9_]+\s*)*$",
    re.S,
)
ASSIGNMENT_RE = re.compile(
    r"(?:object_class|vobject_class)\s*->\s*(nickname|description)\s*=\s*(.+?);",
    re.S,
)
FLAGS_RE = re.compile(r"operation_class\s*->\s*flags\s*([|]?=)\s*(.+?);", re.S)
CLASS_INIT_RE_TEMPLATE = r"(?:static\s+)?void\s+{name}_class_init\s*\([^)]*\)\s*\{{"
DEFINE_MACRO_RE = re.compile(r"\b(?P<macro>G_DEFINE_[A-Z_]+)\s*\(")
ARG_MACRO_RE = re.compile(r"\bVIPS_ARG_([A-Z0-9_]+)\s*\(")


@dataclass
class TypeDefinition:
    type_name: str
    symbol_name: str
    parent_type_name: str | None
    abstract: bool
    body: str
    source_path: str


def load_json(path: Path) -> dict[str, object]:
    return json.loads(path.read_text())


def strip_translation(expr: str) -> str:
    expr = expr.strip()
    if expr.startswith("_(") and expr.endswith(")"):
        expr = expr[2:-1].strip()
    if expr.startswith("N_(") and expr.endswith(")"):
        expr = expr[3:-1].strip()
    pieces = re.findall(r'"((?:\\.|[^"])*)"', expr)
    if len(pieces) > 1:
        return "".join(json.loads(f'"{piece}"') for piece in pieces)
    if expr.startswith('"') and expr.endswith('"'):
        return json.loads(expr)
    return expr


def split_top_level(text: str) -> list[str]:
    parts: list[str] = []
    depth = 0
    current: list[str] = []
    in_string = False
    escaped = False
    for ch in text:
        if in_string:
            current.append(ch)
            if escaped:
                escaped = False
            elif ch == "\\":
                escaped = True
            elif ch == '"':
                in_string = False
            continue

        if ch == '"':
            in_string = True
            current.append(ch)
            continue
        if ch == "(":
            depth += 1
            current.append(ch)
            continue
        if ch == ")":
            depth -= 1
            current.append(ch)
            continue
        if ch == "," and depth == 0:
            parts.append("".join(current).strip())
            current = []
            continue
        current.append(ch)

    tail = "".join(current).strip()
    if tail:
        parts.append(tail)
    return parts


def read_balanced(text: str, open_index: int, open_char: str, close_char: str) -> tuple[str, int]:
    depth = 0
    in_string = False
    escaped = False
    for index in range(open_index, len(text)):
        ch = text[index]
        if in_string:
            if escaped:
                escaped = False
            elif ch == "\\":
                escaped = True
            elif ch == '"':
                in_string = False
            continue
        if ch == '"':
            in_string = True
            continue
        if ch == open_char:
            depth += 1
        elif ch == close_char:
            depth -= 1
            if depth == 0:
                return text[open_index + 1 : index], index + 1
    raise ValueError(f"unbalanced {open_char}{close_char} section")


def parse_define_macros(source_text: str, source_path: str) -> list[TypeDefinition]:
    definitions: list[TypeDefinition] = []
    for match in DEFINE_MACRO_RE.finditer(source_text):
        macro_name = match.group("macro")
        _, end = match.span()
        args_text, _ = read_balanced(source_text, source_text.find("(", match.start()), "(", ")")
        args = split_top_level(args_text)
        if len(args) < 2:
            continue
        type_name = args[0].strip()
        symbol_name = args[1].strip()
        class_init_re = re.compile(CLASS_INIT_RE_TEMPLATE.format(name=re.escape(symbol_name)))
        class_init_match = class_init_re.search(source_text, end)
        if not class_init_match:
            continue
        body_start = source_text.find("{", class_init_match.start())
        body, _ = read_balanced(source_text, body_start, "{", "}")
        definitions.append(
            TypeDefinition(
                type_name=type_name,
                symbol_name=symbol_name,
                parent_type_name=(
                    type_name_from_parent_expr(args[2]) if len(args) >= 3 else None
                ),
                abstract="ABSTRACT" in macro_name,
                body=body,
                source_path=source_path,
            )
        )
    return definitions


def parse_flag_expr(expr: str, aliases: dict[str, list[str]]) -> list[str]:
    tokens = [token.strip() for token in expr.replace("(", "").replace(")", "").split("|")]
    out: list[str] = []
    for token in tokens:
        if not token or token in {"0", "FALSE", "TRUE"}:
            continue
        if token in aliases:
            out.extend(parse_flag_expr(" | ".join(aliases[token]), aliases))
            continue
        if token not in out:
            out.append(token)
    return out


def type_name_from_macro(type_macro: str) -> str | None:
    type_macro = type_macro.strip()
    if type_macro in TYPE_MACRO_OVERRIDES:
        return TYPE_MACRO_OVERRIDES[type_macro]
    if type_macro.startswith("VIPS_TYPE_"):
        suffix = type_macro[len("VIPS_TYPE_") :]
        pieces = suffix.split("_")
        converted: list[str] = []
        for piece in pieces:
            if not piece:
                continue
            if piece.isupper():
                converted.append(piece.lower().capitalize())
            elif piece.islower():
                converted.append(piece.capitalize())
            else:
                converted.append(piece)
        return "Vips" + "".join(converted)
    return None


def type_name_from_parent_expr(expr: str) -> str | None:
    expr = expr.strip()
    macro_name = type_name_from_macro(expr)
    if macro_name is not None:
        return macro_name

    match = re.fullmatch(r"([a-z0-9_]+)_get_type\s*\(\s*\)", expr)
    if not match:
        return None

    pieces = [piece for piece in match.group(1).split("_") if piece]
    if not pieces:
        return None
    return "".join(piece.lower().capitalize() for piece in pieces)


def arg_type_info(kind: str, args: list[str]) -> dict[str, object]:
    if kind in PRIMITIVE_TYPE_INFO:
        return dict(PRIMITIVE_TYPE_INFO[kind])
    if kind == "OBJECT":
        type_macro = args[7]
        return {
            "gtype": type_macro,
            "value_type": type_name_from_macro(type_macro),
            "type_macro": type_macro,
        }
    if kind in {"BOXED", "ENUM", "FLAGS"}:
        type_macro = args[7]
        return {
            "gtype": type_macro,
            "value_type": type_name_from_macro(type_macro),
            "type_macro": type_macro,
        }
    raise ValueError(f"unsupported arg kind {kind}")


def default_value_for(kind: str, args: list[str]) -> str | None:
    if kind == "BOOL":
        return args[7]
    if kind == "DOUBLE":
        return args[9]
    if kind == "INT":
        return args[9]
    if kind == "UINT64":
        return args[9]
    if kind == "STRING":
        return strip_translation(args[7])
    if kind == "ENUM":
        return args[8]
    if kind == "FLAGS":
        return args[8]
    return None


def limits_for(kind: str, args: list[str]) -> dict[str, object]:
    if kind == "DOUBLE":
        return {"min": args[7], "max": args[8]}
    if kind == "INT":
        return {"min": args[7], "max": args[8]}
    if kind == "UINT64":
        return {"min": args[7], "max": args[8]}
    return {}


def parse_arguments(class_body: str) -> list[dict[str, object]]:
    arguments: list[dict[str, object]] = []
    index = 0
    while True:
        match = ARG_MACRO_RE.search(class_body, index)
        if not match:
            break
        kind = match.group(1)
        open_index = class_body.find("(", match.start())
        call_text, next_index = read_balanced(class_body, open_index, "(", ")")
        index = next_index
        args = split_top_level(call_text)
        if len(args) < 7:
            continue

        flags = parse_flag_expr(args[5], ARG_FLAG_ALIASES)
        argument: dict[str, object] = {
            "kind": kind,
            "name": strip_translation(args[1]),
            "priority": int(args[2]),
            "long_name": strip_translation(args[3]),
            "description": strip_translation(args[4]),
            "flags": flags,
            "raw_flags": args[5].strip(),
            "offset": args[6].strip(),
            "required": "VIPS_ARGUMENT_REQUIRED" in flags,
            "construct": "VIPS_ARGUMENT_CONSTRUCT" in flags,
            "direction": (
                "input"
                if "VIPS_ARGUMENT_INPUT" in flags
                else "output"
                if "VIPS_ARGUMENT_OUTPUT" in flags
                else "none"
            ),
            "type": arg_type_info(kind, args),
            "default": default_value_for(kind, args),
        }
        argument.update(limits_for(kind, args))
        arguments.append(argument)
    return arguments


def parse_wrapper_headers(
    include_dir: Path, operation_nicknames: set[str]
) -> dict[str, dict[str, object]]:
    wrappers: dict[str, dict[str, object]] = {}
    for header in sorted(include_dir.glob("*.h")):
        if header.name in {"private.h", "internal.h", "vips7compat.h", "deprecated.h"}:
            continue
        text = header.read_text()
        for statement in text.split(";"):
            statement = " ".join(statement.split()).strip()
            if not statement or "vips_" not in statement:
                continue
            match = PROTOTYPE_RE.match(statement)
            if not match:
                continue
            function_name = match.group(1)
            nickname = function_name.removeprefix("vips_")
            canonical_nickname = WRAPPER_ALIASES.get(nickname, nickname)
            if canonical_nickname not in operation_nicknames:
                continue
            signature = statement + ";"
            parameter_text = " ".join(match.group(2).split())
            last_fixed_name = None
            params: list[dict[str, object]] = []
            if parameter_text and parameter_text != "void":
                raw_params = split_top_level(parameter_text)
                for raw in raw_params:
                    raw = raw.strip()
                    if raw == "...":
                        params.append({"text": raw, "variadic": True})
                        continue
                    type_part, _, name_part = raw.rpartition(" ")
                    clean_name = name_part.strip().lstrip("*")
                    params.append(
                        {
                            "text": raw,
                            "type": type_part.strip(),
                            "name": clean_name,
                            "variadic": False,
                        }
                    )
                fixed = [param for param in params if not param.get("variadic")]
                if fixed:
                    last_fixed_name = fixed[-1]["name"]
            wrappers[canonical_nickname] = {
                "function": function_name,
                "header": header.name,
                "signature": signature,
                "parameters": params,
                "last_fixed_name": last_fixed_name,
                "variadic": parameter_text.endswith("..."),
            }
    return wrappers


def merge_metadata(
    reference_types: dict[str, object],
    reference_operations: dict[str, object],
    definitions: Iterable[TypeDefinition],
    wrappers: dict[str, dict[str, object]],
) -> dict[str, object]:
    definition_by_name = {definition.type_name: definition for definition in definitions}
    type_entries = {
        entry["type_name"]: dict(entry)
        for entry in reference_types["entries"]  # type: ignore[index]
    }
    operation_entries = {
        entry["type_name"]: dict(entry)
        for entry in reference_operations["entries"]  # type: ignore[index]
    }
    parent_names = {
        entry["parent"]
        for entry in reference_types["entries"]  # type: ignore[index]
        if entry.get("parent")
    }

    def is_abstract(type_name: str) -> bool:
        definition = definition_by_name.get(type_name)
        if definition is not None:
            return definition.abstract
        return False

    type_metadata: dict[str, dict[str, object]] = {
        name: {
            **entry,
            "source_file": None,
            "abstract": is_abstract(name),
        }
        for name, entry in type_entries.items()
    }
    operation_metadata: dict[str, dict[str, object]] = {}

    for definition in definitions:
        if definition.type_name not in type_metadata:
            continue
        meta = type_metadata[definition.type_name]
        meta["source_file"] = definition.source_path

        fields: dict[str, str] = {}
        for match in ASSIGNMENT_RE.finditer(definition.body):
            fields[match.group(1)] = strip_translation(match.group(2))
        if "nickname" in fields and not meta.get("nickname"):
            meta["nickname"] = fields["nickname"]
        if "description" in fields and not meta.get("description"):
            meta["description"] = fields["description"]

        flags: list[str] = []
        for match in FLAGS_RE.finditer(definition.body):
            op, expr = match.groups()
            parsed = parse_flag_expr(expr, {})
            if op == "=":
                flags = parsed
            else:
                for item in parsed:
                    if item not in flags:
                        flags.append(item)
        if definition.type_name in operation_entries:
            operation_entry = {
                **operation_entries[definition.type_name],
                "source_file": definition.source_path,
                "abstract": definition.abstract,
                "flags": flags,
                "arguments": parse_arguments(definition.body),
                "wrapper": wrappers.get(operation_entries[definition.type_name]["nickname"]),
            }
            operation_metadata[definition.type_name] = operation_entry

    for type_name, entry in operation_entries.items():
        if type_name not in operation_metadata:
            operation_metadata[type_name] = {
                **entry,
                "source_file": None,
                "abstract": is_abstract(type_name),
                "flags": [],
                "arguments": [],
                "wrapper": wrappers.get(entry["nickname"]),
            }

    supported_operations = {
        "avg",
        "black",
        "copy",
        "crop",
        "pngload",
        "pngload_buffer",
        "pngsave",
        "pngsave_buffer",
    }
    for entry in operation_metadata.values():
        entry["supported"] = entry["nickname"] in supported_operations

    return {
        "count": reference_operations["count"],
        "entries": reference_operations["entries"],
        "nicknames": reference_operations["nicknames"],
        "type_names": reference_operations["type_names"],
        "source": reference_operations["source"],
        "types": reference_types,
        "type_metadata": type_metadata,
        "operation_metadata": operation_metadata,
        "wrappers": {
            wrapper["function"]: wrapper for wrapper in wrappers.values()
        },
    }


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def main() -> int:
    parser = argparse.ArgumentParser(
        description=(
            "Extract operation/type introspection from the in-repo original libvips "
            "sources and merge it with the committed phase-1 reference manifests."
        )
    )
    parser.add_argument(
        "--safe-root",
        type=Path,
        default=SAFE_ROOT,
        help="Path to the safe Rust port root",
    )
    parser.add_argument(
        "--original-root",
        type=Path,
        default=ORIGINAL_ROOT,
        help="Path to the original libvips source tree",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=SAFE_ROOT / "src" / "generated" / "operations.json",
        help="Output JSON path",
    )
    args = parser.parse_args()

    safe_root = args.safe_root.resolve()
    original_root = args.original_root.resolve()
    reference_types = load_json(safe_root / "reference" / "types.json")
    reference_operations = load_json(safe_root / "reference" / "operations.json")

    definitions: list[TypeDefinition] = []
    for source_path in sorted(original_root.rglob("*.c")):
        source_text = source_path.read_text(errors="replace")
        definitions.extend(
            parse_define_macros(source_text, str(source_path.relative_to(REPO_ROOT)))
        )

    operation_nicknames = set(reference_operations["nicknames"])  # type: ignore[arg-type]
    wrappers = parse_wrapper_headers(original_root / "include" / "vips", operation_nicknames)
    merged = merge_metadata(reference_types, reference_operations, definitions, wrappers)
    write_json(args.output.resolve(), merged)
    print(f"wrote {args.output.resolve()}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
