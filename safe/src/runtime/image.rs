use std::ffi::{c_void, CStr, CString};
use std::io::Cursor;
use std::os::raw::c_char;
use std::ptr;
use std::slice;
use std::sync::Mutex;

use crate::abi::connection::{VipsSource, VipsTarget};
use crate::abi::image::*;
use crate::foreign::base::PendingDecode;
use crate::pixels::format::write_sample;
use crate::runtime::error::append_message_str;
use crate::runtime::header::{copy_metadata, MetaStore};
use crate::runtime::object::{get_qdata_ptr, object_new, qdata_quark, set_qdata_box};

static IMAGE_STATE_QUARK: &CStr = c"safe-vips-image-state";

pub(crate) struct ImageState {
    pub pixels: Vec<u8>,
    pub filename: Option<CString>,
    pub mode: Option<CString>,
    pub history: Option<CString>,
    pub meta: Mutex<MetaStore>,
    pub pending_load: Option<PendingDecode>,
    pub source: Option<*mut VipsSource>,
    pub fd: Option<libc::c_int>,
    pub progress: Option<Box<VipsProgress>>,
}

impl Drop for ImageState {
    fn drop(&mut self) {
        if let Some(source) = self.source.take() {
            unsafe {
                crate::runtime::object::object_unref(source);
            }
        }
        if let Some(fd) = self.fd.take() {
            crate::runtime::memory::vips_tracked_close(fd);
        }
        if let Some(progress) = self.progress.as_mut() {
            if !progress.start.is_null() {
                unsafe {
                    glib_sys::g_timer_destroy(progress.start);
                }
                progress.start = ptr::null_mut();
            }
        }
    }
}

pub(crate) fn image_quark() -> glib_sys::GQuark {
    qdata_quark(IMAGE_STATE_QUARK)
}

pub(crate) unsafe fn image_state(image: *mut VipsImage) -> Option<&'static mut ImageState> {
    unsafe { get_qdata_ptr::<ImageState>(image.cast(), image_quark()).as_mut() }
}

pub(crate) fn format_sizeof(format: VipsBandFormat) -> usize {
    match format {
        VIPS_FORMAT_UCHAR | VIPS_FORMAT_CHAR => 1,
        VIPS_FORMAT_USHORT | VIPS_FORMAT_SHORT => 2,
        VIPS_FORMAT_UINT | VIPS_FORMAT_INT | VIPS_FORMAT_FLOAT => 4,
        VIPS_FORMAT_COMPLEX => 8,
        VIPS_FORMAT_DOUBLE => 8,
        VIPS_FORMAT_DPCOMPLEX => 16,
        _ => 0,
    }
}

pub(crate) fn bytes_per_pixel(image: &VipsImage) -> usize {
    format_sizeof(image.BandFmt).saturating_mul(image.Bands.max(0) as usize)
}

pub(crate) fn line_size(image: &VipsImage) -> usize {
    bytes_per_pixel(image).saturating_mul(image.Xsize.max(0) as usize)
}

pub(crate) fn image_size(image: &VipsImage) -> usize {
    line_size(image).saturating_mul(image.Ysize.max(0) as usize)
}

fn init_defaults(image: &mut VipsImage) {
    image.Xsize = 0;
    image.Ysize = 0;
    image.Bands = 0;
    image.BandFmt = VIPS_FORMAT_UCHAR;
    image.Coding = VIPS_CODING_NONE;
    image.Type = VIPS_INTERPRETATION_MULTIBAND;
    image.Xres = 1.0;
    image.Yres = 1.0;
    image.Xoffset = 0;
    image.Yoffset = 0;
    image.Length = 0;
    image.Compression = 0;
    image.Level = 0;
    image.Bbits = 0;
    image.time = ptr::null_mut();
    image.Hist = ptr::null_mut();
    image.filename = ptr::null_mut();
    image.data = ptr::null_mut();
    image.kill = 0;
    image.Xres_float = 1.0;
    image.Yres_float = 1.0;
    image.mode = ptr::null_mut();
    image.dtype = VIPS_IMAGE_NONE;
    image.fd = -1;
    image.baseaddr = ptr::null_mut();
    image.length = 0;
    image.magic = 0xb6a6f208;
    image.start_fn = None;
    image.generate_fn = None;
    image.stop_fn = None;
    image.client1 = ptr::null_mut();
    image.client2 = ptr::null_mut();
    image.sslock = ptr::null_mut();
    image.regions = ptr::null_mut();
    image.dhint = VIPS_DEMAND_STYLE_ANY;
    image.meta = ptr::null_mut();
    image.meta_traverse = ptr::null_mut();
    image.sizeof_header = 0;
    image.windows = ptr::null_mut();
    image.upstream = ptr::null_mut();
    image.downstream = ptr::null_mut();
    image.serial = 0;
    image.history_list = ptr::null_mut();
    image.progress_signal = ptr::null_mut();
    image.file_length = 0;
    image.hint_set = glib_sys::GFALSE;
    image.delete_on_close = glib_sys::GFALSE;
    image.delete_on_close_filename = ptr::null_mut();
}

