use std::ffi::{CStr, c_void};
use std::ptr;

use crate::abi::connection::{VipsSbuf, VipsSource, VIPS_SBUF_BUFFER_SIZE};
use crate::runtime::error::append_message_str;
use crate::runtime::object::{get_qdata_ptr, object_new, object_ref, object_unref, qdata_quark, set_qdata_box};

static SBUF_STATE_QUARK: &CStr = c"safe-vips-sbuf-state";

struct SbufState {
    source: *mut VipsSource,
}

impl Drop for SbufState {
    fn drop(&mut self) {
        unsafe {
            object_unref(self.source);
        }
    }
}

fn sbuf_quark() -> glib_sys::GQuark {
    qdata_quark(SBUF_STATE_QUARK)
}

unsafe fn refill(sbuf: *mut VipsSbuf) -> i64 {
    let Some(sbuf_ref) = (unsafe { sbuf.as_mut() }) else {
        return -1;
    };
    let bytes_read = crate::runtime::source::vips_source_read(
        sbuf_ref.source,
        sbuf_ref.input_buffer.as_mut_ptr().cast::<c_void>(),
        VIPS_SBUF_BUFFER_SIZE,
    );
    if bytes_read < 0 {
        return -1;
    }
    sbuf_ref.read_point = 0;
    sbuf_ref.chars_in_buffer = bytes_read as i32;
    sbuf_ref.input_buffer[bytes_read.max(0) as usize] = 0;
    bytes_read
}

#[no_mangle]
pub extern "C" fn vips_sbuf_new_from_source(source: *mut VipsSource) -> *mut VipsSbuf {
    if source.is_null() {
        return ptr::null_mut();
    }
    let sbuf = unsafe { object_new::<VipsSbuf>(crate::runtime::object::vips_sbuf_get_type()) };
    let Some(sbuf_ref) = (unsafe { sbuf.as_mut() }) else {
        return ptr::null_mut();
    };
    sbuf_ref.source = unsafe { object_ref(source) };
    sbuf_ref.chars_in_buffer = 0;
    sbuf_ref.read_point = 0;
    sbuf_ref.input_buffer[0] = 0;
    sbuf_ref.line[0] = 0;
    unsafe {
        set_qdata_box(
            sbuf.cast(),
            sbuf_quark(),
            SbufState {
                source: sbuf_ref.source,
            },
        );
    }
    sbuf
}

#[no_mangle]
pub extern "C" fn vips_sbuf_unbuffer(sbuf: *mut VipsSbuf) {
    let Some(sbuf_ref) = (unsafe { sbuf.as_mut() }) else {
        return;
    };
    let rewind = sbuf_ref.read_point - sbuf_ref.chars_in_buffer;
    let _ = crate::runtime::source::vips_source_seek(sbuf_ref.source, rewind as i64, libc::SEEK_CUR);
    sbuf_ref.read_point = 0;
    sbuf_ref.chars_in_buffer = 0;
    sbuf_ref.input_buffer[0] = 0;
}

#[no_mangle]
pub extern "C" fn vips_sbuf_getc(sbuf: *mut VipsSbuf) -> libc::c_int {
    let Some(sbuf_ref) = (unsafe { sbuf.as_mut() }) else {
        return -1;
    };
    if sbuf_ref.read_point == sbuf_ref.chars_in_buffer && unsafe { refill(sbuf) } <= 0 {
        return -1;
    }
    let value = sbuf_ref.input_buffer[sbuf_ref.read_point as usize] as libc::c_int;
    sbuf_ref.read_point += 1;
    value
}

#[no_mangle]
pub extern "C" fn vips_sbuf_ungetc(sbuf: *mut VipsSbuf) {
    if let Some(sbuf) = unsafe { sbuf.as_mut() } {
        if sbuf.read_point > 0 {
            sbuf.read_point -= 1;
        }
    }
}

