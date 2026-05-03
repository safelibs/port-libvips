use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;
use serde_json::Value;

const SONAME: &str = "libvips.so.42";
const GENERATED_DIR: &str = "generated";
const EXPORT_MAP_NAME: &str = "export.map";
const FULL_EXPORT_MAP_NAME: &str = "export-full.map";
const CORE_BOOTSTRAP_PATH: &str = "reference/abi/core-bootstrap.symbols";
const FULL_EXPORT_SYMBOLS_PATH: &str = "reference/abi/libvips.symbols";
const DEPRECATED_SYMBOLS_PATH: &str = "reference/abi/deprecated-im.symbols";
const GENERATED_OPERATIONS_PATH: &str = "src/generated/operations.json";
const ERROR_SHIM_NAME: &str = "error_shim.c";
const API_SHIM_NAME: &str = "api_shim.c";
const WRAPPER_SHIM_NAME: &str = "operation_wrapper_shim.c";
const DEPRECATED_SHIM_NAME: &str = "deprecated_compat_shim.c";
const FALLBACK_SHIM_NAME: &str = "full_surface_fallback_shim.c";
const API_TEMPLATE_PATH: &str = "build_support/api_shim.c";
const PUBLIC_HEADER_DIR: &str = "include/vips";
const ORIGINAL_INTERNAL_HEADER_PATH: &str = "../original/libvips/include/vips/internal.h";
const ORIGINAL_INTERNAL_INCLUDE_DIR: &str = "../original/libvips/include";
const NSGIF_HEADER_PATH: &str = "../original/libvips/foreign/libnsgif/nsgif.h";
const NSGIF_INCLUDE_DIR: &str = "../original/libvips/foreign/libnsgif";
const LZW_HEADER_PATH: &str = "../original/libvips/foreign/libnsgif/lzw.h";

#[derive(Copy, Clone, Eq, PartialEq)]
enum ExportSurface {
    CoreBootstrap,
    Full,
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=VIPS_SAFE_EXPORT_SURFACE");
    println!("cargo:rerun-if-changed={CORE_BOOTSTRAP_PATH}");
    println!("cargo:rerun-if-changed={FULL_EXPORT_SYMBOLS_PATH}");
    println!("cargo:rerun-if-changed={DEPRECATED_SYMBOLS_PATH}");
    println!("cargo:rerun-if-changed={GENERATED_OPERATIONS_PATH}");
    println!("cargo:rerun-if-changed={API_TEMPLATE_PATH}");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let generated_dir = manifest_dir.join(GENERATED_DIR);
    fs::create_dir_all(&generated_dir).expect("create generated dir");
    let wrappers = read_generated_wrappers(&manifest_dir.join(GENERATED_OPERATIONS_PATH));
    let deprecated_exports = read_deprecated_exports(&manifest_dir);
    let export_surface = export_surface();
    for header in collect_public_headers(&manifest_dir.join(PUBLIC_HEADER_DIR)) {
        println!("cargo:rerun-if-changed={}", header.display());
    }
    for source in collect_files_recursive(&manifest_dir.join("src"), "rs") {
        println!("cargo:rerun-if-changed={}", source.display());
    }
    for extra_header in fallback_header_inputs(&manifest_dir) {
        if extra_header.starts_with(manifest_dir.join(PUBLIC_HEADER_DIR)) {
            continue;
        }
        println!("cargo:rerun-if-changed={}", extra_header.display());
    }

    let export_map_path = generated_dir.join(EXPORT_MAP_NAME);
    let core_export_map = render_export_map(&manifest_dir.join(CORE_BOOTSTRAP_PATH));
    fs::write(&export_map_path, core_export_map).expect("write export map");
    let full_export_map_path = generated_dir.join(FULL_EXPORT_MAP_NAME);
    let full_export_map = render_export_map(&manifest_dir.join(FULL_EXPORT_SYMBOLS_PATH));
    fs::write(&full_export_map_path, full_export_map).expect("write full export map");

    compile_c_shims(&manifest_dir, &wrappers, &deprecated_exports);

    let selected_export_map = match export_surface {
        ExportSurface::CoreBootstrap => &export_map_path,
        ExportSurface::Full => &full_export_map_path,
    };

    println!("cargo:rustc-cdylib-link-arg=-Wl,-soname,{SONAME}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,--version-script={}",
        selected_export_map.display()
    );
    println!("cargo:rustc-cdylib-link-arg=-Wl,--no-undefined");
}

fn export_surface() -> ExportSurface {
    match env::var("VIPS_SAFE_EXPORT_SURFACE") {
        Ok(value) if value == "full" => ExportSurface::Full,
        Ok(value) if value == "core-bootstrap" => ExportSurface::CoreBootstrap,
        Ok(value) if value.is_empty() => ExportSurface::Full,
        Ok(value) => panic!("unsupported VIPS_SAFE_EXPORT_SURFACE={value}"),
        Err(_) => ExportSurface::Full,
    }
}

fn render_export_map(symbols_path: &Path) -> String {
    let mut lines = vec!["VIPS_42 {".to_owned()];

    let mut symbols = read_symbols(symbols_path);
    symbols.sort();
    symbols.dedup();
    if !symbols.is_empty() {
        lines.push("  global:".to_owned());
        for symbol in symbols {
            lines.push(format!("    {symbol};"));
        }
    }

    lines.push("  local:".to_owned());
    lines.push("    *;".to_owned());
    lines.push("};".to_owned());
    lines.push(String::new());
    lines.join("\n")
}

fn read_symbols(path: &Path) -> Vec<String> {
    let Ok(contents) = fs::read_to_string(path) else {
        return Vec::new();
    };

    contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(ToOwned::to_owned)
        .collect()
}

fn collect_public_headers(dir: &Path) -> Vec<PathBuf> {
    let mut headers = fs::read_dir(dir)
        .expect("read public header dir")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("h"))
        .collect::<Vec<_>>();
    headers.sort();
    headers
}

fn collect_files_recursive(dir: &Path, extension: &str) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return files;
    };

    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            files.extend(collect_files_recursive(&path, extension));
        } else if path.extension().and_then(|ext| ext.to_str()) == Some(extension) {
            files.push(path);
        }
    }

    files.sort();
    files
}

fn fallback_header_inputs(manifest_dir: &Path) -> Vec<PathBuf> {
    let mut headers = collect_public_headers(&manifest_dir.join(PUBLIC_HEADER_DIR));
    for relative in [
        ORIGINAL_INTERNAL_HEADER_PATH,
        NSGIF_HEADER_PATH,
        LZW_HEADER_PATH,
    ] {
        let path = manifest_dir.join(relative);
        if path.is_file() {
            headers.push(path);
        }
    }
    headers.sort();
    headers
}

fn strip_comments(text: &str) -> String {
    let block_comments = Regex::new(r"(?s)/\*.*?\*/").expect("compile block comment regex");
    let line_comments = Regex::new(r"//.*").expect("compile line comment regex");
    let without_block = block_comments.replace_all(text, "");
    line_comments.replace_all(&without_block, "").into_owned()
}

