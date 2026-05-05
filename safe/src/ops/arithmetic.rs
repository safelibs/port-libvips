use crate::abi::basic::{
    VipsOperationBoolean, VipsOperationComplex, VipsOperationComplexget, VipsOperationMath,
    VipsOperationMath2, VipsOperationRelational, VipsOperationRound, VIPS_OPERATION_BOOLEAN_AND,
    VIPS_OPERATION_BOOLEAN_EOR, VIPS_OPERATION_BOOLEAN_LSHIFT, VIPS_OPERATION_BOOLEAN_OR,
    VIPS_OPERATION_BOOLEAN_RSHIFT, VIPS_OPERATION_COMPLEXGET_IMAG, VIPS_OPERATION_COMPLEXGET_REAL,
    VIPS_OPERATION_COMPLEX_CONJ, VIPS_OPERATION_COMPLEX_POLAR, VIPS_OPERATION_COMPLEX_RECT,
    VIPS_OPERATION_MATH2_ATAN2, VIPS_OPERATION_MATH2_POW, VIPS_OPERATION_MATH2_WOP,
    VIPS_OPERATION_MATH_ACOS, VIPS_OPERATION_MATH_ACOSH, VIPS_OPERATION_MATH_ASIN,
    VIPS_OPERATION_MATH_ASINH, VIPS_OPERATION_MATH_ATAN, VIPS_OPERATION_MATH_ATANH,
    VIPS_OPERATION_MATH_COS, VIPS_OPERATION_MATH_COSH, VIPS_OPERATION_MATH_EXP,
    VIPS_OPERATION_MATH_EXP10, VIPS_OPERATION_MATH_LOG, VIPS_OPERATION_MATH_LOG10,
    VIPS_OPERATION_MATH_SIN, VIPS_OPERATION_MATH_SINH, VIPS_OPERATION_MATH_TAN,
    VIPS_OPERATION_MATH_TANH, VIPS_OPERATION_RELATIONAL_EQUAL, VIPS_OPERATION_RELATIONAL_LESS,
    VIPS_OPERATION_RELATIONAL_LESSEQ, VIPS_OPERATION_RELATIONAL_MORE,
    VIPS_OPERATION_RELATIONAL_MOREEQ, VIPS_OPERATION_RELATIONAL_NOTEQ, VIPS_OPERATION_ROUND_CEIL,
    VIPS_OPERATION_ROUND_FLOOR, VIPS_OPERATION_ROUND_RINT,
};
use crate::abi::image::{
    VipsBandFormat, VIPS_CODING_NONE, VIPS_FORMAT_CHAR, VIPS_FORMAT_COMPLEX, VIPS_FORMAT_DOUBLE,
    VIPS_FORMAT_DPCOMPLEX, VIPS_FORMAT_FLOAT, VIPS_FORMAT_INT, VIPS_FORMAT_SHORT,
    VIPS_FORMAT_UCHAR, VIPS_FORMAT_UINT, VIPS_FORMAT_USHORT, VIPS_INTERPRETATION_MATRIX,
};
use crate::abi::object::VipsObject;
use crate::pixels::format::{
    clamp_for_format, common_format, complex_component_format, complex_promoted_format,
    format_bytes, format_components, format_kind, read_sample, NumericKind,
};
use crate::pixels::{
    complex_from_buffer, complex_image_from_samples, read_complex_image, ComplexSample,
    ImageBuffer, ImageSpec,
};
use crate::runtime::image::{ensure_pixels, image_state};

use super::{
    get_array_double, get_array_images, get_enum, get_image_buffer, get_image_ref, get_int,
    set_output_array_double, set_output_double, set_output_image, set_output_image_like,
    set_output_int,
};

fn format_index(format: VipsBandFormat) -> Option<usize> {
    match format {
        VIPS_FORMAT_UCHAR => Some(0),
        VIPS_FORMAT_CHAR => Some(1),
        VIPS_FORMAT_USHORT => Some(2),
        VIPS_FORMAT_SHORT => Some(3),
        VIPS_FORMAT_UINT => Some(4),
        VIPS_FORMAT_INT => Some(5),
        VIPS_FORMAT_FLOAT => Some(6),
        VIPS_FORMAT_DOUBLE => Some(8),
        _ => None,
    }
}

fn table_format(format: VipsBandFormat, table: &[VipsBandFormat; 9]) -> Result<VipsBandFormat, ()> {
    Ok(table[format_index(format).ok_or(())?])
}

pub(crate) fn binary_output_format(op: &str, format: VipsBandFormat) -> Result<VipsBandFormat, ()> {
    let table = match op {
        "add" => [
            VIPS_FORMAT_USHORT,
            VIPS_FORMAT_SHORT,
            VIPS_FORMAT_UINT,
            VIPS_FORMAT_INT,
            VIPS_FORMAT_UINT,
            VIPS_FORMAT_INT,
            VIPS_FORMAT_FLOAT,
            VIPS_FORMAT_FLOAT,
            VIPS_FORMAT_DOUBLE,
        ],
        "subtract" => [
            VIPS_FORMAT_SHORT,
            VIPS_FORMAT_SHORT,
            VIPS_FORMAT_INT,
            VIPS_FORMAT_INT,
            VIPS_FORMAT_INT,
            VIPS_FORMAT_INT,
            VIPS_FORMAT_FLOAT,
            VIPS_FORMAT_FLOAT,
            VIPS_FORMAT_DOUBLE,
        ],
        "multiply" => [
            VIPS_FORMAT_USHORT,
            VIPS_FORMAT_SHORT,
            VIPS_FORMAT_UINT,
            VIPS_FORMAT_INT,
            VIPS_FORMAT_UINT,
            VIPS_FORMAT_INT,
            VIPS_FORMAT_FLOAT,
            VIPS_FORMAT_FLOAT,
            VIPS_FORMAT_DOUBLE,
        ],
        "divide" | "math2" | "linear" => [
            VIPS_FORMAT_FLOAT,
            VIPS_FORMAT_FLOAT,
            VIPS_FORMAT_FLOAT,
            VIPS_FORMAT_FLOAT,
            VIPS_FORMAT_FLOAT,
            VIPS_FORMAT_FLOAT,
            VIPS_FORMAT_FLOAT,
            VIPS_FORMAT_FLOAT,
            VIPS_FORMAT_DOUBLE,
        ],
        "remainder" | "invert" | "round" => [
            VIPS_FORMAT_UCHAR,
            VIPS_FORMAT_CHAR,
            VIPS_FORMAT_USHORT,
            VIPS_FORMAT_SHORT,
            VIPS_FORMAT_UINT,
            VIPS_FORMAT_INT,
            VIPS_FORMAT_FLOAT,
            VIPS_FORMAT_FLOAT,
            VIPS_FORMAT_DOUBLE,
        ],
        "boolean" => [
            VIPS_FORMAT_UCHAR,
            VIPS_FORMAT_CHAR,
            VIPS_FORMAT_USHORT,
            VIPS_FORMAT_SHORT,
            VIPS_FORMAT_UINT,
            VIPS_FORMAT_INT,
            VIPS_FORMAT_INT,
            VIPS_FORMAT_INT,
            VIPS_FORMAT_INT,
        ],
        "relational" => [VIPS_FORMAT_UCHAR; 9],
        "sign" => [
            VIPS_FORMAT_CHAR,
            VIPS_FORMAT_CHAR,
            VIPS_FORMAT_CHAR,
            VIPS_FORMAT_CHAR,
            VIPS_FORMAT_CHAR,
            VIPS_FORMAT_CHAR,
            VIPS_FORMAT_CHAR,
            VIPS_FORMAT_CHAR,
            VIPS_FORMAT_CHAR,
        ],
        "abs" => [
            VIPS_FORMAT_UCHAR,
            VIPS_FORMAT_CHAR,
            VIPS_FORMAT_USHORT,
            VIPS_FORMAT_SHORT,
            VIPS_FORMAT_UINT,
            VIPS_FORMAT_INT,
            VIPS_FORMAT_FLOAT,
            VIPS_FORMAT_FLOAT,
            VIPS_FORMAT_DOUBLE,
        ],
        "sum" => [
            VIPS_FORMAT_UINT,
            VIPS_FORMAT_INT,
            VIPS_FORMAT_UINT,
            VIPS_FORMAT_INT,
            VIPS_FORMAT_UINT,
            VIPS_FORMAT_INT,
            VIPS_FORMAT_FLOAT,
            VIPS_FORMAT_FLOAT,
            VIPS_FORMAT_DOUBLE,
        ],
        "math" => [
            VIPS_FORMAT_FLOAT,
            VIPS_FORMAT_FLOAT,
            VIPS_FORMAT_FLOAT,
            VIPS_FORMAT_FLOAT,
            VIPS_FORMAT_FLOAT,
            VIPS_FORMAT_FLOAT,
            VIPS_FORMAT_FLOAT,
            VIPS_FORMAT_FLOAT,
            VIPS_FORMAT_DOUBLE,
        ],
        _ => return Err(()),
    };
    table_format(format, &table)
}

