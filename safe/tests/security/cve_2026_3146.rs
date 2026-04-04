use std::ffi::{c_char, c_void};
use std::ptr;

use super::*;

unsafe extern "C" {
    fn vips_matrixload_source(source: *mut VipsSource, out: *mut *mut VipsImage, ...) -> i32;
}

#[test]
fn cve_2026_3146_failed_matrix_parse_clears_output_image() {
    let _guard = guard();
    init_vips();
    vips_error_clear();

    let payload = b"2 2 1 0\n1 2 3\n";
    let source =
        unsafe { vips_source_new_from_memory(payload.as_ptr().cast::<c_void>(), payload.len()) };
    assert!(!source.is_null());

    let mut out = 1usize as *mut VipsImage;
    assert_eq!(
        unsafe { vips_matrixload_source(source, &mut out, ptr::null::<c_char>()) },
        -1
    );
    assert!(out.is_null());

    let error = error_message();
    assert!(error.contains("matrix payload length mismatch"));

    unsafe {
        gobject_sys::g_object_unref(source.cast());
    }
}
