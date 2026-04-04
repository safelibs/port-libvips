use std::ffi::CStr;
use std::sync::atomic::{AtomicI64, Ordering};

use crate::abi::connection::VipsConnection;

static PIPE_READ_LIMIT: AtomicI64 = AtomicI64::new(1024 * 1024 * 1024);

#[no_mangle]
pub extern "C" fn vips_connection_filename(connection: *mut VipsConnection) -> *const libc::c_char {
    unsafe { connection.as_ref() }.map_or(std::ptr::null(), |connection| {
        connection.filename.cast_const()
    })
}

#[no_mangle]
pub extern "C" fn vips_connection_nick(connection: *mut VipsConnection) -> *const libc::c_char {
    let Some(connection) = (unsafe { connection.as_ref() }) else {
        return std::ptr::null();
    };

    if !connection.filename.is_null() {
        return connection.filename.cast_const();
    }

    if !connection.parent_object.nickname.is_null() {
        return connection.parent_object.nickname.cast_const();
    }

    c"connection".as_ptr()
}

#[no_mangle]
pub extern "C" fn vips_pipe_read_limit_set(limit: i64) {
    PIPE_READ_LIMIT.store(limit, Ordering::Relaxed);
}

pub(crate) fn pipe_read_limit() -> i64 {
    PIPE_READ_LIMIT.load(Ordering::Relaxed)
}

pub(crate) fn optional_cstr(ptr: *const libc::c_char) -> Option<&'static CStr> {
    if ptr.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(ptr) })
    }
}
