use std::ffi::c_char;
use std::ptr;
use std::slice;
use std::sync::{Mutex, Once, OnceLock};

use vips::*;

unsafe extern "C" {
    fn vips_add(left: *mut VipsImage, right: *mut VipsImage, out: *mut *mut VipsImage, ...) -> i32;
    fn vips_linear(
        input: *mut VipsImage,
        out: *mut *mut VipsImage,
        a: *const f64,
        b: *const f64,
        n: i32,
        ...
    ) -> i32;
    fn vips_crop(
        input: *mut VipsImage,
        out: *mut *mut VipsImage,
        left: i32,
        top: i32,
        width: i32,
        height: i32,
        ...
    ) -> i32;
    fn vips_grey(out: *mut *mut VipsImage, width: i32, height: i32, ...) -> i32;
    fn vips_gaussmat(out: *mut *mut VipsImage, sigma: f64, min_ampl: f64, ...) -> i32;
    fn vips_conv(input: *mut VipsImage, out: *mut *mut VipsImage, mask: *mut VipsImage, ...)
        -> i32;
    fn vips_hist_equal(input: *mut VipsImage, out: *mut *mut VipsImage, ...) -> i32;
    fn vips_mask_ideal(
        out: *mut *mut VipsImage,
        width: i32,
        height: i32,
        frequency_cutoff: f64,
        ...
    ) -> i32;
    fn vips_freqmult(
        input: *mut VipsImage,
        mask: *mut VipsImage,
        out: *mut *mut VipsImage,
        ...
    ) -> i32;
    fn vips_morph(
        input: *mut VipsImage,
        out: *mut *mut VipsImage,
        mask: *mut VipsImage,
        morph: VipsOperationMorphology,
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
        assert_eq!(vips_init(c"ops_core".as_ptr()), 0);
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

fn read_samples(image: *mut VipsImage) -> Vec<f64> {
    let format = vips_image_get_format(image);
    let mut len = 0usize;
    let ptr = vips_image_write_to_memory(image, &mut len);
    let bytes = unsafe { slice::from_raw_parts(ptr.cast::<u8>(), len) };
    let values = match format {
        VIPS_FORMAT_UCHAR => bytes.iter().map(|value| *value as f64).collect(),
        VIPS_FORMAT_USHORT => bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_ne_bytes([chunk[0], chunk[1]]) as f64)
            .collect(),
        VIPS_FORMAT_UINT => bytes
            .chunks_exact(4)
            .map(|chunk| u32::from_ne_bytes(chunk.try_into().unwrap()) as f64)
            .collect(),
        VIPS_FORMAT_FLOAT => bytes
            .chunks_exact(4)
            .map(|chunk| f32::from_ne_bytes(chunk.try_into().unwrap()) as f64)
            .collect(),
        VIPS_FORMAT_DOUBLE => bytes
            .chunks_exact(8)
            .map(|chunk| f64::from_ne_bytes(chunk.try_into().unwrap()))
            .collect(),
        _ => panic!("unsupported format {format}"),
    };
    unsafe {
        glib_sys::g_free(ptr);
    }
    values
}

fn unref_image(image: *mut VipsImage) {
    unsafe {
        gobject_sys::g_object_unref(image.cast());
    }
}

#[test]
fn arithmetic_and_extract_area_flow() {
    let _guard = guard();
    init_vips();

    let left = image_from_uchar(2, 2, 1, &[1, 2, 3, 4]);
    let right = image_from_uchar(2, 2, 1, &[10, 20, 30, 40]);

    let mut added = ptr::null_mut();
    assert_eq!(
        unsafe { vips_add(left, right, &mut added, ptr::null::<c_char>()) },
        0
    );
    assert_eq!(vips_image_get_format(added), VIPS_FORMAT_USHORT);
    assert_eq!(read_samples(added), vec![11.0, 22.0, 33.0, 44.0]);

    let a = [2.0];
    let b = [1.0];
    let mut linear = ptr::null_mut();
    assert_eq!(
        unsafe {
            vips_linear(
                left,
                &mut linear,
                a.as_ptr(),
                b.as_ptr(),
                1,
                ptr::null::<c_char>(),
            )
        },
        0
    );
    let linear_values = read_samples(linear);
    assert_eq!(vips_image_get_format(linear), VIPS_FORMAT_FLOAT);
    assert!((linear_values[0] - 3.0).abs() < 1e-6);
    assert!((linear_values[3] - 9.0).abs() < 1e-6);

    let mut crop = ptr::null_mut();
    assert_eq!(
        unsafe { vips_crop(added, &mut crop, 1, 0, 1, 2, ptr::null::<c_char>()) },
        0
    );
    assert_eq!(vips_image_get_width(crop), 1);
    assert_eq!(vips_image_get_height(crop), 2);
    assert_eq!(read_samples(crop), vec![22.0, 44.0]);
    assert_eq!(unsafe { (*crop).Xoffset }, -1);
    assert_eq!(unsafe { (*crop).Yoffset }, 0);
    assert_eq!(unsafe { (*crop).dhint }, VIPS_DEMAND_STYLE_THINSTRIP);

    unref_image(crop);
    unref_image(linear);
    unref_image(added);
    unref_image(right);
    unref_image(left);
}

#[test]
fn create_convolution_histogram_morphology_and_freqfilt_flow() {
    let _guard = guard();
    init_vips();

    let mut grey = ptr::null_mut();
    assert_eq!(
        unsafe {
            vips_grey(
                &mut grey,
                4,
                4,
                c"uchar".as_ptr(),
                1i32,
                ptr::null::<c_char>(),
            )
        },
        0
    );
    assert_eq!(vips_image_get_format(grey), VIPS_FORMAT_UCHAR);

    let mut gauss = ptr::null_mut();
    assert_eq!(
        unsafe { vips_gaussmat(&mut gauss, 1.0, 0.2, ptr::null::<c_char>()) },
        0
    );
    assert_eq!(vips_image_get_width(gauss), 3);

    let mut blurred = ptr::null_mut();
    assert_eq!(
        unsafe { vips_conv(grey, &mut blurred, gauss, ptr::null::<c_char>()) },
        0
    );
    assert_eq!(vips_image_get_width(blurred), 4);
    assert_eq!(vips_image_get_height(blurred), 4);
    assert_eq!(vips_image_get_format(blurred), VIPS_FORMAT_FLOAT);
    assert_eq!(unsafe { (*blurred).Xoffset }, -1);
    assert_eq!(unsafe { (*blurred).Yoffset }, -1);
    assert_eq!(unsafe { (*blurred).dhint }, VIPS_DEMAND_STYLE_SMALLTILE);
    let blurred_values = read_samples(blurred);
    assert!((blurred_values[0] - 23.020_833).abs() < 1e-5);
    assert!((blurred_values[3] - 231.979_17).abs() < 1e-4);

    let mut hist_equal = ptr::null_mut();
    assert_eq!(
        unsafe { vips_hist_equal(grey, &mut hist_equal, ptr::null::<c_char>()) },
        0
    );
    assert_eq!(vips_image_get_width(hist_equal), 4);
    assert_eq!(vips_image_get_format(hist_equal), VIPS_FORMAT_UCHAR);
    assert_eq!(
        read_samples(hist_equal),
        vec![
            63.0, 127.0, 191.0, 255.0, 63.0, 127.0, 191.0, 255.0, 63.0, 127.0, 191.0, 255.0, 63.0,
            127.0, 191.0, 255.0,
        ]
    );

    let mut mask = ptr::null_mut();
    assert_eq!(
        unsafe {
            vips_mask_ideal(
                &mut mask,
                4,
                4,
                0.0,
                c"reject".as_ptr(),
                1i32,
                ptr::null::<c_char>(),
            )
        },
        0
    );
    assert_eq!(
        read_samples(mask),
        vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
    );

    let mut filtered = ptr::null_mut();
    assert_eq!(
        unsafe { vips_freqmult(grey, mask, &mut filtered, ptr::null::<c_char>()) },
        0
    );
    assert_eq!(vips_image_get_width(filtered), 4);
    assert_eq!(vips_image_get_format(filtered), VIPS_FORMAT_UCHAR);
    assert_eq!(read_samples(filtered), vec![127.0; 16]);

    let binary = image_from_uchar(4, 4, 1, &[0, 0, 0, 0, 0, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let morph_mask = image_from_uchar(3, 3, 1, &[255, 255, 255, 255, 255, 255, 255, 255, 255]);
    let mut morphed = ptr::null_mut();
    assert_eq!(
        unsafe {
            vips_morph(
                binary,
                &mut morphed,
                morph_mask,
                VIPS_OPERATION_MORPHOLOGY_DILATE,
                ptr::null::<c_char>(),
            )
        },
        0
    );
    assert_eq!(vips_image_get_format(morphed), VIPS_FORMAT_UCHAR);
    assert_eq!(unsafe { (*morphed).Xoffset }, -1);
    assert_eq!(unsafe { (*morphed).Yoffset }, -1);
    assert_eq!(unsafe { (*morphed).dhint }, VIPS_DEMAND_STYLE_SMALLTILE);
    assert_eq!(
        read_samples(morphed),
        vec![
            255.0, 255.0, 255.0, 0.0, 255.0, 255.0, 255.0, 0.0, 255.0, 255.0, 255.0, 0.0, 0.0, 0.0,
            0.0, 0.0,
        ]
    );

    unref_image(morphed);
    unref_image(morph_mask);
    unref_image(binary);
    unref_image(filtered);
    unref_image(mask);
    unref_image(hist_equal);
    unref_image(blurred);
    unref_image(gauss);
    unref_image(grey);
}
