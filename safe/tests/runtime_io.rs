use std::ffi::{CStr, c_void};
use std::os::raw::c_char;
use std::path::PathBuf;
use std::ptr;
use std::slice;
use std::sync::{Mutex, Once, OnceLock};

use gobject_sys::{g_signal_connect_data, GObject};
use vips::*;

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
        let argv0 = c"runtime_io";
        assert_eq!(vips_init(argv0.as_ptr()), 0);
    });
}

fn sample_png() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("original")
        .join("test")
        .join("test-suite")
        .join("images")
        .join("sample.png")
}

unsafe fn connect_signal<Cb>(instance: *mut GObject, signal: &CStr, callback: Cb, data: *mut c_void)
where
    Cb: Copy,
{
    unsafe {
        g_signal_connect_data(
            instance,
            signal.as_ptr(),
            Some(std::mem::transmute_copy(&callback)),
            data,
            None,
            0,
        );
    }
}

#[test]
fn buf_and_dbuf_helpers_behave() {
    let _guard = guard();
    init_vips();

    let mut storage = [0 as c_char; 64];
    let mut buf = VipsBuf {
        base: ptr::null_mut(),
        mx: 0,
        i: 0,
        full: glib_sys::GFALSE,
        lasti: 0,
        dynamic: glib_sys::GFALSE,
    };
    vips_buf_init_static(&mut buf, storage.as_mut_ptr(), storage.len() as i32);
    assert_eq!(vips_buf_appends(&mut buf, c"hello\nworld".as_ptr()), glib_sys::GTRUE);
    assert_eq!(unsafe { CStr::from_ptr(vips_buf_firstline(&mut buf)) }.to_str().unwrap(), "hello");
    assert_eq!(vips_buf_len(&mut buf), 11);

    let mut dbuf = VipsDbuf {
        data: ptr::null_mut(),
        allocated_size: 0,
        data_size: 0,
        write_point: 0,
    };
    vips_dbuf_init(&mut dbuf);
    assert_eq!(vips_dbuf_write(&mut dbuf, b"ab".as_ptr(), 2), glib_sys::GTRUE);
    assert_eq!(vips_dbuf_seek(&mut dbuf, 4, libc::SEEK_SET), glib_sys::GTRUE);
    assert_eq!(vips_dbuf_write(&mut dbuf, b"z".as_ptr(), 1), glib_sys::GTRUE);
    let mut len = 0usize;
    let bytes = vips_dbuf_string(&mut dbuf, &mut len);
    let bytes = unsafe { slice::from_raw_parts(bytes, len) };
    assert_eq!(bytes, b"ab\0\0z");
    vips_dbuf_destroy(&mut dbuf);
}

#[test]
fn metadata_get_fields_and_zeroed_region_buffers_work() {
    let _guard = guard();
    init_vips();

    let image = vips_image_new_memory();
    assert!(!image.is_null());
    vips_image_init_fields(
        image,
        4,
        4,
        1,
        VIPS_FORMAT_UCHAR,
        VIPS_CODING_NONE,
        VIPS_INTERPRETATION_B_W,
        1.0,
        1.0,
    );
    vips_image_set_int(image, c"page-height".as_ptr(), 7);
    vips_image_set_string(image, c"comment".as_ptr(), c"runtime".as_ptr());

    let fields = vips_image_get_fields(image);
    let mut names = Vec::new();
    let mut index = 0usize;
    loop {
        let value = unsafe { *fields.add(index) };
        if value.is_null() {
            break;
        }
        names.push(unsafe { CStr::from_ptr(value) }.to_string_lossy().into_owned());
        unsafe { glib_sys::g_free(value.cast::<c_void>()) };
        index += 1;
    }
    unsafe { glib_sys::g_free(fields.cast::<c_void>()) };
    assert!(names.iter().any(|name| name == "width"));
    assert!(names.iter().any(|name| name == "page-height"));
    assert!(names.iter().any(|name| name == "comment"));

    let region = vips_region_new(image);
    let area = VipsRect {
        left: 0,
        top: 0,
        width: 4,
        height: 4,
    };
    assert_eq!(vips_region_buffer(region, &area), 0);
    let region_ref = unsafe { &*region };
    let pixels = unsafe { slice::from_raw_parts(region_ref.data, 16) };
    assert!(pixels.iter().all(|value| *value == 0));

    unsafe {
        gobject_sys::g_object_unref(region.cast());
        gobject_sys::g_object_unref(image.cast());
    }
}

