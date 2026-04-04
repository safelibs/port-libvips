use std::ffi::{CStr, CString};
use std::mem::size_of;
use std::os::raw::c_char;
use std::ptr;
use std::sync::{Mutex, OnceLock};

use libc::{c_int, c_void};

use crate::abi::basic::VipsBuf;
use crate::abi::connection::{
    VipsConnection, VipsConnectionClass, VipsSource, VipsSourceClass, VipsTarget, VipsTargetClass,
};
use crate::abi::image::VipsImage;
use crate::abi::object::{
    VipsArgumentClass, VipsArgumentMapFn, VipsObject, VipsObjectClass, VIPS_ARGUMENT_CONSTRUCT,
    VIPS_ARGUMENT_DEPRECATED, VIPS_ARGUMENT_INPUT, VIPS_ARGUMENT_OUTPUT, VIPS_ARGUMENT_REQUIRED,
};
use crate::abi::operation::{
    VipsForeign, VipsForeignClass, VipsForeignLoad, VipsForeignLoadClass, VipsForeignSave,
    VipsForeignSaveClass, VipsInterpolate, VipsInterpolateClass, VipsOperation, VipsOperationClass,
    VipsOperationFlags, VipsThreadState, VIPS_OPERATION_BLOCKED,
};
use crate::abi::region::VipsRegion;
use crate::runtime::error::append_message_str;
use crate::runtime::object;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeneratedArgumentKind {
    Bool,
    Int,
    UInt64,
    Double,
    String,
    Pointer,
    Object,
    Boxed,
    Enum,
    Flags,
}

#[derive(Debug)]
pub struct GeneratedArgumentMetadata {
    pub name: &'static str,
    pub long_name: &'static str,
    pub description: &'static str,
    pub priority: i32,
    pub flags: crate::abi::object::VipsArgumentFlags,
    pub required: bool,
    pub construct: bool,
    pub direction: &'static str,
    pub kind: GeneratedArgumentKind,
    pub value_type_name: Option<&'static str>,
    pub default_value: Option<&'static str>,
    pub min_value: Option<&'static str>,
    pub max_value: Option<&'static str>,
}

#[derive(Debug)]
pub struct GeneratedOperationMetadata {
    pub flags: VipsOperationFlags,
    pub supported: bool,
    pub arguments: &'static [&'static GeneratedArgumentMetadata],
    pub wrapper_function: Option<&'static str>,
}

#[derive(Debug)]
pub struct GeneratedTypeMetadata {
    pub type_name: &'static str,
    pub parent_type_name: Option<&'static str>,
    pub nickname: &'static str,
    pub description: &'static str,
    pub depth: i32,
    pub abstract_: bool,
    pub source_file: Option<&'static str>,
    pub operation: Option<&'static GeneratedOperationMetadata>,
}

#[derive(Debug)]
pub struct GeneratedWrapperParameter {
    pub text: &'static str,
    pub name: Option<&'static str>,
    pub type_text: Option<&'static str>,
    pub variadic: bool,
}

#[derive(Debug)]
pub struct GeneratedWrapperMetadata {
    pub function: &'static str,
    pub nickname: &'static str,
    pub header: &'static str,
    pub signature: &'static str,
    pub last_fixed_name: Option<&'static str>,
    pub variadic: bool,
    pub parameters: &'static [GeneratedWrapperParameter],
}

mod generated_registry {
    include!("../generated/operations_registry.rs");
}

mod generated_wrappers {
    include!("../generated/operation_wrappers.rs");
}

fn configured_types() -> &'static Mutex<Vec<glib_sys::GType>> {
    static TYPES: OnceLock<Mutex<Vec<glib_sys::GType>>> = OnceLock::new();
    TYPES.get_or_init(|| Mutex::new(Vec::new()))
}

fn blocked_names() -> &'static Mutex<Vec<String>> {
    static BLOCKED: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
    BLOCKED.get_or_init(|| Mutex::new(Vec::new()))
}

fn with_cstring<T>(text: &str, f: impl FnOnce(&CStr) -> T) -> T {
    let text = CString::new(text).expect("cstring");
    f(&text)
}

fn type_name(type_: glib_sys::GType) -> Option<&'static str> {
    let name = unsafe { gobject_sys::g_type_name(type_) };
    if name.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(name) }.to_str().ok()
    }
}

fn append_text(buf: *mut VipsBuf, text: String) {
    if let Ok(text) = CString::new(text) {
        crate::runtime::buf::vips_buf_appends(buf, text.as_ptr());
    }
}

