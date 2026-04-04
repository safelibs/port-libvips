use std::ffi::{c_void, CStr};
use std::mem::offset_of;
use std::os::raw::c_char;
use std::ptr;
use std::sync::OnceLock;

use gobject_sys::{GTypeClass, G_TYPE_INT, G_TYPE_INT64, G_TYPE_NONE, G_TYPE_POINTER};

use crate::abi::connection::{
    VipsTarget, VipsTargetCustom, VipsTargetCustomClass, VIPS_TARGET_BUFFER_SIZE,
};
use crate::runtime::connection::optional_cstr;
use crate::runtime::error::append_message_str;
use crate::runtime::memory::vips_tracked_close;
use crate::runtime::object::{get_qdata_ptr, object_new, qdata_quark, set_qdata_box};

static TARGET_STATE_QUARK: &CStr = c"safe-vips-target-state";
static TARGET_CUSTOM_WRITE_SIGNAL: OnceLock<u32> = OnceLock::new();
static TARGET_CUSTOM_READ_SIGNAL: OnceLock<u32> = OnceLock::new();
static TARGET_CUSTOM_SEEK_SIGNAL: OnceLock<u32> = OnceLock::new();
static TARGET_CUSTOM_END_SIGNAL: OnceLock<u32> = OnceLock::new();
static TARGET_CUSTOM_FINISH_SIGNAL: OnceLock<u32> = OnceLock::new();

fn zero_gvalue() -> gobject_sys::GValue {
    unsafe { std::mem::zeroed() }
}

enum TargetKind {
    Memory {
        bytes: Vec<u8>,
    },
    File {
        path: std::ffi::CString,
        fd: libc::c_int,
    },
    Descriptor {
        fd: libc::c_int,
        close_on_drop: bool,
    },
    Custom,
}

struct TargetState {
    kind: TargetKind,
}

impl Drop for TargetState {
    fn drop(&mut self) {
        match &mut self.kind {
            TargetKind::File { fd, .. }
            | TargetKind::Descriptor {
                fd,
                close_on_drop: true,
            } if *fd >= 0 => {
                vips_tracked_close(*fd);
                *fd = -1;
            }
            _ => {}
        }
    }
}

fn target_quark() -> glib_sys::GQuark {
    qdata_quark(TARGET_STATE_QUARK)
}

unsafe fn target_state(target: *mut VipsTarget) -> Option<&'static mut TargetState> {
    unsafe { get_qdata_ptr::<TargetState>(target.cast(), target_quark()).as_mut() }
}

fn init_target_defaults(target: &mut VipsTarget) {
    target.parent_object.descriptor = -1;
    target.parent_object.tracked_descriptor = -1;
    target.parent_object.close_descriptor = -1;
    target.parent_object.filename = ptr::null_mut();
    target.memory = glib_sys::GFALSE;
    target.ended = glib_sys::GFALSE;
    target.memory_buffer = ptr::null_mut();
    target.blob = ptr::null_mut();
    target.output_buffer.fill(0);
    target.write_point = 0;
    target.position = 0;
    target.delete_on_close = glib_sys::GFALSE;
    target.delete_on_close_filename = ptr::null_mut();
}

unsafe fn flush_buffer(target: *mut VipsTarget) -> Result<(), ()> {
    let Some(target_ref) = (unsafe { target.as_mut() }) else {
        return Err(());
    };
    if target_ref.write_point <= 0 {
        return Ok(());
    }
    let len = target_ref.write_point as usize;
    let bytes = target_ref.output_buffer[..len].to_vec();
    target_ref.write_point = 0;
    write_raw(target, &bytes)
}

