use std::ffi::{CStr, CString};

use libc::{c_char, c_int};

unsafe extern "C" {
    #[link_name = "safe_vips_error_append_internal"]
    fn raw_safe_vips_error_append_internal(domain: *const c_char, message: *const c_char);

    pub fn vips_error(domain: *const c_char, fmt: *const c_char, ...);
    pub fn vips_error_system(err: c_int, domain: *const c_char, fmt: *const c_char, ...);
    pub fn vips_error_exit(fmt: *const c_char, ...);

    #[link_name = "vips_error_buffer"]
    fn raw_vips_error_buffer() -> *const c_char;
    #[link_name = "vips_error_buffer_copy"]
    fn raw_vips_error_buffer_copy() -> *mut c_char;
    #[link_name = "vips_error_clear"]
    fn raw_vips_error_clear();
    #[link_name = "vips_error_freeze"]
    fn raw_vips_error_freeze();
    #[link_name = "vips_error_thaw"]
    fn raw_vips_error_thaw();
    #[link_name = "vips_error_g"]
    fn raw_vips_error_g(error: *mut *mut glib_sys::GError);
}

pub(crate) fn append_message(domain: Option<&CStr>, message: &CStr) {
    let domain = domain.map_or(std::ptr::null(), CStr::as_ptr);
    unsafe {
        raw_safe_vips_error_append_internal(domain, message.as_ptr());
    }
}

pub(crate) fn append_message_str(domain: &str, message: &str) {
    let domain = CString::new(domain).expect("domain");
    let message = CString::new(message).expect("message");
    append_message(Some(&domain), &message);
}

pub fn vips_error_buffer() -> *const c_char {
    unsafe { raw_vips_error_buffer() }
}

pub fn vips_error_buffer_copy() -> *mut c_char {
    unsafe { raw_vips_error_buffer_copy() }
}

pub fn vips_error_clear() {
    unsafe {
        raw_vips_error_clear();
    }
}

pub fn vips_error_freeze() {
    unsafe {
        raw_vips_error_freeze();
    }
}

pub fn vips_error_thaw() {
    unsafe {
        raw_vips_error_thaw();
    }
}

pub fn vips_error_g(error: *mut *mut glib_sys::GError) {
    unsafe {
        raw_vips_error_g(error);
    }
}
