use std::ffi::c_void;
use std::ptr;

use crate::abi::basic::{
    VipsExtend, VipsKernel, VipsSize, VIPS_EXTEND_BACKGROUND, VIPS_EXTEND_BLACK,
    VIPS_EXTEND_COPY, VIPS_EXTEND_MIRROR, VIPS_EXTEND_REPEAT, VIPS_EXTEND_WHITE,
    VIPS_KERNEL_CUBIC, VIPS_KERNEL_LANCZOS2, VIPS_KERNEL_LANCZOS3, VIPS_KERNEL_LINEAR,
    VIPS_KERNEL_MITCHELL, VIPS_KERNEL_NEAREST, VIPS_SIZE_BOTH, VIPS_SIZE_DOWN, VIPS_SIZE_FORCE,
    VIPS_SIZE_UP,
};
use crate::abi::connection::VipsSource;
use crate::abi::image::{VipsBandFormat, VipsImage};
use crate::abi::object::VipsObject;
use crate::abi::operation::VipsInterpolate;
use crate::pixels::ImageBuffer;
use crate::pixels::format::{format_kind, format_max, NumericKind};
use crate::runtime::image::safe_vips_image_new_from_source_internal;
use crate::runtime::object::object_unref;
use crate::runtime::source::{vips_source_new_from_file, vips_source_new_from_memory};
use crate::simd::reduce::{dot, normalize_weights};

use super::{
    argument_assigned, get_array_double, get_blob_bytes, get_bool, get_double, get_enum,
    get_image_buffer, get_image_ref, get_int, get_object_ref, get_string, set_output_image_like,
};

#[derive(Clone)]
struct AxisWeights {
    start: usize,
    weights: Vec<f64>,
}

fn support(kernel: VipsKernel) -> f64 {
    match kernel {
        VIPS_KERNEL_NEAREST => 0.5,
        VIPS_KERNEL_LINEAR => 1.0,
        VIPS_KERNEL_CUBIC | VIPS_KERNEL_MITCHELL => 2.0,
        VIPS_KERNEL_LANCZOS2 => 2.0,
        VIPS_KERNEL_LANCZOS3 => 3.0,
        _ => 3.0,
    }
}

fn sinc(x: f64) -> f64 {
    if x.abs() < f64::EPSILON {
        1.0
    } else {
        let pi_x = std::f64::consts::PI * x;
        pi_x.sin() / pi_x
    }
}

fn cubic_catmull_rom(x: f64) -> f64 {
    let x = x.abs();
    if x < 1.0 {
        1.5 * x.powi(3) - 2.5 * x.powi(2) + 1.0
    } else if x < 2.0 {
        -0.5 * x.powi(3) + 2.5 * x.powi(2) - 4.0 * x + 2.0
    } else {
        0.0
    }
}

fn cubic_mitchell(x: f64) -> f64 {
    let x = x.abs();
    let b = 1.0 / 3.0;
    let c = 1.0 / 3.0;
    if x < 1.0 {
        ((12.0 - 9.0 * b - 6.0 * c) * x.powi(3)
            + (-18.0 + 12.0 * b + 6.0 * c) * x.powi(2)
            + (6.0 - 2.0 * b))
            / 6.0
    } else if x < 2.0 {
        ((-b - 6.0 * c) * x.powi(3)
            + (6.0 * b + 30.0 * c) * x.powi(2)
            + (-12.0 * b - 48.0 * c) * x
            + (8.0 * b + 24.0 * c))
            / 6.0
    } else {
        0.0
    }
}

fn kernel_value(kernel: VipsKernel, x: f64) -> f64 {
    match kernel {
        VIPS_KERNEL_NEAREST => {
            if x.abs() < 0.5 {
                1.0
            } else {
                0.0
            }
        }
        VIPS_KERNEL_LINEAR => (1.0 - x.abs()).max(0.0),
        VIPS_KERNEL_CUBIC => cubic_catmull_rom(x),
        VIPS_KERNEL_MITCHELL => cubic_mitchell(x),
        VIPS_KERNEL_LANCZOS2 => {
            if x.abs() < 2.0 {
                sinc(x) * sinc(x / 2.0)
            } else {
                0.0
            }
        }
        VIPS_KERNEL_LANCZOS3 => {
            if x.abs() < 3.0 {
                sinc(x) * sinc(x / 3.0)
            } else {
                0.0
            }
        }
        _ => {
            if x.abs() < 3.0 {
                sinc(x) * sinc(x / 3.0)
            } else {
                0.0
            }
        }
    }
}