struct SourceCustomState {
    bytes: Vec<u8>,
    pos: usize,
}

unsafe extern "C" fn source_read_cb(
    _source: *mut VipsSourceCustom,
    buffer: *mut c_void,
    length: i64,
    state: *mut c_void,
) -> i64 {
    let state = unsafe { &mut *(state.cast::<SourceCustomState>()) };
    let remaining = state.bytes.len().saturating_sub(state.pos);
    let to_copy = remaining.min(length.max(0) as usize);
    if to_copy > 0 {
        unsafe {
            ptr::copy_nonoverlapping(state.bytes.as_ptr().add(state.pos), buffer.cast::<u8>(), to_copy);
        }
    }
    state.pos += to_copy;
    to_copy as i64
}

unsafe extern "C" fn source_seek_cb(
    _source: *mut VipsSourceCustom,
    offset: i64,
    whence: libc::c_int,
    state: *mut c_void,
) -> i64 {
    let state = unsafe { &mut *(state.cast::<SourceCustomState>()) };
    let next = match whence {
        libc::SEEK_SET => offset,
        libc::SEEK_CUR => state.pos as i64 + offset,
        libc::SEEK_END => state.bytes.len() as i64 + offset,
        _ => return -1,
    }
    .clamp(0, state.bytes.len() as i64);
    state.pos = next as usize;
    next
}

struct TargetCustomState {
    bytes: Vec<u8>,
    finished: bool,
    fail: bool,
}

unsafe extern "C" fn target_write_cb(
    _target: *mut VipsTargetCustom,
    data: *const c_void,
    length: i64,
    state: *mut c_void,
) -> i64 {
    let state = unsafe { &mut *(state.cast::<TargetCustomState>()) };
    if state.fail {
        return -1;
    }
    let bytes = unsafe { slice::from_raw_parts(data.cast::<u8>(), length.max(0) as usize) };
    state.bytes.extend_from_slice(bytes);
    length
}

unsafe extern "C" fn target_finish_cb(_target: *mut VipsTargetCustom, state: *mut c_void) {
    let state = unsafe { &mut *(state.cast::<TargetCustomState>()) };
    state.finished = true;
}

#[test]
fn custom_source_and_target_callbacks_round_trip_and_propagate_errors() {
    let _guard = guard();
    init_vips();

    let bytes = std::fs::read(sample_png()).expect("sample png");
    let source = vips_source_custom_new();
    let mut source_state = Box::new(SourceCustomState { bytes, pos: 0 });
    unsafe {
        connect_signal(
            source.cast::<GObject>(),
            c"read",
            source_read_cb
                as unsafe extern "C" fn(*mut VipsSourceCustom, *mut c_void, i64, *mut c_void) -> i64,
            (&mut *source_state as *mut SourceCustomState).cast(),
        );
        connect_signal(
            source.cast::<GObject>(),
            c"seek",
            source_seek_cb
                as unsafe extern "C" fn(*mut VipsSourceCustom, i64, libc::c_int, *mut c_void) -> i64,
            (&mut *source_state as *mut SourceCustomState).cast(),
        );
    }

    let image = safe_vips_image_new_from_source_internal(source.cast(), c"".as_ptr(), VIPS_ACCESS_SEQUENTIAL);
    assert!(!image.is_null());

    let target = vips_target_custom_new();
    let mut target_state = Box::new(TargetCustomState {
        bytes: Vec::new(),
        finished: false,
        fail: false,
    });
    unsafe {
        connect_signal(
            target.cast::<GObject>(),
            c"write",
            target_write_cb
                as unsafe extern "C" fn(*mut VipsTargetCustom, *const c_void, i64, *mut c_void) -> i64,
            (&mut *target_state as *mut TargetCustomState).cast(),
        );
        connect_signal(
            target.cast::<GObject>(),
            c"finish",
            target_finish_cb as unsafe extern "C" fn(*mut VipsTargetCustom, *mut c_void),
            (&mut *target_state as *mut TargetCustomState).cast(),
        );
    }

    assert_eq!(
        safe_vips_image_write_to_target_internal(image, c".png".as_ptr(), target.cast()),
        0
    );
    assert!(target_state.finished);
    assert!(target_state.bytes.starts_with(b"\x89PNG\r\n\x1a\n"));

    let failing = vips_target_custom_new();
    let mut failing_state = Box::new(TargetCustomState {
        bytes: Vec::new(),
        finished: false,
        fail: true,
    });
    unsafe {
        connect_signal(
            failing.cast::<GObject>(),
            c"write",
            target_write_cb
                as unsafe extern "C" fn(*mut VipsTargetCustom, *const c_void, i64, *mut c_void) -> i64,
            (&mut *failing_state as *mut TargetCustomState).cast(),
        );
    }
    assert_eq!(
        safe_vips_image_write_to_target_internal(image, c".png".as_ptr(), failing.cast()),
        -1
    );

    unsafe {
        gobject_sys::g_object_unref(failing.cast());
        gobject_sys::g_object_unref(target.cast());
        gobject_sys::g_object_unref(image.cast());
        gobject_sys::g_object_unref(source.cast());
    }
}

