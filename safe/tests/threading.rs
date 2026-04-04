use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, Mutex, Once, OnceLock};

use vips::*;

unsafe extern "C" {
    fn vips_thumbnail_source(
        source: *mut VipsSource,
        out: *mut *mut VipsImage,
        width: i32,
        ...
    ) -> i32;
}

struct FailingSourceState {
    read_calls: AtomicUsize,
}

fn guard() -> std::sync::MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    match GUARD.get_or_init(|| Mutex::new(())).lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn init_vips() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        assert_eq!(vips_init(c"threading".as_ptr()), 0);
    });
}

fn states() -> &'static Mutex<HashMap<usize, Arc<FailingSourceState>>> {
    static STATES: OnceLock<Mutex<HashMap<usize, Arc<FailingSourceState>>>> = OnceLock::new();
    STATES.get_or_init(|| Mutex::new(HashMap::new()))
}

unsafe extern "C" fn read_cb(
    source: *mut VipsSourceCustom,
    _data: *mut c_void,
    _length: i64,
    _user_data: glib_sys::gpointer,
) -> i64 {
    if let Some(state) = states().lock().expect("states").get(&(source as usize)).cloned() {
        state.read_calls.fetch_add(1, Ordering::SeqCst);
    }
    -1
}

unsafe extern "C" fn seek_cb(
    _source: *mut VipsSourceCustom,
    offset: i64,
    whence: i32,
    _user_data: glib_sys::gpointer,
) -> i64 {
    if offset == 0 && whence == libc::SEEK_SET {
        0
    } else {
        -1
    }
}

fn failing_source() -> (*mut VipsSource, Arc<FailingSourceState>) {
    let source = vips_source_custom_new().cast::<VipsSource>();
    let state = Arc::new(FailingSourceState {
        read_calls: AtomicUsize::new(0),
    });
    states()
        .lock()
        .expect("states")
        .insert(source as usize, Arc::clone(&state));

    unsafe {
        gobject_sys::g_signal_connect_data(
            source.cast(),
            c"read".as_ptr(),
            Some(std::mem::transmute::<
                unsafe extern "C" fn(*mut VipsSourceCustom, *mut c_void, i64, glib_sys::gpointer) -> i64,
                unsafe extern "C" fn(),
            >(read_cb)),
            ptr::null_mut(),
            None,
            0,
        );
        gobject_sys::g_signal_connect_data(
            source.cast(),
            c"seek".as_ptr(),
            Some(std::mem::transmute::<
                unsafe extern "C" fn(*mut VipsSourceCustom, i64, i32, glib_sys::gpointer) -> i64,
                unsafe extern "C" fn(),
            >(seek_cb)),
            ptr::null_mut(),
            None,
            0,
        );
    }

    (source, state)
}

fn cleanup_source(source: *mut VipsSource) {
    states().lock().expect("states").remove(&(source as usize));
    unsafe {
        gobject_sys::g_object_unref(source.cast());
    }
}

#[test]
fn delayed_load_failure_is_cached_across_threads() {
    let _guard = guard();
    init_vips();

    let (source, state) = failing_source();
    let barrier = Arc::new(Barrier::new(4));
    let source_ptr = source as usize;

    let mut threads = Vec::new();
    for _ in 0..4 {
        let barrier = Arc::clone(&barrier);
        threads.push(std::thread::spawn(move || {
            barrier.wait();
            let source = source_ptr as *mut VipsSource;
            let mut out = ptr::null_mut();
            let result = unsafe { vips_thumbnail_source(source, &mut out, 16, ptr::null::<std::ffi::c_char>()) };
            assert!(out.is_null());
            result
        }));
    }

    let results = threads
        .into_iter()
        .map(|thread| thread.join().expect("thread"))
        .collect::<Vec<_>>();
    assert!(results.iter().all(|result| *result == -1));
    assert_eq!(state.read_calls.load(Ordering::SeqCst), 1);

    cleanup_source(source);
}
