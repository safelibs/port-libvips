use std::ffi::c_char;
use std::ffi::CStr;
use std::ptr;
use std::slice;
use std::sync::{Mutex, Once, OnceLock};

use vips::*;

unsafe extern "C" {
    fn vips_black(out: *mut *mut VipsImage, width: i32, height: i32, ...) -> i32;
    fn vips_HSV2sRGB(input: *mut VipsImage, out: *mut *mut VipsImage, ...) -> i32;
    fn vips_colourspace(
        input: *mut VipsImage,
        out: *mut *mut VipsImage,
        space: VipsInterpretation,
        ...
    ) -> i32;
    fn vips_draw_image(image: *mut VipsImage, sub: *mut VipsImage, x: i32, y: i32, ...) -> i32;
    fn vips_match(
        reference: *mut VipsImage,
        secondary: *mut VipsImage,
        out: *mut *mut VipsImage,
        xr1: i32,
        yr1: i32,
        xs1: i32,
        ys1: i32,
        xr2: i32,
        yr2: i32,
        xs2: i32,
        ys2: i32,
        ...
    ) -> i32;
    fn vips_mosaic(
        reference: *mut VipsImage,
        secondary: *mut VipsImage,
        out: *mut *mut VipsImage,
        direction: VipsDirection,
        xref: i32,
        yref: i32,
        xsec: i32,
        ysec: i32,
        ...
    ) -> i32;
    fn vips_profile(
        input: *mut VipsImage,
        columns: *mut *mut VipsImage,
        rows: *mut *mut VipsImage,
        ...
    ) -> i32;
    fn vips_profile_load(name: *const c_char, profile: *mut *mut VipsBlob, ...) -> i32;
    fn vips_reduce(
        input: *mut VipsImage,
        out: *mut *mut VipsImage,
        hshrink: f64,
        vshrink: f64,
        ...
    ) -> i32;
    fn vips_resize(input: *mut VipsImage, out: *mut *mut VipsImage, scale: f64, ...) -> i32;
    fn vips_sRGB2HSV(input: *mut VipsImage, out: *mut *mut VipsImage, ...) -> i32;
    fn vips_thumbnail_image(
        input: *mut VipsImage,
        out: *mut *mut VipsImage,
        width: i32,
        ...
    ) -> i32;
    fn vips_vector_disable_targets(disabled_targets: i64);
    fn vips_vector_get_supported_targets() -> i64;
    fn vips_vector_target_name(target: i64) -> *const c_char;
}

fn guard() -> std::sync::MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    match GUARD.get_or_init(|| Mutex::new(())).lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn init_vips() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        assert_eq!(vips_init(c"ops_advanced".as_ptr()), 0);
    });
}

fn image_from_uchar(width: i32, height: i32, bands: i32, bytes: &[u8]) -> *mut VipsImage {
    vips_image_new_from_memory_copy(
        bytes.as_ptr().cast(),
        bytes.len(),
        width,
        height,
        bands,
        VIPS_FORMAT_UCHAR,
    )
}

fn read_u8(image: *mut VipsImage) -> Vec<u8> {
    let mut len = 0usize;
    let ptr = vips_image_write_to_memory(image, &mut len);
    let bytes = unsafe { slice::from_raw_parts(ptr.cast::<u8>(), len) }.to_vec();
    unsafe {
        glib_sys::g_free(ptr);
    }
    bytes
}

fn unref_image(image: *mut VipsImage) {
    unsafe {
        gobject_sys::g_object_unref(image.cast());
    }
}

fn error_text() -> String {
    unsafe { CStr::from_ptr(vips_error_buffer()) }
        .to_string_lossy()
        .into_owned()
}

