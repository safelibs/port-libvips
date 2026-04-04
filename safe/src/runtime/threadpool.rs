use std::cell::Cell;
use std::ffi::CStr;
use std::ptr;
use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};

use crate::abi::image::{
    VipsDemandStyle, VipsImage, VIPS_DEMAND_STYLE_FATSTRIP, VIPS_DEMAND_STYLE_SMALLTILE,
    VIPS_DEMAND_STYLE_THINSTRIP,
};
use crate::abi::object::VipsObject;
use crate::abi::operation::{
    VipsThreadStartFn, VipsThreadState, VipsThreadpoolAllocateFn, VipsThreadpoolProgressFn,
    VipsThreadpoolWorkFn,
};
use crate::runtime::object::{object_new, object_ref, object_unref, qdata_quark, set_qdata_box};

static CONCURRENCY: AtomicI32 = AtomicI32::new(0);
static THREADPOOL_EPOCH: AtomicU64 = AtomicU64::new(1);
static THREAD_STATE_QUARK: &CStr = c"safe-vips-thread-state";

thread_local! {
    static THREAD_EPOCH: Cell<u64> = const { Cell::new(0) };
}

struct ThreadStateHold {
    image: *mut VipsImage,
    region: *mut crate::abi::region::VipsRegion,
}

impl Drop for ThreadStateHold {
    fn drop(&mut self) {
        unsafe {
            object_unref(self.region);
            object_unref(self.image);
        }
    }
}

fn max_threads_limit() -> i32 {
    let env = std::env::var("VIPS_MAX_THREADS")
        .ok()
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or(1024);
    env.clamp(3, 1024)
}

fn default_concurrency() -> i32 {
    let env = std::env::var("VIPS_CONCURRENCY")
        .ok()
        .and_then(|value| value.parse::<i32>().ok())
        .filter(|value| *value > 0);
    let processors = std::thread::available_parallelism()
        .map(|value| value.get() as i32)
        .unwrap_or(1);
    env.unwrap_or(processors).clamp(1, max_threads_limit())
}

fn thread_state_quark() -> glib_sys::GQuark {
    qdata_quark(THREAD_STATE_QUARK)
}

fn attach_thread_state() {
    let epoch = THREADPOOL_EPOCH.load(Ordering::Acquire);
    THREAD_EPOCH.with(|state| {
        if state.get() != epoch {
            state.set(epoch);
        }
    });
}

pub(crate) fn thread_shutdown() {
    THREAD_EPOCH.with(|state| state.set(0));
}

pub(crate) fn shutdown_runtime_state() {
    CONCURRENCY.store(0, Ordering::Relaxed);
    THREADPOOL_EPOCH.fetch_add(1, Ordering::AcqRel);
    thread_shutdown();
}

#[no_mangle]
pub extern "C" fn vips_concurrency_set(concurrency: libc::c_int) {
    let value = if concurrency < 1 {
        default_concurrency()
    } else {
        concurrency.clamp(1, max_threads_limit())
    };
    CONCURRENCY.store(value, Ordering::Relaxed);
}

#[no_mangle]
pub extern "C" fn vips_concurrency_get() -> libc::c_int {
    let configured = CONCURRENCY.load(Ordering::Relaxed);
    if configured > 0 {
        configured
    } else {
        default_concurrency()
    }
}

#[no_mangle]
pub extern "C" fn vips_thread_state_set(
    object: *mut VipsObject,
    a: *mut libc::c_void,
    b: *mut libc::c_void,
) -> *mut libc::c_void {
    let Some(state) = (unsafe { object.cast::<VipsThreadState>().as_mut() }) else {
        return ptr::null_mut();
    };
    state.im = a.cast::<VipsImage>();
    state.a = b;
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn vips_thread_state_new(
    im: *mut VipsImage,
    a: *mut libc::c_void,
) -> *mut VipsThreadState {
    if im.is_null() {
        return ptr::null_mut();
    }
    attach_thread_state();
    let state = unsafe {
        object_new::<VipsThreadState>(crate::runtime::object::vips_thread_state_get_type())
    };
    let Some(state_ref) = (unsafe { state.as_mut() }) else {
        return ptr::null_mut();
    };
    let image = unsafe { object_ref(im) };
    let region = crate::runtime::region::vips_region_new(im);
    state_ref.im = image;
    state_ref.reg = region;
    state_ref.pos = crate::abi::basic::VipsRect {
        left: 0,
        top: 0,
        width: 0,
        height: 0,
    };
    state_ref.x = 0;
    state_ref.y = 0;
    state_ref.stop = glib_sys::GFALSE;
    state_ref.a = a;
    state_ref.stall = glib_sys::GFALSE;
    unsafe {
        set_qdata_box(
            state.cast(),
            thread_state_quark(),
            ThreadStateHold { image, region },
        );
    }
    state
}

#[no_mangle]
pub extern "C" fn vips_threadpool_run(
    im: *mut VipsImage,
    start: VipsThreadStartFn,
    allocate: VipsThreadpoolAllocateFn,
    work: VipsThreadpoolWorkFn,
    progress: VipsThreadpoolProgressFn,
    a: *mut libc::c_void,
) -> libc::c_int {
    if im.is_null() {
        return -1;
    }
    attach_thread_state();
    let start_fn = start.unwrap_or(vips_thread_state_new);
    let state = unsafe { start_fn(im, a) };
    if state.is_null() {
        return -1;
    }

    let mut result = 0;
    loop {
        let mut stop = glib_sys::GFALSE;
        if let Some(allocate_fn) = allocate {
            if unsafe { allocate_fn(state, a, &mut stop) } != 0 {
                result = -1;
                break;
            }
            if let Some(progress_fn) = progress {
                if unsafe { progress_fn(a) } != 0 {
                    result = -1;
                    break;
                }
            }
            if stop != glib_sys::GFALSE {
                break;
            }
        }
        if let Some(work_fn) = work {
            if unsafe { work_fn(state, a) } != 0 {
                result = -1;
                break;
            }
        }
        if allocate.is_none()
            || unsafe { state.as_ref() }.is_some_and(|state| state.stop != glib_sys::GFALSE)
        {
            break;
        }
    }

    unsafe {
        object_unref(state);
    }
    result
}

#[no_mangle]
pub extern "C" fn vips_get_tile_size(
    im: *mut VipsImage,
    tile_width: *mut libc::c_int,
    tile_height: *mut libc::c_int,
    n_lines: *mut libc::c_int,
) {
    let Some(image) = (unsafe { im.as_ref() }) else {
        return;
    };
    let (width, height, lines) = match image.dhint as VipsDemandStyle {
        VIPS_DEMAND_STYLE_SMALLTILE => (128, 128, 1),
        VIPS_DEMAND_STYLE_FATSTRIP => (image.Xsize.max(1), 64, 64),
        VIPS_DEMAND_STYLE_THINSTRIP => (image.Xsize.max(1), 16, 16),
        _ => (image.Xsize.clamp(1, 128), image.Ysize.clamp(1, 128), 16),
    };
    unsafe {
        if !tile_width.is_null() {
            *tile_width = width;
        }
        if !tile_height.is_null() {
            *tile_height = height;
        }
        if !n_lines.is_null() {
            *n_lines = lines;
        }
    }
}
