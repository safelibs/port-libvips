pub mod base;
pub mod loaders;
pub mod metadata;
pub mod modules;
pub mod savers;
pub mod sniff;

use std::collections::HashMap;
use std::ffi::{c_void, CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::sync::{Mutex, OnceLock};

use crate::abi::connection::{VipsSource, VipsTarget};
use crate::abi::image::{VIPS_INTERPRETATION_sRGB, VipsImage, VIPS_INTERPRETATION_B_W};
use crate::foreign::base::{
    buffer_save_name, build_load_result, file_save_name, loader_name, options_from_map,
    parse_option_string, save_options_from_map, target_save_name, ForeignKind, ForeignLoadResult,
    InputKind, LoadOptions, SaveOptions,
};
use crate::runtime::error::append_message_str;
use crate::runtime::image::{ensure_pixels, image_state, set_filename, set_history, sync_pixels};
use crate::runtime::object;
use crate::runtime::source::read_all_bytes;
use crate::runtime::target::{vips_target_end, vips_target_write};
use crate::runtime::vips_native::read_all_from_path;

fn file_load_cache() -> &'static Mutex<HashMap<String, ForeignLoadResult>> {
    static CACHE: OnceLock<Mutex<HashMap<String, ForeignLoadResult>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn file_load_cache_key(kind: ForeignKind, filename: &str, options: &LoadOptions) -> String {
    format!(
        "{kind:?}\n{filename}\naccess={:?}\nautorotate={}\ndpi={:?}\nfail_on={:?}\nmemory={}\nn={:?}\npage={:?}\nscale={:?}\nunlimited={}",
        options.access,
        options.autorotate,
        options.dpi.map(f64::to_bits),
        options.fail_on,
        options.memory,
        options.n,
        options.page,
        options.scale.map(f64::to_bits),
        options.unlimited,
    )
}

fn lookup_cached_file_load(key: &str) -> Option<ForeignLoadResult> {
    file_load_cache()
        .lock()
        .expect("file load cache")
        .get(key)
        .cloned()
}

fn store_cached_file_load(key: String, result: ForeignLoadResult) {
    file_load_cache()
        .lock()
        .expect("file load cache")
        .insert(key, result);
}

fn remove_cached_file_load(key: &str) {
    let _ = file_load_cache()
        .lock()
        .expect("file load cache")
        .remove(key);
}

fn options_from_cstr(option_string: *const c_char) -> LoadOptions {
    if option_string.is_null() {
        return LoadOptions::default();
    }
    let text = unsafe { CStr::from_ptr(option_string) }
        .to_str()
        .unwrap_or_default();
    options_from_map(&parse_option_string(text))
}

fn kind_from_source_bytes(bytes: &[u8], filename_hint: Option<&str>) -> ForeignKind {
    sniff::kind_from_bytes(bytes, filename_hint)
}

fn load_from_kind(
    bytes: &[u8],
    kind: ForeignKind,
    input_kind: InputKind,
    options: LoadOptions,
) -> Result<ForeignLoadResult, ()> {
    match kind {
        ForeignKind::Jpeg => parse_jpeg_header(bytes, options),
        ForeignKind::Png => parse_png(bytes),
        ForeignKind::Ppm | ForeignKind::Pfm => parse_ppm(bytes),
        ForeignKind::Csv => parse_csv(bytes, &options),
        ForeignKind::Matrix => parse_matrix(bytes),
        ForeignKind::Vips => {
            if loaders::container::is_container(bytes) {
                loaders::container::parse_container(bytes, options)
            } else {
                let _ = options;
                loaders::legacy_vips::parse_bytes(bytes)
            }
        }
        ForeignKind::Gif
        | ForeignKind::Tiff
        | ForeignKind::Webp
        | ForeignKind::Heif
        | ForeignKind::Svg
        | ForeignKind::Pdf
        | ForeignKind::Radiance => loaders::external::decode_with_convert(bytes, kind, &options),
        ForeignKind::Unknown | ForeignKind::Raw => {
            append_message_str(
                "foreign",
                &format!(
                    "unsupported {} input",
                    match input_kind {
                        InputKind::File => "file",
                        InputKind::Buffer => "buffer",
                        InputKind::Source => "source",
                    }
                ),
            );
            Err(())
        }
    }
}

fn load_from_bytes_for_kind(
    bytes: &[u8],
    kind: ForeignKind,
    input_kind: InputKind,
    options: LoadOptions,
) -> Result<ForeignLoadResult, ()> {
    if loaders::container::is_container(bytes) {
        return loaders::container::parse_container(bytes, options);
    }
    load_from_kind(bytes, kind, input_kind, options)
}

fn parse_jpeg_header(bytes: &[u8], options: LoadOptions) -> Result<ForeignLoadResult, ()> {
    if bytes.len() < 4 || !bytes.starts_with(&[0xff, 0xd8]) {
        append_message_str("jpegload", "invalid jpeg stream");
        return Err(());
    }

    let mut offset = 2usize;
    while offset + 4 <= bytes.len() {
        if bytes[offset] != 0xff {
            append_message_str("jpegload", "invalid jpeg marker");
            return Err(());
        }
        let marker = bytes[offset + 1];
        offset += 2;
        if marker == 0xd9 || marker == 0xda {
            break;
        }
        let segment_len = u16::from_be_bytes([bytes[offset], bytes[offset + 1]]) as usize;
        offset += 2;
        if segment_len < 2 || offset + segment_len - 2 > bytes.len() {
            append_message_str("jpegload", "truncated jpeg header");
            return Err(());
        }
        if matches!(
            marker,
            0xc0 | 0xc1
                | 0xc2
                | 0xc3
                | 0xc5
                | 0xc6
                | 0xc7
                | 0xc9
                | 0xca
                | 0xcb
                | 0xcd
                | 0xce
                | 0xcf
        ) {
            let precision = bytes[offset];
            let height = u16::from_be_bytes([bytes[offset + 1], bytes[offset + 2]]) as i32;
            let width = u16::from_be_bytes([bytes[offset + 3], bytes[offset + 4]]) as i32;
            let bands = bytes[offset + 5] as i32;
            let interpretation = if bands == 1 {
                VIPS_INTERPRETATION_B_W
            } else {
                VIPS_INTERPRETATION_sRGB
            };
            let metadata =
                metadata::extract_jpeg_metadata(bytes).with_string("vips-loader", "jpegload");
            return Ok(build_load_result(
                width,
                height,
                bands,
                if precision > 8 {
                    crate::abi::image::VIPS_FORMAT_USHORT
                } else {
                    crate::abi::image::VIPS_FORMAT_UCHAR
                },
                interpretation,
                "jpegload",
                None,
                metadata,
                Some(base::PendingDecode {
                    bytes: bytes.to_vec(),
                    kind: ForeignKind::Jpeg,
                    options,
                }),
            ));
        }
        offset += segment_len - 2;
    }

    append_message_str("jpegload", "missing jpeg frame header");
    Err(())
}

fn parse_png(bytes: &[u8]) -> Result<ForeignLoadResult, ()> {
    let (pixels, width, height, bands, band_format, interpretation, bits_per_sample) =
        crate::runtime::image::safe_decode_png_bytes(bytes).map_err(|message| {
            append_message_str("pngload", &message);
        })?;
    Ok(build_load_result(
        width as i32,
        height as i32,
        bands,
        band_format,
        interpretation,
        "pngload",
        Some(pixels),
        base::ForeignMetadata::default()
            .with_string("vips-loader", "pngload")
            .with_int("bits-per-sample", bits_per_sample as i32),
        None,
    ))
}

fn skip_ppm_space(bytes: &[u8], offset: &mut usize) {
    loop {
        while *offset < bytes.len() && bytes[*offset].is_ascii_whitespace() {
            *offset += 1;
        }
        if *offset < bytes.len() && bytes[*offset] == b'#' {
            while *offset < bytes.len() && bytes[*offset] != b'\n' {
                *offset += 1;
            }
            continue;
        }
        break;
    }
}

fn read_ppm_token(bytes: &[u8], offset: &mut usize) -> Result<String, ()> {
    skip_ppm_space(bytes, offset);
    let start = *offset;
    while *offset < bytes.len() && !bytes[*offset].is_ascii_whitespace() {
        *offset += 1;
    }
    if start == *offset {
        return Err(());
    }
    Ok(String::from_utf8_lossy(&bytes[start..*offset]).into_owned())
}

fn parse_ppm(bytes: &[u8]) -> Result<ForeignLoadResult, ()> {
    if bytes.len() < 3 {
        append_message_str("ppmload", "truncated ppm header");
        return Err(());
    }
    let magic = &bytes[..2];
    let pfm = magic == b"PF" || magic == b"Pf";
    let mut offset = 2usize;
    let width = read_ppm_token(bytes, &mut offset)
        .ok()
        .and_then(|value| value.parse::<i32>().ok())
        .ok_or_else(|| {
            append_message_str("ppmload", "invalid width");
        })?;
    let height = read_ppm_token(bytes, &mut offset)
        .ok()
        .and_then(|value| value.parse::<i32>().ok())
        .ok_or_else(|| {
            append_message_str("ppmload", "invalid height");
        })?;
    let scale = read_ppm_token(bytes, &mut offset).map_err(|_| {
        append_message_str("ppmload", "invalid ppm scale");
    })?;
    skip_ppm_space(bytes, &mut offset);
    let payload = &bytes[offset..];

    if pfm {
        let bands = if magic == b"PF" { 3 } else { 1 };
        let expected = width.max(0) as usize * height.max(0) as usize * bands as usize * 4;
        if payload.len() != expected {
            append_message_str("ppmload", "pfm payload length mismatch");
            return Err(());
        }
        let mut pixels = payload.to_vec();
        if scale.starts_with('-') {
            for chunk in pixels.chunks_exact_mut(4) {
                let value = f32::from_le_bytes(chunk.try_into().unwrap());
                chunk.copy_from_slice(&value.to_ne_bytes());
            }
        } else {
            for chunk in pixels.chunks_exact_mut(4) {
                let value = f32::from_be_bytes(chunk.try_into().unwrap());
                chunk.copy_from_slice(&value.to_ne_bytes());
            }
        }
        return Ok(build_load_result(
            width,
            height,
            bands,
            crate::abi::image::VIPS_FORMAT_FLOAT,
            if bands == 1 {
                crate::abi::image::VIPS_INTERPRETATION_B_W
            } else {
                crate::abi::image::VIPS_INTERPRETATION_sRGB
            },
            "ppmload",
            Some(pixels),
            base::ForeignMetadata::default().with_string("vips-loader", "ppmload"),
            None,
        ));
    }

    let maxval = scale.parse::<u32>().unwrap_or(255);
    let bands = if magic == b"P6" { 3 } else { 1 };
    if maxval > 255 {
        append_message_str("ppmload", "16-bit ppm is not supported");
        return Err(());
    }
    let expected = width.max(0) as usize * height.max(0) as usize * bands as usize;
    if payload.len() != expected {
        append_message_str("ppmload", "ppm payload length mismatch");
        return Err(());
    }
    Ok(build_load_result(
        width,
        height,
        bands,
        crate::abi::image::VIPS_FORMAT_UCHAR,
        if bands == 1 {
            crate::abi::image::VIPS_INTERPRETATION_B_W
        } else {
            crate::abi::image::VIPS_INTERPRETATION_sRGB
        },
        "ppmload",
        Some(payload.to_vec()),
        base::ForeignMetadata::default().with_string("vips-loader", "ppmload"),
        None,
    ))
}

fn fail_on_level(options: &LoadOptions) -> i32 {
    options
        .fail_on
        .as_deref()
        .map(|value| value.to_ascii_lowercase())
        .and_then(|value| match value.as_str() {
            "none" => Some(0),
            "truncated" => Some(1),
            "error" => Some(2),
            "warning" => Some(3),
            _ => value.parse::<i32>().ok(),
        })
        .unwrap_or(0)
}

fn parse_csv(bytes: &[u8], options: &LoadOptions) -> Result<ForeignLoadResult, ()> {
    let text = std::str::from_utf8(bytes).map_err(|_| {
        append_message_str("csvload", "csv input is not valid utf-8");
    })?;
    let fail_on = fail_on_level(options);
    let lines = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    let width = lines
        .first()
        .map(|line| line.split(',').count())
        .unwrap_or_default() as i32;
    if width == 0 {
        append_message_str("csvload", "csv parse failed");
        return Err(());
    }

    let truncated_input = !bytes.is_empty() && !bytes.ends_with(b"\n");
    let mut saw_truncation = false;
    let mut saw_ragged = false;
    let mut rows = Vec::with_capacity(lines.len());

    for (index, line) in lines.iter().enumerate() {
        let last_line = index + 1 == lines.len();
        let mut row = line
            .split(',')
            .map(|field| {
                let field = field.trim();
                if field.is_empty() {
                    return Ok(0);
                }
                field.parse::<u8>().map_err(|_| ())
            })
            .collect::<Result<Vec<_>, _>>()
            .unwrap_or_else(|_| {
                if last_line && truncated_input {
                    saw_truncation = true;
                } else {
                    saw_ragged = true;
                }
                Vec::new()
            });

        if row.len() < width as usize {
            if last_line && truncated_input {
                saw_truncation = true;
            } else {
                saw_ragged = true;
            }
            row.resize(width as usize, 0);
        } else if row.len() > width as usize {
            saw_ragged = true;
            row.truncate(width as usize);
        }

        rows.push(row);
    }

    if saw_truncation && fail_on >= 1 {
        append_message_str("csvload", "csv parse failed");
        return Err(());
    }
    if saw_ragged && fail_on >= 2 {
        append_message_str("csvload", "ragged csv input");
        return Err(());
    }

    let mut pixels = Vec::with_capacity(rows.len() * width as usize);
    for row in rows {
        pixels.extend_from_slice(&row);
    }
    Ok(build_load_result(
        width,
        (pixels.len() / width as usize) as i32,
        1,
        crate::abi::image::VIPS_FORMAT_UCHAR,
        crate::abi::image::VIPS_INTERPRETATION_B_W,
        "csvload",
        Some(pixels),
        base::ForeignMetadata::default().with_string("vips-loader", "csvload"),
        None,
    ))
}

fn parse_matrix(bytes: &[u8]) -> Result<ForeignLoadResult, ()> {
    let text = std::str::from_utf8(bytes).map_err(|_| {
        append_message_str("matrixload", "matrix input is not valid utf-8");
    })?;
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let header = lines.next().ok_or_else(|| {
        append_message_str("matrixload", "matrix header missing");
    })?;
    let header = header.split_whitespace().collect::<Vec<_>>();
    if header.len() < 4 {
        append_message_str(
            "matrixload",
            "matrix header requires width height scale offset",
        );
        return Err(());
    }
    let width = header[0].parse::<usize>().map_err(|_| {
        append_message_str("matrixload", "invalid matrix width");
    })?;
    let height = header[1].parse::<usize>().map_err(|_| {
        append_message_str("matrixload", "invalid matrix height");
    })?;
    if width == 0 || height == 0 {
        append_message_str("matrixload", "matrix dimensions must be positive");
        return Err(());
    }
    let expected = width.checked_mul(height).ok_or_else(|| {
        append_message_str("matrixload", "matrix dimensions overflow");
    })?;
    let mut values = Vec::with_capacity(expected);
    for line in lines {
        for item in line.split_whitespace() {
            values.push(item.parse::<f32>().map_err(|_| {
                append_message_str("matrixload", "invalid matrix cell");
            })?);
        }
    }
    if values.len() != expected {
        append_message_str("matrixload", "matrix payload length mismatch");
        return Err(());
    }
    let mut pixels = Vec::with_capacity(expected * 4);
    for value in values {
        pixels.extend_from_slice(&value.to_ne_bytes());
    }
    Ok(build_load_result(
        width as i32,
        height as i32,
        1,
        crate::abi::image::VIPS_FORMAT_FLOAT,
        crate::abi::image::VIPS_INTERPRETATION_MATRIX,
        "matrixload",
        Some(pixels),
        base::ForeignMetadata::default().with_string("vips-loader", "matrixload"),
        None,
    ))
}

fn load_from_bytes_inner(
    bytes: &[u8],
    filename_hint: Option<&str>,
    input_kind: InputKind,
    options: LoadOptions,
) -> Result<ForeignLoadResult, ()> {
    if loaders::container::is_container(bytes) {
        return loaders::container::parse_container(bytes, options);
    }
    let kind = kind_from_source_bytes(bytes, filename_hint);
    load_from_kind(bytes, kind, input_kind, options)
}

fn apply_load_result(
    image: *mut VipsImage,
    result: ForeignLoadResult,
    filename: Option<&CStr>,
) -> *mut VipsImage {
    crate::runtime::header::vips_image_init_fields(
        image,
        result.width,
        result.height,
        result.bands,
        result.band_format,
        result.coding,
        result.interpretation,
        1.0,
        1.0,
    );
    if let Some(state) = unsafe { image_state(image) } {
        state.pixels = result.pixels.unwrap_or_default();
        state.pending_load = result.pending;
        state.source = None;
    }
    sync_pixels(image);
    if let Some(filename) = filename {
        set_filename(image, Some(filename));
    }
    set_history(image, result.history.as_deref());
    metadata::install_metadata(image, result.loader_name, &result.metadata);
    image
}

fn load_vips_file_into_image(
    image: *mut VipsImage,
    filename: &CStr,
    bytes: &[u8],
    options: LoadOptions,
) -> *mut VipsImage {
    if loaders::container::is_container(bytes) {
        match loaders::container::parse_container(bytes, options) {
            Ok(result) => apply_load_result(image, result, Some(filename)),
            Err(()) => ptr::null_mut(),
        }
    } else {
        loaders::legacy_vips::load_file_into_image(filename, image)
    }
}

fn load_vips_source_into_image(
    image: *mut VipsImage,
    source: *mut VipsSource,
    filename_hint: Option<&str>,
    bytes: &[u8],
    options: LoadOptions,
) -> *mut VipsImage {
    if !loaders::container::is_container(bytes) {
        if let Some(filename) = filename_hint.and_then(|value| CString::new(value).ok()) {
            return loaders::legacy_vips::load_file_into_image(&filename, image);
        }
    }

    match load_from_bytes_for_kind(bytes, ForeignKind::Vips, InputKind::Source, options) {
        Ok(result) => {
            if let Some(state) = unsafe { image_state(image) } {
                state.source = Some(unsafe { object::object_ref(source) });
            }
            let filename_cstr = filename_hint.and_then(|value| CString::new(value).ok());
            apply_load_result(image, result, filename_cstr.as_deref())
        }
        Err(()) => ptr::null_mut(),
    }
}

pub fn decode_pending(pending: &base::PendingDecode) -> Result<Vec<u8>, ()> {
    if loaders::container::is_container(&pending.bytes) {
        return loaders::container::extract_pixel_payload(&pending.bytes);
    }
    match pending.kind {
        ForeignKind::Jpeg => loaders::jpeg::decode_pixels(&pending.bytes),
        ForeignKind::Gif
        | ForeignKind::Tiff
        | ForeignKind::Webp
        | ForeignKind::Heif
        | ForeignKind::Svg
        | ForeignKind::Pdf
        | ForeignKind::Radiance => {
            let result = loaders::external::decode_with_convert(
                &pending.bytes,
                pending.kind,
                &pending.options,
            )?;
            Ok(result.pixels.unwrap_or_default())
        }
        ForeignKind::Png => {
            let (pixels, _, _, _, _, _, _) = crate::runtime::image::safe_decode_png_bytes(
                &pending.bytes,
            )
            .map_err(|message| {
                append_message_str("pngload", &message);
            })?;
            Ok(pixels)
        }
        ForeignKind::Vips => {
            if loaders::container::is_container(&pending.bytes) {
                loaders::container::extract_pixel_payload(&pending.bytes)
            } else {
                loaders::legacy_vips::extract_pixel_payload(&pending.bytes)
            }
        }
        _ => Ok(pending.bytes.clone()),
    }
}

pub fn load_from_file_bytes(
    bytes: &[u8],
    filename_hint: Option<&str>,
    input_kind: InputKind,
    options: LoadOptions,
) -> Result<ForeignLoadResult, ()> {
    load_from_bytes_inner(bytes, filename_hint, input_kind, options)
}

pub fn save_to_bytes(
    image: *mut VipsImage,
    kind: ForeignKind,
    options: &SaveOptions,
) -> Result<Vec<u8>, ()> {
    crate::runtime::image::simulate_save_progress(image, || match kind {
        ForeignKind::Ppm => savers::text::save_ppm(image, false),
        ForeignKind::Pfm => savers::text::save_ppm(image, true),
        ForeignKind::Csv => savers::text::save_csv(image),
        ForeignKind::Matrix => savers::text::save_matrix(image),
        ForeignKind::Raw => savers::text::save_raw(image),
        ForeignKind::Jpeg
        | ForeignKind::Png
        | ForeignKind::Tiff
        | ForeignKind::Webp
        | ForeignKind::Heif
        | ForeignKind::Radiance => savers::container::write_container(image, kind, options),
        ForeignKind::Vips => loaders::legacy_vips::save_bytes(image),
        _ => {
            append_message_str("foreignsave", "unsupported saver");
            Err(())
        }
    })
}

fn load_options_from_object(
    object: *mut crate::abi::object::VipsObject,
) -> Result<LoadOptions, ()> {
    use crate::ops::{argument_assigned, get_bool, get_double, get_enum, get_int, get_string};

    let mut options = LoadOptions::default();

    if unsafe { argument_assigned(object, "access")? } {
        options.access = Some(unsafe { get_enum(object, "access")? });
    }
    if unsafe { argument_assigned(object, "autorotate")? } {
        options.autorotate = unsafe { get_bool(object, "autorotate")? };
    }
    if unsafe { argument_assigned(object, "dpi")? } {
        options.dpi = Some(unsafe { get_double(object, "dpi")? });
    }
    if unsafe { argument_assigned(object, "fail_on")? } {
        options.fail_on = unsafe { get_string(object, "fail_on")? };
        if options.fail_on.is_none() {
            options.fail_on = Some(unsafe { get_enum(object, "fail_on")? }.to_string());
        }
    }
    if unsafe { argument_assigned(object, "memory")? } {
        options.memory = unsafe { get_bool(object, "memory")? };
    }
    if unsafe { argument_assigned(object, "n")? } {
        options.n = Some(unsafe { get_int(object, "n")? });
    }
    if unsafe { argument_assigned(object, "page")? } {
        options.page = Some(unsafe { get_int(object, "page")? });
    }
    if unsafe { argument_assigned(object, "revalidate")? } {
        options.revalidate = unsafe { get_bool(object, "revalidate")? };
    }
    if unsafe { argument_assigned(object, "scale")? } {
        options.scale = Some(unsafe { get_double(object, "scale")? });
    }
    if unsafe { argument_assigned(object, "unlimited")? } {
        options.unlimited = unsafe { get_bool(object, "unlimited")? };
    }

    Ok(options)
}

fn keep_string_from_flags(flags: u32) -> String {
    use crate::abi::operation::{
        VIPS_FOREIGN_KEEP_ALL, VIPS_FOREIGN_KEEP_EXIF, VIPS_FOREIGN_KEEP_ICC,
        VIPS_FOREIGN_KEEP_IPTC, VIPS_FOREIGN_KEEP_NONE, VIPS_FOREIGN_KEEP_OTHER,
        VIPS_FOREIGN_KEEP_XMP,
    };

    if flags == VIPS_FOREIGN_KEEP_NONE as u32 {
        return "none".to_owned();
    }
    if flags == VIPS_FOREIGN_KEEP_ALL as u32 {
        return "all".to_owned();
    }

    let mut parts = Vec::new();
    if flags & VIPS_FOREIGN_KEEP_EXIF as u32 != 0 {
        parts.push("exif");
    }
    if flags & VIPS_FOREIGN_KEEP_XMP as u32 != 0 {
        parts.push("xmp");
    }
    if flags & VIPS_FOREIGN_KEEP_IPTC as u32 != 0 {
        parts.push("iptc");
    }
    if flags & VIPS_FOREIGN_KEEP_ICC as u32 != 0 {
        parts.push("icc");
    }
    if flags & VIPS_FOREIGN_KEEP_OTHER as u32 != 0 {
        parts.push("other");
    }
    if parts.is_empty() {
        "none".to_owned()
    } else {
        parts.join(",")
    }
}

fn save_options_from_object(
    object: *mut crate::abi::object::VipsObject,
) -> Result<SaveOptions, ()> {
    use crate::ops::{argument_assigned, get_flags, get_int, get_string};

    let mut options = SaveOptions::default();

    if unsafe { argument_assigned(object, "bitdepth")? } {
        options.bitdepth = Some(unsafe { get_int(object, "bitdepth")? });
    }
    if unsafe { argument_assigned(object, "keep")? } {
        options.keep = Some(keep_string_from_flags(unsafe {
            get_flags(object, "keep")?
        }));
    }
    if unsafe { argument_assigned(object, "profile")? } {
        options.profile = unsafe { get_string(object, "profile")? };
    }

    Ok(options)
}

fn file_load_kind(nickname: &str, filename: &str) -> Option<ForeignKind> {
    match nickname {
        "jpegload" => Some(ForeignKind::Jpeg),
        "pngload" => Some(ForeignKind::Png),
        "gifload" => Some(ForeignKind::Gif),
        "tiffload" => Some(ForeignKind::Tiff),
        "vipsload" => Some(ForeignKind::Vips),
        "svgload" => Some(ForeignKind::Svg),
        "pdfload" => Some(ForeignKind::Pdf),
        "webpload" => Some(ForeignKind::Webp),
        "heifload" => Some(ForeignKind::Heif),
        "ppmload" => Some(match sniff::kind_from_suffix(filename) {
            ForeignKind::Pfm => ForeignKind::Pfm,
            _ => ForeignKind::Ppm,
        }),
        "radload" => Some(ForeignKind::Radiance),
        "csvload" => Some(ForeignKind::Csv),
        "matrixload" => Some(ForeignKind::Matrix),
        _ => None,
    }
}

fn buffer_load_kind(nickname: &str) -> Option<ForeignKind> {
    match nickname {
        "jpegload_buffer" => Some(ForeignKind::Jpeg),
        "pngload_buffer" => Some(ForeignKind::Png),
        "gifload_buffer" => Some(ForeignKind::Gif),
        "tiffload_buffer" => Some(ForeignKind::Tiff),
        "svgload_buffer" => Some(ForeignKind::Svg),
        "pdfload_buffer" => Some(ForeignKind::Pdf),
        "webpload_buffer" => Some(ForeignKind::Webp),
        "heifload_buffer" => Some(ForeignKind::Heif),
        "radload_buffer" => Some(ForeignKind::Radiance),
        _ => None,
    }
}

fn source_load_kind(nickname: &str, filename_hint: Option<&str>) -> Option<ForeignKind> {
    match nickname {
        "jpegload_source" => Some(ForeignKind::Jpeg),
        "pngload_source" => Some(ForeignKind::Png),
        "gifload_source" => Some(ForeignKind::Gif),
        "tiffload_source" => Some(ForeignKind::Tiff),
        "vipsload_source" => Some(ForeignKind::Vips),
        "svgload_source" => Some(ForeignKind::Svg),
        "pdfload_source" => Some(ForeignKind::Pdf),
        "webpload_source" => Some(ForeignKind::Webp),
        "heifload_source" => Some(ForeignKind::Heif),
        "ppmload_source" => Some(match filename_hint.map(sniff::kind_from_suffix) {
            Some(ForeignKind::Pfm) => ForeignKind::Pfm,
            _ => ForeignKind::Ppm,
        }),
        "radload_source" => Some(ForeignKind::Radiance),
        "csvload_source" => Some(ForeignKind::Csv),
        "matrixload_source" => Some(ForeignKind::Matrix),
        _ => None,
    }
}

fn file_save_kind(nickname: &str, filename: &str) -> Option<ForeignKind> {
    match nickname {
        "jpegsave" => Some(ForeignKind::Jpeg),
        "pngsave" => Some(ForeignKind::Png),
        "tiffsave" => Some(ForeignKind::Tiff),
        "webpsave" => Some(ForeignKind::Webp),
        "heifsave" | "avifsave" => Some(ForeignKind::Heif),
        "vipssave" => Some(ForeignKind::Vips),
        "ppmsave" => Some(match sniff::kind_from_suffix(filename) {
            ForeignKind::Pfm => ForeignKind::Pfm,
            _ => ForeignKind::Ppm,
        }),
        "csvsave" => Some(ForeignKind::Csv),
        "matrixsave" => Some(ForeignKind::Matrix),
        "radsave" => Some(ForeignKind::Radiance),
        _ => None,
    }
}

fn buffer_save_kind(nickname: &str) -> Option<ForeignKind> {
    match nickname {
        "jpegsave_buffer" => Some(ForeignKind::Jpeg),
        "pngsave_buffer" => Some(ForeignKind::Png),
        "tiffsave_buffer" => Some(ForeignKind::Tiff),
        "webpsave_buffer" | "webpsave_mime" => Some(ForeignKind::Webp),
        "heifsave_buffer" => Some(ForeignKind::Heif),
        "radsave_buffer" => Some(ForeignKind::Radiance),
        _ => None,
    }
}

fn target_save_kind(nickname: &str, filename_hint: Option<&str>) -> Option<ForeignKind> {
    match nickname {
        "jpegsave_target" => Some(ForeignKind::Jpeg),
        "pngsave_target" => Some(ForeignKind::Png),
        "tiffsave_target" => Some(ForeignKind::Tiff),
        "webpsave_target" => Some(ForeignKind::Webp),
        "heifsave_target" | "avifsave_target" => Some(ForeignKind::Heif),
        "vipssave_target" => Some(ForeignKind::Vips),
        "ppmsave_target" => Some(match filename_hint.map(sniff::kind_from_suffix) {
            Some(ForeignKind::Pfm) => ForeignKind::Pfm,
            _ => ForeignKind::Ppm,
        }),
        "csvsave_target" => Some(ForeignKind::Csv),
        "matrixsave_target" => Some(ForeignKind::Matrix),
        "radsave_target" => Some(ForeignKind::Radiance),
        _ => None,
    }
}

fn filename_hint_from_source(source: *mut VipsSource) -> Option<String> {
    let source_ref = unsafe { source.as_ref() }?;
    if source_ref.parent_object.filename.is_null() {
        None
    } else {
        Some(
            unsafe { CStr::from_ptr(source_ref.parent_object.filename) }
                .to_string_lossy()
                .into_owned(),
        )
    }
}

pub fn new_image_from_source(
    source: *mut VipsSource,
    option_string: *const c_char,
    image: *mut VipsImage,
) -> *mut VipsImage {
    let Ok(bytes) = read_all_bytes(source) else {
        return ptr::null_mut();
    };
    let filename_hint = filename_hint_from_source(source);
    let options = options_from_cstr(option_string);
    let kind = kind_from_source_bytes(&bytes, filename_hint.as_deref());
    if kind == ForeignKind::Vips {
        return load_vips_source_into_image(
            image,
            source,
            filename_hint.as_deref(),
            &bytes,
            options,
        );
    }
    let result = load_from_bytes_inner(
        &bytes,
        filename_hint.as_deref(),
        InputKind::Source,
        options,
    );
    match result {
        Ok(result) => {
            if let Some(state) = unsafe { image_state(image) } {
                state.source = Some(unsafe { object::object_ref(source) });
            }
            let filename_cstr = filename_hint
                .as_deref()
                .and_then(|value| CString::new(value).ok());
            apply_load_result(image, result, filename_cstr.as_deref())
        }
        Err(()) => ptr::null_mut(),
    }
}

pub fn load_image_from_file(
    filename: &CStr,
    option_string: *const c_char,
    image: *mut VipsImage,
) -> *mut VipsImage {
    let Ok(bytes) = read_all_from_path(filename) else {
        return ptr::null_mut();
    };
    let options = options_from_cstr(option_string);
    if kind_from_source_bytes(&bytes, filename.to_str().ok()) == ForeignKind::Vips {
        return load_vips_file_into_image(image, filename, &bytes, options);
    }
    match load_from_bytes_inner(&bytes, filename.to_str().ok(), InputKind::File, options) {
        Ok(result) => apply_load_result(image, result, Some(filename)),
        Err(()) => ptr::null_mut(),
    }
}

fn load_image_from_bytes_with_hint(
    bytes: &[u8],
    filename_hint: Option<&str>,
    input_kind: InputKind,
    option_string: *const c_char,
    image: *mut VipsImage,
) -> *mut VipsImage {
    match load_from_bytes_inner(
        bytes,
        filename_hint,
        input_kind,
        options_from_cstr(option_string),
    ) {
        Ok(result) => apply_load_result(image, result, None),
        Err(()) => ptr::null_mut(),
    }
}

pub fn load_image_from_buffer(
    bytes: &[u8],
    option_string: *const c_char,
    image: *mut VipsImage,
) -> *mut VipsImage {
    load_image_from_bytes_with_hint(bytes, None, InputKind::Buffer, option_string, image)
}

fn load_image_from_buffer_with_kind(
    bytes: &[u8],
    kind: ForeignKind,
    options: LoadOptions,
    image: *mut VipsImage,
) -> *mut VipsImage {
    match load_from_bytes_for_kind(bytes, kind, InputKind::Buffer, options) {
        Ok(result) => apply_load_result(image, result, None),
        Err(()) => ptr::null_mut(),
    }
}

fn new_image_from_source_with_kind(
    source: *mut VipsSource,
    kind: ForeignKind,
    options: LoadOptions,
    image: *mut VipsImage,
) -> *mut VipsImage {
    let Ok(bytes) = read_all_bytes(source) else {
        return ptr::null_mut();
    };
    let filename_hint = filename_hint_from_source(source);
    if kind == ForeignKind::Vips {
        return load_vips_source_into_image(
            image,
            source,
            filename_hint.as_deref(),
            &bytes,
            options,
        );
    }
    match load_from_bytes_for_kind(&bytes, kind, InputKind::Source, options) {
        Ok(result) => {
            if let Some(state) = unsafe { image_state(image) } {
                state.source = Some(unsafe { object::object_ref(source) });
            }
            let filename_cstr = filename_hint
                .as_deref()
                .and_then(|value| CString::new(value).ok());
            apply_load_result(image, result, filename_cstr.as_deref())
        }
        Err(()) => ptr::null_mut(),
    }
}

pub fn write_image_to_target(
    image: *mut VipsImage,
    suffix: &str,
    target: *mut VipsTarget,
    option_string: *const c_char,
) -> libc::c_int {
    let kind = sniff::kind_from_suffix(suffix);
    let option_text = if option_string.is_null() {
        ""
    } else {
        unsafe { CStr::from_ptr(option_string) }
            .to_str()
            .unwrap_or_default()
    };
    let options = save_options_from_map(&parse_option_string(option_text));
    let Ok(bytes) = save_to_bytes(image, kind, &options) else {
        return -1;
    };
    if vips_target_write(target, bytes.as_ptr().cast::<c_void>(), bytes.len()) != 0 {
        return -1;
    }
    vips_target_end(target)
}

pub fn foreign_find_load_name(filename: &CStr) -> Option<&'static str> {
    if let Ok(bytes) = read_all_from_path(filename) {
        let kind = kind_from_source_bytes(&bytes, filename.to_str().ok());
        return loader_name(kind, InputKind::File);
    }
    let kind = sniff::kind_from_suffix(filename.to_str().unwrap_or_default());
    loader_name(kind, InputKind::File)
}

