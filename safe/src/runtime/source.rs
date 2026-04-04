use std::ffi::{c_void, CStr};
use std::mem::offset_of;
use std::os::raw::c_char;
use std::ptr;
use std::sync::OnceLock;

use gobject_sys::{GTypeClass, G_TYPE_INT, G_TYPE_INT64, G_TYPE_POINTER};

use crate::abi::connection::{VipsSource, VipsSourceCustom, VipsSourceCustomClass};
use crate::abi::r#type::{VipsArea, VipsBlob};
use crate::runtime::connection::{optional_cstr, pipe_read_limit};
use crate::runtime::error::append_message_str;
use crate::runtime::memory::vips_tracked_close;
use crate::runtime::object::{get_qdata_ptr, object_new, qdata_quark, set_qdata_box};
use crate::runtime::vips_native::read_all_from_path;

static SOURCE_STATE_QUARK: &CStr = c"safe-vips-source-state";
static SOURCE_CUSTOM_READ_SIGNAL: OnceLock<u32> = OnceLock::new();
static SOURCE_CUSTOM_SEEK_SIGNAL: OnceLock<u32> = OnceLock::new();

fn zero_gvalue() -> gobject_sys::GValue {
    unsafe { std::mem::zeroed() }
}

enum SourceKind {
    File {
        path: std::ffi::CString,
        bytes: Option<Vec<u8>>,
    },
    Memory {
        bytes: Vec<u8>,
    },
    Blob {
        blob: *mut VipsBlob,
    },
    Descriptor {
        fd: libc::c_int,
        close_on_drop: bool,
        bytes: Option<Vec<u8>>,
    },
    Custom,
}

struct SourceState {
    kind: SourceKind,
}

impl Drop for SourceState {
    fn drop(&mut self) {
        match &mut self.kind {
            SourceKind::Blob { blob } => {
                if !blob.is_null() {
                    crate::runtime::r#type::vips_area_unref((*blob).cast::<VipsArea>());
                    *blob = ptr::null_mut();
                }
            }
            SourceKind::Descriptor {
                fd, close_on_drop, ..
            } if *close_on_drop && *fd >= 0 => {
                vips_tracked_close(*fd);
                *fd = -1;
            }
            _ => {}
        }
    }
}

fn source_quark() -> glib_sys::GQuark {
    qdata_quark(SOURCE_STATE_QUARK)
}

unsafe fn source_state(source: *mut VipsSource) -> Option<&'static mut SourceState> {
    unsafe { get_qdata_ptr::<SourceState>(source.cast(), source_quark()).as_mut() }
}

fn load_fd(fd: libc::c_int) -> Result<Vec<u8>, ()> {
    if fd < 0 {
        append_message_str("vips_source_read", "invalid descriptor");
        return Err(());
    }

    let mut bytes = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = unsafe { libc::read(fd, buffer.as_mut_ptr().cast::<c_void>(), buffer.len()) };
        if read == 0 {
            break;
        }
        if read < 0 {
            append_message_str("vips_source_read", "read failed");
            return Err(());
        }
        bytes.extend_from_slice(&buffer[..read as usize]);
        if pipe_read_limit() >= 0 && bytes.len() as i64 > pipe_read_limit() {
            append_message_str("vips_source_read", "pipe read limit exceeded");
            return Err(());
        }
    }
    Ok(bytes)
}

