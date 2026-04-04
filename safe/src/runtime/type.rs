use std::ffi::CString;
use std::mem::size_of;
use std::ptr;
use std::sync::OnceLock;

use gobject_sys::{G_TYPE_DOUBLE, G_TYPE_INT};
use libc::strlen;
use libc::{c_char, c_int, c_void};

use crate::abi::basic::*;
use crate::abi::image::*;
use crate::abi::object::*;
use crate::abi::operation::*;
use crate::abi::r#type::*;
use crate::abi::region::*;

unsafe fn plain_copy<T>(src: *const T) -> *mut T {
    if src.is_null() {
        return ptr::null_mut();
    }

    let dst = unsafe { glib_sys::g_malloc(size_of::<T>()) as *mut T };
    unsafe { ptr::copy_nonoverlapping(src, dst, 1) };
    dst
}

unsafe extern "C" fn thing_copy(src: glib_sys::gpointer) -> glib_sys::gpointer {
    unsafe { plain_copy(src.cast::<VipsThing>()) as glib_sys::gpointer }
}

unsafe extern "C" fn thing_free(src: glib_sys::gpointer) {
    if !src.is_null() {
        unsafe { glib_sys::g_free(src) };
    }
}

unsafe extern "C" fn save_string_copy(src: glib_sys::gpointer) -> glib_sys::gpointer {
    let src = src.cast::<VipsSaveString>();
    if src.is_null() {
        return ptr::null_mut();
    }

    let dst = unsafe { glib_sys::g_malloc0(size_of::<VipsSaveString>()) as *mut VipsSaveString };
    unsafe {
        (*dst).s = if (*src).s.is_null() {
            ptr::null_mut()
        } else {
            glib_sys::g_strdup((*src).s)
        };
    }
    dst.cast()
}

unsafe extern "C" fn save_string_free(src: glib_sys::gpointer) {
    let src = src.cast::<VipsSaveString>();
    if src.is_null() {
        return;
    }

    unsafe {
        glib_sys::g_free((*src).s.cast());
        glib_sys::g_free(src.cast());
    }
}

unsafe fn area_copy_impl(area: *mut VipsArea) -> *mut VipsArea {
    if !area.is_null() {
        unsafe {
            if !(*area).lock.is_null() {
                glib_sys::g_mutex_lock((*area).lock);
            }
            (*area).count += 1;
            if !(*area).lock.is_null() {
                glib_sys::g_mutex_unlock((*area).lock);
            }
        }
    }
    area
}

unsafe fn area_free_impl(area: *mut VipsArea) {
    if area.is_null() {
        return;
    }

    unsafe {
        if !(*area).lock.is_null() {
            glib_sys::g_mutex_lock((*area).lock);
        }
        (*area).count -= 1;
        if (*area).count > 0 {
            if !(*area).lock.is_null() {
                glib_sys::g_mutex_unlock((*area).lock);
            }
            return;
        }

        if let Some(free_fn) = (*area).free_fn {
            if !(*area).data.is_null() {
                let _ = free_fn((*area).data, area.cast::<c_void>());
            }
        }

        (*area).data = ptr::null_mut();
        if !(*area).lock.is_null() {
            glib_sys::g_mutex_unlock((*area).lock);
            glib_sys::g_mutex_clear((*area).lock);
            glib_sys::g_free((*area).lock.cast());
            (*area).lock = ptr::null_mut();
        }
        glib_sys::g_free(area.cast());
    }
}

unsafe extern "C" fn area_copy(src: glib_sys::gpointer) -> glib_sys::gpointer {
    unsafe { area_copy_impl(src.cast::<VipsArea>()) as glib_sys::gpointer }
}

unsafe extern "C" fn area_free(src: glib_sys::gpointer) {
    unsafe { area_free_impl(src.cast::<VipsArea>()) };
}

fn register_boxed_type(
    name: &'static [u8],
    copy_fn: gobject_sys::GBoxedCopyFunc,
    free_fn: gobject_sys::GBoxedFreeFunc,
) -> glib_sys::GType {
    unsafe {
        gobject_sys::g_boxed_type_register_static(name.as_ptr().cast::<c_char>(), copy_fn, free_fn)
    }
}

fn register_enum_type(
    name: &'static [u8],
    values: &[(c_int, &'static [u8], &'static [u8])],
) -> glib_sys::GType {
    let prefix = enum_prefix(values);
    let mut raw = Vec::with_capacity(values.len() + 1);
    for (value, value_name, _value_nick) in values {
        let nick = leak_enum_nick(enum_nick_name(value_name, &prefix));
        raw.push(gobject_sys::GEnumValue {
            value: *value,
            value_name: value_name.as_ptr().cast::<c_char>(),
            value_nick: nick,
        });
    }
    raw.push(gobject_sys::GEnumValue {
        value: 0,
        value_name: ptr::null(),
        value_nick: ptr::null(),
    });

    let raw = Box::leak(raw.into_boxed_slice());
    unsafe { gobject_sys::g_enum_register_static(name.as_ptr().cast::<c_char>(), raw.as_ptr()) }
}

fn register_flags_type(
    name: &'static [u8],
    values: &[(c_int, &'static [u8], &'static [u8])],
) -> glib_sys::GType {
    let prefix = enum_prefix(values);
    let mut raw = Vec::with_capacity(values.len() + 1);
    for (value, value_name, _value_nick) in values {
        let nick = leak_enum_nick(enum_nick_name(value_name, &prefix));
        raw.push(gobject_sys::GFlagsValue {
            value: *value as u32,
            value_name: value_name.as_ptr().cast::<c_char>(),
            value_nick: nick,
        });
    }
    raw.push(gobject_sys::GFlagsValue {
        value: 0,
        value_name: ptr::null(),
        value_nick: ptr::null(),
    });

    let raw = Box::leak(raw.into_boxed_slice());
    unsafe { gobject_sys::g_flags_register_static(name.as_ptr().cast::<c_char>(), raw.as_ptr()) }
}