fn project_output_format(format: VipsBandFormat) -> Result<VipsBandFormat, ()> {
    Ok(match format {
        VIPS_FORMAT_UCHAR | VIPS_FORMAT_USHORT | VIPS_FORMAT_UINT => VIPS_FORMAT_UINT,
        VIPS_FORMAT_CHAR | VIPS_FORMAT_SHORT | VIPS_FORMAT_INT => VIPS_FORMAT_INT,
        VIPS_FORMAT_FLOAT | VIPS_FORMAT_DOUBLE => VIPS_FORMAT_DOUBLE,
        _ => return Err(()),
    })
}

fn replicate_if_needed(buffer: &ImageBuffer, bands: usize) -> Result<ImageBuffer, ()> {
    if buffer.spec.bands == bands {
        Ok(buffer.clone())
    } else {
        buffer.replicate_bands(bands)
    }
}

fn expand_const_operands(
    input: &ImageBuffer,
    constants: &[f64],
) -> Result<(ImageBuffer, Vec<f64>), ()> {
    if constants.is_empty() {
        return Err(());
    }
    if constants.len() != 1 && constants.len() != input.spec.bands && input.spec.bands != 1 {
        return Err(());
    }

    let target_bands = input.spec.bands.max(constants.len());
    let input = replicate_if_needed(input, target_bands)?;
    let constants = if constants.len() == 1 {
        vec![constants[0]; target_bands]
    } else {
        constants.to_vec()
    };

    Ok((input, constants))
}

fn align_pair(
    left: &ImageBuffer,
    right: &ImageBuffer,
) -> Result<(ImageBuffer, ImageBuffer, VipsBandFormat), ()> {
    let format = common_format(left.spec.format, right.spec.format).ok_or(())?;
    let width = left.spec.width.max(right.spec.width);
    let height = left.spec.height.max(right.spec.height);
    let bands = match (left.spec.bands, right.spec.bands) {
        (a, b) if a == b => a,
        (1, b) => b,
        (a, 1) => a,
        _ => return Err(()),
    };

    let left = replicate_if_needed(left, bands)?
        .with_format(format)
        .zero_extend(width, height);
    let right = replicate_if_needed(right, bands)?
        .with_format(format)
        .zero_extend(width, height);
    Ok((left, right, format))
}

fn new_output_from_spec(spec: ImageSpec, format: VipsBandFormat) -> ImageBuffer {
    let mut out = ImageBuffer::new(
        spec.width,
        spec.height,
        spec.bands,
        format,
        spec.coding,
        spec.interpretation,
    );
    out.spec.xres = spec.xres;
    out.spec.yres = spec.yres;
    out.spec.xoffset = spec.xoffset;
    out.spec.yoffset = spec.yoffset;
    out.spec.dhint = spec.dhint;
    out
}

fn complexget_output_format(format: VipsBandFormat) -> VipsBandFormat {
    match format {
        VIPS_FORMAT_COMPLEX => VIPS_FORMAT_FLOAT,
        VIPS_FORMAT_DPCOMPLEX => VIPS_FORMAT_DOUBLE,
        _ => format,
    }
}

fn int_binary_u8(left: f64, right: f64, op: VipsOperationBoolean) -> f64 {
    let left_u8 = left as u8;
    let left = left_u8 as i64;
    let right = right.trunc() as i64;
    let shift = (right as u32) & 7;
    let result = match op {
        VIPS_OPERATION_BOOLEAN_AND => left & right,
        VIPS_OPERATION_BOOLEAN_OR => left | right,
        VIPS_OPERATION_BOOLEAN_EOR => left ^ right,
        VIPS_OPERATION_BOOLEAN_LSHIFT => left_u8.wrapping_shl(shift) as i64,
        VIPS_OPERATION_BOOLEAN_RSHIFT => (left_u8 >> shift) as i64,
        _ => 0,
    };
    (result as u8) as f64
}

fn int_binary_i8(left: f64, right: f64, op: VipsOperationBoolean) -> f64 {
    let left_i8 = left as i8;
    let left = left_i8 as i64;
    let right = right.trunc() as i64;
    let shift = (right as u32) & 7;
    let result = match op {
        VIPS_OPERATION_BOOLEAN_AND => left & right,
        VIPS_OPERATION_BOOLEAN_OR => left | right,
        VIPS_OPERATION_BOOLEAN_EOR => left ^ right,
        VIPS_OPERATION_BOOLEAN_LSHIFT => left_i8.wrapping_shl(shift) as i64,
        VIPS_OPERATION_BOOLEAN_RSHIFT => (left_i8 >> shift) as i64,
        _ => 0,
    };
    (result as i8) as f64
}

fn int_binary_u16(left: f64, right: f64, op: VipsOperationBoolean) -> f64 {
    let left_u16 = left as u16;
    let left = left_u16 as i64;
    let right = right.trunc() as i64;
    let shift = (right as u32) & 15;
    let result = match op {
        VIPS_OPERATION_BOOLEAN_AND => left & right,
        VIPS_OPERATION_BOOLEAN_OR => left | right,
        VIPS_OPERATION_BOOLEAN_EOR => left ^ right,
        VIPS_OPERATION_BOOLEAN_LSHIFT => left_u16.wrapping_shl(shift) as i64,
        VIPS_OPERATION_BOOLEAN_RSHIFT => (left_u16 >> shift) as i64,
        _ => 0,
    };
    (result as u16) as f64
}