#[no_mangle]
pub extern "C" fn vips_sbuf_require(sbuf: *mut VipsSbuf, require: libc::c_int) -> libc::c_int {
    let Some(sbuf_ref) = (unsafe { sbuf.as_mut() }) else {
        return -1;
    };
    if require < 0 || require as usize >= VIPS_SBUF_BUFFER_SIZE {
        append_message_str("vips_sbuf_require", "invalid lookahead");
        return -1;
    }
    if sbuf_ref.read_point + require <= sbuf_ref.chars_in_buffer {
        return 0;
    }
    let unread = (sbuf_ref.chars_in_buffer - sbuf_ref.read_point).max(0) as usize;
    if unread > 0 {
        unsafe {
            ptr::copy(
                sbuf_ref.input_buffer.as_ptr().add(sbuf_ref.read_point as usize),
                sbuf_ref.input_buffer.as_mut_ptr(),
                unread,
            );
        }
    }
    sbuf_ref.chars_in_buffer = unread as i32;
    sbuf_ref.read_point = 0;
    while sbuf_ref.chars_in_buffer < require {
        let to = unsafe { sbuf_ref.input_buffer.as_mut_ptr().add(sbuf_ref.chars_in_buffer as usize) };
        let space = VIPS_SBUF_BUFFER_SIZE - sbuf_ref.chars_in_buffer as usize;
        let bytes_read = crate::runtime::source::vips_source_read(
            sbuf_ref.source,
            to.cast::<c_void>(),
            space,
        );
        if bytes_read <= 0 {
            append_message_str("vips_sbuf_require", "end of file");
            return -1;
        }
        sbuf_ref.chars_in_buffer += bytes_read as i32;
        sbuf_ref.input_buffer[sbuf_ref.chars_in_buffer as usize] = 0;
    }
    0
}

#[no_mangle]
pub extern "C" fn vips_sbuf_get_line(sbuf: *mut VipsSbuf) -> *const libc::c_char {
    let Some(sbuf_ref) = (unsafe { sbuf.as_mut() }) else {
        return ptr::null();
    };
    let mut write_point = 0usize;
    let mut ch = -1;
    while write_point < VIPS_SBUF_BUFFER_SIZE {
        ch = vips_sbuf_getc(sbuf);
        if ch == -1 || ch == '\n' as libc::c_int {
            break;
        }
        sbuf_ref.line[write_point] = ch as u8;
        write_point += 1;
    }
    sbuf_ref.line[write_point] = 0;
    if ch == -1 && write_point == 0 {
        return ptr::null();
    }
    if write_point > 0 && sbuf_ref.line[write_point - 1] == b'\r' {
        sbuf_ref.line[write_point - 1] = 0;
    }
    if ch != '\n' as libc::c_int && write_point == VIPS_SBUF_BUFFER_SIZE {
        while {
            ch = vips_sbuf_getc(sbuf);
            ch != -1 && ch != '\n' as libc::c_int
        } {}
    }
    sbuf_ref.line.as_ptr().cast::<libc::c_char>()
}

#[no_mangle]
pub extern "C" fn vips_sbuf_get_line_copy(sbuf: *mut VipsSbuf) -> *mut libc::c_char {
    let line = vips_sbuf_get_line(sbuf);
    if line.is_null() {
        ptr::null_mut()
    } else {
        unsafe { glib_sys::g_strdup(line) }
    }
}

#[no_mangle]
pub extern "C" fn vips_sbuf_get_non_whitespace(sbuf: *mut VipsSbuf) -> *const libc::c_char {
    let Some(sbuf_ref) = (unsafe { sbuf.as_mut() }) else {
        return ptr::null();
    };
    let mut i = 0usize;
    let mut ch = vips_sbuf_getc(sbuf);
    while ch != -1 && !((ch as u8 as char).is_whitespace()) && i < VIPS_SBUF_BUFFER_SIZE {
        sbuf_ref.line[i] = ch as u8;
        i += 1;
        ch = vips_sbuf_getc(sbuf);
    }
    sbuf_ref.line[i] = 0;
    while ch != -1 && !((ch as u8 as char).is_whitespace()) {
        ch = vips_sbuf_getc(sbuf);
    }
    if ch != -1 && (ch as u8 as char).is_whitespace() {
        vips_sbuf_ungetc(sbuf);
    }
    sbuf_ref.line.as_ptr().cast::<libc::c_char>()
}

#[no_mangle]
pub extern "C" fn vips_sbuf_skip_whitespace(sbuf: *mut VipsSbuf) -> libc::c_int {
    loop {
        let ch = vips_sbuf_getc(sbuf);
        if ch == -1 {
            return -1;
        }
        if ch == '#' as libc::c_int {
            if vips_sbuf_get_line(sbuf).is_null() {
                return -1;
            }
            continue;
        }
        if !(ch as u8 as char).is_whitespace() {
            vips_sbuf_ungetc(sbuf);
            return 0;
        }
    }
}

#[allow(dead_code)]
unsafe fn _sbuf_state(sbuf: *mut VipsSbuf) -> Option<&'static mut SbufState> {
    unsafe { get_qdata_ptr::<SbufState>(sbuf.cast(), sbuf_quark()).as_mut() }
}
