use libc::{c_char, c_int, c_void};
use std::ffi::{CStr, CString};
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::runtime::error::append_message_str;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(1);

fn split_vips_filename(name: &str) -> (String, String) {
    crate::foreign::base::parse_embedded_options(name)
}

fn write_c_buffer(out: *mut c_char, text: &str) {
    if out.is_null() {
        return;
    }
    if let Ok(text) = CString::new(text) {
        unsafe {
            ptr::copy_nonoverlapping(text.as_ptr(), out, text.as_bytes_with_nul().len());
        }
    } else {
        unsafe {
            *out = 0;
        }
    }
}

fn foreign_name_ptr(name: &str) -> *const c_char {
    match name {
        "jpegload" => c"jpegload".as_ptr(),
        "jpegload_buffer" => c"jpegload_buffer".as_ptr(),
        "jpegload_source" => c"jpegload_source".as_ptr(),
        "pngload" => c"pngload".as_ptr(),
        "pngload_buffer" => c"pngload_buffer".as_ptr(),
        "pngload_source" => c"pngload_source".as_ptr(),
        "gifload" => c"gifload".as_ptr(),
        "gifload_buffer" => c"gifload_buffer".as_ptr(),
        "gifload_source" => c"gifload_source".as_ptr(),
        "tiffload" => c"tiffload".as_ptr(),
        "tiffload_buffer" => c"tiffload_buffer".as_ptr(),
        "tiffload_source" => c"tiffload_source".as_ptr(),
        "vipsload" => c"vipsload".as_ptr(),
        "vipsload_source" => c"vipsload_source".as_ptr(),
        "svgload" => c"svgload".as_ptr(),
        "svgload_buffer" => c"svgload_buffer".as_ptr(),
        "svgload_source" => c"svgload_source".as_ptr(),
        "pdfload" => c"pdfload".as_ptr(),
        "pdfload_buffer" => c"pdfload_buffer".as_ptr(),
        "pdfload_source" => c"pdfload_source".as_ptr(),
        "webpload" => c"webpload".as_ptr(),
        "webpload_buffer" => c"webpload_buffer".as_ptr(),
        "webpload_source" => c"webpload_source".as_ptr(),
        "heifload" => c"heifload".as_ptr(),
        "heifload_buffer" => c"heifload_buffer".as_ptr(),
        "heifload_source" => c"heifload_source".as_ptr(),
        "ppmload" => c"ppmload".as_ptr(),
        "ppmload_buffer" => c"ppmload_buffer".as_ptr(),
        "ppmload_source" => c"ppmload_source".as_ptr(),
        "radload" => c"radload".as_ptr(),
        "radload_buffer" => c"radload_buffer".as_ptr(),
        "radload_source" => c"radload_source".as_ptr(),
        "csvload" => c"csvload".as_ptr(),
        "csvload_source" => c"csvload_source".as_ptr(),
        "matrixload" => c"matrixload".as_ptr(),
        "matrixload_source" => c"matrixload_source".as_ptr(),
        "jpegsave" => c"jpegsave".as_ptr(),
        "pngsave" => c"pngsave".as_ptr(),
        "tiffsave" => c"tiffsave".as_ptr(),
        "webpsave" => c"webpsave".as_ptr(),
        "heifsave" => c"heifsave".as_ptr(),
        "avifsave" => c"avifsave".as_ptr(),
        "vipssave" => c"vipssave".as_ptr(),
        "ppmsave" => c"ppmsave".as_ptr(),
        "csvsave" => c"csvsave".as_ptr(),
        "matrixsave" => c"matrixsave".as_ptr(),
        "radsave" => c"radsave".as_ptr(),
        "jpegsave_buffer" => c"jpegsave_buffer".as_ptr(),
        "pngsave_buffer" => c"pngsave_buffer".as_ptr(),
        "tiffsave_buffer" => c"tiffsave_buffer".as_ptr(),
        "webpsave_buffer" => c"webpsave_buffer".as_ptr(),
        "heifsave_buffer" => c"heifsave_buffer".as_ptr(),
        "radsave_buffer" => c"radsave_buffer".as_ptr(),
        "pngsave_target" => c"pngsave_target".as_ptr(),
        "jpegsave_target" => c"jpegsave_target".as_ptr(),
        "tiffsave_target" => c"tiffsave_target".as_ptr(),
        "webpsave_target" => c"webpsave_target".as_ptr(),
        "heifsave_target" => c"heifsave_target".as_ptr(),
        "avifsave_target" => c"avifsave_target".as_ptr(),
        "vipssave_target" => c"vipssave_target".as_ptr(),
        "ppmsave_target" => c"ppmsave_target".as_ptr(),
        "csvsave_target" => c"csvsave_target".as_ptr(),
        "matrixsave_target" => c"matrixsave_target".as_ptr(),
        "radsave_target" => c"radsave_target".as_ptr(),
        _ => ptr::null(),
    }
}