fn strip_nul(bytes: &'static [u8]) -> &'static str {
    let bytes = bytes.strip_suffix(b"\0").unwrap_or(bytes);
    std::str::from_utf8(bytes).expect("enum name utf8")
}

fn enum_prefix(values: &[(c_int, &'static [u8], &'static [u8])]) -> String {
    let Some((_, first, _)) = values.first() else {
        return String::new();
    };
    let mut prefix = strip_nul(first).to_owned();
    for (_, value_name, _) in values.iter().skip(1) {
        let value_name = strip_nul(value_name);
        while !value_name.starts_with(&prefix) {
            if prefix.pop().is_none() {
                return String::new();
            }
        }
    }
    if let Some(index) = prefix.rfind('_') {
        prefix.truncate(index + 1);
        prefix
    } else {
        String::new()
    }
}

fn enum_nick_name(value_name: &'static [u8], prefix: &str) -> String {
    let value_name = strip_nul(value_name);
    value_name
        .strip_prefix(prefix)
        .unwrap_or(value_name)
        .to_ascii_lowercase()
        .replace('_', "-")
}

fn leak_enum_nick(nick: String) -> *mut c_char {
    CString::new(nick).expect("enum nick").into_raw()
}

unsafe fn init_value_if_needed(value: *mut gobject_sys::GValue, gtype: glib_sys::GType) {
    if !value.is_null() && unsafe { (*value).g_type } == 0 {
        unsafe {
            gobject_sys::g_value_init(value, gtype);
        }
    }
}

macro_rules! enum_getter {
    ($fn_name:ident, $type_name:literal, [$($value:ident),+ $(,)?]) => {
        #[no_mangle]
        pub extern "C" fn $fn_name() -> glib_sys::GType {
            static ONCE: OnceLock<glib_sys::GType> = OnceLock::new();
            *ONCE.get_or_init(|| {
                register_enum_type(
                    concat!($type_name, "\0").as_bytes(),
                    &[
                        $((
                            $value as c_int,
                            concat!(stringify!($value), "\0").as_bytes(),
                            concat!(stringify!($value), "\0").as_bytes(),
                        )),+
                    ],
                )
            })
        }
    };
}

macro_rules! flags_getter {
    ($fn_name:ident, $type_name:literal, [$($value:ident),+ $(,)?]) => {
        #[no_mangle]
        pub extern "C" fn $fn_name() -> glib_sys::GType {
            static ONCE: OnceLock<glib_sys::GType> = OnceLock::new();
            *ONCE.get_or_init(|| {
                register_flags_type(
                    concat!($type_name, "\0").as_bytes(),
                    &[
                        $((
                            $value as c_int,
                            concat!(stringify!($value), "\0").as_bytes(),
                            concat!(stringify!($value), "\0").as_bytes(),
                        )),+
                    ],
                )
            })
        }
    };
}

macro_rules! boxed_getter {
    ($fn_name:ident, $type_name:literal, $copy:path, $free:path) => {
        #[no_mangle]
        pub extern "C" fn $fn_name() -> glib_sys::GType {
            static ONCE: OnceLock<glib_sys::GType> = OnceLock::new();
            *ONCE.get_or_init(|| {
                register_boxed_type(
                    concat!($type_name, "\0").as_bytes(),
                    Some($copy),
                    Some($free),
                )
            })
        }
    };
}

boxed_getter!(vips_thing_get_type, "VipsThing", thing_copy, thing_free);
boxed_getter!(vips_area_get_type, "VipsArea", area_copy, area_free);
boxed_getter!(
    vips_save_string_get_type,
    "VipsSaveString",
    save_string_copy,
    save_string_free
);
boxed_getter!(
    vips_ref_string_get_type,
    "VipsRefString",
    area_copy,
    area_free
);
boxed_getter!(vips_blob_get_type, "VipsBlob", area_copy, area_free);
boxed_getter!(
    vips_array_double_get_type,
    "VipsArrayDouble",
    area_copy,
    area_free
);
boxed_getter!(
    vips_array_int_get_type,
    "VipsArrayInt",
    area_copy,
    area_free
);
boxed_getter!(
    vips_array_image_get_type,
    "VipsArrayImage",
    area_copy,
    area_free
);

enum_getter!(
    vips_operation_math_get_type,
    "VipsOperationMath",
    [
        VIPS_OPERATION_MATH_SIN,
        VIPS_OPERATION_MATH_COS,
        VIPS_OPERATION_MATH_TAN,
        VIPS_OPERATION_MATH_ASIN,
        VIPS_OPERATION_MATH_ACOS,
        VIPS_OPERATION_MATH_ATAN,
        VIPS_OPERATION_MATH_LOG,
        VIPS_OPERATION_MATH_LOG10,
        VIPS_OPERATION_MATH_EXP,
        VIPS_OPERATION_MATH_EXP10,
        VIPS_OPERATION_MATH_SINH,
        VIPS_OPERATION_MATH_COSH,
        VIPS_OPERATION_MATH_TANH,
        VIPS_OPERATION_MATH_ASINH,
        VIPS_OPERATION_MATH_ACOSH,
        VIPS_OPERATION_MATH_ATANH,
        VIPS_OPERATION_MATH_LAST
    ]
);
enum_getter!(
    vips_operation_math2_get_type,
    "VipsOperationMath2",
    [
        VIPS_OPERATION_MATH2_POW,
        VIPS_OPERATION_MATH2_WOP,
        VIPS_OPERATION_MATH2_ATAN2,
        VIPS_OPERATION_MATH2_LAST
    ]
);
enum_getter!(
    vips_operation_round_get_type,
    "VipsOperationRound",
    [
        VIPS_OPERATION_ROUND_RINT,
        VIPS_OPERATION_ROUND_CEIL,
        VIPS_OPERATION_ROUND_FLOOR,
        VIPS_OPERATION_ROUND_LAST
    ]
);
enum_getter!(
    vips_operation_relational_get_type,
    "VipsOperationRelational",
    [
        VIPS_OPERATION_RELATIONAL_EQUAL,
        VIPS_OPERATION_RELATIONAL_NOTEQ,
        VIPS_OPERATION_RELATIONAL_LESS,
        VIPS_OPERATION_RELATIONAL_LESSEQ,
        VIPS_OPERATION_RELATIONAL_MORE,
        VIPS_OPERATION_RELATIONAL_MOREEQ,
        VIPS_OPERATION_RELATIONAL_LAST
    ]
);
enum_getter!(
    vips_operation_boolean_get_type,
    "VipsOperationBoolean",
    [
        VIPS_OPERATION_BOOLEAN_AND,
        VIPS_OPERATION_BOOLEAN_OR,
        VIPS_OPERATION_BOOLEAN_EOR,
        VIPS_OPERATION_BOOLEAN_LSHIFT,
        VIPS_OPERATION_BOOLEAN_RSHIFT,
        VIPS_OPERATION_BOOLEAN_LAST
    ]
);
enum_getter!(
    vips_operation_complex_get_type,
    "VipsOperationComplex",
    [
        VIPS_OPERATION_COMPLEX_POLAR,
        VIPS_OPERATION_COMPLEX_RECT,
        VIPS_OPERATION_COMPLEX_CONJ,
        VIPS_OPERATION_COMPLEX_LAST
    ]
);
enum_getter!(
    vips_operation_complex2_get_type,
    "VipsOperationComplex2",
    [
        VIPS_OPERATION_COMPLEX2_CROSS_PHASE,
        VIPS_OPERATION_COMPLEX2_LAST
    ]
);
enum_getter!(
    vips_operation_complexget_get_type,
    "VipsOperationComplexget",
    [
        VIPS_OPERATION_COMPLEXGET_REAL,
        VIPS_OPERATION_COMPLEXGET_IMAG,
        VIPS_OPERATION_COMPLEXGET_LAST
    ]
);
enum_getter!(
    vips_precision_get_type,
    "VipsPrecision",
    [
        VIPS_PRECISION_INTEGER,
        VIPS_PRECISION_FLOAT,
        VIPS_PRECISION_APPROXIMATE,
        VIPS_PRECISION_LAST
    ]
);
enum_getter!(
    vips_intent_get_type,
    "VipsIntent",
    [
        VIPS_INTENT_PERCEPTUAL,
        VIPS_INTENT_RELATIVE,
        VIPS_INTENT_SATURATION,
        VIPS_INTENT_ABSOLUTE,
        VIPS_INTENT_LAST
    ]
);
enum_getter!(
    vips_pcs_get_type,
    "VipsPCS",
    [VIPS_PCS_LAB, VIPS_PCS_XYZ, VIPS_PCS_LAST]
);
enum_getter!(
    vips_extend_get_type,
    "VipsExtend",
    [
        VIPS_EXTEND_BLACK,
        VIPS_EXTEND_COPY,
        VIPS_EXTEND_REPEAT,
        VIPS_EXTEND_MIRROR,
        VIPS_EXTEND_WHITE,
        VIPS_EXTEND_BACKGROUND,
        VIPS_EXTEND_LAST
    ]
);
enum_getter!(
    vips_compass_direction_get_type,
    "VipsCompassDirection",
    [
        VIPS_COMPASS_DIRECTION_CENTRE,
        VIPS_COMPASS_DIRECTION_NORTH,
        VIPS_COMPASS_DIRECTION_EAST,
        VIPS_COMPASS_DIRECTION_SOUTH,
        VIPS_COMPASS_DIRECTION_WEST,
        VIPS_COMPASS_DIRECTION_NORTH_EAST,
        VIPS_COMPASS_DIRECTION_SOUTH_EAST,
        VIPS_COMPASS_DIRECTION_SOUTH_WEST,
        VIPS_COMPASS_DIRECTION_NORTH_WEST,
        VIPS_COMPASS_DIRECTION_LAST
    ]
);
enum_getter!(
    vips_direction_get_type,
    "VipsDirection",
    [
        VIPS_DIRECTION_HORIZONTAL,
        VIPS_DIRECTION_VERTICAL,
        VIPS_DIRECTION_LAST
    ]
);
enum_getter!(
    vips_align_get_type,
    "VipsAlign",
    [
        VIPS_ALIGN_LOW,
        VIPS_ALIGN_CENTRE,
        VIPS_ALIGN_HIGH,
        VIPS_ALIGN_LAST
    ]
);
enum_getter!(
    vips_angle_get_type,
    "VipsAngle",
    [
        VIPS_ANGLE_D0,
        VIPS_ANGLE_D90,
        VIPS_ANGLE_D180,
        VIPS_ANGLE_D270,
        VIPS_ANGLE_LAST
    ]
);
enum_getter!(
    vips_angle45_get_type,
    "VipsAngle45",
    [
        VIPS_ANGLE45_D0,
        VIPS_ANGLE45_D45,
        VIPS_ANGLE45_D90,
        VIPS_ANGLE45_D135,
        VIPS_ANGLE45_D180,
        VIPS_ANGLE45_D225,
        VIPS_ANGLE45_D270,
        VIPS_ANGLE45_D315,
        VIPS_ANGLE45_LAST
    ]
);
enum_getter!(
    vips_interesting_get_type,
    "VipsInteresting",
    [
        VIPS_INTERESTING_NONE,
        VIPS_INTERESTING_CENTRE,
        VIPS_INTERESTING_ENTROPY,
        VIPS_INTERESTING_ATTENTION,
        VIPS_INTERESTING_LOW,
        VIPS_INTERESTING_HIGH,
        VIPS_INTERESTING_ALL,
        VIPS_INTERESTING_LAST
    ]
);
enum_getter!(
    vips_blend_mode_get_type,
    "VipsBlendMode",
    [
        VIPS_BLEND_MODE_CLEAR,
        VIPS_BLEND_MODE_SOURCE,
        VIPS_BLEND_MODE_OVER,
        VIPS_BLEND_MODE_IN,
        VIPS_BLEND_MODE_OUT,
        VIPS_BLEND_MODE_ATOP,
        VIPS_BLEND_MODE_DEST,
        VIPS_BLEND_MODE_DEST_OVER,
        VIPS_BLEND_MODE_DEST_IN,
        VIPS_BLEND_MODE_DEST_OUT,
        VIPS_BLEND_MODE_DEST_ATOP,
        VIPS_BLEND_MODE_XOR,
        VIPS_BLEND_MODE_ADD,
        VIPS_BLEND_MODE_SATURATE,
        VIPS_BLEND_MODE_MULTIPLY,
        VIPS_BLEND_MODE_SCREEN,
        VIPS_BLEND_MODE_OVERLAY,
        VIPS_BLEND_MODE_DARKEN,
        VIPS_BLEND_MODE_LIGHTEN,
        VIPS_BLEND_MODE_COLOUR_DODGE,
        VIPS_BLEND_MODE_COLOUR_BURN,
        VIPS_BLEND_MODE_HARD_LIGHT,
        VIPS_BLEND_MODE_SOFT_LIGHT,
        VIPS_BLEND_MODE_DIFFERENCE,
        VIPS_BLEND_MODE_EXCLUSION,
        VIPS_BLEND_MODE_LAST
    ]
);
enum_getter!(
    vips_combine_get_type,
    "VipsCombine",
    [
        VIPS_COMBINE_MAX,
        VIPS_COMBINE_SUM,
        VIPS_COMBINE_MIN,
        VIPS_COMBINE_LAST
    ]
);
enum_getter!(
    vips_text_wrap_get_type,
    "VipsTextWrap",
    [
        VIPS_TEXT_WRAP_WORD,
        VIPS_TEXT_WRAP_CHAR,
        VIPS_TEXT_WRAP_WORD_CHAR,
        VIPS_TEXT_WRAP_NONE,
        VIPS_TEXT_WRAP_LAST
    ]
);
enum_getter!(
    vips_combine_mode_get_type,
    "VipsCombineMode",
    [
        VIPS_COMBINE_MODE_SET,
        VIPS_COMBINE_MODE_ADD,
        VIPS_COMBINE_MODE_LAST
    ]
);
flags_getter!(
    vips_foreign_flags_get_type,
    "VipsForeignFlags",
    [
        VIPS_FOREIGN_NONE,
        VIPS_FOREIGN_PARTIAL,
        VIPS_FOREIGN_BIGENDIAN,
        VIPS_FOREIGN_SEQUENTIAL,
        VIPS_FOREIGN_ALL
    ]
);
enum_getter!(
    vips_fail_on_get_type,
    "VipsFailOn",
    [
        VIPS_FAIL_ON_NONE,
        VIPS_FAIL_ON_TRUNCATED,
        VIPS_FAIL_ON_ERROR,
        VIPS_FAIL_ON_WARNING,
        VIPS_FAIL_ON_LAST
    ]
);
enum_getter!(
    vips_saveable_get_type,
    "VipsSaveable",
    [
        VIPS_SAVEABLE_MONO,
        VIPS_SAVEABLE_RGB,
        VIPS_SAVEABLE_RGBA,
        VIPS_SAVEABLE_RGBA_ONLY,
        VIPS_SAVEABLE_RGB_CMYK,
        VIPS_SAVEABLE_ANY,
        VIPS_SAVEABLE_LAST
    ]
);
flags_getter!(
    vips_foreign_keep_get_type,
    "VipsForeignKeep",
    [
        VIPS_FOREIGN_KEEP_NONE,
        VIPS_FOREIGN_KEEP_EXIF,
        VIPS_FOREIGN_KEEP_XMP,
        VIPS_FOREIGN_KEEP_IPTC,
        VIPS_FOREIGN_KEEP_ICC,
        VIPS_FOREIGN_KEEP_OTHER,
        VIPS_FOREIGN_KEEP_ALL
    ]
);
enum_getter!(
    vips_foreign_subsample_get_type,
    "VipsForeignSubsample",
    [
        VIPS_FOREIGN_SUBSAMPLE_AUTO,
        VIPS_FOREIGN_SUBSAMPLE_ON,
        VIPS_FOREIGN_SUBSAMPLE_OFF,
        VIPS_FOREIGN_SUBSAMPLE_LAST
    ]
);
enum_getter!(
    vips_foreign_jpeg_subsample_get_type,
    "VipsForeignJpegSubsample",
    [
        VIPS_FOREIGN_JPEG_SUBSAMPLE_AUTO,
        VIPS_FOREIGN_JPEG_SUBSAMPLE_ON,
        VIPS_FOREIGN_JPEG_SUBSAMPLE_OFF,
        VIPS_FOREIGN_JPEG_SUBSAMPLE_LAST
    ]
);
enum_getter!(
    vips_foreign_webp_preset_get_type,
    "VipsForeignWebpPreset",
    [
        VIPS_FOREIGN_WEBP_PRESET_DEFAULT,
        VIPS_FOREIGN_WEBP_PRESET_PICTURE,
        VIPS_FOREIGN_WEBP_PRESET_PHOTO,
        VIPS_FOREIGN_WEBP_PRESET_DRAWING,
        VIPS_FOREIGN_WEBP_PRESET_ICON,
        VIPS_FOREIGN_WEBP_PRESET_TEXT,
        VIPS_FOREIGN_WEBP_PRESET_LAST
    ]
);
enum_getter!(
    vips_foreign_tiff_compression_get_type,
    "VipsForeignTiffCompression",
    [
        VIPS_FOREIGN_TIFF_COMPRESSION_NONE,
        VIPS_FOREIGN_TIFF_COMPRESSION_JPEG,
        VIPS_FOREIGN_TIFF_COMPRESSION_DEFLATE,
        VIPS_FOREIGN_TIFF_COMPRESSION_PACKBITS,
        VIPS_FOREIGN_TIFF_COMPRESSION_CCITTFAX4,
        VIPS_FOREIGN_TIFF_COMPRESSION_LZW,
        VIPS_FOREIGN_TIFF_COMPRESSION_WEBP,
        VIPS_FOREIGN_TIFF_COMPRESSION_ZSTD,
        VIPS_FOREIGN_TIFF_COMPRESSION_JP2K,
        VIPS_FOREIGN_TIFF_COMPRESSION_LAST
    ]
);
enum_getter!(
    vips_foreign_tiff_predictor_get_type,
    "VipsForeignTiffPredictor",
    [
        VIPS_FOREIGN_TIFF_PREDICTOR_NONE,
        VIPS_FOREIGN_TIFF_PREDICTOR_HORIZONTAL,
        VIPS_FOREIGN_TIFF_PREDICTOR_FLOAT,
        VIPS_FOREIGN_TIFF_PREDICTOR_LAST
    ]
);
enum_getter!(
    vips_foreign_tiff_resunit_get_type,
    "VipsForeignTiffResunit",
    [
        VIPS_FOREIGN_TIFF_RESUNIT_CM,
        VIPS_FOREIGN_TIFF_RESUNIT_INCH,
        VIPS_FOREIGN_TIFF_RESUNIT_LAST
    ]
);
flags_getter!(
    vips_foreign_png_filter_get_type,
    "VipsForeignPngFilter",
    [
        VIPS_FOREIGN_PNG_FILTER_NONE,
        VIPS_FOREIGN_PNG_FILTER_SUB,
        VIPS_FOREIGN_PNG_FILTER_UP,
        VIPS_FOREIGN_PNG_FILTER_AVG,
        VIPS_FOREIGN_PNG_FILTER_PAETH,
        VIPS_FOREIGN_PNG_FILTER_ALL
    ]
);
enum_getter!(
    vips_foreign_ppm_format_get_type,
    "VipsForeignPpmFormat",
    [
        VIPS_FOREIGN_PPM_FORMAT_PBM,
        VIPS_FOREIGN_PPM_FORMAT_PGM,
        VIPS_FOREIGN_PPM_FORMAT_PPM,
        VIPS_FOREIGN_PPM_FORMAT_PFM,
        VIPS_FOREIGN_PPM_FORMAT_PNM,
        VIPS_FOREIGN_PPM_FORMAT_LAST
    ]
);
enum_getter!(
    vips_foreign_dz_layout_get_type,
    "VipsForeignDzLayout",
    [
        VIPS_FOREIGN_DZ_LAYOUT_DZ,
        VIPS_FOREIGN_DZ_LAYOUT_ZOOMIFY,
        VIPS_FOREIGN_DZ_LAYOUT_GOOGLE,
        VIPS_FOREIGN_DZ_LAYOUT_IIIF,
        VIPS_FOREIGN_DZ_LAYOUT_IIIF3,
        VIPS_FOREIGN_DZ_LAYOUT_LAST
    ]
);
enum_getter!(
    vips_foreign_dz_depth_get_type,
    "VipsForeignDzDepth",
    [
        VIPS_FOREIGN_DZ_DEPTH_ONEPIXEL,
        VIPS_FOREIGN_DZ_DEPTH_ONETILE,
        VIPS_FOREIGN_DZ_DEPTH_ONE,
        VIPS_FOREIGN_DZ_DEPTH_LAST
    ]
);
enum_getter!(
    vips_foreign_dz_container_get_type,
    "VipsForeignDzContainer",
    [
        VIPS_FOREIGN_DZ_CONTAINER_FS,
        VIPS_FOREIGN_DZ_CONTAINER_ZIP,
        VIPS_FOREIGN_DZ_CONTAINER_SZI,
        VIPS_FOREIGN_DZ_CONTAINER_LAST
    ]
);
enum_getter!(
    vips_foreign_heif_compression_get_type,
    "VipsForeignHeifCompression",
    [
        VIPS_FOREIGN_HEIF_COMPRESSION_HEVC,
        VIPS_FOREIGN_HEIF_COMPRESSION_AVC,
        VIPS_FOREIGN_HEIF_COMPRESSION_JPEG,
        VIPS_FOREIGN_HEIF_COMPRESSION_AV1,
        VIPS_FOREIGN_HEIF_COMPRESSION_LAST
    ]
);
enum_getter!(
    vips_foreign_heif_encoder_get_type,
    "VipsForeignHeifEncoder",
    [
        VIPS_FOREIGN_HEIF_ENCODER_AUTO,
        VIPS_FOREIGN_HEIF_ENCODER_AOM,
        VIPS_FOREIGN_HEIF_ENCODER_RAV1E,
        VIPS_FOREIGN_HEIF_ENCODER_SVT,
        VIPS_FOREIGN_HEIF_ENCODER_X265,
        VIPS_FOREIGN_HEIF_ENCODER_LAST
    ]
);
enum_getter!(
    vips_demand_style_get_type,
    "VipsDemandStyle",
    [
        VIPS_DEMAND_STYLE_ERROR,
        VIPS_DEMAND_STYLE_SMALLTILE,
        VIPS_DEMAND_STYLE_FATSTRIP,
        VIPS_DEMAND_STYLE_THINSTRIP,
        VIPS_DEMAND_STYLE_ANY
    ]
);
enum_getter!(
    vips_image_type_get_type,
    "VipsImageType",
    [
        VIPS_IMAGE_ERROR,
        VIPS_IMAGE_NONE,
        VIPS_IMAGE_SETBUF,
        VIPS_IMAGE_SETBUF_FOREIGN,
        VIPS_IMAGE_OPENIN,
        VIPS_IMAGE_MMAPIN,
        VIPS_IMAGE_MMAPINRW,
        VIPS_IMAGE_OPENOUT,
        VIPS_IMAGE_PARTIAL
    ]
);
enum_getter!(
    vips_interpretation_get_type,
    "VipsInterpretation",
    [
        VIPS_INTERPRETATION_ERROR,
        VIPS_INTERPRETATION_MULTIBAND,
        VIPS_INTERPRETATION_B_W,
        VIPS_INTERPRETATION_HISTOGRAM,
        VIPS_INTERPRETATION_XYZ,
        VIPS_INTERPRETATION_LAB,
        VIPS_INTERPRETATION_CMYK,
        VIPS_INTERPRETATION_LABQ,
        VIPS_INTERPRETATION_RGB,
        VIPS_INTERPRETATION_CMC,
        VIPS_INTERPRETATION_LCH,
        VIPS_INTERPRETATION_LABS,
        VIPS_INTERPRETATION_sRGB,
        VIPS_INTERPRETATION_YXY,
        VIPS_INTERPRETATION_FOURIER,
        VIPS_INTERPRETATION_RGB16,
        VIPS_INTERPRETATION_GREY16,
        VIPS_INTERPRETATION_MATRIX,
        VIPS_INTERPRETATION_scRGB,
        VIPS_INTERPRETATION_HSV,
        VIPS_INTERPRETATION_LAST
    ]
);
enum_getter!(
    vips_band_format_get_type,
    "VipsBandFormat",
    [
        VIPS_FORMAT_NOTSET,
        VIPS_FORMAT_UCHAR,
        VIPS_FORMAT_CHAR,
        VIPS_FORMAT_USHORT,
        VIPS_FORMAT_SHORT,
        VIPS_FORMAT_UINT,
        VIPS_FORMAT_INT,
        VIPS_FORMAT_FLOAT,
        VIPS_FORMAT_COMPLEX,
        VIPS_FORMAT_DOUBLE,
        VIPS_FORMAT_DPCOMPLEX,
        VIPS_FORMAT_LAST
    ]
);
enum_getter!(
    vips_coding_get_type,
    "VipsCoding",
    [
        VIPS_CODING_ERROR,
        VIPS_CODING_NONE,
        VIPS_CODING_LABQ,
        VIPS_CODING_RAD,
        VIPS_CODING_LAST
    ]
);
enum_getter!(
    vips_access_get_type,
    "VipsAccess",
    [
        VIPS_ACCESS_RANDOM,
        VIPS_ACCESS_SEQUENTIAL,
        VIPS_ACCESS_SEQUENTIAL_UNBUFFERED,
        VIPS_ACCESS_LAST
    ]
);
enum_getter!(
    vips_operation_morphology_get_type,
    "VipsOperationMorphology",
    [
        VIPS_OPERATION_MORPHOLOGY_ERODE,
        VIPS_OPERATION_MORPHOLOGY_DILATE,
        VIPS_OPERATION_MORPHOLOGY_LAST
    ]
);
flags_getter!(
    vips_argument_flags_get_type,
    "VipsArgumentFlags",
    [
        VIPS_ARGUMENT_NONE,
        VIPS_ARGUMENT_REQUIRED,
        VIPS_ARGUMENT_CONSTRUCT,
        VIPS_ARGUMENT_SET_ONCE,
        VIPS_ARGUMENT_SET_ALWAYS,
        VIPS_ARGUMENT_INPUT,
        VIPS_ARGUMENT_OUTPUT,
        VIPS_ARGUMENT_DEPRECATED,
        VIPS_ARGUMENT_MODIFY,
        VIPS_ARGUMENT_NON_HASHABLE
    ]
);
flags_getter!(
    vips_operation_flags_get_type,
    "VipsOperationFlags",
    [
        VIPS_OPERATION_NONE,
        VIPS_OPERATION_SEQUENTIAL,
        VIPS_OPERATION_SEQUENTIAL_UNBUFFERED,
        VIPS_OPERATION_NOCACHE,
        VIPS_OPERATION_DEPRECATED,
        VIPS_OPERATION_UNTRUSTED,
        VIPS_OPERATION_BLOCKED,
        VIPS_OPERATION_REVALIDATE
    ]
);
enum_getter!(
    vips_region_shrink_get_type,
    "VipsRegionShrink",
    [
        VIPS_REGION_SHRINK_MEAN,
        VIPS_REGION_SHRINK_MEDIAN,
        VIPS_REGION_SHRINK_MODE,
        VIPS_REGION_SHRINK_MAX,
        VIPS_REGION_SHRINK_MIN,
        VIPS_REGION_SHRINK_NEAREST,
        VIPS_REGION_SHRINK_LAST
    ]
);
enum_getter!(
    vips_kernel_get_type,
    "VipsKernel",
    [
        VIPS_KERNEL_NEAREST,
        VIPS_KERNEL_LINEAR,
        VIPS_KERNEL_CUBIC,
        VIPS_KERNEL_MITCHELL,
        VIPS_KERNEL_LANCZOS2,
        VIPS_KERNEL_LANCZOS3,
        VIPS_KERNEL_LAST
    ]
);
enum_getter!(
    vips_size_get_type,
    "VipsSize",
    [
        VIPS_SIZE_BOTH,
        VIPS_SIZE_UP,
        VIPS_SIZE_DOWN,
        VIPS_SIZE_FORCE,
        VIPS_SIZE_LAST
    ]
);
enum_getter!(
    vips_token_get_type,
    "VipsToken",
    [
        VIPS_TOKEN_LEFT,
        VIPS_TOKEN_RIGHT,
        VIPS_TOKEN_STRING,
        VIPS_TOKEN_EQUALS,
        VIPS_TOKEN_COMMA
    ]
);

pub(crate) fn ensure_types() {
    let _ = vips_thing_get_type();
    let _ = vips_area_get_type();
    let _ = vips_save_string_get_type();
    let _ = vips_ref_string_get_type();
    let _ = vips_blob_get_type();
    let _ = vips_array_double_get_type();
    let _ = vips_array_int_get_type();
    let _ = vips_array_image_get_type();
    let _ = vips_operation_math_get_type();
    let _ = vips_operation_math2_get_type();
    let _ = vips_operation_round_get_type();
    let _ = vips_operation_relational_get_type();
    let _ = vips_operation_boolean_get_type();
    let _ = vips_operation_complex_get_type();
    let _ = vips_operation_complex2_get_type();
    let _ = vips_operation_complexget_get_type();
    let _ = vips_precision_get_type();
    let _ = vips_intent_get_type();
    let _ = vips_pcs_get_type();
    let _ = vips_extend_get_type();
    let _ = vips_compass_direction_get_type();
    let _ = vips_direction_get_type();
    let _ = vips_align_get_type();
    let _ = vips_angle_get_type();
    let _ = vips_angle45_get_type();
    let _ = vips_interesting_get_type();
    let _ = vips_blend_mode_get_type();
    let _ = vips_combine_get_type();
    let _ = vips_text_wrap_get_type();
    let _ = vips_combine_mode_get_type();
    let _ = vips_foreign_flags_get_type();
    let _ = vips_fail_on_get_type();
    let _ = vips_saveable_get_type();
    let _ = vips_foreign_keep_get_type();
    let _ = vips_foreign_subsample_get_type();
    let _ = vips_foreign_jpeg_subsample_get_type();
    let _ = vips_foreign_webp_preset_get_type();
    let _ = vips_foreign_tiff_compression_get_type();
    let _ = vips_foreign_tiff_predictor_get_type();
    let _ = vips_foreign_tiff_resunit_get_type();
    let _ = vips_foreign_png_filter_get_type();
    let _ = vips_foreign_ppm_format_get_type();
    let _ = vips_foreign_dz_layout_get_type();
    let _ = vips_foreign_dz_depth_get_type();
    let _ = vips_foreign_dz_container_get_type();
    let _ = vips_foreign_heif_compression_get_type();
    let _ = vips_foreign_heif_encoder_get_type();
    let _ = vips_demand_style_get_type();
    let _ = vips_image_type_get_type();
    let _ = vips_interpretation_get_type();
    let _ = vips_band_format_get_type();
    let _ = vips_coding_get_type();
    let _ = vips_access_get_type();
    let _ = vips_operation_morphology_get_type();
    let _ = vips_argument_flags_get_type();
    let _ = vips_operation_flags_get_type();
    let _ = vips_region_shrink_get_type();
    let _ = vips_kernel_get_type();
    let _ = vips_size_get_type();
    let _ = vips_token_get_type();
}

unsafe fn allocate_lock() -> *mut glib_sys::GMutex {
    let lock =
        unsafe { glib_sys::g_malloc0(size_of::<glib_sys::GMutex>()) }.cast::<glib_sys::GMutex>();
    if !lock.is_null() {
        unsafe {
            glib_sys::g_mutex_init(lock);
        }
    }
    lock
}

#[no_mangle]
pub extern "C" fn vips_thing_new(i: c_int) -> *mut VipsThing {
    let thing = unsafe { glib_sys::g_malloc(size_of::<VipsThing>()) }.cast::<VipsThing>();
    if let Some(thing) = unsafe { thing.as_mut() } {
        thing.i = i;
    }
    thing
}

#[no_mangle]
pub extern "C" fn vips_area_copy(area: *mut VipsArea) -> *mut VipsArea {
    unsafe { area_copy_impl(area) }
}

#[no_mangle]
pub extern "C" fn vips_area_free_cb(mem: *mut c_void, _area: *mut c_void) -> c_int {
    if !mem.is_null() {
        unsafe {
            glib_sys::g_free(mem);
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn vips_area_unref(area: *mut VipsArea) {
    unsafe { area_free_impl(area) };
}

#[no_mangle]
pub extern "C" fn vips_area_new(free_fn: VipsCallbackFn, data: *mut c_void) -> *mut VipsArea {
    let area = unsafe { glib_sys::g_malloc0(size_of::<VipsArea>()) }.cast::<VipsArea>();
    let Some(area_ref) = (unsafe { area.as_mut() }) else {
        return ptr::null_mut();
    };
    area_ref.data = data;
    area_ref.length = 0;
    area_ref.n = 0;
    area_ref.count = 1;
    area_ref.lock = unsafe { allocate_lock() };
    area_ref.free_fn = free_fn;
    area_ref.client = ptr::null_mut();
    area_ref.r#type = 0;
    area_ref.sizeof_type = 0;
    area
}

#[no_mangle]
pub extern "C" fn vips_area_new_array(
    type_: glib_sys::GType,
    sizeof_type: usize,
    n: c_int,
) -> *mut VipsArea {
    let bytes = sizeof_type.saturating_mul(n.max(0) as usize);
    let data = unsafe { glib_sys::g_malloc0(bytes) };
    let area = vips_area_new(Some(vips_area_free_cb), data);
    if let Some(area) = unsafe { area.as_mut() } {
        area.length = bytes;
        area.n = n.max(0);
        area.r#type = type_;
        area.sizeof_type = sizeof_type;
    }
    area
}

unsafe extern "C" fn free_object_array(data: *mut c_void, area: *mut c_void) -> c_int {
    let objects = data.cast::<*mut gobject_sys::GObject>();
    let area = area.cast::<VipsArea>();
    if !objects.is_null() && !area.is_null() {
        for i in 0..unsafe { (*area).n.max(0) as isize } {
            let item = unsafe { *objects.offset(i) };
            if !item.is_null() {
                unsafe {
                    gobject_sys::g_object_unref(item.cast());
                }
            }
        }
        unsafe {
            glib_sys::g_free(objects.cast());
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn vips_area_new_array_object(n: c_int) -> *mut VipsArea {
    let count = n.max(0) as usize + 1;
    let data = unsafe {
        glib_sys::g_malloc0(count.saturating_mul(size_of::<*mut gobject_sys::GObject>()))
    };
    let area = vips_area_new(Some(free_object_array), data);
    if let Some(area) = unsafe { area.as_mut() } {
        area.length = count.saturating_mul(size_of::<*mut gobject_sys::GObject>());
        area.n = n.max(0);
        area.r#type = unsafe { gobject_sys::g_object_get_type() };
        area.sizeof_type = size_of::<*mut gobject_sys::GObject>();
    }
    area
}

#[no_mangle]
pub extern "C" fn vips_area_get_data(
    area: *mut VipsArea,
    length: *mut usize,
    n: *mut c_int,
    type_: *mut glib_sys::GType,
    sizeof_type: *mut usize,
) -> *mut c_void {
    let Some(area) = (unsafe { area.as_ref() }) else {
        return ptr::null_mut();
    };
    unsafe {
        if !length.is_null() {
            *length = area.length;
        }
        if !n.is_null() {
            *n = area.n;
        }
        if !type_.is_null() {
            *type_ = area.r#type;
        }
        if !sizeof_type.is_null() {
            *sizeof_type = area.sizeof_type;
        }
    }
    area.data
}

#[no_mangle]
pub extern "C" fn vips_ref_string_new(str_: *const c_char) -> *mut VipsRefString {
    let text = if str_.is_null() {
        ptr::null_mut()
    } else {
        unsafe { glib_sys::g_strdup(str_) }.cast::<c_void>()
    };
    let area = vips_area_new(Some(vips_area_free_cb), text);
    let Some(area_ref) = (unsafe { area.as_mut() }) else {
        return ptr::null_mut();
    };
    if !text.is_null() {
        area_ref.length = unsafe { strlen(text.cast::<c_char>()) as usize };
    }
    area.cast::<VipsRefString>()
}

#[no_mangle]
pub extern "C" fn vips_ref_string_get(
    refstr: *mut VipsRefString,
    length: *mut usize,
) -> *const c_char {
    let Some(area) = (unsafe { refstr.as_ref() }) else {
        return ptr::null();
    };
    unsafe {
        if !length.is_null() {
            *length = area.area.length;
        }
    }
    area.area.data.cast::<c_char>()
}

#[no_mangle]
pub extern "C" fn vips_blob_new(
    free_fn: VipsCallbackFn,
    data: *const c_void,
    length: usize,
) -> *mut VipsBlob {
    let area = vips_area_new(free_fn, data.cast_mut());
    if let Some(area) = unsafe { area.as_mut() } {
        area.length = length;
    }
    area.cast::<VipsBlob>()
}

#[no_mangle]
pub extern "C" fn vips_blob_copy(data: *const c_void, length: usize) -> *mut VipsBlob {
    if data.is_null() && length > 0 {
        return ptr::null_mut();
    }
    let copy = if length == 0 {
        ptr::null_mut()
    } else {
        unsafe { glib_sys::g_malloc(length) }
    };
    if !copy.is_null() && !data.is_null() {
        unsafe {
            ptr::copy_nonoverlapping(data.cast::<u8>(), copy.cast::<u8>(), length);
        }
    }
    vips_blob_new(Some(vips_area_free_cb), copy, length)
}

#[no_mangle]
pub extern "C" fn vips_blob_get(blob: *mut VipsBlob, length: *mut usize) -> *const c_void {
    let Some(blob) = (unsafe { blob.as_ref() }) else {
        return ptr::null();
    };
    unsafe {
        if !length.is_null() {
            *length = blob.area.length;
        }
    }
    blob.area.data.cast::<c_void>()
}

#[no_mangle]
pub extern "C" fn vips_blob_set(
    blob: *mut VipsBlob,
    free_fn: VipsCallbackFn,
    data: *const c_void,
    length: usize,
) {
    let Some(blob) = (unsafe { blob.as_mut() }) else {
        return;
    };
    if let Some(existing) = blob.area.free_fn {
        if !blob.area.data.is_null() {
            let _ = unsafe { existing(blob.area.data, blob as *mut _ as *mut c_void) };
        }
    }
    blob.area.data = data.cast_mut();
    blob.area.length = length;
    blob.area.free_fn = free_fn;
}

#[no_mangle]
pub extern "C" fn vips_array_double_new(array: *const f64, n: c_int) -> *mut VipsArrayDouble {
    let area = vips_area_new_array(G_TYPE_DOUBLE, size_of::<f64>(), n);
    if let Some(area) = unsafe { area.as_mut() } {
        if !array.is_null() && n > 0 {
            unsafe {
                ptr::copy_nonoverlapping(
                    array.cast::<u8>(),
                    area.data.cast::<u8>(),
                    n as usize * size_of::<f64>(),
                );
            }
        }
    }
    area.cast::<VipsArrayDouble>()
}

#[no_mangle]
pub extern "C" fn vips_array_double_get(array: *mut VipsArrayDouble, n: *mut c_int) -> *mut f64 {
    let Some(array) = (unsafe { array.as_ref() }) else {
        return ptr::null_mut();
    };
    unsafe {
        if !n.is_null() {
            *n = array.area.n;
        }
    }
    array.area.data.cast::<f64>()
}

#[no_mangle]
pub extern "C" fn vips_array_int_new(array: *const c_int, n: c_int) -> *mut VipsArrayInt {
    let area = vips_area_new_array(G_TYPE_INT, size_of::<c_int>(), n);
    if let Some(area) = unsafe { area.as_mut() } {
        if !array.is_null() && n > 0 {
            unsafe {
                ptr::copy_nonoverlapping(
                    array.cast::<u8>(),
                    area.data.cast::<u8>(),
                    n as usize * size_of::<c_int>(),
                );
            }
        }
    }
    area.cast::<VipsArrayInt>()
}

#[no_mangle]
pub extern "C" fn vips_array_int_get(array: *mut VipsArrayInt, n: *mut c_int) -> *mut c_int {
    let Some(array) = (unsafe { array.as_ref() }) else {
        return ptr::null_mut();
    };
    unsafe {
        if !n.is_null() {
            *n = array.area.n;
        }
    }
    array.area.data.cast::<c_int>()
}

#[no_mangle]
pub extern "C" fn vips_array_image_new(
    array: *mut *mut VipsImage,
    n: c_int,
) -> *mut VipsArrayImage {
    let area = vips_area_new_array_object(n);
    let Some(area_ref) = (unsafe { area.as_mut() }) else {
        return ptr::null_mut();
    };
    area_ref.r#type = crate::runtime::object::vips_image_get_type();
    let out = area_ref.data.cast::<*mut VipsImage>();
    for index in 0..n.max(0) as isize {
        let image = unsafe { *array.offset(index) };
        unsafe {
            *out.offset(index) = image;
        }
        if !image.is_null() {
            unsafe {
                gobject_sys::g_object_ref(image.cast());
            }
        }
    }
    area.cast::<VipsArrayImage>()
}

#[no_mangle]
pub extern "C" fn vips_array_image_get(
    array: *mut VipsArrayImage,
    n: *mut c_int,
) -> *mut *mut VipsImage {
    let Some(array) = (unsafe { array.as_ref() }) else {
        return ptr::null_mut();
    };
    unsafe {
        if !n.is_null() {
            *n = array.area.n;
        }
    }
    array.area.data.cast::<*mut VipsImage>()
}

#[no_mangle]
pub extern "C" fn vips_value_set_area(
    value: *mut gobject_sys::GValue,
    free_fn: VipsCallbackFn,
    data: *mut c_void,
) {
    let area = vips_area_new(free_fn, data);
    unsafe {
        init_value_if_needed(value, vips_area_get_type());
        gobject_sys::g_value_set_boxed(value, area.cast::<c_void>());
    }
    vips_area_unref(area);
}

#[no_mangle]
pub extern "C" fn vips_value_get_area(
    value: *const gobject_sys::GValue,
    length: *mut usize,
) -> *mut c_void {
    let area = unsafe { gobject_sys::g_value_get_boxed(value).cast::<VipsArea>() };
    let Some(area) = (unsafe { area.as_ref() }) else {
        return ptr::null_mut();
    };
    unsafe {
        if !length.is_null() {
            *length = area.length;
        }
    }
    area.data
}

#[no_mangle]
pub extern "C" fn vips_value_get_save_string(value: *const gobject_sys::GValue) -> *const c_char {
    let save = unsafe { gobject_sys::g_value_get_boxed(value).cast::<VipsSaveString>() };
    unsafe { save.as_ref() }.map_or(ptr::null(), |save| save.s.cast_const())
}

#[no_mangle]
pub extern "C" fn vips_value_set_save_string(value: *mut gobject_sys::GValue, str_: *const c_char) {
    unsafe {
        init_value_if_needed(value, vips_save_string_get_type());
        let save = glib_sys::g_malloc0(size_of::<VipsSaveString>()).cast::<VipsSaveString>();
        if let Some(save) = save.as_mut() {
            save.s = if str_.is_null() {
                ptr::null_mut()
            } else {
                glib_sys::g_strdup(str_)
            };
        }
        gobject_sys::g_value_take_boxed(value, save.cast::<c_void>());
    }
}

#[no_mangle]
pub extern "C" fn vips_value_get_ref_string(
    value: *const gobject_sys::GValue,
    length: *mut usize,
) -> *const c_char {
    let refstr = unsafe { gobject_sys::g_value_get_boxed(value).cast::<VipsRefString>() };
    vips_ref_string_get(refstr, length)
}

#[no_mangle]
pub extern "C" fn vips_value_set_ref_string(value: *mut gobject_sys::GValue, str_: *const c_char) {
    let refstr = vips_ref_string_new(str_);
    unsafe {
        init_value_if_needed(value, vips_ref_string_get_type());
        gobject_sys::g_value_set_boxed(value, refstr.cast::<c_void>());
    }
    vips_area_unref(refstr.cast::<VipsArea>());
}

#[no_mangle]
pub extern "C" fn vips_value_get_blob(
    value: *const gobject_sys::GValue,
    length: *mut usize,
) -> *mut c_void {
    vips_value_get_area(value, length)
}

#[no_mangle]
pub extern "C" fn vips_value_set_blob(
    value: *mut gobject_sys::GValue,
    free_fn: VipsCallbackFn,
    data: *const c_void,
    length: usize,
) {
    let blob = vips_blob_new(free_fn, data, length);
    unsafe {
        init_value_if_needed(value, vips_blob_get_type());
        gobject_sys::g_value_set_boxed(value, blob.cast::<c_void>());
    }
    vips_area_unref(blob.cast::<VipsArea>());
}

#[no_mangle]
pub extern "C" fn vips_value_set_blob_free(
    value: *mut gobject_sys::GValue,
    data: *mut c_void,
    length: usize,
) {
    vips_value_set_blob(value, Some(vips_area_free_cb), data.cast_const(), length);
}

#[no_mangle]
pub extern "C" fn vips_value_set_array(
    value: *mut gobject_sys::GValue,
    n: c_int,
    type_: glib_sys::GType,
    sizeof_type: usize,
) {
    let area = if type_ == unsafe { gobject_sys::g_object_get_type() } {
        vips_area_new_array_object(n)
    } else {
        vips_area_new_array(type_, sizeof_type, n)
    };
    unsafe {
        init_value_if_needed(value, vips_area_get_type());
        gobject_sys::g_value_set_boxed(value, area.cast::<c_void>());
    }
    vips_area_unref(area);
}

#[no_mangle]
pub extern "C" fn vips_value_get_array(
    value: *const gobject_sys::GValue,
    n: *mut c_int,
    type_: *mut glib_sys::GType,
    sizeof_type: *mut usize,
) -> *mut c_void {
    let area = unsafe { gobject_sys::g_value_get_boxed(value).cast::<VipsArea>() };
    let Some(area) = (unsafe { area.as_ref() }) else {
        return ptr::null_mut();
    };
    unsafe {
        if !n.is_null() {
            *n = area.n;
        }
        if !type_.is_null() {
            *type_ = area.r#type;
        }
        if !sizeof_type.is_null() {
            *sizeof_type = area.sizeof_type;
        }
    }
    area.data
}

#[no_mangle]
pub extern "C" fn vips_value_get_array_double(
    value: *const gobject_sys::GValue,
    n: *mut c_int,
) -> *mut f64 {
    vips_value_get_array(value, n, ptr::null_mut(), ptr::null_mut()).cast::<f64>()
}

#[no_mangle]
pub extern "C" fn vips_value_set_array_double(
    value: *mut gobject_sys::GValue,
    array: *const f64,
    n: c_int,
) {
    let area = vips_array_double_new(array, n);
    unsafe {
        init_value_if_needed(value, vips_array_double_get_type());
        gobject_sys::g_value_set_boxed(value, area.cast::<c_void>());
    }
    vips_area_unref(area.cast::<VipsArea>());
}

#[no_mangle]
pub extern "C" fn vips_value_get_array_int(
    value: *const gobject_sys::GValue,
    n: *mut c_int,
) -> *mut c_int {
    vips_value_get_array(value, n, ptr::null_mut(), ptr::null_mut()).cast::<c_int>()
}

#[no_mangle]
pub extern "C" fn vips_value_set_array_int(
    value: *mut gobject_sys::GValue,
    array: *const c_int,
    n: c_int,
) {
    let area = vips_array_int_new(array, n);
    unsafe {
        init_value_if_needed(value, vips_array_int_get_type());
        gobject_sys::g_value_set_boxed(value, area.cast::<c_void>());
    }
    vips_area_unref(area.cast::<VipsArea>());
}

#[no_mangle]
pub extern "C" fn vips_value_get_array_image(
    value: *const gobject_sys::GValue,
    n: *mut c_int,
) -> *mut *mut VipsImage {
    vips_value_get_array(value, n, ptr::null_mut(), ptr::null_mut()).cast::<*mut VipsImage>()
}

#[no_mangle]
pub extern "C" fn vips_value_set_array_image(value: *mut gobject_sys::GValue, n: c_int) {
    let area = vips_area_new_array_object(n);
    if let Some(area_ref) = unsafe { area.as_mut() } {
        area_ref.r#type = crate::runtime::object::vips_image_get_type();
    }
    unsafe {
        init_value_if_needed(value, vips_array_image_get_type());
        gobject_sys::g_value_set_boxed(value, area.cast::<c_void>());
    }
    vips_area_unref(area.cast::<VipsArea>());
}

#[no_mangle]
pub extern "C" fn vips_value_get_array_object(
    value: *const gobject_sys::GValue,
    n: *mut c_int,
) -> *mut *mut gobject_sys::GObject {
    vips_value_get_array(value, n, ptr::null_mut(), ptr::null_mut())
        .cast::<*mut gobject_sys::GObject>()
}

#[no_mangle]
pub extern "C" fn vips_value_set_array_object(value: *mut gobject_sys::GValue, n: c_int) {
    vips_value_set_array(
        value,
        n,
        unsafe { gobject_sys::g_object_get_type() },
        size_of::<*mut gobject_sys::GObject>(),
    );
}
