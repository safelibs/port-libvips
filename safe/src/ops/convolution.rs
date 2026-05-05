use crate::abi::basic::{
    VipsAngle45, VipsCombine, VipsPrecision, VIPS_ANGLE45_D0, VIPS_ANGLE45_D135, VIPS_ANGLE45_D180,
    VIPS_ANGLE45_D225, VIPS_ANGLE45_D270, VIPS_ANGLE45_D315, VIPS_ANGLE45_D45, VIPS_ANGLE45_D90,
    VIPS_COMBINE_MAX, VIPS_COMBINE_MIN, VIPS_COMBINE_SUM, VIPS_PRECISION_FLOAT,
    VIPS_PRECISION_INTEGER,
};
use crate::abi::image::{
    VipsBandFormat, VIPS_DEMAND_STYLE_SMALLTILE, VIPS_FORMAT_DOUBLE, VIPS_FORMAT_FLOAT,
    VIPS_FORMAT_UCHAR, VIPS_FORMAT_UINT,
};
use crate::abi::object::VipsObject;
use crate::pixels::format::clamp_for_format;
use crate::pixels::kernel::{gaussian_kernel, Kernel};
use crate::pixels::ImageBuffer;

use super::{
    argument_assigned, get_double, get_enum, get_image_buffer, get_image_ref, get_int,
    set_output_image_like,
};

fn conv_output_format(format: VipsBandFormat, precision: VipsPrecision) -> VipsBandFormat {
    if precision == VIPS_PRECISION_INTEGER && matches!(format, VIPS_FORMAT_DOUBLE) {
        VIPS_FORMAT_DOUBLE
    } else if precision == VIPS_PRECISION_INTEGER && !matches!(format, VIPS_FORMAT_DOUBLE) {
        format
    } else if matches!(format, VIPS_FORMAT_DOUBLE) {
        VIPS_FORMAT_DOUBLE
    } else {
        VIPS_FORMAT_FLOAT
    }
}

fn align_reference_bands(
    input: &ImageBuffer,
    reference: &ImageBuffer,
) -> Result<(ImageBuffer, ImageBuffer), ()> {
    let bands = match (input.spec.bands, reference.spec.bands) {
        (left, right) if left == right => left,
        (1, right) => right,
        (left, 1) => left,
        _ => return Err(()),
    };
    Ok((
        if input.spec.bands == bands {
            input.clone()
        } else {
            input.replicate_bands(bands)?
        },
        if reference.spec.bands == bands {
            reference.clone()
        } else {
            reference.replicate_bands(bands)?
        },
    ))
}

fn fastcor_output_format(input: VipsBandFormat, reference: VipsBandFormat) -> VipsBandFormat {
    if input == VIPS_FORMAT_DOUBLE || reference == VIPS_FORMAT_DOUBLE {
        VIPS_FORMAT_DOUBLE
    } else if input == VIPS_FORMAT_FLOAT || reference == VIPS_FORMAT_FLOAT {
        VIPS_FORMAT_FLOAT
    } else {
        VIPS_FORMAT_UINT
    }
}

fn spcor_output_format(input: VipsBandFormat, reference: VipsBandFormat) -> VipsBandFormat {
    if input == VIPS_FORMAT_DOUBLE || reference == VIPS_FORMAT_DOUBLE {
        VIPS_FORMAT_DOUBLE
    } else {
        VIPS_FORMAT_FLOAT
    }
}

pub(crate) fn apply_kernel(
    input: &ImageBuffer,
    kernel: &Kernel,
    precision: VipsPrecision,
) -> ImageBuffer {
    let (cx, cy) = kernel.origin();
    let mut out = input
        .with_format(conv_output_format(input.spec.format, precision))
        .with_origin(-(cx as i32), -(cy as i32))
        .with_demand_style(VIPS_DEMAND_STYLE_SMALLTILE);
    let scale = kernel.scale_or_one();
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            for band in 0..input.spec.bands {
                let mut sum = 0.0;
                for sample in kernel.iter() {
                    let px = x as isize + sample.dx;
                    let py = y as isize + sample.dy;
                    sum += input.sample_clamped(px, py, band) * sample.value;
                }
                out.set(x, y, band, sum / scale + kernel.offset);
            }
        }
    }
    out
}