fn write_raw(target: *mut VipsTarget, bytes: &[u8]) -> Result<(), ()> {
    let Some(state) = (unsafe { target_state(target) }) else {
        append_message_str("vips_target_write", "target state missing");
        return Err(());
    };
    let Some(target_ref) = (unsafe { target.as_mut() }) else {
        return Err(());
    };
    match &mut state.kind {
        TargetKind::Memory { bytes: buffer } => {
            let start = target_ref.position.max(0) as usize;
            let end = start.saturating_add(bytes.len());
            if end > buffer.len() {
                buffer.resize(end, 0);
            }
            buffer[start..end].copy_from_slice(bytes);
            target_ref.position = end as libc::off_t;
            Ok(())
        }
        TargetKind::File { fd, .. } | TargetKind::Descriptor { fd, .. } => {
            let wrote = unsafe { libc::write(*fd, bytes.as_ptr().cast::<c_void>(), bytes.len()) };
            if wrote < 0 || wrote as usize != bytes.len() {
                append_message_str("vips_target_write", "write failed");
                return Err(());
            }
            target_ref.position = target_ref
                .position
                .saturating_add(bytes.len() as libc::off_t);
            Ok(())
        }
        TargetKind::Custom => {
            let wrote = unsafe { emit_write(target, bytes.as_ptr().cast::<c_void>(), bytes.len()) };
            if wrote < 0 || wrote as usize != bytes.len() {
                append_message_str("vips_target_write", "custom write failed");
                return Err(());
            }
            target_ref.position = target_ref
                .position
                .saturating_add(bytes.len() as libc::off_t);
            Ok(())
        }
    }
}

unsafe fn emit_write(target: *mut VipsTarget, data: *const c_void, length: usize) -> i64 {
    let mut args = [zero_gvalue(), zero_gvalue(), zero_gvalue()];
    let mut result = zero_gvalue();
    unsafe {
        gobject_sys::g_value_init(&mut args[0], gobject_sys::g_object_get_type());
        gobject_sys::g_value_set_object(&mut args[0], target.cast());
        gobject_sys::g_value_init(&mut args[1], G_TYPE_POINTER);
        gobject_sys::g_value_set_pointer(&mut args[1], data.cast_mut());
        gobject_sys::g_value_init(&mut args[2], G_TYPE_INT64);
        gobject_sys::g_value_set_int64(&mut args[2], length as i64);
        gobject_sys::g_value_init(&mut result, G_TYPE_INT64);
        gobject_sys::g_value_set_int64(&mut result, 0);
        gobject_sys::g_signal_emitv(
            args.as_ptr(),
            *TARGET_CUSTOM_WRITE_SIGNAL.get().expect("write signal"),
            0,
            &mut result,
        );
        let out = gobject_sys::g_value_get_int64(&result);
        for value in &mut args {
            gobject_sys::g_value_unset(value);
        }
        gobject_sys::g_value_unset(&mut result);
        out
    }
}

unsafe fn emit_read(target: *mut VipsTarget, data: *mut c_void, length: usize) -> i64 {
    let mut args = [zero_gvalue(), zero_gvalue(), zero_gvalue()];
    let mut result = zero_gvalue();
    unsafe {
        gobject_sys::g_value_init(&mut args[0], gobject_sys::g_object_get_type());
        gobject_sys::g_value_set_object(&mut args[0], target.cast());
        gobject_sys::g_value_init(&mut args[1], G_TYPE_POINTER);
        gobject_sys::g_value_set_pointer(&mut args[1], data);
        gobject_sys::g_value_init(&mut args[2], G_TYPE_INT64);
        gobject_sys::g_value_set_int64(&mut args[2], length as i64);
        gobject_sys::g_value_init(&mut result, G_TYPE_INT64);
        gobject_sys::g_value_set_int64(&mut result, 0);
        gobject_sys::g_signal_emitv(
            args.as_ptr(),
            *TARGET_CUSTOM_READ_SIGNAL.get().expect("read signal"),
            0,
            &mut result,
        );
        let out = gobject_sys::g_value_get_int64(&result);
        for value in &mut args {
            gobject_sys::g_value_unset(value);
        }
        gobject_sys::g_value_unset(&mut result);
        out
    }
}

