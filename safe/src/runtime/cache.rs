use std::ffi::CStr;
use std::mem;
use std::ptr;
use std::sync::{Mutex, OnceLock};

use crate::abi::object::{
    VipsArgumentClass, VipsArgumentInstance, VipsObject, VipsObjectClass, VIPS_ARGUMENT_CONSTRUCT,
    VIPS_ARGUMENT_INPUT, VIPS_ARGUMENT_NON_HASHABLE,
};
use crate::abi::operation::{
    VipsOperation, VipsOperationClass, VipsOperationFlags, VIPS_OPERATION_BLOCKED,
    VIPS_OPERATION_NOCACHE, VIPS_OPERATION_REVALIDATE,
};
use crate::runtime::error::append_message_str;
use crate::runtime::memory::{vips_tracked_get_files, vips_tracked_get_mem};
use crate::runtime::object::{object_ref, object_unref, vips_argument_map};

#[derive(Clone, Copy)]
struct CacheConfig {
    max: i32,
    max_files: i32,
    max_mem: usize,
    dump: bool,
    trace: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max: 100,
            max_files: 100,
            max_mem: 100 * 1024 * 1024,
            dump: false,
            trace: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OperationKey {
    type_: glib_sys::GType,
    signature: SignatureHash,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SignatureHash(u64);

#[derive(Clone, Copy)]
struct CacheEntry {
    operation: *mut VipsOperation,
    key: OperationKey,
    time: u64,
}

unsafe impl Send for CacheEntry {}

struct CacheState {
    config: CacheConfig,
    entries: Vec<CacheEntry>,
    time: u64,
}

impl Default for CacheState {
    fn default() -> Self {
        Self {
            config: CacheConfig::default(),
            entries: Vec::new(),
            time: 0,
        }
    }
}

fn state() -> &'static Mutex<CacheState> {
    static STATE: OnceLock<Mutex<CacheState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(CacheState::default()))
}

