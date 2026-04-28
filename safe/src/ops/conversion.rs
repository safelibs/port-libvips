use crate::abi::basic::{
    VipsAlign, VipsAngle, VipsAngle45, VipsCompassDirection, VipsDirection, VipsExtend,
    VipsInteresting, VipsOperationBoolean, VIPS_ALIGN_CENTRE, VIPS_ALIGN_HIGH, VIPS_ALIGN_LOW,
    VIPS_ANGLE45_D0, VIPS_ANGLE45_D135, VIPS_ANGLE45_D180, VIPS_ANGLE45_D225, VIPS_ANGLE45_D270,
    VIPS_ANGLE45_D315, VIPS_ANGLE45_D45, VIPS_ANGLE45_D90, VIPS_ANGLE_D0, VIPS_ANGLE_D180,
    VIPS_ANGLE_D270, VIPS_ANGLE_D90, VIPS_COMPASS_DIRECTION_CENTRE, VIPS_COMPASS_DIRECTION_EAST,
    VIPS_COMPASS_DIRECTION_NORTH, VIPS_COMPASS_DIRECTION_NORTH_EAST,
    VIPS_COMPASS_DIRECTION_NORTH_WEST, VIPS_COMPASS_DIRECTION_SOUTH,
    VIPS_COMPASS_DIRECTION_SOUTH_EAST, VIPS_COMPASS_DIRECTION_SOUTH_WEST,
    VIPS_COMPASS_DIRECTION_WEST, VIPS_DIRECTION_HORIZONTAL, VIPS_DIRECTION_VERTICAL,
    VIPS_EXTEND_BACKGROUND, VIPS_EXTEND_BLACK, VIPS_EXTEND_COPY, VIPS_EXTEND_MIRROR,
    VIPS_EXTEND_REPEAT, VIPS_EXTEND_WHITE, VIPS_INTERESTING_ALL, VIPS_INTERESTING_ATTENTION,
    VIPS_INTERESTING_CENTRE, VIPS_INTERESTING_HIGH, VIPS_INTERESTING_LOW, VIPS_INTERESTING_NONE,
    VIPS_KERNEL_LANCZOS3, VIPS_PRECISION_INTEGER,
};
use crate::abi::image::{
    VipsBandFormat, VipsCoding, VipsInterpretation, VIPS_DEMAND_STYLE_THINSTRIP,
    VIPS_FORMAT_DOUBLE, VIPS_FORMAT_FLOAT, VIPS_FORMAT_UCHAR, VIPS_INTERPRETATION_LAB,
    VIPS_INTERPRETATION_XYZ,
};
use crate::abi::object::VipsObject;
use crate::pixels::format::{
    clamp_for_format, common_format, format_bytes, format_kind, format_max, NumericKind,
};
use crate::pixels::kernel::{gaussian_kernel, Kernel};
use crate::pixels::ImageBuffer;
use crate::runtime::error::append_message_str;
use crate::runtime::header::{copy_metadata, vips_interpretation_max_alpha};
use crate::runtime::image::{ensure_pixels, image_size, image_state, sync_pixels};

use super::{
    argument_assigned, get_array_double, get_array_images, get_bool, get_double, get_enum,
    get_image_buffer, get_image_ref, get_int, set_output_image, set_output_image_like,
    set_output_int,
};

const FALSECOLOUR_PET: [[u8; 3]; 256] = [
    [12, 0, 25],
    [17, 0, 34],
    [20, 0, 41],
    [22, 0, 45],
    [23, 0, 47],
    [27, 0, 55],
    [12, 0, 25],
    [5, 0, 11],
    [5, 0, 11],
    [5, 0, 11],
    [1, 0, 4],
    [1, 0, 4],
    [6, 0, 13],
    [15, 0, 30],
    [19, 0, 40],
    [23, 0, 48],
    [28, 0, 57],
    [36, 0, 74],
    [42, 0, 84],
    [46, 0, 93],
    [51, 0, 102],
    [59, 0, 118],
    [65, 0, 130],
    [69, 0, 138],
    [72, 0, 146],
    [81, 0, 163],
    [47, 0, 95],
    [12, 0, 28],
    [64, 0, 144],
    [61, 0, 146],
    [55, 0, 140],
    [52, 0, 137],
    [47, 0, 132],
    [43, 0, 128],
    [38, 0, 123],
    [30, 0, 115],
    [26, 0, 111],
    [23, 0, 108],
    [17, 0, 102],
    [9, 0, 94],
    [6, 0, 91],
    [2, 0, 87],
    [0, 0, 88],
    [0, 0, 100],
    [0, 0, 104],
    [0, 0, 108],
    [0, 0, 113],
    [0, 0, 121],
    [0, 0, 125],
    [0, 0, 129],
    [0, 0, 133],
    [0, 0, 141],
    [0, 0, 146],
    [0, 0, 150],
    [0, 0, 155],
    [0, 0, 162],
    [0, 0, 167],
    [0, 0, 173],
    [0, 0, 180],
    [0, 0, 188],
    [0, 0, 193],
    [0, 0, 197],
    [0, 0, 201],
    [0, 0, 209],
    [0, 0, 214],
    [0, 0, 218],
    [0, 0, 222],
    [0, 0, 230],
    [0, 0, 235],
    [0, 0, 239],
    [0, 0, 243],
    [0, 0, 247],
    [0, 4, 251],
    [0, 10, 255],
    [0, 14, 255],
    [0, 18, 255],
    [0, 24, 255],
    [0, 31, 255],
    [0, 36, 255],
    [0, 39, 255],
    [0, 45, 255],
    [0, 53, 255],
    [0, 56, 255],
    [0, 60, 255],
    [0, 66, 255],
    [0, 74, 255],
    [0, 77, 255],
    [0, 81, 255],
    [0, 88, 251],
    [0, 99, 239],
    [0, 104, 234],
    [0, 108, 230],
    [0, 113, 225],
    [0, 120, 218],
    [0, 125, 213],
    [0, 128, 210],
    [0, 133, 205],
    [0, 141, 197],
    [0, 145, 193],
    [0, 150, 188],
    [0, 154, 184],
    [0, 162, 176],
    [0, 167, 172],
    [0, 172, 170],
    [0, 180, 170],
    [0, 188, 170],
    [0, 193, 170],
    [0, 197, 170],
    [0, 201, 170],
    [0, 205, 170],
    [0, 211, 170],
    [0, 218, 170],
    [0, 222, 170],
    [0, 226, 170],
    [0, 232, 170],
    [0, 239, 170],
    [0, 243, 170],
    [0, 247, 170],
    [0, 251, 161],
    [0, 255, 147],
    [0, 255, 139],
    [0, 255, 131],
    [0, 255, 120],
    [0, 255, 105],
    [0, 255, 97],
    [0, 255, 89],
    [0, 255, 78],
    [0, 255, 63],
    [0, 255, 55],
    [0, 255, 47],
    [0, 255, 37],
    [0, 255, 21],
    [0, 255, 13],
    [0, 255, 5],
    [2, 255, 2],
    [13, 255, 13],
    [18, 255, 18],
    [23, 255, 23],
    [27, 255, 27],
    [35, 255, 35],
    [40, 255, 40],
    [43, 255, 43],
    [48, 255, 48],
    [55, 255, 55],
    [60, 255, 60],
    [64, 255, 64],
    [69, 255, 69],
    [72, 255, 72],
    [79, 255, 79],
    [90, 255, 82],
    [106, 255, 74],
    [113, 255, 70],
    [126, 255, 63],
    [140, 255, 56],
    [147, 255, 53],
    [155, 255, 48],
    [168, 255, 42],
    [181, 255, 36],
    [189, 255, 31],
    [197, 255, 27],
    [209, 255, 21],
    [224, 255, 14],
    [231, 255, 10],
    [239, 255, 7],
    [247, 251, 3],
    [255, 243, 0],
    [255, 239, 0],
    [255, 235, 0],
    [255, 230, 0],
    [255, 222, 0],
    [255, 218, 0],
    [255, 214, 0],
    [255, 209, 0],
    [255, 201, 0],
    [255, 197, 0],
    [255, 193, 0],
    [255, 188, 0],
    [255, 180, 0],
    [255, 176, 0],
    [255, 172, 0],
    [255, 167, 0],
    [255, 156, 0],
    [255, 150, 0],
    [255, 146, 0],
    [255, 142, 0],
    [255, 138, 0],
    [255, 131, 0],
    [255, 125, 0],
    [255, 121, 0],
    [255, 117, 0],
    [255, 110, 0],
    [255, 104, 0],
    [255, 100, 0],
    [255, 96, 0],
    [255, 90, 0],
    [255, 83, 0],
    [255, 78, 0],
    [255, 75, 0],
    [255, 71, 0],
    [255, 67, 0],
    [255, 65, 0],
    [255, 63, 0],
    [255, 59, 0],
    [255, 54, 0],
    [255, 52, 0],
    [255, 50, 0],
    [255, 46, 0],
    [255, 41, 0],
    [255, 39, 0],
    [255, 36, 0],
    [255, 32, 0],
    [255, 25, 0],
    [255, 22, 0],
    [255, 20, 0],
    [255, 17, 0],
    [255, 13, 0],
    [255, 10, 0],
    [255, 7, 0],
    [255, 4, 0],
    [255, 0, 0],
    [252, 0, 0],
    [251, 0, 0],
    [249, 0, 0],
    [248, 0, 0],
    [244, 0, 0],
    [242, 0, 0],
    [240, 0, 0],
    [237, 0, 0],
    [234, 0, 0],
    [231, 0, 0],
    [229, 0, 0],
    [228, 0, 0],
    [225, 0, 0],
    [222, 0, 0],
    [221, 0, 0],
    [219, 0, 0],
    [216, 0, 0],
    [213, 0, 0],
    [212, 0, 0],
    [210, 0, 0],
    [207, 0, 0],
    [204, 0, 0],
    [201, 0, 0],
    [199, 0, 0],
    [196, 0, 0],
    [193, 0, 0],
    [192, 0, 0],
    [190, 0, 0],
    [188, 0, 0],
    [184, 0, 0],
    [183, 0, 0],
    [181, 0, 0],
    [179, 0, 0],
    [175, 0, 0],
    [174, 0, 0],
    [174, 0, 0],
];