fn build_axis_weights(
    input_len: usize,
    output_len: usize,
    kernel: VipsKernel,
    centre: bool,
) -> Vec<AxisWeights> {
    let input_len = input_len.max(1);
    let output_len = output_len.max(1);
    let scale = output_len as f64 / input_len as f64;
    let mut out = Vec::with_capacity(output_len);

    for out_index in 0..output_len {
        let center = if centre {
            ((out_index as f64 + 0.5) / scale) - 0.5
        } else {
            out_index as f64 / scale
        };

        if kernel == VIPS_KERNEL_NEAREST {
            let source = center.round().clamp(0.0, (input_len - 1) as f64) as usize;
            out.push(AxisWeights {
                start: source,
                weights: vec![1.0],
            });
            continue;
        }

        let scale_weight = scale.min(1.0);
        let radius = support(kernel) / scale_weight.max(f64::EPSILON);
        let start = (center - radius).floor() as isize;
        let end = (center + radius).ceil() as isize;

        let mut actual_start = None;
        let mut weights = Vec::new();
        for src in start..=end {
            if src < 0 || src >= input_len as isize {
                continue;
            }
            if actual_start.is_none() {
                actual_start = Some(src as usize);
            }
            let distance = src as f64 - center;
            let weight = if scale < 1.0 {
                kernel_value(kernel, distance * scale) * scale
            } else {
                kernel_value(kernel, distance)
            };
            weights.push(weight);
        }

        let start = actual_start
            .unwrap_or_else(|| center.round().clamp(0.0, (input_len - 1) as f64) as usize);
        if weights.is_empty() {
            weights.push(1.0);
        }
        normalize_weights(&mut weights);
        out.push(AxisWeights { start, weights });
    }

    out
}

fn resample_horizontal(
    input: &ImageBuffer,
    output_width: usize,
    kernel: VipsKernel,
    centre: bool,
) -> ImageBuffer {
    let weights = build_axis_weights(input.spec.width, output_width, kernel, centre);
    let mut out = input.with_shape(output_width, input.spec.height, input.spec.bands);
    for y in 0..input.spec.height {
        for band in 0..input.spec.bands {
            for (x, weight) in weights.iter().enumerate() {
                let values = (0..weight.weights.len())
                    .map(|offset| input.get(weight.start + offset, y, band))
                    .collect::<Vec<_>>();
                out.set(x, y, band, dot(&values, &weight.weights));
            }
        }
    }
    out
}

fn resample_vertical(
    input: &ImageBuffer,
    output_height: usize,
    kernel: VipsKernel,
    centre: bool,
) -> ImageBuffer {
    let weights = build_axis_weights(input.spec.height, output_height, kernel, centre);
    let mut out = input.with_shape(input.spec.width, output_height, input.spec.bands);
    for x in 0..input.spec.width {
        for band in 0..input.spec.bands {
            for (y, weight) in weights.iter().enumerate() {
                let values = (0..weight.weights.len())
                    .map(|offset| input.get(x, weight.start + offset, band))
                    .collect::<Vec<_>>();
                out.set(x, y, band, dot(&values, &weight.weights));
            }
        }
    }
    out
}

fn uniform_pixel(input: &ImageBuffer) -> Option<Vec<f64>> {
    if input.spec.width == 0 || input.spec.height == 0 || input.spec.bands == 0 {
        return Some(Vec::new());
    }

    let reference = (0..input.spec.bands)
        .map(|band| input.get(0, 0, band))
        .collect::<Vec<_>>();
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            for (band, sample) in reference.iter().enumerate() {
                if input.get(x, y, band).to_bits() != sample.to_bits() {
                    return None;
                }
            }
        }
    }

    Some(reference)
}

pub(crate) fn resample_to(
    input: &ImageBuffer,
    output_width: usize,
    output_height: usize,
    kernel: VipsKernel,
    centre: bool,
) -> ImageBuffer {
    if let Some(pixel) = uniform_pixel(input) {
        let mut out = input.with_shape(output_width.max(1), output_height.max(1), input.spec.bands);
        for y in 0..out.spec.height {
            for x in 0..out.spec.width {
                for (band, sample) in pixel.iter().enumerate() {
                    out.set(x, y, band, *sample);
                }
            }
        }
        return out;
    }

    let tmp = resample_horizontal(input, output_width.max(1), kernel, centre);
    resample_vertical(&tmp, output_height.max(1), kernel, centre)
}

fn shrink_output_len(input_len: usize, shrink: f64, ceil_mode: bool) -> usize {
    let shrink = shrink.max(1.0);
    let value = input_len as f64 / shrink;
    let out = if ceil_mode {
        value.ceil()
    } else {
        value.round().max(1.0)
    };
    out.max(1.0) as usize
}

fn box_average(values: &[(f64, f64)]) -> f64 {
    let mut sum = 0.0;
    let mut weight = 0.0;
    for (value, overlap) in values {
        sum += value * overlap;
        weight += overlap;
    }
    if weight <= 0.0 {
        0.0
    } else {
        sum / weight
    }
}

fn shrinkh_box(input: &ImageBuffer, shrink: f64, ceil_mode: bool) -> ImageBuffer {
    let output_width = shrink_output_len(input.spec.width, shrink, ceil_mode);
    let mut out = input.with_shape(output_width, input.spec.height, input.spec.bands);
    for y in 0..input.spec.height {
        for x in 0..output_width {
            let start = x as f64 * shrink;
            let end = ((x + 1) as f64 * shrink).min(input.spec.width as f64);
            let first = start.floor() as usize;
            let last = end.ceil().max(start.ceil()) as usize;
            for band in 0..input.spec.bands {
                let mut values = Vec::new();
                for src in first..last.min(input.spec.width) {
                    let overlap = (end.min((src + 1) as f64) - start.max(src as f64)).max(0.0);
                    if overlap > 0.0 {
                        values.push((input.get(src, y, band), overlap));
                    }
                }
                out.set(x, y, band, box_average(&values));
            }
        }
    }
    out
}