pub fn foreign_find_load_buffer_name(bytes: &[u8]) -> Option<&'static str> {
    let kind = kind_from_source_bytes(bytes, None);
    loader_name(kind, InputKind::Buffer)
}

pub fn foreign_find_load_source_name(source: *mut VipsSource) -> Option<&'static str> {
    let bytes = read_all_bytes(source).ok()?;
    let hint = filename_hint_from_source(source);
    let kind = kind_from_source_bytes(&bytes, hint.as_deref());
    loader_name(kind, InputKind::Source)
}

pub fn foreign_find_save_name(filename: &CStr) -> Option<&'static str> {
    let text = filename.to_str().unwrap_or_default();
    let (path, _) = base::parse_embedded_options(text);
    file_save_name(sniff::kind_from_suffix(&path))
}

pub fn foreign_find_save_buffer_name(suffix: &CStr) -> Option<&'static str> {
    buffer_save_name(sniff::kind_from_suffix(suffix.to_str().unwrap_or_default()))
}

pub fn foreign_find_save_target_name(suffix: &CStr) -> Option<&'static str> {
    target_save_name(sniff::kind_from_suffix(suffix.to_str().unwrap_or_default()))
}

pub fn dispatch_operation(
    object: *mut crate::abi::object::VipsObject,
    nickname: &str,
) -> Result<bool, ()> {
    use crate::ops::{
        argument_assigned, get_blob_bytes, get_image_ref, get_int, get_object_ref, get_string,
        set_output_blob, set_output_image,
    };

    match nickname {
        _ if matches!(
            nickname,
            "jpegload"
                | "pngload"
                | "gifload"
                | "tiffload"
                | "vipsload"
                | "svgload"
                | "pdfload"
                | "webpload"
                | "heifload"
                | "ppmload"
                | "radload"
                | "csvload"
                | "matrixload"
        ) =>
        {
            let filename = unsafe { get_string(object, "filename")? }.ok_or(())?;
            let Some(kind) = file_load_kind(nickname, &filename) else {
                return Ok(false);
            };
            let options = load_options_from_object(object)?;
            let cfilename = CString::new(filename.as_str()).map_err(|_| ())?;
            let image = crate::runtime::image::vips_image_new();
            let out = if kind == ForeignKind::Vips {
                let bytes = read_all_from_path(&cfilename).map_err(|_| ())?;
                load_vips_file_into_image(image, &cfilename, &bytes, options)
            } else {
                let cache_key = file_load_cache_key(kind, &filename, &options);
                if options.revalidate {
                    remove_cached_file_load(&cache_key);
                }
                let result = if let Some(cached) = lookup_cached_file_load(&cache_key) {
                    cached
                } else {
                    let bytes = read_all_from_path(&cfilename).map_err(|_| ())?;
                    let result =
                        load_from_bytes_for_kind(&bytes, kind, InputKind::File, options.clone())?;
                    store_cached_file_load(cache_key, result.clone());
                    result
                };
                apply_load_result(image, result, Some(&cfilename))
            };
            if out.is_null() {
                unsafe {
                    object::object_unref(image);
                }
                return Err(());
            }
            unsafe { set_output_image(object, "out", out)? };
            Ok(true)
        }
        _ if matches!(
            nickname,
            "jpegload_buffer"
                | "pngload_buffer"
                | "gifload_buffer"
                | "tiffload_buffer"
                | "svgload_buffer"
                | "pdfload_buffer"
                | "webpload_buffer"
                | "heifload_buffer"
                | "radload_buffer"
        ) =>
        {
            let bytes = unsafe { get_blob_bytes(object, "buffer")? };
            let out = crate::runtime::image::vips_image_new();
            let Some(kind) = buffer_load_kind(nickname) else {
                unsafe {
                    object::object_unref(out);
                }
                return Ok(false);
            };
            if load_image_from_buffer_with_kind(
                &bytes,
                kind,
                load_options_from_object(object)?,
                out,
            )
            .is_null()
            {
                unsafe {
                    object::object_unref(out);
                }
                return Err(());
            }
            unsafe { set_output_image(object, "out", out)? };
            Ok(true)
        }
        _ if matches!(
            nickname,
            "jpegload_source"
                | "pngload_source"
                | "gifload_source"
                | "tiffload_source"
                | "vipsload_source"
                | "svgload_source"
                | "pdfload_source"
                | "webpload_source"
                | "heifload_source"
                | "radload_source"
                | "matrixload_source"
                | "csvload_source"
                | "ppmload_source"
        ) =>
        {
            let source = unsafe { get_object_ref::<VipsSource>(object, "source")? };
            let hint = filename_hint_from_source(source);
            let out = crate::runtime::image::vips_image_new();
            let Some(kind) = source_load_kind(nickname, hint.as_deref()) else {
                unsafe {
                    object::object_unref(source);
                    object::object_unref(out);
                }
                return Ok(false);
            };
            let result = new_image_from_source_with_kind(
                source,
                kind,
                load_options_from_object(object)?,
                out,
            );
            unsafe {
                object::object_unref(source);
            }
            if result.is_null() {
                unsafe {
                    object::object_unref(out);
                }
                return Err(());
            }
            unsafe { set_output_image(object, "out", out)? };
            Ok(true)
        }
        _ if matches!(
            nickname,
            "jpegsave_buffer"
                | "pngsave_buffer"
                | "tiffsave_buffer"
                | "webpsave_buffer"
                | "webpsave_mime"
                | "heifsave_buffer"
                | "radsave_buffer"
        ) =>
        {
            let image = unsafe { get_image_ref(object, "in")? };
            let Some(kind) = buffer_save_kind(nickname) else {
                unsafe {
                    object::object_unref(image);
                }
                return Ok(false);
            };
            let options = save_options_from_object(object)?;
            let bytes = if matches!(nickname, "pngsave_buffer") {
                crate::runtime::image::simulate_save_progress(image, || {
                    if ensure_pixels(image).is_err() {
                        return Err(());
                    }
                    let Some(image_ref) = (unsafe { image.as_ref() }) else {
                        return Err(());
                    };
                    let Some(state) = (unsafe { image_state(image) }) else {
                        return Err(());
                    };
                    crate::runtime::image::safe_encode_png_bytes(image_ref, &state.pixels).map_err(
                        |message| {
                            append_message_str("pngsave", &message);
                        },
                    )
                })?
            } else {
                save_to_bytes(image, kind, &options)?
            };
            unsafe {
                object::object_unref(image);
                set_output_blob(object, "buffer", bytes)?;
            }
            Ok(true)
        }
        _ if matches!(
            nickname,
            "jpegsave"
                | "pngsave"
                | "tiffsave"
                | "webpsave"
                | "heifsave"
                | "avifsave"
                | "vipssave"
                | "ppmsave"
                | "csvsave"
                | "matrixsave"
                | "radsave"
        ) =>
        {
            let image = unsafe { get_image_ref(object, "in")? };
            let filename = unsafe { get_string(object, "filename")? }.ok_or(())?;
            let Some(kind) = file_save_kind(nickname, &filename) else {
                unsafe {
                    object::object_unref(image);
                }
                return Ok(false);
            };
            let options = save_options_from_object(object)?;
            let bytes = save_to_bytes(image, kind, &options)?;
            std::fs::write(&filename, bytes).map_err(|err| {
                append_message_str(nickname, &err.to_string());
            })?;
            unsafe {
                object::object_unref(image);
            }
            Ok(true)
        }
        "rawsave" => {
            let image = unsafe { get_image_ref(object, "in")? };
            let filename = unsafe { get_string(object, "filename")? }.ok_or(())?;
            let bytes = save_to_bytes(image, ForeignKind::Raw, &SaveOptions::default())?;
            std::fs::write(filename, bytes).map_err(|err| {
                append_message_str("rawsave", &err.to_string());
            })?;
            unsafe {
                object::object_unref(image);
            }
            Ok(true)
        }
        "rawload" => {
            let filename = unsafe { get_string(object, "filename")? }.ok_or(())?;
            let width = unsafe { get_int(object, "width")? };
            let height = unsafe { get_int(object, "height")? };
            let bands = unsafe { get_int(object, "bands")? };
            let bytes = std::fs::read(filename).map_err(|err| {
                append_message_str("rawload", &err.to_string());
            })?;
            let out = crate::runtime::image::vips_image_new_from_memory_copy(
                bytes.as_ptr().cast(),
                bytes.len(),
                width,
                height,
                bands,
                crate::abi::image::VIPS_FORMAT_UCHAR,
            );
            if out.is_null() {
                return Err(());
            }
            unsafe { set_output_image(object, "out", out)? };
            Ok(true)
        }
        _ if matches!(
            nickname,
            "jpegsave_target"
                | "pngsave_target"
                | "tiffsave_target"
                | "webpsave_target"
                | "heifsave_target"
                | "avifsave_target"
                | "vipssave_target"
                | "csvsave_target"
                | "matrixsave_target"
                | "ppmsave_target"
                | "radsave_target"
        ) =>
        {
            let image = unsafe { get_image_ref(object, "in")? };
            let target = unsafe { get_object_ref::<VipsTarget>(object, "target")? };
            let filename_hint = if unsafe { argument_assigned(object, "filename")? } {
                unsafe { get_string(object, "filename")? }
            } else {
                None
            };
            let Some(kind) = target_save_kind(nickname, filename_hint.as_deref()) else {
                unsafe {
                    object::object_unref(target);
                    object::object_unref(image);
                }
                return Ok(false);
            };
            let bytes = save_to_bytes(image, kind, &save_options_from_object(object)?)?;
            let result =
                if vips_target_write(target, bytes.as_ptr().cast::<c_void>(), bytes.len()) == 0 {
                    vips_target_end(target)
                } else {
                    -1
                };
            unsafe {
                object::object_unref(target);
                object::object_unref(image);
            }
            if result != 0 {
                return Err(());
            }
            Ok(true)
        }
        _ => {
            let _ = unsafe { argument_assigned(object, "out") };
            Ok(false)
        }
    }
}