unsafe fn object_class(object: *mut VipsObject) -> *mut VipsObjectClass {
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

unsafe fn operation_class(operation: *mut VipsOperation) -> *mut VipsOperationClass {
    unsafe { object_class(operation.cast::<VipsObject>()).cast::<VipsOperationClass>() }
}

fn mix_hash(state: &mut u64, bytes: &[u8]) {
    // FNV-1a is enough here: we need stable cache keys, not cryptographic strength.
    for &byte in bytes {
        *state ^= byte as u64;
        *state = state.wrapping_mul(0x100000001b3);
    }
}

fn mix_u64(state: &mut u64, value: u64) {
    mix_hash(state, &value.to_le_bytes());
}

struct SignatureState {
    hash: u64,
}

unsafe extern "C" fn hash_operation_arg(
    object: *mut VipsObject,
    pspec: *mut gobject_sys::GParamSpec,
    argument_class: *mut VipsArgumentClass,
    argument_instance: *mut VipsArgumentInstance,
    a: *mut libc::c_void,
    _b: *mut libc::c_void,
) -> *mut libc::c_void {
    if object.is_null()
        || pspec.is_null()
        || argument_class.is_null()
        || argument_instance.is_null()
        || a.is_null()
    {
        return ptr::null_mut();
    }

    let flags = unsafe { (*argument_class).flags };
    if (flags & VIPS_ARGUMENT_CONSTRUCT) == 0
        || (flags & VIPS_ARGUMENT_INPUT) == 0
        || (flags & VIPS_ARGUMENT_NON_HASHABLE) != 0
        || unsafe { (*argument_instance).assigned } == glib_sys::GFALSE
    {
        return ptr::null_mut();
    }

    let state = unsafe { &mut *a.cast::<SignatureState>() };
    let name = unsafe { CStr::from_ptr(gobject_sys::g_param_spec_get_name(pspec)) };
    mix_hash(&mut state.hash, name.to_bytes());
    mix_hash(&mut state.hash, b"=");

    let value_type = unsafe { (*pspec).value_type };
    let mut value: gobject_sys::GValue = unsafe { mem::zeroed() };
    unsafe {
        gobject_sys::g_value_init(&mut value, value_type);
        gobject_sys::g_object_get_property(
            object.cast::<gobject_sys::GObject>(),
            name.as_ptr(),
            &mut value,
        );
    }
    let rendered = unsafe { gobject_sys::g_strdup_value_contents(&value) };
    if rendered.is_null() {
        mix_hash(&mut state.hash, b"<null>");
    } else {
        mix_hash(
            &mut state.hash,
            unsafe { CStr::from_ptr(rendered) }.to_bytes(),
        );
        unsafe {
            glib_sys::g_free(rendered.cast());
        }
    }
    unsafe {
        gobject_sys::g_value_unset(&mut value);
    }
    mix_hash(&mut state.hash, b";");
    ptr::null_mut()
}

unsafe fn operation_key(operation: *mut VipsOperation) -> OperationKey {
    let type_ = unsafe {
        (*(*operation.cast::<gobject_sys::GObject>())
            .g_type_instance
            .g_class)
            .g_type
    };
    let mut state = SignatureState {
        hash: 0xcbf29ce484222325,
    };
    mix_u64(&mut state.hash, type_ as u64);
    vips_argument_map(
        operation.cast::<VipsObject>(),
        Some(hash_operation_arg),
        (&mut state as *mut SignatureState).cast(),
        ptr::null_mut(),
    );
    OperationKey {
        type_,
        signature: SignatureHash(state.hash | 1),
    }
}

unsafe fn bool_property(operation: *mut VipsOperation, name: &CStr) -> bool {
    let class = unsafe { object_class(operation.cast::<VipsObject>()) };
    if class.is_null() {
        return false;
    }
    let pspec = unsafe {
        gobject_sys::g_object_class_find_property(class.cast::<gobject_sys::GObjectClass>(), name.as_ptr())
    };
    if pspec.is_null() {
        return false;
    }

    let mut value: gobject_sys::GValue = unsafe { mem::zeroed() };
    unsafe {
        gobject_sys::g_value_init(&mut value, (*pspec).value_type);
        gobject_sys::g_object_get_property(
            operation.cast::<gobject_sys::GObject>(),
            name.as_ptr(),
            &mut value,
        );
    }
    let enabled = unsafe { gobject_sys::g_value_get_boolean(&value) } != glib_sys::GFALSE;
    unsafe {
        gobject_sys::g_value_unset(&mut value);
    }
    enabled
}

unsafe fn operation_flags(operation: *mut VipsOperation) -> VipsOperationFlags {
    let class = unsafe { operation_class(operation) };
    let Some(class) = (unsafe { class.as_ref() }) else {
        return 0;
    };

    class
        .get_flags
        .map(|get_flags| unsafe { get_flags(operation) })
        .unwrap_or(class.flags)
}

unsafe fn bypass_operation_cache(operation: *mut VipsOperation, flags: VipsOperationFlags) -> bool {
    (flags & VIPS_OPERATION_NOCACHE) != 0 || unsafe { bool_property(operation, c"nocache") }
}

unsafe fn build_operation(operation: *mut VipsOperation) -> libc::c_int {
    let Some(object) = (unsafe { operation.cast::<VipsObject>().as_mut() }) else {
        append_message_str("vips_cache_operation_buildp", "operation is NULL");
        return -1;
    };
    if object.constructed != glib_sys::GFALSE {
        return 0;
    }

    let Some(class) = (unsafe { object_class(object).as_ref() }) else {
        object.constructed = glib_sys::GTRUE;
        return 0;
    };
    if let Some(build) = class.build {
        if unsafe { build(object) } != 0 {
            return -1;
        }
    }
    if let Some(postbuild) = class.postbuild {
        if unsafe { postbuild(object, ptr::null_mut()) } != 0 {
            return -1;
        }
    }

    object.constructed = glib_sys::GTRUE;
    0
}

fn next_time(state: &mut CacheState) -> u64 {
    state.time = state.time.wrapping_add(1);
    state.time
}

fn trace_message(config: CacheConfig, message: &str, operation: *mut VipsOperation) {
    if config.trace {
        let class = unsafe { operation_class(operation) };
        let nickname = if class.is_null() || unsafe { (*class).parent_class.nickname }.is_null() {
            "unknown".to_string()
        } else {
            unsafe { CStr::from_ptr((*class).parent_class.nickname) }
                .to_string_lossy()
                .into_owned()
        };
        let key = unsafe { operation_key(operation) };
        eprintln!(
            "vips cache {message}: {operation:p} {nickname} key={:016x}",
            key.signature.0
        );
    }
}

fn lookup_locked(state: &mut CacheState, key: OperationKey) -> Option<*mut VipsOperation> {
    let index = state.entries.iter().position(|entry| entry.key == key)?;
    let time = next_time(state);
    let entry = &mut state.entries[index];
    entry.time = time;
    Some(entry.operation)
}

fn insert_locked(state: &mut CacheState, operation: *mut VipsOperation, key: OperationKey) {
    let time = next_time(state);
    state.entries.push(CacheEntry {
        operation: unsafe { object_ref(operation) },
        key,
        time,
    });
}

fn remove_matching_locked(state: &mut CacheState, key: OperationKey) -> Vec<CacheEntry> {
    let mut removed = Vec::new();
    let mut kept = Vec::with_capacity(state.entries.len());
    for entry in state.entries.drain(..) {
        if entry.key == key {
            removed.push(entry);
        } else {
            kept.push(entry);
        }
    }
    state.entries = kept;
    removed
}

fn pop_lru_locked(state: &mut CacheState) -> Option<CacheEntry> {
    let index = state
        .entries
        .iter()
        .enumerate()
        .min_by_key(|(_, entry)| entry.time)
        .map(|(index, _)| index)?;
    Some(state.entries.remove(index))
}

fn trim_locked(state: &mut CacheState) -> Vec<CacheEntry> {
    let mut removed = Vec::new();
    while (state.entries.len() as i32) > state.config.max
        || vips_tracked_get_files() > state.config.max_files
        || vips_tracked_get_mem() > state.config.max_mem
    {
        let Some(entry) = pop_lru_locked(state) else {
            break;
        };
        trace_message(state.config, "-", entry.operation);
        removed.push(entry);
    }
    removed
}

fn unref_entries(entries: Vec<CacheEntry>) {
    for entry in entries {
        unsafe {
            object_unref(entry.operation);
        }
    }
}

fn set_cache_limit(update: impl FnOnce(&mut CacheConfig)) {
    let removed = {
        let mut state = state().lock().expect("cache state");
        update(&mut state.config);
        trim_locked(&mut state)
    };
    unref_entries(removed);
}

#[no_mangle]
pub extern "C" fn vips_cache_drop_all() {
    let (config, removed) = {
        let mut state = state().lock().expect("cache state");
        if state.config.dump && !state.entries.is_empty() {
            eprintln!("vips cache drop_all: {} entries", state.entries.len());
        }
        (state.config, std::mem::take(&mut state.entries))
    };
    if config.trace && !removed.is_empty() {
        eprintln!("vips cache drop_all: {} entries", removed.len());
        for entry in &removed {
            trace_message(config, "-", entry.operation);
        }
    }
    unref_entries(removed);
}

#[no_mangle]
pub extern "C" fn vips_cache_operation_lookup(operation: *mut VipsOperation) -> *mut VipsOperation {
    if operation.is_null() {
        return ptr::null_mut();
    }

    let hit = {
        let mut state = state().lock().expect("cache state");
        lookup_locked(&mut state, unsafe { operation_key(operation) })
    };
    if let Some(hit) = hit {
        unsafe { object_ref(hit) }
    } else {
        ptr::null_mut()
    }
}

#[no_mangle]
pub extern "C" fn vips_cache_operation_add(operation: *mut VipsOperation) {
    if operation.is_null() {
        return;
    }

    let flags = unsafe { operation_flags(operation) };
    if unsafe { bypass_operation_cache(operation, flags) } || (flags & VIPS_OPERATION_BLOCKED) != 0
    {
        return;
    }

    let removed = {
        let mut state = state().lock().expect("cache state");
        let key = unsafe { operation_key(operation) };
        if lookup_locked(&mut state, key).is_none() {
            trace_message(state.config, "+", operation);
            insert_locked(&mut state, operation, key);
        }
        trim_locked(&mut state)
    };
    unref_entries(removed);
}

#[no_mangle]
pub extern "C" fn vips_cache_operation_buildp(operation: *mut *mut VipsOperation) -> libc::c_int {
    if operation.is_null() {
        append_message_str("vips_cache_operation_buildp", "operation pointer is NULL");
        return -1;
    }

    let mut current = unsafe { *operation };
    if current.is_null() {
        append_message_str("vips_cache_operation_buildp", "operation is NULL");
        return -1;
    }

    let mut flags = unsafe { operation_flags(current) };
    if (flags & VIPS_OPERATION_BLOCKED) != 0 {
        append_message_str("vips_cache_operation_buildp", "operation is blocked");
        return -1;
    }
    let bypass_cache = unsafe { bypass_operation_cache(current, flags) };

    let mut removed = Vec::new();
    let key = unsafe { operation_key(current) };
    if !bypass_cache {
        let revalidate = (flags & VIPS_OPERATION_REVALIDATE) != 0
            || unsafe { bool_property(current, c"revalidate") };
        if revalidate {
            removed = {
                let mut state = state().lock().expect("cache state");
                remove_matching_locked(&mut state, key)
            };
        } else {
            let hit = {
                let mut state = state().lock().expect("cache state");
                lookup_locked(&mut state, key)
            };
            if let Some(hit) = hit {
                unsafe {
                    object_ref(hit);
                    object_unref(current);
                    *operation = hit;
                }
                return 0;
            }
        }
    }

    if unsafe { build_operation(current) } != 0 {
        unref_entries(removed);
        return -1;
    }

    flags = unsafe { operation_flags(current) };
    if (flags & VIPS_OPERATION_BLOCKED) != 0 {
        unref_entries(removed);
        append_message_str("vips_cache_operation_buildp", "operation is blocked");
        return -1;
    }

    let (hit, mut trimmed) = {
        let mut state = state().lock().expect("cache state");
        let key = unsafe { operation_key(current) };
        let hit = if bypass_cache {
            None
        } else {
            lookup_locked(&mut state, key)
        };
        if hit.is_none() && !bypass_cache {
            trace_message(state.config, "+", current);
            insert_locked(&mut state, current, key);
        }
        (hit, trim_locked(&mut state))
    };
    removed.append(&mut trimmed);

    if let Some(hit) = hit {
        unsafe {
            object_ref(hit);
            object_unref(current);
            *operation = hit;
        }
    } else {
        current = unsafe { *operation };
        trace_message(
            {
                let state = state().lock().expect("cache state");
                state.config
            },
            if bypass_cache {
                ":"
            } else {
                "*"
            },
            current,
        );
    }

    unref_entries(removed);
    0
}

#[no_mangle]
pub extern "C" fn vips_cache_operation_build(operation: *mut VipsOperation) -> *mut VipsOperation {
    if operation.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        object_ref(operation);
    }
    let mut built = operation;
    if vips_cache_operation_buildp(&mut built) != 0 {
        unsafe {
            object_unref(operation);
        }
        return ptr::null_mut();
    }

    built
}

