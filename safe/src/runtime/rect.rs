use std::ptr;

use crate::abi::basic::VipsRect;
use crate::runtime::object::bool_to_gboolean;

#[no_mangle]
pub extern "C" fn vips_rect_isempty(r: *const VipsRect) -> glib_sys::gboolean {
    if r.is_null() {
        return bool_to_gboolean(true);
    }

    let r = unsafe { &*r };
    bool_to_gboolean(r.width <= 0 || r.height <= 0)
}

#[no_mangle]
pub extern "C" fn vips_rect_includespoint(
    r: *const VipsRect,
    x: libc::c_int,
    y: libc::c_int,
) -> glib_sys::gboolean {
    if r.is_null() {
        return bool_to_gboolean(false);
    }

    let r = unsafe { &*r };
    bool_to_gboolean(
        x >= r.left
            && y >= r.top
            && x < r.left.saturating_add(r.width)
            && y < r.top.saturating_add(r.height),
    )
}

#[no_mangle]
pub extern "C" fn vips_rect_includesrect(
    r1: *const VipsRect,
    r2: *const VipsRect,
) -> glib_sys::gboolean {
    if r1.is_null() || r2.is_null() {
        return bool_to_gboolean(false);
    }

    let r1 = unsafe { &*r1 };
    let r2 = unsafe { &*r2 };
    bool_to_gboolean(
        r2.left >= r1.left
            && r2.top >= r1.top
            && r2.left.saturating_add(r2.width) <= r1.left.saturating_add(r1.width)
            && r2.top.saturating_add(r2.height) <= r1.top.saturating_add(r1.height),
    )
}

#[no_mangle]
pub extern "C" fn vips_rect_equalsrect(
    r1: *const VipsRect,
    r2: *const VipsRect,
) -> glib_sys::gboolean {
    if r1.is_null() || r2.is_null() {
        return bool_to_gboolean(false);
    }

    let r1 = unsafe { &*r1 };
    let r2 = unsafe { &*r2 };
    bool_to_gboolean(
        r1.left == r2.left && r1.top == r2.top && r1.width == r2.width && r1.height == r2.height,
    )
}

#[no_mangle]
pub extern "C" fn vips_rect_overlapsrect(
    r1: *const VipsRect,
    r2: *const VipsRect,
) -> glib_sys::gboolean {
    if r1.is_null() || r2.is_null() {
        return bool_to_gboolean(false);
    }

    let mut out = VipsRect {
        left: 0,
        top: 0,
        width: 0,
        height: 0,
    };
    vips_rect_intersectrect(r1, r2, &mut out);
    bool_to_gboolean(out.width > 0 && out.height > 0)
}

#[no_mangle]
pub extern "C" fn vips_rect_marginadjust(r: *mut VipsRect, n: libc::c_int) {
    if let Some(r) = unsafe { r.as_mut() } {
        r.left = r.left.saturating_sub(n);
        r.top = r.top.saturating_sub(n);
        r.width = r.width.saturating_add(n.saturating_mul(2));
        r.height = r.height.saturating_add(n.saturating_mul(2));
    }
}

#[no_mangle]
pub extern "C" fn vips_rect_intersectrect(
    r1: *const VipsRect,
    r2: *const VipsRect,
    out: *mut VipsRect,
) {
    let Some(out) = (unsafe { out.as_mut() }) else {
        return;
    };
    if r1.is_null() || r2.is_null() {
        *out = VipsRect {
            left: 0,
            top: 0,
            width: 0,
            height: 0,
        };
        return;
    }

    let r1 = unsafe { &*r1 };
    let r2 = unsafe { &*r2 };
    let left = r1.left.max(r2.left);
    let top = r1.top.max(r2.top);
    let right = r1
        .left
        .saturating_add(r1.width)
        .min(r2.left.saturating_add(r2.width));
    let bottom = r1
        .top
        .saturating_add(r1.height)
        .min(r2.top.saturating_add(r2.height));

    *out = VipsRect {
        left,
        top,
        width: (right - left).max(0),
        height: (bottom - top).max(0),
    };
}

#[no_mangle]
pub extern "C" fn vips_rect_unionrect(
    r1: *const VipsRect,
    r2: *const VipsRect,
    out: *mut VipsRect,
) {
    let Some(out) = (unsafe { out.as_mut() }) else {
        return;
    };
    match (unsafe { r1.as_ref() }, unsafe { r2.as_ref() }) {
        (Some(r1), Some(r2)) => {
            let left = r1.left.min(r2.left);
            let top = r1.top.min(r2.top);
            let right = r1
                .left
                .saturating_add(r1.width)
                .max(r2.left.saturating_add(r2.width));
            let bottom = r1
                .top
                .saturating_add(r1.height)
                .max(r2.top.saturating_add(r2.height));
            *out = VipsRect {
                left,
                top,
                width: right - left,
                height: bottom - top,
            };
        }
        (Some(r1), None) => *out = *r1,
        (None, Some(r2)) => *out = *r2,
        (None, None) => {
            *out = VipsRect {
                left: 0,
                top: 0,
                width: 0,
                height: 0,
            };
        }
    }
}

#[no_mangle]
pub extern "C" fn vips_rect_dup(r: *const VipsRect) -> *mut VipsRect {
    let Some(r) = (unsafe { r.as_ref() }) else {
        return ptr::null_mut();
    };
    let out = unsafe { glib_sys::g_malloc(std::mem::size_of::<VipsRect>()) }.cast::<VipsRect>();
    if let Some(out) = unsafe { out.as_mut() } {
        *out = *r;
    }
    out
}

#[no_mangle]
pub extern "C" fn vips_rect_normalise(r: *mut VipsRect) {
    if let Some(r) = unsafe { r.as_mut() } {
        if r.width < 0 {
            r.left += r.width;
            r.width = -r.width;
        }
        if r.height < 0 {
            r.top += r.height;
            r.height = -r.height;
        }
    }
}