fn shrinkv_box(input: &ImageBuffer, shrink: f64, ceil_mode: bool) -> ImageBuffer {
    let output_height = shrink_output_len(input.spec.height, shrink, ceil_mode);
    let mut out = input.with_shape(input.spec.width, output_height, input.spec.bands);
    for y in 0..output_height {
        let start = y as f64 * shrink;
        let end = ((y + 1) as f64 * shrink).min(input.spec.height as f64);
        let first = start.floor() as usize;
        let last = end.ceil().max(start.ceil()) as usize;
        for x in 0..input.spec.width {
            for band in 0..input.spec.bands {
                let mut values = Vec::new();
                for src in first..last.min(input.spec.height) {
                    let overlap = (end.min((src + 1) as f64) - start.max(src as f64)).max(0.0);
                    if overlap > 0.0 {
                        values.push((input.get(x, src, band), overlap));
                    }
                }
                out.set(x, y, band, box_average(&values));
            }
        }
    }
    out
}

fn centre_crop(input: &ImageBuffer, width: usize, height: usize) -> ImageBuffer {
    let width = width.min(input.spec.width).max(1);
    let height = height.min(input.spec.height).max(1);
    let left = input.spec.width.saturating_sub(width) / 2;
    let top = input.spec.height.saturating_sub(height) / 2;
    let mut out = input.with_shape(width, height, input.spec.bands);
    for y in 0..height {
        for x in 0..width {
            for band in 0..input.spec.bands {
                out.set(x, y, band, input.get(left + x, top + y, band));
            }
        }
    }
    out
}

fn apply_size_constraint(scale: f64, size: VipsSize) -> f64 {
    match size {
        VIPS_SIZE_UP => scale.max(1.0),
        VIPS_SIZE_DOWN => scale.min(1.0),
        _ => scale,
    }
}

fn thumbnail_plan(
    input_width: usize,
    input_height: usize,
    width: usize,
    height: Option<usize>,
    size: VipsSize,
    crop: bool,
) -> (usize, usize, Option<(usize, usize)>) {
    if height.is_none() {
        let tall_roll = input_height > input_width.saturating_mul(3);
        let dominant = if tall_roll {
            input_width.max(1)
        } else {
            input_width.max(input_height).max(1)
        };
        let mut scale = width.max(1) as f64 / dominant as f64;
        scale = apply_size_constraint(scale, size);
        let output_width = if tall_roll {
            (input_width as f64 * scale).floor().max(1.0) as usize
        } else {
            (input_width as f64 * scale).round().max(1.0) as usize
        };
        let output_height = if tall_roll {
            (input_height as f64 * scale).floor().max(1.0) as usize
        } else {
            (input_height as f64 * scale).round().max(1.0) as usize
        };
        return (output_width, output_height, None);
    }

    let sx = width.max(1) as f64 / input_width.max(1) as f64;
    let sy = height.unwrap_or_else(|| ((input_height as f64 * sx).round().max(1.0)) as usize)
        as f64
        / input_height.max(1) as f64;

    if size == VIPS_SIZE_FORCE {
        return (
            width.max(1),
            height
                .unwrap_or_else(|| ((input_height as f64 * sx).round().max(1.0)) as usize)
                .max(1),
            None,
        );
    }

    let mut scale = if crop && height.is_some() {
        sx.max(sy)
    } else {
        sx.min(sy)
    };
    scale = apply_size_constraint(scale, size);
    let output_width = ((input_width as f64 * scale).round().max(1.0)) as usize;
    let output_height = ((input_height as f64 * scale).round().max(1.0)) as usize;
    let crop_to = if crop {
        height.map(|target_height| (width.max(1), target_height.max(1)))
    } else {
        None
    };
    (output_width, output_height, crop_to)
}

fn preferred_kernel(object: *mut VipsObject, default_kernel: VipsKernel) -> Result<VipsKernel, ()> {
    if unsafe { argument_assigned(object, "kernel")? } {
        Ok(unsafe { get_enum(object, "kernel")? as VipsKernel })
    } else {
        Ok(default_kernel)
    }
}

fn get_double_with_fallback(
    object: *mut VipsObject,
    primary: &str,
    fallback: &str,
) -> Result<f64, ()> {
    unsafe { get_double(object, primary) }.or_else(|_| unsafe { get_double(object, fallback) })
}

fn get_int_with_fallback(
    object: *mut VipsObject,
    primary: &str,
    fallback: &str,
) -> Result<i32, ()> {
    unsafe { get_int(object, primary) }.or_else(|_| unsafe { get_int(object, fallback) })
}