fn replace_temp_pattern(pattern: &str, base: &str) -> String {
    if let Some(index) = pattern.find('%') {
        if let Some(end) = pattern[index + 1..].find('s') {
            let end = index + 1 + end;
            let mut out = String::with_capacity(pattern.len() + base.len());
            out.push_str(&pattern[..index]);
            out.push_str(base);
            out.push_str(&pattern[end + 1..]);
            return out;
        }
    }
    pattern.to_owned()
}

#[no_mangle]
pub extern "C" fn vips_file_length(fd: c_int) -> i64 {
    if fd < 0 {
        return -1;
    }

    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    let result = unsafe { libc::fstat(fd, stat.as_mut_ptr()) };
    if result != 0 {
        return -1;
    }
    unsafe { stat.assume_init().st_size as i64 }
}

#[no_mangle]
pub extern "C" fn vips__write(fd: c_int, buf: *const c_void, count: usize) -> c_int {
    if fd < 0 || buf.is_null() {
        return -1;
    }
    let mut written = 0usize;
    while written < count {
        let chunk = unsafe {
            libc::write(
                fd,
                buf.cast::<u8>().add(written).cast::<c_void>(),
                count - written,
            )
        };
        if chunk < 0 {
            append_message_str("vips__write", "write failed");
            return -1;
        }
        if chunk == 0 {
            append_message_str("vips__write", "short write");
            return -1;
        }
        written += chunk as usize;
    }
    0
}

#[no_mangle]
pub extern "C" fn vips__open(filename: *const c_char, flags: c_int, mode: c_int) -> c_int {
    if filename.is_null() {
        append_message_str("vips__open", "filename is null");
        return -1;
    }
    unsafe { libc::open(filename, flags, mode) }
}

#[no_mangle]
pub extern "C" fn vips__open_read(filename: *const c_char) -> c_int {
    vips__open(filename, libc::O_RDONLY, 0)
}

#[no_mangle]
pub extern "C" fn vips__seek_no_error(fd: c_int, pos: i64, whence: c_int) -> i64 {
    if fd < 0 {
        return -1;
    }
    unsafe { libc::lseek(fd, pos as libc::off_t, whence) as i64 }
}

#[no_mangle]
pub extern "C" fn vips__seek(fd: c_int, pos: i64, whence: c_int) -> i64 {
    let result = vips__seek_no_error(fd, pos, whence);
    if result < 0 {
        append_message_str("vips__seek", "seek failed");
    }
    result
}

#[no_mangle]
pub extern "C" fn vips__filename_split8(
    name: *const c_char,
    filename: *mut c_char,
    option_string: *mut c_char,
) {
    let Some(name) = (!name.is_null()).then(|| unsafe { CStr::from_ptr(name) }) else {
        write_c_buffer(filename, "");
        write_c_buffer(option_string, "");
        return;
    };
    let (filename_text, options) = split_vips_filename(&name.to_string_lossy());
    write_c_buffer(filename, &filename_text);
    if options.is_empty() {
        write_c_buffer(option_string, "");
    } else {
        write_c_buffer(option_string, &format!("[{options}]"));
    }
}

#[no_mangle]
pub extern "C" fn vips_filename_get_filename(vips_filename: *const c_char) -> *mut c_char {
    let Some(name) = (!vips_filename.is_null()).then(|| unsafe { CStr::from_ptr(vips_filename) })
    else {
        return ptr::null_mut();
    };
    let (filename, _) = split_vips_filename(&name.to_string_lossy());
    let Ok(filename) = CString::new(filename) else {
        return ptr::null_mut();
    };
    unsafe { glib_sys::g_strdup(filename.as_ptr()) }
}

#[no_mangle]
pub extern "C" fn vips_filename_get_options(vips_filename: *const c_char) -> *mut c_char {
    let Some(name) = (!vips_filename.is_null()).then(|| unsafe { CStr::from_ptr(vips_filename) })
    else {
        return ptr::null_mut();
    };
    let (_, options) = split_vips_filename(&name.to_string_lossy());
    let text = if options.is_empty() {
        String::new()
    } else {
        format!("[{options}]")
    };
    let Ok(text) = CString::new(text) else {
        return ptr::null_mut();
    };
    unsafe { glib_sys::g_strdup(text.as_ptr()) }
}

#[no_mangle]
pub extern "C" fn vips__temp_name(format: *const c_char) -> *mut c_char {
    let format = if format.is_null() {
        "%s.v".to_owned()
    } else {
        unsafe { CStr::from_ptr(format) }
            .to_string_lossy()
            .into_owned()
    };
    let base = format!(
        "vips-{}-{}",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    let filename = replace_temp_pattern(&format, &base);
    let path = std::env::temp_dir().join(filename);
    let Ok(path) = CString::new(path.to_string_lossy().as_ref()) else {
        return ptr::null_mut();
    };
    unsafe { glib_sys::g_strdup(path.as_ptr()) }
}

#[no_mangle]
pub extern "C" fn vips_foreign_find_load(name: *const c_char) -> *const c_char {
    let Some(name) = (!name.is_null()).then(|| unsafe { CStr::from_ptr(name) }) else {
        return ptr::null();
    };
    crate::foreign::foreign_find_load_name(name)
        .map(foreign_name_ptr)
        .unwrap_or(ptr::null())
}

#[no_mangle]
pub extern "C" fn vips_foreign_find_load_buffer(data: *const c_void, size: usize) -> *const c_char {
    if data.is_null() && size != 0 {
        return ptr::null();
    }
    let bytes = if size == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(data.cast::<u8>(), size) }
    };
    crate::foreign::foreign_find_load_buffer_name(bytes)
        .map(foreign_name_ptr)
        .unwrap_or(ptr::null())
}

