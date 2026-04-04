use crate::abi::basic::{
    VipsExtend, VipsOperationBoolean, VIPS_EXTEND_BACKGROUND, VIPS_EXTEND_BLACK,
    VIPS_EXTEND_COPY, VIPS_EXTEND_MIRROR, VIPS_EXTEND_REPEAT, VIPS_EXTEND_WHITE,
};
use crate::abi::image::{VipsBandFormat, VipsImage};
use crate::abi::object::VipsObject;
use crate::pixels::format::format_max;
use crate::pixels::iter::pixel_index;
use crate::pixels::ImageBuffer;
use crate::runtime::header::copy_metadata;
use crate::runtime::image::{ensure_pixels, image_state};

use super::{
    argument_assigned, get_array_double, get_array_images, get_bool, get_enum, get_image_buffer,
    get_image_ref, get_int, set_output_image, set_output_image_like,
};

fn background_values(values: &[f64], bands: usize) -> Vec<f64> {
    if values.is_empty() {
        vec![0.0; bands]
    } else {
        (0..bands)
            .map(|band| values.get(band).copied().unwrap_or(values[0]))
            .collect()
    }
}

fn apply_embed_background(
    input: &ImageBuffer,
    width: usize,
    height: usize,
    x: isize,
    y: isize,
    extend: VipsExtend,
    background: &[f64],
) -> ImageBuffer {
    let mut out = ImageBuffer::new(
        width,
        height,
        input.spec.bands,
        input.spec.format,
        input.spec.coding,
        input.spec.interpretation,
    );
    out.spec.xres = input.spec.xres;
    out.spec.yres = input.spec.yres;
    out.spec.xoffset = input.spec.xoffset;
    out.spec.yoffset = input.spec.yoffset;
    out.spec.dhint = input.spec.dhint;

    let bg = background_values(background, input.spec.bands);
    let white = format_max(input.spec.format).unwrap_or(255.0);

    for oy in 0..height {
        for ox in 0..width {
            let sx = ox as isize - x;
            let sy = oy as isize - y;
            let inside = sx >= 0
                && sy >= 0
                && (sx as usize) < input.spec.width
                && (sy as usize) < input.spec.height;

            for band in 0..input.spec.bands {
                let value = if inside {
                    input.get(sx as usize, sy as usize, band)
                } else {
                    match extend {
                        VIPS_EXTEND_BLACK => 0.0,
                        VIPS_EXTEND_WHITE => white,
                        VIPS_EXTEND_BACKGROUND => bg[band],
                        VIPS_EXTEND_COPY => {
                            let sx = sx.clamp(0, input.spec.width.saturating_sub(1) as isize) as usize;
                            let sy = sy.clamp(0, input.spec.height.saturating_sub(1) as isize) as usize;
                            input.get(sx, sy, band)
                        }
                        VIPS_EXTEND_REPEAT => {
                            let sx = sx.rem_euclid(input.spec.width.max(1) as isize) as usize;
                            let sy = sy.rem_euclid(input.spec.height.max(1) as isize) as usize;
                            input.get(sx, sy, band)
                        }
                        VIPS_EXTEND_MIRROR => {
                            let mirror = |coord: isize, size: usize| -> usize {
                                if size <= 1 {
                                    return 0;
                                }
                                let period = (size * 2 - 2) as isize;
                                let mut value = coord.rem_euclid(period);
                                if value >= size as isize {
                                    value = period - value;
                                }
                                value as usize
                            };
                            input.get(mirror(sx, input.spec.width), mirror(sy, input.spec.height), band)
                        }
                        _ => 0.0,
                    }
                };
                out.set(ox, oy, band, value);
            }
        }
    }

    out
}

