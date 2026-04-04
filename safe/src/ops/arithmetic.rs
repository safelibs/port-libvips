use crate::abi::basic::{
    VipsOperationBoolean, VipsOperationMath, VipsOperationMath2, VipsOperationRelational,
    VipsOperationRound, VIPS_OPERATION_BOOLEAN_AND, VIPS_OPERATION_BOOLEAN_EOR,
    VIPS_OPERATION_BOOLEAN_LSHIFT, VIPS_OPERATION_BOOLEAN_OR, VIPS_OPERATION_BOOLEAN_RSHIFT,
    VIPS_OPERATION_MATH2_ATAN2, VIPS_OPERATION_MATH2_POW, VIPS_OPERATION_MATH2_WOP,
    VIPS_OPERATION_MATH_ACOS, VIPS_OPERATION_MATH_ACOSH, VIPS_OPERATION_MATH_ASIN,
    VIPS_OPERATION_MATH_ASINH, VIPS_OPERATION_MATH_ATAN, VIPS_OPERATION_MATH_ATANH,
    VIPS_OPERATION_MATH_COS, VIPS_OPERATION_MATH_COSH, VIPS_OPERATION_MATH_EXP,
    VIPS_OPERATION_MATH_EXP10, VIPS_OPERATION_MATH_LOG, VIPS_OPERATION_MATH_LOG10,
    VIPS_OPERATION_MATH_SIN, VIPS_OPERATION_MATH_SINH, VIPS_OPERATION_MATH_TAN,
    VIPS_OPERATION_MATH_TANH, VIPS_OPERATION_RELATIONAL_EQUAL,
    VIPS_OPERATION_RELATIONAL_LESS, VIPS_OPERATION_RELATIONAL_LESSEQ,
    VIPS_OPERATION_RELATIONAL_MORE, VIPS_OPERATION_RELATIONAL_MOREEQ,
    VIPS_OPERATION_RELATIONAL_NOTEQ, VIPS_OPERATION_ROUND_CEIL, VIPS_OPERATION_ROUND_FLOOR,
    VIPS_OPERATION_ROUND_RINT,
};
use crate::abi::image::{
    VipsBandFormat, VipsImage, VIPS_FORMAT_CHAR, VIPS_FORMAT_DOUBLE, VIPS_FORMAT_FLOAT,
    VIPS_FORMAT_INT, VIPS_FORMAT_SHORT, VIPS_FORMAT_UCHAR, VIPS_FORMAT_UINT,
    VIPS_FORMAT_USHORT,
};
use crate::abi::object::VipsObject;
use crate::pixels::format::{common_format, format_kind, NumericKind};
use crate::pixels::{ImageBuffer, ImageSpec};

