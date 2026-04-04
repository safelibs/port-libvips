use std::ffi::c_void;
use std::ptr;

use crate::abi::basic::{
    VipsKernel, VipsSize, VIPS_KERNEL_CUBIC, VIPS_KERNEL_LANCZOS2, VIPS_KERNEL_LANCZOS3,
    VIPS_KERNEL_LINEAR, VIPS_KERNEL_MITCHELL, VIPS_KERNEL_NEAREST, VIPS_SIZE_BOTH,
    VIPS_SIZE_DOWN, VIPS_SIZE_FORCE, VIPS_SIZE_UP,
};
use crate::abi::connection::VipsSource;
use crate::abi::image::VipsImage;
use crate::abi::object::VipsObject;
use crate::pixels::ImageBuffer;
use crate::runtime::image::safe_vips_image_new_from_source_internal;
use crate::runtime::object::object_unref;
use crate::runtime::source::{vips_source_new_from_file, vips_source_new_from_memory};
use crate::simd::reduce::{dot, normalize_weights};

use super::{
    argument_assigned, get_blob_bytes, get_bool, get_double, get_enum, get_image_buffer,
    get_image_ref, get_int, get_object_ref, get_string, set_output_image_like,
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
    _centre: bool,
) -> Vec<AxisWeights> {
    let input_len = input_len.max(1);
    let output_len = output_len.max(1);
    let scale = output_len as f64 / input_len as f64;
    let mut out = Vec::with_capacity(output_len);

    for out_index in 0..output_len {
        let center = ((out_index as f64 + 0.5) / scale) - 0.5;
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

        let start = actual_start.unwrap_or_else(|| center.round().clamp(0.0, (input_len - 1) as f64) as usize);
        if weights.is_empty() {
            weights.push(1.0);
        }
        normalize_weights(&mut weights);
        out.push(AxisWeights { start, weights });
    }

    out
}

fn resample_horizontal(input: &ImageBuffer, output_width: usize, kernel: VipsKernel, centre: bool) -> ImageBuffer {
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

fn resample_vertical(input: &ImageBuffer, output_height: usize, kernel: VipsKernel, centre: bool) -> ImageBuffer {
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

fn resample_to(
    input: &ImageBuffer,
    output_width: usize,
    output_height: usize,
    kernel: VipsKernel,
    centre: bool,
) -> ImageBuffer {
    let tmp = resample_horizontal(input, output_width.max(1), kernel, centre);
    resample_vertical(&tmp, output_height.max(1), kernel, centre)
}

fn shrink_output_len(input_len: usize, shrink: f64, ceil_mode: bool) -> usize {
    let shrink = shrink.max(1.0);
    let value = input_len as f64 / shrink;
    let out = if ceil_mode { value.ceil() } else { value.floor().max(1.0) };
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
    let sx = width.max(1) as f64 / input_width.max(1) as f64;
    let sy = height.unwrap_or_else(|| ((input_height as f64 * sx).round().max(1.0)) as usize) as f64
        / input_height.max(1) as f64;

    if size == VIPS_SIZE_FORCE {
        return (
            width.max(1),
            height.unwrap_or_else(|| ((input_height as f64 * sx).round().max(1.0)) as usize).max(1),
            None,
        );
    }

    let mut scale = if crop && height.is_some() { sx.max(sy) } else { sx.min(sy) };
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

unsafe fn set_out_image(object: *mut VipsObject, buffer: ImageBuffer, like: *mut VipsImage) -> Result<(), ()> {
    unsafe { set_output_image_like(object, "out", buffer, like) }
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
        false
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
        false
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
        false
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
        false
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
    let crop = unsafe { argument_assigned(object, "crop")? } && unsafe { get_enum(object, "crop")? } != 0;
    let kernel = if unsafe { argument_assigned(object, "linear")? } && unsafe { get_bool(object, "linear")? } {
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
    let mut out = resample_to(input, out_width, out_height, kernel, false);
    if let Some((crop_width, crop_height)) = crop_to {
        out = centre_crop(&out, crop_width, crop_height);
    }
    unsafe { set_out_image(object, out, like) }
}

unsafe fn op_thumbnail(object: *mut VipsObject) -> Result<(), ()> {
    let filename = unsafe { get_string(object, "filename")? }.ok_or(())?;
    let source = unsafe { vips_source_new_from_file(std::ffi::CString::new(filename).map_err(|_| ())?.as_ptr()) };
    if source.is_null() {
        return Err(());
    }
    let image = unsafe { safe_vips_image_new_from_source_internal(source, ptr::null(), 0) };
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
    let source = unsafe { vips_source_new_from_memory(bytes.as_ptr().cast::<c_void>(), bytes.len()) };
    if source.is_null() {
        return Err(());
    }
    let image = unsafe { safe_vips_image_new_from_source_internal(source, ptr::null(), 0) };
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
    let image = unsafe { safe_vips_image_new_from_source_internal(source, ptr::null(), 0) };
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
        _ => Ok(false),
    }
}