unsafe fn ensure_loaded(source: *mut VipsSource) -> Result<(), ()> {
    let Some(source_state) = (unsafe { source_state(source) }) else {
        append_message_str("vips_source_read", "source state missing");
        return Err(());
    };
    let Some(source_ref) = (unsafe { source.as_mut() }) else {
        return Err(());
    };

    match &mut source_state.kind {
        SourceKind::File { path, bytes } => {
            if bytes.is_none() {
                *bytes = Some(read_all_from_path(path.as_c_str())?);
            }
            if let Some(bytes) = bytes.as_ref() {
                source_ref.data = bytes.as_ptr().cast::<c_void>();
                source_ref.length = bytes.len() as i64;
            }
        }
        SourceKind::Memory { bytes } => {
            source_ref.data = bytes.as_ptr().cast::<c_void>();
            source_ref.length = bytes.len() as i64;
        }
        SourceKind::Blob { blob } => {
            let mut length = 0usize;
            source_ref.data =
                crate::runtime::r#type::vips_blob_get(*blob, &mut length).cast::<c_void>();
            source_ref.length = length as i64;
            source_ref.blob = *blob;
        }
        SourceKind::Descriptor {
            fd,
            close_on_drop,
            bytes,
        } => {
            if bytes.is_none() {
                *bytes = Some(load_fd(*fd)?);
                if *close_on_drop && *fd >= 0 {
                    vips_tracked_close(*fd);
                    *fd = -1;
                    source_ref.parent_object.tracked_descriptor = -1;
                    source_ref.parent_object.descriptor = -1;
                }
            }
            if let Some(bytes) = bytes.as_ref() {
                source_ref.data = bytes.as_ptr().cast::<c_void>();
                source_ref.length = bytes.len() as i64;
            }
        }
        SourceKind::Custom => return Err(()),
    }

    Ok(())
}

