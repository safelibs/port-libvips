use std::ffi::CStr;
use std::ptr;
use std::sync::{Mutex, OnceLock};

use libc::{c_char, c_int, c_void};

use crate::abi::object::VipsObject;
use crate::runtime::object::{get_qdata_ptr, qdata_quark, set_qdata_box};

const PREFIX_SIZE: usize = 16;
static LOCAL_ALLOCS_QUARK: &CStr = c"safe-vips-local-allocs";

#[derive(Default)]
struct TrackedState {
    mem: usize,
    highwater: usize,
    allocs: i32,
    files: i32,
}

#[derive(Default)]
struct LocalAllocs {
    ptrs: Vec<*mut c_void>,
}

impl Drop for LocalAllocs {
    fn drop(&mut self) {
        for ptr in self.ptrs.drain(..) {
            unsafe {
                glib_sys::g_free(ptr);
            }
        }
    }
}

fn tracked_state() -> &'static Mutex<TrackedState> {
    static STATE: OnceLock<Mutex<TrackedState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(TrackedState::default()))
}

unsafe fn local_allocs(object: *mut VipsObject) -> *mut LocalAllocs {
    let quark = qdata_quark(LOCAL_ALLOCS_QUARK);
    let object = object.cast::<gobject_sys::GObject>();
    let existing = unsafe { get_qdata_ptr::<LocalAllocs>(object, quark) };
    if !existing.is_null() {
        return existing;
    }

    unsafe {
        set_qdata_box(object, quark, LocalAllocs::default());
        get_qdata_ptr::<LocalAllocs>(object, quark)
    }
}

unsafe fn track_local_alloc(object: *mut VipsObject, ptr: *mut c_void, size: usize) {
    if object.is_null() || ptr.is_null() {
        return;
    }

    let local = unsafe { local_allocs(object) };
    if let Some(local) = unsafe { local.as_mut() } {
        local.ptrs.push(ptr);
        unsafe {
            (*object).local_memory = (*object).local_memory.saturating_add(size);
        }
    }
}

fn record_allocation(size: usize) {
    let mut tracked = tracked_state().lock().expect("tracked state");
    tracked.mem = tracked.mem.saturating_add(size);
    tracked.highwater = tracked.highwater.max(tracked.mem);
    tracked.allocs += 1;
}

fn record_free(size: usize) {
    let mut tracked = tracked_state().lock().expect("tracked state");
    tracked.mem = tracked.mem.saturating_sub(size);
    tracked.allocs = tracked.allocs.saturating_sub(1);
}

fn record_file(delta: i32) {
    let mut tracked = tracked_state().lock().expect("tracked state");
    tracked.files = (tracked.files + delta).max(0);
}

unsafe fn tracked_prefix(ptr: *mut c_void) -> *mut u8 {
    unsafe { ptr.cast::<u8>().sub(PREFIX_SIZE) }
}

unsafe fn tracked_size(ptr: *mut c_void) -> usize {
    unsafe { *(tracked_prefix(ptr).cast::<usize>()) }
}

#[no_mangle]
pub extern "C" fn vips_malloc(object: *mut VipsObject, size: usize) -> *mut c_void {
    let ptr = unsafe { glib_sys::g_malloc0(size) };
    unsafe {
        track_local_alloc(object, ptr, size);
    }
    ptr
}

#[no_mangle]
pub extern "C" fn vips_strdup(object: *mut VipsObject, str_: *const c_char) -> *mut c_char {
    if str_.is_null() {
        return ptr::null_mut();
    }

    let dup = unsafe { glib_sys::g_strdup(str_) };
    let size = unsafe { CStr::from_ptr(str_) }.to_bytes().len();
    unsafe {
        track_local_alloc(object, dup.cast::<c_void>(), size);
    }
    dup
}

#[no_mangle]
pub extern "C" fn vips_tracked_malloc(size: usize) -> *mut c_void {
    let raw = unsafe { glib_sys::g_malloc0(size.saturating_add(PREFIX_SIZE)) }.cast::<u8>();
    if raw.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        *(raw.cast::<usize>()) = size;
    }
    record_allocation(size);
    unsafe { raw.add(PREFIX_SIZE).cast::<c_void>() }
}

#[no_mangle]
pub extern "C" fn vips_tracked_free(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }

    let size = unsafe { tracked_size(ptr) };
    record_free(size);
    unsafe {
        glib_sys::g_free(tracked_prefix(ptr).cast::<c_void>());
    }
}

#[no_mangle]
pub extern "C" fn vips_tracked_aligned_alloc(size: usize, align: usize) -> *mut c_void {
    if align == 0 {
        return vips_tracked_malloc(size);
    }

    let header_words = 2 * std::mem::size_of::<usize>();
    let total = size
        .saturating_add(align)
        .saturating_add(header_words)
        .saturating_add(PREFIX_SIZE);
    let raw = unsafe { glib_sys::g_malloc0(total) }.cast::<u8>();
    if raw.is_null() {
        return ptr::null_mut();
    }

    let start = unsafe { raw.add(header_words) } as usize;
    let aligned = (start + (align - 1)) & !(align - 1);
    let aligned_ptr = aligned as *mut u8;

    unsafe {
        *(aligned_ptr.cast::<usize>().sub(2)) = raw as usize;
        *(aligned_ptr.cast::<usize>().sub(1)) = size;
    }
    record_allocation(size);
    aligned_ptr.cast::<c_void>()
}

#[no_mangle]
pub extern "C" fn vips_tracked_aligned_free(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }

    let aligned_ptr = ptr.cast::<usize>();
    let raw = unsafe { *aligned_ptr.sub(2) as *mut c_void };
    let size = unsafe { *aligned_ptr.sub(1) };
    record_free(size);
    unsafe {
        glib_sys::g_free(raw);
    }
}

#[no_mangle]
pub extern "C" fn vips_tracked_get_mem() -> usize {
    tracked_state().lock().expect("tracked state").mem
}

#[no_mangle]
pub extern "C" fn vips_tracked_get_mem_highwater() -> usize {
    tracked_state().lock().expect("tracked state").highwater
}

#[no_mangle]
pub extern "C" fn vips_tracked_get_allocs() -> c_int {
    tracked_state().lock().expect("tracked state").allocs
}

#[no_mangle]
pub extern "C" fn vips_tracked_get_files() -> c_int {
    tracked_state().lock().expect("tracked state").files
}

#[no_mangle]
pub extern "C" fn vips_tracked_open(pathname: *const c_char, flags: c_int, mode: c_int) -> c_int {
    let fd = crate::runtime::vips_native::vips__open(pathname, flags, mode);
    if fd >= 0 {
        record_file(1);
    }
    fd
}

#[no_mangle]
pub extern "C" fn vips_tracked_close(fd: c_int) -> c_int {
    if fd < 0 {
        return -1;
    }

    let result = unsafe { libc::close(fd) };
    if result == 0 {
        record_file(-1);
    }
    result
}
