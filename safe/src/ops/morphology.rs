use std::collections::VecDeque;

use crate::abi::basic::{
    VipsDirection, VipsOperationMorphology, VIPS_DIRECTION_VERTICAL,
    VIPS_OPERATION_MORPHOLOGY_ERODE,
};
use crate::abi::image::{
    VIPS_CODING_NONE, VIPS_DEMAND_STYLE_SMALLTILE, VIPS_FORMAT_FLOAT, VIPS_FORMAT_INT,
    VIPS_FORMAT_UCHAR, VIPS_INTERPRETATION_MULTIBAND,
};
use crate::abi::object::VipsObject;
use crate::pixels::kernel::Kernel;
use crate::pixels::ImageBuffer;

use super::{
    get_enum, get_image_buffer, get_image_ref, get_int, set_output_double, set_output_image,
    set_output_image_like, set_output_int,
};

fn morph_coeff(value: f64) -> Result<u8, ()> {
    let value = value.round();
    if (value - 0.0).abs() < f64::EPSILON {
        Ok(0)
    } else if (value - 128.0).abs() < f64::EPSILON {
        Ok(128)
    } else if (value - 255.0).abs() < f64::EPSILON {
        Ok(255)
    } else {
        Err(())
    }
}

unsafe fn op_morph(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let mask = unsafe { get_image_ref(object, "mask")? };
    let kernel = Kernel::from_image(mask)?;
    let morph = unsafe { get_enum(object, "morph")? } as VipsOperationMorphology;
    let (cx, cy) = kernel.origin();
    let coeffs = kernel
        .iter()
        .map(|sample| morph_coeff(sample.value).map(|coeff| (sample.dx, sample.dy, coeff)))
        .collect::<Result<Vec<_>, _>>()?;
    let mut out = input
        .with_format(VIPS_FORMAT_UCHAR)
        .with_origin(-(cx as i32), -(cy as i32))
        .with_demand_style(VIPS_DEMAND_STYLE_SMALLTILE);
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            for band in 0..input.spec.bands {
                let mut value = if morph == VIPS_OPERATION_MORPHOLOGY_ERODE {
                    255.0
                } else {
                    0.0
                };
                for (dx, dy, coeff) in &coeffs {
                    let sample =
                        input.sample_clamped(x as isize + dx, y as isize + dy, band) != 0.0;
                    match morph {
                        VIPS_OPERATION_MORPHOLOGY_ERODE => {
                            if (*coeff == 255 && !sample) || (*coeff == 0 && sample) {
                                value = 0.0;
                                break;
                            }
                        }
                        _ => {
                            if (*coeff == 255 && sample) || (*coeff == 0 && !sample) {
                                value = 255.0;
                                break;
                            }
                        }
                    }
                }
                out.set(x, y, band, value);
            }
        }
    }
    unsafe { crate::runtime::object::object_unref(mask) };
    let image = unsafe { get_image_ref(object, "in")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe { crate::runtime::object::object_unref(image) };
    result
}

unsafe fn op_rank(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let width = usize::try_from(unsafe { get_int(object, "width")? }).map_err(|_| ())?;
    let height = usize::try_from(unsafe { get_int(object, "height")? }).map_err(|_| ())?;
    let index = usize::try_from(unsafe { get_int(object, "index")? }).map_err(|_| ())?;
    if width == 0 || height == 0 || index >= width * height {
        return Err(());
    }
    let cx = width as isize / 2;
    let cy = height as isize / 2;
    let mut out = input.clone();
    let mut window = Vec::with_capacity(width * height);
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            for band in 0..input.spec.bands {
                window.clear();
                for ky in 0..height {
                    for kx in 0..width {
                        window.push(input.sample_clamped(
                            x as isize + kx as isize - cx,
                            y as isize + ky as isize - cy,
                            band,
                        ));
                    }
                }
                window.sort_by(f64::total_cmp);
                out.set(x, y, band, window[index]);
            }
        }
    }
    let image = unsafe { get_image_ref(object, "in")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe { crate::runtime::object::object_unref(image) };
    result
}

unsafe fn op_countlines(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let direction = unsafe { get_enum(object, "direction")? } as VipsDirection;
    let mut total = 0.0;
    let count = if direction == VIPS_DIRECTION_VERTICAL {
        input.spec.height.max(1)
    } else {
        input.spec.width.max(1)
    };
    if direction == VIPS_DIRECTION_VERTICAL {
        for y in 0..input.spec.height {
            let mut lines = 0.0;
            for x in 1..input.spec.width {
                let prev = input.get(x - 1, y, 0) < 128.0;
                let next = input.get(x, y, 0) >= 128.0;
                if prev && next {
                    lines += 1.0;
                }
            }
            total += lines;
        }
    } else {
        for x in 0..input.spec.width {
            let mut lines = 0.0;
            for y in 1..input.spec.height {
                let prev = input.get(x, y - 1, 0) < 128.0;
                let next = input.get(x, y, 0) >= 128.0;
                if prev && next {
                    lines += 1.0;
                }
            }
            total += lines;
        }
    }
    unsafe { set_output_double(object, "nolines", total / count as f64) }
}

