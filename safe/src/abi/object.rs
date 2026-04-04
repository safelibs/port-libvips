use libc::{c_char, c_int, c_uint, c_ulong, c_void};

use super::basic::VipsBuf;

c_enum! {
    pub type VipsArgumentFlags {
        VIPS_ARGUMENT_NONE = 0,
        VIPS_ARGUMENT_REQUIRED = 1,
        VIPS_ARGUMENT_CONSTRUCT = 2,
        VIPS_ARGUMENT_SET_ONCE = 4,
        VIPS_ARGUMENT_SET_ALWAYS = 8,
        VIPS_ARGUMENT_INPUT = 16,
        VIPS_ARGUMENT_OUTPUT = 32,
        VIPS_ARGUMENT_DEPRECATED = 64,
        VIPS_ARGUMENT_MODIFY = 128,
        VIPS_ARGUMENT_NON_HASHABLE = 256
    }
}

pub type VipsArgumentTable = glib_sys::GHashTable;

#[repr(C)]
pub struct VipsArgument {
    pub pspec: *mut gobject_sys::GParamSpec,
}

#[repr(C)]
pub struct VipsArgumentClass {
    pub parent: VipsArgument,
    pub object_class: *mut VipsObjectClass,
    pub flags: VipsArgumentFlags,
    pub priority: c_int,
    pub offset: c_uint,
}

#[repr(C)]
pub struct VipsArgumentInstance {
    pub parent: VipsArgument,
    pub argument_class: *mut VipsArgumentClass,
    pub object: *mut VipsObject,
    pub assigned: glib_sys::gboolean,
    pub close_id: c_ulong,
    pub invalidate_id: c_ulong,
}

pub type VipsArgumentMapFn = Option<
    unsafe extern "C" fn(
        object: *mut VipsObject,
        pspec: *mut gobject_sys::GParamSpec,
        argument_class: *mut VipsArgumentClass,
        argument_instance: *mut VipsArgumentInstance,
        a: *mut c_void,
        b: *mut c_void,
    ) -> *mut c_void,
>;

pub type VipsArgumentClassMapFn = Option<
    unsafe extern "C" fn(
        object_class: *mut VipsObjectClass,
        pspec: *mut gobject_sys::GParamSpec,
        argument_class: *mut VipsArgumentClass,
        a: *mut c_void,
        b: *mut c_void,
    ) -> *mut c_void,
>;

pub type VipsObjectSetArguments =
    Option<unsafe extern "C" fn(object: *mut VipsObject, a: *mut c_void, b: *mut c_void) -> *mut c_void>;

pub type VipsTypeMapFn = Option<unsafe extern "C" fn(type_: glib_sys::GType, a: *mut c_void) -> *mut c_void>;
pub type VipsTypeMap2Fn = Option<
    unsafe extern "C" fn(type_: glib_sys::GType, a: *mut c_void, b: *mut c_void) -> *mut c_void,
>;
pub type VipsClassMapFn =
    Option<unsafe extern "C" fn(cls: *mut VipsObjectClass, a: *mut c_void) -> *mut c_void>;

#[repr(C)]
pub struct VipsObject {
    pub parent_instance: gobject_sys::GObject,
    pub constructed: glib_sys::gboolean,
    pub static_object: glib_sys::gboolean,
    pub argument_table: *mut VipsArgumentTable,
    pub nickname: *mut c_char,
    pub description: *mut c_char,
    pub preclose: glib_sys::gboolean,
    pub close: glib_sys::gboolean,
    pub postclose: glib_sys::gboolean,
    pub local_memory: usize,
}

#[repr(C)]
pub struct VipsObjectClass {
    pub parent_class: gobject_sys::GObjectClass,
    pub build: Option<unsafe extern "C" fn(object: *mut VipsObject) -> c_int>,
    pub postbuild: Option<unsafe extern "C" fn(object: *mut VipsObject, data: *mut c_void) -> c_int>,
    pub summary_class:
        Option<unsafe extern "C" fn(cls: *mut VipsObjectClass, buf: *mut VipsBuf)>,
    pub summary: Option<unsafe extern "C" fn(object: *mut VipsObject, buf: *mut VipsBuf)>,
    pub dump: Option<unsafe extern "C" fn(object: *mut VipsObject, buf: *mut VipsBuf)>,
    pub sanity: Option<unsafe extern "C" fn(object: *mut VipsObject, buf: *mut VipsBuf)>,
    pub rewind: Option<unsafe extern "C" fn(object: *mut VipsObject)>,
    pub preclose: Option<unsafe extern "C" fn(object: *mut VipsObject)>,
    pub close: Option<unsafe extern "C" fn(object: *mut VipsObject)>,
    pub postclose: Option<unsafe extern "C" fn(object: *mut VipsObject)>,
    pub new_from_string: Option<unsafe extern "C" fn(string: *const c_char) -> *mut VipsObject>,
    pub to_string: Option<unsafe extern "C" fn(object: *mut VipsObject, buf: *mut VipsBuf)>,
    pub output_needs_arg: glib_sys::gboolean,
    pub output_to_arg:
        Option<unsafe extern "C" fn(object: *mut VipsObject, string: *const c_char) -> c_int>,
    pub nickname: *const c_char,
    pub description: *const c_char,
    pub argument_table: *mut VipsArgumentTable,
    pub argument_table_traverse: *mut glib_sys::GSList,
    pub argument_table_traverse_gtype: glib_sys::GType,
    pub deprecated: glib_sys::gboolean,
    pub _vips_reserved1: Option<unsafe extern "C" fn()>,
    pub _vips_reserved2: Option<unsafe extern "C" fn()>,
    pub _vips_reserved3: Option<unsafe extern "C" fn()>,
    pub _vips_reserved4: Option<unsafe extern "C" fn()>,
}