unsafe fn set_out_image(
    object: *mut VipsObject,
    buffer: ImageBuffer,
    like: *mut VipsImage,
) -> Result<(), ()> {
    unsafe { set_output_image_like(object, "out", buffer, like) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AffineInterpolator {
    Nearest,
    Bilinear,
}

fn affine_interpolator(object: *mut VipsObject) -> Result<AffineInterpolator, ()> {
    if !unsafe { argument_assigned(object, "interpolate")? } {
        return Ok(AffineInterpolator::Bilinear);
    }

    let interpolate = unsafe { get_object_ref::<VipsInterpolate>(object, "interpolate")? };
    let nickname = unsafe {
        let class = crate::runtime::object::object_class(interpolate.cast());
        if class.is_null() || (*class).nickname.is_null() {
            None
        } else {
            Some(
                std::ffi::CStr::from_ptr((*class).nickname)
                    .to_string_lossy()
                    .to_ascii_lowercase(),
            )
        }
    };
    unsafe {
        object_unref(interpolate);
    }

    Ok(match nickname.as_deref() {
        Some("nearest") => AffineInterpolator::Nearest,
        _ => AffineInterpolator::Bilinear,
    })
}

fn affine_background(object: *mut VipsObject, bands: usize) -> Result<Vec<f64>, ()> {
    let values = if unsafe { argument_assigned(object, "background")? } {
        let values = unsafe { get_array_double(object, "background")? };
        if values.is_empty() {
            vec![0.0]
        } else {
            values
        }
    } else {
        vec![0.0]
    };

    let mut out = Vec::with_capacity(bands.max(1));
    for band in 0..bands.max(1) {
        out.push(
            values
                .get(band)
                .copied()
                .unwrap_or_else(|| *values.last().unwrap_or(&0.0)),
        );
    }
    Ok(out)
}

fn affine_output_area(
    width: usize,
    height: usize,
    matrix: [f64; 4],
    idx: f64,
    idy: f64,
    odx: f64,
    ody: f64,
) -> (i32, i32, usize, usize) {
    let corners = [
        (0.0, 0.0),
        (0.0, height as f64),
        (width as f64, 0.0),
        (width as f64, height as f64),
    ];

    let mut left = f64::INFINITY;
    let mut top = f64::INFINITY;
    let mut right = f64::NEG_INFINITY;
    let mut bottom = f64::NEG_INFINITY;
    for (x, y) in corners {
        let x = x + idx;
        let y = y + idy;
        let ox = matrix[0] * x + matrix[1] * y + odx;
        let oy = matrix[2] * x + matrix[3] * y + ody;
        left = left.min(ox);
        top = top.min(oy);
        right = right.max(ox);
        bottom = bottom.max(oy);
    }

    (
        left.round() as i32,
        top.round() as i32,
        ((right - left).round().max(1.0)) as usize,
        ((bottom - top).round().max(1.0)) as usize,
    )
}

fn affine_sample_nearest(input: &ImageBuffer, x: f64, y: f64, band: usize, background: f64) -> f64 {
    let max_x = input.spec.width.saturating_sub(1) as f64;
    let max_y = input.spec.height.saturating_sub(1) as f64;
    if x < 0.0 || y < 0.0 || x > max_x || y > max_y {
        return background;
    }
    input.get(x.round() as usize, y.round() as usize, band)
}

fn affine_sample_bilinear(
    input: &ImageBuffer,
    x: f64,
    y: f64,
    band: usize,
    background: f64,
) -> f64 {
    let max_x = input.spec.width.saturating_sub(1) as f64;
    let max_y = input.spec.height.saturating_sub(1) as f64;
    if x < 0.0 || y < 0.0 || x > max_x || y > max_y {
        return background;
    }

    let x0 = x.floor();
    let y0 = y.floor();
    let x1 = (x0 + 1.0).min(max_x);
    let y1 = (y0 + 1.0).min(max_y);
    let fx = x - x0;
    let fy = y - y0;
    let sample = |sx: f64, sy: f64| input.get(sx as usize, sy as usize, band);
    let top = sample(x0, y0) * (1.0 - fx) + sample(x1, y0) * fx;
    let bottom = sample(x0, y1) * (1.0 - fx) + sample(x1, y1) * fx;
    top * (1.0 - fy) + bottom * fy
}

fn affine_sample(
    input: &ImageBuffer,
    x: f64,
    y: f64,
    band: usize,
    interpolator: AffineInterpolator,
    background: f64,
) -> f64 {
    match interpolator {
        AffineInterpolator::Nearest => affine_sample_nearest(input, x, y, band, background),
        AffineInterpolator::Bilinear => affine_sample_bilinear(input, x, y, band, background),
    }
}

fn mapim_background_values(values: &[f64], bands: usize) -> Vec<f64> {
    if values.is_empty() {
        vec![0.0; bands]
    } else {
        (0..bands)
            .map(|band| values.get(band).copied().unwrap_or(values[0]))
            .collect()
    }
}

fn mapim_white_background_value(format: VipsBandFormat) -> f64 {
    match format_kind(format) {
        Some(NumericKind::Unsigned) => format_max(format).unwrap_or(255.0),
        Some(NumericKind::Signed) => -1.0,
        _ => 255.0,
    }
}

fn mapim_sample_extended(
    input: &ImageBuffer,
    x: isize,
    y: isize,
    band: usize,
    extend: VipsExtend,
    background: &[f64],
) -> f64 {
    let bg = background.get(band).copied().unwrap_or(0.0);
    if input.spec.width == 0 || input.spec.height == 0 {
        return bg;
    }

    let inside = x >= 0
        && y >= 0
        && (x as usize) < input.spec.width
        && (y as usize) < input.spec.height;
    if inside {
        return input.get(x as usize, y as usize, band);
    }

    match extend {
        VIPS_EXTEND_BLACK => 0.0,
        VIPS_EXTEND_WHITE => mapim_white_background_value(input.spec.format),
        VIPS_EXTEND_BACKGROUND => bg,
        VIPS_EXTEND_COPY => input.get(
            x.clamp(0, input.spec.width.saturating_sub(1) as isize) as usize,
            y.clamp(0, input.spec.height.saturating_sub(1) as isize) as usize,
            band,
        ),
        VIPS_EXTEND_REPEAT => input.get(
            x.rem_euclid(input.spec.width as isize) as usize,
            y.rem_euclid(input.spec.height as isize) as usize,
            band,
        ),
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
                mirror(x, input.spec.width),
                mirror(y, input.spec.height),
                band,
            )
        }
        _ => bg,
    }
}

