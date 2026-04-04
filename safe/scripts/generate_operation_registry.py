#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
from pathlib import Path


SCRIPT_DIR = Path(__file__).resolve().parent
SAFE_ROOT = SCRIPT_DIR.parent


ARG_FLAG_VALUES = {
    "VIPS_ARGUMENT_NONE": "0",
    "VIPS_ARGUMENT_REQUIRED": "crate::abi::object::VIPS_ARGUMENT_REQUIRED",
    "VIPS_ARGUMENT_CONSTRUCT": "crate::abi::object::VIPS_ARGUMENT_CONSTRUCT",
    "VIPS_ARGUMENT_SET_ONCE": "crate::abi::object::VIPS_ARGUMENT_SET_ONCE",
    "VIPS_ARGUMENT_SET_ALWAYS": "crate::abi::object::VIPS_ARGUMENT_SET_ALWAYS",
    "VIPS_ARGUMENT_INPUT": "crate::abi::object::VIPS_ARGUMENT_INPUT",
    "VIPS_ARGUMENT_OUTPUT": "crate::abi::object::VIPS_ARGUMENT_OUTPUT",
    "VIPS_ARGUMENT_DEPRECATED": "crate::abi::object::VIPS_ARGUMENT_DEPRECATED",
    "VIPS_ARGUMENT_MODIFY": "crate::abi::object::VIPS_ARGUMENT_MODIFY",
    "VIPS_ARGUMENT_NON_HASHABLE": "crate::abi::object::VIPS_ARGUMENT_NON_HASHABLE",
}

OP_FLAG_VALUES = {
    "VIPS_OPERATION_NONE": "0",
    "VIPS_OPERATION_SEQUENTIAL": "crate::abi::operation::VIPS_OPERATION_SEQUENTIAL",
    "VIPS_OPERATION_SEQUENTIAL_UNBUFFERED": (
        "crate::abi::operation::VIPS_OPERATION_SEQUENTIAL_UNBUFFERED"
    ),
    "VIPS_OPERATION_NOCACHE": "crate::abi::operation::VIPS_OPERATION_NOCACHE",
    "VIPS_OPERATION_DEPRECATED": "crate::abi::operation::VIPS_OPERATION_DEPRECATED",
    "VIPS_OPERATION_UNTRUSTED": "crate::abi::operation::VIPS_OPERATION_UNTRUSTED",
    "VIPS_OPERATION_BLOCKED": "crate::abi::operation::VIPS_OPERATION_BLOCKED",
    "VIPS_OPERATION_REVALIDATE": "crate::abi::operation::VIPS_OPERATION_REVALIDATE",
}


KIND_MAP = {
    "BOOL": "GeneratedArgumentKind::Bool",
    "INT": "GeneratedArgumentKind::Int",
    "UINT64": "GeneratedArgumentKind::UInt64",
    "DOUBLE": "GeneratedArgumentKind::Double",
    "STRING": "GeneratedArgumentKind::String",
    "POINTER": "GeneratedArgumentKind::Pointer",
    "IMAGE": "GeneratedArgumentKind::Object",
    "INTERPOLATE": "GeneratedArgumentKind::Object",
    "OBJECT": "GeneratedArgumentKind::Object",
    "BOXED": "GeneratedArgumentKind::Boxed",
    "ENUM": "GeneratedArgumentKind::Enum",
    "FLAGS": "GeneratedArgumentKind::Flags",
}


def rust_string(value: str | None) -> str:
    if value is None:
        return "None"
    escaped = (
        value.replace("\\", "\\\\")
        .replace('"', '\\"')
        .replace("\n", "\\n")
        .replace("\r", "\\r")
    )
    return f'Some("{escaped}")'


def rust_string_literal(value: str) -> str:
    return rust_string(value).removeprefix("Some(").removesuffix(")")


def combine(values: list[str], mapping: dict[str, str]) -> str:
    parts = [mapping[value] for value in values if value in mapping]
    if not parts:
        return "0"
    return " | ".join(parts)


def render_argument(arg: dict[str, object], suffix: str) -> tuple[str, str]:
    name = str(arg["name"])
    type_info = arg["type"]
    value_type_name = type_info.get("value_type") if isinstance(type_info, dict) else None
    static_name = f"ARGS_{suffix}_{name.upper().replace('-', '_').replace('.', '_')}"
    source = f"""static {static_name}: GeneratedArgumentMetadata = GeneratedArgumentMetadata {{
    name: {rust_string_literal(name)},
    long_name: {rust_string_literal(str(arg["long_name"]))},
    description: {rust_string_literal(str(arg["description"]))},
    priority: {int(arg["priority"])},
    flags: {combine(list(arg["flags"]), ARG_FLAG_VALUES)},
    required: {"true" if arg["required"] else "false"},
    construct: {"true" if arg["construct"] else "false"},
    direction: {rust_string_literal(str(arg["direction"]))},
    kind: {KIND_MAP[str(arg["kind"])]},
    value_type_name: {rust_string(value_type_name if isinstance(value_type_name, str) else None)},
    default_value: {rust_string(str(arg["default"]) if arg.get("default") is not None else None)},
    min_value: {rust_string(str(arg["min"]) if arg.get("min") is not None else None)},
    max_value: {rust_string(str(arg["max"]) if arg.get("max") is not None else None)},
}};"""
    return static_name, source