fn strip_preprocessor_lines(text: &str) -> String {
    text.lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

fn normalized_c_text(text: &str) -> String {
    strip_preprocessor_lines(&strip_comments(text))
}

fn normalize_decl_space(value: &str) -> String {
    let gnu_attributes =
        Regex::new(r"G_GNUC_[A-Z_]+(?:\([^)]*\))?").expect("compile glib attribute regex");
    gnu_attributes
        .replace_all(value, "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn split_parameters(text: &str) -> Vec<String> {
    let mut parameters = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;
    for ch in text.chars() {
        match ch {
            ',' if depth == 0 => {
                if !current.trim().is_empty() {
                    parameters.push(current.trim().to_owned());
                }
                current.clear();
                continue;
            }
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
        current.push(ch);
    }
    if !current.trim().is_empty() {
        parameters.push(current.trim().to_owned());
    }
    parameters
}

fn parameter_name(parameter: &str) -> Option<String> {
    let parameter = parameter.trim();
    if parameter.is_empty() || parameter == "void" || parameter == "..." {
        return None;
    }

    let function_pointer =
        Regex::new(r"\(\s*\*\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)").expect("function pointer regex");
    if let Some(captures) = function_pointer.captures(parameter) {
        return captures.get(1).map(|name| name.as_str().to_owned());
    }

    let ident = Regex::new(r"[A-Za-z_][A-Za-z0-9_]*").expect("identifier regex");
    let keywords = [
        "const", "unsigned", "signed", "struct", "enum", "union", "volatile", "register", "extern",
        "static", "inline", "long", "short", "int", "char", "float", "double", "void", "size_t",
        "gboolean", "gint", "guint", "gdouble", "gpointer", "VipsPel",
    ];
    let tokens = ident
        .find_iter(parameter)
        .map(|token| token.as_str())
        .collect::<Vec<_>>();
    let Some(candidate) = tokens.last() else {
        return None;
    };
    if keywords.contains(candidate) {
        None
    } else {
        Some((*candidate).to_owned())
    }
}

fn parse_deprecated_function(symbol: &str, declaration: String) -> DeprecatedFunction {
    let before_symbol = declaration
        .split(symbol)
        .next()
        .expect("deprecated declaration return type")
        .trim()
        .to_owned();
    let open_paren = declaration
        .find('(')
        .expect("deprecated declaration open paren");
    let close_paren = declaration
        .rfind(')')
        .expect("deprecated declaration close paren");
    let parameter_text = &declaration[open_paren + 1..close_paren];
    let parameters = split_parameters(parameter_text)
        .into_iter()
        .map(|parameter| DeprecatedParameter {
            declaration: parameter.clone(),
            name: parameter_name(&parameter),
            variadic: parameter == "...",
        })
        .collect();

    DeprecatedFunction {
        symbol: symbol.to_owned(),
        declaration,
        return_type: before_symbol,
        parameters,
    }
}

fn find_declaration(text: &str, symbol: &str, function: bool) -> Option<String> {
    let escaped = regex::escape(symbol);
    let pattern = if function {
        format!(
            r"(?ms)(?:^|\n)\s*(?:[A-Z_]+(?:\([^\n;]*\))?\s+)*(?P<decl>[A-Za-z_][A-Za-z0-9_\s\*]*?\b{escaped}\s*\([^;]*\))(?:\s+[A-Z_][A-Z0-9_]*(?:\([^;]*\))?)*\s*;"
        )
    } else {
        format!(
            r"(?ms)(?:^|\n)\s*(?:[A-Z_]+(?:\([^\n;]*\))?\s+)*(?P<decl>[A-Za-z_][A-Za-z0-9_\s\*\[\]]*?\b{escaped}(?:\s*\[[^\]]*\])?)\s*;"
        )
    };
    let regex = Regex::new(&pattern).expect("compile deprecated declaration regex");
    regex
        .captures(text)
        .and_then(|captures| captures.name("decl"))
        .map(|decl| normalize_decl_space(decl.as_str()))
}

fn read_deprecated_exports(manifest_dir: &Path) -> Vec<DeprecatedExport> {
    let symbols = read_symbols(&manifest_dir.join(DEPRECATED_SYMBOLS_PATH));
    let headers = collect_public_headers(&manifest_dir.join(PUBLIC_HEADER_DIR));
    let header_text = headers
        .iter()
        .map(|path| fs::read_to_string(path).expect("read public header"))
        .collect::<Vec<_>>()
        .join("\n");
    let stripped = normalized_c_text(&header_text);

    symbols
        .into_iter()
        .map(|symbol| {
            if let Some(declaration) = find_declaration(&stripped, &symbol, true) {
                DeprecatedExport::Function(parse_deprecated_function(&symbol, declaration))
            } else if let Some(declaration) = find_declaration(&stripped, &symbol, false) {
                DeprecatedExport::Variable(DeprecatedVariable { declaration })
            } else {
                panic!("unable to locate deprecated declaration for {symbol}");
            }
        })
        .collect()
}

#[derive(Debug, Clone)]
struct WrapperParameter {
    text: String,
    name: Option<String>,
    variadic: bool,
}

#[derive(Debug, Clone)]
struct WrapperDefinition {
    function: String,
    nickname: String,
    last_fixed_name: Option<String>,
    variadic: bool,
    parameters: Vec<WrapperParameter>,
}

#[derive(Debug, Clone)]
struct DeprecatedParameter {
    declaration: String,
    name: Option<String>,
    variadic: bool,
}

#[derive(Debug, Clone)]
struct DeprecatedFunction {
    symbol: String,
    declaration: String,
    return_type: String,
    parameters: Vec<DeprecatedParameter>,
}

#[derive(Debug, Clone)]
struct DeprecatedVariable {
    declaration: String,
}

#[derive(Debug, Clone)]
enum DeprecatedExport {
    Function(DeprecatedFunction),
    Variable(DeprecatedVariable),
}

fn read_generated_wrappers(path: &Path) -> Vec<WrapperDefinition> {
    let contents = fs::read_to_string(path).expect("read generated operations.json");
    let root: Value = serde_json::from_str(&contents).expect("parse generated operations.json");
    let wrappers = root
        .get("wrappers")
        .and_then(Value::as_object)
        .expect("generated operations.json wrappers object");

    let mut definitions = wrappers
        .iter()
        .map(|(name, wrapper)| {
            let function = wrapper
                .get("function")
                .and_then(Value::as_str)
                .unwrap_or(name)
                .to_owned();
            let nickname = function
                .strip_prefix("vips_")
                .unwrap_or(function.as_str())
                .to_owned();
            let parameters = wrapper
                .get("parameters")
                .and_then(Value::as_array)
                .map(|parameters| {
                    parameters
                        .iter()
                        .map(|parameter| WrapperParameter {
                            text: parameter
                                .get("text")
                                .and_then(Value::as_str)
                                .expect("wrapper parameter text")
                                .to_owned(),
                            name: parameter
                                .get("name")
                                .and_then(Value::as_str)
                                .map(ToOwned::to_owned),
                            variadic: parameter
                                .get("variadic")
                                .and_then(Value::as_bool)
                                .unwrap_or(false),
                        })
                        .collect()
                })
                .unwrap_or_default();

            WrapperDefinition {
                function,
                nickname,
                last_fixed_name: wrapper
                    .get("last_fixed_name")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                variadic: wrapper
                    .get("variadic")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                parameters,
            }
        })
        .collect::<Vec<_>>();

    definitions.sort_by(|left, right| left.function.cmp(&right.function));
    definitions
}

fn read_rust_exports(src_dir: &Path) -> BTreeSet<String> {
    let extern_fn =
        Regex::new(r#"pub\s+(?:unsafe\s+)?extern\s+"C"\s+fn\s+([A-Za-z_][A-Za-z0-9_]*)"#)
            .expect("compile rust extern fn regex");
    let static_item = Regex::new(r#"pub\s+static(?:\s+mut)?\s+([A-Za-z_][A-Za-z0-9_]*)"#)
        .expect("compile rust static regex");
    let object_type_macro = Regex::new(r#"(?m)^\s*object_type!\(\s*([A-Za-z_][A-Za-z0-9_]*)"#)
        .expect("compile object_type macro regex");
    let type_getter_macro = Regex::new(
        r#"(?m)^\s*(?:boxed_getter|enum_getter|flags_getter)!\(\s*([A-Za-z_][A-Za-z0-9_]*)"#,
    )
    .expect("compile getter macro regex");

    let mut exports = BTreeSet::new();
    for path in collect_files_recursive(src_dir, "rs") {
        let contents = fs::read_to_string(&path).expect("read rust source");
        for captures in extern_fn.captures_iter(&contents) {
            exports.insert(captures[1].to_owned());
        }
        for captures in static_item.captures_iter(&contents) {
            exports.insert(captures[1].to_owned());
        }
        for captures in object_type_macro.captures_iter(&contents) {
            exports.insert(captures[1].to_owned());
        }
        for captures in type_getter_macro.captures_iter(&contents) {
            exports.insert(captures[1].to_owned());
        }
    }
    exports
}

fn read_c_public_exports(path: &Path) -> BTreeSet<String> {
    let declaration =
        Regex::new(r"(?ms)^VIPS_PUBLIC\s+.*?\b([A-Za-z_][A-Za-z0-9_]*)\s*\([^;{]*\)\s*\{")
            .expect("compile C public export regex");
    let contents = fs::read_to_string(path).expect("read public C shim");
    declaration
        .captures_iter(&contents)
        .map(|captures| captures[1].to_owned())
        .collect()
}

fn error_shim_exports() -> BTreeSet<String> {
    [
        "vips_error_buffer",
        "vips_error_buffer_copy",
        "vips_error_clear",
        "vips_error_freeze",
        "vips_error_thaw",
        "vips_error_g",
        "vips_error",
        "vips_error_system",
        "vips_error_exit",
    ]
    .into_iter()
    .map(ToOwned::to_owned)
    .collect()
}

fn deprecated_export_symbol(export: &DeprecatedExport) -> String {
    match export {
        DeprecatedExport::Function(function) => function.symbol.clone(),
        DeprecatedExport::Variable(variable) => variable_symbol(&variable.declaration),
    }
}

fn variable_symbol(declaration: &str) -> String {
    let symbol = Regex::new(r"([A-Za-z_][A-Za-z0-9_]*)\s*(?:\[[^\]]*\])?$")
        .expect("compile variable symbol regex");
    let declaration = declaration.trim().trim_end_matches(';').trim();
    symbol
        .captures(declaration)
        .and_then(|captures| captures.get(1))
        .map(|capture| capture.as_str().to_owned())
        .unwrap_or_else(|| panic!("unable to extract variable symbol from {declaration}"))
}

fn implemented_full_surface_symbols(
    manifest_dir: &Path,
    wrappers: &[WrapperDefinition],
    deprecated_exports: &[DeprecatedExport],
) -> BTreeSet<String> {
    let mut exports = read_rust_exports(&manifest_dir.join("src"));
    exports.extend(read_c_public_exports(&manifest_dir.join(API_TEMPLATE_PATH)));
    exports.extend(error_shim_exports());
    for wrapper in wrappers {
        exports.insert(wrapper.function.clone());
    }
    for export in deprecated_exports {
        exports.insert(deprecated_export_symbol(export));
    }
    for symbol in deprecated_manual_symbols() {
        exports.insert(symbol.to_owned());
    }
    for symbol in fallback_manual_symbols() {
        exports.insert(symbol.to_owned());
    }
    exports
}

fn missing_full_surface_symbols(
    manifest_dir: &Path,
    wrappers: &[WrapperDefinition],
    deprecated_exports: &[DeprecatedExport],
) -> Vec<String> {
    let full_symbols = read_symbols(&manifest_dir.join(FULL_EXPORT_SYMBOLS_PATH))
        .into_iter()
        .collect::<BTreeSet<_>>();
    let implemented = implemented_full_surface_symbols(manifest_dir, wrappers, deprecated_exports);
    full_symbols
        .difference(&implemented)
        .cloned()
        .collect::<Vec<_>>()
}

fn manual_wrapper(function: &str) -> bool {
    // Preserve the bespoke implementations that already dispatch to the
    // working Rust image runtime instead of routing them through metadata-only
    // generated operation types.
    matches!(
        function,
        "vips_avg"
            | "vips_affine"
            | "vips_arrayjoin"
            | "vips_bandrank"
            | "vips_bandjoin"
            | "vips_bandjoin_const"
            | "vips_case"
            | "vips_crop"
            | "vips_getpoint"
            | "vips_jpegload_buffer"
            | "vips_jpegsave_buffer"
            | "vips_linear"
            | "vips_pngload_buffer"
            | "vips_pngsave_buffer"
            | "vips_switch"
            | "vips_sum"
    )
}

fn render_wrapper_call(
    wrapper: &WrapperDefinition,
    fixed_argument_names: &[&str],
    split: bool,
) -> String {
    let target = if split {
        "vips_call_split"
    } else {
        "vips_call"
    };
    let operation_name = format!("\"{}\"", wrapper.nickname);

    if split {
        if fixed_argument_names.is_empty() {
            format!("{target}({operation_name}, ap)")
        } else {
            format!(
                "{target}({operation_name}, ap, {})",
                fixed_argument_names.join(", ")
            )
        }
    } else if fixed_argument_names.is_empty() {
        format!("{target}({operation_name}, NULL)")
    } else {
        format!(
            "{target}({operation_name}, {}, NULL)",
            fixed_argument_names.join(", ")
        )
    }
}

fn save_buffer_wrapper(wrapper: &WrapperDefinition) -> bool {
    wrapper.parameters.iter().any(|parameter| {
        parameter.name.as_deref() == Some("buf") && parameter.text.starts_with("void **")
    }) && wrapper.parameters.iter().any(|parameter| {
        parameter.name.as_deref() == Some("len") && parameter.text.starts_with("size_t *")
    })
}

fn load_buffer_wrapper(wrapper: &WrapperDefinition) -> bool {
    wrapper.parameters.iter().any(|parameter| {
        parameter.name.as_deref() == Some("buf")
            && parameter.text.starts_with("void *")
            && !parameter.text.starts_with("void **")
    }) && wrapper.parameters.iter().any(|parameter| {
        parameter.name.as_deref() == Some("len") && parameter.text.starts_with("size_t")
    }) && wrapper.parameters.iter().any(|parameter| {
        parameter.name.as_deref() == Some("out") && parameter.text.starts_with("VipsImage **")
    })
}

fn render_wrapper_shim(wrappers: &[WrapperDefinition]) -> String {
    let mut source = String::from(
        "// @generated by build.rs from src/generated/operations.json\n\
#include <stdarg.h>\n\
#include <vips/vips.h>\n\
\n\
#if defined(__GNUC__)\n\
#define VIPS_PUBLIC __attribute__((visibility(\"default\")))\n\
#else\n\
#define VIPS_PUBLIC\n\
#endif\n\
\n\
static int\n\
safe_vips_finish_save_buffer(int result, VipsArea *area, void **buf, size_t *len)\n\
{\n\
    if (result) {\n\
        if (area)\n\
            vips_area_unref(area);\n\
        return result;\n\
    }\n\
\n\
    if (area) {\n\
        if (buf) {\n\
            *buf = area->data;\n\
            area->free_fn = NULL;\n\
        }\n\
        if (len)\n\
            *len = area->length;\n\
\n\
        vips_area_unref(area);\n\
    }\n\
\n\
    return result;\n\
}\n\
\n",
    );

    for wrapper in wrappers {
        if manual_wrapper(&wrapper.function) {
            continue;
        }

        let parameter_list = wrapper
            .parameters
            .iter()
            .map(|parameter| parameter.text.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let fixed_argument_names = wrapper
            .parameters
            .iter()
            .filter(|parameter| !parameter.variadic)
            .map(|parameter| {
                parameter
                    .name
                    .as_deref()
                    .expect("fixed wrapper parameter name")
            })
            .collect::<Vec<_>>();

        source.push_str("VIPS_PUBLIC int\n");
        source.push_str(&wrapper.function);
        source.push('(');
        source.push_str(&parameter_list);
        source.push_str(")\n{\n");

        if load_buffer_wrapper(wrapper) {
            source.push_str("    va_list ap;\n");
            source.push_str("    VipsBlob *blob;\n");
            source.push_str("    int result;\n\n");
            source.push_str("    if (out)\n");
            source.push_str("        *out = NULL;\n\n");
            source.push_str("    blob = vips_blob_new(NULL, buf, len);\n\n");
            source.push_str("    va_start(ap, out);\n");
            source.push_str("    result = vips_call_split(\"");
            source.push_str(&wrapper.nickname);
            source.push_str("\", ap, blob, out);\n");
            source.push_str("    va_end(ap);\n\n");
            source.push_str("    vips_area_unref(VIPS_AREA(blob));\n\n");
            source.push_str("    return result;\n");
            source.push_str("}\n\n");
            continue;
        }

        if save_buffer_wrapper(wrapper) {
            source.push_str("    va_list ap;\n");
            source.push_str("    VipsArea *area;\n");
            source.push_str("    int result;\n\n");
            source.push_str("    if (buf)\n");
            source.push_str("        *buf = NULL;\n");
            source.push_str("    if (len)\n");
            source.push_str("        *len = 0;\n");
            source.push_str("    area = NULL;\n\n");
            source.push_str("    va_start(ap, len);\n");
            source.push_str("    result = vips_call_split(\"");
            source.push_str(&wrapper.nickname);
            source.push_str("\", ap, in, &area);\n");
            source.push_str("    va_end(ap);\n\n");
            source.push_str("    return safe_vips_finish_save_buffer(result, area, buf, len);\n");
            source.push_str("}\n\n");
            continue;
        }

        for parameter in wrapper
            .parameters
            .iter()
            .filter(|parameter| !parameter.variadic)
        {
            if parameter.name.as_deref() == Some("out")
                && parameter.text.starts_with("VipsImage **")
            {
                source.push_str("    if (out)\n");
                source.push_str("        *out = NULL;\n");
            }
        }
        if wrapper.parameters.iter().any(|parameter| {
            parameter.name.as_deref() == Some("out") && parameter.text.starts_with("VipsImage **")
        }) {
            source.push('\n');
        }

        if wrapper.variadic {
            let last_fixed_name = wrapper
                .last_fixed_name
                .as_deref()
                .or_else(|| fixed_argument_names.last().copied())
                .expect("variadic wrapper last fixed parameter");
            let call = render_wrapper_call(wrapper, &fixed_argument_names, true);
            source.push_str("    va_list ap;\n");
            source.push_str("    int result;\n\n");
            source.push_str("    va_start(ap, ");
            source.push_str(last_fixed_name);
            source.push_str(");\n");
            source.push_str("    result = ");
            source.push_str(&call);
            source.push_str(";\n");
            source.push_str("    va_end(ap);\n\n");
            source.push_str("    return result;\n");
        } else {
            let call = render_wrapper_call(wrapper, &fixed_argument_names, false);
            source.push_str("    return ");
            source.push_str(&call);
            source.push_str(";\n");
        }

        source.push_str("}\n\n");
    }

    source
}

fn compile_c_shims(
    manifest_dir: &Path,
    wrappers: &[WrapperDefinition],
    deprecated_exports: &[DeprecatedExport],
) {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let error_source_path = out_dir.join(ERROR_SHIM_NAME);
    let api_source_path = out_dir.join(API_SHIM_NAME);
    let wrapper_source_path = out_dir.join(WRAPPER_SHIM_NAME);
    let deprecated_source_path = out_dir.join(DEPRECATED_SHIM_NAME);
    let fallback_source_path = out_dir.join(FALLBACK_SHIM_NAME);
    fs::write(&error_source_path, render_error_shim()).expect("write error shim");
    fs::write(&api_source_path, render_api_shim()).expect("write api shim");
    fs::write(&wrapper_source_path, render_wrapper_shim(wrappers)).expect("write wrapper shim");
    fs::write(
        &deprecated_source_path,
        render_deprecated_shim(manifest_dir, wrappers, deprecated_exports),
    )
    .expect("write deprecated shim");
    fs::write(
        &fallback_source_path,
        render_fallback_shim(manifest_dir, wrappers, deprecated_exports),
    )
    .expect("write fallback shim");

    let gio = pkg_config::Config::new()
        .cargo_metadata(false)
        .probe("gio-2.0")
        .expect("probe gio-2.0");

    let mut build = cc::Build::new();
    build.cargo_metadata(false);
    build.file(&error_source_path);
    build.file(&api_source_path);
    build.file(&wrapper_source_path);
    build.file(&deprecated_source_path);
    build.file(&fallback_source_path);
    build.flag_if_supported("-std=c99");
    build.flag_if_supported("-fvisibility=hidden");
    build.warnings(false);
    build.include(manifest_dir.join("include"));
    build.include(manifest_dir.join(ORIGINAL_INTERNAL_INCLUDE_DIR));
    build.include(manifest_dir.join(NSGIF_INCLUDE_DIR));
    for include_path in gio.include_paths {
        build.include(include_path);
    }
    build.compile("vips_error_shim");

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static:+whole-archive=vips_error_shim");
}

fn deprecated_alias_targets(wrappers: &[WrapperDefinition]) -> BTreeSet<String> {
    let mut targets = read_symbols(Path::new(CORE_BOOTSTRAP_PATH))
        .into_iter()
        .collect::<BTreeSet<_>>();
    for wrapper in wrappers {
        targets.insert(wrapper.function.clone());
    }
    for symbol in [
        "vips_avg",
        "vips_bandjoin",
        "vips_bandjoin_const",
        "vips_crop",
        "vips_init",
        "vips_linear",
        "vips_pngload_buffer",
        "vips_pngsave_buffer",
        "vips_sum",
    ] {
        targets.insert(symbol.to_owned());
    }
    targets
}

fn non_void_parameter_count(parameters: &[DeprecatedParameter]) -> usize {
    parameters
        .iter()
        .filter(|parameter| parameter.declaration.trim() != "void")
        .count()
}

fn compatible_alias_target(
    function: &DeprecatedFunction,
    header_text: &str,
    available_targets: &BTreeSet<String>,
) -> Option<String> {
    if function
        .parameters
        .iter()
        .any(|parameter| parameter.variadic)
    {
        return None;
    }
    if !function.symbol.starts_with("im_") {
        return None;
    }

    let target = format!("vips_{}", &function.symbol[3..]);
    if !available_targets.contains(&target) {
        return None;
    }
    let target_decl = find_declaration(header_text, &target, true)?;
    let target_function = parse_deprecated_function(&target, target_decl);
    if function.return_type != target_function.return_type {
        return None;
    }
    if non_void_parameter_count(&function.parameters)
        != non_void_parameter_count(&target_function.parameters)
    {
        return None;
    }
    if function
        .parameters
        .iter()
        .filter(|parameter| parameter.declaration.trim() != "void")
        .any(|parameter| parameter.name.is_none())
    {
        return None;
    }

    Some(target)
}

fn deprecated_manual_symbols() -> BTreeSet<&'static str> {
    [
        "im_BandFmt2char",
        "im_avg",
        "im_black",
        "im_Coding2char",
        "im_Compression2char",
        "im_Type2char",
        "im_char2BandFmt",
        "im_char2Coding",
        "im_char2Compression",
        "im_char2Type",
        "im_close",
        "im_copy",
        "im_char2dhint",
        "im_char2dtype",
        "im_dhint2char",
        "im_diagnostics",
        "im_dtype2char",
        "im_errormsg",
        "im_errormsg_system",
        "im_extract",
        "im_filename_split",
        "im_filename_suffix",
        "im_filename_suffix_match",
        "im_getnextoption",
        "im_getsuboption",
        "im_init",
        "im_init_world",
        "im_open",
        "im_open_local",
        "im_open_local_array",
        "im_skip_dir",
        "im_verrormsg",
        "im_warning",
        "vips_amiMSBfirst",
        "vips_image_open_input",
        "vips_image_open_output",
        "vips_mapfile",
        "vips_mapfilerw",
        "vips_path_filename7",
        "vips_path_mode7",
        "vips_remapfilerw",
    ]
    .into_iter()
    .collect()
}

fn fallback_manual_symbols() -> BTreeSet<&'static str> {
    ["vips_image_new_mode"].into_iter().collect()
}

fn render_deprecated_manual_block() -> &'static str {
    r#"
static void
safe_vips_deprecated_unimplemented(const char *symbol)
{
    vips_error(symbol, "%s is not implemented in the safe compatibility layer", symbol);
}

static void
safe_vips_im_verror(const char *domain, const char *fmt, va_list ap)
{
    char *message;

    message = g_strdup_vprintf(fmt, ap);
    vips_error(domain, "%s", message ? message : "");
    g_free(message);
}

static void
safe_vips_im_log(GLogLevelFlags level, const char *fmt, va_list ap)
{
    char *message;

    message = g_strdup_vprintf(fmt, ap);
    g_log("VIPS", level, "%s", message ? message : "");
    g_free(message);
}

VIPS_PUBLIC int
vips_amiMSBfirst(void)
{
#if G_BYTE_ORDER == G_BIG_ENDIAN
    return 1;
#elif G_BYTE_ORDER == G_LITTLE_ENDIAN
    return 0;
#else
#error "Byte order not recognised"
#endif
}

static const char *safe_im_type_names[] = {
    "IM_TYPE_MULTIBAND",
    "IM_TYPE_B_W",
    "LUMINACE",
    "XRAY",
    "IR",
    "YUV",
    "RED_ONLY",
    "GREEN_ONLY",
    "BLUE_ONLY",
    "POWER_SPECTRUM",
    "IM_TYPE_HISTOGRAM",
    "LUT",
    "IM_TYPE_XYZ",
    "IM_TYPE_LAB",
    "CMC",
    "IM_TYPE_CMYK",
    "IM_TYPE_LABQ",
    "IM_TYPE_RGB",
    "IM_TYPE_UCS",
    "IM_TYPE_LCH",
    "IM_TYPE_LABS",
    "<unknown>",
    "IM_TYPE_sRGB",
    "IM_TYPE_YXY",
    "IM_TYPE_FOURIER",
    "IM_TYPE_RGB16",
    "IM_TYPE_GREY16",
    NULL
};

static const char *safe_im_bandfmt_names[] = {
    "IM_BANDFMT_UCHAR",
    "IM_BANDFMT_CHAR",
    "IM_BANDFMT_USHORT",
    "IM_BANDFMT_SHORT",
    "IM_BANDFMT_UINT",
    "IM_BANDFMT_INT",
    "IM_BANDFMT_FLOAT",
    "IM_BANDFMT_COMPLEX",
    "IM_BANDFMT_DOUBLE",
    "IM_BANDFMT_DPCOMPLEX",
    NULL
};

static const char *safe_im_coding_names[] = {
    "IM_CODING_NONE",
    "COLQUANT8",
    "IM_CODING_LABQ",
    "IM_CODING_LABQ_COMPRESSED",
    "RGB_COMPRESSED",
    "LUM_COMPRESSED",
    "IM_CODING_RAD",
    NULL
};

static const char *safe_im_dtype_names[] = {
    "IM_NONE",
    "IM_SETBUF",
    "IM_SETBUF_FOREIGN",
    "IM_OPENIN",
    "IM_MMAPIN",
    "IM_MMAPINRW",
    "IM_OPENOUT",
    "IM_PARTIAL",
    NULL
};

static const char *safe_im_dhint_names[] = {
    "IM_SMALLTILE",
    "IM_FATSTRIP",
    "IM_THINSTRIP",
    "IM_ANY",
    NULL
};

static int
safe_lookup_enum(GType type, const char *names[], const char *name)
{
    GEnumClass *class;
    GEnumValue *value;
    int i;

    class = g_type_class_ref(type);
    if ((value = g_enum_get_value_by_nick(class, name))) {
        g_type_class_unref(class);
        return value->value;
    }
    if ((value = g_enum_get_value_by_name(class, name))) {
        g_type_class_unref(class);
        return value->value;
    }
    g_type_class_unref(class);

    for (i = 0; names[i]; i++)
        if (g_ascii_strcasecmp(names[i], name) == 0)
            return i;

    return -1;
}

VIPS_PUBLIC const char *
im_Type2char(VipsInterpretation type)
{
    return vips_enum_nick(VIPS_TYPE_INTERPRETATION, type);
}

VIPS_PUBLIC const char *
im_BandFmt2char(VipsBandFormat format)
{
    return vips_enum_nick(VIPS_TYPE_BAND_FORMAT, format);
}

VIPS_PUBLIC const char *
im_Coding2char(VipsCoding coding)
{
    return vips_enum_nick(VIPS_TYPE_CODING, coding);
}

VIPS_PUBLIC const char *
im_dtype2char(VipsImageType dtype)
{
    return vips_enum_nick(VIPS_TYPE_IMAGE_TYPE, dtype);
}

VIPS_PUBLIC const char *
im_dhint2char(VipsDemandStyle style)
{
    return vips_enum_nick(VIPS_TYPE_DEMAND_STYLE, style);
}

VIPS_PUBLIC VipsInterpretation
im_char2Type(const char *str)
{
    return safe_lookup_enum(VIPS_TYPE_INTERPRETATION, safe_im_type_names, str);
}

VIPS_PUBLIC VipsBandFormat
im_char2BandFmt(const char *str)
{
    return safe_lookup_enum(VIPS_TYPE_BAND_FORMAT, safe_im_bandfmt_names, str);
}

VIPS_PUBLIC VipsCoding
im_char2Coding(const char *str)
{
    return safe_lookup_enum(VIPS_TYPE_CODING, safe_im_coding_names, str);
}

VIPS_PUBLIC VipsImageType
im_char2dtype(const char *str)
{
    return safe_lookup_enum(VIPS_TYPE_IMAGE_TYPE, safe_im_dtype_names, str);
}

VIPS_PUBLIC VipsDemandStyle
im_char2dhint(const char *str)
{
    return safe_lookup_enum(VIPS_TYPE_DEMAND_STYLE, safe_im_dhint_names, str);
}

VIPS_PUBLIC const char *
im_Compression2char(int n)
{
    (void) n;
    return "NONE";
}

VIPS_PUBLIC int
im_char2Compression(const char *str)
{
    (void) str;
    return -1;
}

VIPS_PUBLIC void
im_errormsg(const char *fmt, ...)
{
    va_list ap;

    va_start(ap, fmt);
    safe_vips_im_verror("im_errormsg", fmt, ap);
    va_end(ap);
}

VIPS_PUBLIC void
im_verrormsg(const char *fmt, va_list ap)
{
    safe_vips_im_verror("im_verrormsg", fmt, ap);
}

VIPS_PUBLIC void
im_errormsg_system(int err, const char *fmt, ...)
{
    va_list ap;
    char *message;

    va_start(ap, fmt);
    message = g_strdup_vprintf(fmt, ap);
    va_end(ap);

    vips_error_system(err, "im_errormsg_system", "%s", message ? message : "");
    g_free(message);
}

VIPS_PUBLIC void
im_diagnostics(const char *fmt, ...)
{
    va_list ap;

    va_start(ap, fmt);
    safe_vips_im_log(G_LOG_LEVEL_INFO, fmt, ap);
    va_end(ap);
}

VIPS_PUBLIC void
im_warning(const char *fmt, ...)
{
    va_list ap;

    va_start(ap, fmt);
    safe_vips_im_log(G_LOG_LEVEL_WARNING, fmt, ap);
    va_end(ap);
}

VIPS_PUBLIC void
im_filename_split(const char *path, char *name, char *mode)
{
    char *p;
    size_t len;

    vips_strncpy(name, path, FILENAME_MAX);
    strcpy(mode, "");

    if ((len = strlen(name)) == 0)
        return;

    for (p = name + len - 1; p > name; p -= 1)
        if (*p == ':') {
            char *q;

            for (q = p - 1; isalnum(*q) && q > name; q -= 1)
                ;

            if (*q == '.')
                break;

            if (q == name)
                break;

            if (*q == '/' || *q == '\\')
                break;
        }

    if (*p == ':' && p - name != 1) {
        vips_strncpy(mode, p + 1, FILENAME_MAX);
        *p = '\0';
    }
}

VIPS_PUBLIC char *
vips_path_filename7(const char *path)
{
    char name[FILENAME_MAX];
    char mode[FILENAME_MAX];

    im_filename_split(path, name, mode);
    return g_strdup(name);
}

VIPS_PUBLIC char *
vips_path_mode7(const char *path)
{
    char name[FILENAME_MAX];
    char mode[FILENAME_MAX];

    im_filename_split(path, name, mode);
    return g_strdup(mode);
}

VIPS_PUBLIC const char *
im_skip_dir(const char *path)
{
    char name[FILENAME_MAX];
    char mode[FILENAME_MAX];
    const char *p;
    const char *q;
    const char native_dir_sep = G_DIR_SEPARATOR;
    const char non_native_dir_sep = native_dir_sep == '/' ? '\\' : '/';

    im_filename_split(path, name, mode);
    p = name + strlen(name);

    for (q = p; q > name && q[-1] != native_dir_sep; q--)
        ;
    if (q == name)
        for (q = p; q > name && q[-1] != non_native_dir_sep; q--)
            ;

    return path + (q - name);
}

VIPS_PUBLIC void
im_filename_suffix(const char *path, char *suffix)
{
    char name[FILENAME_MAX];
    char mode[FILENAME_MAX];
    char *p;

    im_filename_split(path, name, mode);
    if ((p = strrchr(name, '.')))
        strcpy(suffix, p);
    else
        strcpy(suffix, "");
}

VIPS_PUBLIC int
im_filename_suffix_match(const char *path, const char *suffixes[])
{
    char suffix[FILENAME_MAX];
    const char **p;

    im_filename_suffix(path, suffix);
    for (p = suffixes; *p; p++)
        if (g_ascii_strcasecmp(suffix, *p) == 0)
            return 1;

    return 0;
}

VIPS_PUBLIC char *
im_getnextoption(char **in)
{
    char *p;
    char *q;

    p = *in;
    q = p;

    if (!p || !*p)
        return NULL;

    for (;;) {
        if (!(p = strchr(p, ',')))
            break;
        if (p == q)
            break;
        if (p[-1] != '\\')
            break;

        p += 1;
    }

    if (p) {
        *p = '\0';
        *in = p + 1;
    }
    else
        *in = NULL;

    if (strlen(q) > 0)
        return q;
    else
        return NULL;
}

VIPS_PUBLIC char *
im_getsuboption(const char *buf)
{
    char *p;
    char *q;
    char *r;

    if (!(p = strchr((char *) buf, ':')))
        return NULL;

    p += 1;
    for (q = p; *q; q++)
        if (q[0] == '\\' && q[1] == ',')
            for (r = q; *r; r++)
                r[0] = r[1];

    return p;
}

VIPS_PUBLIC VipsImage *
im_init(const char *filename)
{
    VipsImage *image;

    image = vips_image_new();
    if (image) {
        g_free(image->filename);
        image->filename = g_strdup(filename ? filename : "");
    }

    return image;
}

VIPS_PUBLIC int
im_init_world(const char *argv0)
{
    return vips_init(argv0);
}

VIPS_PUBLIC VipsImage *
im_open(const char *filename, const char *mode)
{
    if (!filename || !mode) {
        vips_error("im_open", "%s", "filename and mode are required");
        return NULL;
    }

    if (strcmp(mode, "r") == 0 ||
        strcmp(mode, "rd") == 0)
        return vips_image_new_from_file(filename, NULL);

    if (strcmp(mode, "rs") == 0)
        return vips_image_new_from_file(filename,
            "access", VIPS_ACCESS_SEQUENTIAL,
            NULL);

    return vips_image_new_mode(filename, mode);
}

VIPS_PUBLIC VipsImage *
im_open_local(VipsImage *parent,
    const char *filename, const char *mode)
{
    VipsImage *image;

    image = im_open(filename, mode);
    if (!image)
        return NULL;

    if (parent)
        g_signal_connect(parent, "close",
            G_CALLBACK(vips_object_local_cb), image);

    return image;
}

VIPS_PUBLIC int
im_open_local_array(VipsImage *parent,
    VipsImage **images, int n, const char *filename, const char *mode)
{
    int i;

    if (!images || n < 0) {
        vips_error("im_open_local_array", "%s", "images array is invalid");
        return -1;
    }

    for (i = 0; i < n; i++)
        if (!(images[i] = im_open_local(parent, filename, mode)))
            return -1;

    return 0;
}

VIPS_PUBLIC int
im_close(VipsImage *im)
{
    if (!im) {
        vips_error("im_close", "%s", "image is NULL");
        return -1;
    }

    g_object_unref(im);

    return 0;
}

VIPS_PUBLIC int
vips_image_open_input(VipsImage *image)
{
    (void) image;
    return 0;
}

VIPS_PUBLIC int
vips_image_open_output(VipsImage *image)
{
    (void) image;
    return 0;
}

VIPS_PUBLIC int
vips_mapfile(VipsImage *image)
{
    (void) image;
    return 0;
}

VIPS_PUBLIC int
vips_mapfilerw(VipsImage *image)
{
    (void) image;
    return 0;
}

VIPS_PUBLIC int
vips_remapfilerw(VipsImage *image)
{
    (void) image;
    return 0;
}

VIPS_PUBLIC int
im_avg(VipsImage *in, double *out)
{
    return vips_avg(in, out, NULL);
}

VIPS_PUBLIC int
im_copy(VipsImage *in, VipsImage *out)
{
    return vips_image_write(in, out);
}

VIPS_PUBLIC int
im_black(VipsImage *out, int x, int y, int bands)
{
    VipsImage *tmp;
    int result;

    tmp = NULL;
    if (vips_black(&tmp, x, y,
            "bands", bands,
            NULL))
        return -1;

    result = vips_image_write(tmp, out);
    g_object_unref(tmp);

    return result;
}

VIPS_PUBLIC int
im_extract(VipsImage *in, VipsImage *out, IMAGE_BOX *box)
{
    VipsImage *tmp;
    VipsImage *area;
    int result;

    if (!box) {
        vips_error("im_extract", "%s", "box is NULL");
        return -1;
    }

    tmp = NULL;
    area = NULL;
    if (box->chsel == -1) {
        if (vips_crop(in, &tmp,
                box->xstart, box->ystart,
                box->xsize, box->ysize,
                NULL))
            return -1;
    }
    else {
        if (vips_crop(in, &area,
                box->xstart, box->ystart,
                box->xsize, box->ysize,
                NULL) ||
            vips_extract_band(area, &tmp, box->chsel, NULL)) {
            g_clear_object(&area);
            return -1;
        }
    }

    result = vips_image_write(tmp, out);
    g_clear_object(&area);
    g_clear_object(&tmp);

    return result;
}
"#
}

fn deprecated_default_return(return_type: &str) -> Option<&'static str> {
    let return_type = return_type.trim();
    if return_type == "void" {
        None
    } else if return_type == "int" {
        Some("    return -1;\n")
    } else if return_type == "gboolean" {
        Some("    return FALSE;\n")
    } else if return_type.contains('*') {
        Some("    return NULL;\n")
    } else {
        Some("    return 0;\n")
    }
}

fn render_deprecated_function(
    function: &DeprecatedFunction,
    header_text: &str,
    available_targets: &BTreeSet<String>,
) -> String {
    let mut source = String::new();
    source.push_str("VIPS_PUBLIC ");
    source.push_str(&function.declaration);
    source.push_str("\n{\n");

    if let Some(target) = compatible_alias_target(function, header_text, available_targets) {
        let arguments = function
            .parameters
            .iter()
            .filter(|parameter| parameter.declaration.trim() != "void")
            .filter_map(|parameter| parameter.name.as_deref())
            .collect::<Vec<_>>()
            .join(", ");
        if function.return_type.trim() == "void" {
            source.push_str("    ");
            source.push_str(&target);
            source.push('(');
            source.push_str(&arguments);
            source.push_str(");\n");
            source.push_str("    return;\n");
        } else {
            source.push_str("    return ");
            source.push_str(&target);
            source.push('(');
            source.push_str(&arguments);
            source.push_str(");\n");
        }
    } else {
        source.push_str("    safe_vips_deprecated_unimplemented(\"");
        source.push_str(&function.symbol);
        source.push_str("\");\n");
        if let Some(default_return) = deprecated_default_return(&function.return_type) {
            source.push_str(default_return);
        }
    }

    source.push_str("}\n\n");
    source
}

fn render_deprecated_shim(
    manifest_dir: &Path,
    wrappers: &[WrapperDefinition],
    deprecated_exports: &[DeprecatedExport],
) -> String {
    let header_text = strip_comments(
        &collect_public_headers(&manifest_dir.join(PUBLIC_HEADER_DIR))
            .into_iter()
            .map(|path| fs::read_to_string(path).expect("read public header for alias checks"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    let available_targets = deprecated_alias_targets(wrappers);
    let manual_symbols = deprecated_manual_symbols();

    let mut source = String::from(
        "// @generated by build.rs from reference/abi/deprecated-im.symbols\n\
#include <ctype.h>\n\
#include <stdarg.h>\n\
#include <stdio.h>\n\
#include <string.h>\n\
#include <vips/vips.h>\n\
#include <vips/vips7compat.h>\n\
\n\
#if defined(__GNUC__)\n\
#define VIPS_PUBLIC __attribute__((visibility(\"default\")))\n\
#else\n\
#define VIPS_PUBLIC\n\
#endif\n\
\n",
    );
    source.push_str(render_deprecated_manual_block());
    source.push('\n');

    for export in deprecated_exports {
        match export {
            DeprecatedExport::Function(function) => {
                if available_targets.contains(&function.symbol) {
                    continue;
                }
                if manual_symbols.contains(function.symbol.as_str()) {
                    continue;
                }
                source.push_str(&render_deprecated_function(
                    function,
                    &header_text,
                    &available_targets,
                ));
            }
            DeprecatedExport::Variable(variable) => {
                source.push_str("VIPS_PUBLIC ");
                source.push_str(&variable.declaration);
                source.push_str(" = {0};\n");
            }
        }
    }
    source
}

fn fallback_default_return(return_type: &str) -> Option<&'static str> {
    let return_type = return_type.trim();
    if return_type == "void" {
        None
    } else if return_type == "gboolean" || return_type == "bool" {
        Some("    return FALSE;\n")
    } else if return_type.contains('*') {
        Some("    return NULL;\n")
    } else if matches!(
        return_type,
        "int" | "gint64" | "off_t" | "lzw_result" | "nsgif_error"
    ) {
        Some("    return -1;\n")
    } else {
        Some("    return 0;\n")
    }
}

fn render_fallback_function(symbol: &str, declaration: &str) -> String {
    let function = parse_deprecated_function(symbol, declaration.to_owned());
    let mut source = String::new();
    source.push_str("VIPS_PUBLIC ");
    source.push_str(declaration);
    source.push_str("\n{\n");
    source.push_str("    safe_vips_missing_symbol(\"");
    source.push_str(symbol);
    source.push_str("\");\n");
    if let Some(default_return) = fallback_default_return(&function.return_type) {
        source.push_str(default_return);
    }
    source.push_str("}\n\n");
    source
}

fn render_fallback_variable(declaration: &str) -> String {
    let declaration = declaration
        .trim()
        .trim_end_matches(';')
        .trim()
        .trim_start_matches("extern ")
        .trim();
    let initializer = if declaration.contains('[') {
        "{0}"
    } else {
        "0"
    };
    format!("VIPS_PUBLIC {declaration} = {initializer};\n\n")
}

fn render_fallback_shim(
    manifest_dir: &Path,
    wrappers: &[WrapperDefinition],
    deprecated_exports: &[DeprecatedExport],
) -> String {
    let header_text = normalized_c_text(
        &fallback_header_inputs(manifest_dir)
            .into_iter()
            .map(|path| fs::read_to_string(path).expect("read fallback header"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    let missing_symbols = missing_full_surface_symbols(manifest_dir, wrappers, deprecated_exports);

    let mut source = String::from(
        "// @generated by build.rs from reference/abi/libvips.symbols and in-repo headers\n\
#include <glib.h>\n\
#include <string.h>\n\
#include <vips/vips.h>\n\
#include <vips/private.h>\n\
#include <vips/internal.h>\n\
#include \"nsgif.h\"\n\
#include \"lzw.h\"\n\
\n\
#if defined(__GNUC__)\n\
#define VIPS_PUBLIC __attribute__((visibility(\"default\")))\n\
#else\n\
#define VIPS_PUBLIC\n\
#endif\n\
\n\
typedef void *im_object;\n\
\n\
static void\n\
safe_vips_missing_symbol(const char *symbol)\n\
{\n\
\tvips_error(symbol, \"%s is not implemented in the safe compatibility layer\", symbol);\n\
}\n\
\n",
    );
    source.push_str(
        r#"
static VipsImage *
safe_vips_image_new_mode_internal(const char *filename, const char *mode)
{
	if (!filename || !mode) {
		vips_error("vips_image_new_mode", "%s",
			"filename and mode are required");
		return NULL;
	}

	VipsImage *image = strcmp(mode, "t") == 0
		? vips_image_new_memory()
		: vips_image_new();
	if (!image)
		return NULL;

	g_object_set(image,
		"filename", filename,
		"mode", mode,
		NULL);

	return image;
}

VIPS_PUBLIC VipsImage *
vips_image_new_mode(const char *filename, const char *mode)
{
	return safe_vips_image_new_mode_internal(filename, mode);
}

"#,
    );

    for symbol in missing_symbols {
        if let Some(declaration) = find_declaration(&header_text, &symbol, true) {
            source.push_str(&render_fallback_function(&symbol, &declaration));
        } else if let Some(declaration) = find_declaration(&header_text, &symbol, false) {
            source.push_str(&render_fallback_variable(&declaration));
        } else {
            panic!("unable to locate declaration for fallback symbol {symbol}");
        }
    }

    source
}

fn render_error_shim() -> &'static str {
    r#"#include <glib.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#if defined(__GNUC__)
#define VIPS_PUBLIC __attribute__((visibility("default")))
#else
#define VIPS_PUBLIC
#endif

extern const char *vips_get_prgname(void);
extern void vips_shutdown(void);
typedef struct {
	GMutex mutex;
	GString *buffer;
	guint freeze_count;
} SafeVipsErrorState;

static SafeVipsErrorState *
safe_vips_error_state(void)
{
	static gsize state_ptr = 0;

	if (g_once_init_enter(&state_ptr)) {
		SafeVipsErrorState *state;

		state = g_new0(SafeVipsErrorState, 1);
		g_mutex_init(&state->mutex);
		state->buffer = g_string_new("");

		g_once_init_leave(&state_ptr, (gsize) state);
	}

	return (SafeVipsErrorState *) state_ptr;
}

static void
safe_vips_error_reset_locked(SafeVipsErrorState *state)
{
	g_string_truncate(state->buffer, 0);
}

static void
safe_vips_error_append_locked(SafeVipsErrorState *state,
	const char *domain, const char *message)
{
	if (state->freeze_count > 0) {
		return;
	}

	if (domain) {
		g_string_append(state->buffer, domain);
		g_string_append(state->buffer, ": ");
	}
	g_string_append(state->buffer, message ? message : "");
	if (state->buffer->len == 0 ||
		state->buffer->str[state->buffer->len - 1] != '\n') {
		g_string_append_c(state->buffer, '\n');
	}
}

void
safe_vips_error_append_internal(const char *domain, const char *message)
{
	SafeVipsErrorState *state;

	state = safe_vips_error_state();
	g_mutex_lock(&state->mutex);
	safe_vips_error_append_locked(state, domain, message);
	g_mutex_unlock(&state->mutex);
}

static void
safe_vips_verror(const char *domain, const char *fmt, va_list ap)
{
	char *message;

	message = g_strdup_vprintf(fmt, ap);
	safe_vips_error_append_internal(domain, message ? message : "");
	g_free(message);
}

static void
safe_vips_verror_system(int err, const char *domain, const char *fmt, va_list ap)
{
	gchar *utf8;
	const char *message;

	safe_vips_verror(domain, fmt, ap);

	utf8 = g_locale_to_utf8(strerror(err), -1, NULL, NULL, NULL);
	message = utf8 ? utf8 : strerror(err);
	safe_vips_error_append_internal("unix error", message);
	g_free(utf8);
}

VIPS_PUBLIC const char *
vips_error_buffer(void)
{
	SafeVipsErrorState *state;
	const char *buffer;

	state = safe_vips_error_state();
	g_mutex_lock(&state->mutex);
	buffer = state->buffer->str;
	g_mutex_unlock(&state->mutex);

	return buffer;
}

VIPS_PUBLIC char *
vips_error_buffer_copy(void)
{
	SafeVipsErrorState *state;
	char *buffer;

	state = safe_vips_error_state();
	g_mutex_lock(&state->mutex);
	buffer = g_strdup(state->buffer->str);
	safe_vips_error_reset_locked(state);
	g_mutex_unlock(&state->mutex);

	return buffer;
}

VIPS_PUBLIC void
vips_error_clear(void)
{
	SafeVipsErrorState *state;

	state = safe_vips_error_state();
	g_mutex_lock(&state->mutex);
	safe_vips_error_reset_locked(state);
	g_mutex_unlock(&state->mutex);
}

VIPS_PUBLIC void
vips_error_freeze(void)
{
	SafeVipsErrorState *state;

	state = safe_vips_error_state();
	g_mutex_lock(&state->mutex);
	state->freeze_count += 1;
	g_mutex_unlock(&state->mutex);
}

VIPS_PUBLIC void
vips_error_thaw(void)
{
	SafeVipsErrorState *state;

	state = safe_vips_error_state();
	g_mutex_lock(&state->mutex);
	if (state->freeze_count > 0) {
		state->freeze_count -= 1;
	}
	g_mutex_unlock(&state->mutex);
}

VIPS_PUBLIC void
vips_error_g(GError **error)
{
	SafeVipsErrorState *state;
	char *message;
	gsize len;

	if (!error) {
		return;
	}

	state = safe_vips_error_state();
	g_mutex_lock(&state->mutex);
	message = g_strdup(state->buffer->str);
	safe_vips_error_reset_locked(state);
	g_mutex_unlock(&state->mutex);

	len = strlen(message);
	if (len > 0 && message[len - 1] == '\n') {
		message[len - 1] = '\0';
	}

	*error = g_error_new_literal(
		g_quark_from_static_string("libvips"),
		-1,
		message);
	g_free(message);
}

VIPS_PUBLIC void
vips_error(const char *domain, const char *fmt, ...)
{
	va_list ap;

	va_start(ap, fmt);
	safe_vips_verror(domain, fmt, ap);
	va_end(ap);
}

VIPS_PUBLIC void
vips_error_system(int err, const char *domain, const char *fmt, ...)
{
	va_list ap;

	va_start(ap, fmt);
	safe_vips_verror_system(err, domain, fmt, ap);
	va_end(ap);
}

VIPS_PUBLIC void
vips_error_exit(const char *fmt, ...)
{
	const char *prgname;

	if (fmt) {
		va_list ap;

		prgname = vips_get_prgname();
		fprintf(stderr, "%s: ", prgname ? prgname : "vips");

		va_start(ap, fmt);
		vfprintf(stderr, fmt, ap);
		va_end(ap);

		fprintf(stderr, "\n");
	}

	fprintf(stderr, "%s", vips_error_buffer());
	vips_shutdown();
exit(1);
}
"#
}

fn render_api_shim() -> &'static str {
    include_str!("build_support/api_shim.c")
}
