use libc::{c_char, c_int, c_uint, c_void};

use super::basic::{VipsBuf, VipsRect};
use super::connection::VipsSource;
use super::image::{VipsAccess, VipsBandFormat, VipsImage};
use super::object::{VipsObject, VipsObjectClass};
use super::region::VipsRegion;
use crate::abi::r#type::VipsArrayDouble;

c_enum! {
    pub type VipsOperationFlags {
        VIPS_OPERATION_NONE = 0,
        VIPS_OPERATION_SEQUENTIAL = 1,
        VIPS_OPERATION_SEQUENTIAL_UNBUFFERED = 2,
        VIPS_OPERATION_NOCACHE = 4,
        VIPS_OPERATION_DEPRECATED = 8,
        VIPS_OPERATION_UNTRUSTED = 16,
        VIPS_OPERATION_BLOCKED = 32,
        VIPS_OPERATION_REVALIDATE = 64
    }
}

pub type VipsOperationBuildFn =
    Option<unsafe extern "C" fn(object: *mut VipsObject) -> glib_sys::gboolean>;

#[repr(C)]
pub struct VipsOperation {
    pub parent_instance: VipsObject,
    pub hash: c_uint,
    pub found_hash: glib_sys::gboolean,
    pub pixels: c_int,
}

#[repr(C)]
pub struct VipsOperationClass {
    pub parent_class: VipsObjectClass,
    pub usage: Option<unsafe extern "C" fn(cls: *mut VipsOperationClass, buf: *mut VipsBuf)>,
    pub get_flags:
        Option<unsafe extern "C" fn(operation: *mut VipsOperation) -> VipsOperationFlags>,
    pub flags: VipsOperationFlags,
    pub invalidate: Option<unsafe extern "C" fn(operation: *mut VipsOperation)>,
}

c_enum! {
    pub type VipsFormatFlags {
        VIPS_FORMAT_NONE = 0,
        VIPS_FORMAT_PARTIAL = 1,
        VIPS_FORMAT_BIGENDIAN = 2
    }
}

#[repr(C)]
pub struct VipsFormat {
    pub parent_object: VipsObject,
}

#[repr(C)]
pub struct VipsFormatClass {
    pub parent_class: VipsObjectClass,
    pub is_a: Option<unsafe extern "C" fn(filename: *const c_char) -> glib_sys::gboolean>,
    pub header: Option<unsafe extern "C" fn(filename: *const c_char, out: *mut VipsImage) -> c_int>,
    pub load: Option<unsafe extern "C" fn(filename: *const c_char, out: *mut VipsImage) -> c_int>,
    pub save: Option<unsafe extern "C" fn(image: *mut VipsImage, filename: *const c_char) -> c_int>,
    pub get_flags: Option<unsafe extern "C" fn(filename: *const c_char) -> VipsFormatFlags>,
    pub priority: c_int,
    pub suffs: *const *const c_char,
}

#[repr(C)]
pub struct VipsInterpolate {
    pub parent_object: VipsObject,
}

pub type VipsInterpolateMethod = Option<
    unsafe extern "C" fn(
        interpolate: *mut VipsInterpolate,
        out: *mut c_void,
        input: *mut VipsRegion,
        x: f64,
        y: f64,
    ),
>;

#[repr(C)]
pub struct VipsInterpolateClass {
    pub parent_class: VipsObjectClass,
    pub interpolate: VipsInterpolateMethod,
    pub get_window_size: Option<unsafe extern "C" fn(interpolate: *mut VipsInterpolate) -> c_int>,
    pub window_size: c_int,
    pub get_window_offset: Option<unsafe extern "C" fn(interpolate: *mut VipsInterpolate) -> c_int>,
    pub window_offset: c_int,
}

c_enum! {
    pub type VipsForeignFlags {
        VIPS_FOREIGN_NONE = 0,
        VIPS_FOREIGN_PARTIAL = 1,
        VIPS_FOREIGN_BIGENDIAN = 2,
        VIPS_FOREIGN_SEQUENTIAL = 4,
        VIPS_FOREIGN_ALL = 7
    }
}

