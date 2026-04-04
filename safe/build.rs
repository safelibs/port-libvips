use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

const SONAME: &str = "libvips.so.42";
const GENERATED_DIR: &str = "generated";
const EXPORT_MAP_NAME: &str = "export.map";
const CORE_BOOTSTRAP_PATH: &str = "reference/abi/core-bootstrap.symbols";
const GENERATED_OPERATIONS_PATH: &str = "src/generated/operations.json";
const ERROR_SHIM_NAME: &str = "error_shim.c";
const API_SHIM_NAME: &str = "api_shim.c";
const WRAPPER_SHIM_NAME: &str = "operation_wrapper_shim.c";
const API_TEMPLATE_PATH: &str = "build_support/api_shim.c";

const PHASE4_SYMBOLS: &[&str] = &[
    "vips_add_option_entries",
    "vips_argument_class_map",
    "vips_argument_map",
    "vips_call",
    "vips_call_argv",
    "vips_call_options",
    "vips_call_required_optional",
    "vips_call_split",
    "vips_call_split_option_string",
    "vips_class_find",
    "vips_class_map_all",
    "vips_isprefix",
    "vips_nickname_find",
    "vips_object_argument_needsstring",
    "vips_object_build",
    "vips_object_get_argument",
    "vips_object_class_install_argument",
    "vips_object_dump",
    "vips_object_get_argument_to_string",
    "vips_object_get_description",
    "vips_object_get_property",
    "vips_object_local_array",
    "vips_object_local_cb",
    "vips_object_map",
    "vips_object_new",
    "vips_object_new_from_string",
    "vips_object_preclose",
    "vips_object_print_all",
    "vips_object_print_dump",
    "vips_object_print_name",
    "vips_object_print_summary",
    "vips_object_print_summary_class",
    "vips_object_rewind",
    "vips_object_sanity",
    "vips_object_sanity_all",
    "vips_object_set",
    "vips_object_set_argument_from_string",
    "vips_object_set_from_string",
    "vips_object_set_property",
    "vips_object_set_required",
    "vips_object_set_static",
    "vips_object_set_valist",
    "vips_object_summary",
    "vips_object_summary_class",
    "vips_object_to_string",
    "vips_object_unref_outputs",
    "vips_operation_block_set",
    "vips_operation_call_valist",
    "vips_operation_class_print_usage",
    "vips_operation_get_flags",
    "vips_operation_invalidate",
    "vips_operation_new",
    "vips_type_depth",
    "vips_type_find",
    "vips_type_map",
    "vips_type_map_all",
    "vips_value_is_null",
    "vips_vector_disable_targets",
    "vips_vector_get_builtin_targets",
    "vips_vector_get_supported_targets",
    "vips_vector_isenabled",
    "vips_vector_set_enabled",
    "vips_vector_target_name",
];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={CORE_BOOTSTRAP_PATH}");
    println!("cargo:rerun-if-changed={GENERATED_OPERATIONS_PATH}");
    println!("cargo:rerun-if-changed={API_TEMPLATE_PATH}");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let generated_dir = manifest_dir.join(GENERATED_DIR);
    fs::create_dir_all(&generated_dir).expect("create generated dir");
    let wrappers = read_generated_wrappers(&manifest_dir.join(GENERATED_OPERATIONS_PATH));

    let export_map_path = generated_dir.join(EXPORT_MAP_NAME);
    let export_map = render_export_map(&manifest_dir.join(CORE_BOOTSTRAP_PATH), &wrappers);
    fs::write(&export_map_path, export_map).expect("write export map");

    compile_c_shims(&manifest_dir, &wrappers);

    println!("cargo:rustc-cdylib-link-arg=-Wl,-soname,{SONAME}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,--version-script={}",
        export_map_path.display()
    );
    println!("cargo:rustc-cdylib-link-arg=-Wl,--no-undefined");
}

fn render_export_map(symbols_path: &Path, wrappers: &[WrapperDefinition]) -> String {
    let mut lines = vec!["VIPS_42 {".to_owned()];

    let mut symbols = read_symbols(symbols_path);
    for symbol in PHASE4_SYMBOLS {
        if !symbols.iter().any(|existing| existing == symbol) {
            symbols.push((*symbol).to_owned());
        }
    }
    for wrapper in wrappers {
        if !symbols.iter().any(|existing| existing == &wrapper.function) {
            symbols.push(wrapper.function.clone());
        }
    }
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

fn manual_wrapper(function: &str) -> bool {
    // Preserve the bespoke implementations that already dispatch to the
    // working Rust image runtime instead of routing them through metadata-only
    // generated operation types.
    matches!(
        function,
        "vips_avg"
            | "vips_bandjoin"
            | "vips_bandjoin_const"
            | "vips_crop"
            | "vips_linear"
            | "vips_pngload_buffer"
            | "vips_pngsave_buffer"
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

fn compile_c_shims(manifest_dir: &Path, wrappers: &[WrapperDefinition]) {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let error_source_path = out_dir.join(ERROR_SHIM_NAME);
    let api_source_path = out_dir.join(API_SHIM_NAME);
    let wrapper_source_path = out_dir.join(WRAPPER_SHIM_NAME);
    fs::write(&error_source_path, render_error_shim()).expect("write error shim");
    fs::write(&api_source_path, render_api_shim()).expect("write api shim");
    fs::write(&wrapper_source_path, render_wrapper_shim(wrappers)).expect("write wrapper shim");

    let gio = pkg_config::Config::new()
        .cargo_metadata(false)
        .probe("gio-2.0")
        .expect("probe gio-2.0");

    let mut build = cc::Build::new();
    build.cargo_metadata(false);
    build.file(&error_source_path);
    build.file(&api_source_path);
    build.file(&wrapper_source_path);
    build.flag_if_supported("-std=c99");
    build.flag_if_supported("-fvisibility=hidden");
    build.warnings(false);
    build.include(manifest_dir.join("include"));
    for include_path in gio.include_paths {
        build.include(include_path);
    }
    build.compile("vips_error_shim");

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static:+whole-archive=vips_error_shim");
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
