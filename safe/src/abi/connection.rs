use libc::{c_char, c_int, c_void, off_t};

use super::object::{VipsObject, VipsObjectClass};
use crate::abi::r#type::VipsBlob;

pub const VIPS_TARGET_BUFFER_SIZE: usize = 8500;
pub const VIPS_SBUF_BUFFER_SIZE: usize = 4096;

#[repr(C)]
pub struct VipsConnection {
    pub parent_object: VipsObject,
    pub descriptor: c_int,
    pub tracked_descriptor: c_int,
    pub close_descriptor: c_int,
    pub filename: *mut c_char,
}

#[repr(C)]
pub struct VipsConnectionClass {
    pub parent_class: VipsObjectClass,
}

#[repr(C)]
pub struct VipsSource {
    pub parent_object: VipsConnection,
    pub decode: glib_sys::gboolean,
    pub have_tested_seek: glib_sys::gboolean,
    pub is_pipe: glib_sys::gboolean,
    pub read_position: i64,
    pub length: i64,
    pub data: *const c_void,
    pub header_bytes: *mut glib_sys::GByteArray,
    pub sniff: *mut glib_sys::GByteArray,
    pub blob: *mut VipsBlob,
    pub mmap_baseaddr: *mut c_void,
    pub mmap_length: usize,
}

#[repr(C)]
pub struct VipsSourceClass {
    pub parent_class: VipsConnectionClass,
    pub read: Option<unsafe extern "C" fn(source: *mut VipsSource, data: *mut c_void, length: usize) -> i64>,
    pub seek:
        Option<unsafe extern "C" fn(source: *mut VipsSource, offset: i64, whence: c_int) -> i64>,
}

#[repr(C)]
pub struct VipsSourceCustom {
    pub parent_object: VipsSource,
}

#[repr(C)]
pub struct VipsSourceCustomClass {
    pub parent_class: VipsSourceClass,
    pub read:
        Option<unsafe extern "C" fn(source: *mut VipsSourceCustom, data: *mut c_void, length: i64) -> i64>,
    pub seek:
        Option<unsafe extern "C" fn(source: *mut VipsSourceCustom, offset: i64, whence: c_int) -> i64>,
}

#[repr(C)]
pub struct VipsGInputStream {
    pub parent_instance: gio_sys::GInputStream,
    pub source: *mut VipsSource,
}

#[repr(C)]
pub struct VipsGInputStreamClass {
    pub parent_class: gio_sys::GInputStreamClass,
}

#[repr(C)]
pub struct VipsSourceGInputStream {
    pub parent_instance: VipsSource,
    pub stream: *mut gio_sys::GInputStream,
    pub seekable: *mut gio_sys::GSeekable,
    pub info: *mut gio_sys::GFileInfo,
}

#[repr(C)]
pub struct VipsSourceGInputStreamClass {
    pub parent_class: VipsSourceClass,
}

#[repr(C)]
pub struct VipsTarget {
    pub parent_object: VipsConnection,
    pub memory: glib_sys::gboolean,
    pub ended: glib_sys::gboolean,
    pub memory_buffer: *mut glib_sys::GString,
    pub blob: *mut VipsBlob,
    pub output_buffer: [u8; VIPS_TARGET_BUFFER_SIZE],
    pub write_point: c_int,
    pub position: off_t,
    pub delete_on_close: glib_sys::gboolean,
    pub delete_on_close_filename: *mut c_char,
}

#[repr(C)]
pub struct VipsTargetClass {
    pub parent_class: VipsConnectionClass,
    pub write: Option<unsafe extern "C" fn(target: *mut VipsTarget, data: *const c_void, length: usize) -> i64>,
    pub finish: Option<unsafe extern "C" fn(target: *mut VipsTarget)>,
    pub read: Option<unsafe extern "C" fn(target: *mut VipsTarget, data: *mut c_void, length: usize) -> i64>,
    pub seek: Option<unsafe extern "C" fn(target: *mut VipsTarget, offset: off_t, whence: c_int) -> off_t>,
    pub end: Option<unsafe extern "C" fn(target: *mut VipsTarget) -> c_int>,
}

#[repr(C)]
pub struct VipsTargetCustom {
    pub parent_object: VipsTarget,
}

#[repr(C)]
pub struct VipsTargetCustomClass {
    pub parent_class: VipsTargetClass,
    pub write:
        Option<unsafe extern "C" fn(target: *mut VipsTargetCustom, data: *const c_void, length: i64) -> i64>,
    pub finish: Option<unsafe extern "C" fn(target: *mut VipsTargetCustom)>,
    pub read:
        Option<unsafe extern "C" fn(target: *mut VipsTargetCustom, data: *mut c_void, length: i64) -> i64>,
    pub seek:
        Option<unsafe extern "C" fn(target: *mut VipsTargetCustom, offset: i64, whence: c_int) -> i64>,
    pub end: Option<unsafe extern "C" fn(target: *mut VipsTargetCustom) -> c_int>,
}

#[repr(C)]
pub struct VipsSbuf {
    pub parent_object: VipsObject,
    pub source: *mut VipsSource,
    pub input_buffer: [u8; VIPS_SBUF_BUFFER_SIZE + 1],
    pub chars_in_buffer: c_int,
    pub read_point: c_int,
    pub line: [u8; VIPS_SBUF_BUFFER_SIZE + 1],
}

#[repr(C)]
pub struct VipsSbufClass {
    pub parent_class: VipsObjectClass,
}