fn attach_state(image: *mut VipsImage, mode: Option<&str>) -> *mut VipsImage {
    let Some(image_ref) = (unsafe { image.as_mut() }) else {
        return ptr::null_mut();
    };
    init_defaults(image_ref);
    unsafe {
        set_qdata_box(
            image.cast(),
            image_quark(),
            ImageState {
                pixels: Vec::new(),
                filename: None,
                mode: mode.map(|mode| CString::new(mode).expect("mode")),
                history: None,
                meta: Mutex::new(MetaStore::default()),
                pending_load: None,
                source: None,
                fd: None,
                progress: None,
            },
        );
    }
    if let Some(state) = unsafe { image_state(image) } {
        image_ref.mode = state
            .mode
            .as_ref()
            .map_or(ptr::null_mut(), |mode| mode.as_ptr().cast_mut());
    }
    image
}

pub(crate) fn set_filename(image: *mut VipsImage, filename: Option<&CStr>) {
    if let (Some(state), Some(image_ref)) =
        (unsafe { image_state(image) }, unsafe { image.as_mut() })
    {
        state.filename = filename.map(CStr::to_owned);
        image_ref.filename = state
            .filename
            .as_ref()
            .map_or(ptr::null_mut(), |filename| filename.as_ptr().cast_mut());
    }
}

pub(crate) fn set_mode(image: *mut VipsImage, mode: &str) {
    if let (Some(state), Some(image_ref)) =
        (unsafe { image_state(image) }, unsafe { image.as_mut() })
    {
        state.mode = Some(CString::new(mode).expect("mode"));
        image_ref.mode = state.mode.as_ref().unwrap().as_ptr().cast_mut();
    }
}

pub(crate) fn set_history(image: *mut VipsImage, history: Option<&str>) {
    if let (Some(state), Some(image_ref)) =
        (unsafe { image_state(image) }, unsafe { image.as_mut() })
    {
        state.history = history.and_then(|history| {
            CString::new(history)
                .ok()
                .or_else(|| CString::new(history.replace('\0', "")).ok())
        });
        image_ref.Hist = state
            .history
            .as_ref()
            .map_or(ptr::null_mut(), |history| history.as_ptr().cast_mut());
    }
}

pub(crate) fn sync_pixels(image: *mut VipsImage) {
    let Some(image_ref) = (unsafe { image.as_mut() }) else {
        return;
    };
    if let Some(state) = unsafe { image_state(image) } {
        image_ref.data = if state.pixels.is_empty() {
            ptr::null_mut()
        } else {
            state.pixels.as_mut_ptr()
        };
        image_ref.length = state.pixels.len();
    }
}

fn zero_gvalue() -> gobject_sys::GValue {
    unsafe { std::mem::zeroed() }
}

fn image_progress_pixels(image: &VipsImage) -> u64 {
    let width = image.Xsize.max(1) as u64;
    let height = image.Ysize.max(1) as u64;
    width.saturating_mul(height).max(1)
}

fn has_progress_signal(image: *mut VipsImage) -> bool {
    unsafe { image.as_ref() }.is_some_and(|image| !image.progress_signal.is_null())
}

fn progress_target(image: *mut VipsImage) -> *mut VipsImage {
    unsafe { image.as_ref() }
        .and_then(|image| (!image.progress_signal.is_null()).then_some(image.progress_signal))
        .unwrap_or(image)
}

fn emit_progress_visible(image: *mut VipsImage) -> bool {
    has_progress_signal(image)
        && crate::runtime::header::vips_image_get_typeof(image, c"hide-progress".as_ptr()) == 0
}

fn ensure_progress_struct(image: *mut VipsImage) -> Option<*mut VipsProgress> {
    let image_ref = unsafe { image.as_mut() }?;
    let state = unsafe { image_state(image) }?;
    if state.progress.is_none() {
        state.progress = Some(Box::new(VipsProgress {
            im: image,
            run: 0,
            eta: 0,
            tpels: 0,
            npels: 0,
            percent: 0,
            start: ptr::null_mut(),
        }));
    }
    let progress = state.progress.as_mut()?;
    progress.im = image;
    progress.tpels = image_progress_pixels(image_ref).min(i64::MAX as u64) as i64;
    if progress.start.is_null() {
        progress.start = unsafe { glib_sys::g_timer_new() };
    }
    image_ref.time = progress.as_mut() as *mut VipsProgress;
    Some(image_ref.time)
}

fn current_progress_struct(image: *mut VipsImage) -> Option<*mut VipsProgress> {
    let image_ref = unsafe { image.as_ref() }?;
    (!image_ref.time.is_null()).then_some(image_ref.time)
}

fn update_progress_struct(progress: *mut VipsProgress, processed: u64) {
    let Some(progress_ref) = (unsafe { progress.as_mut() }) else {
        return;
    };
    let total = progress_ref.tpels.max(1) as u64;
    let processed = processed.min(total);
    let prop = processed as f64 / total as f64;

    if !progress_ref.start.is_null() {
        let elapsed = unsafe { glib_sys::g_timer_elapsed(progress_ref.start, ptr::null_mut()) };
        progress_ref.run = elapsed as libc::c_int;
        progress_ref.eta = if prop > 0.1 {
            (((1.0 / prop) * elapsed) - elapsed).max(0.0) as libc::c_int
        } else {
            0
        };
    } else {
        progress_ref.run = 0;
        progress_ref.eta = 0;
    }
    progress_ref.npels = processed.min(i64::MAX as u64) as i64;
    progress_ref.percent = (100.0 * prop).round().clamp(0.0, 100.0) as libc::c_int;
}