unsafe fn op_extract_area(object: *mut VipsObject, input_name: &str, output_name: &str) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, input_name)? };
    let left = unsafe { get_int(object, "left")? };
    let top = unsafe { get_int(object, "top")? };
    let width = unsafe { get_int(object, "width")? };
    let height = unsafe { get_int(object, "height")? };

    if left < 0 || top < 0 || width <= 0 || height <= 0 {
        return Err(());
    }

    let left_u = usize::try_from(left).map_err(|_| ())?;
    let top_u = usize::try_from(top).map_err(|_| ())?;
    let width_u = usize::try_from(width).map_err(|_| ())?;
    let height_u = usize::try_from(height).map_err(|_| ())?;

    let right = left_u.checked_add(width_u).ok_or(())?;
    let bottom = top_u.checked_add(height_u).ok_or(())?;
    if right > input.spec.width || bottom > input.spec.height {
        return Err(());
    }

    let mut out = input.with_shape(width_u, height_u, input.spec.bands);
    out.spec.xoffset = input.spec.xoffset.saturating_add(left);
    out.spec.yoffset = input.spec.yoffset.saturating_add(top);
    for y in 0..height_u {
        for x in 0..width_u {
            for band in 0..input.spec.bands {
                out.set(x, y, band, input.get(left_u + x, top_u + y, band));
            }
        }
    }

    let image = unsafe { get_image_ref(object, input_name)? };
    let result = unsafe { set_output_image_like(object, output_name, out, image) };
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