fn background_values(values: &[f64], bands: usize) -> Vec<f64> {
    if values.is_empty() {
        vec![0.0; bands]
    } else {
        (0..bands)
            .map(|band| values.get(band).copied().unwrap_or(values[0]))
            .collect()
    }
}

fn common_format_many(formats: impl Iterator<Item = VipsBandFormat>) -> Result<VipsBandFormat, ()> {
    let mut formats = formats;
    let first = formats.next().ok_or(())?;
    formats.try_fold(first, |acc, format| common_format(acc, format).ok_or(()))
}

fn white_background_value(format: VipsBandFormat) -> f64 {
    match format_kind(format) {
        Some(NumericKind::Unsigned) => format_max(format).unwrap_or(255.0),
        Some(NumericKind::Signed) => -1.0,
        _ => 255.0,
    }
}

fn align_offset(inner: usize, outer: usize, align: VipsAlign) -> usize {
    match align {
        VIPS_ALIGN_LOW => 0,
        VIPS_ALIGN_CENTRE => outer.saturating_sub(inner) / 2,
        VIPS_ALIGN_HIGH => outer.saturating_sub(inner),
        _ => 0,
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
    let white = white_background_value(input.spec.format);

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
                            let sx =
                                sx.clamp(0, input.spec.width.saturating_sub(1) as isize) as usize;
                            let sy =
                                sy.clamp(0, input.spec.height.saturating_sub(1) as isize) as usize;
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
                            input.get(
                                mirror(sx, input.spec.width),
                                mirror(sy, input.spec.height),
                                band,
                            )
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

fn extract_area_buffer(
    input: &ImageBuffer,
    left: usize,
    top: usize,
    width: usize,
    height: usize,
) -> Result<ImageBuffer, ()> {
    let right = left.checked_add(width).ok_or(())?;
    let bottom = top.checked_add(height).ok_or(())?;
    if width == 0 || height == 0 || right > input.spec.width || bottom > input.spec.height {
        return Err(());
    }

    let mut out = input
        .with_shape(width, height, input.spec.bands)
        .with_origin(-(left as i32), -(top as i32))
        .with_demand_style(VIPS_DEMAND_STYLE_THINSTRIP);
    for y in 0..height {
        for x in 0..width {
            for band in 0..input.spec.bands {
                out.set(x, y, band, input.get(left + x, top + y, band));
            }
        }
    }

    Ok(out)
}

unsafe fn op_extract_area(
    object: *mut VipsObject,
    input_name: &str,
    output_name: &str,
) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, input_name)? };
    let left = unsafe { get_int(object, "left")? };
    let top = unsafe { get_int(object, "top")? };
    let width = unsafe { get_int(object, "width")? };
    let height = unsafe { get_int(object, "height")? };

    if left < 0 || top < 0 || width <= 0 || height <= 0 {
        append_message_str("extract_area", "bad extract area");
        return Err(());
    }

    let left_u = usize::try_from(left).map_err(|_| ())?;
    let top_u = usize::try_from(top).map_err(|_| ())?;
    let width_u = usize::try_from(width).map_err(|_| ())?;
    let height_u = usize::try_from(height).map_err(|_| ())?;

    let out = match extract_area_buffer(&input, left_u, top_u, width_u, height_u) {
        Ok(out) => out,
        Err(()) => {
            append_message_str("extract_area", "bad extract area");
            return Err(());
        }
    };

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
    let mut out = first_buffer
        .with_shape(width, height, total_bands)
        .with_format(format);

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
            let sum = (0..input.spec.bands)
                .map(|band| input.get(x, y, band))
                .sum::<f64>();
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

