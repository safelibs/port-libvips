use std::ffi::{c_void, CStr, CString};
use std::io::Cursor;
use std::os::raw::c_char;
use std::ptr;
use std::sync::Mutex;

use crate::abi::connection::{VipsSource, VipsTarget};
use crate::abi::image::*;
use crate::runtime::error::append_message_str;
use crate::runtime::header::{copy_metadata, MetaStore};
use crate::runtime::object::{get_qdata_ptr, object_new, object_ref, qdata_quark, set_qdata_box};

static IMAGE_STATE_QUARK: &CStr = c"safe-vips-image-state";

pub(crate) struct ImageState {
    pub pixels: Vec<u8>,
    pub filename: Option<CString>,
    pub mode: Option<CString>,
    pub history: Option<CString>,
    pub meta: Mutex<MetaStore>,
    pub source: Option<*mut VipsSource>,
}

impl Drop for ImageState {
    fn drop(&mut self) {
        if let Some(source) = self.source.take() {
            unsafe {
                crate::runtime::object::object_unref(source);
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
                source: None,
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

fn decode_png(
    bytes: &[u8],
) -> Result<(Vec<u8>, u32, u32, i32, VipsBandFormat, VipsInterpretation), String> {
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
    Ok((
        out,
        info.width,
        info.height,
        bands,
        band_format,
        interpretation,
    ))
}

fn encode_png(image: &VipsImage, pixels: &[u8]) -> Result<Vec<u8>, String> {
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
    writer
        .write_image_data(pixels)
        .map_err(|err| err.to_string())?;
    drop(writer);
    Ok(bytes)
}

fn read_source_bytes(source: *mut VipsSource) -> Result<Vec<u8>, ()> {
    let mapped = crate::runtime::source::vips_source_map(source, ptr::null_mut());
    if !mapped.is_null() {
        let length = crate::runtime::source::vips_source_length(source);
        if length < 0 {
            return Err(());
        }
        let bytes = unsafe { std::slice::from_raw_parts(mapped.cast::<u8>(), length as usize) };
        return Ok(bytes.to_vec());
    }

    let mut bytes = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = crate::runtime::source::vips_source_read(
            source,
            buffer.as_mut_ptr().cast::<c_void>(),
            buffer.len(),
        );
        if read < 0 {
            return Err(());
        }
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read as usize]);
    }
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
pub extern "C" fn safe_vips_image_new_from_source_internal(
    source: *mut VipsSource,
    _option_string: *const c_char,
    _access: libc::c_int,
) -> *mut VipsImage {
    let Ok(bytes) = read_source_bytes(source) else {
        return ptr::null_mut();
    };
    let Ok((pixels, width, height, bands, format, interpretation)) = decode_png(&bytes) else {
        append_message_str("vips_image_new_from_source", "unsupported input format");
        return ptr::null_mut();
    };

    let image = vips_image_new_memory();
    crate::runtime::header::vips_image_init_fields(
        image,
        width as libc::c_int,
        height as libc::c_int,
        bands,
        format,
        VIPS_CODING_NONE,
        interpretation,
        1.0,
        1.0,
    );
    if let Some(state) = unsafe { image_state(image) } {
        state.pixels = pixels;
        state.source = Some(unsafe { object_ref(source) });
    }
    sync_pixels(image);
    if let Some(source_ref) = unsafe { source.as_ref() } {
        if !source_ref.parent_object.filename.is_null() {
            set_filename(
                image,
                Some(unsafe { CStr::from_ptr(source_ref.parent_object.filename) }),
            );
        }
    }
    image
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
    if suffix != ".png" && suffix != "png" {
        append_message_str("vips_image_write_to_target", "only .png is supported");
        return -1;
    }
    if ensure_pixels(image).is_err() {
        return -1;
    }
    let Some(image_ref) = (unsafe { image.as_ref() }) else {
        return -1;
    };
    let Some(state) = (unsafe { image_state(image) }) else {
        return -1;
    };
    let Ok(encoded) = encode_png(image_ref, &state.pixels) else {
        append_message_str("vips_image_write_to_target", "png encode failed");
        return -1;
    };
    if crate::runtime::target::vips_target_write(
        target,
        encoded.as_ptr().cast::<c_void>(),
        encoded.len(),
    ) != 0
    {
        return -1;
    }
    crate::runtime::target::vips_target_end(target)
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
    if unsafe { image.as_ref() }.is_some_and(|image| image.Bands == 2 || image.Bands == 4) {
        glib_sys::GTRUE
    } else {
        glib_sys::GFALSE
    }
}

#[no_mangle]
pub extern "C" fn vips_image_isMSBfirst(_image: *mut VipsImage) -> glib_sys::gboolean {
    glib_sys::GFALSE
}

#[no_mangle]
pub extern "C" fn vips_image_minimise_all(_image: *mut VipsImage) {}

#[no_mangle]
pub extern "C" fn vips_image_invalidate_all(_image: *mut VipsImage) {}

#[no_mangle]
pub extern "C" fn vips_image_is_sequential(_image: *mut VipsImage) -> glib_sys::gboolean {
    glib_sys::GFALSE
}

#[no_mangle]
pub extern "C" fn vips_image_set_progress(_image: *mut VipsImage, _progress: glib_sys::gboolean) {}

#[no_mangle]
pub extern "C" fn vips_image_iskilled(image: *mut VipsImage) -> glib_sys::gboolean {
    unsafe { image.as_ref() }.map_or(glib_sys::GFALSE, |image| {
        if image.kill != 0 {
            glib_sys::GTRUE
        } else {
            glib_sys::GFALSE
        }
    })
}

#[no_mangle]
pub extern "C" fn vips_image_set_kill(image: *mut VipsImage, kill: glib_sys::gboolean) {
    if let Some(image) = unsafe { image.as_mut() } {
        image.kill = if kill == glib_sys::GFALSE { 0 } else { 1 };
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
    if ensure_pixels(input).is_err() || out.is_null() {
        return -1;
    }
    let Some(input_ref) = (unsafe { input.as_ref() }) else {
        return -1;
    };
    let Some(state) = (unsafe { image_state(input) }) else {
        return -1;
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
    if let Some(out_state) = unsafe { image_state(crop) } {
        out_state.pixels = vec![
            0;
            (width.max(0) as usize)
                .saturating_mul(height.max(0) as usize)
                .saturating_mul(bpp)
        ];
        for y in 0..height.max(0) as usize {
            let src_y = top.max(0) as usize + y;
            let src_x = left.max(0) as usize;
            let src_offset = src_y
                .saturating_mul(line)
                .saturating_add(src_x.saturating_mul(bpp));
            let dst_offset = y.saturating_mul(width.max(0) as usize).saturating_mul(bpp);
            let count = width.max(0) as usize * bpp;
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