pub(crate) fn apply_separable(
    input: &ImageBuffer,
    kernel: &Kernel,
    precision: VipsPrecision,
) -> Result<ImageBuffer, ()> {
    let vector = if kernel.height == 1 {
        kernel.data.clone()
    } else if kernel.width == 1 {
        (0..kernel.height).map(|index| kernel.data[index]).collect()
    } else {
        return Err(());
    };
    let radius = vector.len() as isize / 2;
    let mut tmp = input.with_format(conv_output_format(input.spec.format, precision));
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            for band in 0..input.spec.bands {
                let mut sum = 0.0;
                for (index, value) in vector.iter().copied().enumerate() {
                    let sx = x as isize + index as isize - radius;
                    sum += input.sample_clamped(sx, y as isize, band) * value;
                }
                tmp.set(x, y, band, sum / kernel.scale_or_one());
            }
        }
    }
    let mut out = tmp
        .clone()
        .with_demand_style(VIPS_DEMAND_STYLE_SMALLTILE)
        .with_origin(
            if kernel.width == 1 {
                -(radius as i32)
            } else {
                0
            },
            if kernel.height == 1 {
                -(radius as i32)
            } else {
                0
            },
        );
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            for band in 0..input.spec.bands {
                let mut sum = 0.0;
                for (index, value) in vector.iter().copied().enumerate() {
                    let sy = y as isize + index as isize - radius;
                    sum += tmp.sample_clamped(x as isize, sy, band) * value;
                }
                out.set(x, y, band, sum / kernel.scale_or_one());
            }
        }
    }
    Ok(out)
}