pub fn generated_types() -> &'static [GeneratedTypeMetadata] {
    generated_registry::GENERATED_TYPES
}

pub fn generated_wrappers() -> &'static [GeneratedWrapperMetadata] {
    generated_wrappers::GENERATED_WRAPPERS
}

pub fn generated_type_by_name(name: &str) -> Option<&'static GeneratedTypeMetadata> {
    generated_registry::GENERATED_TYPES
        .iter()
        .find(|meta| meta.type_name == name)
}

pub fn generated_operation_by_name(name: &str) -> Option<&'static GeneratedOperationMetadata> {
    generated_type_by_name(name).and_then(|meta| meta.operation)
}

fn normalize_vips_type_name(name: &str) -> Option<String> {
    if !name.starts_with("Vips") {
        return None;
    }
    let suffix = &name[4..];
    if suffix.is_empty() || suffix.chars().any(|ch| ch.is_ascii_lowercase()) {
        return None;
    }
    let mut out = String::from("Vips");
    for part in suffix.split('_') {
        if part.is_empty() {
            continue;
        }
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_ascii_uppercase());
            for ch in chars {
                out.push(ch.to_ascii_lowercase());
            }
        }
    }
    Some(out)
}

fn resolve_named_type(name: &str) -> glib_sys::GType {
    let direct = with_cstring(name, |name| unsafe {
        gobject_sys::g_type_from_name(name.as_ptr())
    });
    if direct != 0 {
        return direct;
    }
    if let Some(normalized) = normalize_vips_type_name(name) {
        let normalized_type = with_cstring(&normalized, |name| unsafe {
            gobject_sys::g_type_from_name(name.as_ptr())
        });
        if normalized_type != 0 {
            return normalized_type;
        }
    }
    0
}

fn is_type_configured(type_: glib_sys::GType) -> bool {
    configured_types()
        .lock()
        .expect("configured types")
        .contains(&type_)
}

fn mark_type_configured(type_: glib_sys::GType) {
    let mut configured = configured_types().lock().expect("configured types");
    if !configured.contains(&type_) {
        configured.push(type_);
    }
}

fn is_blocked(meta: &GeneratedTypeMetadata) -> bool {
    let blocked = blocked_names().lock().expect("blocked names");
    blocked.iter().any(|name| {
        name.eq_ignore_ascii_case(meta.nickname) || name.eq_ignore_ascii_case(meta.type_name)
    })
}

fn instance_size_for(parent: glib_sys::GType, type_name: &str) -> usize {
    if type_name == "VipsImage"
        || unsafe { gobject_sys::g_type_is_a(parent, object::vips_image_get_type()) }
            != glib_sys::GFALSE
    {
        size_of::<VipsImage>()
    } else if type_name == "VipsRegion"
        || unsafe { gobject_sys::g_type_is_a(parent, object::vips_region_get_type()) }
            != glib_sys::GFALSE
    {
        size_of::<VipsRegion>()
    } else if type_name == "VipsForeignLoad"
        || unsafe { gobject_sys::g_type_is_a(parent, object::vips_foreign_load_get_type()) }
            != glib_sys::GFALSE
    {
        size_of::<VipsForeignLoad>()
    } else if type_name == "VipsForeignSave"
        || unsafe { gobject_sys::g_type_is_a(parent, object::vips_foreign_save_get_type()) }
            != glib_sys::GFALSE
    {
        size_of::<VipsForeignSave>()
    } else if type_name == "VipsForeign"
        || unsafe { gobject_sys::g_type_is_a(parent, object::vips_foreign_get_type()) }
            != glib_sys::GFALSE
    {
        size_of::<VipsForeign>()
    } else if type_name == "VipsInterpolate"
        || unsafe { gobject_sys::g_type_is_a(parent, object::vips_interpolate_get_type()) }
            != glib_sys::GFALSE
    {
        size_of::<VipsInterpolate>()
    } else if type_name == "VipsSource"
        || unsafe { gobject_sys::g_type_is_a(parent, object::vips_source_get_type()) }
            != glib_sys::GFALSE
    {
        size_of::<VipsSource>()
    } else if type_name == "VipsTarget"
        || unsafe { gobject_sys::g_type_is_a(parent, object::vips_target_get_type()) }
            != glib_sys::GFALSE
    {
        size_of::<VipsTarget>()
    } else if type_name == "VipsConnection"
        || unsafe { gobject_sys::g_type_is_a(parent, object::vips_connection_get_type()) }
            != glib_sys::GFALSE
    {
        size_of::<VipsConnection>()
    } else if type_name == "VipsThreadState"
        || unsafe { gobject_sys::g_type_is_a(parent, object::vips_thread_state_get_type()) }
            != glib_sys::GFALSE
    {
        size_of::<VipsThreadState>()
    } else if type_name == "VipsOperation"
        || unsafe { gobject_sys::g_type_is_a(parent, object::vips_operation_get_type()) }
            != glib_sys::GFALSE
    {
        size_of::<VipsOperation>()
    } else {
        size_of::<VipsObject>()
    }
}

