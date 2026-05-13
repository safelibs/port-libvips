use std::ptr;

use super::*;

#[test]
fn cve_2026_3284_crop_rejects_coordinate_overflow() {
    let _guard = guard();
    init_vips();
    vips_error_clear();

    let input = image_from_uchar(4, 1, &[1, 2, 3, 4]);
    let mut out = 1usize as *mut VipsImage;
    let result = unsafe {
        vips_crop(
            input,
            &mut out,
            i32::MAX - 4,
            0,
            8,
            1,
            ptr::null::<std::ffi::c_char>(),
        )
    };
    assert_failed_output_cleared(result, out);
    assert!(
        error_message().contains("bad extract area")
            || error_message().contains("operation failed")
    );
    unref_image(input);
}

#[test]
fn cve_2026_3284_extract_area_rejects_cli_range_overflow() {
    let _guard = guard();
    init_vips();
    vips_error_clear();

    let operation = unsafe { vips_operation_new(c"extract_area".as_ptr()) };
    assert!(!operation.is_null());
    let result = unsafe {
        vips_object_set_argument_from_string(
            operation.cast(),
            c"width".as_ptr(),
            c"2147483646".as_ptr(),
        )
    };
    assert_eq!(result, -1);
    assert!(error_message().contains("outside the allowed range"));
    unsafe {
        gobject_sys::g_object_unref(operation.cast());
    }
}
