use crate::abi::basic::{
    VipsAngle45, VipsCombine, VipsPrecision, VIPS_ANGLE45_D0, VIPS_ANGLE45_D135,
    VIPS_ANGLE45_D180, VIPS_ANGLE45_D225, VIPS_ANGLE45_D270, VIPS_ANGLE45_D315,
    VIPS_ANGLE45_D45, VIPS_ANGLE45_D90, VIPS_COMBINE_MAX, VIPS_COMBINE_MIN,
    VIPS_COMBINE_SUM, VIPS_PRECISION_INTEGER,
};
use crate::abi::image::{VipsBandFormat, VIPS_FORMAT_DOUBLE, VIPS_FORMAT_FLOAT};
use crate::abi::object::VipsObject;
use crate::pixels::kernel::{gaussian_kernel, Kernel};
use crate::pixels::ImageBuffer;

use super::{argument_assigned, get_double, get_enum, get_image_buffer, get_image_ref, set_output_image_like};

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

fn apply_kernel(input: &ImageBuffer, kernel: &Kernel, precision: VipsPrecision) -> ImageBuffer {
    let mut out = input.with_format(conv_output_format(input.spec.format, precision));
    let scale = kernel.scale_or_one();
    let cx = kernel.width as isize / 2;
    let cy = kernel.height as isize / 2;
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            for band in 0..input.spec.bands {
                let mut sum = 0.0;
                for ky in 0..kernel.height {
                    for kx in 0..kernel.width {
                        let px = x as isize + kx as isize - cx;
                        let py = y as isize + ky as isize - cy;
                        sum += input.sample_clamped(px, py, band) * kernel.at(kx, ky);
                    }
                }
                out.set(x, y, band, sum / scale + kernel.offset);
            }
        }
    }
    out
}

fn apply_separable(input: &ImageBuffer, kernel: &Kernel, precision: VipsPrecision) -> Result<ImageBuffer, ()> {
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
    let mut out = tmp.clone();
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
        usize::try_from(unsafe { super::get_int(object, "times")? }).map_err(|_| ())?
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
        "gaussblur" => {
            unsafe { op_gaussblur(object)? };
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