#[test]
fn colour_and_profile_flow() {
    let _guard = guard();
    init_vips();

    let input = image_from_uchar(2, 1, 3, &[255, 0, 0, 0, 255, 0]);
    let mut hsv = ptr::null_mut();
    assert_eq!(
        unsafe { vips_sRGB2HSV(input, &mut hsv, ptr::null::<c_char>()) },
        0
    );
    assert_eq!(vips_image_get_format(hsv), VIPS_FORMAT_UCHAR);

    let mut srgb = ptr::null_mut();
    assert_eq!(
        unsafe { vips_HSV2sRGB(hsv, &mut srgb, ptr::null::<c_char>()) },
        0
    );
    let roundtrip = read_u8(srgb);
    assert!((roundtrip[0] as i32 - 255).abs() <= 2);
    assert!(roundtrip[1] <= 2);
    assert!(roundtrip[3] <= 2);
    assert!((roundtrip[4] as i32 - 255).abs() <= 2);

    let mut lab = ptr::null_mut();
    assert_eq!(
        unsafe {
            vips_colourspace(
                input,
                &mut lab,
                VIPS_INTERPRETATION_LAB,
                ptr::null::<c_char>(),
            )
        },
        0
    );
    assert_eq!(vips_image_get_format(lab), VIPS_FORMAT_FLOAT);

    let mut columns = ptr::null_mut();
    let mut rows = ptr::null_mut();
    assert_eq!(
        unsafe { vips_profile(input, &mut columns, &mut rows, ptr::null::<c_char>()) },
        0
    );
    assert_eq!(vips_image_get_width(columns), 2);
    assert_eq!(vips_image_get_height(columns), 1);
    assert_eq!(vips_image_get_width(rows), 1);
    assert_eq!(vips_image_get_height(rows), 1);

    let mut profile = ptr::null_mut();
    assert_eq!(
        unsafe { vips_profile_load(c"srgb".as_ptr(), &mut profile, ptr::null::<c_char>()) },
        0
    );
    let mut len = 0usize;
    let data = vips_blob_get(profile, &mut len);
    assert!(!data.is_null());
    assert!(len > 0);

    let mut none_profile = ptr::null_mut();
    assert_eq!(
        unsafe { vips_profile_load(c"none".as_ptr(), &mut none_profile, ptr::null::<c_char>()) },
        0
    );
    assert!(none_profile.is_null());

    unref_image(rows);
    unref_image(columns);
    unref_image(lab);
    unref_image(srgb);
    unref_image(hsv);
    unref_image(input);
}

#[test]
fn resample_and_thumbnail_flow() {
    let _guard = guard();
    init_vips();

    let input = image_from_uchar(
        4,
        4,
        1,
        &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
    );

    let mut resized = ptr::null_mut();
    let resized_result = unsafe { vips_resize(input, &mut resized, 0.5, ptr::null::<c_char>()) };
    assert_eq!(resized_result, 0, "{}", error_text());
    assert_eq!(vips_image_get_width(resized), 2);
    assert_eq!(vips_image_get_height(resized), 2);

    let mut reduced = ptr::null_mut();
    let reduced_result =
        unsafe { vips_reduce(input, &mut reduced, 2.0, 2.0, ptr::null::<c_char>()) };
    assert_eq!(reduced_result, 0, "{}", error_text());
    assert_eq!(vips_image_get_width(reduced), 2);
    assert_eq!(vips_image_get_height(reduced), 2);

    let line = image_from_uchar(6, 1, 1, &[0, 20, 80, 140, 200, 255]);
    let mut centred = ptr::null_mut();
    let centred_result = unsafe {
        vips_reduce(
            line,
            &mut centred,
            3.0,
            1.0,
            c"kernel".as_ptr(),
            VIPS_KERNEL_LINEAR,
            c"centre".as_ptr(),
            1i32,
            ptr::null::<c_char>(),
        )
    };
    assert_eq!(centred_result, 0, "{}", error_text());
    let mut corner = ptr::null_mut();
    let corner_result = unsafe {
        vips_reduce(
            line,
            &mut corner,
            3.0,
            1.0,
            c"kernel".as_ptr(),
            VIPS_KERNEL_LINEAR,
            c"centre".as_ptr(),
            0i32,
            ptr::null::<c_char>(),
        )
    };
    assert_eq!(corner_result, 0, "{}", error_text());
    let centred_values = read_u8(centred);
    let corner_values = read_u8(corner);
    assert_ne!(centred_values, corner_values);
    assert!(centred_values[0] > corner_values[0]);
    assert!(centred_values[1] > corner_values[1]);

    let constant = image_from_uchar(16, 16, 1, &[42; 16 * 16]);
    let mut reduced_gap = ptr::null_mut();
    let reduced_gap_result = unsafe {
        vips_reduce(
            constant,
            &mut reduced_gap,
            4.0,
            4.0,
            c"gap".as_ptr(),
            2.0f64,
            ptr::null::<c_char>(),
        )
    };
    assert_eq!(reduced_gap_result, 0, "{}", error_text());
    assert!(read_u8(reduced_gap).iter().all(|value| *value == 42));

    let mut large = ptr::null_mut();
    assert_eq!(
        unsafe { vips_black(&mut large, 1600, 1000, ptr::null::<c_char>()) },
        0
    );
    let mut geometry = ptr::null_mut();
    let geometry_result =
        unsafe { vips_resize(large, &mut geometry, 10.0 / 1600.0, ptr::null::<c_char>()) };
    assert_eq!(geometry_result, 0, "{}", error_text());
    assert_eq!(vips_image_get_width(geometry), 10);
    assert_eq!(vips_image_get_height(geometry), 6);

    let supported_targets = unsafe { vips_vector_get_supported_targets() };
    let target_name = unsafe { CStr::from_ptr(vips_vector_target_name(0)) }
        .to_string_lossy()
        .into_owned();
    assert_eq!(target_name, "none");
    if supported_targets != 0 {
        let lowest_target = supported_targets & supported_targets.wrapping_neg();
        let name = unsafe { CStr::from_ptr(vips_vector_target_name(lowest_target)) }
            .to_string_lossy()
            .into_owned();
        assert_ne!(name, "none");
        unsafe {
            vips_vector_disable_targets(supported_targets);
        }
        assert_eq!(unsafe { vips_vector_get_supported_targets() }, 0);
        unsafe {
            vips_vector_disable_targets(0);
        }
    }

    let mut thumb = ptr::null_mut();
    let thumb_result = unsafe { vips_thumbnail_image(input, &mut thumb, 2, ptr::null::<c_char>()) };
    assert_eq!(thumb_result, 0, "{}", error_text());
    assert_eq!(vips_image_get_width(thumb), 2);
    assert!(vips_image_get_height(thumb) >= 1);

    unref_image(geometry);
    unref_image(large);
    unref_image(reduced_gap);
    unref_image(constant);
    unref_image(corner);
    unref_image(centred);
    unref_image(line);
    unref_image(thumb);
    unref_image(reduced);
    unref_image(resized);
    unref_image(input);
}