#[no_mangle]
pub extern "C" fn vips_cache_print() {
    let state = state().lock().expect("cache state");
    eprintln!(
        "vips cache: size={}, max={}, max_files={}, max_mem={}, dump={}, trace={}",
        state.entries.len(),
        state.config.max,
        state.config.max_files,
        state.config.max_mem,
        state.config.dump,
        state.config.trace
    );
}

#[no_mangle]
pub extern "C" fn vips_cache_set_max(max: libc::c_int) {
    set_cache_limit(|config| {
        config.max = max.max(0);
    });
}

#[no_mangle]
pub extern "C" fn vips_cache_set_max_mem(max_mem: usize) {
    set_cache_limit(|config| {
        config.max_mem = max_mem;
    });
}

#[no_mangle]
pub extern "C" fn vips_cache_get_max() -> libc::c_int {
    state().lock().expect("cache state").config.max
}

#[no_mangle]
pub extern "C" fn vips_cache_get_size() -> libc::c_int {
    state().lock().expect("cache state").entries.len() as libc::c_int
}

#[no_mangle]
pub extern "C" fn vips_cache_get_max_mem() -> usize {
    state().lock().expect("cache state").config.max_mem
}

#[no_mangle]
pub extern "C" fn vips_cache_get_max_files() -> libc::c_int {
    state().lock().expect("cache state").config.max_files
}

#[no_mangle]
pub extern "C" fn vips_cache_set_max_files(max_files: libc::c_int) {
    set_cache_limit(|config| {
        config.max_files = max_files.max(0);
    });
}

#[no_mangle]
pub extern "C" fn vips_cache_set_dump(dump: glib_sys::gboolean) {
    state().lock().expect("cache state").config.dump = dump != glib_sys::GFALSE;
}

#[no_mangle]
pub extern "C" fn vips_cache_set_trace(trace: glib_sys::gboolean) {
    state().lock().expect("cache state").config.trace = trace != glib_sys::GFALSE;
}
