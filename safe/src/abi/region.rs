use libc::c_int;

use super::basic::{VipsPel, VipsRect};
use super::image::VipsImage;
use super::object::{VipsObject, VipsObjectClass};

c_enum! {
    pub type RegionType {
        VIPS_REGION_NONE = 0,
        VIPS_REGION_BUFFER = 1,
        VIPS_REGION_OTHER_REGION = 2,
        VIPS_REGION_OTHER_IMAGE = 3,
        VIPS_REGION_WINDOW = 4
    }
}

c_enum! {
    pub type VipsRegionShrink {
        VIPS_REGION_SHRINK_MEAN = 0,
        VIPS_REGION_SHRINK_MEDIAN = 1,
        VIPS_REGION_SHRINK_MODE = 2,
        VIPS_REGION_SHRINK_MAX = 3,
        VIPS_REGION_SHRINK_MIN = 4,
        VIPS_REGION_SHRINK_NEAREST = 5,
        VIPS_REGION_SHRINK_LAST = 6
    }
}

#[repr(C)]
pub struct VipsWindow {
    pub ref_count: c_int,
    pub im: *mut VipsImage,
    pub top: c_int,
    pub height: c_int,
    pub data: *mut VipsPel,
    pub baseaddr: *mut libc::c_void,
    pub length: usize,
}

#[repr(C)]
pub struct VipsBufferThread {
    pub hash: *mut glib_sys::GHashTable,
    pub thread: *mut glib_sys::GThread,
}

#[repr(C)]
pub struct VipsBufferCache {
    pub buffers: *mut glib_sys::GSList,
    pub thread: *mut glib_sys::GThread,
    pub im: *mut VipsImage,
    pub buffer_thread: *mut VipsBufferThread,
    pub reserve: *mut glib_sys::GSList,
    pub n_reserve: c_int,
}

#[repr(C)]
pub struct VipsBuffer {
    pub ref_count: c_int,
    pub im: *mut VipsImage,
    pub area: VipsRect,
    pub done: glib_sys::gboolean,
    pub cache: *mut VipsBufferCache,
    pub buf: *mut VipsPel,
    pub bsize: usize,
}

#[repr(C)]
pub struct VipsRegion {
    pub parent_object: VipsObject,
    pub im: *mut VipsImage,
    pub valid: VipsRect,
    pub r#type: RegionType,
    pub data: *mut VipsPel,
    pub bpl: c_int,
    pub seq: *mut libc::c_void,
    pub thread: *mut glib_sys::GThread,
    pub window: *mut VipsWindow,
    pub buffer: *mut VipsBuffer,
    pub invalid: glib_sys::gboolean,
}

#[repr(C)]
pub struct VipsRegionClass {
    pub parent_class: VipsObjectClass,
}
