use crate::abi::basic::{VipsPrecision, VIPS_PRECISION_FLOAT, VIPS_PRECISION_INTEGER};
use crate::abi::image::{
    VipsInterpretation, VIPS_FORMAT_DOUBLE, VIPS_FORMAT_FLOAT, VIPS_FORMAT_UCHAR,
    VIPS_FORMAT_UINT, VIPS_FORMAT_USHORT, VIPS_INTERPRETATION_FOURIER,
    VIPS_INTERPRETATION_HISTOGRAM, VIPS_INTERPRETATION_MULTIBAND, VIPS_INTERPRETATION_XYZ,
};
use crate::abi::object::VipsObject;
use crate::pixels::kernel::{gaussian_kernel, log_kernel};
use crate::pixels::ImageBuffer;

use super::{argument_assigned, get_bool, get_double, get_enum, get_int, set_output_image};

fn blank(width: usize, height: usize, bands: usize) -> ImageBuffer {
    ImageBuffer::new(
        width,
        height,
        bands,
        VIPS_FORMAT_UCHAR,
        crate::abi::image::VIPS_CODING_NONE,
        if bands == 1 {
            crate::abi::image::VIPS_INTERPRETATION_B_W
        } else {
            VIPS_INTERPRETATION_MULTIBAND
        },
    )
}

fn point_to_uchar(value: f64) -> f64 {
    (value * 255.0).clamp(0.0, 255.0)
}

unsafe fn op_black(object: *mut VipsObject) -> Result<(), ()> {
    let width = usize::try_from(unsafe { get_int(object, "width")? }).map_err(|_| ())?;
    let height = usize::try_from(unsafe { get_int(object, "height")? }).map_err(|_| ())?;
    let bands = if unsafe { argument_assigned(object, "bands")? } {
        usize::try_from(unsafe { get_int(object, "bands")? }).map_err(|_| ())?
    } else {
        1
    };
    let out = blank(width, height, bands).to_image();
    unsafe { set_output_image(object, "out", out) }
}

unsafe fn op_grey(object: *mut VipsObject) -> Result<(), ()> {
    let width = usize::try_from(unsafe { get_int(object, "width")? }).map_err(|_| ())?;
    let height = usize::try_from(unsafe { get_int(object, "height")? }).map_err(|_| ())?;
    let uchar = unsafe { argument_assigned(object, "uchar")? } && unsafe { get_bool(object, "uchar")? };
    let mut out = ImageBuffer::new(
        width,
        height,
        1,
        if uchar { VIPS_FORMAT_UCHAR } else { VIPS_FORMAT_FLOAT },
        crate::abi::image::VIPS_CODING_NONE,
        crate::abi::image::VIPS_INTERPRETATION_B_W,
    );
    let denom = width.saturating_sub(1).max(1) as f64;
    for y in 0..height {
        for x in 0..width {
            let value = x as f64 / denom;
            out.set(x, y, 0, if uchar { point_to_uchar(value) } else { value });
        }
    }
    unsafe { set_output_image(object, "out", out.to_image()) }
}

unsafe fn op_xyz(object: *mut VipsObject) -> Result<(), ()> {
    let width = usize::try_from(unsafe { get_int(object, "width")? }).map_err(|_| ())?;
    let height = usize::try_from(unsafe { get_int(object, "height")? }).map_err(|_| ())?;
    let csize = if unsafe { argument_assigned(object, "csize")? } {
        usize::try_from(unsafe { get_int(object, "csize")? }).map_err(|_| ())?
    } else {
        1
    };
    let dsize = if unsafe { argument_assigned(object, "dsize")? } {
        usize::try_from(unsafe { get_int(object, "dsize")? }).map_err(|_| ())?
    } else {
        1
    };
    let esize = if unsafe { argument_assigned(object, "esize")? } {
        usize::try_from(unsafe { get_int(object, "esize")? }).map_err(|_| ())?
    } else {
        1
    };
    let dims = if esize > 1 {
        5
    } else if dsize > 1 {
        4
    } else if csize > 1 {
        3
    } else {
        2
    };
    let out_height = height
        .checked_mul(csize)
        .and_then(|v| v.checked_mul(dsize))
        .and_then(|v| v.checked_mul(esize))
        .ok_or(())?;
    let mut out = ImageBuffer::new(
        width,
        out_height,
        dims,
        VIPS_FORMAT_UINT,
        crate::abi::image::VIPS_CODING_NONE,
        VIPS_INTERPRETATION_XYZ,
    );
    for y in 0..out_height {
        let h4 = height * csize * dsize;
        let dim4 = y / h4.max(1);
        let r4 = y % h4.max(1);
        let h3 = (height * csize).max(1);
        let dim3 = r4 / h3;
        let r3 = r4 % h3;
        let dim2 = r3 / height.max(1);
        let dim1 = r3 % height.max(1);
        for x in 0..width {
            let coords = [x as f64, dim1 as f64, dim2 as f64, dim3 as f64, dim4 as f64];
            for band in 0..dims {
                out.set(x, y, band, coords[band]);
            }
        }
    }
    unsafe { set_output_image(object, "out", out.to_image()) }
}