fn int_binary_i16(left: f64, right: f64, op: VipsOperationBoolean) -> f64 {
    let left_i16 = left as i16;
    let left = left_i16 as i64;
    let right = right.trunc() as i64;
    let shift = (right as u32) & 15;
    let result = match op {
        VIPS_OPERATION_BOOLEAN_AND => left & right,
        VIPS_OPERATION_BOOLEAN_OR => left | right,
        VIPS_OPERATION_BOOLEAN_EOR => left ^ right,
        VIPS_OPERATION_BOOLEAN_LSHIFT => left_i16.wrapping_shl(shift) as i64,
        VIPS_OPERATION_BOOLEAN_RSHIFT => (left_i16 >> shift) as i64,
        _ => 0,
    };
    (result as i16) as f64
}

fn int_binary_u32(left: f64, right: f64, op: VipsOperationBoolean) -> f64 {
    let left = left as u32;
    let right = (right.trunc() as i64) as u32;
    let shift = right & 31;
    match op {
        VIPS_OPERATION_BOOLEAN_AND => (left & right) as f64,
        VIPS_OPERATION_BOOLEAN_OR => (left | right) as f64,
        VIPS_OPERATION_BOOLEAN_EOR => (left ^ right) as f64,
        VIPS_OPERATION_BOOLEAN_LSHIFT => left.wrapping_shl(shift) as f64,
        VIPS_OPERATION_BOOLEAN_RSHIFT => (left >> shift) as f64,
        _ => 0.0,
    }
}

fn int_binary_i32(left: f64, right: f64, op: VipsOperationBoolean) -> f64 {
    let shift = (right as u32) & 31;
    match op {
        VIPS_OPERATION_BOOLEAN_AND => (((left as i32) & (right as i32)) as i32) as f64,
        VIPS_OPERATION_BOOLEAN_OR => (((left as i32) | (right as i32)) as i32) as f64,
        VIPS_OPERATION_BOOLEAN_EOR => (((left as i32) ^ (right as i32)) as i32) as f64,
        VIPS_OPERATION_BOOLEAN_LSHIFT => (left as i32).wrapping_shl(shift) as f64,
        VIPS_OPERATION_BOOLEAN_RSHIFT => ((left as i32) >> shift) as f64,
        _ => 0.0,
    }
}

pub(crate) fn boolean_value(
    format: VipsBandFormat,
    left: f64,
    right: f64,
    op: VipsOperationBoolean,
) -> f64 {
    match format {
        VIPS_FORMAT_UCHAR => int_binary_u8(left, right, op),
        VIPS_FORMAT_CHAR => int_binary_i8(left, right, op),
        VIPS_FORMAT_USHORT => int_binary_u16(left, right, op),
        VIPS_FORMAT_SHORT => int_binary_i16(left, right, op),
        VIPS_FORMAT_UINT => int_binary_u32(left, right, op),
        VIPS_FORMAT_INT => int_binary_i32(left, right, op),
        VIPS_FORMAT_FLOAT | VIPS_FORMAT_DOUBLE => int_binary_i32(left, right, op),
        _ => 0.0,
    }
}

fn unary_round(value: f64, round: VipsOperationRound) -> f64 {
    match round {
        VIPS_OPERATION_ROUND_RINT => value.round_ties_even(),
        VIPS_OPERATION_ROUND_CEIL => value.ceil(),
        VIPS_OPERATION_ROUND_FLOOR => value.floor(),
        _ => value,
    }
}

fn unary_math(value: f64, math: VipsOperationMath) -> f64 {
    let to_radians = std::f64::consts::PI / 180.0;
    let to_degrees = 180.0 / std::f64::consts::PI;
    match math {
        VIPS_OPERATION_MATH_SIN => (value * to_radians).sin(),
        VIPS_OPERATION_MATH_COS => (value * to_radians).cos(),
        VIPS_OPERATION_MATH_TAN => (value * to_radians).tan(),
        VIPS_OPERATION_MATH_ASIN => value.asin() * to_degrees,
        VIPS_OPERATION_MATH_ACOS => value.acos() * to_degrees,
        VIPS_OPERATION_MATH_ATAN => value.atan() * to_degrees,
        VIPS_OPERATION_MATH_LOG => {
            if value == 0.0 {
                0.0
            } else {
                value.ln()
            }
        }
        VIPS_OPERATION_MATH_LOG10 => {
            if value == 0.0 {
                0.0
            } else {
                value.log10()
            }
        }
        VIPS_OPERATION_MATH_EXP => value.exp(),
        VIPS_OPERATION_MATH_EXP10 => 10f64.powf(value),
        VIPS_OPERATION_MATH_SINH => value.sinh(),
        VIPS_OPERATION_MATH_COSH => value.cosh(),
        VIPS_OPERATION_MATH_TANH => value.tanh(),
        VIPS_OPERATION_MATH_ASINH => value.asinh(),
        VIPS_OPERATION_MATH_ACOSH => value.acosh(),
        VIPS_OPERATION_MATH_ATANH => value.atanh(),
        _ => value,
    }
}

fn binary_math2(left: f64, right: f64, math2: VipsOperationMath2) -> f64 {
    match math2 {
        VIPS_OPERATION_MATH2_POW => {
            if left == 0.0 {
                0.0
            } else if right == -1.0 {
                1.0 / left
            } else if right == 0.5 {
                left.sqrt()
            } else {
                left.powf(right)
            }
        }
        VIPS_OPERATION_MATH2_WOP => {
            if right == 0.0 {
                0.0
            } else if left == -1.0 {
                1.0 / right
            } else if left == 0.5 {
                right.sqrt()
            } else {
                right.powf(left)
            }
        }
        VIPS_OPERATION_MATH2_ATAN2 => {
            let mut angle = left.atan2(right).to_degrees();
            if angle < 0.0 {
                angle += 360.0;
            }
            angle
        }
        _ => left,
    }
}

fn relation_value(left: f64, right: f64, rel: VipsOperationRelational) -> f64 {
    let yes = 255.0;
    let no = 0.0;
    match rel {
        VIPS_OPERATION_RELATIONAL_EQUAL => {
            if left == right {
                yes
            } else {
                no
            }
        }
        VIPS_OPERATION_RELATIONAL_NOTEQ => {
            if left != right {
                yes
            } else {
                no
            }
        }
        VIPS_OPERATION_RELATIONAL_LESS => {
            if left < right {
                yes
            } else {
                no
            }
        }
        VIPS_OPERATION_RELATIONAL_LESSEQ => {
            if left <= right {
                yes
            } else {
                no
            }
        }
        VIPS_OPERATION_RELATIONAL_MORE => {
            if left > right {
                yes
            } else {
                no
            }
        }
        VIPS_OPERATION_RELATIONAL_MOREEQ => {
            if left >= right {
                yes
            } else {
                no
            }
        }
        _ => no,
    }
}

