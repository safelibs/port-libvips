use std::ptr;

use super::*;

#[test]
fn cve_2026_3284_crop_rejects_coordinate_overflow() {
    let _guard = guard();
    init_vips();

    let input = image_from_uchar(4, 1, &[1, 2, 3, 4]);
    let mut out = ptr::null_mut();
    assert_eq!(
        unsafe {
            vips_crop(
                input,
                &mut out,
                i32::MAX - 4,
                0,
                8,
                1,
                ptr::null::<std::ffi::c_char>(),
            )
        },
        -1
    );
    unref_image(input);
}
