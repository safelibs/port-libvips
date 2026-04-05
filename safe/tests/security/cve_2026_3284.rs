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