fn emit_progress_signal(signal_image: *mut VipsImage, signal_id: u32, progress: *mut VipsProgress) {
    let mut args = [zero_gvalue(), zero_gvalue()];
    unsafe {
        gobject_sys::g_value_init(&mut args[0], gobject_sys::g_object_get_type());
        gobject_sys::g_value_set_object(&mut args[0], signal_image.cast());
        gobject_sys::g_value_init(&mut args[1], gobject_sys::G_TYPE_POINTER);
        gobject_sys::g_value_set_pointer(&mut args[1], progress.cast::<c_void>());
        gobject_sys::g_signal_emitv(args.as_ptr(), signal_id, 0, ptr::null_mut());
        for value in &mut args {
            gobject_sys::g_value_unset(value);
        }
    }
}

pub(crate) fn ensure_pixels(image: *mut VipsImage) -> Result<(), ()> {
    let Some(image_ref) = (unsafe { image.as_mut() }) else {
        return Err(());
    };
    let Some(state) = (unsafe { image_state(image) }) else {
        return Err(());
    };
    if !state.pixels.is_empty() {
        sync_pixels(image);
        return Ok(());
    }
    if let Some(pending) = state.pending_load.clone() {
        let pixels = crate::foreign::decode_pending(&pending)?;
        if let Some(state) = unsafe { image_state(image) } {
            state.pixels = pixels;
            state.pending_load = None;
        }
        sync_pixels(image);
        return Ok(());
    }
    if image_ref.generate_fn.is_some() {
        return crate::runtime::region::materialize_generated_image(image);
    }
    if image_ref.dtype == VIPS_IMAGE_SETBUF || image_ref.dtype == VIPS_IMAGE_OPENOUT {
        state.pixels.resize(image_size(image_ref), 0);
        sync_pixels(image);
        return Ok(());
    }
    if !image_ref.data.is_null() && image_ref.length > 0 {
        let bytes =
            unsafe { std::slice::from_raw_parts(image_ref.data.cast::<u8>(), image_ref.length) };
        state.pixels = bytes.to_vec();
        sync_pixels(image);
        return Ok(());
    }
    append_message_str("vips_image", "image has no pixel data");
    Err(())
}

pub(crate) fn safe_decode_png_bytes(
    bytes: &[u8],
) -> Result<
    (
        Vec<u8>,
        u32,
        u32,
        i32,
        VipsBandFormat,
        VipsInterpretation,
        u8,
    ),
    String,
> {
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let mut reader = decoder.read_info().map_err(|err| err.to_string())?;
    let mut out = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut out).map_err(|err| err.to_string())?;
    let bands = match info.color_type {
        png::ColorType::Grayscale => 1,
        png::ColorType::Rgb => 3,
        png::ColorType::Indexed => 1,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Rgba => 4,
    };
    let interpretation = match (bands, info.bit_depth) {
        (1, png::BitDepth::Eight) => VIPS_INTERPRETATION_B_W,
        (1, png::BitDepth::Sixteen) => VIPS_INTERPRETATION_GREY16,
        (3 | 4, png::BitDepth::Eight) => VIPS_INTERPRETATION_sRGB,
        (3 | 4, png::BitDepth::Sixteen) => VIPS_INTERPRETATION_RGB16,
        _ => VIPS_INTERPRETATION_MULTIBAND,
    };
    let band_format = match info.bit_depth {
        png::BitDepth::Sixteen => VIPS_FORMAT_USHORT,
        _ => VIPS_FORMAT_UCHAR,
    };
    out.truncate(info.buffer_size());
    if matches!(info.bit_depth, png::BitDepth::Sixteen) {
        for chunk in out.chunks_exact_mut(2) {
            let value = u16::from_be_bytes([chunk[0], chunk[1]]);
            chunk.copy_from_slice(&value.to_ne_bytes());
        }
    }
    Ok((
        out,
        info.width,
        info.height,
        bands,
        band_format,
        interpretation,
        match info.bit_depth {
            png::BitDepth::One => 1,
            png::BitDepth::Two => 2,
            png::BitDepth::Four => 4,
            png::BitDepth::Eight => 8,
            png::BitDepth::Sixteen => 16,
        },
    ))
}

pub(crate) fn safe_encode_png_bytes(image: &VipsImage, pixels: &[u8]) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(
        &mut bytes,
        image.Xsize.max(0) as u32,
        image.Ysize.max(0) as u32,
    );
    encoder.set_depth(match image.BandFmt {
        VIPS_FORMAT_USHORT => png::BitDepth::Sixteen,
        _ => png::BitDepth::Eight,
    });
    encoder.set_color(match image.Bands {
        1 => png::ColorType::Grayscale,
        2 => png::ColorType::GrayscaleAlpha,
        3 => png::ColorType::Rgb,
        4 => png::ColorType::Rgba,
        _ => return Err("unsupported band count for png".to_owned()),
    });
    let mut writer = encoder.write_header().map_err(|err| err.to_string())?;
    let data = if image.BandFmt == VIPS_FORMAT_USHORT {
        let mut data = pixels.to_vec();
        for chunk in data.chunks_exact_mut(2) {
            let value = u16::from_ne_bytes([chunk[0], chunk[1]]);
            chunk.copy_from_slice(&value.to_be_bytes());
        }
        data
    } else {
        pixels.to_vec()
    };
    writer
        .write_image_data(&data)
        .map_err(|err| err.to_string())?;
    drop(writer);
    Ok(bytes)
}