fn remainder_value(format: VipsBandFormat, left: f64, right: f64) -> f64 {
    if right == 0.0 {
        return -1.0;
    }
    match format_kind(format) {
        Some(NumericKind::Float) => left - right * (left / right).floor(),
        Some(NumericKind::Unsigned) | Some(NumericKind::Signed) => {
            if matches!(format_kind(format), Some(NumericKind::Unsigned)) {
                let left = left.trunc() as u64;
                let right = right.trunc() as u64;
                if right == 0 {
                    -1.0
                } else {
                    (left % right) as f64
                }
            } else {
                let left = left.trunc() as i64;
                let right = right.trunc() as i64;
                if right == 0 {
                    -1.0
                } else {
                    (left % right) as f64
                }
            }
        }
        _ => 0.0,
    }
}

fn invert_value(format: VipsBandFormat, value: f64) -> f64 {
    match format {
        VIPS_FORMAT_UCHAR => u8::MAX as f64 - value,
        VIPS_FORMAT_USHORT => u16::MAX as f64 - value,
        VIPS_FORMAT_UINT => u32::MAX as f64 - value,
        _ => -value,
    }
}

fn sign_value(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        0.0
    }
}

fn apply_unary(
    input: &ImageBuffer,
    out_format: VipsBandFormat,
    mut f: impl FnMut(f64) -> f64,
) -> ImageBuffer {
    let mut out = new_output_from_spec(input.spec, out_format);
    out.data = input.data.iter().copied().map(&mut f).collect();
    out
}

fn apply_binary(
    left: &ImageBuffer,
    right: &ImageBuffer,
    out_format: VipsBandFormat,
    mut f: impl FnMut(f64, f64) -> f64,
) -> Result<ImageBuffer, ()> {
    let (left, right, _) = align_pair(left, right)?;
    let mut out = new_output_from_spec(left.spec, out_format);
    out.data = left
        .data
        .iter()
        .zip(&right.data)
        .map(|(l, r)| f(*l, *r))
        .collect();
    Ok(out)
}

unsafe fn unary_image_op(
    object: *mut VipsObject,
    name: &str,
    out_format: impl Fn(VipsBandFormat) -> Result<VipsBandFormat, ()>,
    f: impl FnMut(f64) -> f64,
) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let out = apply_unary(&input, out_format(input.spec.format)?, f);
    let image = unsafe { get_image_ref(object, "in")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result.map_err(|_| {
        let _ = name;
    })
}

unsafe fn binary_image_op(
    object: *mut VipsObject,
    left_name: &str,
    right_name: &str,
    out_name: &str,
    op_name: &str,
    f: impl FnMut(f64, f64, VipsBandFormat) -> f64,
) -> Result<(), ()> {
    let left = unsafe { get_image_buffer(object, left_name)? };
    let right = unsafe { get_image_buffer(object, right_name)? };
    let (_, _, common) = align_pair(&left, &right)?;
    let out_format = binary_output_format(op_name, common)?;
    let mut f = f;
    let out = apply_binary(&left, &right, out_format, |l, r| f(l, r, common))?;
    let left_image = unsafe { get_image_ref(object, left_name)? };
    let result = unsafe { set_output_image_like(object, out_name, out, left_image) };
    unsafe {
        crate::runtime::object::object_unref(left_image);
    }
    result
}

unsafe fn unary_const_image_op(
    object: *mut VipsObject,
    op_name: &str,
    out_format: impl Fn(VipsBandFormat) -> Result<VipsBandFormat, ()>,
    f: impl FnMut(f64, f64, VipsBandFormat) -> f64,
) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let constants = unsafe { get_array_double(object, "c")? };
    let (input, constants) = expand_const_operands(&input, &constants)?;
    let mut f = f;
    let mut out = new_output_from_spec(input.spec, out_format(input.spec.format)?);
    out.data = input
        .data
        .iter()
        .copied()
        .enumerate()
        .map(|(index, value)| {
            let band = index % input.spec.bands;
            f(value, constants[band], input.spec.format)
        })
        .collect();
    let image = unsafe { get_image_ref(object, "in")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result.map_err(|_| {
        let _ = op_name;
    })
}