unsafe fn op_identity(object: *mut VipsObject) -> Result<(), ()> {
    let bands = if unsafe { argument_assigned(object, "bands")? } {
        usize::try_from(unsafe { get_int(object, "bands")? }).map_err(|_| ())?
    } else {
        1
    };
    let ushort = unsafe { argument_assigned(object, "ushort")? } && unsafe { get_bool(object, "ushort")? };
    let size = if ushort && unsafe { argument_assigned(object, "size")? } {
        usize::try_from(unsafe { get_int(object, "size")? }).map_err(|_| ())?
    } else if ushort {
        65536
    } else {
        256
    };
    let mut out = ImageBuffer::new(
        size,
        1,
        bands,
        if ushort { VIPS_FORMAT_USHORT } else { VIPS_FORMAT_UCHAR },
        crate::abi::image::VIPS_CODING_NONE,
        VIPS_INTERPRETATION_HISTOGRAM,
    );
    for x in 0..size {
        for band in 0..bands {
            out.set(x, 0, band, x as f64);
        }
    }
    unsafe { set_output_image(object, "out", out.to_image()) }
}

unsafe fn op_eye(object: *mut VipsObject) -> Result<(), ()> {
    let width = usize::try_from(unsafe { get_int(object, "width")? }).map_err(|_| ())?;
    let height = usize::try_from(unsafe { get_int(object, "height")? }).map_err(|_| ())?;
    let factor = if unsafe { argument_assigned(object, "factor")? } {
        unsafe { get_double(object, "factor")? }
    } else {
        0.5
    };
    let uchar = unsafe { argument_assigned(object, "uchar")? } && unsafe { get_bool(object, "uchar")? };
    let mut out = ImageBuffer::new(
        width,
        height,
        1,
        if uchar { VIPS_FORMAT_UCHAR } else { VIPS_FORMAT_FLOAT },
        crate::abi::image::VIPS_CODING_NONE,
        crate::abi::image::VIPS_INTERPRETATION_B_W,
    );
    let max_x = width.saturating_sub(1).max(1) as f64;
    let max_y = height.saturating_sub(1).max(1) as f64;
    let c = factor * std::f64::consts::PI / (2.0 * max_x);
    let h = max_y * max_y;
    for y in 0..height {
        for x in 0..width {
            let value = (y as f64 * y as f64) * (c * x as f64 * x as f64).cos() / h;
            out.set(x, y, 0, if uchar { point_to_uchar(value) } else { value });
        }
    }
    unsafe { set_output_image(object, "out", out.to_image()) }
}

unsafe fn op_matrix_kernel(
    object: *mut VipsObject,
    build: impl FnOnce(f64, f64, bool, VipsPrecision) -> Result<crate::pixels::kernel::Kernel, ()>,
) -> Result<(), ()> {
    let sigma = unsafe { get_double(object, "sigma")? };
    let min_ampl = unsafe { get_double(object, "min_ampl")? };
    let separable = unsafe { argument_assigned(object, "separable")? } && unsafe { get_bool(object, "separable")? };
    let precision = if unsafe { argument_assigned(object, "precision")? } {
        unsafe { get_enum(object, "precision")? as VipsPrecision }
    } else if unsafe { argument_assigned(object, "integer")? } && unsafe { get_bool(object, "integer")? } {
        VIPS_PRECISION_INTEGER
    } else {
        VIPS_PRECISION_FLOAT
    };
    let kernel = build(sigma, min_ampl, separable, precision)?;
    unsafe { set_output_image(object, "out", kernel.to_image()) }
}

fn mask_base(width: usize, height: usize, x: usize, y: usize, optical: bool) -> (f64, f64, bool) {
    let half_width = (width / 2).max(1);
    let half_height = (height / 2).max(1);
    let mut xx = x as isize;
    let mut yy = y as isize;
    if !optical {
        xx = (xx + half_width as isize) % width.max(1) as isize;
        yy = (yy + half_height as isize) % height.max(1) as isize;
    }
    xx -= half_width as isize;
    yy -= half_height as isize;
    let is_dc = xx == 0 && yy == 0;
    (
        xx as f64 / half_width as f64,
        yy as f64 / half_height as f64,
        is_dc,
    )
}