use super::{
    get_array_double, get_array_images, get_double, get_enum, get_image_buffer, get_image_ref,
    set_output_double, set_output_image_like,
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

fn replicate_if_needed(buffer: &ImageBuffer, bands: usize) -> Result<ImageBuffer, ()> {
    if buffer.spec.bands == bands {
        Ok(buffer.clone())
    } else {
        buffer.replicate_bands(bands)
    }
}

fn align_pair(left: &ImageBuffer, right: &ImageBuffer) -> Result<(ImageBuffer, ImageBuffer, VipsBandFormat), ()> {
    let format = common_format(left.spec.format, right.spec.format).ok_or(())?;
    let width = left.spec.width.max(right.spec.width);
    let height = left.spec.height.max(right.spec.height);
    let bands = match (left.spec.bands, right.spec.bands) {
        (a, b) if a == b => a,
        (1, b) => b,
        (a, 1) => a,
        _ => return Err(()),
    };

    let left = replicate_if_needed(left, bands)?.with_format(format).zero_extend(width, height);
    let right = replicate_if_needed(right, bands)?.with_format(format).zero_extend(width, height);
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

fn int_binary_u8(left: f64, right: f64, op: VipsOperationBoolean) -> f64 {
    let shift = (right as u32) & 7;
    match op {
        VIPS_OPERATION_BOOLEAN_AND => ((left as u8) & (right as u8)) as f64,
        VIPS_OPERATION_BOOLEAN_OR => ((left as u8) | (right as u8)) as f64,
        VIPS_OPERATION_BOOLEAN_EOR => ((left as u8) ^ (right as u8)) as f64,
        VIPS_OPERATION_BOOLEAN_LSHIFT => (left as u8).wrapping_shl(shift) as f64,
        VIPS_OPERATION_BOOLEAN_RSHIFT => ((left as u8) >> shift) as f64,
        _ => 0.0,
    }
}

fn int_binary_i8(left: f64, right: f64, op: VipsOperationBoolean) -> f64 {
    let shift = (right as u32) & 7;
    match op {
        VIPS_OPERATION_BOOLEAN_AND => (((left as i8) & (right as i8)) as i8) as f64,
        VIPS_OPERATION_BOOLEAN_OR => (((left as i8) | (right as i8)) as i8) as f64,
        VIPS_OPERATION_BOOLEAN_EOR => (((left as i8) ^ (right as i8)) as i8) as f64,
        VIPS_OPERATION_BOOLEAN_LSHIFT => (left as i8).wrapping_shl(shift) as f64,
        VIPS_OPERATION_BOOLEAN_RSHIFT => ((left as i8) >> shift) as f64,
        _ => 0.0,
    }
}

fn int_binary_u16(left: f64, right: f64, op: VipsOperationBoolean) -> f64 {
    let shift = (right as u32) & 15;
    match op {
        VIPS_OPERATION_BOOLEAN_AND => ((left as u16) & (right as u16)) as f64,
        VIPS_OPERATION_BOOLEAN_OR => ((left as u16) | (right as u16)) as f64,
        VIPS_OPERATION_BOOLEAN_EOR => ((left as u16) ^ (right as u16)) as f64,
        VIPS_OPERATION_BOOLEAN_LSHIFT => (left as u16).wrapping_shl(shift) as f64,
        VIPS_OPERATION_BOOLEAN_RSHIFT => ((left as u16) >> shift) as f64,
        _ => 0.0,
    }
}

fn int_binary_i16(left: f64, right: f64, op: VipsOperationBoolean) -> f64 {
    let shift = (right as u32) & 15;
    match op {
        VIPS_OPERATION_BOOLEAN_AND => (((left as i16) & (right as i16)) as i16) as f64,
        VIPS_OPERATION_BOOLEAN_OR => (((left as i16) | (right as i16)) as i16) as f64,
        VIPS_OPERATION_BOOLEAN_EOR => (((left as i16) ^ (right as i16)) as i16) as f64,
        VIPS_OPERATION_BOOLEAN_LSHIFT => (left as i16).wrapping_shl(shift) as f64,
        VIPS_OPERATION_BOOLEAN_RSHIFT => ((left as i16) >> shift) as f64,
        _ => 0.0,
    }
}

fn int_binary_u32(left: f64, right: f64, op: VipsOperationBoolean) -> f64 {
    let shift = (right as u32) & 31;
    match op {
        VIPS_OPERATION_BOOLEAN_AND => ((left as u32) & (right as u32)) as f64,
        VIPS_OPERATION_BOOLEAN_OR => ((left as u32) | (right as u32)) as f64,
        VIPS_OPERATION_BOOLEAN_EOR => ((left as u32) ^ (right as u32)) as f64,
        VIPS_OPERATION_BOOLEAN_LSHIFT => (left as u32).wrapping_shl(shift) as f64,
        VIPS_OPERATION_BOOLEAN_RSHIFT => ((left as u32) >> shift) as f64,
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
        VIPS_OPERATION_ROUND_RINT => value.round(),
        VIPS_OPERATION_ROUND_CEIL => value.ceil(),
        VIPS_OPERATION_ROUND_FLOOR => value.floor(),
        _ => value,
    }
}

fn unary_math(value: f64, math: VipsOperationMath) -> f64 {
    match math {
        VIPS_OPERATION_MATH_SIN => value.sin(),
        VIPS_OPERATION_MATH_COS => value.cos(),
        VIPS_OPERATION_MATH_TAN => value.tan(),
        VIPS_OPERATION_MATH_ASIN => value.asin(),
        VIPS_OPERATION_MATH_ACOS => value.acos(),
        VIPS_OPERATION_MATH_ATAN => value.atan(),
        VIPS_OPERATION_MATH_LOG => value.ln(),
        VIPS_OPERATION_MATH_LOG10 => value.log10(),
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
        VIPS_OPERATION_RELATIONAL_EQUAL => if left == right { yes } else { no },
        VIPS_OPERATION_RELATIONAL_NOTEQ => if left != right { yes } else { no },
        VIPS_OPERATION_RELATIONAL_LESS => if left < right { yes } else { no },
        VIPS_OPERATION_RELATIONAL_LESSEQ => if left <= right { yes } else { no },
        VIPS_OPERATION_RELATIONAL_MORE => if left > right { yes } else { no },
        VIPS_OPERATION_RELATIONAL_MOREEQ => if left >= right { yes } else { no },
        _ => no,
    }
}

fn remainder_value(format: VipsBandFormat, left: f64, right: f64) -> f64 {
    if right == 0.0 {
        return 0.0;
    }
    match format_kind(format) {
        Some(NumericKind::Float) => left % right,
        Some(NumericKind::Unsigned) | Some(NumericKind::Signed) => {
            let left = left.trunc() as i64;
            let right = right.trunc() as i64;
            if right == 0 { 0.0 } else { (left % right) as f64 }
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

fn apply_unary(input: &ImageBuffer, out_format: VipsBandFormat, mut f: impl FnMut(f64) -> f64) -> ImageBuffer {
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
    let out = apply_unary(&input, format, |value| invert_value(input.spec.format, value));
    let image = unsafe { get_image_ref(object, "in")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

unsafe fn op_sign(object: *mut VipsObject) -> Result<(), ()> {
    unsafe { unary_image_op(object, "sign", |format| binary_output_format("sign", format), sign_value) }
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

unsafe fn op_math(object: *mut VipsObject) -> Result<(), ()> {
    let math = unsafe { get_enum(object, "math")? } as VipsOperationMath;
    unsafe { unary_image_op(object, "math", |format| binary_output_format("math", format), |value| unary_math(value, math)) }
}

unsafe fn op_linear(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let a = unsafe { get_array_double(object, "a")? };
    let b = unsafe { get_array_double(object, "b")? };
    let bands = input.spec.bands.max(1);
    let mut out = new_output_from_spec(
        input.spec,
        if unsafe { super::argument_assigned(object, "uchar")? } && unsafe { super::get_bool(object, "uchar")? } {
            VIPS_FORMAT_UCHAR
        } else {
            binary_output_format("linear", input.spec.format)?
        },
    );
    out.data = Vec::with_capacity(input.data.len());
    for (index, value) in input.data.iter().copied().enumerate() {
        let band = index % bands;
        let aa = a.get(band).copied().unwrap_or_else(|| *a.first().unwrap_or(&1.0));
        let bb = b.get(band).copied().unwrap_or_else(|| *b.first().unwrap_or(&0.0));
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
            unsafe { binary_image_op(object, "left", "right", "out", "subtract", |l, r, _| l - r)? };
            Ok(true)
        }
        "multiply" => {
            unsafe { binary_image_op(object, "left", "right", "out", "multiply", |l, r, _| l * r)? };
            Ok(true)
        }
        "divide" => {
            unsafe {
                binary_image_op(object, "left", "right", "out", "divide", |l, r, _| {
                    if r == 0.0 { 0.0 } else { l / r }
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
        "relational" => {
            let op = unsafe { get_enum(object, "relational")? } as VipsOperationRelational;
            unsafe {
                binary_image_op(object, "left", "right", "out", "relational", |l, r, _| {
                    relation_value(l, r, op)
                })?
            };
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
        "math" => {
            unsafe { op_math(object)? };
            Ok(true)
        }
        "linear" => {
            unsafe { op_linear(object)? };
            Ok(true)
        }
        "sum" => {
            unsafe { op_sum(object)? };
            Ok(true)
        }
        "avg" => {
            unsafe { op_avg(object)? };
            Ok(true)
        }
        _ => Ok(false),
    }
}