#[no_mangle]
pub extern "C" fn vips_image_new() -> *mut VipsImage {
    let image = unsafe { object_new::<VipsImage>(crate::runtime::object::vips_image_get_type()) };
    attach_state(image, Some("p"))
}

#[no_mangle]
pub extern "C" fn vips_image_new_memory() -> *mut VipsImage {
    let image = vips_image_new();
    if let Some(image_ref) = unsafe { image.as_mut() } {
        image_ref.dtype = VIPS_IMAGE_SETBUF;
        image_ref.dhint = VIPS_DEMAND_STYLE_ANY;
    }
    set_mode(image, "t");
    image
}

#[no_mangle]
pub extern "C" fn vips_image_memory() -> *mut VipsImage {
    vips_image_new_memory()
}

#[no_mangle]
pub extern "C" fn vips_image_new_from_memory(
    data: *const c_void,
    size: usize,
    width: libc::c_int,
    height: libc::c_int,
    bands: libc::c_int,
    format: VipsBandFormat,
) -> *mut VipsImage {
    let image = vips_image_new_memory();
    crate::runtime::header::vips_image_init_fields(
        image,
        width,
        height,
        bands,
        format,
        VIPS_CODING_NONE,
        if bands == 1 {
            VIPS_INTERPRETATION_B_W
        } else {
            VIPS_INTERPRETATION_sRGB
        },
        1.0,
        1.0,
    );
    if let Some(state) = unsafe { image_state(image) } {
        if !data.is_null() && size > 0 {
            state.pixels = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), size) }.to_vec();
        }
    }
    sync_pixels(image);
    image
}

#[no_mangle]
pub extern "C" fn vips_image_new_from_memory_copy(
    data: *const c_void,
    size: usize,
    width: libc::c_int,
    height: libc::c_int,
    bands: libc::c_int,
    format: VipsBandFormat,
) -> *mut VipsImage {
    vips_image_new_from_memory(data, size, width, height, bands, format)
}

#[no_mangle]
pub extern "C" fn vips_image_new_from_image(
    image: *mut VipsImage,
    values: *const f64,
    n_values: libc::c_int,
) -> *mut VipsImage {
    let Some(input) = (unsafe { image.as_ref() }) else {
        append_message_str("vips_image_new_from_image", "image is NULL");
        return ptr::null_mut();
    };
    if input.Xsize <= 0 || input.Ysize <= 0 || input.Bands <= 0 {
        append_message_str("vips_image_new_from_image", "image has invalid dimensions");
        return ptr::null_mut();
    }
    if values.is_null() || n_values <= 0 {
        append_message_str("vips_image_new_from_image", "pixel array is NULL");
        return ptr::null_mut();
    }

    let band_count = input.Bands as usize;
    let value_count = n_values as usize;
    if value_count != 1 && value_count != band_count {
        append_message_str(
            "vips_image_new_from_image",
            "pixel array must contain either one value or one value per band",
        );
        return ptr::null_mut();
    }

    let sample_bytes = format_sizeof(input.BandFmt);
    if sample_bytes == 0 {
        append_message_str("vips_image_new_from_image", "unsupported band format");
        return ptr::null_mut();
    }

    let pixel_count = input.Xsize as usize * input.Ysize as usize;
    let bytes_per_pixel = sample_bytes * band_count;
    let values = unsafe { slice::from_raw_parts(values, value_count) };
    let out = vips_image_new_memory();
    crate::runtime::header::vips_image_init_fields(
        out,
        input.Xsize,
        input.Ysize,
        input.Bands,
        input.BandFmt,
        input.Coding,
        input.Type,
        input.Xres,
        input.Yres,
    );

    let Some(state) = (unsafe { image_state(out) }) else {
        return ptr::null_mut();
    };

    let mut pixel = vec![0u8; bytes_per_pixel];
    for band in 0..band_count {
        let value = if value_count == 1 {
            values[0]
        } else {
            values[band]
        };
        if !write_sample(
            &mut pixel[band * sample_bytes..(band + 1) * sample_bytes],
            input.BandFmt,
            value,
        ) {
            unsafe {
                crate::runtime::object::object_unref(out);
            }
            append_message_str(
                "vips_image_new_from_image",
                "failed to encode constant pixel",
            );
            return ptr::null_mut();
        }
    }

    state.pixels = pixel.repeat(pixel_count);
    sync_pixels(out);
    copy_metadata(out, image);
    out
}

#[no_mangle]
pub extern "C" fn vips_image_new_from_image1(image: *mut VipsImage, value: f64) -> *mut VipsImage {
    vips_image_new_from_image(image, &value, 1)
}

#[no_mangle]
pub extern "C" fn vips_image_new_matrix(width: libc::c_int, height: libc::c_int) -> *mut VipsImage {
    if width <= 0 || height <= 0 {
        append_message_str(
            "vips_image_new_matrix",
            "matrix dimensions must be positive",
        );
        return ptr::null_mut();
    }

    let image = vips_image_new_memory();
    crate::runtime::header::vips_image_init_fields(
        image,
        width,
        height,
        1,
        VIPS_FORMAT_DOUBLE,
        VIPS_CODING_NONE,
        VIPS_INTERPRETATION_MATRIX,
        1.0,
        1.0,
    );
    if let Some(state) = unsafe { image_state(image) } {
        state.pixels =
            vec![0; width as usize * height as usize * format_sizeof(VIPS_FORMAT_DOUBLE)];
    }
    sync_pixels(image);
    image
}

