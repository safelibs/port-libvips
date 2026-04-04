use std::ffi::{CStr, CString};
use std::ptr;

use crate::abi::basic::{VipsBuf, VipsRect};
use crate::abi::image::VipsImage;
use crate::abi::region::VipsBuffer;
use crate::runtime::object::bool_to_gboolean;

const DEFAULT_BUF_SIZE: i32 = 1024;

fn clip_capacity(mx: i32) -> i32 {
    mx.max(4)
}

unsafe fn clear_contents(buf: *mut VipsBuf) {
    let Some(buf) = (unsafe { buf.as_mut() }) else {
        return;
    };
    buf.i = 0;
    buf.lasti = 0;
    buf.full = glib_sys::GFALSE;
    if !buf.base.is_null() {
        unsafe {
            *buf.base = 0;
        }
    }
}

unsafe fn ensure_dynamic(buf: *mut VipsBuf, mx: i32) -> bool {
    let Some(buf_ref) = (unsafe { buf.as_mut() }) else {
        return false;
    };
    let mx = clip_capacity(mx);
    if buf_ref.dynamic != glib_sys::GFALSE && !buf_ref.base.is_null() && buf_ref.mx == mx {
        unsafe { clear_contents(buf) };
        return true;
    }
    crate::runtime::buf::vips_buf_destroy(buf);
    let base = unsafe { glib_sys::g_malloc0(mx as usize) }.cast::<libc::c_char>();
    if base.is_null() {
        return false;
    }
    buf_ref.base = base;
    buf_ref.mx = mx;
    buf_ref.dynamic = glib_sys::GTRUE;
    unsafe { clear_contents(buf) };
    true
}

unsafe fn append_bytes(buf: *mut VipsBuf, bytes: &[u8]) -> bool {
    let Some(buf_ref) = (unsafe { buf.as_mut() }) else {
        return false;
    };
    if buf_ref.full != glib_sys::GFALSE {
        return false;
    }
    if bytes.is_empty() {
        return true;
    }
    if buf_ref.base.is_null() || buf_ref.mx < 4 {
        return false;
    }

    let avail = (buf_ref.mx - buf_ref.i - 4).max(0) as usize;
    let to_copy = bytes.len().min(avail);
    if to_copy > 0 {
        unsafe {
            ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                buf_ref.base.add(buf_ref.i as usize).cast::<u8>(),
                to_copy,
            );
        }
        buf_ref.i += to_copy as i32;
    }

    if to_copy < bytes.len() || buf_ref.i >= buf_ref.mx - 4 {
        buf_ref.full = glib_sys::GTRUE;
        unsafe {
            ptr::copy_nonoverlapping(b"...\0".as_ptr(), buf_ref.base.add((buf_ref.mx - 4) as usize).cast::<u8>(), 4);
        }
        buf_ref.i = buf_ref.mx - 1;
        return false;
    }

    unsafe {
        *buf_ref.base.add(buf_ref.i as usize) = 0;
    }
    true
}

fn quote_string(input: &str) -> String {
    let mut quoted = String::with_capacity(input.len() + 2);
    quoted.push('"');
    for ch in input.chars() {
        match ch {
            '\\' => quoted.push_str("\\\\"),
            '"' => quoted.push_str("\\\""),
            '\n' => quoted.push_str("\\n"),
            '\r' => quoted.push_str("\\r"),
            '\t' => quoted.push_str("\\t"),
            _ => quoted.push(ch),
        }
    }
    quoted.push('"');
    quoted
}

#[no_mangle]
pub extern "C" fn vips_buf_rewind(buf: *mut VipsBuf) {
    unsafe { clear_contents(buf) };
}

#[no_mangle]
pub extern "C" fn vips_buf_init(buf: *mut VipsBuf) {
    let Some(buf) = (unsafe { buf.as_mut() }) else {
        return;
    };
    buf.base = ptr::null_mut();
    buf.mx = 0;
    buf.i = 0;
    buf.full = glib_sys::GFALSE;
    buf.lasti = 0;
    buf.dynamic = glib_sys::GFALSE;
}

