use std::ffi::CStr;
use std::ptr;

use crate::abi::basic::VipsDbuf;
use crate::runtime::error::append_message_str;
use crate::runtime::object::bool_to_gboolean;

fn grow_size(size: usize) -> usize {
    3usize.saturating_mul(16usize.saturating_add(size)) / 2
}

unsafe fn minimum_size(dbuf: *mut VipsDbuf, size: usize) -> bool {
    let Some(dbuf) = (unsafe { dbuf.as_mut() }) else {
        return false;
    };
    if size <= dbuf.allocated_size {
        return true;
    }
    let new_size = grow_size(size).max(size).max(1);
    let new_data = unsafe {
        if dbuf.data.is_null() {
            glib_sys::g_try_malloc0(new_size)
        } else {
            glib_sys::g_try_realloc(dbuf.data.cast(), new_size)
        }
    }
    .cast::<u8>();
    if new_data.is_null() {
        append_message_str("VipsDbuf", "out of memory");
        return false;
    }
    if new_size > dbuf.allocated_size {
        unsafe {
            ptr::write_bytes(new_data.add(dbuf.allocated_size), 0, new_size - dbuf.allocated_size);
        }
    }
    dbuf.data = new_data;
    dbuf.allocated_size = new_size;
    true
}

#[no_mangle]
pub extern "C" fn vips_dbuf_init(dbuf: *mut VipsDbuf) {
    let Some(dbuf) = (unsafe { dbuf.as_mut() }) else {
        return;
    };
    dbuf.data = ptr::null_mut();
    dbuf.allocated_size = 0;
    dbuf.data_size = 0;
    dbuf.write_point = 0;
}

#[no_mangle]
pub extern "C" fn vips_dbuf_minimum_size(dbuf: *mut VipsDbuf, size: usize) -> glib_sys::gboolean {
    bool_to_gboolean(unsafe { minimum_size(dbuf, size) })
}

#[no_mangle]
pub extern "C" fn vips_dbuf_allocate(dbuf: *mut VipsDbuf, size: usize) -> glib_sys::gboolean {
    let required = unsafe { dbuf.as_ref() }
        .map(|dbuf| dbuf.write_point.saturating_add(size))
        .unwrap_or(size);
    bool_to_gboolean(unsafe { minimum_size(dbuf, required) })
}

#[no_mangle]
pub extern "C" fn vips_dbuf_read(dbuf: *mut VipsDbuf, data: *mut u8, size: usize) -> usize {
    let Some(dbuf) = (unsafe { dbuf.as_mut() }) else {
        return 0;
    };
    if data.is_null() && size > 0 {
        return 0;
    }
    let available = dbuf.data_size.saturating_sub(dbuf.write_point);
    let copied = size.min(available);
    if copied > 0 {
        unsafe {
            ptr::copy_nonoverlapping(dbuf.data.add(dbuf.write_point), data, copied);
        }
    }
    dbuf.write_point = dbuf.write_point.saturating_add(copied);
    copied
}

#[no_mangle]
pub extern "C" fn vips_dbuf_get_write(dbuf: *mut VipsDbuf, size: *mut usize) -> *mut u8 {
    let Some(dbuf) = (unsafe { dbuf.as_mut() }) else {
        return ptr::null_mut();
    };
    let available = dbuf.allocated_size.saturating_sub(dbuf.write_point);
    unsafe {
        if !size.is_null() {
            *size = available;
        }
    }
    if available == 0 || dbuf.data.is_null() {
        return ptr::null_mut();
    }
    let out = unsafe { dbuf.data.add(dbuf.write_point) };
    unsafe {
        ptr::write_bytes(out, 0, available);
    }
    dbuf.write_point = dbuf.allocated_size;
    dbuf.data_size = dbuf.allocated_size;
    out
}

#[no_mangle]
pub extern "C" fn vips_dbuf_write(
    dbuf: *mut VipsDbuf,
    data: *const u8,
    size: usize,
) -> glib_sys::gboolean {
    if data.is_null() && size > 0 {
        return glib_sys::GFALSE;
    }
    let required = unsafe { dbuf.as_ref() }
        .map_or(size, |dbuf| dbuf.write_point.saturating_add(size));
    if unsafe { minimum_size(dbuf, required) } {
        let Some(dbuf) = (unsafe { dbuf.as_mut() }) else {
            return glib_sys::GFALSE;
        };
        if size > 0 {
            unsafe {
                ptr::copy_nonoverlapping(data, dbuf.data.add(dbuf.write_point), size);
            }
        }
        dbuf.write_point = dbuf.write_point.saturating_add(size);
        dbuf.data_size = dbuf.data_size.max(dbuf.write_point);
        glib_sys::GTRUE
    } else {
        glib_sys::GFALSE
    }
}

