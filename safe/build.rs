use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const SONAME: &str = "libvips.so.42";
const GENERATED_DIR: &str = "generated";
const EXPORT_MAP_NAME: &str = "export.map";
const CORE_BOOTSTRAP_PATH: &str = "reference/abi/core-bootstrap.symbols";
const ERROR_SHIM_NAME: &str = "error_shim.c";
const API_SHIM_NAME: &str = "api_shim.c";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={CORE_BOOTSTRAP_PATH}");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let generated_dir = manifest_dir.join(GENERATED_DIR);
    fs::create_dir_all(&generated_dir).expect("create generated dir");

    let export_map_path = generated_dir.join(EXPORT_MAP_NAME);
    let export_map = render_export_map(&manifest_dir.join(CORE_BOOTSTRAP_PATH));
    fs::write(&export_map_path, export_map).expect("write export map");

    compile_c_shims(&manifest_dir);

    println!("cargo:rustc-cdylib-link-arg=-Wl,-soname,{SONAME}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,--version-script={}",
        export_map_path.display()
    );
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,--retain-symbols-file={}",
        manifest_dir.join(CORE_BOOTSTRAP_PATH).display()
    );
    println!("cargo:rustc-cdylib-link-arg=-Wl,--no-undefined");
}

fn render_export_map(symbols_path: &Path) -> String {
    let mut lines = vec!["VIPS_42 {".to_owned()];

    let symbols = read_symbols(symbols_path);
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

fn compile_c_shims(manifest_dir: &Path) {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let error_source_path = out_dir.join(ERROR_SHIM_NAME);
    let api_source_path = out_dir.join(API_SHIM_NAME);
    fs::write(&error_source_path, render_error_shim()).expect("write error shim");
    fs::write(&api_source_path, render_api_shim()).expect("write api shim");

    let gio = pkg_config::Config::new()
        .cargo_metadata(false)
        .probe("gio-2.0")
        .expect("probe gio-2.0");

    let mut build = cc::Build::new();
    build.cargo_metadata(false);
    build.file(&error_source_path);
    build.file(&api_source_path);
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
    r#"#include <glib.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>
#include <vips/vips.h>

#if defined(__GNUC__)
#define VIPS_PUBLIC __attribute__((visibility("default")))
#else
#define VIPS_PUBLIC
#endif

extern VipsImage *safe_vips_image_new_from_source_internal(
    VipsSource *source, const char *option_string, int access);
extern int safe_vips_image_write_to_target_internal(
    VipsImage *image, const char *suffix, VipsTarget *target);
extern int safe_vips_crop_internal(
    VipsImage *in, VipsImage **out, int left, int top, int width, int height);
extern int safe_vips_avg_internal(VipsImage *image, double *out);

VIPS_PUBLIC VipsImage *
vips_image_new_from_source(VipsSource *source, const char *option_string, ...)
{
    va_list ap;
    const char *name;
    int access = VIPS_ACCESS_RANDOM;

    va_start(ap, option_string);
    while ((name = va_arg(ap, const char *))) {
        if (strcmp(name, "access") == 0)
            access = va_arg(ap, int);
        else
            (void) va_arg(ap, void *);
    }
    va_end(ap);

    return safe_vips_image_new_from_source_internal(source, option_string, access);
}

VIPS_PUBLIC int
vips_image_write_to_target(VipsImage *in, const char *suffix, VipsTarget *target, ...)
{
    return safe_vips_image_write_to_target_internal(in, suffix, target);
}

VIPS_PUBLIC int
vips_crop(VipsImage *in, VipsImage **out, int left, int top, int width, int height, ...)
{
    return safe_vips_crop_internal(in, out, left, top, width, height);
}

VIPS_PUBLIC int
vips_avg(VipsImage *in, double *out, ...)
{
    return safe_vips_avg_internal(in, out);
}

VIPS_PUBLIC gboolean
vips_buf_vappendf(VipsBuf *buf, const char *fmt, va_list ap)
{
    char *line;
    gboolean ok;

    line = g_strdup_vprintf(fmt, ap);
    ok = vips_buf_appends(buf, line);
    g_free(line);

    return ok;
}

VIPS_PUBLIC gboolean
vips_buf_appendf(VipsBuf *buf, const char *fmt, ...)
{
    va_list ap;
    gboolean ok;

    va_start(ap, fmt);
    ok = vips_buf_vappendf(buf, fmt, ap);
    va_end(ap);

    return ok;
}

VIPS_PUBLIC gboolean
vips_dbuf_writef(VipsDbuf *dbuf, const char *fmt, ...)
{
    va_list ap;
    char *line;
    gboolean ok;

    va_start(ap, fmt);
    line = g_strdup_vprintf(fmt, ap);
    va_end(ap);

    ok = vips_dbuf_write(dbuf, (const unsigned char *) line, strlen(line));
    g_free(line);

    return ok;
}

VIPS_PUBLIC int
vips_image_pipelinev(VipsImage *image, VipsDemandStyle hint, ...)
{
    va_list ap;
    VipsImage *value;
    VipsImage **inputs;
    int n = 0;
    int i;
    int result;

    va_start(ap, hint);
    while ((value = va_arg(ap, VipsImage *)))
        n += 1;
    va_end(ap);

    inputs = g_new0(VipsImage *, n + 1);
    va_start(ap, hint);
    for (i = 0; i < n; i++)
        inputs[i] = va_arg(ap, VipsImage *);
    va_end(ap);

    result = vips_image_pipeline_array(image, hint, inputs);
    g_free(inputs);

    return result;
}
"#
}