fn class_size_for(parent: glib_sys::GType, type_name: &str, is_operation: bool) -> usize {
    if is_operation {
        if type_name == "VipsForeignLoad"
            || unsafe { gobject_sys::g_type_is_a(parent, object::vips_foreign_load_get_type()) }
                != glib_sys::GFALSE
        {
            size_of::<VipsForeignLoadClass>()
        } else if type_name == "VipsForeignSave"
            || unsafe { gobject_sys::g_type_is_a(parent, object::vips_foreign_save_get_type()) }
                != glib_sys::GFALSE
        {
            size_of::<VipsForeignSaveClass>()
        } else if type_name == "VipsForeign"
            || unsafe { gobject_sys::g_type_is_a(parent, object::vips_foreign_get_type()) }
                != glib_sys::GFALSE
        {
            size_of::<VipsForeignClass>()
        } else {
            size_of::<VipsOperationClass>()
        }
    } else if type_name == "VipsInterpolate"
        || unsafe { gobject_sys::g_type_is_a(parent, object::vips_interpolate_get_type()) }
            != glib_sys::GFALSE
    {
        size_of::<VipsInterpolateClass>()
    } else if type_name == "VipsSource"
        || unsafe { gobject_sys::g_type_is_a(parent, object::vips_source_get_type()) }
            != glib_sys::GFALSE
    {
        size_of::<VipsSourceClass>()
    } else if type_name == "VipsTarget"
        || unsafe { gobject_sys::g_type_is_a(parent, object::vips_target_get_type()) }
            != glib_sys::GFALSE
    {
        size_of::<VipsTargetClass>()
    } else if type_name == "VipsConnection"
        || unsafe { gobject_sys::g_type_is_a(parent, object::vips_connection_get_type()) }
            != glib_sys::GFALSE
    {
        size_of::<VipsConnectionClass>()
    } else {
        size_of::<VipsObjectClass>()
    }
}

fn property_flags(argument: &GeneratedArgumentMetadata) -> gobject_sys::GParamFlags {
    let mut flags = gobject_sys::G_PARAM_READWRITE;
    if argument.construct {
        flags |= gobject_sys::G_PARAM_CONSTRUCT;
    }
    flags
}