#[test]
fn draw_invalidation_and_mosaic_flow() {
    let _guard = guard();
    init_vips();

    let image = image_from_uchar(4, 4, 1, &[0; 16]);
    let stamp = image_from_uchar(1, 1, 1, &[100]);
    assert_eq!(
        unsafe { vips_draw_image(image, stamp, 0, 0, ptr::null::<c_char>()) },
        0
    );
    let drawn = read_u8(image);
    assert_eq!(drawn[0], 100);

    let reference = image_from_uchar(8, 1, 1, &[10; 8]);
    let secondary = image_from_uchar(8, 1, 1, &[200; 8]);

    let mut wide_blend = ptr::null_mut();
    assert_eq!(
        unsafe {
            vips_mosaic(
                reference,
                secondary,
                &mut wide_blend,
                VIPS_DIRECTION_HORIZONTAL,
                2,
                0,
                0,
                0,
                c"mblend".as_ptr(),
                20i32,
                ptr::null::<c_char>(),
            )
        },
        0
    );
    let mut narrow_blend = ptr::null_mut();
    assert_eq!(
        unsafe {
            vips_mosaic(
                reference,
                secondary,
                &mut narrow_blend,
                VIPS_DIRECTION_HORIZONTAL,
                2,
                0,
                0,
                0,
                c"mblend".as_ptr(),
                2i32,
                ptr::null::<c_char>(),
            )
        },
        0
    );
    assert_eq!(vips_image_get_width(wide_blend), 10);
    assert_eq!(vips_image_get_width(narrow_blend), 10);
    let wide_values = read_u8(wide_blend);
    let narrow_values = read_u8(narrow_blend);
    assert!(wide_values[3] > 10 && wide_values[3] < 200);
    assert_eq!(narrow_values[3], 10);
    assert!(wide_values[6] > 10 && wide_values[6] < 200);
    assert_eq!(narrow_values[6], 200);

    let match_reference = image_from_uchar(4, 1, 1, &[0, 0, 0, 0]);
    let match_secondary = image_from_uchar(2, 1, 1, &[50, 150]);
    let mut matched = ptr::null_mut();
    assert_eq!(
        unsafe {
            vips_match(
                match_reference,
                match_secondary,
                &mut matched,
                0,
                0,
                0,
                0,
                2,
                0,
                1,
                0,
                ptr::null::<c_char>(),
            )
        },
        0
    );
    assert_eq!(vips_image_get_width(matched), 4);
    assert_eq!(vips_image_get_height(matched), 1);
    assert_eq!(read_u8(matched), vec![50, 100, 150, 0]);

    unref_image(matched);
    unref_image(match_secondary);
    unref_image(match_reference);
    unref_image(narrow_blend);
    unref_image(wide_blend);
    unref_image(stamp);
    unref_image(secondary);
    unref_image(reference);
    unref_image(image);
}
