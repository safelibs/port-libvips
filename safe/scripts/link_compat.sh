#!/usr/bin/env bash
set -euo pipefail

exec python3 - "$0" "$@" <<'PY'
from __future__ import annotations

import argparse
import glob
import json
import os
from pathlib import Path
import shlex
import subprocess
import sys


script_path = Path(sys.argv[1]).resolve()
safe_root = script_path.parent.parent
project_root = safe_root.parent

EXPECTED_NAMES = {
    "test_connections",
    "test_descriptors",
    "test_timeout_webpsave",
    "vips",
    "vipsedit",
    "vipsheader",
    "vipsthumbnail",
    "annotate-animated",
    "new-from-buffer",
    "progress-cancel",
    "use-vips-func",
    "gifsave_buffer_fuzzer",
    "jpegsave_buffer_fuzzer",
    "jpegsave_file_fuzzer",
    "mosaic_fuzzer",
    "pngsave_buffer_fuzzer",
    "sharpen_fuzzer",
    "smartcrop_fuzzer",
    "thumbnail_fuzzer",
    "webpsave_buffer_fuzzer",
    "libvips-cpp",
}


def run(cmd: list[str], *, env: dict[str, str] | None = None, cwd: Path | None = None, capture: bool = False) -> str:
    kwargs = {
        "check": True,
        "cwd": str(cwd) if cwd else None,
        "env": env,
        "text": True,
    }
    if capture:
        completed = subprocess.run(cmd, capture_output=True, **kwargs)
        return completed.stdout
    subprocess.run(cmd, **kwargs)
    return ""


def pkg_config(pcdir: Path, *args: str) -> list[str]:
    output = run(
        ["env", f"PKG_CONFIG_PATH={pcdir}", "pkg-config", *args],
        capture=True,
    ).strip()
    return shlex.split(output)


def find_first(root: Path, pattern: str) -> Path:
    matches = sorted(path.resolve() for path in root.rglob(pattern) if path.is_file())
    if not matches:
        raise SystemExit(f"unable to locate {pattern!r} under {root}")
    return matches[0]


def find_pkgconfig_dir(root: Path, filename: str) -> Path:
    return find_first(root, filename).parent


def resolve_manifest_path(text: str) -> Path:
    path = Path(text)
    if path.is_absolute():
        return path.resolve()
    return (project_root / path).resolve()


def resolve_script_path(text: str) -> Path:
    path = Path(text)
    if path.is_absolute():
        return path.resolve()
    return (safe_root / path).resolve()


def ensure_paths_exist(paths: list[Path], *, label: str) -> None:
    missing = [str(path) for path in paths if not path.exists()]
    if missing:
        raise SystemExit(f"missing {label}: {missing}")


def expand_arg(text: str, *, output: Path, case_workdir: Path, safe_prefix: Path, corpus: Path | None = None) -> str:
    result = text
    replacements = {
        "@output@": str(output),
        "@workdir@": str(case_workdir),
        "@safe_prefix@": str(safe_prefix),
    }
    if corpus is not None:
        replacements["@corpus@"] = str(corpus)
    for key, value in replacements.items():
        result = result.replace(key, value)
    return result


def runtime_env(*, safe_prefix: Path, safe_libdir: Path, cpp_libdir: Path | None = None, fuzz: bool = False) -> dict[str, str]:
    env = os.environ.copy()
    ld_parts = []
    if cpp_libdir is not None:
        ld_parts.append(str(cpp_libdir))
    ld_parts.append(str(safe_libdir))
    if env.get("LD_LIBRARY_PATH"):
        ld_parts.append(env["LD_LIBRARY_PATH"])
    env["LD_LIBRARY_PATH"] = ":".join(ld_parts)
    env["VIPSHOME"] = str(safe_prefix)
    if fuzz:
        env["VIPS_WARNING"] = "0"
    return env


def run_prepare_steps(steps: list[dict[str, object]], *, output: Path, case_workdir: Path, safe_prefix: Path, env: dict[str, str]) -> None:
    for step in steps:
        argv = [
            expand_arg(
                item,
                output=output,
                case_workdir=case_workdir,
                safe_prefix=safe_prefix,
            )
            for item in step["argv"]
        ]
        run(argv, env=env, cwd=case_workdir)