#[no_mangle]
pub extern "C" fn vips_image_new_matrix_from_array(
    width: libc::c_int,
    height: libc::c_int,
    array: *const f64,
    size: libc::c_int,
) -> *mut VipsImage {
    if width <= 0 || height <= 0 || size != width.saturating_mul(height) {
        append_message_str(
            "VipsImage",
            &format!(
                "bad array length --- should be {}, you passed {}",
                width.saturating_mul(height),
                size
            ),
        );
        return ptr::null_mut();
    }
    if size != 0 && array.is_null() {
        append_message_str("VipsImage", "matrix array is NULL");
        return ptr::null_mut();
    }

    let image = vips_image_new_memory();
    crate::runtime::header::vips_image_init_fields(
        image,
        width,
        height,
        1,
        VIPS_FORMAT_DOUBLE,
        VIPS_CODING_NONE,
        VIPS_INTERPRETATION_MATRIX,
        1.0,
        1.0,
    );
    if let Some(state) = unsafe { image_state(image) } {
        let values = if size == 0 {
            &[][..]
        } else {
            unsafe { std::slice::from_raw_parts(array, size as usize) }
        };
        state.pixels = values
            .iter()
            .flat_map(|value| value.to_ne_bytes())
            .collect::<Vec<_>>();
    }
    sync_pixels(image);
    image
}

#[no_mangle]
pub extern "C" fn vips_image_matrix_from_array(
    width: libc::c_int,
    height: libc::c_int,
    array: *const f64,
    size: libc::c_int,
) -> *mut VipsImage {
    vips_image_new_matrix_from_array(width, height, array, size)
}

#[no_mangle]
pub extern "C" fn safe_vips_image_new_from_source_internal(
    source: *mut VipsSource,
    option_string: *const c_char,
    _access: libc::c_int,
) -> *mut VipsImage {
    let image = vips_image_new();
    let out = crate::foreign::new_image_from_source(source, option_string, image);
    if out.is_null() {
        unsafe {
            crate::runtime::object::object_unref(image);
        }
    }
    out
}

#[no_mangle]
pub extern "C" fn safe_vips_image_write_to_target_internal(
    image: *mut VipsImage,
    suffix: *const c_char,
    target: *mut VipsTarget,
) -> libc::c_int {
    let suffix = if suffix.is_null() {
        ".png"
    } else {
        unsafe { CStr::from_ptr(suffix) }.to_str().unwrap_or(".png")
    };
    crate::foreign::write_image_to_target(image, suffix, target, ptr::null())
}

#[no_mangle]
pub extern "C" fn vips_image_set_delete_on_close(
    image: *mut VipsImage,
    delete_on_close: glib_sys::gboolean,
) {
    if let Some(image_ref) = unsafe { image.as_mut() } {
        image_ref.delete_on_close = delete_on_close;
    }
}

#[no_mangle]
pub extern "C" fn vips_image_write_to_memory(
    image: *mut VipsImage,
    size_out: *mut usize,
) -> *mut c_void {
    if ensure_pixels(image).is_err() {
        return ptr::null_mut();
    }
    let Some(state) = (unsafe { image_state(image) }) else {
        return ptr::null_mut();
    };
    unsafe {
        if !size_out.is_null() {
            *size_out = state.pixels.len();
        }
    }
    if state.pixels.is_empty() {
        return ptr::null_mut();
    }
    let out = unsafe { glib_sys::g_malloc(state.pixels.len()) };
    unsafe {
        ptr::copy_nonoverlapping(state.pixels.as_ptr(), out.cast::<u8>(), state.pixels.len());
    }
    out
}

#[no_mangle]
pub extern "C" fn vips_image_free_buffer(_image: *mut VipsImage, buffer: *mut c_void) {
    if !buffer.is_null() {
        unsafe {
            libc::free(buffer);
        }
    }
}

#[no_mangle]
pub extern "C" fn vips_image_write_prepare(image: *mut VipsImage) -> libc::c_int {
    let Some(image_ref) = (unsafe { image.as_mut() }) else {
        return -1;
    };
    let Some(state) = (unsafe { image_state(image) }) else {
        return -1;
    };
    let expected = image_size(image_ref);
    if state.pixels.len() != expected {
        state.pixels = vec![0; expected];
    }
    image_ref.dtype = VIPS_IMAGE_SETBUF;
    sync_pixels(image);
    vips_image_invalidate_all(image);
    0
}

#[no_mangle]
pub extern "C" fn vips_image_write_line(
    image: *mut VipsImage,
    ypos: libc::c_int,
    linebuffer: *mut u8,
) -> libc::c_int {
    if vips_image_write_prepare(image) != 0 {
        return -1;
    }
    let Some(image_ref) = (unsafe { image.as_ref() }) else {
        return -1;
    };
    let Some(state) = (unsafe { image_state(image) }) else {
        return -1;
    };
    let line = line_size(image_ref);
    let y = ypos.max(0) as usize;
    if y >= image_ref.Ysize.max(0) as usize {
        return -1;
    }
    let offset = y.saturating_mul(line);
    unsafe {
        ptr::copy_nonoverlapping(linebuffer, state.pixels[offset..].as_mut_ptr(), line);
    }
    sync_pixels(image);
    vips_image_invalidate_all(image);
    0
}

