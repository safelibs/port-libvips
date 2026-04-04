use std::ffi::{c_char, c_void};
use std::ptr;

use super::*;

unsafe extern "C" {
    fn vips_matrixload_source(source: *mut VipsSource, out: *mut *mut VipsImage, ...) -> i32;
}

#[test]
fn cve_2019_6976_matrix_header_requires_scale_and_offset() {
    let _guard = guard();
    init_vips();
    vips_error_clear();

    let payload = b"3 2 1\n1 2 3\n4 5 6\n";
    let source =
        unsafe { vips_source_new_from_memory(payload.as_ptr().cast::<c_void>(), payload.len()) };
    assert!(!source.is_null());

    let mut out = ptr::null_mut();
    assert_eq!(
        unsafe { vips_matrixload_source(source, &mut out, ptr::null::<c_char>()) },
        -1
    );
    assert!(out.is_null());

    let error = error_message();
    assert!(error.contains("matrix header requires width height scale offset"));

    unsafe {
        gobject_sys::g_object_unref(source.cast());
    }
}