unsafe fn emit_seek(target: *mut VipsTarget, offset: i64, whence: libc::c_int) -> i64 {
    let mut args = [zero_gvalue(), zero_gvalue(), zero_gvalue()];
    let mut result = zero_gvalue();
    unsafe {
        gobject_sys::g_value_init(&mut args[0], gobject_sys::g_object_get_type());
        gobject_sys::g_value_set_object(&mut args[0], target.cast());
        gobject_sys::g_value_init(&mut args[1], G_TYPE_INT64);
        gobject_sys::g_value_set_int64(&mut args[1], offset);
        gobject_sys::g_value_init(&mut args[2], G_TYPE_INT);
        gobject_sys::g_value_set_int(&mut args[2], whence);
        gobject_sys::g_value_init(&mut result, G_TYPE_INT64);
        gobject_sys::g_value_set_int64(&mut result, -1);
        gobject_sys::g_signal_emitv(
            args.as_ptr(),
            *TARGET_CUSTOM_SEEK_SIGNAL.get().expect("seek signal"),
            0,
            &mut result,
        );
        let out = gobject_sys::g_value_get_int64(&result);
        for value in &mut args {
            gobject_sys::g_value_unset(value);
        }
        gobject_sys::g_value_unset(&mut result);
        out
    }
}

unsafe fn emit_finish(target: *mut VipsTarget) {
    let mut args = [zero_gvalue()];
    unsafe {
        gobject_sys::g_value_init(&mut args[0], gobject_sys::g_object_get_type());
        gobject_sys::g_value_set_object(&mut args[0], target.cast());
        gobject_sys::g_signal_emitv(
            args.as_ptr(),
            *TARGET_CUSTOM_FINISH_SIGNAL.get().expect("finish signal"),
            0,
            ptr::null_mut(),
        );
        gobject_sys::g_value_unset(&mut args[0]);
    }
}

unsafe fn emit_end(target: *mut VipsTarget) -> libc::c_int {
    let mut args = [zero_gvalue()];
    let mut result = zero_gvalue();
    unsafe {
        gobject_sys::g_value_init(&mut args[0], gobject_sys::g_object_get_type());
        gobject_sys::g_value_set_object(&mut args[0], target.cast());
        gobject_sys::g_value_init(&mut result, G_TYPE_INT);
        gobject_sys::g_value_set_int(&mut result, 0);
        gobject_sys::g_signal_emitv(
            args.as_ptr(),
            *TARGET_CUSTOM_END_SIGNAL.get().expect("end signal"),
            0,
            &mut result,
        );
        let out = gobject_sys::g_value_get_int(&result);
        gobject_sys::g_value_unset(&mut args[0]);
        gobject_sys::g_value_unset(&mut result);
        out
    }
}

#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn vips_target_custom_class_init(
    klass: glib_sys::gpointer,
    _class_data: glib_sys::gpointer,
) {
    let class = unsafe { &mut *(klass.cast::<VipsTargetCustomClass>()) };
    let type_ = unsafe { (*(klass.cast::<GTypeClass>())).g_type };

    let write_id = unsafe {
        gobject_sys::g_signal_new(
            c"write".as_ptr().cast::<c_char>(),
            type_,
            gobject_sys::G_SIGNAL_ACTION,
            offset_of!(VipsTargetCustomClass, write) as u32,
            None,
            ptr::null_mut(),
            None,
            G_TYPE_INT64,
            2,
            G_TYPE_POINTER,
            G_TYPE_INT64,
        )
    };
    let read_id = unsafe {
        gobject_sys::g_signal_new(
            c"read".as_ptr().cast::<c_char>(),
            type_,
            gobject_sys::G_SIGNAL_ACTION,
            offset_of!(VipsTargetCustomClass, read) as u32,
            None,
            ptr::null_mut(),
            None,
            G_TYPE_INT64,
            2,
            G_TYPE_POINTER,
            G_TYPE_INT64,
        )
    };
    let seek_id = unsafe {
        gobject_sys::g_signal_new(
            c"seek".as_ptr().cast::<c_char>(),
            type_,
            gobject_sys::G_SIGNAL_ACTION,
            offset_of!(VipsTargetCustomClass, seek) as u32,
            None,
            ptr::null_mut(),
            None,
            G_TYPE_INT64,
            2,
            G_TYPE_INT64,
            G_TYPE_INT,
        )
    };
    let end_id = unsafe {
        gobject_sys::g_signal_new(
            c"end".as_ptr().cast::<c_char>(),
            type_,
            gobject_sys::G_SIGNAL_ACTION,
            offset_of!(VipsTargetCustomClass, end) as u32,
            None,
            ptr::null_mut(),
            None,
            G_TYPE_INT,
            0,
        )
    };
    let finish_id = unsafe {
        gobject_sys::g_signal_new(
            c"finish".as_ptr().cast::<c_char>(),
            type_,
            gobject_sys::G_SIGNAL_ACTION,
            offset_of!(VipsTargetCustomClass, finish) as u32,
            None,
            ptr::null_mut(),
            None,
            G_TYPE_NONE,
            0,
        )
    };
    let _ = TARGET_CUSTOM_WRITE_SIGNAL.set(write_id);
    let _ = TARGET_CUSTOM_READ_SIGNAL.set(read_id);
    let _ = TARGET_CUSTOM_SEEK_SIGNAL.set(seek_id);
    let _ = TARGET_CUSTOM_END_SIGNAL.set(end_id);
    let _ = TARGET_CUSTOM_FINISH_SIGNAL.set(finish_id);

    class.write = Some(vips_target_custom_write_default);
    class.read = Some(vips_target_custom_read_default);
    class.seek = Some(vips_target_custom_seek_default);
    class.end = Some(vips_target_custom_end_default);
    class.finish = Some(vips_target_custom_finish_default);
}

