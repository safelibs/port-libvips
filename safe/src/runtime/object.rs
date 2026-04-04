use std::collections::{HashMap, HashSet};
use std::ffi::{CStr, CString};
use std::mem::{offset_of, size_of};
use std::os::raw::c_char;
use std::ptr;
use std::sync::{Mutex, OnceLock};

use libc::{c_int, c_uint, c_void};

use crate::abi::basic::{VipsBuf, VipsSListMap2Fn};
use crate::abi::connection::{
    VipsConnection, VipsConnectionClass, VipsSbuf, VipsSbufClass, VipsSource, VipsSourceClass,
    VipsSourceCustom, VipsSourceCustomClass, VipsTarget, VipsTargetClass, VipsTargetCustom,
    VipsTargetCustomClass,
};
use crate::abi::image::{VipsImage, VipsImageClass};
use crate::abi::object::{
    VipsArgument, VipsArgumentClass, VipsArgumentClassMapFn, VipsArgumentFlags,
    VipsArgumentInstance, VipsArgumentMapFn, VipsArgumentTable, VipsClassMapFn, VipsObject,
    VipsObjectClass, VipsObjectSetArguments, VipsTypeMap2Fn, VipsTypeMapFn, VIPS_ARGUMENT_CONSTRUCT,
    VIPS_ARGUMENT_DEPRECATED, VIPS_ARGUMENT_INPUT, VIPS_ARGUMENT_OUTPUT, VIPS_ARGUMENT_REQUIRED,
    VIPS_ARGUMENT_SET_ALWAYS, VIPS_ARGUMENT_SET_ONCE,
};
use crate::abi::operation::{
    VipsForeign, VipsForeignClass, VipsForeignLoad, VipsForeignLoadClass, VipsForeignSave,
    VipsForeignSaveClass, VipsFormat, VipsFormatClass, VipsInterpolate, VipsInterpolateClass,
    VipsOperation, VipsOperationClass, VipsThreadState, VipsThreadStateClass,
};
use crate::abi::region::{VipsRegion, VipsRegionClass};
use crate::runtime::error::append_message_str;

#[no_mangle]
pub static mut _vips__argument_id: c_int = 1;

pub(crate) const DYNAMIC_ARGUMENT_OFFSET: c_uint = c_uint::MAX;

static OBJECT_STATE_QUARK: &CStr = c"safe-vips-object-state";
static OBJECT_NICKNAME: &[u8] = b"object\0";
static OBJECT_DESCRIPTION: &[u8] = b"base class\0";

fn object_registry() -> &'static Mutex<HashSet<usize>> {
    static REGISTRY: OnceLock<Mutex<HashSet<usize>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashSet::new()))
}

fn leaked_cstrings() -> &'static Mutex<Vec<usize>> {
    static STRINGS: OnceLock<Mutex<Vec<usize>>> = OnceLock::new();
    STRINGS.get_or_init(|| Mutex::new(Vec::new()))
}

pub(crate) fn leak_cstring(text: &str) -> *const c_char {
    let raw = CString::new(text)
        .expect("cstring")
        .into_raw();
    leaked_cstrings()
        .lock()
        .expect("leaked cstrings")
        .push(raw as usize);
    raw.cast_const()
}

pub(crate) fn register_type(
    parent: glib_sys::GType,
    name: *const c_char,
    class_size: usize,
    class_init: gobject_sys::GClassInitFunc,
    instance_size: usize,
    instance_init: gobject_sys::GInstanceInitFunc,
    flags: u32,
) -> glib_sys::GType {
    unsafe {
        gobject_sys::g_type_register_static_simple(
            parent,
            name,
            class_size as u32,
            class_init,
            instance_size as u32,
            instance_init,
            flags,
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
                    concat!($name, "\0").as_ptr().cast(),
                    size_of::<$class>(),
                    None,
                    size_of::<$instance>(),
                    None,
                    0,
                )
            })
        }
    };
}

#[no_mangle]
pub extern "C" fn vips_object_get_type() -> glib_sys::GType {
    static ONCE: OnceLock<glib_sys::GType> = OnceLock::new();
    *ONCE.get_or_init(|| {
        register_type(
            g_object_type(),
            c"VipsObject".as_ptr(),
            size_of::<VipsObjectClass>(),
            Some(vips_object_class_init),
            size_of::<VipsObject>(),
            Some(vips_object_instance_init),
            gobject_sys::G_TYPE_FLAG_ABSTRACT,
        )
    })
}

#[no_mangle]
pub extern "C" fn vips_operation_get_type() -> glib_sys::GType {
    static ONCE: OnceLock<glib_sys::GType> = OnceLock::new();
    *ONCE.get_or_init(|| {
        register_type(
            vips_object_get_type(),
            c"VipsOperation".as_ptr(),
            size_of::<VipsOperationClass>(),
            Some(crate::runtime::operation::vips_operation_class_init),
            size_of::<VipsOperation>(),
            None,
            gobject_sys::G_TYPE_FLAG_ABSTRACT,
        )
    })
}
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
    *ONCE.get_or_init(|| {
        register_type(
            vips_source_get_type(),
            c"VipsSourceCustom".as_ptr(),
            size_of::<VipsSourceCustomClass>(),
            Some(crate::runtime::source::vips_source_custom_class_init),
            size_of::<VipsSourceCustom>(),
            None,
            0,
        )
    })
}

