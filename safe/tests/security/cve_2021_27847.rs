use std::ptr;

use super::*;

#[test]
fn cve_2021_27847_eye_handles_degenerate_dimensions() {
    let _guard = guard();
    init_vips();

    let mut eye = ptr::null_mut();
    let result = unsafe { vips_eye(&mut eye, 0, 0, ptr::null::<std::ffi::c_char>()) };
    assert!(result == 0 || result == -1);
    if result == 0 {
        let values = read_samples(eye);
        assert!(values.iter().all(|value| value.is_finite()));
        assert!(vips_image_get_width(eye) >= 1);
        assert!(vips_image_get_height(eye) >= 1);
        unref_image(eye);
    }
}

#[test]
fn cve_2021_27847_mask_handles_degenerate_dimensions() {
    let _guard = guard();
    init_vips();

    let mut mask = ptr::null_mut();
    let result = unsafe { vips_mask_ideal(&mut mask, 0, 0, 0.5, ptr::null::<std::ffi::c_char>()) };
    assert!(result == 0 || result == -1);
    if result == 0 {
        let values = read_samples(mask);
        assert!(values.iter().all(|value| value.is_finite()));
        assert!(vips_image_get_width(mask) >= 1);
        assert!(vips_image_get_height(mask) >= 1);
        unref_image(mask);
    }
}