unsafe fn op_abs(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let out_format = binary_output_format("abs", input.spec.format)?;
    let out = apply_unary(&input, out_format, f64::abs);
    let image = unsafe { get_image_ref(object, "in")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

unsafe fn op_invert(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let format = binary_output_format("invert", input.spec.format)?;
    let out = apply_unary(&input, format, |value| {
        invert_value(input.spec.format, value)
    });
    let image = unsafe { get_image_ref(object, "in")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

unsafe fn op_sign(object: *mut VipsObject) -> Result<(), ()> {
    unsafe {
        unary_image_op(
            object,
            "sign",
            |format| binary_output_format("sign", format),
            sign_value,
        )
    }
}

unsafe fn op_round(object: *mut VipsObject) -> Result<(), ()> {
    let round = unsafe { get_enum(object, "round")? } as VipsOperationRound;
    let input = unsafe { get_image_buffer(object, "in")? };
    let format = binary_output_format("round", input.spec.format)?;
    let out = if matches!(
        input.spec.format,
        VIPS_FORMAT_UCHAR
            | VIPS_FORMAT_CHAR
            | VIPS_FORMAT_USHORT
            | VIPS_FORMAT_SHORT
            | VIPS_FORMAT_UINT
            | VIPS_FORMAT_INT
    ) {
        input.with_format(format)
    } else {
        apply_unary(&input, format, |value| unary_round(value, round))
    };
    let image = unsafe { get_image_ref(object, "in")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

fn trim_background(input: &ImageBuffer, background: Vec<f64>) -> Vec<f64> {
    if !background.is_empty() {
        return background;
    }
    if input.spec.width == 0 || input.spec.height == 0 || input.spec.bands == 0 {
        return vec![0.0];
    }
    (0..input.spec.bands)
        .map(|band| input.get(0, 0, band))
        .collect()
}

fn trim_pixel_differs(
    input: &ImageBuffer,
    x: usize,
    y: usize,
    background: &[f64],
    threshold: f64,
) -> bool {
    for band in 0..input.spec.bands {
        let bg = background[band.min(background.len().saturating_sub(1))];
        if (input.get(x, y, band) - bg).abs() > threshold {
            return true;
        }
    }
    false
}

unsafe fn op_find_trim(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let threshold = if unsafe { super::argument_assigned(object, "threshold")? } {
        unsafe { super::get_double(object, "threshold")? }
    } else {
        10.0
    };
    let background = if unsafe { super::argument_assigned(object, "background")? } {
        unsafe { get_array_double(object, "background")? }
    } else {
        Vec::new()
    };
    let background = trim_background(&input, background);

    let mut min_x = input.spec.width;
    let mut min_y = input.spec.height;
    let mut max_x = 0usize;
    let mut max_y = 0usize;
    let mut found = false;
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            if trim_pixel_differs(&input, x, y, &background, threshold) {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
                found = true;
            }
        }
    }

    if found {
        unsafe { set_output_int(object, "left", min_x as i32)? };
        unsafe { set_output_int(object, "top", min_y as i32)? };
        unsafe { set_output_int(object, "width", (max_x - min_x + 1) as i32)? };
        unsafe { set_output_int(object, "height", (max_y - min_y + 1) as i32)? };
    } else {
        unsafe { set_output_int(object, "left", 0)? };
        unsafe { set_output_int(object, "top", 0)? };
        unsafe { set_output_int(object, "width", 0)? };
        unsafe { set_output_int(object, "height", 0)? };
    }
    Ok(())
}

unsafe fn op_math(object: *mut VipsObject) -> Result<(), ()> {
    let math = unsafe { get_enum(object, "math")? } as VipsOperationMath;
    unsafe {
        unary_image_op(
            object,
            "math",
            |format| binary_output_format("math", format),
            |value| unary_math(value, math),
        )
    }
}

unsafe fn op_complex(object: *mut VipsObject) -> Result<(), ()> {
    let cmplx = unsafe { get_enum(object, "cmplx")? } as VipsOperationComplex;
    let image = unsafe { get_image_ref(object, "in")? };
    let result = (|| {
        let (spec, mut pairs) = unsafe { read_complex_image(image)? };
        for pair in &mut pairs {
            match cmplx {
                VIPS_OPERATION_COMPLEX_POLAR => {
                    let amplitude = pair.real.hypot(pair.imag);
                    let mut phase = pair.imag.atan2(pair.real).to_degrees();
                    if phase < 0.0 {
                        phase += 360.0;
                    }
                    *pair = ComplexSample {
                        real: amplitude,
                        imag: phase,
                    };
                }
                VIPS_OPERATION_COMPLEX_RECT => {
                    let radians = pair.imag.to_radians();
                    *pair = ComplexSample {
                        real: pair.real * radians.cos(),
                        imag: pair.real * radians.sin(),
                    };
                }
                VIPS_OPERATION_COMPLEX_CONJ => {
                    pair.imag = -pair.imag;
                }
                _ => return Err(()),
            }
        }
        let out = unsafe { complex_image_from_samples(spec, &pairs, image)? };
        unsafe { set_output_image(object, "out", out) }
    })();
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

unsafe fn op_complexget(object: *mut VipsObject) -> Result<(), ()> {
    let get = unsafe { get_enum(object, "get")? } as VipsOperationComplexget;
    let image = unsafe { get_image_ref(object, "in")? };
    let result = (|| {
        let input_format = unsafe { image.as_ref() }.ok_or(())?.BandFmt;
        let out = if complex_component_format(input_format).is_some() {
            let (spec, pairs) = unsafe { read_complex_image(image)? };
            let mut out = new_output_from_spec(spec, complexget_output_format(spec.format));
            out.data = pairs
                .into_iter()
                .map(|pair| match get {
                    VIPS_OPERATION_COMPLEXGET_REAL => pair.real,
                    VIPS_OPERATION_COMPLEXGET_IMAG => pair.imag,
                    _ => 0.0,
                })
                .collect();
            out
        } else {
            let mut out = ImageBuffer::from_image(image)?;
            match get {
                VIPS_OPERATION_COMPLEXGET_REAL => {}
                VIPS_OPERATION_COMPLEXGET_IMAG => out.data.fill(0.0),
                _ => return Err(()),
            }
            out
        };
        let out = out.into_image_like(image);
        unsafe { set_output_image(object, "out", out) }
    })();
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

unsafe fn op_complexform(object: *mut VipsObject) -> Result<(), ()> {
    let left_image = unsafe { get_image_ref(object, "left")? };
    let right_image = unsafe { get_image_ref(object, "right")? };
    let result = (|| {
        let left_ref = unsafe { left_image.as_ref() }.ok_or(())?;
        let right_ref = unsafe { right_image.as_ref() }.ok_or(())?;
        if complex_component_format(left_ref.BandFmt).is_some()
            || complex_component_format(right_ref.BandFmt).is_some()
        {
            return Err(());
        }

        let left = ImageBuffer::from_image(left_image)?;
        let right = ImageBuffer::from_image(right_image)?;
        let (left, right, common) = align_pair(&left, &right)?;
        let (mut spec, _) = complex_from_buffer(&left);
        spec.format = complex_promoted_format(common);
        let pairs: Vec<ComplexSample> = left
            .data
            .into_iter()
            .zip(right.data)
            .map(|(real, imag)| ComplexSample { real, imag })
            .collect();
        let out = unsafe { complex_image_from_samples(spec, &pairs, left_image)? };
        unsafe { set_output_image(object, "out", out) }
    })();
    unsafe {
        crate::runtime::object::object_unref(right_image);
        crate::runtime::object::object_unref(left_image);
    }
    result
}

unsafe fn op_boolean_const(object: *mut VipsObject) -> Result<(), ()> {
    let op = unsafe { get_enum(object, "boolean")? } as VipsOperationBoolean;
    unsafe {
        unary_const_image_op(
            object,
            "boolean_const",
            |format| binary_output_format("boolean", format),
            |left, right, format| boolean_value(format, left, right, op),
        )
    }
}

unsafe fn op_relational_const(object: *mut VipsObject) -> Result<(), ()> {
    let op = unsafe { get_enum(object, "relational")? } as VipsOperationRelational;
    unsafe {
        unary_const_image_op(
            object,
            "relational_const",
            |format| binary_output_format("relational", format),
            |left, right, _| relation_value(left, right, op),
        )
    }
}

unsafe fn op_math2_const(object: *mut VipsObject) -> Result<(), ()> {
    let op = unsafe { get_enum(object, "math2")? } as VipsOperationMath2;
    unsafe {
        unary_const_image_op(
            object,
            "math2_const",
            |format| binary_output_format("math2", format),
            |left, right, _| binary_math2(left, right, op),
        )
    }
}

unsafe fn op_remainder_const(object: *mut VipsObject) -> Result<(), ()> {
    unsafe {
        unary_const_image_op(
            object,
            "remainder_const",
            |format| binary_output_format("remainder", format),
            |left, right, format| {
                let right = right.trunc();
                remainder_value(format, left, right)
            },
        )
    }
}

unsafe fn op_linear(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let a = unsafe { get_array_double(object, "a")? };
    let b = unsafe { get_array_double(object, "b")? };
    let target_bands = input.spec.bands.max(a.len().max(1)).max(b.len().max(1));
    for len in [input.spec.bands, a.len().max(1), b.len().max(1)] {
        if len != 1 && len != target_bands {
            return Err(());
        }
    }
    let input = replicate_if_needed(&input, target_bands)?;
    let mut out = new_output_from_spec(
        input.spec,
        if unsafe { super::argument_assigned(object, "uchar")? }
            && unsafe { super::get_bool(object, "uchar")? }
        {
            VIPS_FORMAT_UCHAR
        } else {
            binary_output_format("linear", input.spec.format)?
        },
    );
    out.data = Vec::with_capacity(input.data.len());
    for (index, value) in input.data.iter().copied().enumerate() {
        let band = index % target_bands;
        let aa = a
            .get(if a.len() <= 1 { 0 } else { band })
            .copied()
            .unwrap_or_else(|| *a.first().unwrap_or(&1.0));
        let bb = b
            .get(if b.len() <= 1 { 0 } else { band })
            .copied()
            .unwrap_or_else(|| *b.first().unwrap_or(&0.0));
        out.data.push(value * aa + bb);
    }
    let image = unsafe { get_image_ref(object, "in")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

unsafe fn op_sum(object: *mut VipsObject) -> Result<(), ()> {
    let images = unsafe { get_array_images(object, "in")? };
    let mut buffers = Vec::with_capacity(images.len());
    for image in &images {
        buffers.push(ImageBuffer::from_image(*image)?);
    }
    let first = buffers.first().ok_or(())?.clone();
    let mut acc = first;
    let mut common = acc.spec.format;
    for next in buffers.iter().skip(1) {
        let (left, right, fmt) = align_pair(&acc, next)?;
        common = fmt;
        let mut out = new_output_from_spec(left.spec, binary_output_format("sum", fmt)?);
        out.data = left
            .data
            .iter()
            .zip(&right.data)
            .map(|(l, r)| l + r)
            .collect();
        acc = out;
    }
    acc.spec.format = binary_output_format("sum", common)?;
    let like = *images.first().ok_or(())?;
    unsafe { set_output_image_like(object, "out", acc, like) }
}

unsafe fn op_avg(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let avg = input.data.iter().sum::<f64>() / input.data.len().max(1) as f64;
    unsafe { set_output_double(object, "out", avg) }
}

unsafe fn op_deviate(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let mean = input.data.iter().sum::<f64>() / input.data.len().max(1) as f64;
    let variance = input
        .data
        .iter()
        .map(|value| {
            let delta = *value - mean;
            delta * delta
        })
        .sum::<f64>()
        / input.data.len().max(1) as f64;
    unsafe { set_output_double(object, "out", variance.sqrt()) }
}

unsafe fn op_hough_line(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    if input.spec.bands != 1 || input.spec.width == 0 || input.spec.height == 0 {
        return Err(());
    }

    let width = if unsafe { super::argument_assigned(object, "width")? } {
        usize::try_from(unsafe { get_int(object, "width")? }).map_err(|_| ())?
    } else {
        256
    };
    let height = if unsafe { super::argument_assigned(object, "height")? } {
        usize::try_from(unsafe { get_int(object, "height")? }).map_err(|_| ())?
    } else {
        256
    };
    if width == 0 || height == 0 {
        return Err(());
    }

    let mut out = ImageBuffer::new(
        width,
        height,
        1,
        VIPS_FORMAT_UINT,
        VIPS_CODING_NONE,
        VIPS_INTERPRETATION_MATRIX,
    );
    let sin_lut: Vec<f64> = (0..(2 * width))
        .map(|index| {
            let theta = std::f64::consts::PI * index as f64 / width as f64;
            theta.sin()
        })
        .collect();

    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            if clamp_for_format(input.get(x, y, 0), VIPS_FORMAT_UCHAR) == 0.0 {
                continue;
            }
            let xd = x as f64 / input.spec.width as f64;
            let yd = y as f64 / input.spec.height as f64;
            for angle in 0..width {
                let angle90 = angle + width / 2;
                let r = xd * sin_lut[angle90] + yd * sin_lut[angle];
                let ri = (height as f64 * r) as isize;
                if ri >= 0 && (ri as usize) < height {
                    let current = out.get(angle, ri as usize, 0);
                    out.set(angle, ri as usize, 0, current + 1.0);
                }
            }
        }
    }

    unsafe { set_output_image(object, "out", out.to_image()) }
}

unsafe fn op_hough_circle(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    if input.spec.bands != 1 || input.spec.width == 0 || input.spec.height == 0 {
        return Err(());
    }

    let scale = if unsafe { super::argument_assigned(object, "scale")? } {
        usize::try_from(unsafe { get_int(object, "scale")? }).map_err(|_| ())?
    } else {
        1
    };
    let min_radius = if unsafe { super::argument_assigned(object, "min_radius")? } {
        usize::try_from(unsafe { get_int(object, "min_radius")? }).map_err(|_| ())?
    } else {
        10
    };
    let max_radius = if unsafe { super::argument_assigned(object, "max_radius")? } {
        usize::try_from(unsafe { get_int(object, "max_radius")? }).map_err(|_| ())?
    } else {
        20
    };
    if scale == 0 || min_radius == 0 || max_radius <= min_radius {
        return Err(());
    }

    let out_width = input.spec.width / scale;
    let out_height = input.spec.height / scale;
    let bands = 1 + (max_radius - min_radius) / scale;
    if out_width == 0 || out_height == 0 || bands == 0 {
        return Err(());
    }

    let mut accum = vec![0.0; out_width * out_height * bands];
    let out_index = |x: usize, y: usize, band: usize| (y * out_width + x) * bands + band;
    let mut vote = |cx: isize, cy: isize, dx: isize, dy: isize, band: usize| {
        let points = [
            (cx + dx, cy + dy),
            (cx + dy, cy + dx),
            (cx - dy, cy + dx),
            (cx - dx, cy + dy),
            (cx - dx, cy - dy),
            (cx - dy, cy - dx),
            (cx + dy, cy - dx),
            (cx + dx, cy - dy),
        ];
        for (px, py) in points {
            if px < 0 || py < 0 {
                continue;
            }
            let (px, py) = (px as usize, py as usize);
            if px >= out_width || py >= out_height {
                continue;
            }
            accum[out_index(px, py, band)] += 1.0;
        }
    };

    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            if clamp_for_format(input.get(x, y, 0), VIPS_FORMAT_UCHAR) == 0.0 {
                continue;
            }
            let sx = (x / scale) as isize;
            let sy = (y / scale) as isize;
            for band in 0..bands {
                let radius = (band + min_radius / scale) as isize;
                let mut dx = radius;
                let mut dy = 0isize;
                let mut err = 1 - dx;
                while dx >= dy {
                    vote(sx, sy, dx, dy, band);
                    dy += 1;
                    if err < 0 {
                        err += 2 * dy + 1;
                    } else {
                        dx -= 1;
                        err += 2 * (dy - dx + 1);
                    }
                }
            }
        }
    }

    let mut out = ImageBuffer::new(
        out_width,
        out_height,
        bands,
        VIPS_FORMAT_DOUBLE,
        VIPS_CODING_NONE,
        VIPS_INTERPRETATION_MATRIX,
    );
    out.data = accum
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            if value == 0.0 {
                0.0
            } else {
                value + (index % bands) as f64 * 1e-6
            }
        })
        .collect();
    unsafe { set_output_image(object, "out", out.to_image()) }
}

unsafe fn op_measure(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let h = usize::try_from(unsafe { get_int(object, "h")? }).map_err(|_| ())?;
    let v = usize::try_from(unsafe { get_int(object, "v")? }).map_err(|_| ())?;
    if h == 0 || v == 0 {
        return Err(());
    }

    let left = if unsafe { super::argument_assigned(object, "left")? } {
        usize::try_from(unsafe { get_int(object, "left")? }).map_err(|_| ())?
    } else {
        0
    };
    let top = if unsafe { super::argument_assigned(object, "top")? } {
        usize::try_from(unsafe { get_int(object, "top")? }).map_err(|_| ())?
    } else {
        0
    };
    let width = if unsafe { super::argument_assigned(object, "width")? } {
        usize::try_from(unsafe { get_int(object, "width")? }).map_err(|_| ())?
    } else {
        input.spec.width
    };
    let height = if unsafe { super::argument_assigned(object, "height")? } {
        usize::try_from(unsafe { get_int(object, "height")? }).map_err(|_| ())?
    } else {
        input.spec.height
    };
    if left >= input.spec.width
        || top >= input.spec.height
        || left.saturating_add(width) > input.spec.width
        || top.saturating_add(height) > input.spec.height
    {
        return Err(());
    }

    let patch_width = width as f64 / h as f64;
    let patch_height = height as f64 / v as f64;
    let sample_width = ((patch_width + 1.0) / 2.0) as usize;
    let sample_height = ((patch_height + 1.0) / 2.0) as usize;
    if sample_width == 0 || sample_height == 0 {
        return Err(());
    }

    let mut out = ImageBuffer::new(
        input.spec.bands,
        h * v,
        1,
        VIPS_FORMAT_DOUBLE,
        VIPS_CODING_NONE,
        VIPS_INTERPRETATION_MATRIX,
    );

    for j in 0..v {
        for i in 0..h {
            let x = left + (i as f64 * patch_width + (patch_width + 2.0) / 4.0) as usize;
            let y = top + (j as f64 * patch_height + (patch_height + 2.0) / 4.0) as usize;
            if x.saturating_add(sample_width) > input.spec.width
                || y.saturating_add(sample_height) > input.spec.height
            {
                return Err(());
            }

            for band in 0..input.spec.bands {
                let mut sum = 0.0;
                for py in y..(y + sample_height) {
                    for px in x..(x + sample_width) {
                        sum += input.get(px, py, band);
                    }
                }
                let avg = sum / (sample_width * sample_height) as f64;
                out.set(band, i + j * h, 0, avg);
            }
        }
    }

    unsafe { set_output_image(object, "out", out.to_image()) }
}

unsafe fn op_project(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let format = project_output_format(input.spec.format)?;
    let mut columns = ImageBuffer::new(
        input.spec.width,
        1,
        input.spec.bands,
        format,
        VIPS_CODING_NONE,
        crate::abi::image::VIPS_INTERPRETATION_HISTOGRAM,
    );
    let mut rows = ImageBuffer::new(
        1,
        input.spec.height,
        input.spec.bands,
        format,
        VIPS_CODING_NONE,
        crate::abi::image::VIPS_INTERPRETATION_HISTOGRAM,
    );

    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            for band in 0..input.spec.bands {
                let value = input.get(x, y, band);
                let column_sum = columns.get(x, 0, band) + value;
                let row_sum = rows.get(0, y, band) + value;
                columns.set(x, 0, band, column_sum);
                rows.set(0, y, band, row_sum);
            }
        }
    }

    unsafe {
        set_output_image(object, "columns", columns.to_image())?;
        set_output_image(object, "rows", rows.to_image())?;
    }
    Ok(())
}