#[no_mangle]
pub extern "C" fn vips_buf_destroy(buf: *mut VipsBuf) {
    let Some(buf) = (unsafe { buf.as_mut() }) else {
        return;
    };
    if buf.dynamic != glib_sys::GFALSE && !buf.base.is_null() {
        unsafe {
            glib_sys::g_free(buf.base.cast());
        }
    }
    vips_buf_init(buf);
}

#[no_mangle]
pub extern "C" fn vips_buf_set_static(buf: *mut VipsBuf, base: *mut libc::c_char, mx: libc::c_int) {
    if base.is_null() {
        return;
    }
    vips_buf_destroy(buf);
    if let Some(buf) = unsafe { buf.as_mut() } {
        buf.base = base;
        buf.mx = clip_capacity(mx);
        buf.dynamic = glib_sys::GFALSE;
    }
    vips_buf_rewind(buf);
}

#[no_mangle]
pub extern "C" fn vips_buf_set_dynamic(buf: *mut VipsBuf, mx: libc::c_int) {
    let _ = unsafe { ensure_dynamic(buf, mx) };
}

#[no_mangle]
pub extern "C" fn vips_buf_init_static(buf: *mut VipsBuf, base: *mut libc::c_char, mx: libc::c_int) {
    vips_buf_init(buf);
    vips_buf_set_static(buf, base, mx);
}

#[no_mangle]
pub extern "C" fn vips_buf_init_dynamic(buf: *mut VipsBuf, mx: libc::c_int) {
    vips_buf_init(buf);
    vips_buf_set_dynamic(buf, mx);
}

#[no_mangle]
pub extern "C" fn vips_buf_appendns(
    buf: *mut VipsBuf,
    str_: *const libc::c_char,
    sz: libc::c_int,
) -> glib_sys::gboolean {
    if str_.is_null() {
        return glib_sys::GTRUE;
    }
    let bytes = unsafe { CStr::from_ptr(str_) }.to_bytes();
    let len = if sz >= 0 { bytes.len().min(sz as usize) } else { bytes.len() };
    bool_to_gboolean(unsafe { append_bytes(buf, &bytes[..len]) })
}

#[no_mangle]
pub extern "C" fn vips_buf_appends(buf: *mut VipsBuf, str_: *const libc::c_char) -> glib_sys::gboolean {
    vips_buf_appendns(buf, str_, -1)
}

#[no_mangle]
pub extern "C" fn vips_buf_appendc(buf: *mut VipsBuf, ch: libc::c_char) -> glib_sys::gboolean {
    bool_to_gboolean(unsafe { append_bytes(buf, &[ch as u8]) })
}

#[no_mangle]
pub extern "C" fn vips_buf_appendsc(
    buf: *mut VipsBuf,
    quote: glib_sys::gboolean,
    str_: *const libc::c_char,
) -> glib_sys::gboolean {
    if str_.is_null() {
        return glib_sys::GTRUE;
    }
    let text = unsafe { CStr::from_ptr(str_) }.to_string_lossy();
    let rendered = if quote == glib_sys::GFALSE {
        text.into_owned()
    } else {
        quote_string(&text)
    };
    bool_to_gboolean(unsafe { append_bytes(buf, rendered.as_bytes()) })
}

#[no_mangle]
pub extern "C" fn vips_buf_appendgv(
    buf: *mut VipsBuf,
    value: *mut gobject_sys::GValue,
) -> glib_sys::gboolean {
    if value.is_null() {
        return glib_sys::GTRUE;
    }
    let rendered = unsafe { gobject_sys::g_strdup_value_contents(value) };
    if rendered.is_null() {
        return glib_sys::GFALSE;
    }
    let result = unsafe { append_bytes(buf, CStr::from_ptr(rendered).to_bytes()) };
    unsafe {
        glib_sys::g_free(rendered.cast());
    }
    bool_to_gboolean(result)
}