unsafe extern "C" fn vips_target_custom_write_default(
    _target_custom: *mut VipsTargetCustom,
    _data: *const c_void,
    _length: i64,
) -> i64 {
    0
}

unsafe extern "C" fn vips_target_custom_read_default(
    _target_custom: *mut VipsTargetCustom,
    _data: *mut c_void,
    _length: i64,
) -> i64 {
    0
}

unsafe extern "C" fn vips_target_custom_seek_default(
    _target_custom: *mut VipsTargetCustom,
    _offset: i64,
    _whence: libc::c_int,
) -> i64 {
    -1
}

unsafe extern "C" fn vips_target_custom_end_default(
    _target_custom: *mut VipsTargetCustom,
) -> libc::c_int {
    0
}

unsafe extern "C" fn vips_target_custom_finish_default(_target_custom: *mut VipsTargetCustom) {}

#[no_mangle]
pub extern "C" fn vips_target_custom_new() -> *mut VipsTargetCustom {
    let target = unsafe {
        object_new::<VipsTargetCustom>(crate::runtime::object::vips_target_custom_get_type())
    };
    let Some(target_ref) = (unsafe { target.as_mut() }) else {
        return ptr::null_mut();
    };
    init_target_defaults(&mut target_ref.parent_object);
    unsafe {
        set_qdata_box(
            target.cast(),
            target_quark(),
            TargetState {
                kind: TargetKind::Custom,
            },
        );
    }
    target
}

#[no_mangle]
pub extern "C" fn vips_target_new_to_memory() -> *mut VipsTarget {
    let target =
        unsafe { object_new::<VipsTarget>(crate::runtime::object::vips_target_get_type()) };
    let Some(target_ref) = (unsafe { target.as_mut() }) else {
        return ptr::null_mut();
    };
    init_target_defaults(target_ref);
    target_ref.memory = glib_sys::GTRUE;
    unsafe {
        set_qdata_box(
            target.cast(),
            target_quark(),
            TargetState {
                kind: TargetKind::Memory { bytes: Vec::new() },
            },
        );
    }
    target
}

#[no_mangle]
pub extern "C" fn vips_target_new_to_file(filename: *const c_char) -> *mut VipsTarget {
    let Some(filename) = optional_cstr(filename) else {
        append_message_str("vips_target_new_to_file", "filename is null");
        return ptr::null_mut();
    };
    let fd = crate::runtime::memory::vips_tracked_open(
        filename.as_ptr(),
        libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC,
        0o644,
    );
    if fd < 0 {
        append_message_str("vips_target_new_to_file", "unable to open file");
        return ptr::null_mut();
    }

    let target =
        unsafe { object_new::<VipsTarget>(crate::runtime::object::vips_target_get_type()) };
    let Some(target_ref) = (unsafe { target.as_mut() }) else {
        vips_tracked_close(fd);
        return ptr::null_mut();
    };
    init_target_defaults(target_ref);
    target_ref.parent_object.descriptor = fd;
    target_ref.parent_object.tracked_descriptor = fd;
    unsafe {
        set_qdata_box(
            target.cast(),
            target_quark(),
            TargetState {
                kind: TargetKind::File {
                    path: filename.to_owned(),
                    fd,
                },
            },
        );
    }
    if let Some(state) = unsafe { target_state(target) } {
        if let TargetKind::File { path, .. } = &state.kind {
            target_ref.parent_object.filename = path.as_ptr().cast_mut();
        }
    }
    target
}

