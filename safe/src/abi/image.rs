use libc::c_int;

use super::basic::{VipsGenerateFn, VipsPel, VipsStartFn, VipsStopFn};
use super::object::{VipsObject, VipsObjectClass};

c_enum! {
    pub type VipsDemandStyle {
        VIPS_DEMAND_STYLE_ERROR = -1,
        VIPS_DEMAND_STYLE_SMALLTILE = 0,
        VIPS_DEMAND_STYLE_FATSTRIP = 1,
        VIPS_DEMAND_STYLE_THINSTRIP = 2,
        VIPS_DEMAND_STYLE_ANY = 3
    }
}

c_enum! {
    pub type VipsImageType {
        VIPS_IMAGE_ERROR = -1,
        VIPS_IMAGE_NONE = 0,
        VIPS_IMAGE_SETBUF = 1,
        VIPS_IMAGE_SETBUF_FOREIGN = 2,
        VIPS_IMAGE_OPENIN = 3,
        VIPS_IMAGE_MMAPIN = 4,
        VIPS_IMAGE_MMAPINRW = 5,
        VIPS_IMAGE_OPENOUT = 6,
        VIPS_IMAGE_PARTIAL = 7
    }
}

c_enum! {
    pub type VipsInterpretation {
        VIPS_INTERPRETATION_ERROR = -1,
        VIPS_INTERPRETATION_MULTIBAND = 0,
        VIPS_INTERPRETATION_B_W = 1,
        VIPS_INTERPRETATION_HISTOGRAM = 10,
        VIPS_INTERPRETATION_XYZ = 12,
        VIPS_INTERPRETATION_LAB = 13,
        VIPS_INTERPRETATION_CMYK = 15,
        VIPS_INTERPRETATION_LABQ = 16,
        VIPS_INTERPRETATION_RGB = 17,
        VIPS_INTERPRETATION_CMC = 18,
        VIPS_INTERPRETATION_LCH = 19,
        VIPS_INTERPRETATION_LABS = 21,
        VIPS_INTERPRETATION_sRGB = 22,
        VIPS_INTERPRETATION_YXY = 23,
        VIPS_INTERPRETATION_FOURIER = 24,
        VIPS_INTERPRETATION_RGB16 = 25,
        VIPS_INTERPRETATION_GREY16 = 26,
        VIPS_INTERPRETATION_MATRIX = 27,
        VIPS_INTERPRETATION_scRGB = 28,
        VIPS_INTERPRETATION_HSV = 29,
        VIPS_INTERPRETATION_LAST = 30
    }
}

c_enum! {
    pub type VipsBandFormat {
        VIPS_FORMAT_NOTSET = -1,
        VIPS_FORMAT_UCHAR = 0,
        VIPS_FORMAT_CHAR = 1,
        VIPS_FORMAT_USHORT = 2,
        VIPS_FORMAT_SHORT = 3,
        VIPS_FORMAT_UINT = 4,
        VIPS_FORMAT_INT = 5,
        VIPS_FORMAT_FLOAT = 6,
        VIPS_FORMAT_COMPLEX = 7,
        VIPS_FORMAT_DOUBLE = 8,
        VIPS_FORMAT_DPCOMPLEX = 9,
        VIPS_FORMAT_LAST = 10
    }
}

c_enum! {
    pub type VipsCoding {
        VIPS_CODING_ERROR = -1,
        VIPS_CODING_NONE = 0,
        VIPS_CODING_LABQ = 2,
        VIPS_CODING_RAD = 6,
        VIPS_CODING_LAST = 7
    }
}

c_enum! {
    pub type VipsAccess {
        VIPS_ACCESS_RANDOM = 0,
        VIPS_ACCESS_SEQUENTIAL = 1,
        VIPS_ACCESS_SEQUENTIAL_UNBUFFERED = 2,
        VIPS_ACCESS_LAST = 3
    }
}

#[repr(C)]
pub struct VipsProgress {
    pub im: *mut VipsImage,
    pub run: c_int,
    pub eta: c_int,
    pub tpels: i64,
    pub npels: i64,
    pub percent: c_int,
    pub start: *mut glib_sys::GTimer,
}

#[repr(C)]
pub struct VipsImage {
    pub parent_instance: VipsObject,
    pub Xsize: c_int,
    pub Ysize: c_int,
    pub Bands: c_int,
    pub BandFmt: VipsBandFormat,
    pub Coding: VipsCoding,
    pub Type: VipsInterpretation,
    pub Xres: f64,
    pub Yres: f64,
    pub Xoffset: c_int,
    pub Yoffset: c_int,
    pub Length: c_int,
    pub Compression: i16,
    pub Level: i16,
    pub Bbits: c_int,
    pub time: *mut VipsProgress,
    pub Hist: *mut libc::c_char,
    pub filename: *mut libc::c_char,
    pub data: *mut VipsPel,
    pub kill: c_int,
    pub Xres_float: f32,
    pub Yres_float: f32,
    pub mode: *mut libc::c_char,
    pub dtype: VipsImageType,
    pub fd: c_int,
    pub baseaddr: *mut libc::c_void,
    pub length: usize,
    pub magic: u32,
    pub start_fn: VipsStartFn,
    pub generate_fn: VipsGenerateFn,
    pub stop_fn: VipsStopFn,
    pub client1: *mut libc::c_void,
    pub client2: *mut libc::c_void,
    pub sslock: *mut glib_sys::GMutex,
    pub regions: *mut glib_sys::GSList,
    pub dhint: VipsDemandStyle,
    pub meta: *mut glib_sys::GHashTable,
    pub meta_traverse: *mut glib_sys::GSList,
    pub sizeof_header: i64,
    pub windows: *mut glib_sys::GSList,
    pub upstream: *mut glib_sys::GSList,
    pub downstream: *mut glib_sys::GSList,
    pub serial: c_int,
    pub history_list: *mut glib_sys::GSList,
    pub progress_signal: *mut VipsImage,
    pub file_length: i64,
    pub hint_set: glib_sys::gboolean,
    pub delete_on_close: glib_sys::gboolean,
    pub delete_on_close_filename: *mut libc::c_char,
}

#[repr(C)]
pub struct VipsImageClass {
    pub parent_class: VipsObjectClass,
    pub preeval: Option<unsafe extern "C" fn(image: *mut VipsImage, progress: *mut VipsProgress, data: *mut libc::c_void)>,
    pub eval: Option<unsafe extern "C" fn(image: *mut VipsImage, progress: *mut VipsProgress, data: *mut libc::c_void)>,
    pub posteval:
        Option<unsafe extern "C" fn(image: *mut VipsImage, progress: *mut VipsProgress, data: *mut libc::c_void)>,
    pub written: Option<unsafe extern "C" fn(image: *mut VipsImage, result: *mut c_int, data: *mut libc::c_void)>,
    pub invalidate: Option<unsafe extern "C" fn(image: *mut VipsImage, data: *mut libc::c_void)>,
    pub minimise: Option<unsafe extern "C" fn(image: *mut VipsImage, data: *mut libc::c_void)>,
}
