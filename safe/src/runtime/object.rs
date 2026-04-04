use std::ffi::CStr;
use std::mem::size_of;
use std::os::raw::c_char;
use std::ptr;
use std::sync::OnceLock;

use libc::c_void;

use crate::abi::connection::{
    VipsConnection, VipsConnectionClass, VipsSbuf, VipsSbufClass, VipsSource, VipsSourceClass,
    VipsSourceCustom, VipsSourceCustomClass, VipsTarget, VipsTargetClass, VipsTargetCustom,
    VipsTargetCustomClass,
};
use crate::abi::image::{VipsImage, VipsImageClass};
use crate::abi::object::{VipsObject, VipsObjectClass};
use crate::abi::operation::{
    VipsForeign, VipsForeignClass, VipsForeignLoad, VipsForeignLoadClass, VipsForeignSave,
    VipsForeignSaveClass, VipsFormat, VipsFormatClass, VipsInterpolate, VipsInterpolateClass,
    VipsOperation, VipsOperationClass, VipsThreadState, VipsThreadStateClass,
};
use crate::abi::region::{VipsRegion, VipsRegionClass};

#[no_mangle]
pub static mut _vips__argument_id: libc::c_int = 1;

fn register_type(
    parent: glib_sys::GType,
    name: &'static [u8],
    class_size: usize,
    class_init: gobject_sys::GClassInitFunc,
    instance_size: usize,
    instance_init: gobject_sys::GInstanceInitFunc,
) -> glib_sys::GType {
    unsafe {
        gobject_sys::g_type_register_static_simple(
            parent,
            name.as_ptr().cast::<c_char>(),
            class_size as u32,
            class_init,
            instance_size as u32,
            instance_init,
            0,
        )
    }
}

fn g_object_type() -> glib_sys::GType {
    unsafe { gobject_sys::g_object_get_type() }
}

macro_rules! object_type {
    ($fn_name:ident, $parent:path, $class:ty, $instance:ty, $name:literal) => {
        #[no_mangle]
        pub extern "C" fn $fn_name() -> glib_sys::GType {
            static ONCE: OnceLock<glib_sys::GType> = OnceLock::new();
            *ONCE.get_or_init(|| {
                register_type(
                    $parent(),
                    concat!($name, "\0").as_bytes(),
                    size_of::<$class>(),
                    None,
                    size_of::<$instance>(),
                    None,
                )
            })
        }
    };
}

object_type!(
    vips_object_get_type,
    g_object_type,
    VipsObjectClass,
    VipsObject,
    "VipsObject"
);
object_type!(
    vips_operation_get_type,
    vips_object_get_type,
    VipsOperationClass,
    VipsOperation,
    "VipsOperation"
);
object_type!(
    vips_image_get_type,
    vips_object_get_type,
    VipsImageClass,
    VipsImage,
    "VipsImage"
);
object_type!(
    vips_region_get_type,
    vips_object_get_type,
    VipsRegionClass,
    VipsRegion,
    "VipsRegion"
);
object_type!(
    vips_connection_get_type,
    vips_object_get_type,
    VipsConnectionClass,
    VipsConnection,
    "VipsConnection"
);
object_type!(
    vips_source_get_type,
    vips_connection_get_type,
    VipsSourceClass,
    VipsSource,
    "VipsSource"
);
object_type!(
    vips_target_get_type,
    vips_connection_get_type,
    VipsTargetClass,
    VipsTarget,
    "VipsTarget"
);
object_type!(
    vips_foreign_get_type,
    vips_operation_get_type,
    VipsForeignClass,
    VipsForeign,
    "VipsForeign"
);
object_type!(
    vips_foreign_load_get_type,
    vips_foreign_get_type,
    VipsForeignLoadClass,
    VipsForeignLoad,
    "VipsForeignLoad"
);
object_type!(
    vips_foreign_save_get_type,
    vips_foreign_get_type,
    VipsForeignSaveClass,
    VipsForeignSave,
    "VipsForeignSave"
);
object_type!(
    vips_format_get_type,
    vips_object_get_type,
    VipsFormatClass,
    VipsFormat,
    "VipsFormat"
);
object_type!(
    vips_interpolate_get_type,
    vips_object_get_type,
    VipsInterpolateClass,
    VipsInterpolate,
    "VipsInterpolate"
);
object_type!(
    vips_sbuf_get_type,
    vips_object_get_type,
    VipsSbufClass,
    VipsSbuf,
    "VipsSbuf"
);
object_type!(
    vips_thread_state_get_type,
    vips_object_get_type,
    VipsThreadStateClass,
    VipsThreadState,
    "VipsThreadState"
);