fn mapim_sample_nearest(
    input: &ImageBuffer,
    x: f64,
    y: f64,
    band: usize,
    extend: VipsExtend,
    background: &[f64],
) -> f64 {
    if !x.is_finite() || !y.is_finite() {
        return background.get(band).copied().unwrap_or(0.0);
    }

    mapim_sample_extended(input, x.round() as isize, y.round() as isize, band, extend, background)
}

fn mapim_sample_bilinear(
    input: &ImageBuffer,
    x: f64,
    y: f64,
    band: usize,
    extend: VipsExtend,
    background: &[f64],
) -> f64 {
    if !x.is_finite() || !y.is_finite() {
        return background.get(band).copied().unwrap_or(0.0);
    }

    let x0 = x.floor() as isize;
    let y0 = y.floor() as isize;
    let x1 = x0 + 1;
    let y1 = y0 + 1;
    let fx = x - x0 as f64;
    let fy = y - y0 as f64;
    let top = mapim_sample_extended(input, x0, y0, band, extend, background) * (1.0 - fx)
        + mapim_sample_extended(input, x1, y0, band, extend, background) * fx;
    let bottom = mapim_sample_extended(input, x0, y1, band, extend, background) * (1.0 - fx)
        + mapim_sample_extended(input, x1, y1, band, extend, background) * fx;
    top * (1.0 - fy) + bottom * fy
}

fn mapim_sample(
    input: &ImageBuffer,
    x: f64,
    y: f64,
    band: usize,
    interpolator: AffineInterpolator,
    extend: VipsExtend,
    background: &[f64],
) -> f64 {
    match interpolator {
        AffineInterpolator::Nearest => {
            mapim_sample_nearest(input, x, y, band, extend, background)
        }
        AffineInterpolator::Bilinear => {
            mapim_sample_bilinear(input, x, y, band, extend, background)
        }
    }
}

fn apply_affine(
    input: &ImageBuffer,
    matrix: [f64; 4],
    interpolator: AffineInterpolator,
    background: &[f64],
    idx: f64,
    idy: f64,
    odx: f64,
    ody: f64,
) -> Result<ImageBuffer, ()> {
    let det = matrix[0] * matrix[3] - matrix[1] * matrix[2];
    if det.abs() <= f64::EPSILON {
        return Err(());
    }

    let ia = matrix[3] / det;
    let ib = -matrix[1] / det;
    let ic = -matrix[2] / det;
    let id = matrix[0] / det;
    let (left, top, out_width, out_height) = affine_output_area(
        input.spec.width,
        input.spec.height,
        matrix,
        idx,
        idy,
        odx,
        ody,
    );

    let mut out = input.with_shape(out_width, out_height, input.spec.bands);
    out.spec.xoffset = (odx - left as f64).round() as i32;
    out.spec.yoffset = (ody - top as f64).round() as i32;

    for y in 0..out_height {
        let oy = y as f64 + top as f64 - ody;
        for x in 0..out_width {
            let ox = x as f64 + left as f64 - odx;
            let ix = ia * ox + ib * oy - idx;
            let iy = ic * ox + id * oy - idy;
            for band in 0..input.spec.bands {
                let bg = background.get(band).copied().unwrap_or(0.0);
                out.set(
                    x,
                    y,
                    band,
                    affine_sample(input, ix, iy, band, interpolator, bg),
                );
            }
        }
    }

    Ok(out)
}