unsafe extern "C" fn gradient_generate(
    region: *mut VipsRegion,
    _seq: *mut c_void,
    _a: *mut c_void,
    _b: *mut c_void,
    _stop: *mut glib_sys::gboolean,
) -> libc::c_int {
    let region_ref = unsafe { &mut *region };
    for y in 0..region_ref.valid.height {
        for x in 0..region_ref.valid.width {
            let value = (region_ref.valid.left + x + region_ref.valid.top + y) as u8;
            let offset = y as usize * region_ref.bpl as usize + x as usize;
            unsafe {
                *region_ref.data.add(offset) = value;
            }
        }
    }
    0
}

#[test]
fn region_prepare_and_prepare_to_generate_on_demand() {
    let _guard = guard();
    init_vips();

    let image = vips_image_new();
    vips_image_init_fields(
        image,
        4,
        4,
        1,
        VIPS_FORMAT_UCHAR,
        VIPS_CODING_NONE,
        VIPS_INTERPRETATION_B_W,
        1.0,
        1.0,
    );
    assert_eq!(vips_image_pipeline_array(image, VIPS_DEMAND_STYLE_ANY, ptr::null_mut()), 0);
    assert_eq!(vips_image_generate(image, None, Some(gradient_generate), None, ptr::null_mut(), ptr::null_mut()), 0);

    let region = vips_region_new(image);
    let request = VipsRect {
        left: 1,
        top: 1,
        width: 2,
        height: 2,
    };
    assert_eq!(vips_region_prepare(region, &request), 0);
    let region_ref = unsafe { &*region };
    let generated = unsafe { slice::from_raw_parts(region_ref.data, 4) };
    assert_eq!(generated, &[2, 3, 3, 4]);

    let dest_image = vips_image_new_memory();
    vips_image_init_fields(
        dest_image,
        4,
        4,
        1,
        VIPS_FORMAT_UCHAR,
        VIPS_CODING_NONE,
        VIPS_INTERPRETATION_B_W,
        1.0,
        1.0,
    );
    let dest_region = vips_region_new(dest_image);
    let dest_area = VipsRect {
        left: 0,
        top: 0,
        width: 4,
        height: 4,
    };
    assert_eq!(vips_region_buffer(dest_region, &dest_area), 0);
    assert_eq!(vips_region_prepare_to(region, dest_region, &request, 0, 1), 0);
    let dest_ref = unsafe { &*dest_region };
    let dest_pixels = unsafe { slice::from_raw_parts(dest_ref.data, 16) };
    assert_eq!(&dest_pixels[4..6], &[2, 3]);
    assert_eq!(&dest_pixels[8..10], &[3, 4]);

    unsafe {
        gobject_sys::g_object_unref(dest_region.cast());
        gobject_sys::g_object_unref(dest_image.cast());
        gobject_sys::g_object_unref(region.cast());
        gobject_sys::g_object_unref(image.cast());
    }
}