fn parse_bool(text: Option<&str>) -> bool {
    matches!(
        text.unwrap_or("").to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn parse_i64_symbolic(text: &str) -> Option<i64> {
    let normalized = text
        .trim()
        .trim_matches(|ch| ch == '(' || ch == ')')
        .replace(' ', "");
    match normalized.as_str() {
        "VIPS_MAX_COORD" => Some(10_000_000),
        "-VIPS_MAX_COORD" => Some(-10_000_000),
        "INT_MAX" => Some(c_int::MAX as i64),
        "INT_MAX-1" => Some(c_int::MAX as i64 - 1),
        "INT_MIN" => Some(c_int::MIN as i64),
        _ => normalized.parse::<i64>().ok(),
    }
}

fn parse_i32(text: Option<&str>) -> c_int {
    let value = text.and_then(parse_i64_symbolic).unwrap_or_default();
    value.clamp(c_int::MIN as i64, c_int::MAX as i64) as c_int
}

fn parse_u64(text: Option<&str>) -> u64 {
    text.and_then(|value| value.parse::<u64>().ok())
        .unwrap_or_default()
}

fn parse_f64(text: Option<&str>) -> f64 {
    text.and_then(|value| {
        let trimmed = value.trim();
        match trimmed.to_ascii_uppercase().as_str() {
            "INFINITY" => Some(f64::INFINITY),
            "-INFINITY" => Some(f64::NEG_INFINITY),
            _ => trimmed.parse::<f64>().ok(),
        }
    })
    .unwrap_or_default()
}

unsafe fn parse_enum_default(
    type_: glib_sys::GType,
    default_value: Option<&str>,
    flags: bool,
) -> c_int {
    let Some(default_value) = default_value else {
        return 0;
    };
    let value = CString::new(default_value).expect("enum default");
    unsafe { object::parse_enum_like(type_, &value, flags) }.unwrap_or_default()
}

unsafe fn create_param_spec(argument: &GeneratedArgumentMetadata) -> *mut gobject_sys::GParamSpec {
    let name = CString::new(argument.name).expect("param name");
    let nick = CString::new(argument.long_name).expect("param nick");
    let blurb = CString::new(argument.description).expect("param blurb");
    let flags = property_flags(argument);

    match argument.kind {
        GeneratedArgumentKind::Bool => unsafe {
            gobject_sys::g_param_spec_boolean(
                name.as_ptr(),
                nick.as_ptr(),
                blurb.as_ptr(),
                object::bool_to_gboolean(parse_bool(argument.default_value)),
                flags,
            )
        },
        GeneratedArgumentKind::Int => unsafe {
            let min = parse_i32(argument.min_value);
            let max = parse_i32(argument.max_value).max(min);
            let default = parse_i32(argument.default_value).clamp(min, max);
            gobject_sys::g_param_spec_int(
                name.as_ptr(),
                nick.as_ptr(),
                blurb.as_ptr(),
                min,
                max,
                default,
                flags,
            )
        },
        GeneratedArgumentKind::UInt64 => unsafe {
            gobject_sys::g_param_spec_uint64(
                name.as_ptr(),
                nick.as_ptr(),
                blurb.as_ptr(),
                parse_u64(argument.min_value),
                parse_u64(argument.max_value),
                parse_u64(argument.default_value),
                flags,
            )
        },
        GeneratedArgumentKind::Double => unsafe {
            gobject_sys::g_param_spec_double(
                name.as_ptr(),
                nick.as_ptr(),
                blurb.as_ptr(),
                parse_f64(argument.min_value),
                parse_f64(argument.max_value),
                parse_f64(argument.default_value),
                flags,
            )
        },
        GeneratedArgumentKind::String => unsafe {
            let default = argument
                .default_value
                .map(|value| CString::new(value).expect("default string"));
            gobject_sys::g_param_spec_string(
                name.as_ptr(),
                nick.as_ptr(),
                blurb.as_ptr(),
                default.as_ref().map_or(ptr::null(), |value| value.as_ptr()),
                flags,
            )
        },
        GeneratedArgumentKind::Pointer => unsafe {
            gobject_sys::g_param_spec_pointer(name.as_ptr(), nick.as_ptr(), blurb.as_ptr(), flags)
        },
        GeneratedArgumentKind::Object => {
            let value_type = argument
                .value_type_name
                .map(resolve_named_type)
                .filter(|type_| *type_ != 0)
                .unwrap_or(gobject_sys::G_TYPE_OBJECT);
            unsafe {
                gobject_sys::g_param_spec_object(
                    name.as_ptr(),
                    nick.as_ptr(),
                    blurb.as_ptr(),
                    value_type,
                    flags,
                )
            }
        }
        GeneratedArgumentKind::Boxed => {
            let value_type = argument
                .value_type_name
                .map(resolve_named_type)
                .filter(|type_| *type_ != 0)
                .unwrap_or(gobject_sys::G_TYPE_POINTER);
            unsafe {
                gobject_sys::g_param_spec_boxed(
                    name.as_ptr(),
                    nick.as_ptr(),
                    blurb.as_ptr(),
                    value_type,
                    flags,
                )
            }
        }
        GeneratedArgumentKind::Enum => {
            let value_type = argument
                .value_type_name
                .map(resolve_named_type)
                .filter(|type_| *type_ != 0)
                .unwrap_or(gobject_sys::G_TYPE_INT);
            unsafe {
                gobject_sys::g_param_spec_enum(
                    name.as_ptr(),
                    nick.as_ptr(),
                    blurb.as_ptr(),
                    value_type,
                    parse_enum_default(value_type, argument.default_value, false),
                    flags,
                )
            }
        }
        GeneratedArgumentKind::Flags => {
            let value_type = argument
                .value_type_name
                .map(resolve_named_type)
                .filter(|type_| *type_ != 0)
                .unwrap_or(gobject_sys::G_TYPE_FLAGS);
            unsafe {
                gobject_sys::g_param_spec_flags(
                    name.as_ptr(),
                    nick.as_ptr(),
                    blurb.as_ptr(),
                    value_type,
                    parse_enum_default(value_type, argument.default_value, true) as u32,
                    flags,
                )
            }
        }
    }
}

unsafe fn install_generated_arguments(
    class: *mut VipsObjectClass,
    operation: &'static GeneratedOperationMetadata,
) {
    let gobject_class = class.cast::<gobject_sys::GObjectClass>();
    for argument in operation.arguments {
        let pspec = unsafe { create_param_spec(argument) };
        unsafe {
            gobject_sys::g_object_class_install_property(
                gobject_class,
                object::vips_argument_get_id() as u32,
                pspec,
            );
        }
        object::vips_object_class_install_argument(
            class,
            pspec,
            argument.flags,
            argument.priority,
            object::DYNAMIC_ARGUMENT_OFFSET,
        );
    }
}

unsafe fn configure_registered_type(type_: glib_sys::GType, meta: &'static GeneratedTypeMetadata) {
    if is_type_configured(type_) {
        return;
    }
    let class = unsafe { object::object_class_for_type(type_) };
    if class.is_null() {
        return;
    }
    unsafe {
        object::prepare_existing_class(class);
        (*class).nickname = object::leak_cstring(meta.nickname);
        (*class).description = object::leak_cstring(meta.description);
        if let Some(operation) = meta.operation {
            let operation_class = class.cast::<VipsOperationClass>();
            (*operation_class).usage = Some(vips_operation_usage);
            (*operation_class).get_flags = Some(vips_operation_class_flags);
            (*class).build = Some(crate::ops::generated_operation_build);
            let parent_type =
                gobject_sys::g_type_parent((*class.cast::<gobject_sys::GTypeClass>()).g_type);
            let inherited_flags = if parent_type != 0
                && gobject_sys::g_type_is_a(parent_type, object::vips_operation_get_type())
                    != glib_sys::GFALSE
            {
                let parent_class =
                    object::object_class_for_type(parent_type).cast::<VipsOperationClass>();
                if parent_class.is_null() {
                    0
                } else {
                    (*parent_class).flags
                }
            } else {
                0
            };
            (*operation_class).flags |= inherited_flags | operation.flags;
            if is_blocked(meta) {
                (*operation_class).flags |= VIPS_OPERATION_BLOCKED;
            }
            (*operation_class).invalidate = Some(vips_operation_invalidate);
            install_generated_arguments(class, operation);
        }
    }
    mark_type_configured(type_);
}

unsafe fn register_generated_type(meta: &'static GeneratedTypeMetadata) -> Option<glib_sys::GType> {
    let existing = resolve_named_type(meta.type_name);
    if existing != 0 {
        return Some(existing);
    }
    let parent_name = meta.parent_type_name?;
    let parent = resolve_named_type(parent_name);
    if parent == 0 {
        return None;
    }
    unsafe {
        gobject_sys::g_type_class_ref(parent);
    }
    let is_operation =
        unsafe { gobject_sys::g_type_is_a(parent, object::vips_operation_get_type()) }
            != glib_sys::GFALSE;
    let type_name = object::leak_cstring(meta.type_name);
    let type_ = object::register_type(
        parent,
        type_name,
        class_size_for(parent, meta.type_name, is_operation),
        if is_operation {
            Some(generated_operation_class_init)
        } else {
            Some(generated_object_class_init)
        },
        instance_size_for(parent, meta.type_name),
        None,
        if meta.abstract_ {
            gobject_sys::G_TYPE_FLAG_ABSTRACT
        } else {
            0
        },
    );
    if type_ == 0 {
        None
    } else {
        Some(type_)
    }
}

pub(crate) fn ensure_generated_types() -> bool {
    static ONCE: OnceLock<Result<(), String>> = OnceLock::new();
    match ONCE.get_or_init(|| {
        let mut pending: Vec<&'static GeneratedTypeMetadata> =
            generated_registry::GENERATED_TYPES.iter().collect();
        pending.sort_by_key(|meta| (meta.depth, meta.type_name));

        while !pending.is_empty() {
            let before = pending.len();
            pending.retain(|meta| {
                let type_ = unsafe { register_generated_type(meta) };
                let Some(type_) = type_ else {
                    return true;
                };
                unsafe {
                    configure_registered_type(type_, meta);
                }
                false
            });
            if pending.len() == before {
                let unresolved = pending
                    .iter()
                    .map(|meta| meta.type_name)
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(format!("unable to register generated types: {unresolved}"));
            }
        }
        Ok(())
    }) {
        Ok(()) => true,
        Err(message) => {
            append_message_str("ensure_generated_types", message);
            false
        }
    }
}

unsafe extern "C" fn vips_operation_usage(class: *mut VipsOperationClass, buf: *mut VipsBuf) {
    if class.is_null() || buf.is_null() {
        return;
    }
    let object_class = class.cast::<VipsObjectClass>();
    let nickname = unsafe {
        if (*object_class).nickname.is_null() {
            c"operation".as_ptr()
        } else {
            (*object_class).nickname
        }
    };
    let nickname = unsafe { CStr::from_ptr(nickname) }.to_string_lossy();
    append_text(buf, format!("usage: {nickname}"));

    let type_name = unsafe { type_name((*class.cast::<gobject_sys::GTypeClass>()).g_type) };
    if let Some(type_name) = type_name {
        if let Some(meta) = generated_operation_by_name(type_name) {
            for argument in meta.arguments {
                if argument.required
                    && argument.construct
                    && argument.flags & VIPS_ARGUMENT_DEPRECATED == 0
                {
                    append_text(buf, format!(" {}", argument.name));
                }
            }
        }
    }
}

unsafe extern "C" fn vips_operation_class_flags(
    operation: *mut VipsOperation,
) -> VipsOperationFlags {
    if operation.is_null() {
        return 0;
    }
    let class = unsafe { object::object_class(operation.cast()).cast::<VipsOperationClass>() };
    if class.is_null() {
        0
    } else {
        unsafe { (*class).flags }
    }
}

#[no_mangle]
pub extern "C" fn vips_operation_get_flags(operation: *mut VipsOperation) -> VipsOperationFlags {
    if operation.is_null() {
        return 0;
    }
    let class = unsafe { object::object_class(operation.cast()).cast::<VipsOperationClass>() };
    if class.is_null() {
        return 0;
    }
    unsafe {
        (*class)
            .get_flags
            .map(|get_flags| get_flags(operation))
            .unwrap_or((*class).flags)
    }
}

#[no_mangle]
pub extern "C" fn vips_operation_class_print_usage(class: *mut VipsOperationClass) {
    if class.is_null() {
        return;
    }
    let mut buf = VipsBuf {
        base: ptr::null_mut(),
        mx: 0,
        i: 0,
        full: glib_sys::GFALSE,
        lasti: 0,
        dynamic: glib_sys::GFALSE,
    };
    crate::runtime::buf::vips_buf_init_dynamic(&mut buf, 1024);
    if let Some(usage) = unsafe { (*class).usage } {
        unsafe {
            usage(class, &mut buf);
        }
    }
    let text = crate::runtime::buf::vips_buf_all(&mut buf);
    if !text.is_null() {
        unsafe {
            libc::printf(c"%s\n".as_ptr(), text);
        }
    }
    crate::runtime::buf::vips_buf_destroy(&mut buf);
}

#[no_mangle]
pub extern "C" fn vips_operation_invalidate(operation: *mut VipsOperation) {
    if operation.is_null() {
        return;
    }
    let class = unsafe { object::object_class(operation.cast()).cast::<VipsOperationClass>() };
    if class.is_null() {
        return;
    }
    if let Some(invalidate) = unsafe { (*class).invalidate } {
        unsafe {
            invalidate(operation);
        }
    }
}

pub(crate) unsafe extern "C" fn vips_operation_class_init(
    klass: glib_sys::gpointer,
    _class_data: glib_sys::gpointer,
) {
    let class = klass.cast::<VipsOperationClass>();
    unsafe {
        object::init_subclass_class(class.cast());
        (*class).usage = Some(vips_operation_usage);
        (*class).get_flags = Some(vips_operation_class_flags);
        (*class).flags = 0;
        (*class).invalidate = Some(vips_operation_invalidate);
    }
}

unsafe extern "C" fn generated_object_class_init(
    klass: glib_sys::gpointer,
    _class_data: glib_sys::gpointer,
) {
    unsafe {
        let class = klass.cast::<VipsObjectClass>();
        object::init_subclass_class(class);
        let gobject_class = klass.cast::<gobject_sys::GObjectClass>();
        let parent_class =
            gobject_sys::g_type_class_peek_parent(klass).cast::<gobject_sys::GObjectClass>();
        if !parent_class.is_null() {
            if (*gobject_class).dispose.is_none() {
                (*gobject_class).dispose = (*parent_class).dispose;
            }
            if (*gobject_class).finalize.is_none() {
                (*gobject_class).finalize = (*parent_class).finalize;
            }
        }
        (*gobject_class).set_property = Some(object::vips_object_set_property);
        (*gobject_class).get_property = Some(object::vips_object_get_property);
    }
}

unsafe extern "C" fn generated_operation_class_init(
    klass: glib_sys::gpointer,
    _class_data: glib_sys::gpointer,
) {
    unsafe {
        vips_operation_class_init(klass, ptr::null_mut());
        let gobject_class = klass.cast::<gobject_sys::GObjectClass>();
        (*gobject_class).set_property = Some(object::vips_object_set_property);
        (*gobject_class).get_property = Some(object::vips_object_get_property);
    }
}

#[no_mangle]
pub extern "C" fn vips_operation_call_valist(
    _operation: *mut VipsOperation,
    _ap: *mut c_void,
) -> c_int {
    append_message_str("vips_operation_call_valist", "not implemented");
    -1
}

#[no_mangle]
pub extern "C" fn vips_operation_new(name: *const c_char) -> *mut VipsOperation {
    if name.is_null() {
        append_message_str("vips_operation_new", "operation name is NULL");
        return ptr::null_mut();
    }
    if !ensure_generated_types() {
        return ptr::null_mut();
    }
    let requested = unsafe { CStr::from_ptr(name) }.to_string_lossy();
    let lookup_name = if requested.eq_ignore_ascii_case("crop") {
        c"extract_area".as_ptr()
    } else {
        name
    };
    let mut type_ = object::vips_type_find_unfiltered(c"VipsOperation".as_ptr(), lookup_name);
    if type_ == 0 {
        crate::foreign::modules::try_load_for_operation(requested.as_ref());
        type_ = object::vips_type_find_unfiltered(c"VipsOperation".as_ptr(), lookup_name);
    }
    if type_ == 0 {
        append_message_str(
            "vips_operation_new",
            &format!(
                "unknown operation {}",
                unsafe { CStr::from_ptr(name) }.to_string_lossy()
            ),
        );
        return ptr::null_mut();
    }
    unsafe { object::object_new::<VipsOperation>(type_) }
}

struct CallState {
    operation: *mut VipsOperation,
    argc: c_int,
    argv: *mut *mut c_char,
    index: c_int,
}

unsafe fn call_get_arg(call: &CallState, index: c_int) -> Option<&'static CStr> {
    if index < 0 || index >= call.argc || call.argv.is_null() {
        return None;
    }
    let arg = unsafe { *call.argv.add(index as usize) };
    if arg.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(arg) })
    }
}

