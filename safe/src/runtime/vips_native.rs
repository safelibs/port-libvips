use std::ffi::CStr;
use std::ptr;
use std::sync::{Mutex, OnceLock};

use libc::{c_char, c_int, c_void};

use crate::runtime::error::append_message_str;

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
    unsafe { libc::write(fd, buf, count) as c_int }
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

fn vector_state() -> &'static Mutex<bool> {
    static ENABLED: OnceLock<Mutex<bool>> = OnceLock::new();
    ENABLED.get_or_init(|| Mutex::new(true))
}

static VECTOR_NONE: &[u8] = b"none\0";

#[no_mangle]
pub extern "C" fn vips_vector_isenabled() -> glib_sys::gboolean {
    if *vector_state().lock().expect("vector state") {
        glib_sys::GTRUE
    } else {
        glib_sys::GFALSE
    }
}

#[no_mangle]
pub extern "C" fn vips_vector_set_enabled(enabled: glib_sys::gboolean) {
    *vector_state().lock().expect("vector state") = enabled != glib_sys::GFALSE;
}

#[no_mangle]
pub extern "C" fn vips_vector_get_builtin_targets() -> i64 {
    0
}

#[no_mangle]
pub extern "C" fn vips_vector_get_supported_targets() -> i64 {
    0
}

#[no_mangle]
pub extern "C" fn vips_vector_target_name(_target: i64) -> *const c_char {
    VECTOR_NONE.as_ptr().cast()
}

#[no_mangle]
pub extern "C" fn vips_vector_disable_targets(_disabled_targets: i64) {}