#[no_mangle]
pub extern "C" fn vips_buf_append_size(buf: *mut VipsBuf, n: usize) -> glib_sys::gboolean {
    let units = ["bytes", "KB", "MB", "GB", "TB"];
    let mut size = n as f64;
    let mut unit = 0usize;
    while size > 1024.0 && unit + 1 < units.len() {
        size /= 1024.0;
        unit += 1;
    }
    let text = if unit == 0 {
        format!("{size:.0} {}", units[unit])
    } else {
        format!("{size:.2} {}", units[unit])
    };
    bool_to_gboolean(unsafe { append_bytes(buf, text.as_bytes()) })
}

#[no_mangle]
pub extern "C" fn vips_buf_removec(buf: *mut VipsBuf, ch: libc::c_char) -> glib_sys::gboolean {
    let Some(buf) = (unsafe { buf.as_mut() }) else {
        return glib_sys::GFALSE;
    };
    if buf.full != glib_sys::GFALSE || buf.i <= 0 || buf.base.is_null() {
        return glib_sys::GFALSE;
    }
    unsafe {
        if *buf.base.add((buf.i - 1) as usize) == ch {
            buf.i -= 1;
            *buf.base.add(buf.i as usize) = 0;
        }
    }
    glib_sys::GTRUE
}

#[no_mangle]
pub extern "C" fn vips_buf_change(
    buf: *mut VipsBuf,
    old: *const libc::c_char,
    new: *const libc::c_char,
) -> glib_sys::gboolean {
    let Some(buf_ref) = (unsafe { buf.as_mut() }) else {
        return glib_sys::GFALSE;
    };
    if buf_ref.full != glib_sys::GFALSE || buf_ref.base.is_null() || old.is_null() || new.is_null() {
        return glib_sys::GFALSE;
    }
    let current = vips_buf_all(buf);
    if current.is_null() {
        return glib_sys::GFALSE;
    }
    let current = unsafe { CStr::from_ptr(current) }.to_string_lossy().into_owned();
    let old = unsafe { CStr::from_ptr(old) }.to_string_lossy();
    let new = unsafe { CStr::from_ptr(new) }.to_string_lossy();
    if let Some(index) = current.rfind(old.as_ref()) {
        let mut updated = current;
        updated.replace_range(index..index + old.len(), &new);
        unsafe { clear_contents(buf) };
        return bool_to_gboolean(unsafe { append_bytes(buf, updated.as_bytes()) });
    }
    glib_sys::GTRUE
}

#[no_mangle]
pub extern "C" fn vips_buf_is_empty(buf: *mut VipsBuf) -> glib_sys::gboolean {
    bool_to_gboolean(unsafe { buf.as_ref() }.is_some_and(|buf| buf.i == 0))
}

#[no_mangle]
pub extern "C" fn vips_buf_is_full(buf: *mut VipsBuf) -> glib_sys::gboolean {
    bool_to_gboolean(unsafe { buf.as_ref() }.is_some_and(|buf| buf.full != glib_sys::GFALSE))
}

#[no_mangle]
pub extern "C" fn vips_buf_all(buf: *mut VipsBuf) -> *const libc::c_char {
    let buf_ptr = buf;
    let Some(buf) = (unsafe { buf_ptr.as_mut() }) else {
        return ptr::null();
    };
    if buf.base.is_null() {
        let _ = unsafe { ensure_dynamic(buf_ptr, DEFAULT_BUF_SIZE) };
    }
    if !buf.base.is_null() {
        unsafe {
            *buf.base.add(buf.i.max(0) as usize) = 0;
        }
        return buf.base.cast_const();
    }
    ptr::null()
}

#[no_mangle]
pub extern "C" fn vips_buf_firstline(buf: *mut VipsBuf) -> *const libc::c_char {
    let base = vips_buf_all(buf);
    if base.is_null() {
        return ptr::null();
    }
    let Some(newline) = (unsafe { libc::strchr(base, '\n' as i32).as_mut() }) else {
        return base;
    };
    *newline = 0;
    base
}

#[no_mangle]
pub extern "C" fn vips_buf_appendg(buf: *mut VipsBuf, g: f64) -> glib_sys::gboolean {
    bool_to_gboolean(unsafe { append_bytes(buf, format!("{g}").as_bytes()) })
}