c_enum! {
    pub type VipsFailOn {
        VIPS_FAIL_ON_NONE = 0,
        VIPS_FAIL_ON_TRUNCATED = 1,
        VIPS_FAIL_ON_ERROR = 2,
        VIPS_FAIL_ON_WARNING = 3,
        VIPS_FAIL_ON_LAST = 4
    }
}

c_enum! {
    pub type VipsSaveable {
        VIPS_SAVEABLE_MONO = 0,
        VIPS_SAVEABLE_RGB = 1,
        VIPS_SAVEABLE_RGBA = 2,
        VIPS_SAVEABLE_RGBA_ONLY = 3,
        VIPS_SAVEABLE_RGB_CMYK = 4,
        VIPS_SAVEABLE_ANY = 5,
        VIPS_SAVEABLE_LAST = 6
    }
}

c_enum! {
    pub type VipsForeignKeep {
        VIPS_FOREIGN_KEEP_NONE = 0,
        VIPS_FOREIGN_KEEP_EXIF = 1,
        VIPS_FOREIGN_KEEP_XMP = 2,
        VIPS_FOREIGN_KEEP_IPTC = 4,
        VIPS_FOREIGN_KEEP_ICC = 8,
        VIPS_FOREIGN_KEEP_OTHER = 16,
        VIPS_FOREIGN_KEEP_ALL = 31
    }
}

c_enum! {
    pub type VipsForeignSubsample {
        VIPS_FOREIGN_SUBSAMPLE_AUTO = 0,
        VIPS_FOREIGN_SUBSAMPLE_ON = 1,
        VIPS_FOREIGN_SUBSAMPLE_OFF = 2,
        VIPS_FOREIGN_SUBSAMPLE_LAST = 3
    }
}

c_enum! {
    pub type VipsForeignJpegSubsample {
        VIPS_FOREIGN_JPEG_SUBSAMPLE_AUTO = 0,
        VIPS_FOREIGN_JPEG_SUBSAMPLE_ON = 1,
        VIPS_FOREIGN_JPEG_SUBSAMPLE_OFF = 2,
        VIPS_FOREIGN_JPEG_SUBSAMPLE_LAST = 3
    }
}

c_enum! {
    pub type VipsForeignWebpPreset {
        VIPS_FOREIGN_WEBP_PRESET_DEFAULT = 0,
        VIPS_FOREIGN_WEBP_PRESET_PICTURE = 1,
        VIPS_FOREIGN_WEBP_PRESET_PHOTO = 2,
        VIPS_FOREIGN_WEBP_PRESET_DRAWING = 3,
        VIPS_FOREIGN_WEBP_PRESET_ICON = 4,
        VIPS_FOREIGN_WEBP_PRESET_TEXT = 5,
        VIPS_FOREIGN_WEBP_PRESET_LAST = 6
    }
}

c_enum! {
    pub type VipsForeignTiffCompression {
        VIPS_FOREIGN_TIFF_COMPRESSION_NONE = 0,
        VIPS_FOREIGN_TIFF_COMPRESSION_JPEG = 1,
        VIPS_FOREIGN_TIFF_COMPRESSION_DEFLATE = 2,
        VIPS_FOREIGN_TIFF_COMPRESSION_PACKBITS = 3,
        VIPS_FOREIGN_TIFF_COMPRESSION_CCITTFAX4 = 4,
        VIPS_FOREIGN_TIFF_COMPRESSION_LZW = 5,
        VIPS_FOREIGN_TIFF_COMPRESSION_WEBP = 6,
        VIPS_FOREIGN_TIFF_COMPRESSION_ZSTD = 7,
        VIPS_FOREIGN_TIFF_COMPRESSION_JP2K = 8,
        VIPS_FOREIGN_TIFF_COMPRESSION_LAST = 9
    }
}