unsafe fn emit_read(source: *mut VipsSource, data: *mut c_void, length: usize) -> i64 {
    let mut args = [zero_gvalue(), zero_gvalue(), zero_gvalue()];
    let mut result = zero_gvalue();
    unsafe {
        gobject_sys::g_value_init(&mut args[0], gobject_sys::g_object_get_type());
        gobject_sys::g_value_set_object(&mut args[0], source.cast());
        gobject_sys::g_value_init(&mut args[1], G_TYPE_POINTER);
        gobject_sys::g_value_set_pointer(&mut args[1], data);
        gobject_sys::g_value_init(&mut args[2], G_TYPE_INT64);
        gobject_sys::g_value_set_int64(&mut args[2], length as i64);
        gobject_sys::g_value_init(&mut result, G_TYPE_INT64);
        gobject_sys::g_value_set_int64(&mut result, 0);
        gobject_sys::g_signal_emitv(
            args.as_ptr(),
            *SOURCE_CUSTOM_READ_SIGNAL.get().expect("read signal"),
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

unsafe fn emit_seek(source: *mut VipsSource, offset: i64, whence: libc::c_int) -> i64 {
    let mut args = [zero_gvalue(), zero_gvalue(), zero_gvalue()];
    let mut result = zero_gvalue();
    unsafe {
        gobject_sys::g_value_init(&mut args[0], gobject_sys::g_object_get_type());
        gobject_sys::g_value_set_object(&mut args[0], source.cast());
        gobject_sys::g_value_init(&mut args[1], G_TYPE_INT64);
        gobject_sys::g_value_set_int64(&mut args[1], offset);
        gobject_sys::g_value_init(&mut args[2], G_TYPE_INT);
        gobject_sys::g_value_set_int(&mut args[2], whence);
        gobject_sys::g_value_init(&mut result, G_TYPE_INT64);
        gobject_sys::g_value_set_int64(&mut result, -1);
        gobject_sys::g_signal_emitv(
            args.as_ptr(),
            *SOURCE_CUSTOM_SEEK_SIGNAL.get().expect("seek signal"),
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

fn clipped_seek(current: i64, length: i64, offset: i64, whence: libc::c_int) -> i64 {
    let raw = match whence {
        libc::SEEK_SET => offset,
        libc::SEEK_CUR => current.saturating_add(offset),
        libc::SEEK_END => length.saturating_add(offset),
        _ => return -1,
    };
    raw.clamp(0, length.max(0))
}

#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn vips_source_custom_class_init(
    klass: glib_sys::gpointer,
    _class_data: glib_sys::gpointer,
) {
    let class = unsafe { &mut *(klass.cast::<VipsSourceCustomClass>()) };
    let type_ = unsafe { (*(klass.cast::<GTypeClass>())).g_type };

    let read_id = unsafe {
        gobject_sys::g_signal_new(
            c"read".as_ptr().cast::<c_char>(),
            type_,
            gobject_sys::G_SIGNAL_ACTION,
            offset_of!(VipsSourceCustomClass, read) as u32,
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
            offset_of!(VipsSourceCustomClass, seek) as u32,
            None,
            ptr::null_mut(),
            None,
            G_TYPE_INT64,
            2,
            G_TYPE_INT64,
            G_TYPE_INT,
        )
    };
    let _ = SOURCE_CUSTOM_READ_SIGNAL.set(read_id);
    let _ = SOURCE_CUSTOM_SEEK_SIGNAL.set(seek_id);

    class.read = Some(vips_source_custom_read_default);
    class.seek = Some(vips_source_custom_seek_default);
}

unsafe extern "C" fn vips_source_custom_read_default(
    _source_custom: *mut VipsSourceCustom,
    _data: *mut c_void,
    _length: i64,
) -> i64 {
    0
}

unsafe extern "C" fn vips_source_custom_seek_default(
    _source_custom: *mut VipsSourceCustom,
    _offset: i64,
    _whence: libc::c_int,
) -> i64 {
    -1
}

fn init_source_defaults(source: &mut VipsSource) {
    source.parent_object.descriptor = -1;
    source.parent_object.tracked_descriptor = -1;
    source.parent_object.close_descriptor = -1;
    source.decode = glib_sys::GFALSE;
    source.have_tested_seek = glib_sys::GFALSE;
    source.is_pipe = glib_sys::GFALSE;
    source.read_position = 0;
    source.length = -1;
    source.data = ptr::null();
    source.header_bytes = ptr::null_mut();
    source.sniff = ptr::null_mut();
    source.blob = ptr::null_mut();
    source.mmap_baseaddr = ptr::null_mut();
    source.mmap_length = 0;
}

#[no_mangle]
pub extern "C" fn vips_source_custom_new() -> *mut VipsSourceCustom {
    let source = unsafe {
        object_new::<VipsSourceCustom>(crate::runtime::object::vips_source_custom_get_type())
    };
    let Some(source_ref) = (unsafe { source.as_mut() }) else {
        return ptr::null_mut();
    };
    init_source_defaults(&mut source_ref.parent_object);
    unsafe {
        set_qdata_box(
            source.cast(),
            source_quark(),
            SourceState {
                kind: SourceKind::Custom,
            },
        );
    }
    source
}

#[no_mangle]
pub extern "C" fn vips_source_new_from_file(filename: *const c_char) -> *mut VipsSource {
    let Some(filename) = optional_cstr(filename) else {
        append_message_str("vips_source_new_from_file", "filename is null");
        return ptr::null_mut();
    };

    let source =
        unsafe { object_new::<VipsSource>(crate::runtime::object::vips_source_get_type()) };
    let Some(source_ref) = (unsafe { source.as_mut() }) else {
        return ptr::null_mut();
    };
    init_source_defaults(source_ref);
    unsafe {
        set_qdata_box(
            source.cast(),
            source_quark(),
            SourceState {
                kind: SourceKind::File {
                    path: filename.to_owned(),
                    bytes: None,
                },
            },
        );
    }
    if let Some(state) = unsafe { source_state(source) } {
        if let SourceKind::File { path, .. } = &state.kind {
            source_ref.parent_object.filename = path.as_ptr().cast_mut();
        }
    }
    source
}

#[no_mangle]
pub extern "C" fn vips_source_new_from_memory(data: *const c_void, size: usize) -> *mut VipsSource {
    let bytes = if data.is_null() || size == 0 {
        Vec::new()
    } else {
        let slice = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), size) };
        slice.to_vec()
    };
    let source =
        unsafe { object_new::<VipsSource>(crate::runtime::object::vips_source_get_type()) };
    let Some(source_ref) = (unsafe { source.as_mut() }) else {
        return ptr::null_mut();
    };
    init_source_defaults(source_ref);
    source_ref.length = bytes.len() as i64;
    source_ref.data = if bytes.is_empty() {
        ptr::null()
    } else {
        bytes.as_ptr().cast::<c_void>()
    };
    unsafe {
        set_qdata_box(
            source.cast(),
            source_quark(),
            SourceState {
                kind: SourceKind::Memory { bytes },
            },
        );
    }
    source
}

#[no_mangle]
pub extern "C" fn vips_source_new_from_blob(blob: *mut VipsBlob) -> *mut VipsSource {
    if blob.is_null() {
        append_message_str("vips_source_new_from_blob", "blob is null");
        return ptr::null_mut();
    }
    crate::runtime::r#type::vips_area_copy(blob.cast::<VipsArea>());
    let source =
        unsafe { object_new::<VipsSource>(crate::runtime::object::vips_source_get_type()) };
    let Some(source_ref) = (unsafe { source.as_mut() }) else {
        crate::runtime::r#type::vips_area_unref(blob.cast::<VipsArea>());
        return ptr::null_mut();
    };
    init_source_defaults(source_ref);
    unsafe {
        set_qdata_box(
            source.cast(),
            source_quark(),
            SourceState {
                kind: SourceKind::Blob { blob },
            },
        );
    }
    let _ = unsafe { ensure_loaded(source) };
    source
}

#[no_mangle]
pub extern "C" fn vips_source_new_from_descriptor(descriptor: libc::c_int) -> *mut VipsSource {
    if descriptor < 0 {
        append_message_str("vips_source_new_from_descriptor", "descriptor is invalid");
        return ptr::null_mut();
    }
    let source =
        unsafe { object_new::<VipsSource>(crate::runtime::object::vips_source_get_type()) };
    let Some(source_ref) = (unsafe { source.as_mut() }) else {
        return ptr::null_mut();
    };
    init_source_defaults(source_ref);
    let fd = unsafe { libc::dup(descriptor) };
    source_ref.parent_object.descriptor = fd;
    source_ref.parent_object.close_descriptor = fd;
    unsafe {
        set_qdata_box(
            source.cast(),
            source_quark(),
            SourceState {
                kind: SourceKind::Descriptor {
                    fd,
                    close_on_drop: true,
                    bytes: None,
                },
            },
        );
    }
    source
}

#[no_mangle]
pub extern "C" fn vips_source_new_from_target(
    _target: *mut crate::abi::connection::VipsTarget,
) -> *mut VipsSource {
    append_message_str("vips_source_new_from_target", "not implemented");
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn vips_source_new_from_options(_options: *const c_char) -> *mut VipsSource {
    append_message_str("vips_source_new_from_options", "not implemented");
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn vips_source_minimise(_source: *mut VipsSource) {}

#[no_mangle]
pub extern "C" fn vips_source_unminimise(source: *mut VipsSource) -> libc::c_int {
    if matches!(
        unsafe { source_state(source) }.map(|state| &state.kind),
        Some(SourceKind::Custom)
    ) {
        return 0;
    }
    match unsafe { ensure_loaded(source) } {
        Ok(()) => 0,
        Err(()) => -1,
    }
}

#[no_mangle]
pub extern "C" fn vips_source_decode(source: *mut VipsSource) -> libc::c_int {
    if let Some(source) = unsafe { source.as_mut() } {
        source.decode = glib_sys::GTRUE;
    }
    0
}

#[no_mangle]
pub extern "C" fn vips_source_read(
    source: *mut VipsSource,
    data: *mut c_void,
    length: usize,
) -> i64 {
    let Some(source_ref) = (unsafe { source.as_mut() }) else {
        return -1;
    };
    if data.is_null() && length > 0 {
        return -1;
    }

    if matches!(
        unsafe { source_state(source) }.map(|state| &state.kind),
        Some(SourceKind::Custom)
    ) {
        let bytes = unsafe { emit_read(source, data, length) };
        if bytes >= 0 {
            source_ref.read_position = source_ref.read_position.saturating_add(bytes);
        }
        return bytes;
    }

    if unsafe { ensure_loaded(source) }.is_err() {
        return -1;
    }

    let start = source_ref.read_position.max(0) as usize;
    let bytes = unsafe {
        std::slice::from_raw_parts(
            source_ref.data.cast::<u8>(),
            source_ref.length.max(0) as usize,
        )
    };
    let remaining = bytes.len().saturating_sub(start);
    let to_copy = remaining.min(length);
    if to_copy > 0 {
        unsafe {
            ptr::copy_nonoverlapping(bytes[start..].as_ptr(), data.cast::<u8>(), to_copy);
        }
    }
    source_ref.read_position = source_ref.read_position.saturating_add(to_copy as i64);
    to_copy as i64
}

#[no_mangle]
pub extern "C" fn vips_source_is_mappable(source: *mut VipsSource) -> glib_sys::gboolean {
    let custom = matches!(
        unsafe { source_state(source) }.map(|state| &state.kind),
        Some(SourceKind::Custom)
    );
    if custom {
        glib_sys::GFALSE
    } else {
        glib_sys::GTRUE
    }
}

#[no_mangle]
pub extern "C" fn vips_source_is_file(source: *mut VipsSource) -> glib_sys::gboolean {
    if unsafe { source.as_ref() }.is_some_and(|source| !source.parent_object.filename.is_null()) {
        glib_sys::GTRUE
    } else {
        glib_sys::GFALSE
    }
}

#[no_mangle]
pub extern "C" fn vips_source_map(source: *mut VipsSource, length: *mut usize) -> *const c_void {
    if unsafe { ensure_loaded(source) }.is_err() {
        return ptr::null();
    }
    let Some(source_ref) = (unsafe { source.as_ref() }) else {
        return ptr::null();
    };
    unsafe {
        if !length.is_null() {
            *length = source_ref.length.max(0) as usize;
        }
    }
    source_ref.data.cast::<c_void>()
}

#[no_mangle]
pub extern "C" fn vips_source_map_blob(source: *mut VipsSource) -> *mut VipsBlob {
    if unsafe { ensure_loaded(source) }.is_err() {
        return ptr::null_mut();
    }
    let Some(source_ref) = (unsafe { source.as_ref() }) else {
        return ptr::null_mut();
    };
    crate::runtime::r#type::vips_blob_copy(
        source_ref.data.cast::<c_void>(),
        source_ref.length.max(0) as usize,
    )
}

#[no_mangle]
pub extern "C" fn vips_source_seek(
    source: *mut VipsSource,
    offset: i64,
    whence: libc::c_int,
) -> i64 {
    let Some(source_ref) = (unsafe { source.as_mut() }) else {
        return -1;
    };

    if matches!(
        unsafe { source_state(source) }.map(|state| &state.kind),
        Some(SourceKind::Custom)
    ) {
        let new_pos = unsafe { emit_seek(source, offset, whence) };
        if new_pos >= 0 {
            source_ref.read_position = new_pos;
        }
        return new_pos;
    }

    if unsafe { ensure_loaded(source) }.is_err() {
        return -1;
    }
    let new_pos = clipped_seek(source_ref.read_position, source_ref.length, offset, whence);
    if new_pos < 0 {
        append_message_str("vips_source_seek", "bad whence");
        return -1;
    }
    source_ref.read_position = new_pos;
    new_pos
}

#[no_mangle]
pub extern "C" fn vips_source_rewind(source: *mut VipsSource) -> libc::c_int {
    if vips_source_seek(source, 0, libc::SEEK_SET) < 0 {
        -1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn vips_source_sniff_at_most(
    source: *mut VipsSource,
    data: *mut *mut u8,
    length: usize,
) -> i64 {
    let map = vips_source_map(source, ptr::null_mut());
    if map.is_null() {
        return -1;
    }
    let Some(source_ref) = (unsafe { source.as_ref() }) else {
        return -1;
    };
    let sniff = source_ref.length.max(0) as usize;
    let sniff = sniff.min(length);
    unsafe {
        if !data.is_null() {
            *data = map.cast::<u8>().cast_mut();
        }
    }
    sniff as i64
}

#[no_mangle]
pub extern "C" fn vips_source_sniff(source: *mut VipsSource, length: usize) -> *mut u8 {
    let mut data = ptr::null_mut();
    if vips_source_sniff_at_most(source, &mut data, length) < 0 {
        ptr::null_mut()
    } else {
        data
    }
}

#[no_mangle]
pub extern "C" fn vips_source_length(source: *mut VipsSource) -> i64 {
    if unsafe { ensure_loaded(source) }.is_ok() {
        unsafe { source.as_ref() }.map_or(-1, |source| source.length)
    } else {
        -1
    }
}