#[no_mangle]
pub extern "C" fn vips_image_copy_memory(image: *mut VipsImage) -> *mut VipsImage {
    if ensure_pixels(image).is_err() {
        return ptr::null_mut();
    }
    let Some(image_ref) = (unsafe { image.as_ref() }) else {
        return ptr::null_mut();
    };
    let out = vips_image_new_memory();
    crate::runtime::header::vips_image_init_fields(
        out,
        image_ref.Xsize,
        image_ref.Ysize,
        image_ref.Bands,
        image_ref.BandFmt,
        image_ref.Coding,
        image_ref.Type,
        image_ref.Xres,
        image_ref.Yres,
    );
    if let (Some(src), Some(dst)) = (unsafe { image_state(image) }, unsafe { image_state(out) }) {
        dst.pixels = src.pixels.clone();
    }
    sync_pixels(out);
    copy_metadata(out, image);
    out
}

#[no_mangle]
pub extern "C" fn vips_image_wio_input(image: *mut VipsImage) -> libc::c_int {
    if ensure_pixels(image).is_ok() {
        0
    } else {
        -1
    }
}

#[no_mangle]
pub extern "C" fn vips_image_pio_input(image: *mut VipsImage) -> libc::c_int {
    vips_image_wio_input(image)
}

#[no_mangle]
pub extern "C" fn vips_image_pio_output(image: *mut VipsImage) -> libc::c_int {
    vips_image_write_prepare(image)
}

#[no_mangle]
pub extern "C" fn vips_image_inplace(image: *mut VipsImage) -> libc::c_int {
    vips_image_wio_input(image)
}

#[no_mangle]
pub extern "C" fn vips_image_write(image: *mut VipsImage, out: *mut VipsImage) -> libc::c_int {
    if ensure_pixels(image).is_err() {
        return -1;
    }
    let Some(input) = (unsafe { image.as_ref() }) else {
        return -1;
    };
    if out.is_null() {
        return -1;
    }
    crate::runtime::header::vips_image_init_fields(
        out,
        input.Xsize,
        input.Ysize,
        input.Bands,
        input.BandFmt,
        input.Coding,
        input.Type,
        input.Xres,
        input.Yres,
    );
    if let (Some(src), Some(dst)) = (unsafe { image_state(image) }, unsafe { image_state(out) }) {
        dst.pixels = src.pixels.clone();
        dst.pending_load = None;
    } else {
        return -1;
    }
    sync_pixels(out);
    copy_metadata(out, image);
    0
}

#[no_mangle]
pub extern "C" fn vips_image_isfile(image: *mut VipsImage) -> glib_sys::gboolean {
    if unsafe { image_state(image) }.is_some_and(|state| state.filename.is_some()) {
        glib_sys::GTRUE
    } else {
        glib_sys::GFALSE
    }
}

#[no_mangle]
pub extern "C" fn vips_image_ispartial(image: *mut VipsImage) -> glib_sys::gboolean {
    if unsafe { image.as_ref() }.is_some_and(|image| image.dtype == VIPS_IMAGE_PARTIAL) {
        glib_sys::GTRUE
    } else {
        glib_sys::GFALSE
    }
}

#[no_mangle]
pub extern "C" fn vips_image_hasalpha(image: *mut VipsImage) -> glib_sys::gboolean {
    let has_alpha = unsafe { image.as_ref() }.is_some_and(|image| match image.Type {
        VIPS_INTERPRETATION_B_W | VIPS_INTERPRETATION_GREY16 => image.Bands > 1,
        VIPS_INTERPRETATION_RGB
        | VIPS_INTERPRETATION_CMC
        | VIPS_INTERPRETATION_LCH
        | VIPS_INTERPRETATION_LABS
        | VIPS_INTERPRETATION_sRGB
        | VIPS_INTERPRETATION_YXY
        | VIPS_INTERPRETATION_XYZ
        | VIPS_INTERPRETATION_LAB
        | VIPS_INTERPRETATION_RGB16
        | VIPS_INTERPRETATION_scRGB
        | VIPS_INTERPRETATION_HSV => image.Bands > 3,
        VIPS_INTERPRETATION_CMYK => image.Bands > 4,
        _ => false,
    });
    if has_alpha {
        glib_sys::GTRUE
    } else {
        glib_sys::GFALSE
    }
}

#[no_mangle]
pub extern "C" fn vips_image_isMSBfirst(_image: *mut VipsImage) -> glib_sys::gboolean {
    const VIPS_MAGIC_SPARC: u32 = 0x08f2a6b6;

    if unsafe { _image.as_ref() }.is_some_and(|image| image.magic == VIPS_MAGIC_SPARC) {
        glib_sys::GTRUE
    } else {
        glib_sys::GFALSE
    }
}

#[no_mangle]
pub extern "C" fn vips_image_minimise_all(_image: *mut VipsImage) {}

#[no_mangle]
pub extern "C" fn vips_image_invalidate_all(image: *mut VipsImage) {
    if let Some(image_ref) = unsafe { image.as_mut() } {
        image_ref.serial = image_ref.serial.wrapping_add(1);
    }
    crate::runtime::cache::vips_cache_drop_all();
}

#[no_mangle]
pub extern "C" fn vips_image_is_sequential(_image: *mut VipsImage) -> glib_sys::gboolean {
    glib_sys::GFALSE
}