c_enum! {
    pub type VipsForeignTiffPredictor {
        VIPS_FOREIGN_TIFF_PREDICTOR_NONE = 1,
        VIPS_FOREIGN_TIFF_PREDICTOR_HORIZONTAL = 2,
        VIPS_FOREIGN_TIFF_PREDICTOR_FLOAT = 3,
        VIPS_FOREIGN_TIFF_PREDICTOR_LAST = 4
    }
}

c_enum! {
    pub type VipsForeignTiffResunit {
        VIPS_FOREIGN_TIFF_RESUNIT_CM = 0,
        VIPS_FOREIGN_TIFF_RESUNIT_INCH = 1,
        VIPS_FOREIGN_TIFF_RESUNIT_LAST = 2
    }
}

c_enum! {
    pub type VipsForeignPngFilter {
        VIPS_FOREIGN_PNG_FILTER_NONE = 8,
        VIPS_FOREIGN_PNG_FILTER_SUB = 16,
        VIPS_FOREIGN_PNG_FILTER_UP = 32,
        VIPS_FOREIGN_PNG_FILTER_AVG = 64,
        VIPS_FOREIGN_PNG_FILTER_PAETH = 128,
        VIPS_FOREIGN_PNG_FILTER_ALL = 248
    }
}

c_enum! {
    pub type VipsForeignPpmFormat {
        VIPS_FOREIGN_PPM_FORMAT_PBM = 0,
        VIPS_FOREIGN_PPM_FORMAT_PGM = 1,
        VIPS_FOREIGN_PPM_FORMAT_PPM = 2,
        VIPS_FOREIGN_PPM_FORMAT_PFM = 3,
        VIPS_FOREIGN_PPM_FORMAT_PNM = 4,
        VIPS_FOREIGN_PPM_FORMAT_LAST = 5
    }
}

c_enum! {
    pub type VipsForeignDzLayout {
        VIPS_FOREIGN_DZ_LAYOUT_DZ = 0,
        VIPS_FOREIGN_DZ_LAYOUT_ZOOMIFY = 1,
        VIPS_FOREIGN_DZ_LAYOUT_GOOGLE = 2,
        VIPS_FOREIGN_DZ_LAYOUT_IIIF = 3,
        VIPS_FOREIGN_DZ_LAYOUT_IIIF3 = 4,
        VIPS_FOREIGN_DZ_LAYOUT_LAST = 5
    }
}

c_enum! {
    pub type VipsForeignDzDepth {
        VIPS_FOREIGN_DZ_DEPTH_ONEPIXEL = 0,
        VIPS_FOREIGN_DZ_DEPTH_ONETILE = 1,
        VIPS_FOREIGN_DZ_DEPTH_ONE = 2,
        VIPS_FOREIGN_DZ_DEPTH_LAST = 3
    }
}

c_enum! {
    pub type VipsForeignDzContainer {
        VIPS_FOREIGN_DZ_CONTAINER_FS = 0,
        VIPS_FOREIGN_DZ_CONTAINER_ZIP = 1,
        VIPS_FOREIGN_DZ_CONTAINER_SZI = 2,
        VIPS_FOREIGN_DZ_CONTAINER_LAST = 3
    }
}

c_enum! {
    pub type VipsForeignHeifCompression {
        VIPS_FOREIGN_HEIF_COMPRESSION_HEVC = 1,
        VIPS_FOREIGN_HEIF_COMPRESSION_AVC = 2,
        VIPS_FOREIGN_HEIF_COMPRESSION_JPEG = 3,
        VIPS_FOREIGN_HEIF_COMPRESSION_AV1 = 4,
        VIPS_FOREIGN_HEIF_COMPRESSION_LAST = 5
    }
}

c_enum! {
    pub type VipsForeignHeifEncoder {
        VIPS_FOREIGN_HEIF_ENCODER_AUTO = 0,
        VIPS_FOREIGN_HEIF_ENCODER_AOM = 1,
        VIPS_FOREIGN_HEIF_ENCODER_RAV1E = 2,
        VIPS_FOREIGN_HEIF_ENCODER_SVT = 3,
        VIPS_FOREIGN_HEIF_ENCODER_X265 = 4,
        VIPS_FOREIGN_HEIF_ENCODER_LAST = 5
    }
}

