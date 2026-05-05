use std::ffi::{c_char, CString};
use std::path::PathBuf;
use std::ptr;
use std::slice;
use std::sync::{Mutex, Once, OnceLock};

use vips::*;

unsafe extern "C" {
    fn vips_add(left: *mut VipsImage, right: *mut VipsImage, out: *mut *mut VipsImage, ...) -> i32;
    fn vips_autorot(input: *mut VipsImage, out: *mut *mut VipsImage, ...) -> i32;
    fn vips_canny(input: *mut VipsImage, out: *mut *mut VipsImage, ...) -> i32;
    fn vips_composite(
        input: *mut *mut VipsImage,
        out: *mut *mut VipsImage,
        n: i32,
        mode: *mut VipsBlendMode,
        ...
    ) -> i32;
    fn vips_find_trim(
        input: *mut VipsImage,
        left: *mut i32,
        top: *mut i32,
        width: *mut i32,
        height: *mut i32,
        ...
    ) -> i32;
    fn vips_hist_norm(input: *mut VipsImage, out: *mut *mut VipsImage, ...) -> i32;
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
    fn vips_gravity(
        input: *mut VipsImage,
        out: *mut *mut VipsImage,
        direction: VipsCompassDirection,
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
    fn vips_round(
        input: *mut VipsImage,
        out: *mut *mut VipsImage,
        round: VipsOperationRound,
        ...
    ) -> i32;
    fn vips_image_write_to_file(image: *mut VipsImage, name: *const c_char, ...) -> i32;
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

fn image_from_double(width: i32, height: i32, bands: i32, values: &[f64]) -> *mut VipsImage {
    vips_image_new_from_memory_copy(
        values.as_ptr().cast(),
        std::mem::size_of_val(values),
        width,
        height,
        bands,
        VIPS_FORMAT_DOUBLE,
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

fn temp_output_path(suffix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "libvips-safe-ops-{}-{}{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos(),
        suffix
    ));
    path
}

fn assert_write_file_magic(image: *mut VipsImage, suffix: &str, magic: &[u8]) {
    let path = temp_output_path(suffix);
    let c_path = CString::new(path.to_string_lossy().into_owned()).expect("output path");
    assert_eq!(
        unsafe { vips_image_write_to_file(image, c_path.as_ptr(), ptr::null::<c_char>()) },
        0
    );
    let bytes = std::fs::read(&path).expect("saved output");
    let _ = std::fs::remove_file(&path);
    assert!(
        bytes.starts_with(magic),
        "bad file magic for {suffix}: {bytes:?}"
    );
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
fn gravity_centre_crop_matches_ruby_usage_case() {
    let _guard = guard();
    init_vips();

    let input = image_from_uchar(3, 3, 1, &[1, 2, 3, 4, 5, 6, 7, 8, 9]);
    let mut output = ptr::null_mut();
    assert_eq!(
        unsafe {
            vips_gravity(
                input,
                &mut output,
                VIPS_COMPASS_DIRECTION_CENTRE,
                2,
                2,
                ptr::null::<c_char>(),
            )
        },
        0
    );
    assert_eq!(vips_image_get_width(output), 2);
    assert_eq!(vips_image_get_height(output), 2);
    assert_eq!(vips_image_get_bands(output), 1);
    assert_eq!(vips_image_get_format(output), VIPS_FORMAT_UCHAR);
    assert_eq!(read_samples(output), vec![1.0, 2.0, 4.0, 5.0]);

    unref_image(output);
    unref_image(input);
}

#[test]
fn operation_semantics_ruby_failure_regressions() {
    let _guard = guard();
    init_vips();

    let autorot_input = image_from_uchar(
        6,
        4,
        1,
        &[
            10, 20, 30, 40, 50, 60, 11, 21, 31, 41, 51, 61, 12, 22, 32, 42, 52, 62, 13, 23, 33, 43,
            53, 63,
        ],
    );
    let mut autorot = ptr::null_mut();
    assert_eq!(
        unsafe { vips_autorot(autorot_input, &mut autorot, ptr::null::<c_char>()) },
        0
    );
    assert_eq!(vips_image_get_width(autorot), 6);
    assert_eq!(vips_image_get_height(autorot), 4);
    assert_eq!(vips_image_get_bands(autorot), 1);
    assert_eq!(vips_image_get_format(autorot), VIPS_FORMAT_UCHAR);
    assert_eq!(read_samples(autorot), read_samples(autorot_input));

    let mut canny_pixels = Vec::new();
    for _ in 0..8 {
        canny_pixels.extend([0, 0, 0, 0, 255, 255, 255, 255]);
    }
    let canny_input = image_from_uchar(8, 8, 1, &canny_pixels);
    let mut canny = ptr::null_mut();
    assert_eq!(
        unsafe { vips_canny(canny_input, &mut canny, ptr::null::<c_char>()) },
        0
    );
    assert_eq!(vips_image_get_width(canny), 8);
    assert_eq!(vips_image_get_height(canny), 8);
    assert_eq!(vips_image_get_bands(canny), 1);
    assert_eq!(vips_image_get_format(canny), VIPS_FORMAT_UCHAR);
    let canny_values = read_samples(canny);
    assert_eq!(canny_values[2 + 4 * 8], 0.0);
    assert!(canny_values[3 + 4 * 8] + canny_values[4 + 4 * 8] > 1.0);
    assert_write_file_magic(canny, ".tif", b"II*\0");

    let mut canny_rgb_pixels = Vec::new();
    for _ in 0..8 {
        for x in 0..8 {
            let value = if x < 4 { 0 } else { 255 };
            canny_rgb_pixels.extend([value, value / 2, 255 - value]);
        }
    }
    let canny_rgb_input = image_from_uchar(8, 8, 3, &canny_rgb_pixels);
    let mut canny_rgb = ptr::null_mut();
    assert_eq!(
        unsafe {
            vips_canny(
                canny_rgb_input,
                &mut canny_rgb,
                c"sigma".as_ptr(),
                0.7f64,
                c"precision".as_ptr(),
                VIPS_PRECISION_FLOAT,
                ptr::null::<c_char>(),
            )
        },
        0
    );
    assert_eq!(vips_image_get_width(canny_rgb), 8);
    assert_eq!(vips_image_get_height(canny_rgb), 8);
    assert_eq!(vips_image_get_bands(canny_rgb), 3);
    assert_eq!(vips_image_get_format(canny_rgb), VIPS_FORMAT_UCHAR);

    let trim_input = image_from_uchar(
        6,
        5,
        1,
        &[
            200, 200, 200, 200, 200, 200, 200, 200, 50, 50, 50, 200, 200, 200, 50, 50, 50, 200,
            200, 200, 50, 50, 50, 200, 200, 200, 200, 200, 200, 200,
        ],
    );
    let background = [200.0];
    let background_array = vips_array_double_new(background.as_ptr(), background.len() as i32);
    let mut left = -1;
    let mut top = -1;
    let mut width = -1;
    let mut height = -1;
    assert_eq!(
        unsafe {
            vips_find_trim(
                trim_input,
                &mut left,
                &mut top,
                &mut width,
                &mut height,
                c"background".as_ptr(),
                background_array,
                c"threshold".as_ptr(),
                60.0f64,
                ptr::null::<c_char>(),
            )
        },
        0
    );
    assert_eq!((left, top, width, height), (2, 1, 3, 3));
    left = -1;
    top = -1;
    width = -1;
    height = -1;
    assert_eq!(
        unsafe {
            vips_find_trim(
                trim_input,
                &mut left,
                &mut top,
                &mut width,
                &mut height,
                c"background".as_ptr(),
                background_array,
                c"threshold".as_ptr(),
                200.0f64,
                ptr::null::<c_char>(),
            )
        },
        0
    );
    assert_eq!((width, height), (0, 0));
    vips_area_unref(background_array.cast());

    let default_trim_input = image_from_uchar(
        5,
        5,
        1,
        &[
            0, 0, 0, 255, 255, 0, 0, 0, 255, 255, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255,
            255, 255, 255, 255,
        ],
    );
    left = -1;
    top = -1;
    width = -1;
    height = -1;
    assert_eq!(
        unsafe {
            vips_find_trim(
                default_trim_input,
                &mut left,
                &mut top,
                &mut width,
                &mut height,
                c"line_art".as_ptr(),
                1i32,
                ptr::null::<c_char>(),
            )
        },
        0
    );
    assert_eq!((left, top, width, height), (0, 0, 3, 3));

    let line_art_input = image_from_uchar(
        5,
        5,
        1,
        &[
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 255, 255, 255, 255, 255,
            255, 255, 255, 255, 255, 255, 255,
        ],
    );
    left = -1;
    top = -1;
    width = -1;
    height = -1;
    assert_eq!(
        unsafe {
            vips_find_trim(
                line_art_input,
                &mut left,
                &mut top,
                &mut width,
                &mut height,
                ptr::null::<c_char>(),
            )
        },
        0
    );
    assert_eq!((width, height), (0, 0));
    assert_eq!(
        unsafe {
            vips_find_trim(
                line_art_input,
                &mut left,
                &mut top,
                &mut width,
                &mut height,
                c"line_art".as_ptr(),
                1i32,
                ptr::null::<c_char>(),
            )
        },
        0
    );
    assert_eq!((left, top, width, height), (2, 2, 1, 1));

    let hist_pixels = [20, 30, 40, 80];
    let hist_input = image_from_uchar(2, 2, 1, &hist_pixels);
    let mut hist_norm = ptr::null_mut();
    assert_eq!(
        unsafe { vips_hist_norm(hist_input, &mut hist_norm, ptr::null::<c_char>()) },
        0
    );
    assert_eq!(vips_image_get_width(hist_norm), 2);
    assert_eq!(vips_image_get_height(hist_norm), 2);
    assert_eq!(vips_image_get_bands(hist_norm), 1);
    assert_eq!(vips_image_get_format(hist_norm), VIPS_FORMAT_UCHAR);
    let hist_values = read_samples(hist_norm);
    assert_eq!(hist_values, vec![0.0, 1.0, 1.0, 3.0]);
    assert_write_file_magic(hist_norm, ".png", b"\x89PNG\r\n\x1a\n");

    let round_input = image_from_double(6, 1, 1, &[-1.4, -0.5, 0.5, 1.5, 2.5, 2.6]);
    let mut rounded = ptr::null_mut();
    assert_eq!(
        unsafe {
            vips_round(
                round_input,
                &mut rounded,
                VIPS_OPERATION_ROUND_RINT,
                ptr::null::<c_char>(),
            )
        },
        0
    );
    assert_eq!(vips_image_get_format(rounded), VIPS_FORMAT_DOUBLE);
    assert_eq!(read_samples(rounded), vec![-1.0, -0.0, 0.0, 2.0, 2.0, 3.0]);

    let base = image_from_uchar(2, 2, 3, &[200, 0, 0, 200, 0, 0, 200, 0, 0, 200, 0, 0]);
    let overlay = image_from_uchar(
        2,
        2,
        4,
        &[0, 200, 0, 0, 0, 200, 0, 0, 0, 200, 0, 0, 0, 200, 0, 0],
    );
    unsafe {
        (*base).Type = VIPS_INTERPRETATION_sRGB;
        (*overlay).Type = VIPS_INTERPRETATION_sRGB;
    }
    let mut images = [base, overlay];
    let mut modes = [VIPS_BLEND_MODE_OVER];
    let mut composited = ptr::null_mut();
    assert_eq!(
        unsafe {
            vips_composite(
                images.as_mut_ptr(),
                &mut composited,
                images.len() as i32,
                modes.as_mut_ptr(),
                ptr::null::<c_char>(),
            )
        },
        0
    );
    assert_eq!(vips_image_get_width(composited), 2);
    assert_eq!(vips_image_get_height(composited), 2);
    assert_eq!(vips_image_get_bands(composited), 4);
    assert_eq!(vips_image_get_format(composited), VIPS_FORMAT_UCHAR);
    assert_eq!(
        read_samples(composited),
        vec![
            200.0, 0.0, 0.0, 255.0, 200.0, 0.0, 0.0, 255.0, 200.0, 0.0, 0.0, 255.0, 200.0, 0.0,
            0.0, 255.0,
        ]
    );
    assert_write_file_magic(composited, ".png", b"\x89PNG\r\n\x1a\n");

    unref_image(composited);
    unref_image(overlay);
    unref_image(base);
    unref_image(rounded);
    unref_image(round_input);
    unref_image(hist_norm);
    unref_image(hist_input);
    unref_image(line_art_input);
    unref_image(default_trim_input);
    unref_image(trim_input);
    unref_image(canny_rgb);
    unref_image(canny_rgb_input);
    unref_image(canny);
    unref_image(canny_input);
    unref_image(autorot);
    unref_image(autorot_input);
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