fn pixel_is_non_zero(buffer: &ImageBuffer, x: usize, y: usize) -> bool {
    (0..buffer.spec.bands).any(|band| buffer.get(x, y, band) != 0.0)
}

fn pixel_matches(buffer: &ImageBuffer, x: usize, y: usize, reference: &[f64]) -> bool {
    reference
        .iter()
        .enumerate()
        .all(|(band, sample)| buffer.get(x, y, band).to_bits() == sample.to_bits())
}

fn copy_pixel(
    source: &ImageBuffer,
    sx: usize,
    sy: usize,
    target: &mut ImageBuffer,
    tx: usize,
    ty: usize,
) {
    for band in 0..source.spec.bands {
        target.set(tx, ty, band, source.get(sx, sy, band));
    }
}

fn init_output_like(input: &ImageBuffer, buffer: &mut ImageBuffer) {
    buffer.spec.xres = input.spec.xres;
    buffer.spec.yres = input.spec.yres;
    buffer.spec.xoffset = input.spec.xoffset;
    buffer.spec.yoffset = input.spec.yoffset;
    buffer.spec.dhint = input.spec.dhint;
}

unsafe fn op_labelregions(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let like = unsafe { get_image_ref(object, "in")? };
    let mut mask = ImageBuffer::new(
        input.spec.width,
        input.spec.height,
        1,
        VIPS_FORMAT_INT,
        VIPS_CODING_NONE,
        VIPS_INTERPRETATION_MULTIBAND,
    );
    init_output_like(&input, &mut mask);

    let mut queue = VecDeque::new();
    let mut segments = 1i32;

    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            if mask.get(x, y, 0) != 0.0 {
                continue;
            }

            let reference = (0..input.spec.bands)
                .map(|band| input.get(x, y, band))
                .collect::<Vec<_>>();
            queue.push_back((x, y));
            while let Some((cx, cy)) = queue.pop_front() {
                if mask.get(cx, cy, 0) != 0.0 || !pixel_matches(&input, cx, cy, &reference) {
                    continue;
                }

                mask.set(cx, cy, 0, segments as f64);
                if cx > 0 {
                    queue.push_back((cx - 1, cy));
                }
                if cx + 1 < input.spec.width {
                    queue.push_back((cx + 1, cy));
                }
                if cy > 0 {
                    queue.push_back((cx, cy - 1));
                }
                if cy + 1 < input.spec.height {
                    queue.push_back((cx, cy + 1));
                }
            }

            segments += 1;
        }
    }

    let mask_image = mask.into_image_like(like);
    let result = unsafe {
        set_output_image(object, "mask", mask_image)
            .and_then(|_| set_output_int(object, "segments", segments))
    };
    unsafe { crate::runtime::object::object_unref(like) };
    result
}

unsafe fn op_fill_nearest(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let like = unsafe { get_image_ref(object, "in")? };
    let mut out = input.clone();
    let mut distance = ImageBuffer::new(
        input.spec.width,
        input.spec.height,
        1,
        VIPS_FORMAT_FLOAT,
        VIPS_CODING_NONE,
        VIPS_INTERPRETATION_MULTIBAND,
    );
    init_output_like(&input, &mut distance);

    let mut seeds = Vec::new();
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            if pixel_is_non_zero(&input, x, y) {
                seeds.push((x, y));
            }
        }
    }

    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            if pixel_is_non_zero(&input, x, y) {
                continue;
            }

            let mut best: Option<(u64, usize, usize)> = None;
            for &(sx, sy) in &seeds {
                let dx = sx as i64 - x as i64;
                let dy = sy as i64 - y as i64;
                let distance_sq = (dx * dx + dy * dy) as u64;
                if best.is_none_or(|(best_sq, _, _)| distance_sq < best_sq) {
                    best = Some((distance_sq, sx, sy));
                }
            }

            if let Some((distance_sq, sx, sy)) = best {
                copy_pixel(&input, sx, sy, &mut out, x, y);
                distance.set(x, y, 0, (distance_sq as f64).sqrt());
            }
        }
    }

    let out_image = out.into_image_like(like);
    let distance_image = distance.into_image_like(like);
    let result = unsafe {
        set_output_image(object, "out", out_image)
            .and_then(|_| set_output_image(object, "distance", distance_image))
    };
    unsafe { crate::runtime::object::object_unref(like) };
    result
}

pub(crate) unsafe fn dispatch(object: *mut VipsObject, nickname: &str) -> Result<bool, ()> {
    match nickname {
        "fill_nearest" => {
            unsafe { op_fill_nearest(object)? };
            Ok(true)
        }
        "labelregions" => {
            unsafe { op_labelregions(object)? };
            Ok(true)
        }
        "morph" => {
            unsafe { op_morph(object)? };
            Ok(true)
        }
        "rank" => {
            unsafe { op_rank(object)? };
            Ok(true)
        }
        "countlines" => {
            unsafe { op_countlines(object)? };
            Ok(true)
        }
        _ => Ok(false),
    }
}