#[no_mangle]
pub extern "C" fn vips_buf_appendd(buf: *mut VipsBuf, d: libc::c_int) -> glib_sys::gboolean {
    let rendered = if d < 0 {
        format!(" ({d})")
    } else {
        format!(" {d}")
    };
    bool_to_gboolean(unsafe { append_bytes(buf, rendered.as_bytes()) })
}

#[no_mangle]
pub extern "C" fn vips_buf_len(buf: *mut VipsBuf) -> libc::c_int {
    unsafe { buf.as_ref() }.map_or(0, |buf| buf.i)
}

#[allow(dead_code)]
fn _cstring_from_lossy(text: &str) -> CString {
    CString::new(text).unwrap_or_else(|_| CString::new("").expect("empty cstring"))
}

fn buffer_size(image: &VipsImage, area: &VipsRect) -> usize {
    area.width.max(0) as usize
        * area.height.max(0) as usize
        * crate::runtime::image::bytes_per_pixel(image)
}

#[no_mangle]
pub extern "C" fn vips_buffer_new(im: *mut VipsImage, area: *const VipsRect) -> *mut VipsBuffer {
    let (Some(image), Some(area)) = (unsafe { im.as_ref() }, unsafe { area.as_ref() }) else {
        return ptr::null_mut();
    };
    let size = buffer_size(image, area);
    let buf = if size == 0 {
        ptr::null_mut()
    } else {
        crate::runtime::memory::vips_tracked_aligned_alloc(size, 16).cast::<u8>()
    };
    let buffer = Box::new(VipsBuffer {
        ref_count: 1,
        im: unsafe { crate::runtime::object::object_ref(im) },
        area: *area,
        done: glib_sys::GFALSE,
        cache: ptr::null_mut(),
        buf,
        bsize: size,
    });
    Box::into_raw(buffer)
}

#[no_mangle]
pub extern "C" fn vips_buffer_ref(im: *mut VipsImage, area: *const VipsRect) -> *mut VipsBuffer {
    vips_buffer_new(im, area)
}

#[no_mangle]
pub extern "C" fn vips_buffer_unref_ref(
    buffer: *mut VipsBuffer,
    im: *mut VipsImage,
    area: *const VipsRect,
) -> *mut VipsBuffer {
    vips_buffer_unref(buffer);
    vips_buffer_ref(im, area)
}

#[no_mangle]
pub extern "C" fn vips_buffer_done(buffer: *mut VipsBuffer) {
    if let Some(buffer) = unsafe { buffer.as_mut() } {
        buffer.done = glib_sys::GTRUE;
    }
}

#[no_mangle]
pub extern "C" fn vips_buffer_undone(buffer: *mut VipsBuffer) {
    if let Some(buffer) = unsafe { buffer.as_mut() } {
        buffer.done = glib_sys::GFALSE;
    }
}

#[no_mangle]
pub extern "C" fn vips_buffer_unref(buffer: *mut VipsBuffer) {
    if buffer.is_null() {
        return;
    }
    let buffer_ref = unsafe { &mut *buffer };
    buffer_ref.ref_count -= 1;
    if buffer_ref.ref_count > 0 {
        return;
    }
    unsafe {
        crate::runtime::memory::vips_tracked_aligned_free(buffer_ref.buf.cast());
        crate::runtime::object::object_unref(buffer_ref.im);
        drop(Box::from_raw(buffer));
    }
}

#[no_mangle]
pub extern "C" fn vips_buffer_print(buffer: *mut VipsBuffer) {
    if let Some(buffer) = unsafe { buffer.as_ref() } {
        eprintln!(
            "VipsBuffer({:p}) area={}x{}+{}+{} size={}",
            buffer,
            buffer.area.width,
            buffer.area.height,
            buffer.area.left,
            buffer.area.top,
            buffer.bsize
        );
    }
}

#[no_mangle]
pub extern "C" fn vips_buffer_dump_all() {}