def render_registry(metadata: dict[str, object]) -> str:
    type_metadata: dict[str, dict[str, object]] = metadata["type_metadata"]
    operation_metadata: dict[str, dict[str, object]] = metadata["operation_metadata"]

    argument_defs: list[str] = []
    operation_defs: list[str] = []
    type_entries: list[str] = []

    for type_name in metadata["types"]["type_names"]:
        type_entry = dict(type_metadata[type_name])
        operation_entry = operation_metadata.get(type_name)
        suffix = type_name.upper().replace("-", "_")

        argument_names: list[str] = []
        if operation_entry:
            for arg in operation_entry["arguments"]:
                static_name, source = render_argument(arg, suffix)
                argument_defs.append(source)
                argument_names.append(f"&{static_name}")

            op_static = f"OPERATION_{suffix}"
            operation_defs.append(
                f"""static {op_static}: GeneratedOperationMetadata = GeneratedOperationMetadata {{
    flags: {combine(list(operation_entry["flags"]), OP_FLAG_VALUES)},
    supported: {"true" if operation_entry.get("supported") else "false"},
    arguments: &[{", ".join(argument_names)}],
    wrapper_function: {rust_string(
        operation_entry["wrapper"]["function"]
        if isinstance(operation_entry.get("wrapper"), dict)
        else None
    )},
}};"""
            )
            operation_ref = f"Some(&{op_static})"
        else:
            operation_ref = "None"

        type_entries.append(
            f"""    GeneratedTypeMetadata {{
        type_name: {rust_string_literal(str(type_entry["type_name"]))},
        parent_type_name: {rust_string(str(type_entry.get("parent")))},
        nickname: {rust_string_literal(str(type_entry["nickname"]))},
        description: {rust_string_literal(str(type_entry["description"]))},
        depth: {int(type_entry["depth"])},
        abstract_: {"true" if type_entry.get("abstract") else "false"},
        source_file: {rust_string(type_entry.get("source_file"))},
        operation: {operation_ref},
    }},"""
        )

    header = """// @generated by scripts/generate_operation_registry.py
use crate::runtime::operation::{
    GeneratedArgumentKind, GeneratedArgumentMetadata, GeneratedOperationMetadata,
    GeneratedTypeMetadata,
};

"""
    return (
        header
        + "\n\n".join(argument_defs)
        + ("\n\n" if argument_defs else "")
        + "\n\n".join(operation_defs)
        + ("\n\n" if operation_defs else "")
        + "pub(crate) static GENERATED_TYPES: &[GeneratedTypeMetadata] = &[\n"
        + "\n".join(type_entries)
        + "\n];\n"
    )


def render_wrappers(metadata: dict[str, object]) -> str:
    wrapper_entries: list[str] = []
    seen: set[str] = set()
    for wrapper_name in sorted(metadata["wrappers"]):
        wrapper = metadata["wrappers"][wrapper_name]
        if wrapper_name in seen:
            continue
        seen.add(wrapper_name)
        params = wrapper.get("parameters", [])
        rendered_params = ", ".join(
            f"GeneratedWrapperParameter {{ text: {rust_string_literal(param['text'])}, name: {rust_string(param.get('name'))}, type_text: {rust_string(param.get('type'))}, variadic: {'true' if param.get('variadic') else 'false'} }}"
            for param in params
        )
        wrapper_entries.append(
            f"""    GeneratedWrapperMetadata {{
        function: {rust_string_literal(str(wrapper["function"]))},
        nickname: {rust_string_literal(str(wrapper["function"]).removeprefix("vips_"))},
        header: {rust_string_literal(str(wrapper["header"]))},
        signature: {rust_string_literal(str(wrapper["signature"]))},
        last_fixed_name: {rust_string(wrapper.get("last_fixed_name"))},
        variadic: {"true" if wrapper.get("variadic") else "false"},
        parameters: &[{rendered_params}],
    }},"""
        )

    return """// @generated by scripts/generate_operation_registry.py
use crate::runtime::operation::{GeneratedWrapperMetadata, GeneratedWrapperParameter};

pub(crate) static GENERATED_WRAPPERS: &[GeneratedWrapperMetadata] = &[
""" + "\n".join(wrapper_entries) + "\n];\n"


def main() -> int:
    parser = argparse.ArgumentParser(
        description=(
            "Generate Rust registry sources from the committed operation introspection JSON."
        )
    )
    parser.add_argument(
        "--input",
        type=Path,
        default=SAFE_ROOT / "src" / "generated" / "operations.json",
    )
    parser.add_argument(
        "--registry-output",
        type=Path,
        default=SAFE_ROOT / "src" / "generated" / "operations_registry.rs",
    )
    parser.add_argument(
        "--wrappers-output",
        type=Path,
        default=SAFE_ROOT / "src" / "generated" / "operation_wrappers.rs",
    )
    args = parser.parse_args()

    metadata = json.loads(args.input.read_text())
    args.registry_output.write_text(render_registry(metadata))
    args.wrappers_output.write_text(render_wrappers(metadata))
    print(f"wrote {args.registry_output}")
    print(f"wrote {args.wrappers_output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