#[no_mangle]
pub extern "C" fn vips_foreign_find_load_source(
    source: *mut crate::abi::connection::VipsSource,
) -> *const c_char {
    crate::foreign::foreign_find_load_source_name(source)
        .map(foreign_name_ptr)
        .unwrap_or(ptr::null())
}

#[no_mangle]
pub extern "C" fn vips_foreign_find_save(name: *const c_char) -> *const c_char {
    let Some(name) = (!name.is_null()).then(|| unsafe { CStr::from_ptr(name) }) else {
        return ptr::null();
    };
    crate::foreign::foreign_find_save_name(name)
        .map(foreign_name_ptr)
        .unwrap_or(ptr::null())
}

#[no_mangle]
pub extern "C" fn vips_foreign_find_save_buffer(suffix: *const c_char) -> *const c_char {
    let Some(suffix) = (!suffix.is_null()).then(|| unsafe { CStr::from_ptr(suffix) }) else {
        return ptr::null();
    };
    crate::foreign::foreign_find_save_buffer_name(suffix)
        .map(foreign_name_ptr)
        .unwrap_or(ptr::null())
}

#[no_mangle]
pub extern "C" fn vips_foreign_find_save_target(suffix: *const c_char) -> *const c_char {
    let Some(suffix) = (!suffix.is_null()).then(|| unsafe { CStr::from_ptr(suffix) }) else {
        return ptr::null();
    };
    crate::foreign::foreign_find_save_target_name(suffix)
        .map(foreign_name_ptr)
        .unwrap_or(ptr::null())
}

pub(crate) fn read_all_from_path(path: &CStr) -> Result<Vec<u8>, ()> {
    match std::fs::read(path.to_string_lossy().as_ref()) {
        Ok(bytes) => Ok(bytes),
        Err(err) => {
            append_message_str("vips_source_new_from_file", &err.to_string());
            Err(())
        }
    }
}

pub(crate) fn alloc_copy(bytes: &[u8]) -> *mut c_void {
    if bytes.is_empty() {
        return ptr::null_mut();
    }
    let copy = unsafe { glib_sys::g_malloc(bytes.len()) };
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), copy.cast::<u8>(), bytes.len());
    }
    copy
}

#[no_mangle]
pub extern "C" fn vips_isprefix(a: *const c_char, b: *const c_char) -> glib_sys::gboolean {
    if a.is_null() || b.is_null() {
        return glib_sys::GFALSE;
    }
    let a = unsafe { CStr::from_ptr(a) }.to_bytes();
    let b = unsafe { CStr::from_ptr(b) }.to_bytes();
    if b.starts_with(a) {
        glib_sys::GTRUE
    } else {
        glib_sys::GFALSE
    }
}

#[no_mangle]
pub extern "C" fn vips_add_option_entries(_option_group: *mut glib_sys::GOptionGroup) {}

static VECTOR_NONE: &[u8] = b"none\0";
static VECTOR_SCALAR: &[u8] = b"scalar\0";
static VECTOR_SIMD128: &[u8] = b"simd128\0";
static VECTOR_SIMD256: &[u8] = b"simd256\0";

#[no_mangle]
pub extern "C" fn vips_vector_isenabled() -> glib_sys::gboolean {
    if crate::simd::vector::is_enabled() {
        glib_sys::GTRUE
    } else {
        glib_sys::GFALSE
    }
}

#[no_mangle]
pub extern "C" fn vips_vector_set_enabled(enabled: glib_sys::gboolean) {
    crate::simd::vector::set_enabled(enabled != glib_sys::GFALSE);
}

#[no_mangle]
pub extern "C" fn vips_vector_get_builtin_targets() -> i64 {
    crate::simd::vector::builtin_targets()
}

#[no_mangle]
pub extern "C" fn vips_vector_get_supported_targets() -> i64 {
    crate::simd::vector::supported_targets()
}

#[no_mangle]
pub extern "C" fn vips_vector_target_name(target: i64) -> *const c_char {
    match crate::simd::vector::target_name(target) {
        Some("scalar") => VECTOR_SCALAR.as_ptr().cast(),
        Some("simd128") => VECTOR_SIMD128.as_ptr().cast(),
        Some("simd256") => VECTOR_SIMD256.as_ptr().cast(),
        _ => VECTOR_NONE.as_ptr().cast(),
    }
}

#[no_mangle]
pub extern "C" fn vips_vector_disable_targets(disabled_targets: i64) {
    crate::simd::vector::disable_targets(disabled_targets);
}
