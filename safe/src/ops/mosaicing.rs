use crate::abi::basic::{
    VipsDirection, VIPS_DIRECTION_HORIZONTAL, VIPS_DIRECTION_VERTICAL,
};
use crate::abi::object::VipsObject;
use crate::pixels::format::common_format;
use crate::pixels::ImageBuffer;
use crate::runtime::object::object_unref;

use super::{get_enum, get_image_buffer, get_image_ref, get_int, set_output_image_like};

fn replicate_if_needed(buffer: &ImageBuffer, bands: usize) -> Result<ImageBuffer, ()> {
    if buffer.spec.bands == bands {
        Ok(buffer.clone())
    } else {
        buffer.replicate_bands(bands)
    }
}

fn align_for_mosaic(reference: &ImageBuffer, secondary: &ImageBuffer) -> Result<(ImageBuffer, ImageBuffer), ()> {
    let format = common_format(reference.spec.format, secondary.spec.format).ok_or(())?;
    let bands = match (reference.spec.bands, secondary.spec.bands) {
        (a, b) if a == b => a,
        (1, b) => b,
        (a, 1) => a,
        _ => return Err(()),
    };
    Ok((
        replicate_if_needed(reference, bands)?.with_format(format),
        replicate_if_needed(secondary, bands)?.with_format(format),
    ))
}

fn overlap_weight(
    direction: VipsDirection,
    x: usize,
    y: usize,
    overlap_left: usize,
    overlap_top: usize,
    overlap_width: usize,
    overlap_height: usize,
) -> f64 {
    match direction {
        VIPS_DIRECTION_VERTICAL => {
            if overlap_height <= 1 {
                0.5
            } else {
                (y.saturating_sub(overlap_top) as f64 / (overlap_height - 1) as f64).clamp(0.0, 1.0)
            }
        }
        _ => {
            if overlap_width <= 1 {
                0.5
            } else {
                (x.saturating_sub(overlap_left) as f64 / (overlap_width - 1) as f64).clamp(0.0, 1.0)
            }
        }
    }
}

fn compose(
    reference: &ImageBuffer,
    secondary: &ImageBuffer,
    dx: i32,
    dy: i32,
    direction: VipsDirection,
    _blend: i32,
) -> ImageBuffer {
    let min_x = 0.min(dx);
    let min_y = 0.min(dy);
    let max_x = (reference.spec.width as i32).max(dx.saturating_add(secondary.spec.width as i32));
    let max_y = (reference.spec.height as i32).max(dy.saturating_add(secondary.spec.height as i32));

    let ref_left = (-min_x) as usize;
    let ref_top = (-min_y) as usize;
    let sec_left = dx.saturating_sub(min_x) as usize;
    let sec_top = dy.saturating_sub(min_y) as usize;
    let out_width = max_x.saturating_sub(min_x) as usize;
    let out_height = max_y.saturating_sub(min_y) as usize;

    let mut out = ImageBuffer::new(
        out_width,
        out_height,
        reference.spec.bands,
        reference.spec.format,
        reference.spec.coding,
        reference.spec.interpretation,
    );
    out.spec.xres = reference.spec.xres;
    out.spec.yres = reference.spec.yres;
    out.spec.xoffset = reference.spec.xoffset;
    out.spec.yoffset = reference.spec.yoffset;
    out.spec.dhint = reference.spec.dhint;

    let overlap_left = ref_left.max(sec_left);
    let overlap_top = ref_top.max(sec_top);
    let overlap_right = (ref_left + reference.spec.width).min(sec_left + secondary.spec.width);
    let overlap_bottom = (ref_top + reference.spec.height).min(sec_top + secondary.spec.height);
    let overlap_width = overlap_right.saturating_sub(overlap_left);
    let overlap_height = overlap_bottom.saturating_sub(overlap_top);

    for y in 0..out_height {
        for x in 0..out_width {
            let ref_inside = x >= ref_left
                && y >= ref_top
                && x < ref_left + reference.spec.width
                && y < ref_top + reference.spec.height;
            let sec_inside = x >= sec_left
                && y >= sec_top
                && x < sec_left + secondary.spec.width
                && y < sec_top + secondary.spec.height;

            for band in 0..reference.spec.bands {
                let value = match (ref_inside, sec_inside) {
                    (true, true) => {
                        let ref_value = reference.get(x - ref_left, y - ref_top, band);
                        let sec_value = secondary.get(x - sec_left, y - sec_top, band);
                        let weight = overlap_weight(
                            direction,
                            x,
                            y,
                            overlap_left,
                            overlap_top,
                            overlap_width,
                            overlap_height,
                        );
                        ref_value * (1.0 - weight) + sec_value * weight
                    }
                    (true, false) => reference.get(x - ref_left, y - ref_top, band),
                    (false, true) => secondary.get(x - sec_left, y - sec_top, band),
                    (false, false) => 0.0,
                };
                out.set(x, y, band, value);
            }
        }
    }

    out
}

