use std::ffi::c_char;
use std::ffi::CStr;
use std::ptr;
use std::slice;
use std::sync::{Mutex, Once, OnceLock};

use vips::*;

unsafe extern "C" {
    fn vips_HSV2sRGB(input: *mut VipsImage, out: *mut *mut VipsImage, ...) -> i32;
    fn vips_avg(input: *mut VipsImage, out: *mut f64, ...) -> i32;
    fn vips_colourspace(
        input: *mut VipsImage,
        out: *mut *mut VipsImage,
        space: VipsInterpretation,
        ...
    ) -> i32;
    fn vips_draw_rect(
        image: *mut VipsImage,
        ink: *const f64,
        n: i32,
        left: i32,
        top: i32,
        width: i32,
        height: i32,
        ...
    ) -> i32;
    fn vips_draw_image(
        image: *mut VipsImage,
        sub: *mut VipsImage,
        x: i32,
        y: i32,
        ...
    ) -> i32;
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
    fn vips_reduceh(input: *mut VipsImage, out: *mut *mut VipsImage, hshrink: f64, ...) -> i32;
    fn vips_reducev(input: *mut VipsImage, out: *mut *mut VipsImage, vshrink: f64, ...) -> i32;
    fn vips_resize(input: *mut VipsImage, out: *mut *mut VipsImage, scale: f64, ...) -> i32;
    fn vips_sRGB2HSV(input: *mut VipsImage, out: *mut *mut VipsImage, ...) -> i32;
    fn vips_thumbnail_buffer(
        buf: *mut std::ffi::c_void,
        len: usize,
        out: *mut *mut VipsImage,
        width: i32,
        ...
    ) -> i32;
    fn vips_thumbnail_image(
        input: *mut VipsImage,
        out: *mut *mut VipsImage,
        width: i32,
        ...
    ) -> i32;
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
    assert_eq!(unsafe { vips_sRGB2HSV(input, &mut hsv, ptr::null::<c_char>()) }, 0);
    assert_eq!(vips_image_get_format(hsv), VIPS_FORMAT_UCHAR);

    let mut srgb = ptr::null_mut();
    assert_eq!(unsafe { vips_HSV2sRGB(hsv, &mut srgb, ptr::null::<c_char>()) }, 0);
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
    assert_eq!(unsafe { vips_profile(input, &mut columns, &mut rows, ptr::null::<c_char>()) }, 0);
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

    let input = image_from_uchar(4, 4, 1, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);

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

    let mut thumb = ptr::null_mut();
    let thumb_result = unsafe { vips_thumbnail_image(input, &mut thumb, 2, ptr::null::<c_char>()) };
    assert_eq!(thumb_result, 0, "{}", error_text());
    assert_eq!(vips_image_get_width(thumb), 2);
    assert!(vips_image_get_height(thumb) >= 1);

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

    let reference = image_from_uchar(3, 1, 1, &[10, 20, 30]);
    let secondary = image_from_uchar(3, 1, 1, &[100, 110, 120]);

    let mut mosaic = ptr::null_mut();
    assert_eq!(
        unsafe {
            vips_mosaic(
                reference,
                secondary,
                &mut mosaic,
                VIPS_DIRECTION_HORIZONTAL,
                2,
                0,
                0,
                0,
                ptr::null::<c_char>(),
            )
        },
        0
    );
    assert_eq!(vips_image_get_width(mosaic), 5);
    let mosaic_values = read_u8(mosaic);
    assert_eq!(mosaic_values.first().copied(), Some(10));
    assert_eq!(mosaic_values.last().copied(), Some(120));

    let mut matched = ptr::null_mut();
    assert_eq!(
        unsafe {
            vips_match(
                reference,
                secondary,
                &mut matched,
                2,
                0,
                0,
                0,
                2,
                0,
                0,
                0,
                ptr::null::<c_char>(),
            )
        },
        0
    );
    assert_eq!(vips_image_get_width(matched), 5);

    unref_image(matched);
    unref_image(mosaic);
    unref_image(stamp);
    unref_image(secondary);
    unref_image(reference);
    unref_image(image);
}