unsafe fn op_mask_ideal(object: *mut VipsObject) -> Result<(), ()> {
    let width = usize::try_from(unsafe { get_int(object, "width")? }).map_err(|_| ())?;
    let height = usize::try_from(unsafe { get_int(object, "height")? }).map_err(|_| ())?;
    let fc = unsafe { get_double(object, "frequency_cutoff")? };
    let optical = unsafe { argument_assigned(object, "optical")? } && unsafe { get_bool(object, "optical")? };
    let reject = unsafe { argument_assigned(object, "reject")? } && unsafe { get_bool(object, "reject")? };
    let nodc = unsafe { argument_assigned(object, "nodc")? } && unsafe { get_bool(object, "nodc")? };
    let uchar = unsafe { argument_assigned(object, "uchar")? } && unsafe { get_bool(object, "uchar")? };
    let mut out = ImageBuffer::new(
        width,
        height,
        1,
        if uchar { VIPS_FORMAT_UCHAR } else { VIPS_FORMAT_FLOAT },
        crate::abi::image::VIPS_CODING_NONE,
        VIPS_INTERPRETATION_FOURIER,
    );
    let fc2 = fc * fc;
    for y in 0..height {
        for x in 0..width {
            let (dx, dy, is_dc) = mask_base(width, height, x, y, optical);
            let mut value = if !nodc && is_dc {
                1.0
            } else if dx * dx + dy * dy <= fc2 {
                0.0
            } else {
                1.0
            };
            if reject {
                value = 1.0 - value;
            }
            out.set(x, y, 0, if uchar { point_to_uchar(value) } else { value });
        }
    }
    unsafe { set_output_image(object, "out", out.to_image()) }
}

unsafe fn op_mask_gaussian(object: *mut VipsObject) -> Result<(), ()> {
    let width = usize::try_from(unsafe { get_int(object, "width")? }).map_err(|_| ())?;
    let height = usize::try_from(unsafe { get_int(object, "height")? }).map_err(|_| ())?;
    let fc = unsafe { get_double(object, "frequency_cutoff")? };
    let ac = unsafe { get_double(object, "amplitude_cutoff")? };
    let optical = unsafe { argument_assigned(object, "optical")? } && unsafe { get_bool(object, "optical")? };
    let reject = unsafe { argument_assigned(object, "reject")? } && unsafe { get_bool(object, "reject")? };
    let nodc = unsafe { argument_assigned(object, "nodc")? } && unsafe { get_bool(object, "nodc")? };
    let uchar = unsafe { argument_assigned(object, "uchar")? } && unsafe { get_bool(object, "uchar")? };
    let mut out = ImageBuffer::new(
        width,
        height,
        1,
        if uchar { VIPS_FORMAT_UCHAR } else { VIPS_FORMAT_FLOAT },
        crate::abi::image::VIPS_CODING_NONE,
        VIPS_INTERPRETATION_FOURIER,
    );
    let fc2 = (fc * fc).max(f64::MIN_POSITIVE);
    let cnst = ac.max(f64::MIN_POSITIVE).ln();
    for y in 0..height {
        for x in 0..width {
            let (dx, dy, is_dc) = mask_base(width, height, x, y, optical);
            let mut value = if !nodc && is_dc {
                1.0
            } else {
                1.0 - (cnst * ((dx * dx + dy * dy) / fc2)).exp()
            };
            if reject {
                value = 1.0 - value;
            }
            out.set(x, y, 0, if uchar { point_to_uchar(value) } else { value });
        }
    }
    unsafe { set_output_image(object, "out", out.to_image()) }
}

pub(crate) unsafe fn dispatch(object: *mut VipsObject, nickname: &str) -> Result<bool, ()> {
    match nickname {
        "black" => {
            unsafe { op_black(object)? };
            Ok(true)
        }
        "grey" => {
            unsafe { op_grey(object)? };
            Ok(true)
        }
        "xyz" => {
            unsafe { op_xyz(object)? };
            Ok(true)
        }
        "identity" => {
            unsafe { op_identity(object)? };
            Ok(true)
        }
        "eye" => {
            unsafe { op_eye(object)? };
            Ok(true)
        }
        "gaussmat" => {
            unsafe { op_matrix_kernel(object, gaussian_kernel)? };
            Ok(true)
        }
        "logmat" => {
            unsafe { op_matrix_kernel(object, log_kernel)? };
            Ok(true)
        }
        "mask_ideal" => {
            unsafe { op_mask_ideal(object)? };
            Ok(true)
        }
        "mask_gaussian" => {
            unsafe { op_mask_gaussian(object)? };
            Ok(true)
        }
        _ => Ok(false),
    }
}