def run_post_check(post_check: dict[str, object], *, output: Path, case_workdir: Path, safe_prefix: Path, env: dict[str, str]) -> None:
    argv = [
        expand_arg(
            item,
            output=output,
            case_workdir=case_workdir,
            safe_prefix=safe_prefix,
        )
        for item in post_check["argv"]
    ]
    actual = run(argv, env=env, cwd=case_workdir, capture=True).strip()
    expected = str(post_check["equals"])
    if actual != expected:
        raise SystemExit(
            f"post-check for {output.name} expected {expected!r}, found {actual!r}"
        )


def verify_artifacts(artifacts: list[str], *, output: Path, case_workdir: Path, safe_prefix: Path) -> None:
    for artifact in artifacts:
        artifact_path = Path(
            expand_arg(
                artifact,
                output=output,
                case_workdir=case_workdir,
                safe_prefix=safe_prefix,
            )
        )
        if not artifact_path.exists():
            raise SystemExit(f"expected runtime artifact was not created: {artifact_path}")


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(
        description=(
            "Relink the captured upstream object-compatibility surface against the "
            "safe install and run the manifest-defined smoke coverage."
        )
    )
    parser.add_argument("--manifest", required=True, type=Path)
    parser.add_argument("--reference-install", required=True, type=Path)
    parser.add_argument("--build-check", required=True, type=Path)
    parser.add_argument("--safe-prefix", required=True, type=Path)
    parser.add_argument("--workdir", required=True, type=Path)
    args = parser.parse_args(argv)

    manifest = json.loads(args.manifest.read_text())
    cases = manifest.get("cases", [])
    seen_names = {case["name"] for case in cases}
    missing_names = sorted(EXPECTED_NAMES - seen_names)
    if missing_names:
        raise SystemExit(f"manifest is missing required link-compat cases: {missing_names}")

    build_check = args.build_check.resolve()
    reference_install = args.reference_install.resolve()
    safe_prefix = args.safe_prefix.resolve()
    workdir = args.workdir.resolve()
    workdir.mkdir(parents=True, exist_ok=True)
    bin_dir = workdir / "bin"
    fuzz_dir = workdir / "fuzz"
    lib_dir = workdir / "lib"
    obj_dir = workdir / "obj"
    run_dir = workdir / "run"
    for path in [bin_dir, fuzz_dir, lib_dir, obj_dir, run_dir]:
        path.mkdir(parents=True, exist_ok=True)

    reference_pcdir = find_pkgconfig_dir(reference_install, "vips.pc")
    safe_pcdir = find_pkgconfig_dir(safe_prefix, "vips.pc")
    safe_libvips = find_first(safe_prefix, "libvips.so.42.17.1")
    safe_libvips_cpp = find_first(safe_prefix, "libvips-cpp.so.42.17.1")
    safe_libdir = safe_libvips.parent
    if safe_libvips_cpp.parent != safe_libdir:
        raise SystemExit(
            f"libvips and libvips-cpp must share a runtime directory, found "
            f"{safe_libdir} and {safe_libvips_cpp.parent}"
        )

    cxx = os.environ.get("CXX", "c++")
    cc = os.environ.get("CC", "cc")

    for case in cases:
        name = str(case["name"])
        category = str(case["category"])
        output_name = str(case["output"])
        link_lang = str(case["link_lang"])
        case_workdir = run_dir / name
        case_workdir.mkdir(parents=True, exist_ok=True)

        objects = [resolve_manifest_path(item) for item in case.get("objects", [])]
        extra_link_inputs = [
            resolve_manifest_path(item) for item in case.get("extra_link_inputs", [])
        ]
        support_objects = [
            resolve_manifest_path(item) for item in case.get("support_objects", [])
        ]
        ensure_paths_exist(objects, label=f"objects for {name}")
        ensure_paths_exist(extra_link_inputs, label=f"extra link inputs for {name}")
        ensure_paths_exist(support_objects, label=f"support objects for {name}")

        print(f"[link-compat] relinking {name}")
        if category == "cplusplus":
            real_output = lib_dir / output_name
            vips_libs = pkg_config(safe_pcdir, "--libs", "vips")
            link_cmd = [
                cxx,
                "-shared",
                "-fPIC",
                "-Wl,--no-undefined",
                "-Wl,-soname,libvips-cpp.so.42",
                f"-Wl,-rpath,{safe_libdir}",
                "-o",
                str(real_output),
                *map(str, objects),
                *map(str, extra_link_inputs),
                *map(str, support_objects),
                *vips_libs,
            ]
            run(link_cmd)
            soname = lib_dir / "libvips-cpp.so.42"
            link_name = lib_dir / "libvips-cpp.so"
            for symlink, target in [(soname, real_output.name), (link_name, soname.name)]:
                if symlink.exists() or symlink.is_symlink():
                    symlink.unlink()
                symlink.symlink_to(target)

            smoke_source = resolve_script_path(str(case["run"]["smoke_source"]))
            smoke_object = obj_dir / "vips_cpp_smoke.o"
            smoke_binary = case_workdir / "vips_cpp_smoke"
            compile_cmd = [
                cxx,
                "-std=c++11",
                "-c",
                str(smoke_source),
                "-o",
                str(smoke_object),
                *pkg_config(reference_pcdir, "--cflags", "vips-cpp"),
            ]
            run(compile_cmd)
            smoke_link_cmd = [
                cxx,
                "-o",
                str(smoke_binary),
                str(smoke_object),
                f"-L{lib_dir}",
                "-lvips-cpp",
                f"-Wl,-rpath,{lib_dir}",
                f"-Wl,-rpath,{safe_libdir}",
                *pkg_config(safe_pcdir, "--libs", "vips"),
            ]
            run(smoke_link_cmd)
            case_output = smoke_binary
            case_env = runtime_env(
                safe_prefix=safe_prefix,
                safe_libdir=safe_libdir,
                cpp_libdir=lib_dir,
            )
        else:
            if category == "fuzz":
                case_output = fuzz_dir / output_name
            else:
                case_output = bin_dir / output_name
            linker = cxx if link_lang == "cpp" else cc
            link_cmd = [
                linker,
                "-o",
                str(case_output),
                *map(str, objects),
                *map(str, extra_link_inputs),
                *map(str, support_objects),
                f"-Wl,-rpath,{safe_libdir}",
                *pkg_config(safe_pcdir, "--libs", str(case["pkg_config"])),
            ]
            run(link_cmd)
            case_env = runtime_env(
                safe_prefix=safe_prefix,
                safe_libdir=safe_libdir,
                fuzz=(category == "fuzz"),
            )

        run_spec = case["run"]
        prepare_steps = run_spec.get("prepare", [])
        if prepare_steps:
            run_prepare_steps(
                prepare_steps,
                output=case_output,
                case_workdir=case_workdir,
                safe_prefix=safe_prefix,
                env=case_env,
            )

        mode = run_spec.get("mode", "argv")
        if mode == "corpus":
            corpus_glob = str(run_spec["corpus_glob"])
            corpus_files = sorted(Path(path).resolve() for path in glob.glob(corpus_glob))
            if not corpus_files:
                raise SystemExit(f"no corpus files matched {corpus_glob!r} for {name}")
            for corpus in corpus_files:
                argv = [
                    expand_arg(
                        item,
                        output=case_output,
                        case_workdir=case_workdir,
                        safe_prefix=safe_prefix,
                        corpus=corpus,
                    )
                    for item in run_spec["argv"]
                ]
                run(argv, env=case_env, cwd=case_workdir)
        else:
            argv = [
                expand_arg(
                    item,
                    output=case_output,
                    case_workdir=case_workdir,
                    safe_prefix=safe_prefix,
                )
                for item in run_spec["argv"]
            ]
            run(argv, env=case_env, cwd=case_workdir)

        if run_spec.get("post_check"):
            run_post_check(
                run_spec["post_check"],
                output=case_output,
                case_workdir=case_workdir,
                safe_prefix=safe_prefix,
                env=case_env,
            )
        if run_spec.get("artifacts"):
            verify_artifacts(
                list(run_spec["artifacts"]),
                output=case_output,
                case_workdir=case_workdir,
                safe_prefix=safe_prefix,
            )

    print(f"[link-compat] completed manifest-driven relink coverage in {workdir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[2:]))
PY