#[no_mangle]
pub extern "C" fn vips_image_set_progress(image: *mut VipsImage, progress: glib_sys::gboolean) {
    let Some(image_ref) = (unsafe { image.as_mut() }) else {
        return;
    };

    if progress != glib_sys::GFALSE {
        if image_ref.progress_signal.is_null() {
            image_ref.progress_signal = image;
        }
    } else {
        image_ref.progress_signal = ptr::null_mut();
    }
}
#[no_mangle]
pub extern "C" fn vips_image_preeval(image: *mut VipsImage) {
    if !has_progress_signal(image) {
        return;
    }

    let signal_image = progress_target(image);
    let Some(progress) = ensure_progress_struct(image) else {
        return;
    };
    if let Some(progress_ref) = unsafe { progress.as_mut() } {
        if !progress_ref.start.is_null() {
            unsafe {
                glib_sys::g_timer_start(progress_ref.start);
            }
        }
        progress_ref.run = 0;
        progress_ref.eta = 0;
        progress_ref.npels = 0;
        progress_ref.percent = 0;
    }

    if signal_image != image {
        if let Some(signal_progress) = ensure_progress_struct(signal_image) {
            update_progress_struct(signal_progress, 0);
        }
    }

    if emit_progress_visible(image) {
        emit_progress_signal(
            signal_image,
            crate::runtime::object::vips_image_preeval_signal_id(),
            progress,
        );
    }
}

#[no_mangle]
pub extern "C" fn vips_image_eval(image: *mut VipsImage, processed: u64) {
    if !has_progress_signal(image) {
        return;
    }

    let signal_image = progress_target(image);
    let Some(progress) = current_progress_struct(image).or_else(|| ensure_progress_struct(image))
    else {
        return;
    };
    update_progress_struct(progress, processed);
    if signal_image != image {
        if let Some(signal_progress) =
            current_progress_struct(signal_image).or_else(|| ensure_progress_struct(signal_image))
        {
            update_progress_struct(signal_progress, processed);
        }
    }

    if emit_progress_visible(image) {
        emit_progress_signal(
            signal_image,
            crate::runtime::object::vips_image_eval_signal_id(),
            progress,
        );
    }
}

#[no_mangle]
pub extern "C" fn vips_image_posteval(image: *mut VipsImage) {
    if !has_progress_signal(image) {
        return;
    }

    let signal_image = progress_target(image);
    let Some(progress) = current_progress_struct(image).or_else(|| ensure_progress_struct(image))
    else {
        return;
    };
    let total = unsafe { progress.as_ref() }
        .map(|progress| progress.tpels.max(1) as u64)
        .unwrap_or(1);
    update_progress_struct(progress, total);
    if signal_image != image {
        if let Some(signal_progress) =
            current_progress_struct(signal_image).or_else(|| ensure_progress_struct(signal_image))
        {
            update_progress_struct(signal_progress, total);
        }
    }

    if emit_progress_visible(image) {
        emit_progress_signal(
            signal_image,
            crate::runtime::object::vips_image_posteval_signal_id(),
            progress,
        );
    }
}

#[no_mangle]
pub extern "C" fn vips_image_iskilled(image: *mut VipsImage) -> glib_sys::gboolean {
    let Some(image_ref) = (unsafe { image.as_mut() }) else {
        return glib_sys::GFALSE;
    };
    let kill = image_ref.kill != 0;
    if kill {
        let filename = if image_ref.filename.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(image_ref.filename) }
                .to_string_lossy()
                .into_owned()
        };
        append_message_str("VipsImage", &format!("killed for image \"{filename}\""));
        image_ref.kill = 0;
        glib_sys::GTRUE
    } else {
        glib_sys::GFALSE
    }
}

#[no_mangle]
pub extern "C" fn vips_image_set_kill(image: *mut VipsImage, kill: glib_sys::gboolean) {
    if let Some(image) = unsafe { image.as_mut() } {
        image.kill = if kill == glib_sys::GFALSE { 0 } else { 1 };
    }
}

pub(crate) fn simulate_save_progress<T, F>(image: *mut VipsImage, work: F) -> Result<T, ()>
where
    F: FnOnce() -> Result<T, ()>,
{
    if !has_progress_signal(image) {
        return work();
    }

    vips_image_preeval(image);
    let total = unsafe { current_progress_struct(image).and_then(|progress| progress.as_ref()) }
        .map(|progress| progress.tpels.max(1) as u64)
        .unwrap_or_else(|| unsafe { image.as_ref().map(image_progress_pixels).unwrap_or(1) });
    let checkpoint = ((total.saturating_mul(94)) / 100).clamp(1, total);
    vips_image_eval(image, checkpoint);

    let signal_image = progress_target(image);
    let result = if vips_image_iskilled(signal_image) != glib_sys::GFALSE {
        Err(())
    } else {
        work()
    };

    vips_image_posteval(image);
    if result.is_ok() && vips_image_iskilled(signal_image) != glib_sys::GFALSE {
        Err(())
    } else {
        result
    }
}

#[no_mangle]
pub extern "C" fn vips_band_format_isint(format: VipsBandFormat) -> glib_sys::gboolean {
    if matches!(
        format,
        VIPS_FORMAT_CHAR | VIPS_FORMAT_SHORT | VIPS_FORMAT_INT
    ) {
        glib_sys::GTRUE
    } else {
        glib_sys::GFALSE
    }
}

#[no_mangle]
pub extern "C" fn vips_band_format_isuint(format: VipsBandFormat) -> glib_sys::gboolean {
    if matches!(
        format,
        VIPS_FORMAT_UCHAR | VIPS_FORMAT_USHORT | VIPS_FORMAT_UINT
    ) {
        glib_sys::GTRUE
    } else {
        glib_sys::GFALSE
    }
}