#[no_mangle]
pub extern "C" fn vips_dbuf_write_amp(dbuf: *mut VipsDbuf, str_: *const libc::c_char) -> glib_sys::gboolean {
    if str_.is_null() {
        return glib_sys::GTRUE;
    }
    let mut rendered = Vec::new();
    for byte in unsafe { CStr::from_ptr(str_) }.to_bytes() {
        match *byte {
            b'<' => rendered.extend_from_slice(b"&lt;"),
            b'>' => rendered.extend_from_slice(b"&gt;"),
            b'&' => rendered.extend_from_slice(b"&amp;"),
            0..=8 | 11 | 12 | 14..=31 => {
                rendered.extend_from_slice(format!("&#x{:04x};", 0x2400 + *byte as u32).as_bytes())
            }
            value => rendered.push(value),
        }
    }
    vips_dbuf_write(dbuf, rendered.as_ptr(), rendered.len())
}

#[no_mangle]
pub extern "C" fn vips_dbuf_reset(dbuf: *mut VipsDbuf) {
    if let Some(dbuf) = unsafe { dbuf.as_mut() } {
        dbuf.write_point = 0;
        dbuf.data_size = 0;
    }
}

#[no_mangle]
pub extern "C" fn vips_dbuf_destroy(dbuf: *mut VipsDbuf) {
    let Some(dbuf) = (unsafe { dbuf.as_mut() }) else {
        return;
    };
    unsafe {
        glib_sys::g_free(dbuf.data.cast());
    }
    dbuf.data = ptr::null_mut();
    dbuf.allocated_size = 0;
    dbuf.data_size = 0;
    dbuf.write_point = 0;
}

#[no_mangle]
pub extern "C" fn vips_dbuf_seek(
    dbuf: *mut VipsDbuf,
    offset: libc::off_t,
    whence: libc::c_int,
) -> glib_sys::gboolean {
    let Some(dbuf_ref) = (unsafe { dbuf.as_ref() }) else {
        return glib_sys::GFALSE;
    };
    let next = match whence {
        libc::SEEK_SET => offset as i64,
        libc::SEEK_CUR => dbuf_ref.write_point as i64 + offset as i64,
        libc::SEEK_END => dbuf_ref.data_size as i64 + offset as i64,
        _ => {
            append_message_str("VipsDbuf", "invalid seek mode");
            return glib_sys::GFALSE;
        }
    };
    if next < 0 {
        append_message_str("VipsDbuf", "negative seek");
        return glib_sys::GFALSE;
    }
    let next = next as usize;
    if !unsafe { minimum_size(dbuf, next) } {
        return glib_sys::GFALSE;
    }
    if let Some(dbuf) = unsafe { dbuf.as_mut() } {
        if next > dbuf.data_size {
            unsafe {
                ptr::write_bytes(dbuf.data.add(dbuf.data_size), 0, next - dbuf.data_size);
            }
            dbuf.data_size = next;
        }
        dbuf.write_point = next;
        glib_sys::GTRUE
    } else {
        glib_sys::GFALSE
    }
}

#[no_mangle]
pub extern "C" fn vips_dbuf_truncate(dbuf: *mut VipsDbuf) {
    if let Some(dbuf) = unsafe { dbuf.as_mut() } {
        dbuf.data_size = dbuf.write_point;
    }
}

#[no_mangle]
pub extern "C" fn vips_dbuf_tell(dbuf: *mut VipsDbuf) -> libc::off_t {
    unsafe { dbuf.as_ref() }.map_or(0, |dbuf| dbuf.write_point as libc::off_t)
}

unsafe fn null_terminate(dbuf: *mut VipsDbuf) -> bool {
    let Some(dbuf_ref) = (unsafe { dbuf.as_ref() }) else {
        return false;
    };
    let size = dbuf_ref.data_size.saturating_add(1);
    if !unsafe { minimum_size(dbuf, size) } {
        return false;
    }
    if let Some(dbuf) = unsafe { dbuf.as_mut() } {
        unsafe {
            *dbuf.data.add(dbuf.data_size) = 0;
        }
        true
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn vips_dbuf_string(dbuf: *mut VipsDbuf, size: *mut usize) -> *mut u8 {
    if !unsafe { null_terminate(dbuf) } {
        return ptr::null_mut();
    }
    let Some(dbuf) = (unsafe { dbuf.as_mut() }) else {
        return ptr::null_mut();
    };
    unsafe {
        if !size.is_null() {
            *size = dbuf.data_size;
        }
    }
    dbuf.data
}

#[no_mangle]
pub extern "C" fn vips_dbuf_steal(dbuf: *mut VipsDbuf, size: *mut usize) -> *mut u8 {
    if !unsafe { null_terminate(dbuf) } {
        return ptr::null_mut();
    }
    let Some(dbuf) = (unsafe { dbuf.as_mut() }) else {
        return ptr::null_mut();
    };
    let data = dbuf.data;
    unsafe {
        if !size.is_null() {
            *size = dbuf.data_size;
        }
    }
    dbuf.data = ptr::null_mut();
    dbuf.allocated_size = 0;
    dbuf.data_size = 0;
    dbuf.write_point = 0;
    data
}