#[test]
fn descriptor_reopens_do_not_leak_handles() {
    let _guard = guard();
    init_vips();

    let snapshot = std::fs::read_dir("/proc/self/fd").expect("fd dir").count();
    let path = std::ffi::CString::new(sample_png().to_string_lossy().into_owned()).expect("path");
    let source = vips_source_new_from_file(path.as_ptr());
    assert!(!source.is_null());
    let image = safe_vips_image_new_from_source_internal(source, c"".as_ptr(), VIPS_ACCESS_RANDOM);
    assert!(!image.is_null());
    let after_load = std::fs::read_dir("/proc/self/fd").expect("fd dir").count();
    assert_eq!(after_load, snapshot);

    let mut cropped = ptr::null_mut();
    let mut average = 0.0;
    assert_eq!(safe_vips_crop_internal(image, &mut cropped, 0, 0, unsafe { (*image).Xsize }, 10), 0);
    assert_eq!(safe_vips_avg_internal(cropped, &mut average), 0);
    unsafe { gobject_sys::g_object_unref(cropped.cast()) };
    assert_eq!(std::fs::read_dir("/proc/self/fd").expect("fd dir").count(), snapshot);

    assert_eq!(safe_vips_crop_internal(image, &mut cropped, 0, 20, unsafe { (*image).Xsize }, 10), 0);
    assert_eq!(safe_vips_avg_internal(cropped, &mut average), 0);
    unsafe { gobject_sys::g_object_unref(cropped.cast()) };
    assert_eq!(std::fs::read_dir("/proc/self/fd").expect("fd dir").count(), snapshot);

    unsafe {
        gobject_sys::g_object_unref(image.cast());
        gobject_sys::g_object_unref(source.cast());
    }
}

struct ThreadCounters {
    allocations: usize,
    work: usize,
    progress: usize,
}

unsafe extern "C" fn alloc_cb(
    state: *mut VipsThreadState,
    a: *mut c_void,
    stop: *mut glib_sys::gboolean,
) -> libc::c_int {
    let counters = unsafe { &mut *(a.cast::<ThreadCounters>()) };
    counters.allocations += 1;
    let thread_state = unsafe { &mut *state };
    assert!(!thread_state.reg.is_null());
    if counters.allocations >= 3 {
        unsafe {
            *stop = glib_sys::GTRUE;
        }
    }
    0
}

unsafe extern "C" fn work_cb(_state: *mut VipsThreadState, a: *mut c_void) -> libc::c_int {
    let counters = unsafe { &mut *(a.cast::<ThreadCounters>()) };
    counters.work += 1;
    0
}

unsafe extern "C" fn progress_cb(a: *mut c_void) -> libc::c_int {
    let counters = unsafe { &mut *(a.cast::<ThreadCounters>()) };
    counters.progress += 1;
    0
}

#[test]
fn threadpool_and_cache_controls_are_serial_and_stable() {
    let _guard = guard();
    init_vips();

    vips_concurrency_set(5);
    assert_eq!(vips_concurrency_get(), 5);
    vips_cache_set_max(23);
    vips_cache_set_max_files(7);
    vips_cache_set_max_mem(1234);
    assert_eq!(vips_cache_get_max(), 23);
    assert_eq!(vips_cache_get_max_files(), 7);
    assert_eq!(vips_cache_get_max_mem(), 1234);

    let image = vips_image_new_memory();
    vips_image_init_fields(
        image,
        1,
        1,
        1,
        VIPS_FORMAT_UCHAR,
        VIPS_CODING_NONE,
        VIPS_INTERPRETATION_B_W,
        1.0,
        1.0,
    );
    let mut counters = ThreadCounters {
        allocations: 0,
        work: 0,
        progress: 0,
    };
    assert_eq!(
        vips_threadpool_run(
            image,
            None,
            Some(alloc_cb),
            Some(work_cb),
            Some(progress_cb),
            (&mut counters as *mut ThreadCounters).cast(),
        ),
        0
    );
    assert_eq!(counters.allocations, 3);
    assert_eq!(counters.work, 2);
    assert_eq!(counters.progress, 3);

    unsafe {
        gobject_sys::g_object_unref(image.cast());
    }
}