#[no_mangle]
pub extern "C" fn vips_band_format_is8bit(format: VipsBandFormat) -> glib_sys::gboolean {
    if matches!(format, VIPS_FORMAT_UCHAR | VIPS_FORMAT_CHAR) {
        glib_sys::GTRUE
    } else {
        glib_sys::GFALSE
    }
}

#[no_mangle]
pub extern "C" fn vips_band_format_isfloat(format: VipsBandFormat) -> glib_sys::gboolean {
    if matches!(format, VIPS_FORMAT_FLOAT | VIPS_FORMAT_DOUBLE) {
        glib_sys::GTRUE
    } else {
        glib_sys::GFALSE
    }
}

#[no_mangle]
pub extern "C" fn vips_band_format_iscomplex(format: VipsBandFormat) -> glib_sys::gboolean {
    if matches!(format, VIPS_FORMAT_COMPLEX | VIPS_FORMAT_DPCOMPLEX) {
        glib_sys::GTRUE
    } else {
        glib_sys::GFALSE
    }
}

#[no_mangle]
pub extern "C" fn safe_vips_crop_internal(
    input: *mut VipsImage,
    out: *mut *mut VipsImage,
    left: libc::c_int,
    top: libc::c_int,
    width: libc::c_int,
    height: libc::c_int,
) -> libc::c_int {
    if !out.is_null() {
        unsafe {
            *out = std::ptr::null_mut();
        }
    }
    let bad_extract_area = || {
        append_message_str("extract_area", "bad extract area");
        -1
    };
    if ensure_pixels(input).is_err() || out.is_null() {
        return bad_extract_area();
    }
    let Some(input_ref) = (unsafe { input.as_ref() }) else {
        return bad_extract_area();
    };
    if left < 0 || top < 0 || width <= 0 || height <= 0 {
        return bad_extract_area();
    }
    let Ok(left_u) = usize::try_from(left) else {
        return bad_extract_area();
    };
    let Ok(top_u) = usize::try_from(top) else {
        return bad_extract_area();
    };
    let Ok(width_u) = usize::try_from(width) else {
        return bad_extract_area();
    };
    let Ok(height_u) = usize::try_from(height) else {
        return bad_extract_area();
    };
    let Some(right_u) = left_u.checked_add(width_u) else {
        return bad_extract_area();
    };
    let Some(bottom_u) = top_u.checked_add(height_u) else {
        return bad_extract_area();
    };
    if right_u > input_ref.Xsize.max(0) as usize || bottom_u > input_ref.Ysize.max(0) as usize {
        return bad_extract_area();
    }
    let Some(state) = (unsafe { image_state(input) }) else {
        return bad_extract_area();
    };
    let bpp = bytes_per_pixel(input_ref);
    let line = line_size(input_ref);
    let crop = vips_image_new_memory();
    crate::runtime::header::vips_image_init_fields(
        crop,
        width,
        height,
        input_ref.Bands,
        input_ref.BandFmt,
        input_ref.Coding,
        input_ref.Type,
        input_ref.Xres,
        input_ref.Yres,
    );
    if let Some(crop_ref) = unsafe { crop.as_mut() } {
        crop_ref.Xoffset = -left;
        crop_ref.Yoffset = -top;
        crop_ref.dhint = crate::abi::image::VIPS_DEMAND_STYLE_THINSTRIP;
        crop_ref.hint_set = glib_sys::GTRUE;
    }
    if let Some(out_state) = unsafe { image_state(crop) } {
        out_state.pixels = vec![0; width_u.saturating_mul(height_u).saturating_mul(bpp)];
        for y in 0..height_u {
            let src_y = top_u + y;
            let src_x = left_u;
            let src_offset = src_y
                .saturating_mul(line)
                .saturating_add(src_x.saturating_mul(bpp));
            let dst_offset = y.saturating_mul(width_u).saturating_mul(bpp);
            let count = width_u * bpp;
            if src_offset + count <= state.pixels.len()
                && dst_offset + count <= out_state.pixels.len()
            {
                out_state.pixels[dst_offset..dst_offset + count]
                    .copy_from_slice(&state.pixels[src_offset..src_offset + count]);
            }
        }
    }
    sync_pixels(crop);
    copy_metadata(crop, input);
    unsafe {
        *out = crop;
    }
    0
}

#[no_mangle]
pub extern "C" fn safe_vips_avg_internal(image: *mut VipsImage, out: *mut f64) -> libc::c_int {
    if ensure_pixels(image).is_err() || out.is_null() {
        return -1;
    }
    let Some(image_ref) = (unsafe { image.as_ref() }) else {
        return -1;
    };
    let Some(state) = (unsafe { image_state(image) }) else {
        return -1;
    };
    let avg = match image_ref.BandFmt {
        VIPS_FORMAT_UCHAR => {
            let sum: u64 = state.pixels.iter().map(|value| *value as u64).sum();
            sum as f64 / state.pixels.len().max(1) as f64
        }
        VIPS_FORMAT_USHORT => {
            let mut sum = 0f64;
            for chunk in state.pixels.chunks_exact(2) {
                sum += u16::from_be_bytes([chunk[0], chunk[1]]) as f64;
            }
            sum / (state.pixels.len() / 2).max(1) as f64
        }
        _ => return -1,
    };
    unsafe {
        *out = avg;
    }
    0
}