unsafe extern "C" fn call_argv_input(
    object: *mut VipsObject,
    pspec: *mut gobject_sys::GParamSpec,
    argument_class: *mut VipsArgumentClass,
    _argument_instance: *mut crate::abi::object::VipsArgumentInstance,
    a: *mut c_void,
    _b: *mut c_void,
) -> *mut c_void {
    if argument_class.is_null() {
        return ptr::null_mut();
    }
    let call = unsafe { &mut *a.cast::<CallState>() };
    let flags = unsafe { (*argument_class).flags };
    if flags & VIPS_ARGUMENT_REQUIRED != 0
        && flags & VIPS_ARGUMENT_CONSTRUCT != 0
        && flags & VIPS_ARGUMENT_DEPRECATED == 0
    {
        if flags & VIPS_ARGUMENT_INPUT != 0 {
            let Some(arg) = (unsafe { call_get_arg(call, call.index) }) else {
                append_message_str(
                    unsafe { CStr::from_ptr((*object::object_class(object)).nickname) }
                        .to_str()
                        .unwrap_or("VipsOperation"),
                    "too few arguments",
                );
                return pspec.cast();
            };
            if object::vips_object_set_argument_from_string(
                object,
                unsafe { gobject_sys::g_param_spec_get_name(pspec) },
                arg.as_ptr(),
            ) != 0
            {
                return pspec.cast();
            }
            call.index += 1;
        } else if flags & VIPS_ARGUMENT_OUTPUT != 0
            && object::vips_object_argument_needsstring(object, unsafe {
                gobject_sys::g_param_spec_get_name(pspec)
            }) != glib_sys::GFALSE
        {
            call.index += 1;
        }
    }
    ptr::null_mut()
}