#[no_mangle]
pub extern "C" fn vips_target_new_to_descriptor(descriptor: libc::c_int) -> *mut VipsTarget {
    if descriptor < 0 {
        append_message_str("vips_target_new_to_descriptor", "descriptor is invalid");
        return ptr::null_mut();
    }
    let fd = unsafe { libc::dup(descriptor) };
    let target =
        unsafe { object_new::<VipsTarget>(crate::runtime::object::vips_target_get_type()) };
    let Some(target_ref) = (unsafe { target.as_mut() }) else {
        return ptr::null_mut();
    };
    init_target_defaults(target_ref);
    target_ref.parent_object.descriptor = fd;
    target_ref.parent_object.close_descriptor = fd;
    unsafe {
        set_qdata_box(
            target.cast(),
            target_quark(),
            TargetState {
                kind: TargetKind::Descriptor {
                    fd,
                    close_on_drop: true,
                },
            },
        );
    }
    target
}

#[no_mangle]
pub extern "C" fn vips_target_new_temp(_target: *mut VipsTarget) -> *mut VipsTarget {
    vips_target_new_to_memory()
}

#[no_mangle]
pub extern "C" fn vips_target_write(
    target: *mut VipsTarget,
    data: *const c_void,
    length: usize,
) -> libc::c_int {
    let Some(target_ref) = (unsafe { target.as_mut() }) else {
        return -1;
    };
    if target_ref.ended != glib_sys::GFALSE {
        append_message_str("vips_target_write", "target already ended");
        return -1;
    }
    if unsafe { flush_buffer(target) }.is_err() {
        return -1;
    }
    if data.is_null() && length > 0 {
        return -1;
    }
    let bytes = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), length) };
    if write_raw(target, bytes).is_err() {
        -1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn vips_target_read(
    target: *mut VipsTarget,
    buffer: *mut c_void,
    length: usize,
) -> i64 {
    let Some(state) = (unsafe { target_state(target) }) else {
        return -1;
    };
    let Some(target_ref) = (unsafe { target.as_mut() }) else {
        return -1;
    };
    match &mut state.kind {
        TargetKind::Memory { bytes } => {
            let start = target_ref.position.max(0) as usize;
            let to_copy = bytes.len().saturating_sub(start).min(length);
            if to_copy > 0 {
                unsafe {
                    ptr::copy_nonoverlapping(bytes[start..].as_ptr(), buffer.cast::<u8>(), to_copy);
                }
            }
            target_ref.position = target_ref.position.saturating_add(to_copy as libc::off_t);
            to_copy as i64
        }
        TargetKind::File { fd, .. } | TargetKind::Descriptor { fd, .. } => unsafe {
            libc::read(*fd, buffer, length) as i64
        },
        TargetKind::Custom => unsafe { emit_read(target, buffer, length) },
    }
}

#[no_mangle]
pub extern "C" fn vips_target_seek(
    target: *mut VipsTarget,
    offset: libc::off_t,
    whence: libc::c_int,
) -> libc::off_t {
    let Some(state) = (unsafe { target_state(target) }) else {
        return -1;
    };
    let Some(target_ref) = (unsafe { target.as_mut() }) else {
        return -1;
    };
    match &mut state.kind {
        TargetKind::Memory { bytes } => {
            let current = target_ref.position.max(0) as i64;
            let len = bytes.len() as i64;
            let next = match whence {
                libc::SEEK_SET => offset as i64,
                libc::SEEK_CUR => current.saturating_add(offset as i64),
                libc::SEEK_END => len.saturating_add(offset as i64),
                _ => return -1,
            };
            if next < 0 {
                return -1;
            }
            if next as usize > bytes.len() {
                bytes.resize(next as usize, 0);
            }
            target_ref.position = next as libc::off_t;
            target_ref.position
        }
        TargetKind::File { fd, .. } | TargetKind::Descriptor { fd, .. } => unsafe {
            libc::lseek(*fd, offset, whence)
        },
        TargetKind::Custom => unsafe { emit_seek(target, offset as i64, whence) as libc::off_t },
    }
}

#[no_mangle]
pub extern "C" fn vips_target_end(target: *mut VipsTarget) -> libc::c_int {
    let Some(target_ref) = (unsafe { target.as_mut() }) else {
        return -1;
    };
    if target_ref.ended != glib_sys::GFALSE {
        return 0;
    }
    if unsafe { flush_buffer(target) }.is_err() {
        return -1;
    }
    if matches!(
        unsafe { target_state(target) }.map(|state| &state.kind),
        Some(TargetKind::Custom)
    ) {
        unsafe {
            emit_finish(target);
            if emit_end(target) != 0 {
                return -1;
            }
        }
    }
    target_ref.ended = glib_sys::GTRUE;
    0
}

#[no_mangle]
pub extern "C" fn vips_target_finish(target: *mut VipsTarget) {
    let _ = vips_target_end(target);
}

#[no_mangle]
pub extern "C" fn vips_target_steal(target: *mut VipsTarget, length: *mut usize) -> *mut u8 {
    let _ = vips_target_end(target);
    let Some(state) = (unsafe { target_state(target) }) else {
        return ptr::null_mut();
    };
    match &mut state.kind {
        TargetKind::Memory { bytes } => {
            unsafe {
                if !length.is_null() {
                    *length = bytes.len();
                }
            }
            let out = if bytes.is_empty() {
                ptr::null_mut()
            } else {
                unsafe { glib_sys::g_malloc(bytes.len()) }.cast::<u8>()
            };
            if !out.is_null() {
                unsafe {
                    ptr::copy_nonoverlapping(bytes.as_ptr(), out, bytes.len());
                }
            }
            out
        }
        _ => ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn vips_target_steal_text(target: *mut VipsTarget) -> *mut c_char {
    let mut length = 0usize;
    let bytes = vips_target_steal(target, &mut length);
    let text = unsafe { glib_sys::g_malloc(length.saturating_add(1)) }.cast::<u8>();
    if !bytes.is_null() && !text.is_null() {
        unsafe {
            ptr::copy_nonoverlapping(bytes, text, length);
            *text.add(length) = 0;
            glib_sys::g_free(bytes.cast::<c_void>());
        }
    } else if !text.is_null() {
        unsafe {
            *text = 0;
        }
    }
    text.cast::<c_char>()
}

#[no_mangle]
pub extern "C" fn vips_target_putc(target: *mut VipsTarget, ch: libc::c_int) -> libc::c_int {
    let Some(target_ref) = (unsafe { target.as_mut() }) else {
        return -1;
    };
    if target_ref.write_point as usize >= VIPS_TARGET_BUFFER_SIZE {
        if unsafe { flush_buffer(target) }.is_err() {
            return -1;
        }
    }
    target_ref.output_buffer[target_ref.write_point as usize] = ch as u8;
    target_ref.write_point += 1;
    0
}

#[no_mangle]
pub extern "C" fn vips_target_writes(target: *mut VipsTarget, str_: *const c_char) -> libc::c_int {
    let Some(str_) = optional_cstr(str_) else {
        return -1;
    };
    vips_target_write(
        target,
        str_.as_ptr().cast::<c_void>(),
        str_.to_bytes().len(),
    )
}

#[no_mangle]
pub extern "C" fn vips_target_write_amp(
    target: *mut VipsTarget,
    str_: *const c_char,
) -> libc::c_int {
    let Some(str_) = optional_cstr(str_) else {
        return -1;
    };
    let escaped = str_
        .to_string_lossy()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let escaped = std::ffi::CString::new(escaped).expect("escaped");
    vips_target_writes(target, escaped.as_ptr())
}
