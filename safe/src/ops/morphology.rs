use crate::abi::basic::{
    VipsDirection, VipsOperationMorphology, VIPS_DIRECTION_HORIZONTAL, VIPS_DIRECTION_VERTICAL,
    VIPS_OPERATION_MORPHOLOGY_DILATE, VIPS_OPERATION_MORPHOLOGY_ERODE,
};
use crate::abi::object::VipsObject;
use crate::pixels::kernel::Kernel;
use crate::pixels::ImageBuffer;

use super::{
    get_enum, get_image_buffer, get_image_ref, get_int, set_output_double, set_output_image_like,
};

unsafe fn op_morph(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let mask = unsafe { get_image_ref(object, "mask")? };
    let kernel = Kernel::from_image(mask)?;
    let morph = unsafe { get_enum(object, "morph")? } as VipsOperationMorphology;
    let mut out = input.clone();
    let cx = kernel.width as isize / 2;
    let cy = kernel.height as isize / 2;
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            for band in 0..input.spec.bands {
                let mut value = if morph == VIPS_OPERATION_MORPHOLOGY_ERODE {
                    f64::INFINITY
                } else {
                    f64::NEG_INFINITY
                };
                for ky in 0..kernel.height {
                    for kx in 0..kernel.width {
                        if kernel.at(kx, ky) == 0.0 {
                            continue;
                        }
                        let sample = input.sample_clamped(
                            x as isize + kx as isize - cx,
                            y as isize + ky as isize - cy,
                            band,
                        );
                        if morph == VIPS_OPERATION_MORPHOLOGY_ERODE {
                            value = value.min(sample);
                        } else {
                            value = value.max(sample);
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
        input.spec.width.max(1)
    } else {
        input.spec.height.max(1)
    };
    if direction == VIPS_DIRECTION_VERTICAL {
        for x in 0..input.spec.width {
            let mut lines = 0.0;
            for y in 1..input.spec.height {
                let prev = input.get(x, y - 1, 0) >= 128.0;
                let next = input.get(x, y, 0) >= 128.0;
                if prev != next {
                    lines += 1.0;
                }
            }
            total += lines;
        }
    } else {
        for y in 0..input.spec.height {
            let mut lines = 0.0;
            for x in 1..input.spec.width {
                let prev = input.get(x - 1, y, 0) >= 128.0;
                let next = input.get(x, y, 0) >= 128.0;
                if prev != next {
                    lines += 1.0;
                }
            }
            total += lines;
        }
    }
    unsafe { set_output_double(object, "nolines", total / count as f64) }
}

pub(crate) unsafe fn dispatch(object: *mut VipsObject, nickname: &str) -> Result<bool, ()> {
    match nickname {
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