unsafe extern "C" fn call_argv_output(
    object: *mut VipsObject,
    pspec: *mut gobject_sys::GParamSpec,
    argument_class: *mut VipsArgumentClass,
    _argument_instance: *mut crate::abi::object::VipsArgumentInstance,
    a: *mut c_void,
    _b: *mut c_void,
) -> *mut c_void {
    if argument_class.is_null() {
        return ptr::null_mut();
    }
    let call = unsafe { &mut *a.cast::<CallState>() };
    let flags = unsafe { (*argument_class).flags };
    if flags & VIPS_ARGUMENT_REQUIRED != 0
        && flags & VIPS_ARGUMENT_CONSTRUCT != 0
        && flags & VIPS_ARGUMENT_DEPRECATED == 0
    {
        let output_name = unsafe { gobject_sys::g_param_spec_get_name(pspec) };
        if flags & VIPS_ARGUMENT_INPUT != 0 {
            if object::vips_object_argument_needsstring(object, output_name) != glib_sys::GFALSE {
                let Some(_) = (unsafe { call_get_arg(call, call.index) }) else {
                    append_message_str(
                        unsafe { CStr::from_ptr((*object::object_class(object)).nickname) }
                            .to_str()
                            .unwrap_or("VipsOperation"),
                        "too few arguments",
                    );
                    return pspec.cast();
                };
                call.index += 1;
            }
        } else if flags & VIPS_ARGUMENT_OUTPUT != 0 {
            let arg = if object::vips_object_argument_needsstring(object, output_name)
                != glib_sys::GFALSE
            {
                let Some(arg) = (unsafe { call_get_arg(call, call.index) }) else {
                    append_message_str(
                        unsafe { CStr::from_ptr((*object::object_class(object)).nickname) }
                            .to_str()
                            .unwrap_or("VipsOperation"),
                        "too few arguments",
                    );
                    return pspec.cast();
                };
                call.index += 1;
                arg.as_ptr()
            } else {
                ptr::null()
            };
            if object::vips_object_get_argument_to_string(object, output_name, arg) != 0 {
                return pspec.cast();
            }
        }
    }
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn vips_call_argv(
    operation: *mut VipsOperation,
    argc: c_int,
    argv: *mut *mut c_char,
) -> c_int {
    if operation.is_null() {
        append_message_str("vips_call_argv", "operation is NULL");
        return -1;
    }
    let mut call = CallState {
        operation,
        argc,
        argv,
        index: 0,
    };
    let input_result = object::vips_argument_map(
        operation.cast(),
        Some(call_argv_input),
        (&mut call as *mut CallState).cast(),
        ptr::null_mut(),
    );
    if !input_result.is_null() {
        return -1;
    }
    if call.index < argc {
        append_message_str(
            unsafe { CStr::from_ptr((*object::object_class(operation.cast())).nickname) }
                .to_str()
                .unwrap_or("VipsOperation"),
            "too many arguments",
        );
        return -1;
    }

    if object::vips_object_build(operation.cast()) != 0 {
        return -1;
    }

    call.index = 0;
    if !object::vips_argument_map(
        operation.cast(),
        Some(call_argv_output),
        (&mut call as *mut CallState).cast(),
        ptr::null_mut(),
    )
    .is_null()
    {
        return -1;
    }

    0
}

unsafe extern "C" fn block_operation_class(
    class: *mut VipsObjectClass,
    state: *mut c_void,
) -> *mut c_void {
    if class.is_null() || state.is_null() {
        return ptr::null_mut();
    }
    let blocked = unsafe { *(state.cast::<glib_sys::gboolean>()) } != glib_sys::GFALSE;
    let operation_class = class.cast::<VipsOperationClass>();
    unsafe {
        if blocked {
            (*operation_class).flags |= VIPS_OPERATION_BLOCKED;
        } else {
            (*operation_class).flags &= !VIPS_OPERATION_BLOCKED;
        }
    }
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn vips_operation_block_set(name: *const c_char, state: glib_sys::gboolean) {
    if name.is_null() {
        return;
    }
    if !ensure_generated_types() {
        return;
    }
    let name = unsafe { CStr::from_ptr(name) }
        .to_string_lossy()
        .into_owned();
    {
        let mut blocked = blocked_names().lock().expect("blocked names");
        if state == glib_sys::GFALSE {
            blocked.retain(|item| !item.eq_ignore_ascii_case(&name));
        } else if !blocked.iter().any(|item| item.eq_ignore_ascii_case(&name)) {
            blocked.push(name.clone());
        }
    }

    let type_ =
        object::vips_type_find_unfiltered(c"VipsOperation".as_ptr(), object::leak_cstring(&name));
    if type_ != 0 {
        object::vips_class_map_all(
            type_,
            Some(block_operation_class),
            &state as *const _ as *mut c_void,
        );
    }
}