unsafe fn op_affine_like(object: *mut VipsObject, matrix: [f64; 4]) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let like = unsafe { get_image_ref(object, "in")? };
    let interpolator = affine_interpolator(object)?;
    let background = affine_background(object, input.spec.bands)?;
    let idx = if unsafe { argument_assigned(object, "idx")? } {
        unsafe { get_double(object, "idx")? }
    } else {
        0.0
    };
    let idy = if unsafe { argument_assigned(object, "idy")? } {
        unsafe { get_double(object, "idy")? }
    } else {
        0.0
    };
    let odx = if unsafe { argument_assigned(object, "odx")? } {
        unsafe { get_double(object, "odx")? }
    } else {
        0.0
    };
    let ody = if unsafe { argument_assigned(object, "ody")? } {
        unsafe { get_double(object, "ody")? }
    } else {
        0.0
    };

    let out = apply_affine(
        &input,
        matrix,
        interpolator,
        &background,
        idx,
        idy,
        odx,
        ody,
    )?;
    let result = unsafe { set_out_image(object, out, like) };
    unsafe {
        object_unref(like);
    }
    result
}

unsafe fn op_affine(object: *mut VipsObject) -> Result<(), ()> {
    let matrix = unsafe { get_array_double(object, "matrix")? };
    if matrix.len() != 4 {
        return Err(());
    }
    unsafe { op_affine_like(object, [matrix[0], matrix[1], matrix[2], matrix[3]]) }
}

fn snap_affine_component(value: f64) -> f64 {
    if value.abs() < 1e-12 {
        0.0
    } else if (value - 1.0).abs() < 1e-12 {
        1.0
    } else if (value + 1.0).abs() < 1e-12 {
        -1.0
    } else {
        value
    }
}

unsafe fn op_similarity(object: *mut VipsObject) -> Result<(), ()> {
    let scale = if unsafe { argument_assigned(object, "scale")? } {
        unsafe { get_double(object, "scale")? }
    } else {
        1.0
    };
    let angle = if unsafe { argument_assigned(object, "angle")? } {
        unsafe { get_double(object, "angle")? }
    } else {
        0.0
    };
    let radians = angle.to_radians();
    let a = snap_affine_component(scale * radians.cos());
    let b = snap_affine_component(scale * -radians.sin());
    let c = snap_affine_component(-b);
    let d = snap_affine_component(a);
    unsafe { op_affine_like(object, [a, b, c, d]) }
}

unsafe fn op_rotate(object: *mut VipsObject) -> Result<(), ()> {
    let angle = unsafe { get_double(object, "angle")? };
    let radians = angle.to_radians();
    let a = snap_affine_component(radians.cos());
    let b = snap_affine_component(-radians.sin());
    let c = snap_affine_component(-b);
    let d = snap_affine_component(a);
    unsafe { op_affine_like(object, [a, b, c, d]) }
}

unsafe fn op_mapim(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let index = unsafe { get_image_buffer(object, "index")? };
    let like = unsafe { get_image_ref(object, "in")? };
    if index.spec.bands < 2 {
        unsafe {
            object_unref(like);
        }
        return Err(());
    }

    let interpolator = affine_interpolator(object)?;
    let extend = if unsafe { argument_assigned(object, "extend")? } {
        unsafe { get_enum(object, "extend")? as VipsExtend }
    } else {
        VIPS_EXTEND_BACKGROUND
    };
    let background = if unsafe { argument_assigned(object, "background")? } {
        unsafe { get_array_double(object, "background")? }
    } else {
        vec![0.0]
    };
    let background = mapim_background_values(&background, input.spec.bands);
    let mut out = input.with_shape(index.spec.width, index.spec.height, input.spec.bands);
    out.spec.xoffset = 0;
    out.spec.yoffset = 0;
    for y in 0..index.spec.height {
        for x in 0..index.spec.width {
            let src_x = index.get(x, y, 0);
            let src_y = index.get(x, y, 1);
            for band in 0..input.spec.bands {
                out.set(
                    x,
                    y,
                    band,
                    mapim_sample(
                        &input,
                        src_x,
                        src_y,
                        band,
                        interpolator,
                        extend,
                        &background,
                    ),
                );
            }
        }
    }

    let result = unsafe { set_out_image(object, out, like) };
    unsafe {
        object_unref(like);
    }
    result
}

unsafe fn op_reduce(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let like = unsafe { get_image_ref(object, "in")? };
    let hshrink = get_double_with_fallback(object, "hshrink", "xshrink")?.max(1.0);
    let vshrink = get_double_with_fallback(object, "vshrink", "yshrink")?.max(1.0);
    let kernel = preferred_kernel(object, VIPS_KERNEL_LANCZOS3)?;
    let centre = if unsafe { argument_assigned(object, "centre")? } {
        unsafe { get_bool(object, "centre")? }
    } else {
        true
    };
    let out_width = ((input.spec.width as f64 / hshrink).round().max(1.0)) as usize;
    let out_height = ((input.spec.height as f64 / vshrink).round().max(1.0)) as usize;
    let out = resample_to(&input, out_width, out_height, kernel, centre);
    let result = unsafe { set_out_image(object, out, like) };
    unsafe {
        object_unref(like);
    }
    result
}