unsafe fn op_conv(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let mask = unsafe { get_image_ref(object, "mask")? };
    let kernel = Kernel::from_image(mask)?;
    let precision = if unsafe { argument_assigned(object, "precision")? } {
        unsafe { get_enum(object, "precision")? as VipsPrecision }
    } else {
        crate::abi::basic::VIPS_PRECISION_FLOAT
    };
    let out = apply_kernel(&input, &kernel, precision);
    unsafe {
        crate::runtime::object::object_unref(mask);
    }
    let image = unsafe { get_image_ref(object, "in")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

unsafe fn op_convsep(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let mask = unsafe { get_image_ref(object, "mask")? };
    let kernel = Kernel::from_image(mask)?;
    let precision = if unsafe { argument_assigned(object, "precision")? } {
        unsafe { get_enum(object, "precision")? as VipsPrecision }
    } else {
        crate::abi::basic::VIPS_PRECISION_FLOAT
    };
    let out = apply_separable(&input, &kernel, precision)?;
    unsafe {
        crate::runtime::object::object_unref(mask);
    }
    let image = unsafe { get_image_ref(object, "in")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

fn rotate_mask(mut mask: Kernel, angle: VipsAngle45) -> Kernel {
    let turns = match angle {
        VIPS_ANGLE45_D0 => 0,
        VIPS_ANGLE45_D45 => 1,
        VIPS_ANGLE45_D90 => 2,
        VIPS_ANGLE45_D135 => 3,
        VIPS_ANGLE45_D180 => 4,
        VIPS_ANGLE45_D225 => 5,
        VIPS_ANGLE45_D270 => 6,
        VIPS_ANGLE45_D315 => 7,
        _ => 0,
    };
    for _ in 0..turns {
        mask = mask.rotate_45();
    }
    mask
}

unsafe fn op_compass(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let mask = unsafe { get_image_ref(object, "mask")? };
    let base = Kernel::from_image(mask)?;
    let times = if unsafe { argument_assigned(object, "times")? } {
        usize::try_from(unsafe { get_int(object, "times")? }).map_err(|_| ())?
    } else {
        2
    };
    let angle = if unsafe { argument_assigned(object, "angle")? } {
        unsafe { get_enum(object, "angle")? as VipsAngle45 }
    } else {
        VIPS_ANGLE45_D90
    };
    let combine = if unsafe { argument_assigned(object, "combine")? } {
        unsafe { get_enum(object, "combine")? as VipsCombine }
    } else {
        VIPS_COMBINE_MAX
    };
    let precision = if unsafe { argument_assigned(object, "precision")? } {
        unsafe { get_enum(object, "precision")? as VipsPrecision }
    } else {
        crate::abi::basic::VIPS_PRECISION_FLOAT
    };
    let mut results = Vec::with_capacity(times);
    let mut current = base.clone();
    for _ in 0..times {
        let conv = apply_kernel(&input, &current, precision);
        results.push(conv.with_format(conv.spec.format));
        current = rotate_mask(current, angle);
    }
    let mut out = results.first().ok_or(())?.clone();
    for index in 0..out.data.len() {
        let value = match combine {
            VIPS_COMBINE_SUM => results.iter().map(|image| image.data[index].abs()).sum(),
            VIPS_COMBINE_MIN => results
                .iter()
                .map(|image| image.data[index].abs())
                .fold(f64::INFINITY, f64::min),
            _ => results
                .iter()
                .map(|image| image.data[index].abs())
                .fold(f64::NEG_INFINITY, f64::max),
        };
        out.data[index] = value;
    }
    unsafe {
        crate::runtime::object::object_unref(mask);
    }
    let image = unsafe { get_image_ref(object, "in")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

fn fixed_kernel(values: &[f64]) -> Kernel {
    Kernel::new(3, 3, values.to_vec(), 1.0, 0.0)
}

unsafe fn edge_pair(object: *mut VipsObject, gx: Kernel, gy: Kernel) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let x = apply_kernel(&input, &gx, crate::abi::basic::VIPS_PRECISION_FLOAT);
    let y = apply_kernel(&input, &gy, crate::abi::basic::VIPS_PRECISION_FLOAT);
    let mut out = x.clone();
    for index in 0..out.data.len() {
        out.data[index] = (x.data[index] * x.data[index] + y.data[index] * y.data[index]).sqrt();
    }
    let image = unsafe { get_image_ref(object, "in")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

fn canny_output_format(format: VipsBandFormat) -> VipsBandFormat {
    if format == VIPS_FORMAT_UCHAR {
        VIPS_FORMAT_UCHAR
    } else if format == VIPS_FORMAT_DOUBLE {
        VIPS_FORMAT_DOUBLE
    } else {
        VIPS_FORMAT_FLOAT
    }
}

fn canny_blur(
    input: &ImageBuffer,
    sigma: f64,
    precision: VipsPrecision,
) -> Result<ImageBuffer, ()> {
    if !(sigma.is_finite() && sigma >= 0.01) {
        return Err(());
    }
    if sigma < 0.2 {
        return Ok(input.clone());
    }

    let kernel = gaussian_kernel(sigma, 0.2, true, precision)?;
    apply_separable(input, &kernel, precision)
}

fn canny_index(width: usize, bands: usize, x: usize, y: usize, band: usize) -> usize {
    (y * width + x) * bands + band
}

fn canny_direction_offset(direction: usize) -> (isize, isize) {
    match direction & 7 {
        0 => (0, -1),
        1 => (-1, -1),
        2 => (-1, 0),
        3 => (-1, 1),
        4 => (0, 1),
        5 => (1, 1),
        6 => (1, 0),
        _ => (1, -1),
    }
}

fn canny_magnitude_at(
    magnitudes: &[f64],
    width: usize,
    height: usize,
    bands: usize,
    x: usize,
    y: usize,
    band: usize,
    direction: usize,
) -> f64 {
    let (dx, dy) = canny_direction_offset(direction);
    let sx = (x as isize + dx).clamp(0, width.saturating_sub(1) as isize) as usize;
    let sy = (y as isize + dy).clamp(0, height.saturating_sub(1) as isize) as usize;
    magnitudes[canny_index(width, bands, sx, sy, band)]
}

unsafe fn op_canny(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    if input.spec.bands == 0 {
        return Err(());
    }
    let sigma = if unsafe { argument_assigned(object, "sigma")? } {
        unsafe { get_double(object, "sigma")? }
    } else {
        1.4
    };
    let precision = if unsafe { argument_assigned(object, "precision")? } {
        unsafe { get_enum(object, "precision")? as VipsPrecision }
    } else {
        VIPS_PRECISION_FLOAT
    };
    let blurred = canny_blur(&input, sigma, precision)?;
    let width = input.spec.width;
    let height = input.spec.height;
    let bands = input.spec.bands;
    let mut magnitudes = vec![0.0; width.saturating_mul(height).saturating_mul(bands)];
    let mut directions = magnitudes.clone();

    for y in 0..height {
        for x in 0..width {
            let ix = x as isize;
            let iy = y as isize;
            for band in 0..bands {
                let gx = -blurred.sample_clamped(ix - 1, iy - 1, band)
                    + blurred.sample_clamped(ix, iy - 1, band)
                    - blurred.sample_clamped(ix - 1, iy, band)
                    + blurred.sample_clamped(ix, iy, band);
                let gy = -blurred.sample_clamped(ix - 1, iy - 1, band)
                    - blurred.sample_clamped(ix, iy - 1, band)
                    + blurred.sample_clamped(ix - 1, iy, band)
                    + blurred.sample_clamped(ix, iy, band);
                let index = canny_index(width, bands, x, y, band);
                magnitudes[index] = (gx * gx + gy * gy + 256.0) / 512.0;
                directions[index] =
                    256.0 * (gx.atan2(gy).to_degrees() + 360.0).rem_euclid(360.0) / 360.0;
            }
        }
    }

    let output_format = canny_output_format(input.spec.format);
    let mut out = ImageBuffer::new(
        width,
        height,
        bands,
        output_format,
        input.spec.coding,
        input.spec.interpretation,
    );
    out.spec.xres = input.spec.xres;
    out.spec.yres = input.spec.yres;
    out.spec.xoffset = input.spec.xoffset;
    out.spec.yoffset = input.spec.yoffset;
    out.spec.dhint = input.spec.dhint;

    for y in 0..height {
        for x in 0..width {
            for band in 0..bands {
                let index = canny_index(width, bands, x, y, band);
                let magnitude = magnitudes[index];
                let theta = directions[index];
                let low_theta = ((theta / 32.0) as usize) & 7;
                let high_theta = (low_theta + 1) & 7;
                let residual = theta - (low_theta as f64 * 32.0);
                let low_a =
                    canny_magnitude_at(&magnitudes, width, height, bands, x, y, band, low_theta);
                let low_b =
                    canny_magnitude_at(&magnitudes, width, height, bands, x, y, band, high_theta);
                let low = (low_a * (32.0 - residual) + low_b * residual) / 32.0;
                let high_a = canny_magnitude_at(
                    &magnitudes,
                    width,
                    height,
                    bands,
                    x,
                    y,
                    band,
                    (low_theta + 4) & 7,
                );
                let high_b = canny_magnitude_at(
                    &magnitudes,
                    width,
                    height,
                    bands,
                    x,
                    y,
                    band,
                    (high_theta + 4) & 7,
                );
                let high = (high_a * (32.0 - residual) + high_b * residual) / 32.0;
                let value = if magnitude <= low || magnitude < high {
                    0.0
                } else {
                    magnitude
                };
                out.set(x, y, band, clamp_for_format(value, output_format));
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

unsafe fn op_fastcor(object: *mut VipsObject) -> Result<(), ()> {
    let like = unsafe { get_image_ref(object, "in")? };
    let input = unsafe { get_image_buffer(object, "in")? };
    let reference = unsafe { get_image_buffer(object, "ref")? };
    let (input, reference) = align_reference_bands(&input, &reference)?;
    let mut out = input.with_shape(input.spec.width, input.spec.height, input.spec.bands);
    out.spec.format = fastcor_output_format(input.spec.format, reference.spec.format);
    let cx = reference.spec.width as isize / 2;
    let cy = reference.spec.height as isize / 2;
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            for band in 0..input.spec.bands {
                let mut sum = 0.0;
                for ry in 0..reference.spec.height {
                    for rx in 0..reference.spec.width {
                        let input_value = input.sample_clamped(
                            x as isize + rx as isize - cx,
                            y as isize + ry as isize - cy,
                            band,
                        );
                        let ref_value = reference.get(rx, ry, band);
                        let diff = ref_value - input_value;
                        sum += diff * diff;
                    }
                }
                out.set(x, y, band, sum);
            }
        }
    }
    let result = unsafe { set_output_image_like(object, "out", out, like) };
    unsafe {
        crate::runtime::object::object_unref(like);
    }
    result
}

unsafe fn op_spcor(object: *mut VipsObject) -> Result<(), ()> {
    let like = unsafe { get_image_ref(object, "in")? };
    let input = unsafe { get_image_buffer(object, "in")? };
    let reference = unsafe { get_image_buffer(object, "ref")? };
    let (input, reference) = align_reference_bands(&input, &reference)?;
    let ref_pixels = (reference.spec.width * reference.spec.height).max(1) as f64;
    let mut rmean = vec![0.0; reference.spec.bands];
    let mut c1 = vec![0.0; reference.spec.bands];
    for band in 0..reference.spec.bands {
        let mut sum = 0.0;
        for y in 0..reference.spec.height {
            for x in 0..reference.spec.width {
                sum += reference.get(x, y, band);
            }
        }
        rmean[band] = sum / ref_pixels;
        let mut variance = 0.0;
        for y in 0..reference.spec.height {
            for x in 0..reference.spec.width {
                let diff = reference.get(x, y, band) - rmean[band];
                variance += diff * diff;
            }
        }
        c1[band] = variance.sqrt();
    }

    let mut out = input.with_shape(input.spec.width, input.spec.height, input.spec.bands);
    out.spec.format = spcor_output_format(input.spec.format, reference.spec.format);
    let cx = reference.spec.width as isize / 2;
    let cy = reference.spec.height as isize / 2;
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            for band in 0..input.spec.bands {
                let mut sum1 = 0.0;
                for ry in 0..reference.spec.height {
                    for rx in 0..reference.spec.width {
                        sum1 += input.sample_clamped(
                            x as isize + rx as isize - cx,
                            y as isize + ry as isize - cy,
                            band,
                        );
                    }
                }
                let imean = sum1 / ref_pixels;
                let mut sum2 = 0.0;
                let mut sum3 = 0.0;
                for ry in 0..reference.spec.height {
                    for rx in 0..reference.spec.width {
                        let input_value = input.sample_clamped(
                            x as isize + rx as isize - cx,
                            y as isize + ry as isize - cy,
                            band,
                        );
                        let ref_value = reference.get(rx, ry, band);
                        let delta = input_value - imean;
                        sum2 += delta * delta;
                        sum3 += (ref_value - rmean[band]) * delta;
                    }
                }
                let denom = c1[band] * sum2.sqrt();
                let mut cc = if denom <= f64::EPSILON {
                    0.0
                } else {
                    sum3 / denom
                };
                if (cc - 1.0).abs() < 1e-12 {
                    cc = 1.0;
                } else if (cc + 1.0).abs() < 1e-12 {
                    cc = -1.0;
                } else {
                    cc = cc.clamp(-1.0, 1.0);
                }
                out.set(x, y, band, cc);
            }
        }
    }
    let result = unsafe { set_output_image_like(object, "out", out, like) };
    unsafe {
        crate::runtime::object::object_unref(like);
    }
    result
}

unsafe fn op_gaussblur(object: *mut VipsObject) -> Result<(), ()> {
    let sigma = unsafe { get_double(object, "sigma")? };
    let min_ampl = if unsafe { argument_assigned(object, "min_ampl")? } {
        unsafe { get_double(object, "min_ampl")? }
    } else {
        0.2
    };
    if sigma < 0.2 {
        let input = unsafe { get_image_buffer(object, "in")? };
        let image = unsafe { get_image_ref(object, "in")? };
        let result = unsafe { set_output_image_like(object, "out", input, image) };
        unsafe {
            crate::runtime::object::object_unref(image);
        }
        return result;
    }
    let precision = if unsafe { argument_assigned(object, "precision")? } {
        unsafe { get_enum(object, "precision")? as VipsPrecision }
    } else {
        VIPS_PRECISION_INTEGER
    };
    let kernel = gaussian_kernel(sigma, min_ampl, true, precision)?;
    let input = unsafe { get_image_buffer(object, "in")? };
    let out = apply_separable(&input, &kernel, precision)?;
    let image = unsafe { get_image_ref(object, "in")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

unsafe fn op_sharpen(object: *mut VipsObject) -> Result<(), ()> {
    let like = unsafe { get_image_ref(object, "in")? };
    let input = unsafe { get_image_buffer(object, "in")? };
    let sigma = if unsafe { argument_assigned(object, "sigma")? } {
        unsafe { get_double(object, "sigma")? }
    } else if unsafe { argument_assigned(object, "radius")? } {
        1.0 + unsafe { get_int(object, "radius")? } as f64 / 2.0
    } else {
        0.5
    };
    let x1 = if unsafe { argument_assigned(object, "x1")? } {
        unsafe { get_double(object, "x1")? }
    } else {
        2.0
    };
    let y2 = if unsafe { argument_assigned(object, "y2")? } {
        unsafe { get_double(object, "y2")? }
    } else {
        10.0
    };
    let y3 = if unsafe { argument_assigned(object, "y3")? } {
        unsafe { get_double(object, "y3")? }
    } else {
        20.0
    };
    let m1 = if unsafe { argument_assigned(object, "m1")? } {
        unsafe { get_double(object, "m1")? }
    } else {
        0.0
    };
    let m2 = if unsafe { argument_assigned(object, "m2")? } {
        unsafe { get_double(object, "m2")? }
    } else {
        3.0
    };

    if m1 == 0.0 && m2 == 0.0 {
        let result = unsafe { set_output_image_like(object, "out", input, like) };
        unsafe {
            crate::runtime::object::object_unref(like);
        }
        return result;
    }

    let blur = if sigma < 0.2 {
        input.clone()
    } else {
        let kernel = gaussian_kernel(sigma, 0.1, true, VIPS_PRECISION_INTEGER)?;
        apply_separable(&input, &kernel, VIPS_PRECISION_INTEGER)?
    };
    let mut out = input.clone();
    for index in 0..out.data.len() {
        let diff = input.data[index] - blur.data[index];
        let mut mapped = if diff.abs() < x1 {
            diff * m1
        } else {
            diff * m2
        };
        mapped = mapped.clamp(-y3, y2);
        out.data[index] = clamp_for_format(input.data[index] + mapped, out.spec.format);
    }
    let result = unsafe { set_output_image_like(object, "out", out, like) };
    unsafe {
        crate::runtime::object::object_unref(like);
    }
    result
}

pub(crate) unsafe fn dispatch(object: *mut VipsObject, nickname: &str) -> Result<bool, ()> {
    match nickname {
        "conv" => {
            unsafe { op_conv(object)? };
            Ok(true)
        }
        "convsep" => {
            unsafe { op_convsep(object)? };
            Ok(true)
        }
        "compass" => {
            unsafe { op_compass(object)? };
            Ok(true)
        }
        "fastcor" => {
            unsafe { op_fastcor(object)? };
            Ok(true)
        }
        "gaussblur" => {
            unsafe { op_gaussblur(object)? };
            Ok(true)
        }
        "sharpen" => {
            unsafe { op_sharpen(object)? };
            Ok(true)
        }
        "canny" => {
            unsafe { op_canny(object)? };
            Ok(true)
        }
        "sobel" => {
            unsafe {
                edge_pair(
                    object,
                    fixed_kernel(&[-1.0, 0.0, 1.0, -2.0, 0.0, 2.0, -1.0, 0.0, 1.0]),
                    fixed_kernel(&[-1.0, -2.0, -1.0, 0.0, 0.0, 0.0, 1.0, 2.0, 1.0]),
                )?
            };
            Ok(true)
        }
        "spcor" => {
            unsafe { op_spcor(object)? };
            Ok(true)
        }
        "scharr" => {
            unsafe {
                edge_pair(
                    object,
                    fixed_kernel(&[-3.0, 0.0, 3.0, -10.0, 0.0, 10.0, -3.0, 0.0, 3.0]),
                    fixed_kernel(&[-3.0, -10.0, -3.0, 0.0, 0.0, 0.0, 3.0, 10.0, 3.0]),
                )?
            };
            Ok(true)
        }
        "prewitt" => {
            unsafe {
                edge_pair(
                    object,
                    fixed_kernel(&[-1.0, 0.0, 1.0, -1.0, 0.0, 1.0, -1.0, 0.0, 1.0]),
                    fixed_kernel(&[-1.0, -1.0, -1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0]),
                )?
            };
            Ok(true)
        }
        _ => Ok(false),
    }
}
