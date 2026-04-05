use std::ffi::{CStr, CString};
use std::sync::{Mutex, OnceLock};

use libc::{c_char, c_int};

use crate::runtime;

const VERSION_STRING: &[u8] = b"8.15.1\0";

struct InitState {
    init_count: usize,
    shut_down: bool,
    argv0: Option<CString>,
    prgname: Option<CString>,
}

impl InitState {
    fn new() -> Self {
        Self {
            init_count: 0,
            shut_down: false,
            argv0: None,
            prgname: None,
        }
    }
}

fn state() -> &'static Mutex<InitState> {
    static STATE: OnceLock<Mutex<InitState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(InitState::new()))
}

fn basename(argv0: &CStr) -> CString {
    let bytes = argv0.to_bytes();
    let tail = bytes
        .rsplit(|byte| *byte == b'/' || *byte == b'\\')
        .next()
        .filter(|slice| !slice.is_empty())
        .unwrap_or(b"vips");
    CString::new(tail).expect("program name")
}

fn ensure_bootstrap_types() -> bool {
    runtime::object::ensure_types();
    let _ = runtime::object::vips_format_get_type();
    let _ = runtime::object::vips_sbuf_get_type();
    runtime::r#type::ensure_types();
    runtime::operation::ensure_generated_types()
}

#[no_mangle]
pub extern "C" fn vips_init(argv0: *const c_char) -> c_int {
    let needs_bootstrap = {
        let mut state = state().lock().expect("init state");
        if state.shut_down && state.init_count == 0 {
            runtime::error::append_message_str("vips_init", "library has already been shut down");
            return -1;
        }

        let needs_bootstrap = state.init_count == 0;
        state.init_count += 1;

        if needs_bootstrap {
            if !argv0.is_null() {
                let argv0 = unsafe { CStr::from_ptr(argv0) };
                let argv0 = argv0.to_owned();
                let prgname = basename(&argv0);
                unsafe {
                    glib_sys::g_set_prgname(prgname.as_ptr());
                }
                state.argv0 = Some(argv0);
                state.prgname = Some(prgname);
            }
        }

        needs_bootstrap
    };

    if needs_bootstrap {
        if !ensure_bootstrap_types() {
            let mut state = state().lock().expect("init state");
            if state.init_count > 0 {
                state.init_count -= 1;
            }
            if state.init_count == 0 {
                state.argv0 = None;
                state.prgname = None;
            }
            return -1;
        }
    }

    0
}

#[no_mangle]
pub extern "C" fn vips_shutdown() {
    let final_shutdown = {
        let mut state = state().lock().expect("init state");
        if state.init_count == 0 {
            return;
        }

        state.init_count -= 1;
        if state.init_count == 0 {
            state.shut_down = true;
            state.argv0 = None;
            state.prgname = None;
            true
        } else {
            false
        }
    };

    if final_shutdown {
        runtime::cache::vips_cache_drop_all();
        vips_thread_shutdown();
        runtime::threadpool::shutdown_runtime_state();
    }
}

#[no_mangle]
pub extern "C" fn vips_thread_shutdown() {
    runtime::threadpool::thread_shutdown();
}

#[no_mangle]
pub extern "C" fn vips_get_argv0() -> *const c_char {
    let state = state().lock().expect("init state");
    state
        .argv0
        .as_ref()
        .map_or(std::ptr::null(), |value| value.as_ptr())
}

#[no_mangle]
pub extern "C" fn vips_get_prgname() -> *const c_char {
    let prgname = unsafe { glib_sys::g_get_prgname() };
    if !prgname.is_null() {
        return prgname;
    }

    let state = state().lock().expect("init state");
    state
        .prgname
        .as_ref()
        .map_or(std::ptr::null(), |value| value.as_ptr())
}

#[no_mangle]
pub extern "C" fn vips_version_string() -> *const c_char {
    VERSION_STRING.as_ptr().cast()
}

#[no_mangle]
pub extern "C" fn vips_version(flag: c_int) -> c_int {
    match flag {
        0 => 8,
        1 => 15,
        2 => 1,
        3 => 59,
        4 => 1,
        5 => 17,
        _ => {
            runtime::error::append_message_str("vips_version", "flag not in [0, 5]");
            -1
        }
    }
}