unsafe fn op_reduceh(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let like = unsafe { get_image_ref(object, "in")? };
    let hshrink = get_double_with_fallback(object, "hshrink", "xshrink")?.max(1.0);
    let kernel = preferred_kernel(object, VIPS_KERNEL_LANCZOS3)?;
    let centre = if unsafe { argument_assigned(object, "centre")? } {
        unsafe { get_bool(object, "centre")? }
    } else {
        true
    };
    let out_width = ((input.spec.width as f64 / hshrink).round().max(1.0)) as usize;
    let out = resample_to(&input, out_width, input.spec.height, kernel, centre);
    let result = unsafe { set_out_image(object, out, like) };
    unsafe {
        object_unref(like);
    }
    result
}

unsafe fn op_reducev(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let like = unsafe { get_image_ref(object, "in")? };
    let vshrink = get_double_with_fallback(object, "vshrink", "yshrink")?.max(1.0);
    let kernel = preferred_kernel(object, VIPS_KERNEL_LANCZOS3)?;
    let centre = if unsafe { argument_assigned(object, "centre")? } {
        unsafe { get_bool(object, "centre")? }
    } else {
        true
    };
    let out_height = ((input.spec.height as f64 / vshrink).round().max(1.0)) as usize;
    let out = resample_to(&input, input.spec.width, out_height, kernel, centre);
    let result = unsafe { set_out_image(object, out, like) };
    unsafe {
        object_unref(like);
    }
    result
}

unsafe fn op_resize(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let like = unsafe { get_image_ref(object, "in")? };
    let scale = unsafe { get_double(object, "scale")? }.max(0.0);
    let vscale = if unsafe { argument_assigned(object, "vscale")? } {
        unsafe { get_double(object, "vscale")? }.max(0.0)
    } else {
        scale
    };
    let kernel = preferred_kernel(object, VIPS_KERNEL_LANCZOS3)?;
    let centre = if unsafe { argument_assigned(object, "centre")? } {
        unsafe { get_bool(object, "centre")? }
    } else {
        true
    };
    let out_width = ((input.spec.width as f64 * scale).round().max(1.0)) as usize;
    let out_height = ((input.spec.height as f64 * vscale).round().max(1.0)) as usize;
    let out = resample_to(&input, out_width, out_height, kernel, centre);
    let result = unsafe { set_out_image(object, out, like) };
    unsafe {
        object_unref(like);
    }
    result
}

unsafe fn op_shrink(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let like = unsafe { get_image_ref(object, "in")? };
    let hshrink = get_double_with_fallback(object, "hshrink", "xshrink")?.max(1.0);
    let vshrink = get_double_with_fallback(object, "vshrink", "yshrink")?.max(1.0);
    let ceil_mode = if unsafe { argument_assigned(object, "ceil")? } {
        unsafe { get_bool(object, "ceil")? }
    } else {
        false
    };
    let tmp = shrinkh_box(&input, hshrink, ceil_mode);
    let out = shrinkv_box(&tmp, vshrink, ceil_mode);
    let result = unsafe { set_out_image(object, out, like) };
    unsafe {
        object_unref(like);
    }
    result
}

unsafe fn op_shrinkh(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let like = unsafe { get_image_ref(object, "in")? };
    let hshrink = get_int_with_fallback(object, "hshrink", "xshrink")?.max(1) as f64;
    let ceil_mode = if unsafe { argument_assigned(object, "ceil")? } {
        unsafe { get_bool(object, "ceil")? }
    } else {
        false
    };
    let out = shrinkh_box(&input, hshrink, ceil_mode);
    let result = unsafe { set_out_image(object, out, like) };
    unsafe {
        object_unref(like);
    }
    result
}

unsafe fn op_shrinkv(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let like = unsafe { get_image_ref(object, "in")? };
    let vshrink = get_int_with_fallback(object, "vshrink", "yshrink")?.max(1) as f64;
    let ceil_mode = if unsafe { argument_assigned(object, "ceil")? } {
        unsafe { get_bool(object, "ceil")? }
    } else {
        false
    };
    let out = shrinkv_box(&input, vshrink, ceil_mode);
    let result = unsafe { set_out_image(object, out, like) };
    unsafe {
        object_unref(like);
    }
    result
}

