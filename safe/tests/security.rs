use std::slice;
use std::sync::{Mutex, Once, OnceLock};

use vips::*;

unsafe extern "C" {
    fn vips_crop(
        input: *mut VipsImage,
        out: *mut *mut VipsImage,
        left: i32,
        top: i32,
        width: i32,
        height: i32,
        ...
    ) -> i32;
    fn vips_eye(out: *mut *mut VipsImage, width: i32, height: i32, ...) -> i32;
    fn vips_mask_ideal(
        out: *mut *mut VipsImage,
        width: i32,
        height: i32,
        frequency_cutoff: f64,
        ...
    ) -> i32;
}

pub(crate) fn guard() -> std::sync::MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    match GUARD.get_or_init(|| Mutex::new(())).lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

pub(crate) fn init_vips() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        assert_eq!(vips_init(c"security".as_ptr()), 0);
    });
}

pub(crate) fn image_from_uchar(width: i32, height: i32, bytes: &[u8]) -> *mut VipsImage {
    vips_image_new_from_memory_copy(
        bytes.as_ptr().cast(),
        bytes.len(),
        width,
        height,
        1,
        VIPS_FORMAT_UCHAR,
    )
}

pub(crate) fn read_samples(image: *mut VipsImage) -> Vec<f64> {
    let format = vips_image_get_format(image);
    let mut len = 0usize;
    let ptr = vips_image_write_to_memory(image, &mut len);
    let bytes = unsafe { slice::from_raw_parts(ptr.cast::<u8>(), len) };
    let values = match format {
        VIPS_FORMAT_UCHAR => bytes.iter().map(|value| *value as f64).collect(),
        VIPS_FORMAT_FLOAT => bytes
            .chunks_exact(4)
            .map(|chunk| f32::from_ne_bytes(chunk.try_into().unwrap()) as f64)
            .collect(),
        _ => bytes.iter().map(|value| *value as f64).collect(),
    };
    unsafe {
        glib_sys::g_free(ptr);
    }
    values
}

pub(crate) fn error_message() -> String {
    let ptr = vips_error_buffer();
    if ptr.is_null() {
        String::new()
    } else {
        unsafe { std::ffi::CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned()
    }
}

pub(crate) fn unref_image(image: *mut VipsImage) {
    unsafe {
        gobject_sys::g_object_unref(image.cast());
    }
}

#[path = "security/cve_2018_7998.rs"]
mod cve_2018_7998;
#[path = "security/cve_2019_6976.rs"]
mod cve_2019_6976;
#[path = "security/cve_2021_27847.rs"]
mod cve_2021_27847;
#[path = "security/cve_2023_40032.rs"]
mod cve_2023_40032;
#[path = "security/cve_2026_3146.rs"]
mod cve_2026_3146;
#[path = "security/cve_2026_3284.rs"]
mod cve_2026_3284;