unsafe fn op_extract_band(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let band = usize::try_from(unsafe { get_int(object, "band")? }).map_err(|_| ())?;
    let n = if unsafe { argument_assigned(object, "n")? } {
        usize::try_from(unsafe { get_int(object, "n")? }).map_err(|_| ())?
    } else {
        1
    };
    let end = band.checked_add(n).ok_or(())?;
    if band >= input.spec.bands || end > input.spec.bands || n == 0 {
        return Err(());
    }

    let mut out = input.with_shape(input.spec.width, input.spec.height, n);
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            for ob in 0..n {
                out.set(x, y, ob, input.get(x, y, band + ob));
            }
        }
    }

    let image = unsafe { get_image_ref(object, "in")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

unsafe fn op_bandjoin(object: *mut VipsObject) -> Result<(), ()> {
    let images = unsafe { get_array_images(object, "in")? };
    let first = *images.first().ok_or(())?;
    let first_buffer = ImageBuffer::from_image(first)?;
    let width = first_buffer.spec.width;
    let height = first_buffer.spec.height;
    let format = first_buffer.spec.format;
    let total_bands = images
        .iter()
        .map(|image| ImageBuffer::from_image(*image).map(|buffer| buffer.spec.bands))
        .try_fold(0usize, |acc, item| item.map(|bands| acc + bands))?;
    let mut out = first_buffer.with_shape(width, height, total_bands).with_format(format);

    let mut band_offset = 0;
    for image in &images {
        let buffer = ImageBuffer::from_image(*image)?;
        if buffer.spec.width != width || buffer.spec.height != height {
            return Err(());
        }
        for y in 0..height {
            for x in 0..width {
                for band in 0..buffer.spec.bands {
                    out.set(x, y, band_offset + band, buffer.get(x, y, band));
                }
            }
        }
        band_offset += buffer.spec.bands;
    }

    unsafe { set_output_image_like(object, "out", out, first) }
}

unsafe fn op_bandjoin_const(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let constants = unsafe { get_array_double(object, "c")? };
    if constants.is_empty() {
        return Err(());
    }
    let mut out = input.with_shape(
        input.spec.width,
        input.spec.height,
        input.spec.bands + constants.len(),
    );
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            for band in 0..input.spec.bands {
                out.set(x, y, band, input.get(x, y, band));
            }
            for (index, value) in constants.iter().copied().enumerate() {
                out.set(x, y, input.spec.bands + index, value);
            }
        }
    }

    let image = unsafe { get_image_ref(object, "in")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

unsafe fn op_bandmean(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let mut out = input.with_shape(input.spec.width, input.spec.height, 1);
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            let sum = (0..input.spec.bands).map(|band| input.get(x, y, band)).sum::<f64>();
            out.set(x, y, 0, sum / input.spec.bands.max(1) as f64);
        }
    }
    let image = unsafe { get_image_ref(object, "in")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

fn bandbool_reduce(values: impl Iterator<Item = f64>, format: VipsBandFormat, op: VipsOperationBoolean) -> f64 {
    let mut iter = values.peekable();
    let Some(mut acc) = iter.next() else {
        return 0.0;
    };
    for value in iter {
        acc = super::arithmetic::boolean_value(format, acc, value, op);
    }
    acc
}

unsafe fn op_bandbool(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let op = unsafe { get_enum(object, "boolean")? } as VipsOperationBoolean;
    let mut out = input.with_shape(input.spec.width, input.spec.height, 1);
    out.spec.format = super::arithmetic::binary_output_format("boolean", input.spec.format)?;
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            let value = bandbool_reduce((0..input.spec.bands).map(|band| input.get(x, y, band)), input.spec.format, op);
            out.set(x, y, 0, value);
        }
    }
    let image = unsafe { get_image_ref(object, "in")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

unsafe fn op_bandfold(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let factor = if unsafe { argument_assigned(object, "factor")? } {
        usize::try_from(unsafe { get_int(object, "factor")? }).map_err(|_| ())?
    } else {
        0
    };
    let factor = if factor == 0 { input.spec.width } else { factor };
    if factor == 0 || input.spec.width % factor != 0 {
        return Err(());
    }

    let out_width = input.spec.width / factor;
    let out_bands = input.spec.bands * factor;
    let mut out = input.with_shape(out_width, input.spec.height, out_bands);
    for y in 0..input.spec.height {
        for ox in 0..out_width {
            for ob in 0..out_bands {
                let source_x = ox * factor + ob / input.spec.bands;
                let source_band = ob % input.spec.bands;
                out.set(ox, y, ob, input.get(source_x, y, source_band));
            }
        }
    }

    let image = unsafe { get_image_ref(object, "in")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

unsafe fn op_bandunfold(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let factor = if unsafe { argument_assigned(object, "factor")? } {
        usize::try_from(unsafe { get_int(object, "factor")? }).map_err(|_| ())?
    } else {
        0
    };
    let factor = if factor == 0 { input.spec.bands } else { factor };
    if factor == 0 || input.spec.bands % factor != 0 {
        return Err(());
    }

    let out_width = input.spec.width * factor;
    let out_bands = input.spec.bands / factor;
    let mut out = input.with_shape(out_width, input.spec.height, out_bands);
    for y in 0..input.spec.height {
        for x in 0..out_width {
            let source_x = x / factor;
            let offset = x % factor;
            for band in 0..out_bands {
                out.set(
                    x,
                    y,
                    band,
                    input.get(source_x, y, offset * out_bands + band),
                );
            }
        }
    }

    let image = unsafe { get_image_ref(object, "in")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

unsafe fn op_embed(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let x = unsafe { get_int(object, "x")? } as isize;
    let y = unsafe { get_int(object, "y")? } as isize;
    let width = usize::try_from(unsafe { get_int(object, "width")? }).map_err(|_| ())?;
    let height = usize::try_from(unsafe { get_int(object, "height")? }).map_err(|_| ())?;
    let extend = if unsafe { argument_assigned(object, "extend")? } {
        unsafe { get_enum(object, "extend")? as VipsExtend }
    } else {
        VIPS_EXTEND_BLACK
    };
    let background = if unsafe { argument_assigned(object, "background")? } {
        unsafe { get_array_double(object, "background")? }
    } else {
        Vec::new()
    };
    let out = apply_embed_background(&input, width, height, x, y, extend, &background);
    let image = unsafe { get_image_ref(object, "in")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

unsafe fn op_cast(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let format = unsafe { get_enum(object, "format")? } as VipsBandFormat;
    let mut out = input.with_format(format);
    if unsafe { argument_assigned(object, "shift")? } && unsafe { get_bool(object, "shift")? } {
        let scale = 1u64 << 8;
        for value in &mut out.data {
            *value *= scale as f64;
        }
    }
    let image = unsafe { get_image_ref(object, "in")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

unsafe fn op_copy(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let mut out = input.clone();
    if unsafe { argument_assigned(object, "width")? } {
        out.spec.width = usize::try_from(unsafe { get_int(object, "width")? }).map_err(|_| ())?;
    }
    if unsafe { argument_assigned(object, "height")? } {
        out.spec.height = usize::try_from(unsafe { get_int(object, "height")? }).map_err(|_| ())?;
    }
    if unsafe { argument_assigned(object, "bands")? } {
        out.spec.bands = usize::try_from(unsafe { get_int(object, "bands")? }).map_err(|_| ())?;
    }
    if unsafe { argument_assigned(object, "format")? } {
        out.spec.format = unsafe { get_enum(object, "format")? } as VipsBandFormat;
    }
    if unsafe { argument_assigned(object, "xres")? } {
        out.spec.xres = unsafe { super::get_double(object, "xres")? };
    }
    if unsafe { argument_assigned(object, "yres")? } {
        out.spec.yres = unsafe { super::get_double(object, "yres")? };
    }
    if unsafe { argument_assigned(object, "xoffset")? } {
        out.spec.xoffset = unsafe { get_int(object, "xoffset")? };
    }
    if unsafe { argument_assigned(object, "yoffset")? } {
        out.spec.yoffset = unsafe { get_int(object, "yoffset")? };
    }
    let expected = out.spec.width.saturating_mul(out.spec.height).saturating_mul(out.spec.bands);
    out.data.resize(expected, 0.0);
    let image = unsafe { get_image_ref(object, "in")? };
    let mut out_image = out.into_image_like(image);
    unsafe {
        crate::runtime::object::object_unref(image);
    }

    if unsafe { argument_assigned(object, "swap")? } && unsafe { get_bool(object, "swap")? } {
        ensure_pixels(out_image)?;
        let sample_size = crate::pixels::format::format_bytes(unsafe { (*out_image).BandFmt });
        if let Some(state) = unsafe { image_state(out_image) } {
            if sample_size > 1 {
                for chunk in state.pixels.chunks_exact_mut(sample_size) {
                    chunk.reverse();
                }
            }
        }
        crate::runtime::image::sync_pixels(out_image);
    }

    unsafe { set_output_image(object, "out", out_image) }
}

unsafe fn op_byteswap(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_ref(object, "in")? };
    ensure_pixels(input)?;
    let source = unsafe { image_state(input) }.ok_or(())?;
    let sample_size = crate::pixels::format::format_bytes(unsafe { (*input).BandFmt });
    let mut out = ImageBuffer::from_image(input)?;
    let out_image = out.to_image();
    copy_metadata(out_image, input);
    ensure_pixels(out_image)?;
    let target = unsafe { image_state(out_image) }.ok_or(())?;
    target.pixels = source.pixels.clone();
    if sample_size > 1 {
        for chunk in target.pixels.chunks_exact_mut(sample_size) {
            chunk.reverse();
        }
    }
    crate::runtime::image::sync_pixels(out_image);
    unsafe {
        crate::runtime::object::object_unref(input);
    }
    unsafe { set_output_image(object, "out", out_image) }
}

pub(crate) unsafe fn dispatch(object: *mut VipsObject, nickname: &str) -> Result<bool, ()> {
    match nickname {
        "copy" => {
            unsafe { op_copy(object)? };
            Ok(true)
        }
        "crop" | "extract_area" => {
            unsafe { op_extract_area(object, if nickname == "crop" { "in" } else { "input" }, "out")? };
            Ok(true)
        }
        "extract_band" => {
            unsafe { op_extract_band(object)? };
            Ok(true)
        }
        "bandjoin" => {
            unsafe { op_bandjoin(object)? };
            Ok(true)
        }
        "bandjoin_const" => {
            unsafe { op_bandjoin_const(object)? };
            Ok(true)
        }
        "bandmean" => {
            unsafe { op_bandmean(object)? };
            Ok(true)
        }
        "bandbool" => {
            unsafe { op_bandbool(object)? };
            Ok(true)
        }
        "bandfold" => {
            unsafe { op_bandfold(object)? };
            Ok(true)
        }
        "bandunfold" => {
            unsafe { op_bandunfold(object)? };
            Ok(true)
        }
        "embed" => {
            unsafe { op_embed(object)? };
            Ok(true)
        }
        "cast" => {
            unsafe { op_cast(object)? };
            Ok(true)
        }
        "byteswap" => {
            unsafe { op_byteswap(object)? };
            Ok(true)
        }
        _ => Ok(false),
    }
}