unsafe fn op_stats(object: *mut VipsObject) -> Result<(), ()> {
    const COL_MIN: usize = 0;
    const COL_MAX: usize = 1;
    const COL_SUM: usize = 2;
    const COL_SUM2: usize = 3;
    const COL_AVG: usize = 4;
    const COL_SD: usize = 5;
    const COL_XMIN: usize = 6;
    const COL_YMIN: usize = 7;
    const COL_XMAX: usize = 8;
    const COL_YMAX: usize = 9;
    const COL_LAST: usize = 10;

    let input = unsafe { get_image_buffer(object, "in")? };
    if input.spec.width == 0 || input.spec.height == 0 || input.spec.bands == 0 {
        return Err(());
    }

    let pels = (input.spec.width * input.spec.height) as f64;
    let vals = pels * input.spec.bands as f64;
    let mut out = ImageBuffer::new(
        COL_LAST,
        input.spec.bands + 1,
        1,
        VIPS_FORMAT_DOUBLE,
        VIPS_CODING_NONE,
        VIPS_INTERPRETATION_MATRIX,
    );

    let mut all_min = f64::INFINITY;
    let mut all_max = f64::NEG_INFINITY;
    let mut all_sum = 0.0;
    let mut all_sum2 = 0.0;
    let mut all_xmin = 0.0;
    let mut all_ymin = 0.0;
    let mut all_xmax = 0.0;
    let mut all_ymax = 0.0;

    for band in 0..input.spec.bands {
        let mut min_value = input.get(0, 0, band);
        let mut max_value = min_value;
        let mut sum = 0.0;
        let mut sum2 = 0.0;
        let mut xmin = 0usize;
        let mut ymin = 0usize;
        let mut xmax = 0usize;
        let mut ymax = 0usize;

        for y in 0..input.spec.height {
            for x in 0..input.spec.width {
                let value = input.get(x, y, band);
                sum += value;
                sum2 += value * value;
                if value > max_value {
                    max_value = value;
                    xmax = x;
                    ymax = y;
                } else if value < min_value {
                    min_value = value;
                    xmin = x;
                    ymin = y;
                }
            }
        }

        let row = band + 1;
        out.set(COL_MIN, row, 0, min_value);
        out.set(COL_MAX, row, 0, max_value);
        out.set(COL_SUM, row, 0, sum);
        out.set(COL_SUM2, row, 0, sum2);
        out.set(COL_AVG, row, 0, sum / pels);
        out.set(
            COL_SD,
            row,
            0,
            if pels > 0.0 {
                ((sum2 - (sum * sum / pels)).abs() / pels).sqrt()
            } else {
                0.0
            },
        );
        out.set(COL_XMIN, row, 0, xmin as f64);
        out.set(COL_YMIN, row, 0, ymin as f64);
        out.set(COL_XMAX, row, 0, xmax as f64);
        out.set(COL_YMAX, row, 0, ymax as f64);

        if min_value < all_min {
            all_min = min_value;
            all_xmin = xmin as f64;
            all_ymin = ymin as f64;
        }
        if max_value > all_max {
            all_max = max_value;
            all_xmax = xmax as f64;
            all_ymax = ymax as f64;
        }
        all_sum += sum;
        all_sum2 += sum2;
    }

    out.set(COL_MIN, 0, 0, all_min);
    out.set(COL_MAX, 0, 0, all_max);
    out.set(COL_SUM, 0, 0, all_sum);
    out.set(COL_SUM2, 0, 0, all_sum2);
    out.set(COL_AVG, 0, 0, all_sum / vals);
    out.set(
        COL_SD,
        0,
        0,
        if vals > 0.0 {
            ((all_sum2 - (all_sum * all_sum / vals)).abs() / vals).sqrt()
        } else {
            0.0
        },
    );
    out.set(COL_XMIN, 0, 0, all_xmin);
    out.set(COL_YMIN, 0, 0, all_ymin);
    out.set(COL_XMAX, 0, 0, all_xmax);
    out.set(COL_YMAX, 0, 0, all_ymax);

    unsafe { set_output_image(object, "out", out.to_image()) }
}

