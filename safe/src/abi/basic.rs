use libc::{c_char, c_int, c_void};

use crate::abi::image::VipsImage;
use crate::abi::region::VipsRegion;

pub type VipsPel = u8;

pub type VipsCallbackFn = Option<unsafe extern "C" fn(a: *mut c_void, b: *mut c_void) -> c_int>;
pub type VipsSListMap2Fn =
    Option<unsafe extern "C" fn(item: *mut c_void, a: *mut c_void, b: *mut c_void) -> *mut c_void>;
pub type VipsSListMap4Fn = Option<
    unsafe extern "C" fn(
        item: *mut c_void,
        a: *mut c_void,
        b: *mut c_void,
        c: *mut c_void,
        d: *mut c_void,
    ) -> *mut c_void,
>;
pub type VipsSListFold2Fn = Option<
    unsafe extern "C" fn(
        item: *mut c_void,
        a: *mut c_void,
        b: *mut c_void,
        c: *mut c_void,
    ) -> *mut c_void,
>;

#[repr(C)]
pub struct VipsBuf {
    pub base: *mut c_char,
    pub mx: c_int,
    pub i: c_int,
    pub full: glib_sys::gboolean,
    pub lasti: c_int,
    pub dynamic: glib_sys::gboolean,
}

#[repr(C)]
pub struct VipsDbuf {
    pub data: *mut u8,
    pub allocated_size: usize,
    pub data_size: usize,
    pub write_point: usize,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct VipsRect {
    pub left: c_int,
    pub top: c_int,
    pub width: c_int,
    pub height: c_int,
}

c_enum! {
    pub type VipsOperationMath {
        VIPS_OPERATION_MATH_SIN = 0,
        VIPS_OPERATION_MATH_COS = 1,
        VIPS_OPERATION_MATH_TAN = 2,
        VIPS_OPERATION_MATH_ASIN = 3,
        VIPS_OPERATION_MATH_ACOS = 4,
        VIPS_OPERATION_MATH_ATAN = 5,
        VIPS_OPERATION_MATH_LOG = 6,
        VIPS_OPERATION_MATH_LOG10 = 7,
        VIPS_OPERATION_MATH_EXP = 8,
        VIPS_OPERATION_MATH_EXP10 = 9,
        VIPS_OPERATION_MATH_SINH = 10,
        VIPS_OPERATION_MATH_COSH = 11,
        VIPS_OPERATION_MATH_TANH = 12,
        VIPS_OPERATION_MATH_ASINH = 13,
        VIPS_OPERATION_MATH_ACOSH = 14,
        VIPS_OPERATION_MATH_ATANH = 15,
        VIPS_OPERATION_MATH_LAST = 16
    }
}

c_enum! {
    pub type VipsOperationMath2 {
        VIPS_OPERATION_MATH2_POW = 0,
        VIPS_OPERATION_MATH2_WOP = 1,
        VIPS_OPERATION_MATH2_ATAN2 = 2,
        VIPS_OPERATION_MATH2_LAST = 3
    }
}

c_enum! {
    pub type VipsOperationRound {
        VIPS_OPERATION_ROUND_RINT = 0,
        VIPS_OPERATION_ROUND_CEIL = 1,
        VIPS_OPERATION_ROUND_FLOOR = 2,
        VIPS_OPERATION_ROUND_LAST = 3
    }
}

c_enum! {
    pub type VipsOperationRelational {
        VIPS_OPERATION_RELATIONAL_EQUAL = 0,
        VIPS_OPERATION_RELATIONAL_NOTEQ = 1,
        VIPS_OPERATION_RELATIONAL_LESS = 2,
        VIPS_OPERATION_RELATIONAL_LESSEQ = 3,
        VIPS_OPERATION_RELATIONAL_MORE = 4,
        VIPS_OPERATION_RELATIONAL_MOREEQ = 5,
        VIPS_OPERATION_RELATIONAL_LAST = 6
    }
}

c_enum! {
    pub type VipsOperationBoolean {
        VIPS_OPERATION_BOOLEAN_AND = 0,
        VIPS_OPERATION_BOOLEAN_OR = 1,
        VIPS_OPERATION_BOOLEAN_EOR = 2,
        VIPS_OPERATION_BOOLEAN_LSHIFT = 3,
        VIPS_OPERATION_BOOLEAN_RSHIFT = 4,
        VIPS_OPERATION_BOOLEAN_LAST = 5
    }
}

c_enum! {
    pub type VipsOperationComplex {
        VIPS_OPERATION_COMPLEX_POLAR = 0,
        VIPS_OPERATION_COMPLEX_RECT = 1,
        VIPS_OPERATION_COMPLEX_CONJ = 2,
        VIPS_OPERATION_COMPLEX_LAST = 3
    }
}

c_enum! {
    pub type VipsOperationComplex2 {
        VIPS_OPERATION_COMPLEX2_CROSS_PHASE = 0,
        VIPS_OPERATION_COMPLEX2_LAST = 1
    }
}

c_enum! {
    pub type VipsOperationComplexget {
        VIPS_OPERATION_COMPLEXGET_REAL = 0,
        VIPS_OPERATION_COMPLEXGET_IMAG = 1,
        VIPS_OPERATION_COMPLEXGET_LAST = 2
    }
}

c_enum! {
    pub type VipsPrecision {
        VIPS_PRECISION_INTEGER = 0,
        VIPS_PRECISION_FLOAT = 1,
        VIPS_PRECISION_APPROXIMATE = 2,
        VIPS_PRECISION_LAST = 3
    }
}

c_enum! {
    pub type VipsIntent {
        VIPS_INTENT_PERCEPTUAL = 0,
        VIPS_INTENT_RELATIVE = 1,
        VIPS_INTENT_SATURATION = 2,
        VIPS_INTENT_ABSOLUTE = 3,
        VIPS_INTENT_LAST = 4
    }
}

c_enum! {
    pub type VipsPCS {
        VIPS_PCS_LAB = 0,
        VIPS_PCS_XYZ = 1,
        VIPS_PCS_LAST = 2
    }
}

c_enum! {
    pub type VipsExtend {
        VIPS_EXTEND_BLACK = 0,
        VIPS_EXTEND_COPY = 1,
        VIPS_EXTEND_REPEAT = 2,
        VIPS_EXTEND_MIRROR = 3,
        VIPS_EXTEND_WHITE = 4,
        VIPS_EXTEND_BACKGROUND = 5,
        VIPS_EXTEND_LAST = 6
    }
}

c_enum! {
    pub type VipsCompassDirection {
        VIPS_COMPASS_DIRECTION_CENTRE = 0,
        VIPS_COMPASS_DIRECTION_NORTH = 1,
        VIPS_COMPASS_DIRECTION_EAST = 2,
        VIPS_COMPASS_DIRECTION_SOUTH = 3,
        VIPS_COMPASS_DIRECTION_WEST = 4,
        VIPS_COMPASS_DIRECTION_NORTH_EAST = 5,
        VIPS_COMPASS_DIRECTION_SOUTH_EAST = 6,
        VIPS_COMPASS_DIRECTION_SOUTH_WEST = 7,
        VIPS_COMPASS_DIRECTION_NORTH_WEST = 8,
        VIPS_COMPASS_DIRECTION_LAST = 9
    }
}

c_enum! {
    pub type VipsDirection {
        VIPS_DIRECTION_HORIZONTAL = 0,
        VIPS_DIRECTION_VERTICAL = 1,
        VIPS_DIRECTION_LAST = 2
    }
}

c_enum! {
    pub type VipsAlign {
        VIPS_ALIGN_LOW = 0,
        VIPS_ALIGN_CENTRE = 1,
        VIPS_ALIGN_HIGH = 2,
        VIPS_ALIGN_LAST = 3
    }
}

c_enum! {
    pub type VipsAngle {
        VIPS_ANGLE_D0 = 0,
        VIPS_ANGLE_D90 = 1,
        VIPS_ANGLE_D180 = 2,
        VIPS_ANGLE_D270 = 3,
        VIPS_ANGLE_LAST = 4
    }
}

c_enum! {
    pub type VipsAngle45 {
        VIPS_ANGLE45_D0 = 0,
        VIPS_ANGLE45_D45 = 1,
        VIPS_ANGLE45_D90 = 2,
        VIPS_ANGLE45_D135 = 3,
        VIPS_ANGLE45_D180 = 4,
        VIPS_ANGLE45_D225 = 5,
        VIPS_ANGLE45_D270 = 6,
        VIPS_ANGLE45_D315 = 7,
        VIPS_ANGLE45_LAST = 8
    }
}

c_enum! {
    pub type VipsInteresting {
        VIPS_INTERESTING_NONE = 0,
        VIPS_INTERESTING_CENTRE = 1,
        VIPS_INTERESTING_ENTROPY = 2,
        VIPS_INTERESTING_ATTENTION = 3,
        VIPS_INTERESTING_LOW = 4,
        VIPS_INTERESTING_HIGH = 5,
        VIPS_INTERESTING_ALL = 6,
        VIPS_INTERESTING_LAST = 7
    }
}

c_enum! {
    pub type VipsBlendMode {
        VIPS_BLEND_MODE_CLEAR = 0,
        VIPS_BLEND_MODE_SOURCE = 1,
        VIPS_BLEND_MODE_OVER = 2,
        VIPS_BLEND_MODE_IN = 3,
        VIPS_BLEND_MODE_OUT = 4,
        VIPS_BLEND_MODE_ATOP = 5,
        VIPS_BLEND_MODE_DEST = 6,
        VIPS_BLEND_MODE_DEST_OVER = 7,
        VIPS_BLEND_MODE_DEST_IN = 8,
        VIPS_BLEND_MODE_DEST_OUT = 9,
        VIPS_BLEND_MODE_DEST_ATOP = 10,
        VIPS_BLEND_MODE_XOR = 11,
        VIPS_BLEND_MODE_ADD = 12,
        VIPS_BLEND_MODE_SATURATE = 13,
        VIPS_BLEND_MODE_MULTIPLY = 14,
        VIPS_BLEND_MODE_SCREEN = 15,
        VIPS_BLEND_MODE_OVERLAY = 16,
        VIPS_BLEND_MODE_DARKEN = 17,
        VIPS_BLEND_MODE_LIGHTEN = 18,
        VIPS_BLEND_MODE_COLOUR_DODGE = 19,
        VIPS_BLEND_MODE_COLOUR_BURN = 20,
        VIPS_BLEND_MODE_HARD_LIGHT = 21,
        VIPS_BLEND_MODE_SOFT_LIGHT = 22,
        VIPS_BLEND_MODE_DIFFERENCE = 23,
        VIPS_BLEND_MODE_EXCLUSION = 24,
        VIPS_BLEND_MODE_LAST = 25
    }
}

c_enum! {
    pub type VipsCombine {
        VIPS_COMBINE_MAX = 0,
        VIPS_COMBINE_SUM = 1,
        VIPS_COMBINE_MIN = 2,
        VIPS_COMBINE_LAST = 3
    }
}

c_enum! {
    pub type VipsTextWrap {
        VIPS_TEXT_WRAP_WORD = 0,
        VIPS_TEXT_WRAP_CHAR = 1,
        VIPS_TEXT_WRAP_WORD_CHAR = 2,
        VIPS_TEXT_WRAP_NONE = 3,
        VIPS_TEXT_WRAP_LAST = 4
    }
}

c_enum! {
    pub type VipsCombineMode {
        VIPS_COMBINE_MODE_SET = 0,
        VIPS_COMBINE_MODE_ADD = 1,
        VIPS_COMBINE_MODE_LAST = 2
    }
}

c_enum! {
    pub type VipsOperationMorphology {
        VIPS_OPERATION_MORPHOLOGY_ERODE = 0,
        VIPS_OPERATION_MORPHOLOGY_DILATE = 1,
        VIPS_OPERATION_MORPHOLOGY_LAST = 2
    }
}

c_enum! {
    pub type VipsKernel {
        VIPS_KERNEL_NEAREST = 0,
        VIPS_KERNEL_LINEAR = 1,
        VIPS_KERNEL_CUBIC = 2,
        VIPS_KERNEL_MITCHELL = 3,
        VIPS_KERNEL_LANCZOS2 = 4,
        VIPS_KERNEL_LANCZOS3 = 5,
        VIPS_KERNEL_LAST = 6
    }
}

c_enum! {
    pub type VipsSize {
        VIPS_SIZE_BOTH = 0,
        VIPS_SIZE_UP = 1,
        VIPS_SIZE_DOWN = 2,
        VIPS_SIZE_FORCE = 3,
        VIPS_SIZE_LAST = 4
    }
}

c_enum! {
    pub type VipsToken {
        VIPS_TOKEN_LEFT = 1,
        VIPS_TOKEN_RIGHT = 2,
        VIPS_TOKEN_STRING = 3,
        VIPS_TOKEN_EQUALS = 4,
        VIPS_TOKEN_COMMA = 5
    }
}

pub type VipsStartFn =
    Option<unsafe extern "C" fn(out: *mut VipsImage, a: *mut c_void, b: *mut c_void) -> *mut c_void>;
pub type VipsGenerateFn = Option<
    unsafe extern "C" fn(
        out: *mut VipsRegion,
        seq: *mut c_void,
        a: *mut c_void,
        b: *mut c_void,
        stop: *mut glib_sys::gboolean,
    ) -> c_int,
>;
pub type VipsStopFn =
    Option<unsafe extern "C" fn(seq: *mut c_void, a: *mut c_void, b: *mut c_void) -> c_int>;