#[no_mangle]
pub extern "C" fn vips_target_custom_get_type() -> glib_sys::GType {
    static ONCE: OnceLock<glib_sys::GType> = OnceLock::new();
    *ONCE.get_or_init(|| {
        register_type(
            vips_target_get_type(),
            c"VipsTargetCustom".as_ptr(),
            size_of::<VipsTargetCustomClass>(),
            Some(crate::runtime::target::vips_target_custom_class_init),
            size_of::<VipsTargetCustom>(),
            None,
            0,
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
    let _ = vips_interpolate_get_type();
    let _ = vips_thread_state_get_type();
}

#[no_mangle]
pub extern "C" fn vips_argument_get_id() -> c_int {
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
            boxed.cast(),
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

enum DynamicValue {
    String(*mut c_char),
    Object(*mut gobject_sys::GObject),
    Int(c_int),
    UInt64(u64),
    Bool(glib_sys::gboolean),
    Enum(c_int),
    Flags(u32),
    Pointer(*mut c_void),
    Double(f64),
    Boxed {
        gtype: glib_sys::GType,
        value: glib_sys::gpointer,
    },
}

impl DynamicValue {
    unsafe fn clear(&mut self) {
        match self {
            DynamicValue::String(value) => unsafe {
                glib_sys::g_free((*value).cast());
                *value = ptr::null_mut();
            },
            DynamicValue::Object(value) => {
                if !value.is_null() {
                    unsafe {
                        gobject_sys::g_object_unref((*value).cast());
                    }
                    *value = ptr::null_mut();
                }
            }
            DynamicValue::Boxed { gtype, value } => {
                if !value.is_null() {
                    unsafe {
                        gobject_sys::g_boxed_free(*gtype, *value);
                    }
                    *value = ptr::null_mut();
                }
            }
            _ => {}
        }
    }
}

impl Drop for DynamicValue {
    fn drop(&mut self) {
        unsafe {
            self.clear();
        }
    }
}

#[derive(Default)]
struct ObjectState {
    values: HashMap<String, DynamicValue>,
}

fn object_state_quark() -> glib_sys::GQuark {
    qdata_quark(OBJECT_STATE_QUARK)
}

unsafe fn object_state(object: *mut VipsObject) -> Option<&'static mut ObjectState> {
    let object_ptr = object.cast::<gobject_sys::GObject>();
    let current = unsafe { get_qdata_ptr::<ObjectState>(object_ptr, object_state_quark()) };
    if !current.is_null() {
        return unsafe { current.as_mut() };
    }

    unsafe {
        set_qdata_box(object_ptr, object_state_quark(), ObjectState::default());
        get_qdata_ptr::<ObjectState>(object_ptr, object_state_quark()).as_mut()
    }
}

fn vips_object_parent_class() -> *mut gobject_sys::GObjectClass {
    let object_class =
        unsafe { gobject_sys::g_type_class_peek(vips_object_get_type()) }.cast::<gobject_sys::GTypeClass>();
    if object_class.is_null() {
        ptr::null_mut()
    } else {
        unsafe { gobject_sys::g_type_class_peek_parent(object_class.cast()) }
            .cast::<gobject_sys::GObjectClass>()
    }
}

unsafe fn clear_dynamic_value(value: Option<DynamicValue>) {
    drop(value);
}

unsafe fn set_dynamic_value(object: *mut VipsObject, name: &str, value: DynamicValue) {
    if let Some(state) = unsafe { object_state(object) } {
        unsafe {
            clear_dynamic_value(state.values.insert(name.to_owned(), value));
        }
    }
}

unsafe fn dynamic_value<'a>(object: *mut VipsObject, name: &str) -> Option<&'a DynamicValue> {
    unsafe { object_state(object) }?.values.get(name)
}

pub(crate) unsafe fn dynamic_boxed_value(
    object: *mut VipsObject,
    name: &str,
) -> Option<(glib_sys::GType, glib_sys::gpointer)> {
    match unsafe { dynamic_value(object, name) } {
        Some(DynamicValue::Boxed { gtype, value }) => Some((*gtype, *value)),
        _ => None,
    }
}

unsafe fn remove_dynamic_value(object: *mut VipsObject, name: &str) {
    if let Some(state) = unsafe { object_state(object) } {
        unsafe {
            clear_dynamic_value(state.values.remove(name));
        }
    }
}

unsafe fn hash_table_lookup<T>(
    table: *mut VipsArgumentTable,
    key: glib_sys::gconstpointer,
) -> *mut T {
    if table.is_null() {
        return ptr::null_mut();
    }
    unsafe { glib_sys::g_hash_table_lookup(table.cast(), key).cast::<T>() }
}

unsafe fn hash_table_replace(
    table: *mut VipsArgumentTable,
    key: glib_sys::gpointer,
    value: glib_sys::gpointer,
) {
    if !table.is_null() {
        unsafe {
            glib_sys::g_hash_table_replace(table.cast(), key, value);
        }
    }
}

unsafe extern "C" fn free_argument_instance(data: glib_sys::gpointer) {
    if !data.is_null() {
        unsafe {
            glib_sys::g_free(data);
        }
    }
}

unsafe fn argument_table_new(class_table: bool) -> *mut VipsArgumentTable {
    unsafe {
        glib_sys::g_hash_table_new_full(
            Some(glib_sys::g_direct_hash),
            Some(glib_sys::g_direct_equal),
            None,
            if class_table {
                Some(glib_sys::g_free)
            } else {
                Some(free_argument_instance)
            },
        )
        .cast()
    }
}

unsafe fn argument_table_destroy(table: *mut VipsArgumentTable) {
    if !table.is_null() {
        unsafe {
            glib_sys::g_hash_table_destroy(table.cast());
        }
    }
}

pub(crate) unsafe fn object_class(object: *mut VipsObject) -> *mut VipsObjectClass {
    if object.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        (*object.cast::<gobject_sys::GObject>())
            .g_type_instance
            .g_class
            .cast::<VipsObjectClass>()
    }
}

pub(crate) unsafe fn object_class_for_type(type_: glib_sys::GType) -> *mut VipsObjectClass {
    if type_ == 0 {
        return ptr::null_mut();
    }
    unsafe { gobject_sys::g_type_class_ref(type_).cast::<VipsObjectClass>() }
}

pub(crate) unsafe fn init_subclass_class(class: *mut VipsObjectClass) {
    if class.is_null() {
        return;
    }
    let current_type = unsafe { (*(class.cast::<gobject_sys::GTypeClass>())).g_type };
    if unsafe { (*class).argument_table_traverse_gtype } == current_type {
        return;
    }
    let parent = unsafe {
        gobject_sys::g_type_class_peek_parent(class.cast())
    }
    .cast::<VipsObjectClass>();
    unsafe {
        (*class).argument_table = argument_table_new(true);
        (*class).argument_table_traverse = if parent.is_null() {
            ptr::null_mut()
        } else {
            glib_sys::g_slist_copy((*parent).argument_table_traverse)
        };
        (*class).argument_table_traverse_gtype = current_type;
    }
}

pub(crate) unsafe fn prepare_existing_class(class: *mut VipsObjectClass) {
    unsafe {
        init_subclass_class(class);
        let gobject_class = class.cast::<gobject_sys::GObjectClass>();
        let parent_class =
            gobject_sys::g_type_class_peek_parent(class.cast()).cast::<gobject_sys::GObjectClass>();
        if !parent_class.is_null() {
            if (*gobject_class).dispose.is_none() {
                (*gobject_class).dispose = (*parent_class).dispose;
            }
            if (*gobject_class).finalize.is_none() {
                (*gobject_class).finalize = (*parent_class).finalize;
            }
        }
        (*gobject_class).set_property = Some(vips_object_set_property);
        (*gobject_class).get_property = Some(vips_object_get_property);
    }
}

unsafe fn find_argument_class(
    class: *mut VipsObjectClass,
    pspec: *mut gobject_sys::GParamSpec,
) -> *mut VipsArgumentClass {
    if class.is_null() || pspec.is_null() {
        return ptr::null_mut();
    }
    let local = unsafe { hash_table_lookup::<VipsArgumentClass>((*class).argument_table, pspec.cast()) };
    if !local.is_null() {
        return local;
    }
    let mut node = unsafe { (*class).argument_table_traverse };
    while !node.is_null() {
        let argument = unsafe { (*node).data.cast::<VipsArgumentClass>() };
        if !argument.is_null() && unsafe { (*argument).parent.pspec == pspec } {
            return argument;
        }
        node = unsafe { (*node).next };
    }
    ptr::null_mut()
}

unsafe fn argument_instance(
    object: *mut VipsObject,
    class: *mut VipsArgumentClass,
) -> *mut VipsArgumentInstance {
    if object.is_null() || class.is_null() {
        return ptr::null_mut();
    }

    let object_ref = unsafe { &mut *object };
    if object_ref.argument_table.is_null() {
        object_ref.argument_table = unsafe { argument_table_new(false) };
        let mut node = unsafe { (*object_class(object)).argument_table_traverse };
        while !node.is_null() {
            let class_argument = unsafe { (*node).data.cast::<VipsArgumentClass>() };
            if !class_argument.is_null() {
                let instance_ptr =
                    unsafe { glib_sys::g_malloc0(size_of::<VipsArgumentInstance>()) }
                        .cast::<VipsArgumentInstance>();
                unsafe {
                    ptr::write(
                        instance_ptr,
                        VipsArgumentInstance {
                            parent: VipsArgument {
                                pspec: (*class_argument).parent.pspec,
                            },
                            argument_class: class_argument,
                            object,
                            assigned: if (*class_argument).flags & VIPS_ARGUMENT_SET_ALWAYS != 0 {
                                glib_sys::GTRUE
                            } else {
                                glib_sys::GFALSE
                            },
                            close_id: 0,
                            invalidate_id: 0,
                        },
                    );
                    hash_table_replace(
                        object_ref.argument_table,
                        (*class_argument).parent.pspec.cast(),
                        instance_ptr.cast(),
                    );
                }
            }
            node = unsafe { (*node).next };
        }
    }

    unsafe { hash_table_lookup(object_ref.argument_table, (*class).parent.pspec.cast()) }
}

#[no_mangle]
pub extern "C" fn vips__argument_get_instance(
    argument_class: *mut VipsArgumentClass,
    object: *mut VipsObject,
) -> *mut VipsArgumentInstance {
    unsafe { argument_instance(object, argument_class) }
}

unsafe fn set_assigned(
    object: *mut VipsObject,
    pspec: *mut gobject_sys::GParamSpec,
    assigned: bool,
) {
    let class = unsafe { find_argument_class(object_class(object), pspec) };
    if class.is_null() {
        return;
    }
    let instance = unsafe { argument_instance(object, class) };
    if !instance.is_null() {
        unsafe {
            (*instance).assigned = bool_to_gboolean(assigned);
        }
    }
}

pub(crate) unsafe fn mark_argument_assigned(
    object: *mut VipsObject,
    name: &str,
    assigned: bool,
) -> Result<(), ()> {
    if object.is_null() {
        return Err(());
    }
    let name = CString::new(name).map_err(|_| ())?;
    let class = unsafe { object_class(object) };
    if class.is_null() {
        return Err(());
    }
    let pspec = unsafe { gobject_sys::g_object_class_find_property(class.cast(), name.as_ptr()) };
    if pspec.is_null() {
        return Err(());
    }
    unsafe {
        set_assigned(object, pspec, assigned);
    }
    Ok(())
}

#[no_mangle]
pub unsafe extern "C" fn safe_vips_object_mark_argument_assigned(
    object: *mut VipsObject,
    name: *const c_char,
    assigned: glib_sys::gboolean,
) -> c_int {
    if object.is_null() || name.is_null() {
        return -1;
    }
    let Ok(name) = unsafe { CStr::from_ptr(name) }.to_str() else {
        return -1;
    };
    match unsafe { mark_argument_assigned(object, name, assigned != glib_sys::GFALSE) } {
        Ok(()) => 0,
        Err(()) => -1,
    }
}

pub(crate) unsafe fn parse_enum_like(
    type_: glib_sys::GType,
    value: &CStr,
    flags: bool,
) -> Option<c_int> {
    let class = unsafe { gobject_sys::g_type_class_ref(type_) };
    if class.is_null() {
        return None;
    }

    let needle = value.to_string_lossy();
    if flags {
        let mut out = 0u32;
        for part in needle.split(':') {
            let part = part.trim();
            let flags_class = class.cast::<gobject_sys::GFlagsClass>();
            let mut found = None;
            for index in 0..unsafe { (*flags_class).n_values } {
                let item = unsafe { *(*flags_class).values.add(index as usize) };
                if item.value_nick.is_null() {
                    continue;
                }
                let nick = unsafe { CStr::from_ptr(item.value_nick) }.to_string_lossy();
                let name = unsafe { CStr::from_ptr(item.value_name) }.to_string_lossy();
                if nick.eq_ignore_ascii_case(part) || name.eq_ignore_ascii_case(part) {
                    found = Some(item.value);
                    break;
                }
            }
            out |= found?;
        }
        Some(out as c_int)
    } else {
        let enum_class = class.cast::<gobject_sys::GEnumClass>();
        for index in 0..unsafe { (*enum_class).n_values } {
            let item = unsafe { *(*enum_class).values.add(index as usize) };
            if item.value_nick.is_null() {
                continue;
            }
            let nick = unsafe { CStr::from_ptr(item.value_nick) }.to_string_lossy();
            let name = unsafe { CStr::from_ptr(item.value_name) }.to_string_lossy();
            if nick.eq_ignore_ascii_case(&needle) || name.eq_ignore_ascii_case(&needle) {
                return Some(item.value);
            }
        }
        None
    }
}

unsafe fn dynamic_from_gvalue(
    pspec: *mut gobject_sys::GParamSpec,
    value: *const gobject_sys::GValue,
) -> Option<DynamicValue> {
    if pspec.is_null() || value.is_null() {
        return None;
    }
    let value_type = unsafe { (*pspec).value_type };
    if value_type == gobject_sys::G_TYPE_NONE {
        return None;
    }

    if value_type == gobject_sys::G_TYPE_STRING {
        Some(DynamicValue::String(unsafe { gobject_sys::g_value_dup_string(value) }))
    } else if value_type == gobject_sys::G_TYPE_BOOLEAN {
        Some(DynamicValue::Bool(unsafe { gobject_sys::g_value_get_boolean(value) }))
    } else if value_type == gobject_sys::G_TYPE_INT {
        Some(DynamicValue::Int(unsafe { gobject_sys::g_value_get_int(value) }))
    } else if value_type == gobject_sys::G_TYPE_UINT64 {
        Some(DynamicValue::UInt64(unsafe { gobject_sys::g_value_get_uint64(value) }))
    } else if value_type == gobject_sys::G_TYPE_DOUBLE {
        Some(DynamicValue::Double(unsafe { gobject_sys::g_value_get_double(value) }))
    } else if value_type == gobject_sys::G_TYPE_POINTER {
        Some(DynamicValue::Pointer(unsafe { gobject_sys::g_value_get_pointer(value) }))
    } else if unsafe { gobject_sys::g_type_is_a(value_type, gobject_sys::G_TYPE_OBJECT) }
        != glib_sys::GFALSE
    {
        Some(DynamicValue::Object(unsafe { gobject_sys::g_value_dup_object(value) }))
    } else if unsafe { gobject_sys::g_type_is_a(value_type, gobject_sys::G_TYPE_ENUM) }
        != glib_sys::GFALSE
    {
        Some(DynamicValue::Enum(unsafe { gobject_sys::g_value_get_enum(value) }))
    } else if unsafe { gobject_sys::g_type_is_a(value_type, gobject_sys::G_TYPE_FLAGS) }
        != glib_sys::GFALSE
    {
        Some(DynamicValue::Flags(unsafe { gobject_sys::g_value_get_flags(value) }))
    } else if unsafe { gobject_sys::g_type_is_a(value_type, gobject_sys::G_TYPE_BOXED) }
        != glib_sys::GFALSE
    {
        Some(DynamicValue::Boxed {
            gtype: value_type,
            value: unsafe { gobject_sys::g_value_dup_boxed(value) },
        })
    } else {
        None
    }
}

unsafe fn set_gvalue_from_dynamic(value: *mut gobject_sys::GValue, dynamic: &DynamicValue) {
    match dynamic {
        DynamicValue::String(text) => unsafe {
            gobject_sys::g_value_set_string(value, *text);
        },
        DynamicValue::Object(object) => unsafe {
            gobject_sys::g_value_set_object(value, *object);
        },
        DynamicValue::Int(number) => unsafe {
            gobject_sys::g_value_set_int(value, *number);
        },
        DynamicValue::UInt64(number) => unsafe {
            gobject_sys::g_value_set_uint64(value, *number);
        },
        DynamicValue::Bool(flag) => unsafe {
            gobject_sys::g_value_set_boolean(value, *flag);
        },
        DynamicValue::Enum(number) => unsafe {
            gobject_sys::g_value_set_enum(value, *number);
        },
        DynamicValue::Flags(bits) => unsafe {
            gobject_sys::g_value_set_flags(value, *bits);
        },
        DynamicValue::Pointer(pointer) => unsafe {
            gobject_sys::g_value_set_pointer(value, *pointer);
        },
        DynamicValue::Double(number) => unsafe {
            gobject_sys::g_value_set_double(value, *number);
        },
        DynamicValue::Boxed { value: boxed, .. } => unsafe {
            gobject_sys::g_value_set_boxed(value, *boxed);
        },
    }
}

unsafe fn append_text(buf: *mut VipsBuf, text: String) {
    if let Ok(text) = CString::new(text) {
        crate::runtime::buf::vips_buf_appends(buf, text.as_ptr());
    }
}

unsafe fn check_required_inputs(object: *mut VipsObject) -> bool {
    let class = unsafe { object_class(object) };
    if class.is_null() {
        return true;
    }
    let mut ok = true;
    let mut node = unsafe { (*class).argument_table_traverse };
    while !node.is_null() {
        let argument = unsafe { (*node).data.cast::<VipsArgumentClass>() };
        if !argument.is_null() {
            let instance = unsafe { argument_instance(object, argument) };
            let required = unsafe { (*argument).flags & VIPS_ARGUMENT_REQUIRED != 0 };
            let input = unsafe { (*argument).flags & VIPS_ARGUMENT_INPUT != 0 };
            let deprecated = unsafe { (*argument).flags & VIPS_ARGUMENT_DEPRECATED != 0 };
            let assigned = !instance.is_null() && unsafe { (*instance).assigned != glib_sys::GFALSE };
            if required && input && !deprecated && !assigned {
                ok = false;
                let name =
                    unsafe { CStr::from_ptr(gobject_sys::g_param_spec_get_name((*argument).parent.pspec)) };
                let domain = unsafe {
                    if (*class).nickname.is_null() {
                        "VipsObject".to_owned()
                    } else {
                        CStr::from_ptr((*class).nickname)
                            .to_string_lossy()
                            .into_owned()
                    }
                };
                append_message_str(&domain, &format!("parameter {} not set", name.to_string_lossy()));
            }
        }
        node = unsafe { (*node).next };
    }
    ok
}

unsafe extern "C" fn vips_object_instance_init(
    instance: *mut gobject_sys::GTypeInstance,
    _klass: glib_sys::gpointer,
) {
    let object = instance.cast::<VipsObject>();
    unsafe {
        set_qdata_box(object.cast(), object_state_quark(), ObjectState::default());
        (*object).constructed = glib_sys::GFALSE;
        (*object).static_object = glib_sys::GFALSE;
        (*object).argument_table = ptr::null_mut();
        (*object).nickname = ptr::null_mut();
        (*object).description = ptr::null_mut();
        (*object).preclose = glib_sys::GFALSE;
        (*object).close = glib_sys::GFALSE;
        (*object).postclose = glib_sys::GFALSE;
        (*object).local_memory = 0;
    }
    object_registry()
        .lock()
        .expect("object registry")
        .insert(object as usize);
}

unsafe extern "C" fn vips_object_real_build(object: *mut VipsObject) -> c_int {
    if object.is_null() {
        return -1;
    }
    let class = unsafe { object_class(object) };
    if class.is_null() {
        return -1;
    }
    if unsafe { (*object).nickname.is_null() && !(*class).nickname.is_null() } {
        unsafe {
            (*object).nickname = glib_sys::g_strdup((*class).nickname);
        }
    }
    if unsafe { (*object).description.is_null() && !(*class).description.is_null() } {
        unsafe {
            (*object).description = glib_sys::g_strdup((*class).description);
        }
    }
    if unsafe { check_required_inputs(object) } {
        0
    } else {
        -1
    }
}

pub(crate) unsafe fn default_object_build(object: *mut VipsObject) -> c_int {
    unsafe { vips_object_real_build(object) }
}

unsafe extern "C" fn vips_object_real_postbuild(
    _object: *mut VipsObject,
    _data: *mut c_void,
) -> c_int {
    0
}

unsafe extern "C" fn vips_object_real_summary_class(class: *mut VipsObjectClass, buf: *mut VipsBuf) {
    if class.is_null() || buf.is_null() {
        return;
    }
    let type_name = unsafe {
        CStr::from_ptr(gobject_sys::g_type_name(
            (*class.cast::<gobject_sys::GTypeClass>()).g_type,
        ))
        .to_string_lossy()
        .into_owned()
    };
    let nickname = unsafe { CStr::from_ptr((*class).nickname).to_string_lossy().into_owned() };
    let description =
        unsafe { CStr::from_ptr((*class).description).to_string_lossy().into_owned() };
    unsafe { append_text(buf, format!("{type_name} ({nickname}), {description}")) };
}

unsafe extern "C" fn vips_object_real_summary(object: *mut VipsObject, buf: *mut VipsBuf) {
    if object.is_null() || buf.is_null() {
        return;
    }
    let class = unsafe { object_class(object) };
    if class.is_null() {
        return;
    }
    let text = unsafe {
        CStr::from_ptr(if (*object).nickname.is_null() {
            (*class).nickname
        } else {
            (*object).nickname
        })
        .to_string_lossy()
        .into_owned()
    };
    unsafe { append_text(buf, text) };
}

unsafe extern "C" fn vips_object_real_dump(object: *mut VipsObject, buf: *mut VipsBuf) {
    if object.is_null() || buf.is_null() {
        return;
    }
    let class = unsafe { object_class(object) };
    if class.is_null() {
        return;
    }
    let type_name = unsafe {
        CStr::from_ptr(gobject_sys::g_type_name(
            (*class.cast::<gobject_sys::GTypeClass>()).g_type,
        ))
        .to_string_lossy()
        .into_owned()
    };
    unsafe { append_text(buf, format!("{type_name} ({:p})", object.cast::<c_void>())) };
}

unsafe extern "C" fn vips_object_real_sanity(_object: *mut VipsObject, _buf: *mut VipsBuf) {}

unsafe extern "C" fn vips_object_real_rewind(object: *mut VipsObject) {
    if let Some(object) = unsafe { object.as_mut() } {
        object.constructed = glib_sys::GFALSE;
        object.preclose = glib_sys::GFALSE;
        object.close = glib_sys::GFALSE;
        object.postclose = glib_sys::GFALSE;
    }
}

unsafe extern "C" fn vips_object_real_new_from_string(string: *const c_char) -> *mut VipsObject {
    if string.is_null() {
        return ptr::null_mut();
    }
    let type_ = vips_type_find(ptr::null(), string);
    if type_ == 0 {
        return ptr::null_mut();
    }
    unsafe { object_new::<VipsObject>(type_) }
}

unsafe extern "C" fn vips_object_real_to_string(object: *mut VipsObject, buf: *mut VipsBuf) {
    if object.is_null() || buf.is_null() {
        return;
    }
    let class = unsafe { object_class(object) };
    if class.is_null() {
        return;
    }
    unsafe {
        crate::runtime::buf::vips_buf_appends(
            buf,
            if (*object).nickname.is_null() {
                (*class).nickname
            } else {
                (*object).nickname
            },
        );
    }
}

unsafe extern "C" fn vips_object_dispose(gobject: *mut gobject_sys::GObject) {
    let object = gobject.cast::<VipsObject>();
    if !object.is_null() && unsafe { (*object).argument_table } != ptr::null_mut() {
        unsafe {
            argument_table_destroy((*object).argument_table);
            (*object).argument_table = ptr::null_mut();
        }
    }

    let parent_class = vips_object_parent_class();
    if !parent_class.is_null() {
        if let Some(dispose) = unsafe { (*parent_class).dispose } {
            unsafe {
                dispose(gobject);
            }
        }
    }
}

unsafe extern "C" fn vips_object_finalize(gobject: *mut gobject_sys::GObject) {
    object_registry()
        .lock()
        .expect("object registry")
        .remove(&(gobject as usize));

    let object = gobject.cast::<VipsObject>();
    if !object.is_null() {
        unsafe {
            glib_sys::g_free((*object).nickname.cast());
            glib_sys::g_free((*object).description.cast());
            (*object).nickname = ptr::null_mut();
            (*object).description = ptr::null_mut();
        }
    }

    let parent_class = vips_object_parent_class();
    if !parent_class.is_null() {
        if let Some(finalize) = unsafe { (*parent_class).finalize } {
            unsafe {
                finalize(gobject);
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn vips_value_is_null(
    pspec: *mut gobject_sys::GParamSpec,
    value: *const gobject_sys::GValue,
) -> glib_sys::gboolean {
    if pspec.is_null() || value.is_null() {
        return glib_sys::GTRUE;
    }
    unsafe { gobject_sys::g_param_value_defaults(pspec, value) }
}

#[no_mangle]
pub unsafe extern "C" fn vips_object_set_property(
    gobject: *mut gobject_sys::GObject,
    _property_id: c_uint,
    value: *mut gobject_sys::GValue,
    pspec: *mut gobject_sys::GParamSpec,
) {
    let object = gobject.cast::<VipsObject>();
    if object.is_null() || pspec.is_null() || value.is_null() {
        return;
    }

    let name = unsafe { CStr::from_ptr(gobject_sys::g_param_spec_get_name(pspec)) }
        .to_string_lossy()
        .into_owned();
    let value_type = unsafe { (*pspec).value_type };

    if name == "nickname" && value_type == gobject_sys::G_TYPE_STRING {
        unsafe {
            glib_sys::g_free((*object).nickname.cast());
            (*object).nickname = gobject_sys::g_value_dup_string(value);
        }
        return;
    }
    if name == "description" && value_type == gobject_sys::G_TYPE_STRING {
        unsafe {
            glib_sys::g_free((*object).description.cast());
            (*object).description = gobject_sys::g_value_dup_string(value);
        }
        return;
    }

    if let Some(dynamic) = unsafe { dynamic_from_gvalue(pspec, value) } {
        unsafe {
            set_dynamic_value(object, &name, dynamic);
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn vips_object_get_property(
    gobject: *mut gobject_sys::GObject,
    _property_id: c_uint,
    value: *mut gobject_sys::GValue,
    pspec: *mut gobject_sys::GParamSpec,
) {
    let object = gobject.cast::<VipsObject>();
    if object.is_null() || pspec.is_null() || value.is_null() {
        return;
    }

    let name = unsafe { CStr::from_ptr(gobject_sys::g_param_spec_get_name(pspec)) }
        .to_string_lossy()
        .into_owned();
    if name == "nickname" {
        unsafe {
            gobject_sys::g_value_set_string(value, (*object).nickname);
        }
        return;
    }
    if name == "description" {
        unsafe {
            gobject_sys::g_value_set_string(value, (*object).description);
        }
        return;
    }

    if let Some(dynamic) = unsafe { dynamic_value(object, &name) } {
        unsafe {
            set_gvalue_from_dynamic(value, dynamic);
        }
    } else {
        unsafe {
            gobject_sys::g_param_value_set_default(pspec, value);
        }
    }
}

unsafe extern "C" fn vips_object_class_init(
    klass: glib_sys::gpointer,
    _class_data: glib_sys::gpointer,
) {
    let class = unsafe { &mut *klass.cast::<VipsObjectClass>() };
    let gobject_class = klass.cast::<gobject_sys::GObjectClass>();
    unsafe {
        (*gobject_class).dispose = Some(vips_object_dispose);
        (*gobject_class).finalize = Some(vips_object_finalize);
        (*gobject_class).set_property = Some(vips_object_set_property);
        (*gobject_class).get_property = Some(vips_object_get_property);
    }

    class.build = Some(vips_object_real_build);
    class.postbuild = Some(vips_object_real_postbuild);
    class.summary_class = Some(vips_object_real_summary_class);
    class.summary = Some(vips_object_real_summary);
    class.dump = Some(vips_object_real_dump);
    class.sanity = Some(vips_object_real_sanity);
    class.rewind = Some(vips_object_real_rewind);
    class.new_from_string = Some(vips_object_real_new_from_string);
    class.to_string = Some(vips_object_real_to_string);
    class.output_needs_arg = glib_sys::GFALSE;
    class.output_to_arg = None;
    class.nickname = OBJECT_NICKNAME.as_ptr().cast();
    class.description = OBJECT_DESCRIPTION.as_ptr().cast();
    class.argument_table = unsafe { argument_table_new(true) };
    class.argument_table_traverse = ptr::null_mut();
    class.argument_table_traverse_gtype =
        unsafe { (*(klass.cast::<gobject_sys::GTypeClass>())).g_type };
    class.deprecated = glib_sys::GFALSE;

    let nickname = unsafe {
        gobject_sys::g_param_spec_string(
            c"nickname".as_ptr(),
            c"Nickname".as_ptr(),
            c"Class nickname".as_ptr(),
            ptr::null(),
            gobject_sys::G_PARAM_READWRITE,
        )
    };
    unsafe {
        gobject_sys::g_object_class_install_property(
            gobject_class,
            vips_argument_get_id() as u32,
            nickname,
        );
    }
    vips_object_class_install_argument(
        class,
        nickname,
        VIPS_ARGUMENT_SET_ONCE,
        1,
        offset_of!(VipsObject, nickname) as c_uint,
    );

    let description = unsafe {
        gobject_sys::g_param_spec_string(
            c"description".as_ptr(),
            c"Description".as_ptr(),
            c"Class description".as_ptr(),
            ptr::null(),
            gobject_sys::G_PARAM_READWRITE,
        )
    };
    unsafe {
        gobject_sys::g_object_class_install_property(
            gobject_class,
            vips_argument_get_id() as u32,
            description,
        );
    }
    vips_object_class_install_argument(
        class,
        description,
        VIPS_ARGUMENT_SET_ONCE,
        2,
        offset_of!(VipsObject, description) as c_uint,
    );
}

#[no_mangle]
pub extern "C" fn vips_object_preclose(object: *mut VipsObject) {
    if object.is_null() {
        return;
    }
    let class = unsafe { object_class(object) };
    if class.is_null() {
        return;
    }
    if unsafe { (*object).preclose != glib_sys::GFALSE } {
        return;
    }
    unsafe {
        (*object).preclose = glib_sys::GTRUE;
        if let Some(preclose) = (*class).preclose {
            preclose(object);
        }
    }
}

#[no_mangle]
pub extern "C" fn vips_object_build(object: *mut VipsObject) -> c_int {
    if object.is_null() {
        return -1;
    }
    if unsafe { (*object).constructed != glib_sys::GFALSE } {
        return 0;
    }
    let class = unsafe { object_class(object) };
    if class.is_null() {
        return -1;
    }
    if let Some(build) = unsafe { (*class).build } {
        if unsafe { build(object) } != 0 {
            return -1;
        }
    }
    if let Some(postbuild) = unsafe { (*class).postbuild } {
        if unsafe { postbuild(object, ptr::null_mut()) } != 0 {
            return -1;
        }
    }
    unsafe {
        (*object).constructed = glib_sys::GTRUE;
    }
    0
}

#[no_mangle]
pub extern "C" fn vips_object_summary_class(class: *mut VipsObjectClass, buf: *mut VipsBuf) {
    if class.is_null() || buf.is_null() {
        return;
    }
    if let Some(summary) = unsafe { (*class).summary_class } {
        unsafe {
            summary(class, buf);
        }
    }
}

#[no_mangle]
pub extern "C" fn vips_object_summary(object: *mut VipsObject, buf: *mut VipsBuf) {
    if object.is_null() || buf.is_null() {
        return;
    }
    let class = unsafe { object_class(object) };
    if class.is_null() {
        return;
    }
    if let Some(summary) = unsafe { (*class).summary } {
        unsafe {
            summary(object, buf);
        }
    }
}

#[no_mangle]
pub extern "C" fn vips_object_dump(object: *mut VipsObject, buf: *mut VipsBuf) {
    if object.is_null() || buf.is_null() {
        return;
    }
    let class = unsafe { object_class(object) };
    if class.is_null() {
        return;
    }
    if let Some(dump) = unsafe { (*class).dump } {
        unsafe {
            dump(object, buf);
        }
    }
}

unsafe fn print_buf_with<F>(mut fill: F)
where
    F: FnMut(*mut VipsBuf),
{
    let mut buf = VipsBuf {
        base: ptr::null_mut(),
        mx: 0,
        i: 0,
        full: glib_sys::GFALSE,
        lasti: 0,
        dynamic: glib_sys::GFALSE,
    };
    crate::runtime::buf::vips_buf_init_dynamic(&mut buf, 1024);
    fill(&mut buf);
    let text = crate::runtime::buf::vips_buf_all(&mut buf);
    if !text.is_null() {
        unsafe {
            libc::printf(c"%s\n".as_ptr(), text);
        }
    }
    crate::runtime::buf::vips_buf_destroy(&mut buf);
}

#[no_mangle]
pub extern "C" fn vips_object_print_summary_class(class: *mut VipsObjectClass) {
    unsafe {
        print_buf_with(|buf| vips_object_summary_class(class, buf));
    }
}

#[no_mangle]
pub extern "C" fn vips_object_print_summary(object: *mut VipsObject) {
    unsafe {
        print_buf_with(|buf| vips_object_summary(object, buf));
    }
}

#[no_mangle]
pub extern "C" fn vips_object_print_dump(object: *mut VipsObject) {
    unsafe {
        print_buf_with(|buf| vips_object_dump(object, buf));
    }
}

#[no_mangle]
pub extern "C" fn vips_object_print_name(object: *mut VipsObject) {
    if object.is_null() {
        return;
    }
    let text = unsafe {
        if !(*object).nickname.is_null() {
            (*object).nickname
        } else {
            let class = object_class(object);
            if class.is_null() {
                ptr::null()
            } else {
                (*class).nickname
            }
        }
    };
    if !text.is_null() {
        unsafe {
            libc::printf(c"%s".as_ptr(), text);
        }
    }
}

#[no_mangle]
pub extern "C" fn vips_object_sanity(object: *mut VipsObject) -> glib_sys::gboolean {
    if object.is_null() {
        return glib_sys::GFALSE;
    }
    let class = unsafe { object_class(object) };
    if class.is_null() {
        return glib_sys::GFALSE;
    }
    if let Some(sanity) = unsafe { (*class).sanity } {
        unsafe {
            print_buf_with(|buf| sanity(object, buf));
        }
    }
    glib_sys::GTRUE
}

#[no_mangle]
pub extern "C" fn vips_object_class_install_argument(
    class: *mut VipsObjectClass,
    pspec: *mut gobject_sys::GParamSpec,
    flags: VipsArgumentFlags,
    priority: c_int,
    offset: c_uint,
) {
    if class.is_null() || pspec.is_null() {
        return;
    }
    unsafe {
        init_subclass_class(class);
        let argument = glib_sys::g_malloc0(size_of::<VipsArgumentClass>()).cast::<VipsArgumentClass>();
        ptr::write(
            argument,
            VipsArgumentClass {
                parent: VipsArgument { pspec },
                object_class: class,
                flags,
                priority,
                offset,
            },
        );
        hash_table_replace((*class).argument_table, pspec.cast(), argument.cast());
        (*class).argument_table_traverse =
            glib_sys::g_slist_insert_sorted(
                (*class).argument_table_traverse,
                argument.cast(),
                Some(argument_priority_compare),
            );
    }
}

unsafe extern "C" fn argument_priority_compare(
    a: glib_sys::gconstpointer,
    b: glib_sys::gconstpointer,
) -> c_int {
    let left = unsafe { &*a.cast::<VipsArgumentClass>() };
    let right = unsafe { &*b.cast::<VipsArgumentClass>() };
    left.priority - right.priority
}

unsafe fn parse_string_for_value(
    pspec: *mut gobject_sys::GParamSpec,
    text: &CStr,
    value: *mut gobject_sys::GValue,
) -> Result<(), ()> {
    let value_type = unsafe { (*pspec).value_type };
    unsafe {
        gobject_sys::g_value_init(value, value_type);
    }
    if value_type == gobject_sys::G_TYPE_STRING {
        unsafe {
            gobject_sys::g_value_set_string(value, text.as_ptr());
        }
        Ok(())
    } else if value_type == gobject_sys::G_TYPE_BOOLEAN {
        let parsed = matches!(
            text.to_string_lossy().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        );
        unsafe {
            gobject_sys::g_value_set_boolean(value, bool_to_gboolean(parsed));
        }
        Ok(())
    } else if value_type == gobject_sys::G_TYPE_INT {
        let parsed = text.to_string_lossy().parse::<c_int>().map_err(|_| ())?;
        unsafe {
            gobject_sys::g_value_set_int(value, parsed);
        }
        Ok(())
    } else if value_type == gobject_sys::G_TYPE_UINT64 {
        let parsed = text.to_string_lossy().parse::<u64>().map_err(|_| ())?;
        unsafe {
            gobject_sys::g_value_set_uint64(value, parsed);
        }
        Ok(())
    } else if value_type == gobject_sys::G_TYPE_DOUBLE {
        let parsed = text.to_string_lossy().parse::<f64>().map_err(|_| ())?;
        unsafe {
            gobject_sys::g_value_set_double(value, parsed);
        }
        Ok(())
    } else if unsafe { gobject_sys::g_type_is_a(value_type, gobject_sys::G_TYPE_ENUM) }
        != glib_sys::GFALSE
    {
        let parsed = unsafe { parse_enum_like(value_type, text, false) }.ok_or(())?;
        unsafe {
            gobject_sys::g_value_set_enum(value, parsed);
        }
        Ok(())
    } else if unsafe { gobject_sys::g_type_is_a(value_type, gobject_sys::G_TYPE_FLAGS) }
        != glib_sys::GFALSE
    {
        let parsed = unsafe { parse_enum_like(value_type, text, true) }.ok_or(())?;
        unsafe {
            gobject_sys::g_value_set_flags(value, parsed as u32);
        }
        Ok(())
    } else {
        Err(())
    }
}

#[no_mangle]
pub extern "C" fn vips_object_set_argument_from_string(
    object: *mut VipsObject,
    name: *const c_char,
    value: *const c_char,
) -> c_int {
    if object.is_null() || name.is_null() || value.is_null() {
        return -1;
    }
    let class = unsafe { object_class(object) };
    if class.is_null() {
        return -1;
    }
    let pspec = unsafe {
        gobject_sys::g_object_class_find_property(class.cast(), name)
    };
    if pspec.is_null() {
        append_message_str(
            "vips_object_set_argument_from_string",
            &format!(
                "unknown argument {}",
                unsafe { CStr::from_ptr(name) }.to_string_lossy()
            ),
        );
        return -1;
    }

    let mut gvalue: gobject_sys::GValue = unsafe { std::mem::zeroed() };
    let input = unsafe { CStr::from_ptr(value) };
    let result = unsafe { parse_string_for_value(pspec, input, &mut gvalue) };
    if result.is_err() {
        append_message_str(
            "vips_object_set_argument_from_string",
            &format!(
                "unable to parse {} for {}",
                input.to_string_lossy(),
                unsafe { CStr::from_ptr(name) }.to_string_lossy()
            ),
        );
        return -1;
    }
    unsafe {
        gobject_sys::g_object_set_property(object.cast(), name, &gvalue);
        let _ = mark_argument_assigned(
            object,
            &CStr::from_ptr(name).to_string_lossy(),
            true,
        );
        gobject_sys::g_value_unset(&mut gvalue);
    }
    0
}

#[no_mangle]
pub extern "C" fn vips_object_argument_needsstring(
    object: *mut VipsObject,
    name: *const c_char,
) -> glib_sys::gboolean {
    if object.is_null() || name.is_null() {
        return glib_sys::GFALSE;
    }
    let class = unsafe { object_class(object) };
    if class.is_null() {
        return glib_sys::GFALSE;
    }
    let pspec = unsafe { gobject_sys::g_object_class_find_property(class.cast(), name) };
    if pspec.is_null() {
        return glib_sys::GFALSE;
    }
    let argument = unsafe { find_argument_class(class, pspec) };
    if argument.is_null() {
        return glib_sys::GFALSE;
    }
    let value_type = unsafe { (*pspec).value_type };
    let needs = unsafe { (*argument).flags & VIPS_ARGUMENT_OUTPUT != 0 }
        && (value_type == gobject_sys::G_TYPE_STRING
            || value_type == gobject_sys::G_TYPE_POINTER);
    bool_to_gboolean(needs)
}

#[no_mangle]
pub extern "C" fn vips_object_get_argument_to_string(
    object: *mut VipsObject,
    name: *const c_char,
    _arg: *const c_char,
) -> c_int {
    if object.is_null() || name.is_null() {
        return -1;
    }
    let name = unsafe { CStr::from_ptr(name) }.to_string_lossy().into_owned();
    if let Some(value) = unsafe { dynamic_value(object, &name) } {
        match value {
            DynamicValue::String(text) => unsafe {
                if !text.is_null() {
                    libc::printf(c"%s\n".as_ptr(), *text);
                }
            },
            DynamicValue::Int(number) => unsafe {
                libc::printf(c"%d\n".as_ptr(), *number);
            },
            DynamicValue::UInt64(number) => unsafe {
                libc::printf(c"%llu\n".as_ptr(), *number);
            },
            DynamicValue::Bool(flag) => unsafe {
                libc::printf(c"%s\n".as_ptr(), if *flag == glib_sys::GFALSE { c"false".as_ptr() } else { c"true".as_ptr() });
            },
            DynamicValue::Enum(number) => unsafe {
                libc::printf(c"%d\n".as_ptr(), *number);
            },
            DynamicValue::Flags(bits) => unsafe {
                libc::printf(c"%u\n".as_ptr(), *bits);
            },
            DynamicValue::Double(number) => unsafe {
                libc::printf(c"%g\n".as_ptr(), *number);
            },
            DynamicValue::Object(object) => unsafe {
                vips_object_print_summary((*object).cast());
            },
            DynamicValue::Pointer(pointer) => unsafe {
                libc::printf(c"%p\n".as_ptr(), *pointer);
            },
            DynamicValue::Boxed { value, .. } => unsafe {
                libc::printf(c"%p\n".as_ptr(), *value);
            },
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn vips_object_set_required(
    _object: *mut VipsObject,
    _value: *const c_char,
) -> c_int {
    0
}

#[no_mangle]
pub extern "C" fn vips_object_new(
    type_: glib_sys::GType,
    set: VipsObjectSetArguments,
    a: *mut c_void,
    b: *mut c_void,
) -> *mut VipsObject {
    let object = unsafe { object_new::<VipsObject>(type_) };
    if object.is_null() {
        return ptr::null_mut();
    }
    if let Some(set) = set {
        let _ = unsafe { set(object, a, b) };
    }
    if vips_object_build(object) != 0 {
        unsafe {
            object_unref(object);
        }
        return ptr::null_mut();
    }
    object
}

#[no_mangle]
pub extern "C" fn vips_object_get_argument(
    object: *mut VipsObject,
    name: *const c_char,
    pspec_out: *mut *mut gobject_sys::GParamSpec,
    argument_class_out: *mut *mut VipsArgumentClass,
    argument_instance_out: *mut *mut VipsArgumentInstance,
) -> c_int {
    if object.is_null() || name.is_null() {
        append_message_str("vips_object_get_argument", "object or name is NULL");
        return -1;
    }

    let class = unsafe { object_class(object) };
    if class.is_null() {
        append_message_str("vips_object_get_argument", "object class is NULL");
        return -1;
    }

    let pspec =
        unsafe { gobject_sys::g_object_class_find_property(class.cast(), name) };
    if pspec.is_null() {
        append_message_str(
            "vips_object_get_argument",
            &format!(
                "unknown argument {}",
                unsafe { CStr::from_ptr(name) }.to_string_lossy()
            ),
        );
        return -1;
    }

    let argument_class = unsafe { find_argument_class(class, pspec) };
    if argument_class.is_null() {
        append_message_str(
            "vips_object_get_argument",
            &format!(
                "argument metadata missing for {}",
                unsafe { CStr::from_ptr(gobject_sys::g_param_spec_get_name(pspec)) }
                    .to_string_lossy()
            ),
        );
        return -1;
    }

    let argument_instance = unsafe { argument_instance(object, argument_class) };
    if !pspec_out.is_null() {
        unsafe {
            *pspec_out = pspec;
        }
    }
    if !argument_class_out.is_null() {
        unsafe {
            *argument_class_out = argument_class;
        }
    }
    if !argument_instance_out.is_null() {
        unsafe {
            *argument_instance_out = argument_instance;
        }
    }

    0
}

#[no_mangle]
pub extern "C" fn vips_object_new_from_string(
    object_class: *mut VipsObjectClass,
    string: *const c_char,
) -> *mut VipsObject {
    if object_class.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        (*object_class)
            .new_from_string
            .map(|new_from_string| new_from_string(string))
            .unwrap_or(ptr::null_mut())
    }
}

#[no_mangle]
pub extern "C" fn vips_object_to_string(object: *mut VipsObject, buf: *mut VipsBuf) {
    if object.is_null() || buf.is_null() {
        return;
    }
    let class = unsafe { object_class(object) };
    if class.is_null() {
        return;
    }
    if let Some(to_string) = unsafe { (*class).to_string } {
        unsafe {
            to_string(object, buf);
        }
    }
}

#[no_mangle]
pub extern "C" fn vips_object_map(
    fn_: VipsSListMap2Fn,
    a: *mut c_void,
    b: *mut c_void,
) -> *mut c_void {
    let Some(fn_) = fn_ else {
        return ptr::null_mut();
    };
    let live: Vec<usize> = object_registry()
        .lock()
        .expect("object registry")
        .iter()
        .copied()
        .collect();
    for object in live {
        let result = unsafe { fn_(object as *mut c_void, a, b) };
        if !result.is_null() {
            return result;
        }
    }
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn vips_type_map(
    base: glib_sys::GType,
    fn_: VipsTypeMap2Fn,
    a: *mut c_void,
    b: *mut c_void,
) -> *mut c_void {
    let Some(fn_) = fn_ else {
        return ptr::null_mut();
    };
    let mut count = 0u32;
    let children = unsafe { gobject_sys::g_type_children(base, &mut count) };
    if children.is_null() {
        return ptr::null_mut();
    }
    let slice = unsafe { std::slice::from_raw_parts(children, count as usize) };
    let mut result = ptr::null_mut();
    for child in slice {
        result = unsafe { fn_(*child, a, b) };
        if !result.is_null() {
            break;
        }
    }
    unsafe {
        glib_sys::g_free(children.cast());
    }
    result
}

unsafe extern "C" fn vips_type_map_all_cb(
    type_: glib_sys::GType,
    a: *mut c_void,
    b: *mut c_void,
) -> *mut c_void {
    let fn_: VipsTypeMapFn = unsafe { std::mem::transmute(a) };
    if let Some(fn_) = fn_ {
        let result = unsafe { fn_(type_, b) };
        if !result.is_null() {
            return result;
        }
    }
    vips_type_map(type_, Some(vips_type_map_all_cb), a, b)
}

#[no_mangle]
pub extern "C" fn vips_type_map_all(
    base: glib_sys::GType,
    fn_: VipsTypeMapFn,
    a: *mut c_void,
) -> *mut c_void {
    let Some(fn_) = fn_ else {
        return ptr::null_mut();
    };
    let result = unsafe { fn_(base, a) };
    if !result.is_null() {
        return result;
    }
    unsafe { vips_type_map(base, Some(vips_type_map_all_cb), std::mem::transmute(fn_), a) }
}

#[no_mangle]
pub extern "C" fn vips_type_depth(mut type_: glib_sys::GType) -> c_int {
    let mut depth = 0;
    while type_ != vips_object_get_type() && type_ != 0 {
        depth += 1;
        type_ = unsafe { gobject_sys::g_type_parent(type_) };
    }
    depth
}

unsafe extern "C" fn class_map_all_cb(
    type_: glib_sys::GType,
    a: *mut c_void,
    b: *mut c_void,
) -> *mut c_void {
    let fn_: VipsClassMapFn = unsafe { std::mem::transmute(a) };
    if unsafe { gobject_sys::g_type_test_flags(type_, gobject_sys::G_TYPE_FLAG_ABSTRACT) }
        == glib_sys::GFALSE
    {
        if let Some(fn_) = fn_ {
            let class = unsafe { object_class_for_type(type_) };
            let result = unsafe { fn_(class, b) };
            if !result.is_null() {
                return result;
            }
        }
    }
    vips_type_map(type_, Some(class_map_all_cb), a, b)
}

#[no_mangle]
pub extern "C" fn vips_class_map_all(
    type_: glib_sys::GType,
    fn_: VipsClassMapFn,
    a: *mut c_void,
) -> *mut c_void {
    let Some(fn_) = fn_ else {
        return ptr::null_mut();
    };
    if unsafe { gobject_sys::g_type_test_flags(type_, gobject_sys::G_TYPE_FLAG_ABSTRACT) }
        == glib_sys::GFALSE
    {
        let class = unsafe { object_class_for_type(type_) };
        let result = unsafe { fn_(class, a) };
        if !result.is_null() {
            return result;
        }
    }
    unsafe { vips_type_map(type_, Some(class_map_all_cb), std::mem::transmute(fn_), a) }
}

unsafe extern "C" fn test_name_cb(class: *mut VipsObjectClass, name: *mut c_void) -> *mut c_void {
    if class.is_null() || name.is_null() {
        return ptr::null_mut();
    }
    let needle = unsafe { CStr::from_ptr(name.cast::<c_char>()) }.to_string_lossy();
    let nickname = unsafe {
        if (*class).nickname.is_null() {
            String::new()
        } else {
            CStr::from_ptr((*class).nickname).to_string_lossy().into_owned()
        }
    };
    let type_name = unsafe {
        CStr::from_ptr(gobject_sys::g_type_name(
            (*class.cast::<gobject_sys::GTypeClass>()).g_type,
        ))
        .to_string_lossy()
        .into_owned()
    };
    if nickname.eq_ignore_ascii_case(&needle) || type_name.eq_ignore_ascii_case(&needle) {
        class.cast()
    } else {
        ptr::null_mut()
    }
}

#[no_mangle]
pub extern "C" fn vips_class_find(
    basename: *const c_char,
    nickname: *const c_char,
) -> *const VipsObjectClass {
    if nickname.is_null() {
        return ptr::null();
    }
    let classname = if basename.is_null() {
        "VipsObject"
    } else {
        unsafe { CStr::from_ptr(basename) }.to_str().unwrap_or("VipsObject")
    };
    let Ok(classname) = CString::new(classname) else {
        return ptr::null();
    };
    let base = unsafe { gobject_sys::g_type_from_name(classname.as_ptr()) };
    if base == 0 {
        return ptr::null();
    }
    vips_class_map_all(base, Some(test_name_cb), nickname as *mut c_void).cast()
}

#[no_mangle]
pub extern "C" fn vips_type_find(
    basename: *const c_char,
    nickname: *const c_char,
) -> glib_sys::GType {
    let class = vips_class_find(basename, nickname);
    if class.is_null() {
        0
    } else {
        unsafe { (*class.cast::<gobject_sys::GTypeClass>()).g_type }
    }
}

#[no_mangle]
pub extern "C" fn vips_nickname_find(type_: glib_sys::GType) -> *const c_char {
    let class = unsafe { object_class_for_type(type_) };
    if class.is_null() {
        ptr::null()
    } else {
        unsafe { (*class).nickname }
    }
}

#[no_mangle]
pub extern "C" fn vips_object_local_array(
    _parent: *mut VipsObject,
    n: c_int,
) -> *mut *mut VipsObject {
    if n <= 0 {
        return ptr::null_mut();
    }
    unsafe { glib_sys::g_malloc0((n as usize + 1) * size_of::<*mut VipsObject>()) }.cast()
}

#[no_mangle]
pub extern "C" fn vips_object_local_cb(
    _vobject: *mut VipsObject,
    gobject: *mut gobject_sys::GObject,
) {
    if !gobject.is_null() {
        unsafe {
            gobject_sys::g_object_unref(gobject);
        }
    }
}

#[no_mangle]
pub extern "C" fn vips_object_set_static(object: *mut VipsObject, static_object: glib_sys::gboolean) {
    if let Some(object) = unsafe { object.as_mut() } {
        object.static_object = static_object;
    }
}

#[no_mangle]
pub extern "C" fn vips_object_print_all() {
    let live: Vec<usize> = object_registry()
        .lock()
        .expect("object registry")
        .iter()
        .copied()
        .collect();
    for object in live {
        vips_object_print_summary(object as *mut VipsObject);
    }
}

#[no_mangle]
pub extern "C" fn vips_object_sanity_all() {
    let live: Vec<usize> = object_registry()
        .lock()
        .expect("object registry")
        .iter()
        .copied()
        .collect();
    for object in live {
        let _ = vips_object_sanity(object as *mut VipsObject);
    }
}

#[no_mangle]
pub extern "C" fn vips_object_rewind(object: *mut VipsObject) {
    if object.is_null() {
        return;
    }
    let class = unsafe { object_class(object) };
    if class.is_null() {
        return;
    }
    if let Some(rewind) = unsafe { (*class).rewind } {
        unsafe {
            rewind(object);
        }
    }
}

#[no_mangle]
pub extern "C" fn vips_object_unref_outputs(object: *mut VipsObject) {
    if object.is_null() {
        return;
    }
    let class = unsafe { object_class(object) };
    if class.is_null() {
        return;
    }
    let mut node = unsafe { (*class).argument_table_traverse };
    while !node.is_null() {
        let argument = unsafe { (*node).data.cast::<VipsArgumentClass>() };
        if !argument.is_null() && unsafe { (*argument).flags & VIPS_ARGUMENT_OUTPUT != 0 } {
            let name = unsafe {
                CStr::from_ptr(gobject_sys::g_param_spec_get_name((*argument).parent.pspec))
                    .to_string_lossy()
                    .into_owned()
            };
            if let Some(DynamicValue::Object(value)) = unsafe { dynamic_value(object, &name) } {
                if !value.is_null() {
                    unsafe {
                        gobject_sys::g_object_unref((*value).cast());
                    }
                }
                unsafe {
                    remove_dynamic_value(object, &name);
                }
            }
        }
        node = unsafe { (*node).next };
    }
}

#[no_mangle]
pub extern "C" fn vips_object_get_description(object: *mut VipsObject) -> *const c_char {
    if object.is_null() {
        return ptr::null();
    }
    unsafe {
        if !(*object).description.is_null() {
            (*object).description
        } else {
            let class = object_class(object);
            if class.is_null() {
                ptr::null()
            } else {
                (*class).description
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn vips_argument_map(
    object: *mut VipsObject,
    fn_: VipsArgumentMapFn,
    a: *mut c_void,
    b: *mut c_void,
) -> *mut c_void {
    let Some(fn_) = fn_ else {
        return ptr::null_mut();
    };
    let class = unsafe { object_class(object) };
    if class.is_null() {
        return ptr::null_mut();
    }
    let mut node = unsafe { (*class).argument_table_traverse };
    while !node.is_null() {
        let argument_class = unsafe { (*node).data.cast::<VipsArgumentClass>() };
        if !argument_class.is_null() {
            let argument_instance = unsafe { argument_instance(object, argument_class) };
            let result = unsafe {
                fn_(
                    object,
                    (*argument_class).parent.pspec,
                    argument_class,
                    argument_instance,
                    a,
                    b,
                )
            };
            if !result.is_null() {
                return result;
            }
        }
        node = unsafe { (*node).next };
    }
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn vips_argument_class_map(
    class: *mut VipsObjectClass,
    fn_: VipsArgumentClassMapFn,
    a: *mut c_void,
    b: *mut c_void,
) -> *mut c_void {
    let Some(fn_) = fn_ else {
        return ptr::null_mut();
    };
    if class.is_null() {
        return ptr::null_mut();
    }
    let mut node = unsafe { (*class).argument_table_traverse };
    while !node.is_null() {
        let argument_class = unsafe { (*node).data.cast::<VipsArgumentClass>() };
        if !argument_class.is_null() {
            let result =
                unsafe { fn_(class, (*argument_class).parent.pspec, argument_class, a, b) };
            if !result.is_null() {
                return result;
            }
        }
        node = unsafe { (*node).next };
    }
    ptr::null_mut()
}