unsafe fn op_mosaic(object: *mut VipsObject) -> Result<(), ()> {
    let reference = unsafe { get_image_buffer(object, "ref")? };
    let secondary = unsafe { get_image_buffer(object, "sec")? };
    let like = unsafe { get_image_ref(object, "ref")? };
    let (reference, secondary) = align_for_mosaic(&reference, &secondary)?;
    let direction = unsafe { get_enum(object, "direction")? as VipsDirection };
    let dx = unsafe { get_int(object, "xref")? }.saturating_sub(unsafe { get_int(object, "xsec")? });
    let dy = unsafe { get_int(object, "yref")? }.saturating_sub(unsafe { get_int(object, "ysec")? });
    let blend = if unsafe { super::argument_assigned(object, "mblend")? } {
        unsafe { get_int(object, "mblend")? }
    } else {
        0
    };
    let out = compose(&reference, &secondary, dx, dy, direction, blend);
    let result = unsafe { set_output_image_like(object, "out", out, like) };
    unsafe {
        object_unref(like);
    }
    result
}

fn tiepoint_delta(
    xr1: i32,
    yr1: i32,
    xs1: i32,
    ys1: i32,
    xr2: i32,
    yr2: i32,
    xs2: i32,
    ys2: i32,
) -> (i32, i32) {
    (
        ((xr1 - xs1) + (xr2 - xs2)) / 2,
        ((yr1 - ys1) + (yr2 - ys2)) / 2,
    )
}

unsafe fn op_mosaic1(object: *mut VipsObject) -> Result<(), ()> {
    let reference = unsafe { get_image_buffer(object, "ref")? };
    let secondary = unsafe { get_image_buffer(object, "sec")? };
    let like = unsafe { get_image_ref(object, "ref")? };
    let (reference, secondary) = align_for_mosaic(&reference, &secondary)?;
    let direction = unsafe { get_enum(object, "direction")? as VipsDirection };
    let (dx, dy) = tiepoint_delta(
        unsafe { get_int(object, "xr1")? },
        unsafe { get_int(object, "yr1")? },
        unsafe { get_int(object, "xs1")? },
        unsafe { get_int(object, "ys1")? },
        unsafe { get_int(object, "xr2")? },
        unsafe { get_int(object, "yr2")? },
        unsafe { get_int(object, "xs2")? },
        unsafe { get_int(object, "ys2")? },
    );
    let blend = if unsafe { super::argument_assigned(object, "mblend")? } {
        unsafe { get_int(object, "mblend")? }
    } else {
        0
    };
    let out = compose(&reference, &secondary, dx, dy, direction, blend);
    let result = unsafe { set_output_image_like(object, "out", out, like) };
    unsafe {
        object_unref(like);
    }
    result
}

unsafe fn op_match(object: *mut VipsObject) -> Result<(), ()> {
    let reference = unsafe { get_image_buffer(object, "ref")? };
    let secondary = unsafe { get_image_buffer(object, "sec")? };
    let like = unsafe { get_image_ref(object, "ref")? };
    let (reference, secondary) = align_for_mosaic(&reference, &secondary)?;
    let (dx, dy) = tiepoint_delta(
        unsafe { get_int(object, "xr1")? },
        unsafe { get_int(object, "yr1")? },
        unsafe { get_int(object, "xs1")? },
        unsafe { get_int(object, "ys1")? },
        unsafe { get_int(object, "xr2")? },
        unsafe { get_int(object, "yr2")? },
        unsafe { get_int(object, "xs2")? },
        unsafe { get_int(object, "ys2")? },
    );
    let direction = if dx.abs() >= dy.abs() {
        VIPS_DIRECTION_HORIZONTAL
    } else {
        VIPS_DIRECTION_VERTICAL
    };
    let out = compose(&reference, &secondary, dx, dy, direction, 0);
    let result = unsafe { set_output_image_like(object, "out", out, like) };
    unsafe {
        object_unref(like);
    }
    result
}

pub(crate) unsafe fn dispatch(object: *mut VipsObject, nickname: &str) -> Result<bool, ()> {
    match nickname {
        "mosaic" => {
            unsafe { op_mosaic(object)? };
            Ok(true)
        }
        "mosaic1" => {
            unsafe { op_mosaic1(object)? };
            Ok(true)
        }
        "match" => {
            unsafe { op_match(object)? };
            Ok(true)
        }
        _ => Ok(false),
    }
}
