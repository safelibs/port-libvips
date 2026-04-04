use std::sync::{Mutex, OnceLock};

use crate::abi::operation::VipsOperation;

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

fn config() -> &'static Mutex<CacheConfig> {
    static CONFIG: OnceLock<Mutex<CacheConfig>> = OnceLock::new();
    CONFIG.get_or_init(|| Mutex::new(CacheConfig::default()))
}

#[no_mangle]
pub extern "C" fn vips_cache_drop_all() {}

#[no_mangle]
pub extern "C" fn vips_cache_operation_lookup(operation: *mut VipsOperation) -> *mut VipsOperation {
    operation
}

#[no_mangle]
pub extern "C" fn vips_cache_operation_add(_operation: *mut VipsOperation) {}

#[no_mangle]
pub extern "C" fn vips_cache_operation_buildp(operation: *mut *mut VipsOperation) -> libc::c_int {
    if operation.is_null() || unsafe { *operation }.is_null() {
        -1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn vips_cache_operation_build(operation: *mut VipsOperation) -> *mut VipsOperation {
    operation
}

#[no_mangle]
pub extern "C" fn vips_cache_print() {
    let config = config().lock().expect("cache config");
    eprintln!(
        "vips cache: max={}, max_files={}, max_mem={}, dump={}, trace={}",
        config.max, config.max_files, config.max_mem, config.dump, config.trace
    );
}

#[no_mangle]
pub extern "C" fn vips_cache_set_max(max: libc::c_int) {
    config().lock().expect("cache config").max = max.max(0);
}

#[no_mangle]
pub extern "C" fn vips_cache_set_max_mem(max_mem: usize) {
    config().lock().expect("cache config").max_mem = max_mem;
}

#[no_mangle]
pub extern "C" fn vips_cache_get_max() -> libc::c_int {
    config().lock().expect("cache config").max
}

#[no_mangle]
pub extern "C" fn vips_cache_get_size() -> libc::c_int {
    0
}

#[no_mangle]
pub extern "C" fn vips_cache_get_max_mem() -> usize {
    config().lock().expect("cache config").max_mem
}

#[no_mangle]
pub extern "C" fn vips_cache_get_max_files() -> libc::c_int {
    config().lock().expect("cache config").max_files
}

#[no_mangle]
pub extern "C" fn vips_cache_set_max_files(max_files: libc::c_int) {
    config().lock().expect("cache config").max_files = max_files.max(0);
}

#[no_mangle]
pub extern "C" fn vips_cache_set_dump(dump: glib_sys::gboolean) {
    config().lock().expect("cache config").dump = dump != glib_sys::GFALSE;
}

#[no_mangle]
pub extern "C" fn vips_cache_set_trace(trace: glib_sys::gboolean) {
    config().lock().expect("cache config").trace = trace != glib_sys::GFALSE;
}