#[repr(C)]
pub struct VipsForeign {
    pub parent_object: VipsOperation,
}

#[repr(C)]
pub struct VipsForeignClass {
    pub parent_class: VipsOperationClass,
    pub priority: c_int,
    pub suffs: *const *const c_char,
}

#[repr(C)]
pub struct VipsForeignLoad {
    pub parent_object: VipsForeign,
    pub memory: glib_sys::gboolean,
    pub access: VipsAccess,
    pub flags: VipsForeignFlags,
    pub fail_on: VipsFailOn,
    pub fail: glib_sys::gboolean,
    pub sequential: glib_sys::gboolean,
    pub out: *mut VipsImage,
    pub real: *mut VipsImage,
    pub nocache: glib_sys::gboolean,
    pub disc: glib_sys::gboolean,
    pub error: glib_sys::gboolean,
    pub revalidate: glib_sys::gboolean,
}

#[repr(C)]
pub struct VipsForeignLoadClass {
    pub parent_class: VipsForeignClass,
    pub is_a: Option<unsafe extern "C" fn(filename: *const c_char) -> glib_sys::gboolean>,
    pub is_a_buffer:
        Option<unsafe extern "C" fn(data: *const c_void, size: usize) -> glib_sys::gboolean>,
    pub is_a_source: Option<unsafe extern "C" fn(source: *mut VipsSource) -> glib_sys::gboolean>,
    pub get_flags_filename:
        Option<unsafe extern "C" fn(filename: *const c_char) -> VipsForeignFlags>,
    pub get_flags: Option<unsafe extern "C" fn(load: *mut VipsForeignLoad) -> VipsForeignFlags>,
    pub header: Option<unsafe extern "C" fn(load: *mut VipsForeignLoad) -> c_int>,
    pub load: Option<unsafe extern "C" fn(load: *mut VipsForeignLoad) -> c_int>,
}

#[repr(C)]
pub struct VipsForeignSave {
    pub parent_object: VipsForeign,
    pub strip: glib_sys::gboolean,
    pub keep: VipsForeignKeep,
    pub profile: *mut c_char,
    pub background: *mut VipsArrayDouble,
    pub page_height: c_int,
    pub r#in: *mut VipsImage,
    pub ready: *mut VipsImage,
}

#[repr(C)]
pub struct VipsForeignSaveClass {
    pub parent_class: VipsForeignClass,
    pub saveable: VipsSaveable,
    pub format_table: *mut VipsBandFormat,
    pub coding: [glib_sys::gboolean; super::image::VIPS_CODING_LAST as usize],
}

#[repr(C)]
pub struct VipsThreadState {
    pub parent_object: VipsObject,
    pub im: *mut VipsImage,
    pub reg: *mut VipsRegion,
    pub pos: VipsRect,
    pub x: c_int,
    pub y: c_int,
    pub stop: glib_sys::gboolean,
    pub a: *mut c_void,
    pub stall: glib_sys::gboolean,
}

#[repr(C)]
pub struct VipsThreadStateClass {
    pub parent_class: VipsObjectClass,
}

pub type VipsThreadStartFn =
    Option<unsafe extern "C" fn(im: *mut VipsImage, a: *mut c_void) -> *mut VipsThreadState>;
pub type VipsThreadpoolAllocateFn = Option<
    unsafe extern "C" fn(
        state: *mut VipsThreadState,
        a: *mut c_void,
        stop: *mut glib_sys::gboolean,
    ) -> c_int,
>;
pub type VipsThreadpoolWorkFn =
    Option<unsafe extern "C" fn(state: *mut VipsThreadState, a: *mut c_void) -> c_int>;
pub type VipsThreadpoolProgressFn = Option<unsafe extern "C" fn(a: *mut c_void) -> c_int>;