#[no_mangle]
pub extern "C" fn vips_source_custom_get_type() -> glib_sys::GType {
    static ONCE: OnceLock<glib_sys::GType> = OnceLock::new();
    *ONCE.get_or_init(|| unsafe {
        register_type(
            vips_source_get_type(),
            b"VipsSourceCustom\0",
            size_of::<VipsSourceCustomClass>(),
            Some(crate::runtime::source::vips_source_custom_class_init),
            size_of::<VipsSourceCustom>(),
            None,
        )
    })
}

#[no_mangle]
pub extern "C" fn vips_target_custom_get_type() -> glib_sys::GType {
    static ONCE: OnceLock<glib_sys::GType> = OnceLock::new();
    *ONCE.get_or_init(|| unsafe {
        register_type(
            vips_target_get_type(),
            b"VipsTargetCustom\0",
            size_of::<VipsTargetCustomClass>(),
            Some(crate::runtime::target::vips_target_custom_class_init),
            size_of::<VipsTargetCustom>(),
            None,
        )
    })
}

pub(crate) fn ensure_types() {
    let _ = vips_object_get_type();
    let _ = vips_operation_get_type();
    let _ = vips_image_get_type();
    let _ = vips_region_get_type();
    let _ = vips_connection_get_type();
    let _ = vips_source_get_type();
    let _ = vips_source_custom_get_type();
    let _ = vips_target_get_type();
    let _ = vips_target_custom_get_type();
    let _ = vips_foreign_get_type();
    let _ = vips_foreign_load_get_type();
    let _ = vips_foreign_save_get_type();
    let _ = vips_format_get_type();
    let _ = vips_interpolate_get_type();
    let _ = vips_sbuf_get_type();
    let _ = vips_thread_state_get_type();
}

#[no_mangle]
pub extern "C" fn vips_argument_get_id() -> libc::c_int {
    unsafe {
        let id = _vips__argument_id;
        _vips__argument_id += 1;
        id
    }
}

pub(crate) fn bool_to_gboolean(value: bool) -> glib_sys::gboolean {
    if value {
        glib_sys::GTRUE
    } else {
        glib_sys::GFALSE
    }
}

pub(crate) fn gboolean_to_bool(value: glib_sys::gboolean) -> bool {
    value != glib_sys::GFALSE
}

pub(crate) fn qdata_quark(name: &'static CStr) -> glib_sys::GQuark {
    unsafe { glib_sys::g_quark_from_static_string(name.as_ptr()) }
}

pub(crate) unsafe fn object_new<T>(type_: glib_sys::GType) -> *mut T {
    unsafe { gobject_sys::g_object_new(type_, ptr::null::<c_char>()) as *mut T }
}

pub(crate) unsafe fn object_ref<T>(ptr: *mut T) -> *mut T {
    if !ptr.is_null() {
        unsafe {
            gobject_sys::g_object_ref(ptr.cast());
        }
    }
    ptr
}

pub(crate) unsafe fn object_unref<T>(ptr: *mut T) {
    if !ptr.is_null() {
        unsafe {
            gobject_sys::g_object_unref(ptr.cast());
        }
    }
}

pub(crate) unsafe extern "C" fn destroy_box<T>(data: glib_sys::gpointer) {
    if !data.is_null() {
        let _ = unsafe { Box::from_raw(data.cast::<T>()) };
    }
}

pub(crate) unsafe fn set_qdata_box<T>(
    object: *mut gobject_sys::GObject,
    quark: glib_sys::GQuark,
    value: T,
) {
    let boxed = Box::into_raw(Box::new(value));
    unsafe {
        gobject_sys::g_object_set_qdata_full(
            object,
            quark,
            boxed.cast::<c_void>(),
            Some(destroy_box::<T>),
        );
    }
}

pub(crate) unsafe fn get_qdata_ptr<T>(
    object: *mut gobject_sys::GObject,
    quark: glib_sys::GQuark,
) -> *mut T {
    unsafe { gobject_sys::g_object_get_qdata(object, quark).cast::<T>() }
}

pub(crate) unsafe fn take_qdata_ptr<T>(
    object: *mut gobject_sys::GObject,
    quark: glib_sys::GQuark,
) -> *mut T {
    unsafe { gobject_sys::g_object_steal_qdata(object, quark).cast::<T>() }
}
