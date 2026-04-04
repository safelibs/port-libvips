use std::ffi::{c_char, c_void, CStr};
use std::ptr;

use super::*;

unsafe extern "C" {
    fn vips_image_new_from_buffer(
        buf: *const c_void,
        len: usize,
        option_string: *const c_char,
        ...
    ) -> *mut VipsImage;
}

fn loader_name(bytes: &[u8]) -> Option<String> {
    let name =
        unsafe { vips_foreign_find_load_buffer(bytes.as_ptr().cast::<c_void>(), bytes.len()) };
    if name.is_null() {
        None
    } else {
        Some(
            unsafe { CStr::from_ptr(name) }
                .to_string_lossy()
                .into_owned(),
        )
    }
}

#[test]
fn cve_2023_40032_svgload_rejects_malformed_utf8_prefix() {
    let _guard = guard();
    init_vips();
    vips_error_clear();

    let payload = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\xff<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 8 8\"></svg>";
    assert_eq!(loader_name(payload).as_deref(), Some("svgload_buffer"));

    let image = unsafe {
        vips_image_new_from_buffer(
            payload.as_ptr().cast::<c_void>(),
            payload.len(),
            c"".as_ptr(),
            ptr::null::<c_char>(),
        )
    };
    assert!(image.is_null());

    let error = error_message();
    assert!(!error.is_empty());
}

#[test]
fn cve_2023_40032_svgload_rejects_truncated_document() {
    let _guard = guard();
    init_vips();
    vips_error_clear();

    let payload = b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"32\" height=\"32\"><rect";
    assert_eq!(loader_name(payload).as_deref(), Some("svgload_buffer"));

    let image = unsafe {
        vips_image_new_from_buffer(
            payload.as_ptr().cast::<c_void>(),
            payload.len(),
            c"".as_ptr(),
            ptr::null::<c_char>(),
        )
    };
    assert!(image.is_null());

    let error = error_message();
    assert!(!error.is_empty());
}