unsafe fn op_bandrank(object: *mut VipsObject) -> Result<(), ()> {
    let images = unsafe { get_array_images(object, "in")? };
    let first = *images.first().ok_or(())?;
    let buffers = images
        .iter()
        .map(|image| ImageBuffer::from_image(*image))
        .collect::<Result<Vec<_>, _>>()?;
    let n = buffers.len();
    if n == 0 {
        return Err(());
    }

    let target_bands = buffers.iter().try_fold(0usize, |current, buffer| {
        if current == 0 || buffer.spec.bands == current {
            Ok(current.max(buffer.spec.bands))
        } else if buffer.spec.bands == 1 {
            Ok(current)
        } else if current == 1 {
            Ok(buffer.spec.bands)
        } else {
            Err(())
        }
    })?;
    let format = buffers
        .iter()
        .skip(1)
        .try_fold(buffers[0].spec.format, |format, buffer| {
            common_format(format, buffer.spec.format).ok_or(())
        })?;
    let width = buffers
        .iter()
        .map(|buffer| buffer.spec.width)
        .max()
        .unwrap_or(0);
    let height = buffers
        .iter()
        .map(|buffer| buffer.spec.height)
        .max()
        .unwrap_or(0);

    let buffers = buffers
        .iter()
        .map(|buffer| {
            replicate_to_bands(buffer, target_bands)
                .map(|buffer| buffer.with_format(format).zero_extend(width, height))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let index = if unsafe { argument_assigned(object, "index")? } {
        usize::try_from(unsafe { get_int(object, "index")? }).map_err(|_| ())?
    } else {
        n / 2
    };
    if index >= n {
        return Err(());
    }

    let mut out = buffers[0]
        .with_shape(width, height, target_bands)
        .with_format(format);
    let mut values = vec![0.0; n];
    for y in 0..height {
        for x in 0..width {
            for band in 0..target_bands {
                for (i, buffer) in buffers.iter().enumerate() {
                    values[i] = buffer.get(x, y, band);
                }
                values.sort_by(|left, right| left.total_cmp(right));
                out.set(x, y, band, values[index]);
            }
        }
    }

    let image = unsafe { get_image_ref(object, "out") }
        .or_else(|_| unsafe { get_image_ref(object, "in") })
        .or_else(|_| Ok(first))?;
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    if image != first {
        unsafe {
            crate::runtime::object::object_unref(image);
        }
    }
    result
}

fn align_bandalike(
    buffers: &[&ImageBuffer],
    base_bands: usize,
) -> Result<(usize, VipsInterpretation), ()> {
    let mut target_bands = base_bands;
    let mut interpretation = VipsInterpretation::default();
    let mut found = false;

    for buffer in buffers {
        if buffer.spec.bands >= target_bands {
            target_bands = buffer.spec.bands;
            interpretation = buffer.spec.interpretation;
            found = true;
        }
    }

    if !found || target_bands == 0 {
        Err(())
    } else {
        Ok((target_bands, interpretation))
    }
}

fn ifthenelse_blend_value(
    format: VipsBandFormat,
    cond: f64,
    then_value: f64,
    else_value: f64,
) -> f64 {
    let alpha = clamp_for_format(cond, VIPS_FORMAT_UCHAR).clamp(0.0, 255.0);
    match format_kind(format) {
        Some(NumericKind::Unsigned | NumericKind::Signed) => {
            let then_value = clamp_for_format(then_value, format);
            let else_value = clamp_for_format(else_value, format);
            ((alpha * then_value + (255.0 - alpha) * else_value + 128.0) / 255.0).floor()
        }
        _ => {
            let alpha = alpha / 255.0;
            alpha * then_value + (1.0 - alpha) * else_value
        }
    }
}

unsafe fn op_ifthenelse(object: *mut VipsObject) -> Result<(), ()> {
    let cond = unsafe { get_image_buffer(object, "cond")? };
    let in1 = unsafe { get_image_buffer(object, "in1")? };
    let in2 = unsafe { get_image_buffer(object, "in2")? };
    let blend =
        unsafe { argument_assigned(object, "blend")? } && unsafe { get_bool(object, "blend")? };

    let (target_bands, interpretation) = align_bandalike(&[&in1, &in2, &cond], 0)?;
    let format = common_format(in1.spec.format, in2.spec.format).ok_or(())?;
    let width = in1.spec.width.max(in2.spec.width).max(cond.spec.width);
    let height = in1.spec.height.max(in2.spec.height).max(cond.spec.height);

    let in1 = replicate_to_bands(&in1, target_bands)?
        .with_format(format)
        .zero_extend(width, height);
    let in2 = replicate_to_bands(&in2, target_bands)?
        .with_format(format)
        .zero_extend(width, height);
    let cond = replicate_to_bands(&cond, target_bands)?
        .with_format(VIPS_FORMAT_UCHAR)
        .zero_extend(width, height);

    let mut out = in1
        .with_shape(width, height, target_bands)
        .with_format(format);
    out.spec.interpretation = interpretation;

    for y in 0..height {
        for x in 0..width {
            for band in 0..target_bands {
                let cond_value = cond.get(x, y, band);
                let value = if blend {
                    ifthenelse_blend_value(
                        format,
                        cond_value,
                        in1.get(x, y, band),
                        in2.get(x, y, band),
                    )
                } else if clamp_for_format(cond_value, VIPS_FORMAT_UCHAR) != 0.0 {
                    in1.get(x, y, band)
                } else {
                    in2.get(x, y, band)
                };
                out.set(x, y, band, value);
            }
        }
    }

    let image = unsafe { get_image_ref(object, "in1")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

fn bandbool_reduce(
    values: impl Iterator<Item = f64>,
    format: VipsBandFormat,
    op: VipsOperationBoolean,
) -> f64 {
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
            let value = bandbool_reduce(
                (0..input.spec.bands).map(|band| input.get(x, y, band)),
                input.spec.format,
                op,
            );
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
    let factor = if factor == 0 {
        input.spec.width
    } else {
        factor
    };
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
    let factor = if factor == 0 {
        input.spec.bands
    } else {
        factor
    };
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

unsafe fn op_gravity(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let direction = unsafe { get_enum(object, "direction")? } as VipsCompassDirection;
    let width_i32 = unsafe { get_int(object, "width")? };
    let height_i32 = unsafe { get_int(object, "height")? };
    if width_i32 <= 0 || height_i32 <= 0 {
        return Err(());
    }

    let width = usize::try_from(width_i32).map_err(|_| ())?;
    let height = usize::try_from(height_i32).map_err(|_| ())?;
    let in_width = i32::try_from(input.spec.width).map_err(|_| ())?;
    let in_height = i32::try_from(input.spec.height).map_err(|_| ())?;
    let x_mid = (width_i32 - in_width) / 2;
    let y_mid = (height_i32 - in_height) / 2;
    let x_high = width_i32 - in_width;
    let y_high = height_i32 - in_height;
    let (x, y) = match direction {
        VIPS_COMPASS_DIRECTION_CENTRE => (x_mid, y_mid),
        VIPS_COMPASS_DIRECTION_NORTH => (x_mid, 0),
        VIPS_COMPASS_DIRECTION_EAST => (x_high, y_mid),
        VIPS_COMPASS_DIRECTION_SOUTH => (x_mid, y_high),
        VIPS_COMPASS_DIRECTION_WEST => (0, y_mid),
        VIPS_COMPASS_DIRECTION_NORTH_EAST => (x_high, 0),
        VIPS_COMPASS_DIRECTION_SOUTH_EAST => (x_high, y_high),
        VIPS_COMPASS_DIRECTION_SOUTH_WEST => (0, y_high),
        VIPS_COMPASS_DIRECTION_NORTH_WEST => (0, 0),
        _ => return Err(()),
    };
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

    let out = apply_embed_background(
        &input,
        width,
        height,
        x as isize,
        y as isize,
        extend,
        &background,
    );
    let image = unsafe { get_image_ref(object, "in")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

unsafe fn op_replicate(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let across = usize::try_from(unsafe { get_int(object, "across")? }).map_err(|_| ())?;
    let down = usize::try_from(unsafe { get_int(object, "down")? }).map_err(|_| ())?;
    if across == 0 || down == 0 {
        return Err(());
    }

    let width = input.spec.width.checked_mul(across).ok_or(())?;
    let height = input.spec.height.checked_mul(down).ok_or(())?;
    let mut out = input.with_shape(width, height, input.spec.bands);
    for y in 0..height {
        for x in 0..width {
            let sx = x % input.spec.width;
            let sy = y % input.spec.height;
            for band in 0..input.spec.bands {
                out.set(x, y, band, input.get(sx, sy, band));
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

fn insert_buffers(
    main: &ImageBuffer,
    sub: &ImageBuffer,
    x: i32,
    y: i32,
    expand: bool,
    background: &[f64],
) -> Result<ImageBuffer, ()> {
    let format = common_format(main.spec.format, sub.spec.format).ok_or(())?;
    let bands = match (main.spec.bands, sub.spec.bands) {
        (a, b) if a == b => a,
        (1, b) => b,
        (a, 1) => a,
        _ => return Err(()),
    };
    let main = if main.spec.bands == bands {
        main.with_format(format)
    } else {
        main.replicate_bands(bands)?.with_format(format)
    };
    let sub = if sub.spec.bands == bands {
        sub.with_format(format)
    } else {
        sub.replicate_bands(bands)?.with_format(format)
    };

    let (out_width, out_height, main_offset_x, main_offset_y, sub_offset_x, sub_offset_y) =
        if expand {
            let left = 0.min(x);
            let top = 0.min(y);
            let right = (main.spec.width as i32).max(x.saturating_add(sub.spec.width as i32));
            let bottom = (main.spec.height as i32).max(y.saturating_add(sub.spec.height as i32));
            (
                usize::try_from(right.saturating_sub(left)).map_err(|_| ())?,
                usize::try_from(bottom.saturating_sub(top)).map_err(|_| ())?,
                usize::try_from(-left).map_err(|_| ())?,
                usize::try_from(-top).map_err(|_| ())?,
                usize::try_from(x.saturating_sub(left)).map_err(|_| ())?,
                usize::try_from(y.saturating_sub(top)).map_err(|_| ())?,
            )
        } else {
            if x < 0 || y < 0 {
                return Err(());
            }
            (
                main.spec.width,
                main.spec.height,
                0,
                0,
                usize::try_from(x).map_err(|_| ())?,
                usize::try_from(y).map_err(|_| ())?,
            )
        };

    let bg = background_values(background, bands);
    let mut out = ImageBuffer::new(
        out_width,
        out_height,
        bands,
        format,
        main.spec.coding,
        main.spec.interpretation,
    );
    out.spec.xres = main.spec.xres;
    out.spec.yres = main.spec.yres;
    out.spec.xoffset = main.spec.xoffset;
    out.spec.yoffset = main.spec.yoffset;
    out.spec.dhint = main.spec.dhint;

    if expand {
        for oy in 0..out_height {
            for ox in 0..out_width {
                for band in 0..bands {
                    out.set(ox, oy, band, bg[band]);
                }
            }
        }
    }

    for sy in 0..main.spec.height {
        let oy = main_offset_y + sy;
        if oy >= out_height {
            continue;
        }
        for sx in 0..main.spec.width {
            let ox = main_offset_x + sx;
            if ox >= out_width {
                continue;
            }
            for band in 0..bands {
                out.set(ox, oy, band, main.get(sx, sy, band));
            }
        }
    }

    for sy in 0..sub.spec.height {
        let Some(oy) = sub_offset_y.checked_add(sy) else {
            continue;
        };
        if oy >= out_height {
            continue;
        }
        for sx in 0..sub.spec.width {
            let Some(ox) = sub_offset_x.checked_add(sx) else {
                continue;
            };
            if ox >= out_width {
                continue;
            }
            for band in 0..bands {
                out.set(ox, oy, band, sub.get(sx, sy, band));
            }
        }
    }

    Ok(out)
}

unsafe fn op_insert(object: *mut VipsObject) -> Result<(), ()> {
    let main = unsafe { get_image_buffer(object, "main")? };
    let sub = unsafe { get_image_buffer(object, "sub")? };
    let x = unsafe { get_int(object, "x")? };
    let y = unsafe { get_int(object, "y")? };
    let expand =
        unsafe { argument_assigned(object, "expand")? } && unsafe { get_bool(object, "expand")? };
    let background = if unsafe { argument_assigned(object, "background")? } {
        unsafe { get_array_double(object, "background")? }
    } else {
        vec![0.0]
    };
    let out = insert_buffers(&main, &sub, x, y, expand, &background)?;

    let image = unsafe { get_image_ref(object, "main")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

unsafe fn op_cache(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let image = unsafe { get_image_ref(object, "in")? };
    let result = unsafe { set_output_image_like(object, "out", input, image) };
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

unsafe fn op_flip(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let direction = unsafe { get_enum(object, "direction")? } as VipsDirection;
    let mut out = input.with_shape(input.spec.width, input.spec.height, input.spec.bands);

    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            let (sx, sy) = match direction {
                VIPS_DIRECTION_HORIZONTAL => (input.spec.width - 1 - x, y),
                VIPS_DIRECTION_VERTICAL => (x, input.spec.height - 1 - y),
                _ => return Err(()),
            };
            for band in 0..input.spec.bands {
                out.set(x, y, band, input.get(sx, sy, band));
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

fn gamma_max_value(format: VipsBandFormat) -> f64 {
    match format {
        crate::abi::image::VIPS_FORMAT_UCHAR => u8::MAX as f64,
        crate::abi::image::VIPS_FORMAT_CHAR => i8::MAX as f64,
        crate::abi::image::VIPS_FORMAT_USHORT => u16::MAX as f64,
        crate::abi::image::VIPS_FORMAT_SHORT => i16::MAX as f64,
        crate::abi::image::VIPS_FORMAT_UINT => u32::MAX as f64,
        crate::abi::image::VIPS_FORMAT_INT => i32::MAX as f64,
        _ => 1.0,
    }
}

unsafe fn op_gamma(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let exponent = if unsafe { argument_assigned(object, "exponent")? } {
        unsafe { get_double(object, "exponent")? }
    } else {
        1.0 / 2.4
    };
    if exponent <= 0.0 {
        return Err(());
    }
    let power = 1.0 / exponent;
    let max_value = gamma_max_value(input.spec.format);
    let scale = max_value.powf(power) / max_value.max(f64::EPSILON);
    let mut out = input.with_shape(input.spec.width, input.spec.height, input.spec.bands);

    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            for band in 0..input.spec.bands {
                let value = input.get(x, y, band);
                let value = if value <= 0.0 {
                    0.0
                } else {
                    value.powf(power) / scale
                };
                out.set(x, y, band, value);
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

unsafe fn op_grid(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let tile_height =
        usize::try_from(unsafe { get_int(object, "tile_height")? }).map_err(|_| ())?;
    let across = usize::try_from(unsafe { get_int(object, "across")? }).map_err(|_| ())?;
    let down = usize::try_from(unsafe { get_int(object, "down")? }).map_err(|_| ())?;
    if tile_height == 0 || across == 0 || down == 0 {
        return Err(());
    }
    let out_width = input.spec.width.checked_mul(across).ok_or(())?;
    let out_height = tile_height.checked_mul(down).ok_or(())?;
    if tile_height
        .checked_mul(across.checked_mul(down).ok_or(())?)
        .ok_or(())?
        > input.spec.height
    {
        return Err(());
    }

    let mut out = input.with_shape(out_width, out_height, input.spec.bands);
    for index in 0..across.checked_mul(down).ok_or(())? {
        let src_top = index * tile_height;
        let dst_left = (index % across) * input.spec.width;
        let dst_top = (index / across) * tile_height;
        for y in 0..tile_height {
            for x in 0..input.spec.width {
                for band in 0..input.spec.bands {
                    out.set(
                        dst_left + x,
                        dst_top + y,
                        band,
                        input.get(x, src_top + y, band),
                    );
                }
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

unsafe fn op_switch(object: *mut VipsObject) -> Result<(), ()> {
    let images = unsafe { get_array_images(object, "tests")? };
    if images.is_empty() || images.len() > 255 {
        return Err(());
    }
    let mut buffers = images
        .iter()
        .map(|image| ImageBuffer::from_image(*image))
        .collect::<Result<Vec<_>, _>>()?;
    let width = buffers
        .iter()
        .map(|buffer| buffer.spec.width)
        .max()
        .unwrap_or(0);
    let height = buffers
        .iter()
        .map(|buffer| buffer.spec.height)
        .max()
        .unwrap_or(0);
    if width == 0 || height == 0 {
        return Err(());
    }
    for buffer in &mut buffers {
        if buffer.spec.bands != 1 {
            return Err(());
        }
        *buffer = buffer
            .with_format(VIPS_FORMAT_UCHAR)
            .zero_extend(width, height);
    }

    let mut out = ImageBuffer::new(
        width,
        height,
        1,
        VIPS_FORMAT_UCHAR,
        buffers[0].spec.coding,
        buffers[0].spec.interpretation,
    );
    for y in 0..height {
        for x in 0..width {
            let mut index = buffers.len();
            for (i, buffer) in buffers.iter().enumerate() {
                if clamp_for_format(buffer.get(x, y, 0), VIPS_FORMAT_UCHAR) != 0.0 {
                    index = i;
                    break;
                }
            }
            out.set(x, y, 0, index as f64);
        }
    }

    let result = unsafe { set_output_image_like(object, "out", out, images[0]) };
    Ok(result?)
}

unsafe fn op_arrayjoin(object: *mut VipsObject) -> Result<(), ()> {
    let images = unsafe { get_array_images(object, "in")? };
    let first = *images.first().ok_or(())?;
    let buffers = images
        .iter()
        .map(|image| ImageBuffer::from_image(*image))
        .collect::<Result<Vec<_>, _>>()?;
    let background = if unsafe { argument_assigned(object, "background")? } {
        unsafe { get_array_double(object, "background")? }
    } else {
        vec![0.0]
    };
    let background_bands = background.len().max(1);
    let target_bands = buffers
        .iter()
        .map(|buffer| buffer.spec.bands)
        .max()
        .unwrap_or(0)
        .max(background_bands);
    let format = common_format_many(buffers.iter().map(|buffer| buffer.spec.format))?;
    let max_width = buffers
        .iter()
        .map(|buffer| buffer.spec.width)
        .max()
        .unwrap_or(0);
    let max_height = buffers
        .iter()
        .map(|buffer| buffer.spec.height)
        .max()
        .unwrap_or(0);
    let hspacing = if unsafe { argument_assigned(object, "hspacing")? } {
        usize::try_from(unsafe { get_int(object, "hspacing")? }).map_err(|_| ())?
    } else {
        max_width
    };
    let vspacing = if unsafe { argument_assigned(object, "vspacing")? } {
        usize::try_from(unsafe { get_int(object, "vspacing")? }).map_err(|_| ())?
    } else {
        max_height
    };
    let across = if unsafe { argument_assigned(object, "across")? } {
        usize::try_from(unsafe { get_int(object, "across")? }).map_err(|_| ())?
    } else {
        buffers.len()
    };
    let shim = if unsafe { argument_assigned(object, "shim")? } {
        usize::try_from(unsafe { get_int(object, "shim")? }).map_err(|_| ())?
    } else {
        0
    };
    let halign = if unsafe { argument_assigned(object, "halign")? } {
        unsafe { get_enum(object, "halign")? as VipsAlign }
    } else {
        VIPS_ALIGN_LOW
    };
    let valign = if unsafe { argument_assigned(object, "valign")? } {
        unsafe { get_enum(object, "valign")? as VipsAlign }
    } else {
        VIPS_ALIGN_LOW
    };
    if across == 0 || hspacing == 0 || vspacing == 0 {
        return Err(());
    }
    let down = buffers.len().div_ceil(across);
    let out_width = hspacing
        .checked_mul(across)
        .and_then(|value| value.checked_add(shim.saturating_mul(across.saturating_sub(1))))
        .ok_or(())?;
    let out_height = vspacing
        .checked_mul(down)
        .and_then(|value| value.checked_add(shim.saturating_mul(down.saturating_sub(1))))
        .ok_or(())?;
    let interpretation = buffers
        .iter()
        .find(|buffer| buffer.spec.bands == target_bands)
        .map(|buffer| buffer.spec.interpretation)
        .unwrap_or(buffers[0].spec.interpretation);

    let buffers = buffers
        .iter()
        .map(|buffer| {
            replicate_to_bands(buffer, target_bands).map(|buffer| buffer.with_format(format))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut out = ImageBuffer::new(
        out_width,
        out_height,
        target_bands,
        format,
        buffers[0].spec.coding,
        interpretation,
    );
    let bg = background_values(&background, target_bands);
    for y in 0..out_height {
        for x in 0..out_width {
            for band in 0..target_bands {
                out.set(x, y, band, bg[band]);
            }
        }
    }

    for cell in 0..across.checked_mul(down).ok_or(())? {
        let buffer = &buffers[cell.min(buffers.len() - 1)];
        let cell_x = (cell % across) * (hspacing + shim);
        let cell_y = (cell / across) * (vspacing + shim);
        let dx = cell_x + align_offset(buffer.spec.width, hspacing, halign);
        let dy = cell_y + align_offset(buffer.spec.height, vspacing, valign);
        for y in 0..buffer.spec.height {
            for x in 0..buffer.spec.width {
                for band in 0..target_bands {
                    out.set(dx + x, dy + y, band, buffer.get(x, y, band));
                }
            }
        }
    }

    unsafe { set_output_image_like(object, "out", out, first) }
}

unsafe fn op_join(object: *mut VipsObject) -> Result<(), ()> {
    let in1 = unsafe { get_image_buffer(object, "in1")? };
    let in2 = unsafe { get_image_buffer(object, "in2")? };
    let direction = unsafe { get_enum(object, "direction")? } as VipsDirection;
    let expand =
        unsafe { argument_assigned(object, "expand")? } && unsafe { get_bool(object, "expand")? };
    let shim = if unsafe { argument_assigned(object, "shim")? } {
        unsafe { get_int(object, "shim")? }
    } else {
        0
    };
    let background = if unsafe { argument_assigned(object, "background")? } {
        unsafe { get_array_double(object, "background")? }
    } else {
        vec![0.0]
    };
    let align = if unsafe { argument_assigned(object, "align")? } {
        unsafe { get_enum(object, "align")? as VipsAlign }
    } else {
        VIPS_ALIGN_LOW
    };

    let (x, y) = match direction {
        VIPS_DIRECTION_HORIZONTAL => {
            let y = match align {
                VIPS_ALIGN_LOW => 0,
                VIPS_ALIGN_CENTRE => in1.spec.height as i32 / 2 - in2.spec.height as i32 / 2,
                VIPS_ALIGN_HIGH => in1.spec.height as i32 - in2.spec.height as i32,
                _ => return Err(()),
            };
            (in1.spec.width as i32 + shim, y)
        }
        VIPS_DIRECTION_VERTICAL => {
            let x = match align {
                VIPS_ALIGN_LOW => 0,
                VIPS_ALIGN_CENTRE => in1.spec.width as i32 / 2 - in2.spec.width as i32 / 2,
                VIPS_ALIGN_HIGH => in1.spec.width as i32 - in2.spec.width as i32,
                _ => return Err(()),
            };
            (x, in1.spec.height as i32 + shim)
        }
        _ => return Err(()),
    };

    let joined = insert_buffers(&in1, &in2, x, y, true, &background)?;
    let out = if expand {
        joined
    } else {
        match direction {
            VIPS_DIRECTION_HORIZONTAL => {
                let left = (0.max(y) - y) as usize;
                let height = in1.spec.height.min(in2.spec.height);
                extract_area_buffer(&joined, 0, left, joined.spec.width, height)?
            }
            VIPS_DIRECTION_VERTICAL => {
                let left = (0.max(x) - x) as usize;
                let width = in1.spec.width.min(in2.spec.width);
                extract_area_buffer(&joined, left, 0, width, joined.spec.height)?
            }
            _ => return Err(()),
        }
    };

    let image = unsafe { get_image_ref(object, "in1")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

unsafe fn op_composite2(object: *mut VipsObject) -> Result<(), ()> {
    let Ok(images) = (unsafe { get_array_images(object, "in") }) else {
        append_message_str("composite2", "reading input image array failed");
        return Err(());
    };
    if images.len() != 2 {
        append_message_str("composite2", "expected exactly two input images");
        return Err(());
    }
    let like_image = images[0];
    let Ok(base) = ImageBuffer::from_image(images[0]) else {
        append_message_str("composite2", "decoding base image failed");
        return Err(());
    };
    let Ok(overlay) = ImageBuffer::from_image(images[1]) else {
        append_message_str("composite2", "decoding overlay image failed");
        return Err(());
    };

    let x = if unsafe { argument_assigned(object, "x")? } {
        unsafe { get_int(object, "x")? }
    } else {
        0
    };
    let y = if unsafe { argument_assigned(object, "y")? } {
        unsafe { get_int(object, "y")? }
    } else {
        0
    };

    let Ok(out) = composite2_over_buffers(&base, &overlay, x, y) else {
        append_message_str("composite2", "compositing failed");
        return Err(());
    };
    let result = unsafe { set_output_image_like(object, "out", out, like_image) };
    if result.is_err() {
        append_message_str("composite2", "setting output image failed");
    }
    result
}

fn composite2_over_buffers(
    base: &ImageBuffer,
    overlay: &ImageBuffer,
    x: i32,
    y: i32,
) -> Result<ImageBuffer, ()> {
    let target_bands = base.spec.bands.max(overlay.spec.bands);
    if !matches!(target_bands, 1 | 2 | 4) {
        return Err(());
    }
    let format = common_format(base.spec.format, overlay.spec.format).ok_or(())?;
    let base = replicate_to_bands(base, target_bands)?.with_format(format);
    let overlay = replicate_to_bands(overlay, target_bands)?.with_format(format);
    let mut out = base.clone();

    let alpha_enabled = matches!(target_bands, 2 | 4);
    let alpha_band = target_bands.saturating_sub(1);
    let max_alpha = vips_interpretation_max_alpha(base.spec.interpretation).max(f64::EPSILON);

    for oy in 0..overlay.spec.height {
        let dy = oy as i32 + y;
        if dy < 0 || dy >= out.spec.height as i32 {
            continue;
        }
        for ox in 0..overlay.spec.width {
            let dx = ox as i32 + x;
            if dx < 0 || dx >= out.spec.width as i32 {
                continue;
            }
            let dx = dx as usize;
            let dy = dy as usize;
            if alpha_enabled {
                let oa = overlay.get(ox, oy, alpha_band).clamp(0.0, max_alpha) / max_alpha;
                if oa <= 0.0 {
                    continue;
                }
                let ba = base.get(dx, dy, alpha_band).clamp(0.0, max_alpha) / max_alpha;
                let out_a = oa + ba * (1.0 - oa);
                for band in 0..alpha_band {
                    let ov = overlay.get(ox, oy, band);
                    let bv = base.get(dx, dy, band);
                    let value = if out_a <= f64::EPSILON {
                        0.0
                    } else {
                        (ov * oa + bv * ba * (1.0 - oa)) / out_a
                    };
                    out.set(dx, dy, band, value);
                }
                out.set(dx, dy, alpha_band, out_a * max_alpha);
            } else {
                out.set(dx, dy, 0, overlay.get(ox, oy, 0));
            }
        }
    }

    Ok(out)
}

#[no_mangle]
pub extern "C" fn safe_vips_composite2_internal(
    base: *mut crate::abi::image::VipsImage,
    overlay: *mut crate::abi::image::VipsImage,
    out: *mut *mut crate::abi::image::VipsImage,
    x: libc::c_int,
    y: libc::c_int,
) -> libc::c_int {
    let base_image = base;
    let Ok(base) = ImageBuffer::from_image(base) else {
        append_message_str("composite2", "decoding base image failed");
        return -1;
    };
    let Ok(overlay) = ImageBuffer::from_image(overlay) else {
        append_message_str("composite2", "decoding overlay image failed");
        return -1;
    };
    let Ok(result) = composite2_over_buffers(&base, &overlay, x, y) else {
        append_message_str("composite2", "compositing failed");
        return -1;
    };
    let image = result.into_image_like(base_image);
    if out.is_null() {
        unsafe {
            crate::runtime::object::object_unref(image);
        }
        append_message_str("composite2", "output pointer is null");
        return -1;
    }
    unsafe {
        *out = image;
    }
    0
}

unsafe fn op_msb(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let format = input.spec.format;
    let shift = format_bytes(format).saturating_sub(1) * 8;
    let selected_band = if unsafe { argument_assigned(object, "band")? } {
        Some(usize::try_from(unsafe { get_int(object, "band")? }).map_err(|_| ())?)
    } else {
        None
    };
    let out_bands = if selected_band.is_some() {
        1
    } else {
        input.spec.bands
    };
    let mut out = ImageBuffer::new(
        input.spec.width,
        input.spec.height,
        out_bands,
        VIPS_FORMAT_UCHAR,
        input.spec.coding,
        input.spec.interpretation,
    );
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            for ob in 0..out_bands {
                let band = selected_band.unwrap_or(ob);
                if band >= input.spec.bands {
                    return Err(());
                }
                let value = input.get(x, y, band) as i64;
                let msb = match format_kind(format) {
                    Some(NumericKind::Unsigned) => (value >> shift) as f64,
                    Some(NumericKind::Signed) => (128i64 + (value >> shift)) as f64,
                    _ => return Err(()),
                };
                out.set(x, y, ob, msb);
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

unsafe fn op_recomb(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let matrix = unsafe { get_image_buffer(object, "m")? };
    if matrix.spec.bands != 1 || matrix.spec.width != input.spec.bands {
        return Err(());
    }
    let out_format = if input.spec.format == VIPS_FORMAT_DOUBLE {
        VIPS_FORMAT_DOUBLE
    } else {
        VIPS_FORMAT_FLOAT
    };
    let mut out = ImageBuffer::new(
        input.spec.width,
        input.spec.height,
        matrix.spec.height,
        out_format,
        input.spec.coding,
        input.spec.interpretation,
    );
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            for row in 0..matrix.spec.height {
                let mut sum = 0.0;
                for col in 0..matrix.spec.width {
                    sum += matrix.get(col, row, 0) * input.get(x, y, col);
                }
                out.set(x, y, row, sum);
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

fn rotate_quadrant(input: &ImageBuffer, angle: VipsAngle) -> Result<ImageBuffer, ()> {
    match angle {
        VIPS_ANGLE_D0 => Ok(input.clone()),
        VIPS_ANGLE_D90 => {
            let mut out = input.with_shape(input.spec.height, input.spec.width, input.spec.bands);
            for y in 0..out.spec.height {
                for x in 0..out.spec.width {
                    for band in 0..out.spec.bands {
                        out.set(x, y, band, input.get(y, input.spec.height - 1 - x, band));
                    }
                }
            }
            Ok(out)
        }
        VIPS_ANGLE_D180 => {
            let mut out = input.with_shape(input.spec.width, input.spec.height, input.spec.bands);
            for y in 0..out.spec.height {
                for x in 0..out.spec.width {
                    for band in 0..out.spec.bands {
                        out.set(
                            x,
                            y,
                            band,
                            input.get(input.spec.width - 1 - x, input.spec.height - 1 - y, band),
                        );
                    }
                }
            }
            Ok(out)
        }
        VIPS_ANGLE_D270 => {
            let mut out = input.with_shape(input.spec.height, input.spec.width, input.spec.bands);
            for y in 0..out.spec.height {
                for x in 0..out.spec.width {
                    for band in 0..out.spec.bands {
                        out.set(x, y, band, input.get(input.spec.width - 1 - y, x, band));
                    }
                }
            }
            Ok(out)
        }
        _ => Err(()),
    }
}

fn rotate_45_once(input: &ImageBuffer) -> Result<ImageBuffer, ()> {
    if input.spec.width != input.spec.height || input.spec.width % 2 == 0 {
        return Err(());
    }
    let size = input.spec.width;
    let size_2 = size / 2;
    let mut out = input.with_shape(size, size, input.spec.bands);
    for y in 0..size_2 {
        for x in y..size_2 {
            let mut temp = vec![0.0; input.spec.bands];
            for band in 0..input.spec.bands {
                temp[band] = input.get(x, y, band);
            }
            for band in 0..input.spec.bands {
                out.set(x, y, band, input.get(y, size_2 - (x - y), band));
                out.set(
                    y,
                    size_2 - (x - y),
                    band,
                    input.get(y, (size - 1) - x, band),
                );
                out.set(
                    y,
                    (size - 1) - x,
                    band,
                    input.get(size_2 - (x - y), (size - 1) - y, band),
                );
                out.set(
                    size_2 - (x - y),
                    (size - 1) - y,
                    band,
                    input.get((size - 1) - x, (size - 1) - y, band),
                );
                out.set(
                    (size - 1) - x,
                    (size - 1) - y,
                    band,
                    input.get((size - 1) - y, (x - y) + size_2, band),
                );
                out.set(
                    (size - 1) - y,
                    (x - y) + size_2,
                    band,
                    input.get((size - 1) - y, x, band),
                );
                out.set(
                    (size - 1) - y,
                    x,
                    band,
                    input.get((x - y) + size_2, y, band),
                );
                out.set((x - y) + size_2, y, band, temp[band]);
            }
        }
    }
    for band in 0..input.spec.bands {
        out.set(size_2, size_2, band, input.get(size_2, size_2, band));
    }
    Ok(out)
}

unsafe fn op_rot(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let angle = unsafe { get_enum(object, "angle")? } as VipsAngle;
    let out = rotate_quadrant(&input, angle)?;
    let image = unsafe { get_image_ref(object, "in")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

unsafe fn op_rot45(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let angle = if unsafe { argument_assigned(object, "angle")? } {
        unsafe { get_enum(object, "angle")? as VipsAngle45 }
    } else {
        VIPS_ANGLE45_D45
    };
    let steps = match angle {
        VIPS_ANGLE45_D0 => 0,
        VIPS_ANGLE45_D45 => 1,
        VIPS_ANGLE45_D90 => 2,
        VIPS_ANGLE45_D135 => 3,
        VIPS_ANGLE45_D180 => 4,
        VIPS_ANGLE45_D225 => 5,
        VIPS_ANGLE45_D270 => 6,
        VIPS_ANGLE45_D315 => 7,
        _ => return Err(()),
    };
    let mut out = input.clone();
    for _ in 0..steps {
        out = rotate_45_once(&out)?;
    }
    let image = unsafe { get_image_ref(object, "in")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

unsafe fn op_scale(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    if input.data.is_empty() {
        return Err(());
    }
    let log = unsafe { argument_assigned(object, "log")? } && unsafe { get_bool(object, "log")? };
    let exp = if unsafe { argument_assigned(object, "exp")? } {
        unsafe { get_double(object, "exp")? }
    } else {
        0.25
    };
    let mut mn = f64::INFINITY;
    let mut mx = f64::NEG_INFINITY;
    for value in &input.data {
        mn = mn.min(*value);
        mx = mx.max(*value);
    }
    let mut out = ImageBuffer::new(
        input.spec.width,
        input.spec.height,
        input.spec.bands,
        VIPS_FORMAT_UCHAR,
        input.spec.coding,
        input.spec.interpretation,
    );
    if (mn - mx).abs() < f64::EPSILON {
        let image = unsafe { get_image_ref(object, "in")? };
        let result = unsafe { set_output_image_like(object, "out", out, image) };
        unsafe {
            crate::runtime::object::object_unref(image);
        }
        return result;
    }
    if log {
        let factor = 255.0 / (1.0 + mx.powf(exp)).log10();
        for y in 0..input.spec.height {
            for x in 0..input.spec.width {
                for band in 0..input.spec.bands {
                    let value =
                        (1.0 + input.get(x, y, band).max(0.0).powf(exp)).log10() * factor + 0.5;
                    out.set(x, y, band, value);
                }
            }
        }
    } else {
        let factor = 255.0 / (mx - mn);
        let offset = -(mn * factor) + 0.5;
        for y in 0..input.spec.height {
            for x in 0..input.spec.width {
                for band in 0..input.spec.bands {
                    out.set(x, y, band, input.get(x, y, band) * factor + offset);
                }
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

unsafe fn op_subsample(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "input")? };
    let xfac = usize::try_from(unsafe { get_int(object, "xfac")? }).map_err(|_| ())?;
    let yfac = usize::try_from(unsafe { get_int(object, "yfac")? }).map_err(|_| ())?;
    if xfac == 0 || yfac == 0 {
        return Err(());
    }
    let out_width = input.spec.width / xfac;
    let out_height = input.spec.height / yfac;
    let mut out = input.with_shape(out_width, out_height, input.spec.bands);
    for y in 0..out_height {
        for x in 0..out_width {
            for band in 0..input.spec.bands {
                out.set(x, y, band, input.get(x * xfac, y * yfac, band));
            }
        }
    }
    let image = unsafe { get_image_ref(object, "input")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

unsafe fn op_zoom(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "input")? };
    let xfac = usize::try_from(unsafe { get_int(object, "xfac")? }).map_err(|_| ())?;
    let yfac = usize::try_from(unsafe { get_int(object, "yfac")? }).map_err(|_| ())?;
    if xfac == 0 || yfac == 0 {
        return Err(());
    }
    let out_width = input.spec.width.checked_mul(xfac).ok_or(())?;
    let out_height = input.spec.height.checked_mul(yfac).ok_or(())?;
    let mut out = input.with_shape(out_width, out_height, input.spec.bands);
    for y in 0..out_height {
        for x in 0..out_width {
            for band in 0..input.spec.bands {
                out.set(x, y, band, input.get(x / xfac, y / yfac, band));
            }
        }
    }
    let image = unsafe { get_image_ref(object, "input")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

unsafe fn op_wrap(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    if input.spec.width == 0 || input.spec.height == 0 {
        return Err(());
    }
    let requested_x = if unsafe { argument_assigned(object, "x")? } {
        unsafe { get_int(object, "x")? }
    } else {
        (input.spec.width / 2) as i32
    };
    let requested_y = if unsafe { argument_assigned(object, "y")? } {
        unsafe { get_int(object, "y")? }
    } else {
        (input.spec.height / 2) as i32
    };
    let x = if requested_x < 0 {
        (-requested_x as usize) % input.spec.width
    } else {
        input.spec.width - requested_x as usize % input.spec.width
    } % input.spec.width;
    let y = if requested_y < 0 {
        (-requested_y as usize) % input.spec.height
    } else {
        input.spec.height - requested_y as usize % input.spec.height
    } % input.spec.height;
    let mut out = input.with_shape(input.spec.width, input.spec.height, input.spec.bands);
    out.spec.xoffset = x as i32;
    out.spec.yoffset = y as i32;
    for oy in 0..out.spec.height {
        for ox in 0..out.spec.width {
            let sx = (ox + x) % input.spec.width;
            let sy = (oy + y) % input.spec.height;
            for band in 0..input.spec.bands {
                out.set(ox, oy, band, input.get(sx, sy, band));
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

unsafe fn op_premultiply(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    if input.spec.bands == 1 {
        let image = unsafe { get_image_ref(object, "in")? };
        let result = unsafe { set_output_image_like(object, "out", input, image) };
        unsafe {
            crate::runtime::object::object_unref(image);
        }
        return result;
    }
    let max_alpha = if unsafe { argument_assigned(object, "max_alpha")? } {
        unsafe { get_double(object, "max_alpha")? }
    } else {
        vips_interpretation_max_alpha(input.spec.interpretation)
    };
    let out = premultiply_buffer(&input, max_alpha);
    let image = unsafe { get_image_ref(object, "in")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

unsafe fn op_flatten(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    if input.spec.bands == 1 {
        let image = unsafe { get_image_ref(object, "in")? };
        let result = unsafe { set_output_image_like(object, "out", input, image) };
        unsafe {
            crate::runtime::object::object_unref(image);
        }
        return result;
    }
    let background = if unsafe { argument_assigned(object, "background")? } {
        unsafe { get_array_double(object, "background")? }
    } else {
        vec![0.0]
    };
    let max_alpha = if unsafe { argument_assigned(object, "max_alpha")? } {
        unsafe { get_double(object, "max_alpha")? }
    } else {
        vips_interpretation_max_alpha(input.spec.interpretation)
    };
    let bg = background_values(&background, input.spec.bands - 1);
    let mut out = input.with_shape(input.spec.width, input.spec.height, input.spec.bands - 1);
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            let alpha = input.get(x, y, input.spec.bands - 1).clamp(0.0, max_alpha);
            let nalpha = max_alpha - alpha;
            for band in 0..input.spec.bands - 1 {
                let value = (input.get(x, y, band) * alpha + bg[band] * nalpha)
                    / max_alpha.max(f64::EPSILON);
                out.set(x, y, band, value);
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

unsafe fn op_unpremultiply(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    if input.spec.bands == 1 {
        let image = unsafe { get_image_ref(object, "in")? };
        let result = unsafe { set_output_image_like(object, "out", input, image) };
        unsafe {
            crate::runtime::object::object_unref(image);
        }
        return result;
    }
    let max_alpha = if unsafe { argument_assigned(object, "max_alpha")? } {
        unsafe { get_double(object, "max_alpha")? }
    } else {
        vips_interpretation_max_alpha(input.spec.interpretation)
    };
    let alpha_band = if unsafe { argument_assigned(object, "alpha_band")? } {
        usize::try_from(unsafe { get_int(object, "alpha_band")? }).map_err(|_| ())?
    } else {
        input.spec.bands - 1
    };
    if alpha_band >= input.spec.bands {
        return Err(());
    }
    let out_format = if input.spec.format == VIPS_FORMAT_DOUBLE {
        VIPS_FORMAT_DOUBLE
    } else {
        VIPS_FORMAT_FLOAT
    };
    let mut out = input.with_shape(input.spec.width, input.spec.height, input.spec.bands);
    out.spec.format = out_format;
    let float_like = matches!(format_kind(input.spec.format), Some(NumericKind::Float));
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            let alpha = input.get(x, y, alpha_band);
            let factor = if float_like {
                if alpha.abs() < 0.01 {
                    0.0
                } else {
                    max_alpha / alpha
                }
            } else if alpha == 0.0 {
                0.0
            } else {
                max_alpha / alpha
            };
            for band in 0..alpha_band {
                out.set(x, y, band, input.get(x, y, band) * factor);
            }
            out.set(x, y, alpha_band, alpha.clamp(0.0, max_alpha));
            for band in alpha_band + 1..input.spec.bands {
                out.set(x, y, band, input.get(x, y, band));
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

unsafe fn op_falsecolour(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let mut out = ImageBuffer::new(
        input.spec.width,
        input.spec.height,
        3,
        VIPS_FORMAT_UCHAR,
        input.spec.coding,
        input.spec.interpretation,
    );
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            let index = clamp_for_format(input.get(x, y, 0), VIPS_FORMAT_UCHAR) as usize;
            let colour = FALSECOLOUR_PET[index];
            for (band, value) in colour.into_iter().enumerate() {
                out.set(x, y, band, value as f64);
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

fn extract_bands_buffer(input: &ImageBuffer, band: usize, n: usize) -> Result<ImageBuffer, ()> {
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

    Ok(out)
}

fn premultiply_buffer(input: &ImageBuffer, max_alpha: f64) -> ImageBuffer {
    if input.spec.bands == 1 {
        return input.clone();
    }

    let out_format = if input.spec.format == VIPS_FORMAT_DOUBLE {
        VIPS_FORMAT_DOUBLE
    } else {
        VIPS_FORMAT_FLOAT
    };
    let mut out = input.with_shape(input.spec.width, input.spec.height, input.spec.bands);
    out.spec.format = out_format;
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            let alpha = input.get(x, y, input.spec.bands - 1);
            let clip_alpha = alpha.clamp(0.0, max_alpha);
            let factor = clip_alpha / max_alpha.max(f64::EPSILON);
            for band in 0..input.spec.bands - 1 {
                out.set(x, y, band, input.get(x, y, band) * factor);
            }
            out.set(x, y, input.spec.bands - 1, alpha);
        }
    }
    out
}

fn unpremultiply_buffer(input: &ImageBuffer, max_alpha: f64, alpha_band: usize) -> ImageBuffer {
    if input.spec.bands == 1 {
        return input.clone();
    }

    let out_format = if input.spec.format == VIPS_FORMAT_DOUBLE {
        VIPS_FORMAT_DOUBLE
    } else {
        VIPS_FORMAT_FLOAT
    };
    let mut out = input.with_shape(input.spec.width, input.spec.height, input.spec.bands);
    out.spec.format = out_format;
    let float_like = matches!(format_kind(input.spec.format), Some(NumericKind::Float));
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            let alpha = input.get(x, y, alpha_band);
            let factor = if float_like {
                if alpha.abs() < 0.01 {
                    0.0
                } else {
                    max_alpha / alpha
                }
            } else if alpha == 0.0 {
                0.0
            } else {
                max_alpha / alpha
            };
            for band in 0..alpha_band {
                out.set(x, y, band, input.get(x, y, band) * factor);
            }
            out.set(x, y, alpha_band, alpha.clamp(0.0, max_alpha));
            for band in alpha_band + 1..input.spec.bands {
                out.set(x, y, band, input.get(x, y, band));
            }
        }
    }
    out
}

fn flatten_buffer(input: &ImageBuffer, background: &[f64], max_alpha: f64) -> ImageBuffer {
    if input.spec.bands == 1 {
        return input.clone();
    }

    let bg = background_values(background, input.spec.bands - 1);
    let mut out = input.with_shape(input.spec.width, input.spec.height, input.spec.bands - 1);
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            let alpha = input.get(x, y, input.spec.bands - 1).clamp(0.0, max_alpha);
            let nalpha = max_alpha - alpha;
            for band in 0..input.spec.bands - 1 {
                let value = (input.get(x, y, band) * alpha + bg[band] * nalpha)
                    / max_alpha.max(f64::EPSILON);
                out.set(x, y, band, value);
            }
        }
    }
    out
}

fn linear_buffer(input: &ImageBuffer, a: &[f64], b: &[f64]) -> Result<ImageBuffer, ()> {
    let target_bands = input.spec.bands.max(a.len().max(1)).max(b.len().max(1));
    for len in [input.spec.bands, a.len().max(1), b.len().max(1)] {
        if len != 1 && len != target_bands {
            return Err(());
        }
    }

    let input = if input.spec.bands == target_bands {
        input.clone()
    } else {
        input.replicate_bands(target_bands)?
    };
    let out_format = if input.spec.format == VIPS_FORMAT_DOUBLE {
        VIPS_FORMAT_DOUBLE
    } else {
        VIPS_FORMAT_FLOAT
    };
    let mut out = input.with_shape(input.spec.width, input.spec.height, target_bands);
    out.spec.format = out_format;
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            for band in 0..target_bands {
                let aa = a
                    .get(if a.len() <= 1 { 0 } else { band })
                    .copied()
                    .unwrap_or_else(|| *a.first().unwrap_or(&1.0));
                let bb = b
                    .get(if b.len() <= 1 { 0 } else { band })
                    .copied()
                    .unwrap_or_else(|| *b.first().unwrap_or(&0.0));
                out.set(x, y, band, input.get(x, y, band) * aa + bb);
            }
        }
    }
    Ok(out)
}

fn pythagoras_buffer(input: &ImageBuffer) -> ImageBuffer {
    let out_format = if input.spec.format == VIPS_FORMAT_DOUBLE {
        VIPS_FORMAT_DOUBLE
    } else {
        VIPS_FORMAT_FLOAT
    };
    let mut out = ImageBuffer::new(
        input.spec.width,
        input.spec.height,
        1,
        out_format,
        input.spec.coding,
        input.spec.interpretation,
    );
    out.spec.xres = input.spec.xres;
    out.spec.yres = input.spec.yres;
    out.spec.xoffset = input.spec.xoffset;
    out.spec.yoffset = input.spec.yoffset;
    out.spec.dhint = input.spec.dhint;
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            let mut sum = 0.0;
            for band in 0..input.spec.bands {
                let value = input.get(x, y, band);
                sum += value * value;
            }
            out.set(x, y, 0, sum.sqrt());
        }
    }
    out
}

fn divide_buffer(left: &ImageBuffer, right: &ImageBuffer) -> Result<ImageBuffer, ()> {
    let right = if right.spec.bands == left.spec.bands {
        right.clone()
    } else if right.spec.bands == 1 {
        right.replicate_bands(left.spec.bands)?
    } else {
        return Err(());
    };

    let mut out = left.with_shape(left.spec.width, left.spec.height, left.spec.bands);
    out.spec.format =
        if left.spec.format == VIPS_FORMAT_DOUBLE || right.spec.format == VIPS_FORMAT_DOUBLE {
            VIPS_FORMAT_DOUBLE
        } else {
            VIPS_FORMAT_FLOAT
        };
    for y in 0..left.spec.height {
        for x in 0..left.spec.width {
            for band in 0..left.spec.bands {
                let denom = right.get(x, y, band);
                let value = if denom == 0.0 {
                    0.0
                } else {
                    left.get(x, y, band) / denom
                };
                out.set(x, y, band, value);
            }
        }
    }
    Ok(out)
}

fn threshold_mask_buffer(input: &ImageBuffer, threshold: f64) -> ImageBuffer {
    let mut out = ImageBuffer::new(
        input.spec.width,
        input.spec.height,
        input.spec.bands,
        VIPS_FORMAT_UCHAR,
        input.spec.coding,
        input.spec.interpretation,
    );
    out.spec.xres = input.spec.xres;
    out.spec.yres = input.spec.yres;
    out.spec.xoffset = input.spec.xoffset;
    out.spec.yoffset = input.spec.yoffset;
    out.spec.dhint = input.spec.dhint;
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            for band in 0..input.spec.bands {
                out.set(
                    x,
                    y,
                    band,
                    if input.get(x, y, band) > threshold {
                        1.0
                    } else {
                        0.0
                    },
                );
            }
        }
    }
    out
}

fn apply_mask_zero(input: &ImageBuffer, mask: &ImageBuffer) -> Result<ImageBuffer, ()> {
    let mask = if mask.spec.bands == input.spec.bands {
        mask.clone()
    } else if mask.spec.bands == 1 {
        mask.replicate_bands(input.spec.bands)?
    } else {
        return Err(());
    };

    let mut out = input.clone();
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            for band in 0..input.spec.bands {
                let value = if mask.get(x, y, band) != 0.0 {
                    input.get(x, y, band)
                } else {
                    0.0
                };
                out.set(x, y, band, value);
            }
        }
    }
    Ok(out)
}

fn sum_single_band_buffers(buffers: &[&ImageBuffer]) -> Result<ImageBuffer, ()> {
    let first = *buffers.first().ok_or(())?;
    let mut out = first.clone();
    for y in 0..out.spec.height {
        for x in 0..out.spec.width {
            let value = buffers
                .iter()
                .map(|buffer| buffer.get(x, y, 0))
                .sum::<f64>();
            out.set(x, y, 0, value);
        }
    }
    Ok(out)
}

fn max_position(buffer: &ImageBuffer) -> Result<(usize, usize), ()> {
    let mut best_index = None;
    let mut best_value = f64::NEG_INFINITY;
    for (index, value) in buffer.data.iter().copied().enumerate() {
        if value > best_value {
            best_value = value;
            best_index = Some(index);
        }
    }
    let best_index = best_index.ok_or(())?;
    let pixel = best_index / buffer.spec.bands.max(1);
    Ok((
        pixel % buffer.spec.width.max(1),
        pixel / buffer.spec.width.max(1),
    ))
}

fn smartcrop_attention_position(
    input: &ImageBuffer,
    crop_width: usize,
    crop_height: usize,
    _premultiplied: bool,
) -> Result<(usize, usize), ()> {
    let working = if input.spec.bands >= 4 {
        let max_alpha = vips_interpretation_max_alpha(input.spec.interpretation);
        flatten_buffer(input, &[max_alpha], max_alpha)
    } else {
        input.clone()
    };
    let hscale = 32.0 / working.spec.width.max(1) as f64;
    let vscale = 32.0 / working.spec.height.max(1) as f64;
    let sigma = (((crop_width as f64 * hscale).powi(2) + (crop_height as f64 * vscale).powi(2))
        .sqrt()
        / 10.0)
        .max(1.0);

    let small = super::resample::resample_to(&working, 32, 32, VIPS_KERNEL_LANCZOS3, true);
    let xyz = super::colour::prepare_colour_buffer(&small, VIPS_INTERPRETATION_XYZ)?;
    let xyz = extract_bands_buffer(&xyz, 0, 3)?;
    let y_band = extract_bands_buffer(&xyz, 1, 1)?;
    let edge_kernel = Kernel::new(
        3,
        3,
        vec![0.0, -1.0, 0.0, -1.0, 4.0, -1.0, 0.0, -1.0, 0.0],
        1.0,
        0.0,
    );
    let edge = super::convolution::apply_kernel(&y_band, &edge_kernel, VIPS_PRECISION_INTEGER);
    let edge = linear_buffer(&edge, &[5.0], &[0.0])?;
    let mut edge_abs = edge.clone();
    for value in &mut edge_abs.data {
        *value = value.abs();
    }

    let magnitude = pythagoras_buffer(&xyz);
    let norm = divide_buffer(&xyz, &magnitude)?;
    let skin_delta = linear_buffer(&norm, &[1.0, 1.0, 1.0], &[-0.78, -0.57, -0.44])?;
    let skin_distance = pythagoras_buffer(&skin_delta);
    let skin_score = linear_buffer(&skin_distance, &[-100.0], &[100.0])?;
    let mask = threshold_mask_buffer(&y_band, 5.0);
    let skin_score = apply_mask_zero(&skin_score, &mask)?;

    let lab = super::colour::prepare_colour_buffer(&xyz, VIPS_INTERPRETATION_LAB)?;
    let saturation = extract_bands_buffer(&lab, 1, 1)?;
    let saturation = apply_mask_zero(&saturation, &mask)?;

    let merged = sum_single_band_buffers(&[&edge_abs, &skin_score, &saturation])?;
    let gauss = gaussian_kernel(sigma, 0.2, true, VIPS_PRECISION_INTEGER)?;
    let blurred = super::convolution::apply_separable(&merged, &gauss, VIPS_PRECISION_INTEGER)?;
    let (x_pos, y_pos) = max_position(&blurred)?;
    Ok((
        (x_pos as f64 / hscale) as usize,
        (y_pos as f64 / vscale) as usize,
    ))
}

unsafe fn op_smartcrop(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "input")? };
    let mut crop_width = usize::try_from(unsafe { get_int(object, "width")? }).map_err(|_| ())?;
    let mut crop_height = usize::try_from(unsafe { get_int(object, "height")? }).map_err(|_| ())?;
    if crop_width == 0
        || crop_height == 0
        || crop_width > input.spec.width
        || crop_height > input.spec.height
    {
        return Err(());
    }
    let interesting = if unsafe { argument_assigned(object, "interesting")? } {
        unsafe { get_enum(object, "interesting")? as VipsInteresting }
    } else {
        VIPS_INTERESTING_ATTENTION
    };

    if interesting == VIPS_INTERESTING_ALL {
        crop_width = input.spec.width;
        crop_height = input.spec.height;
    }

    let image = unsafe { get_image_ref(object, "input")? };
    let result = (|| {
        let (attention_x, attention_y) = match interesting {
            VIPS_INTERESTING_NONE | VIPS_INTERESTING_LOW => (crop_width / 2, crop_height / 2),
            VIPS_INTERESTING_CENTRE => (input.spec.width / 2, input.spec.height / 2),
            VIPS_INTERESTING_HIGH => (
                input.spec.width - crop_width / 2,
                input.spec.height - crop_height / 2,
            ),
            VIPS_INTERESTING_ATTENTION => {
                let premultiplied = if unsafe { argument_assigned(object, "premultiplied")? } {
                    unsafe { get_bool(object, "premultiplied")? }
                } else {
                    false
                };
                smartcrop_attention_position(&input, crop_width, crop_height, premultiplied)?
            }
            _ => (input.spec.width / 2, input.spec.height / 2),
        };
        let left = attention_x
            .saturating_sub(crop_width / 2)
            .min(input.spec.width - crop_width);
        let top = attention_y
            .saturating_sub(crop_height / 2)
            .min(input.spec.height - crop_height);
        let out = extract_area_buffer(&input, left, top, crop_width, crop_height)?;

        unsafe { set_output_image_like(object, "out", out, image) }?;
        unsafe { set_output_int(object, "attention_x", attention_x as i32) }?;
        unsafe { set_output_int(object, "attention_y", attention_y as i32) }?;
        Ok(())
    })();
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

unsafe fn op_copy(object: *mut VipsObject) -> Result<(), ()> {
    let image = unsafe { get_image_ref(object, "in")? };
    let result = (|| {
        ensure_pixels(image)?;
        let input_ref = unsafe { image.as_ref() }.ok_or(())?;
        let input_len = unsafe { image_state(image) }.ok_or(())?.pixels.len();
        let input_pel_size =
            format_bytes(input_ref.BandFmt).saturating_mul(input_ref.Bands.max(0) as usize);
        let out_image = crate::runtime::image::vips_image_copy_memory(image);
        if out_image.is_null() {
            return Err(());
        }

        {
            let out_ref = unsafe { out_image.as_mut() }.ok_or(())?;
            if unsafe { argument_assigned(object, "interpretation")? } {
                out_ref.Type = unsafe { get_enum(object, "interpretation")? } as VipsInterpretation;
            }
            if unsafe { argument_assigned(object, "xres")? } {
                out_ref.Xres = unsafe { super::get_double(object, "xres")? };
                out_ref.Xres_float = out_ref.Xres as f32;
            }
            if unsafe { argument_assigned(object, "yres")? } {
                out_ref.Yres = unsafe { super::get_double(object, "yres")? };
                out_ref.Yres_float = out_ref.Yres as f32;
            }
            if unsafe { argument_assigned(object, "xoffset")? } {
                out_ref.Xoffset = unsafe { get_int(object, "xoffset")? };
            }
            if unsafe { argument_assigned(object, "yoffset")? } {
                out_ref.Yoffset = unsafe { get_int(object, "yoffset")? };
            }
            if unsafe { argument_assigned(object, "bands")? } {
                out_ref.Bands = unsafe { get_int(object, "bands")? };
            }
            if unsafe { argument_assigned(object, "format")? } {
                out_ref.BandFmt = unsafe { get_enum(object, "format")? } as VipsBandFormat;
            }
            if unsafe { argument_assigned(object, "coding")? } {
                out_ref.Coding = unsafe { get_enum(object, "coding")? } as VipsCoding;
            }
            if unsafe { argument_assigned(object, "width")? } {
                out_ref.Xsize = unsafe { get_int(object, "width")? };
            }
            if unsafe { argument_assigned(object, "height")? } {
                out_ref.Ysize = unsafe { get_int(object, "height")? };
            }
        }

        let out_ref = unsafe { out_image.as_ref() }.ok_or(())?;
        let out_pel_size =
            format_bytes(out_ref.BandFmt).saturating_mul(out_ref.Bands.max(0) as usize);
        if out_pel_size != input_pel_size || image_size(out_ref) != input_len {
            unsafe {
                crate::runtime::object::object_unref(out_image);
            }
            return Err(());
        }

        if unsafe { argument_assigned(object, "swap")? } && unsafe { get_bool(object, "swap")? } {
            let sample_size = format_bytes(out_ref.BandFmt);
            if let Some(state) = unsafe { image_state(out_image) } {
                if sample_size > 1 {
                    for chunk in state.pixels.chunks_exact_mut(sample_size) {
                        chunk.reverse();
                    }
                }
            }
            sync_pixels(out_image);
        }

        unsafe { set_output_image(object, "out", out_image) }
    })();
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

unsafe fn op_byteswap(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_ref(object, "in")? };
    ensure_pixels(input)?;
    let source = unsafe { image_state(input) }.ok_or(())?;
    let sample_size = crate::pixels::format::format_bytes(unsafe { (*input).BandFmt });
    let out = ImageBuffer::from_image(input)?;
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
        "blockcache" | "cache" | "linecache" | "tilecache" => {
            unsafe { op_cache(object)? };
            Ok(true)
        }
        "copy" => {
            unsafe { op_copy(object)? };
            Ok(true)
        }
        "crop" | "extract_area" => {
            unsafe {
                op_extract_area(
                    object,
                    if nickname == "crop" { "in" } else { "input" },
                    "out",
                )?
            };
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
        "bandrank" => {
            unsafe { op_bandrank(object)? };
            Ok(true)
        }
        "ifthenelse" => {
            unsafe { op_ifthenelse(object)? };
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
        "gravity" => {
            unsafe { op_gravity(object)? };
            Ok(true)
        }
        "replicate" => {
            unsafe { op_replicate(object)? };
            Ok(true)
        }
        "cast" => {
            unsafe { op_cast(object)? };
            Ok(true)
        }
        "insert" => {
            unsafe { op_insert(object)? };
            Ok(true)
        }
        "flip" => {
            unsafe { op_flip(object)? };
            Ok(true)
        }
        "gamma" => {
            unsafe { op_gamma(object)? };
            Ok(true)
        }
        "grid" => {
            unsafe { op_grid(object)? };
            Ok(true)
        }
        "switch" => {
            unsafe { op_switch(object)? };
            Ok(true)
        }
        "arrayjoin" => {
            unsafe { op_arrayjoin(object)? };
            Ok(true)
        }
        "composite2" => {
            unsafe { op_composite2(object)? };
            Ok(true)
        }
        "join" => {
            unsafe { op_join(object)? };
            Ok(true)
        }
        "msb" => {
            unsafe { op_msb(object)? };
            Ok(true)
        }
        "recomb" => {
            unsafe { op_recomb(object)? };
            Ok(true)
        }
        "rot" => {
            unsafe { op_rot(object)? };
            Ok(true)
        }
        "rot45" => {
            unsafe { op_rot45(object)? };
            Ok(true)
        }
        "scale" => {
            unsafe { op_scale(object)? };
            Ok(true)
        }
        "subsample" => {
            unsafe { op_subsample(object)? };
            Ok(true)
        }
        "zoom" => {
            unsafe { op_zoom(object)? };
            Ok(true)
        }
        "wrap" => {
            unsafe { op_wrap(object)? };
            Ok(true)
        }
        "premultiply" => {
            unsafe { op_premultiply(object)? };
            Ok(true)
        }
        "flatten" => {
            unsafe { op_flatten(object)? };
            Ok(true)
        }
        "unpremultiply" => {
            unsafe { op_unpremultiply(object)? };
            Ok(true)
        }
        "falsecolour" => {
            unsafe { op_falsecolour(object)? };
            Ok(true)
        }
        "smartcrop" => {
            unsafe { op_smartcrop(object)? };
            Ok(true)
        }
        "byteswap" => {
            unsafe { op_byteswap(object)? };
            Ok(true)
        }
        _ => Ok(false),
    }
}
fn replicate_to_bands(input: &ImageBuffer, bands: usize) -> Result<ImageBuffer, ()> {
    if input.spec.bands == bands {
        return Ok(input.clone());
    }
    if input.spec.bands != 1 || bands == 0 {
        return Err(());
    }

    let mut out = input.with_shape(input.spec.width, input.spec.height, bands);
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            let value = input.get(x, y, 0);
            for band in 0..bands {
                out.set(x, y, band, value);
            }
        }
    }
    Ok(out)
}