unsafe fn op_extrema(object: *mut VipsObject, find_max: bool) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let mut best_index = None;
    let mut best_value = if find_max {
        f64::NEG_INFINITY
    } else {
        f64::INFINITY
    };

    for (index, value) in input.data.iter().copied().enumerate() {
        let better = if find_max {
            value > best_value
        } else {
            value < best_value
        };
        if better {
            best_value = value;
            best_index = Some(index);
        }
    }

    let best_index = best_index.ok_or(())?;
    let pixel = best_index / input.spec.bands.max(1);
    let x = (pixel % input.spec.width.max(1)) as i32;
    let y = (pixel / input.spec.width.max(1)) as i32;

    unsafe {
        set_output_double(object, "out", best_value)?;
        let _ = set_output_int(object, "x", x);
        let _ = set_output_int(object, "y", y);
    }
    Ok(())
}

unsafe fn op_getpoint(object: *mut VipsObject) -> Result<(), ()> {
    let image = unsafe { get_image_ref(object, "in")? };
    let x = unsafe { get_int(object, "x")? };
    let y = unsafe { get_int(object, "y")? };
    if x < 0 || y < 0 {
        unsafe {
            crate::runtime::object::object_unref(image);
        }
        return Err(());
    }
    let x = x as usize;
    let y = y as usize;
    ensure_pixels(image)?;
    let Some(image_ref) = (unsafe { image.as_ref() }) else {
        unsafe {
            crate::runtime::object::object_unref(image);
        }
        return Err(());
    };
    if x >= image_ref.Xsize.max(0) as usize || y >= image_ref.Ysize.max(0) as usize {
        unsafe {
            crate::runtime::object::object_unref(image);
        }
        return Err(());
    }
    let Some(state) = (unsafe { image_state(image) }) else {
        unsafe {
            crate::runtime::object::object_unref(image);
        }
        return Err(());
    };

    let sample_size = format_bytes(image_ref.BandFmt);
    let components = format_components(image_ref.BandFmt);
    if sample_size == 0 || components == 0 {
        unsafe {
            crate::runtime::object::object_unref(image);
        }
        return Err(());
    }

    let pixel_stride = sample_size * image_ref.Bands.max(0) as usize;
    let offset = (y * image_ref.Xsize.max(0) as usize + x) * pixel_stride;
    let pixel = state.pixels.get(offset..offset + pixel_stride).ok_or(())?;

    let mut values = Vec::with_capacity(image_ref.Bands.max(0) as usize);
    for sample in pixel.chunks_exact(sample_size) {
        match image_ref.BandFmt {
            VIPS_FORMAT_COMPLEX => {
                let real = f32::from_ne_bytes(sample[0..4].try_into().map_err(|_| ())?) as f64;
                values.push(real);
            }
            VIPS_FORMAT_DPCOMPLEX => {
                let real = f64::from_ne_bytes(sample[0..8].try_into().map_err(|_| ())?);
                values.push(real);
            }
            _ => values.push(read_sample(sample, image_ref.BandFmt).ok_or(())?),
        }
    }

    let result = unsafe { set_output_array_double(object, "out_array", &values) };
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

pub(crate) unsafe fn dispatch(object: *mut VipsObject, nickname: &str) -> Result<bool, ()> {
    match nickname {
        "abs" => {
            unsafe { op_abs(object)? };
            Ok(true)
        }
        "add" => {
            unsafe { binary_image_op(object, "left", "right", "out", "add", |l, r, _| l + r)? };
            Ok(true)
        }
        "subtract" => {
            unsafe {
                binary_image_op(object, "left", "right", "out", "subtract", |l, r, _| l - r)?
            };
            Ok(true)
        }
        "multiply" => {
            unsafe {
                binary_image_op(object, "left", "right", "out", "multiply", |l, r, _| l * r)?
            };
            Ok(true)
        }
        "divide" => {
            unsafe {
                binary_image_op(object, "left", "right", "out", "divide", |l, r, _| {
                    if r == 0.0 {
                        0.0
                    } else {
                        l / r
                    }
                })?
            };
            Ok(true)
        }
        "remainder" => {
            unsafe {
                binary_image_op(object, "left", "right", "out", "remainder", |l, r, fmt| {
                    remainder_value(fmt, l, r)
                })?
            };
            Ok(true)
        }
        "boolean" => {
            let op = unsafe { get_enum(object, "boolean")? } as VipsOperationBoolean;
            unsafe {
                binary_image_op(object, "left", "right", "out", "boolean", |l, r, fmt| {
                    boolean_value(fmt, l, r, op)
                })?
            };
            Ok(true)
        }
        "boolean_const" => {
            unsafe { op_boolean_const(object)? };
            Ok(true)
        }
        "complex" => {
            unsafe { op_complex(object)? };
            Ok(true)
        }
        "complexform" => {
            unsafe { op_complexform(object)? };
            Ok(true)
        }
        "complexget" => {
            unsafe { op_complexget(object)? };
            Ok(true)
        }
        "relational" => {
            let op = unsafe { get_enum(object, "relational")? } as VipsOperationRelational;
            unsafe {
                binary_image_op(object, "left", "right", "out", "relational", |l, r, _| {
                    relation_value(l, r, op)
                })?
            };
            Ok(true)
        }
        "relational_const" => {
            unsafe { op_relational_const(object)? };
            Ok(true)
        }
        "math2" => {
            let op = unsafe { get_enum(object, "math2")? } as VipsOperationMath2;
            unsafe {
                binary_image_op(object, "left", "right", "out", "math2", |l, r, _| {
                    binary_math2(l, r, op)
                })?
            };
            Ok(true)
        }
        "math2_const" => {
            unsafe { op_math2_const(object)? };
            Ok(true)
        }
        "invert" => {
            unsafe { op_invert(object)? };
            Ok(true)
        }
        "sign" => {
            unsafe { op_sign(object)? };
            Ok(true)
        }
        "round" => {
            unsafe { op_round(object)? };
            Ok(true)
        }
        "find_trim" => {
            unsafe { op_find_trim(object)? };
            Ok(true)
        }
        "math" => {
            unsafe { op_math(object)? };
            Ok(true)
        }
        "linear" => {
            unsafe { op_linear(object)? };
            Ok(true)
        }
        "measure" => {
            unsafe { op_measure(object)? };
            Ok(true)
        }
        "project" => {
            unsafe { op_project(object)? };
            Ok(true)
        }
        "remainder_const" => {
            unsafe { op_remainder_const(object)? };
            Ok(true)
        }
        "sum" => {
            unsafe { op_sum(object)? };
            Ok(true)
        }
        "stats" => {
            unsafe { op_stats(object)? };
            Ok(true)
        }
        "avg" => {
            unsafe { op_avg(object)? };
            Ok(true)
        }
        "deviate" => {
            unsafe { op_deviate(object)? };
            Ok(true)
        }
        "hough_circle" => {
            unsafe { op_hough_circle(object)? };
            Ok(true)
        }
        "hough_line" => {
            unsafe { op_hough_line(object)? };
            Ok(true)
        }
        "getpoint" => {
            unsafe { op_getpoint(object)? };
            Ok(true)
        }
        "max" => {
            unsafe { op_extrema(object, true)? };
            Ok(true)
        }
        "min" => {
            unsafe { op_extrema(object, false)? };
            Ok(true)
        }
        _ => Ok(false),
    }
}