unsafe fn apply_thumbnail(
    object: *mut VipsObject,
    input: &ImageBuffer,
    like: *mut VipsImage,
) -> Result<(), ()> {
    let width = usize::try_from(unsafe { get_int(object, "width")? }.max(1)).map_err(|_| ())?;
    let height = if unsafe { argument_assigned(object, "height")? } {
        Some(usize::try_from(unsafe { get_int(object, "height")? }.max(1)).map_err(|_| ())?)
    } else {
        None
    };
    let size = if unsafe { argument_assigned(object, "size")? } {
        unsafe { get_enum(object, "size")? as VipsSize }
    } else {
        VIPS_SIZE_BOTH
    };
    let crop =
        unsafe { argument_assigned(object, "crop")? } && unsafe { get_enum(object, "crop")? } != 0;
    let kernel = if unsafe { argument_assigned(object, "linear")? }
        && unsafe { get_bool(object, "linear")? }
    {
        VIPS_KERNEL_LINEAR
    } else {
        VIPS_KERNEL_LANCZOS3
    };

    let (out_width, out_height, crop_to) = thumbnail_plan(
        input.spec.width,
        input.spec.height,
        width,
        height,
        size,
        crop,
    );
    let mut out = resample_to(input, out_width, out_height, kernel, true);
    if let Some((crop_width, crop_height)) = crop_to {
        out = centre_crop(&out, crop_width, crop_height);
    }
    unsafe { set_out_image(object, out, like) }
}

unsafe fn op_thumbnail(object: *mut VipsObject) -> Result<(), ()> {
    let filename = unsafe { get_string(object, "filename")? }.ok_or(())?;
    let (path, options) = crate::foreign::base::parse_embedded_options(&filename);
    let path = std::ffi::CString::new(path).map_err(|_| ())?;
    let options = std::ffi::CString::new(options).map_err(|_| ())?;
    let source = vips_source_new_from_file(path.as_ptr());
    if source.is_null() {
        return Err(());
    }
    let option_ptr = if options.as_bytes().is_empty() {
        ptr::null()
    } else {
        options.as_ptr()
    };
    let image = safe_vips_image_new_from_source_internal(source, option_ptr, 0);
    unsafe {
        object_unref(source);
    }
    if image.is_null() {
        return Err(());
    }
    let input = ImageBuffer::from_image(image)?;
    let result = unsafe { apply_thumbnail(object, &input, image) };
    unsafe {
        object_unref(image);
    }
    result
}

unsafe fn op_thumbnail_buffer(object: *mut VipsObject) -> Result<(), ()> {
    let bytes = unsafe { get_blob_bytes(object, "buffer")? };
    let source = vips_source_new_from_memory(bytes.as_ptr().cast::<c_void>(), bytes.len());
    if source.is_null() {
        return Err(());
    }
    let image = safe_vips_image_new_from_source_internal(source, ptr::null(), 0);
    unsafe {
        object_unref(source);
    }
    if image.is_null() {
        return Err(());
    }
    let input = ImageBuffer::from_image(image)?;
    let result = unsafe { apply_thumbnail(object, &input, image) };
    unsafe {
        object_unref(image);
    }
    result
}

unsafe fn op_thumbnail_image(object: *mut VipsObject) -> Result<(), ()> {
    let image = unsafe { get_image_ref(object, "in")? };
    let input = ImageBuffer::from_image(image)?;
    let result = unsafe { apply_thumbnail(object, &input, image) };
    unsafe {
        object_unref(image);
    }
    result
}

unsafe fn op_thumbnail_source(object: *mut VipsObject) -> Result<(), ()> {
    let source = unsafe { get_object_ref::<VipsSource>(object, "source")? };
    let image = safe_vips_image_new_from_source_internal(source, ptr::null(), 0);
    unsafe {
        object_unref(source);
    }
    if image.is_null() {
        return Err(());
    }
    let input = ImageBuffer::from_image(image)?;
    let result = unsafe { apply_thumbnail(object, &input, image) };
    unsafe {
        object_unref(image);
    }
    result
}

pub(crate) unsafe fn dispatch(object: *mut VipsObject, nickname: &str) -> Result<bool, ()> {
    match nickname {
        "affine" => {
            unsafe { op_affine(object)? };
            Ok(true)
        }
        "reduce" => {
            unsafe { op_reduce(object)? };
            Ok(true)
        }
        "reduceh" => {
            unsafe { op_reduceh(object)? };
            Ok(true)
        }
        "reducev" => {
            unsafe { op_reducev(object)? };
            Ok(true)
        }
        "resize" => {
            unsafe { op_resize(object)? };
            Ok(true)
        }
        "similarity" => {
            unsafe { op_similarity(object)? };
            Ok(true)
        }
        "mapim" => {
            unsafe { op_mapim(object)? };
            Ok(true)
        }
        "shrink" => {
            unsafe { op_shrink(object)? };
            Ok(true)
        }
        "shrinkh" => {
            unsafe { op_shrinkh(object)? };
            Ok(true)
        }
        "shrinkv" => {
            unsafe { op_shrinkv(object)? };
            Ok(true)
        }
        "thumbnail" => {
            unsafe { op_thumbnail(object)? };
            Ok(true)
        }
        "thumbnail_buffer" => {
            unsafe { op_thumbnail_buffer(object)? };
            Ok(true)
        }
        "thumbnail_image" => {
            unsafe { op_thumbnail_image(object)? };
            Ok(true)
        }
        "thumbnail_source" => {
            unsafe { op_thumbnail_source(object)? };
            Ok(true)
        }
        "rotate" => {
            unsafe { op_rotate(object)? };
            Ok(true)
        }
        _ => Ok(false),
    }
}
