use std::ffi::{CStr, c_void};
use std::ptr;

use crate::abi::basic::{VipsPel, VipsRect};
use crate::abi::image::{VipsImage, VIPS_IMAGE_OPENOUT, VIPS_IMAGE_PARTIAL};
use crate::abi::region::{RegionType, VipsRegion, VipsRegionShrink, VIPS_REGION_BUFFER, VIPS_REGION_NONE, VIPS_REGION_OTHER_IMAGE, VIPS_REGION_OTHER_REGION, VIPS_REGION_SHRINK_NEAREST};
use crate::runtime::error::append_message_str;
use crate::runtime::image::{bytes_per_pixel, image_size, image_state, line_size};
use crate::runtime::object::{get_qdata_ptr, object_new, object_ref, object_unref, qdata_quark, set_qdata_box};

static REGION_STATE_QUARK: &CStr = c"safe-vips-region-state";

struct RegionState {
    image: *mut VipsImage,
    owned: Vec<u8>,
    seq: *mut c_void,
}

impl Drop for RegionState {
    fn drop(&mut self) {
        unsafe {
            if !self.seq.is_null() {
                if let Some(image) = self.image.as_ref() {
                    if let Some(stop_fn) = image.stop_fn {
                        let _ = stop_fn(self.seq, image.client1, image.client2);
                    }
                }
            }
            object_unref(self.image);
        }
    }
}

fn region_quark() -> glib_sys::GQuark {
    qdata_quark(REGION_STATE_QUARK)
}

unsafe fn region_state(region: *mut VipsRegion) -> Option<&'static mut RegionState> {
    unsafe { get_qdata_ptr::<RegionState>(region.cast(), region_quark()).as_mut() }
}

fn empty_rect() -> VipsRect {
    VipsRect {
        left: 0,
        top: 0,
        width: 0,
        height: 0,
    }
}

fn image_rect(image: &VipsImage) -> VipsRect {
    VipsRect {
        left: 0,
        top: 0,
        width: image.Xsize.max(0),
        height: image.Ysize.max(0),
    }
}

fn clip_rect(image: &VipsImage, request: &VipsRect) -> VipsRect {
    let mut clipped = *request;
    crate::runtime::rect::vips_rect_intersectrect(request, &image_rect(image), &mut clipped);
    clipped
}

fn rect_is_empty(rect: &VipsRect) -> bool {
    rect.width <= 0 || rect.height <= 0
}

unsafe fn assert_region_thread(region: *mut VipsRegion) -> Result<(), ()> {
    let Some(region_ref) = (unsafe { region.as_mut() }) else {
        return Err(());
    };
    let current = unsafe { glib_sys::g_thread_self() };
    if region_ref.thread.is_null() {
        region_ref.thread = current;
        return Ok(());
    }
    if region_ref.thread != current {
        append_message_str("vips_region", "region used from a different thread");
        return Err(());
    }
    Ok(())
}

unsafe fn ensure_sequence(region: *mut VipsRegion) -> Result<*mut c_void, ()> {
    let Some(region_ref) = (unsafe { region.as_mut() }) else {
        return Err(());
    };
    let Some(image) = (unsafe { region_ref.im.as_ref() }) else {
        return Err(());
    };
    let Some(state) = (unsafe { region_state(region) }) else {
        return Err(());
    };
    if state.seq.is_null() {
        if let Some(start_fn) = image.start_fn {
            state.seq = unsafe { start_fn(region_ref.im, image.client1, image.client2) };
            region_ref.seq = state.seq;
        }
    } else {
        region_ref.seq = state.seq;
    }
    Ok(state.seq)
}

fn copy_rows(
    src: *const u8,
    src_bpl: usize,
    dst: *mut u8,
    dst_bpl: usize,
    row_size: usize,
    rows: usize,
) {
    for row in 0..rows {
        unsafe {
            ptr::copy_nonoverlapping(
                src.add(row * src_bpl),
                dst.add(row * dst_bpl),
                row_size,
            );
        }
    }
}

unsafe fn prepare_generate(region: *mut VipsRegion, request: &VipsRect) -> Result<(), ()> {
    let Some(region_ref) = (unsafe { region.as_mut() }) else {
        return Err(());
    };
    let Some(image) = (unsafe { region_ref.im.as_ref() }) else {
        return Err(());
    };
    let final_rect = clip_rect(image, request);
    crate::runtime::region::vips_region_buffer(region, &final_rect);
    let seq = unsafe { ensure_sequence(region) }?;
    let Some(image_mut) = (unsafe { region_ref.im.as_ref() }) else {
        return Err(());
    };
    let Some(generate_fn) = image_mut.generate_fn else {
        append_message_str("vips_region_prepare", "incomplete header");
        return Err(());
    };
    let mut stop = glib_sys::GFALSE;
    if unsafe { generate_fn(region, seq, image_mut.client1, image_mut.client2, &mut stop) } != 0 {
        return Err(());
    }
    if stop != glib_sys::GFALSE {
        append_message_str("vips_region_prepare", "stop requested");
        return Err(());
    }
    region_ref.invalid = glib_sys::GFALSE;
    Ok(())
}

pub(crate) fn materialize_generated_image(image: *mut VipsImage) -> Result<(), ()> {
    let Some(image_ref) = (unsafe { image.as_ref() }) else {
        return Err(());
    };
    if image_ref.generate_fn.is_none() {
        return Ok(());
    }
    let expected = image_size(image_ref);
    let _ = crate::runtime::image::vips_image_write_prepare(image);
    let region = vips_region_new(image);
    if region.is_null() {
        return Err(());
    }
    let request = image_rect(image_ref);
    let result = if vips_region_prepare(region, &request) != 0 {
        Err(())
    } else {
        let Some(state) = (unsafe { image_state(image) }) else {
            unsafe {
                object_unref(region);
            }
            return Err(());
        };
        if state.pixels.len() != expected {
            state.pixels.resize(expected, 0);
        }
        if let Some(region_ref) = unsafe { region.as_ref() } {
            let bpl = line_size(image_ref);
            let row_size = (region_ref.valid.width.max(0) as usize).saturating_mul(bytes_per_pixel(image_ref));
            copy_rows(
                region_ref.data.cast::<u8>(),
                region_ref.bpl.max(0) as usize,
                state.pixels.as_mut_ptr(),
                bpl,
                row_size,
                region_ref.valid.height.max(0) as usize,
            );
        }
        crate::runtime::image::sync_pixels(image);
        Ok(())
    };
    unsafe {
        object_unref(region);
    }
    result
}

#[no_mangle]
pub extern "C" fn vips_region_new(image: *mut VipsImage) -> *mut VipsRegion {
    if image.is_null() {
        return ptr::null_mut();
    }
    let region = unsafe { object_new::<VipsRegion>(crate::runtime::object::vips_region_get_type()) };
    let Some(region_ref) = (unsafe { region.as_mut() }) else {
        return ptr::null_mut();
    };
    let image_ref = unsafe { object_ref(image) };
    region_ref.im = image_ref;
    region_ref.valid = empty_rect();
    region_ref.r#type = VIPS_REGION_NONE;
    region_ref.data = ptr::null_mut();
    region_ref.bpl = 0;
    region_ref.seq = ptr::null_mut();
    region_ref.thread = unsafe { glib_sys::g_thread_self() };
    region_ref.window = ptr::null_mut();
    region_ref.buffer = ptr::null_mut();
    region_ref.invalid = glib_sys::GFALSE;
    unsafe {
        set_qdata_box(
            region.cast(),
            region_quark(),
            RegionState {
                image: image_ref,
                owned: Vec::new(),
                seq: ptr::null_mut(),
            },
        );
    }
    region
}

#[no_mangle]
pub extern "C" fn vips_region_buffer(region: *mut VipsRegion, rect: *const VipsRect) -> libc::c_int {
    if rect.is_null() || unsafe { assert_region_thread(region) }.is_err() {
        return -1;
    }
    let Some(region_ref) = (unsafe { region.as_mut() }) else {
        return -1;
    };
    let Some(image) = (unsafe { region_ref.im.as_ref() }) else {
        return -1;
    };
    let Some(state) = (unsafe { region_state(region) }) else {
        return -1;
    };
    let rect = unsafe { *rect };
    let bpp = bytes_per_pixel(image);
    let height = rect.height.max(0) as usize;
    let width = rect.width.max(0) as usize;
    state.owned = vec![0; width.saturating_mul(height).saturating_mul(bpp)];
    region_ref.valid = rect;
    region_ref.r#type = VIPS_REGION_BUFFER;
    region_ref.data = if state.owned.is_empty() {
        ptr::null_mut()
    } else {
        state.owned.as_mut_ptr()
    };
    region_ref.bpl = width.saturating_mul(bpp) as libc::c_int;
    region_ref.invalid = glib_sys::GFALSE;
    0
}

#[no_mangle]
pub extern "C" fn vips_region_image(region: *mut VipsRegion, rect: *const VipsRect) -> libc::c_int {
    if rect.is_null() || unsafe { assert_region_thread(region) }.is_err() {
        return -1;
    }
    let Some(region_ref) = (unsafe { region.as_mut() }) else {
        return -1;
    };
    if crate::runtime::image::ensure_pixels(region_ref.im).is_err() {
        return -1;
    }
    let Some(image) = (unsafe { region_ref.im.as_ref() }) else {
        return -1;
    };
    let clipped = clip_rect(image, unsafe { rect.as_ref() }.expect("rect"));
    region_ref.valid = clipped;
    region_ref.r#type = VIPS_REGION_OTHER_IMAGE;
    if rect_is_empty(&clipped) {
        region_ref.data = ptr::null_mut();
        region_ref.bpl = 0;
        region_ref.invalid = glib_sys::GFALSE;
        return 0;
    }
    let bpp = bytes_per_pixel(image);
    let offset = clipped.top.max(0) as usize * line_size(image) + clipped.left.max(0) as usize * bpp;
    region_ref.data = unsafe { image.data.add(offset) };
    region_ref.bpl = line_size(image) as libc::c_int;
    region_ref.invalid = glib_sys::GFALSE;
    0
}

#[no_mangle]
pub extern "C" fn vips_region_region(
    region: *mut VipsRegion,
    dest: *mut VipsRegion,
    rect: *const VipsRect,
    x: libc::c_int,
    y: libc::c_int,
) -> libc::c_int {
    if rect.is_null() || region.is_null() || dest.is_null() {
        return -1;
    }
    let Some(region_ref) = (unsafe { region.as_mut() }) else {
        return -1;
    };
    let Some(dest_ref) = (unsafe { dest.as_ref() }) else {
        return -1;
    };
    let Some(image) = (unsafe { region_ref.im.as_ref() }) else {
        return -1;
    };
    if dest_ref.data.is_null() {
        append_message_str("vips_region_region", "destination region has no buffer");
        return -1;
    }
    let rect = unsafe { *rect };
    let wanted = VipsRect {
        left: x,
        top: y,
        width: rect.width,
        height: rect.height,
    };
    if crate::runtime::rect::vips_rect_includesrect(&dest_ref.valid, &wanted) == glib_sys::GFALSE {
        append_message_str("vips_region_region", "destination region too small");
        return -1;
    }
    let bpp = bytes_per_pixel(image);
    let offset = (y - dest_ref.valid.top).max(0) as usize * dest_ref.bpl.max(0) as usize
        + (x - dest_ref.valid.left).max(0) as usize * bpp;
    region_ref.valid = rect;
    region_ref.r#type = VIPS_REGION_OTHER_REGION;
    region_ref.data = unsafe { dest_ref.data.add(offset) };
    region_ref.bpl = dest_ref.bpl;
    region_ref.invalid = dest_ref.invalid;
    0
}

#[no_mangle]
pub extern "C" fn vips_region_equalsregion(reg1: *mut VipsRegion, reg2: *mut VipsRegion) -> libc::c_int {
    match (unsafe { reg1.as_ref() }, unsafe { reg2.as_ref() }) {
        (Some(left), Some(right))
            if left.im == right.im
                && left.data == right.data
                && left.bpl == right.bpl
                && crate::runtime::rect::vips_rect_equalsrect(&left.valid, &right.valid) != glib_sys::GFALSE =>
        {
            1
        }
        _ => 0,
    }
}

#[no_mangle]
pub extern "C" fn vips_region_position(region: *mut VipsRegion, x: libc::c_int, y: libc::c_int) -> libc::c_int {
    let Some(region_ref) = (unsafe { region.as_ref() }) else {
        return -1;
    };
    if crate::runtime::rect::vips_rect_includespoint(&region_ref.valid, x, y) == glib_sys::GFALSE {
        -1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn vips_region_paint(region: *mut VipsRegion, rect: *const VipsRect, value: libc::c_int) {
    let Some(region_ref) = (unsafe { region.as_ref() }) else {
        return;
    };
    let Some(image) = (unsafe { region_ref.im.as_ref() }) else {
        return;
    };
    let Some(rect) = (unsafe { rect.as_ref() }) else {
        return;
    };
    let bpp = bytes_per_pixel(image);
    let start_x = rect.left.max(region_ref.valid.left);
    let start_y = rect.top.max(region_ref.valid.top);
    let end_x = (rect.left + rect.width).min(region_ref.valid.left + region_ref.valid.width);
    let end_y = (rect.top + rect.height).min(region_ref.valid.top + region_ref.valid.height);
    for y in start_y..end_y {
        for x in start_x..end_x {
            let pixel = unsafe {
                region_ref
                    .data
                    .add((y - region_ref.valid.top) as usize * region_ref.bpl.max(0) as usize)
                    .add((x - region_ref.valid.left) as usize * bpp)
            };
            unsafe {
                ptr::write_bytes(pixel, value as u8, bpp);
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn vips_region_paint_pel(region: *mut VipsRegion, rect: *const VipsRect, ink: *const VipsPel) {
    let Some(region_ref) = (unsafe { region.as_ref() }) else {
        return;
    };
    let Some(image) = (unsafe { region_ref.im.as_ref() }) else {
        return;
    };
    let Some(rect) = (unsafe { rect.as_ref() }) else {
        return;
    };
    if ink.is_null() {
        return;
    }
    let bpp = bytes_per_pixel(image);
    let ink = unsafe { std::slice::from_raw_parts(ink, bpp) };
    let start_x = rect.left.max(region_ref.valid.left);
    let start_y = rect.top.max(region_ref.valid.top);
    let end_x = (rect.left + rect.width).min(region_ref.valid.left + region_ref.valid.width);
    let end_y = (rect.top + rect.height).min(region_ref.valid.top + region_ref.valid.height);
    for y in start_y..end_y {
        for x in start_x..end_x {
            let pixel = unsafe {
                region_ref
                    .data
                    .add((y - region_ref.valid.top) as usize * region_ref.bpl.max(0) as usize)
                    .add((x - region_ref.valid.left) as usize * bpp)
            };
            unsafe {
                ptr::copy_nonoverlapping(ink.as_ptr(), pixel, bpp);
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn vips_region_black(region: *mut VipsRegion) {
    let Some(region_ref) = (unsafe { region.as_ref() }) else {
        return;
    };
    vips_region_paint(region, &region_ref.valid, 0);
}

#[no_mangle]
pub extern "C" fn vips_region_copy(
    region: *mut VipsRegion,
    dest: *mut VipsRegion,
    rect: *const VipsRect,
    x: libc::c_int,
    y: libc::c_int,
) {
    let (Some(src), Some(dest_ref), Some(image), Some(rect)) = (
        unsafe { region.as_ref() },
        unsafe { dest.as_ref() },
        unsafe { region.as_ref() }.and_then(|region| unsafe { region.im.as_ref() }),
        unsafe { rect.as_ref() },
    ) else {
        return;
    };
    if src.data.is_null() || dest_ref.data.is_null() {
        return;
    }
    let mut src_rect = *rect;
    crate::runtime::rect::vips_rect_intersectrect(&src_rect, &src.valid, &mut src_rect);
    if rect_is_empty(&src_rect) {
        return;
    }
    let mut wanted = VipsRect {
        left: x + (src_rect.left - rect.left),
        top: y + (src_rect.top - rect.top),
        width: src_rect.width,
        height: src_rect.height,
    };
    crate::runtime::rect::vips_rect_intersectrect(&wanted, &dest_ref.valid, &mut wanted);
    if rect_is_empty(&wanted) {
        return;
    }
    let adjusted_src = VipsRect {
        left: src_rect.left + (wanted.left - x),
        top: src_rect.top + (wanted.top - y),
        width: wanted.width,
        height: wanted.height,
    };
    let bpp = bytes_per_pixel(image);
    let row_size = wanted.width.max(0) as usize * bpp;
    let src_ptr = unsafe {
        src.data
            .add((adjusted_src.top - src.valid.top) as usize * src.bpl.max(0) as usize)
            .add((adjusted_src.left - src.valid.left) as usize * bpp)
    };
    let dest_ptr = unsafe {
        dest_ref
            .data
            .add((wanted.top - dest_ref.valid.top) as usize * dest_ref.bpl.max(0) as usize)
            .add((wanted.left - dest_ref.valid.left) as usize * bpp)
    };
    copy_rows(
        src_ptr.cast::<u8>(),
        src.bpl.max(0) as usize,
        dest_ptr.cast::<u8>(),
        dest_ref.bpl.max(0) as usize,
        row_size,
        wanted.height.max(0) as usize,
    );
}

#[no_mangle]
pub extern "C" fn vips_region_shrink_method(
    from: *mut VipsRegion,
    to: *mut VipsRegion,
    target: *const VipsRect,
    method: VipsRegionShrink,
) -> libc::c_int {
    if target.is_null() {
        return -1;
    }
    let method = method;
    if method != VIPS_REGION_SHRINK_NEAREST {
        return vips_region_shrink(from, to, target);
    }
    let Some(target) = (unsafe { target.as_ref() }) else {
        return -1;
    };
    if vips_region_buffer(to, target) != 0 {
        return -1;
    }
    vips_region_copy(from, to, target, target.left, target.top);
    0
}

#[no_mangle]
pub extern "C" fn vips_region_shrink(
    from: *mut VipsRegion,
    to: *mut VipsRegion,
    target: *const VipsRect,
) -> libc::c_int {
    vips_region_shrink_method(from, to, target, VIPS_REGION_SHRINK_NEAREST)
}

#[no_mangle]
pub extern "C" fn vips_region_prepare(region: *mut VipsRegion, rect: *const VipsRect) -> libc::c_int {
    if rect.is_null() || unsafe { assert_region_thread(region) }.is_err() {
        return -1;
    }
    let Some(region_ref) = (unsafe { region.as_mut() }) else {
        return -1;
    };
    let Some(image) = (unsafe { region_ref.im.as_ref() }) else {
        return -1;
    };
    if crate::runtime::image::vips_image_iskilled(region_ref.im) != glib_sys::GFALSE {
        return -1;
    }
    let request = unsafe { *rect };
    let is_generated =
        image.generate_fn.is_some() || image.dtype == VIPS_IMAGE_PARTIAL || image.dtype == VIPS_IMAGE_OPENOUT;
    if is_generated {
        if unsafe { prepare_generate(region, &request) }.is_err() {
            return -1;
        }
        return 0;
    }
    vips_region_image(region, &request)
}

#[no_mangle]
pub extern "C" fn vips_region_prepare_to(
    region: *mut VipsRegion,
    dest: *mut VipsRegion,
    rect: *const VipsRect,
    x: libc::c_int,
    y: libc::c_int,
) -> libc::c_int {
    if rect.is_null() || dest.is_null() || region.is_null() {
        return -1;
    }
    let Some(region_ref) = (unsafe { region.as_ref() }) else {
        return -1;
    };
    let Some(dest_ref) = (unsafe { dest.as_mut() }) else {
        return -1;
    };
    let Some(image) = (unsafe { region_ref.im.as_ref() }) else {
        return -1;
    };
    let Some(dest_image) = (unsafe { dest_ref.im.as_ref() }) else {
        return -1;
    };
    if dest_ref.data.is_null() || image.Bands != dest_image.Bands || image.BandFmt != dest_image.BandFmt {
        append_message_str("vips_region_prepare_to", "inappropriate region type");
        return -1;
    }
    let requested = unsafe { *rect };
    let clipped = clip_rect(image, &requested);
    let wanted = VipsRect {
        left: x + (clipped.left - requested.left),
        top: y + (clipped.top - requested.top),
        width: clipped.width,
        height: clipped.height,
    };
    if crate::runtime::rect::vips_rect_includesrect(&dest_ref.valid, &wanted) == glib_sys::GFALSE {
        append_message_str("vips_region_prepare_to", "dest too small");
        return -1;
    }
    if rect_is_empty(&clipped) {
        append_message_str("vips_region_prepare_to", "valid clipped to nothing");
        return -1;
    }
    if vips_region_prepare(region, &clipped) != 0 {
        return -1;
    }
    vips_region_copy(region, dest, &clipped, wanted.left, wanted.top);
    if let Some(dest_mut) = unsafe { dest.as_mut() } {
        dest_mut.invalid = glib_sys::GFALSE;
    }
    let _ = region_ref;
    0
}

#[no_mangle]
pub extern "C" fn vips_region_prepare_many(regions: *mut *mut VipsRegion, rect: *const VipsRect) -> libc::c_int {
    if regions.is_null() || rect.is_null() {
        return -1;
    }
    let mut current = regions;
    loop {
        let region = unsafe { *current };
        if region.is_null() {
            break;
        }
        if vips_region_prepare(region, rect) != 0 {
            return -1;
        }
        current = unsafe { current.add(1) };
    }
    0
}

#[no_mangle]
pub extern "C" fn vips_region_fetch(
    region: *mut VipsRegion,
    left: libc::c_int,
    top: libc::c_int,
    width: libc::c_int,
    height: libc::c_int,
    len: *mut usize,
) -> *mut VipsPel {
    let request = VipsRect {
        left,
        top,
        width,
        height,
    };
    if vips_region_prepare(region, &request) != 0 {
        return ptr::null_mut();
    }
    let Some(region_ref) = (unsafe { region.as_ref() }) else {
        return ptr::null_mut();
    };
    let Some(image) = (unsafe { region_ref.im.as_ref() }) else {
        return ptr::null_mut();
    };
    let bpp = bytes_per_pixel(image);
    let size = width.max(0) as usize * height.max(0) as usize * bpp;
    unsafe {
        if !len.is_null() {
            *len = size;
        }
    }
    if size == 0 || region_ref.data.is_null() {
        return ptr::null_mut();
    }
    let out = unsafe { glib_sys::g_malloc(size) }.cast::<u8>();
    if out.is_null() {
        return ptr::null_mut();
    }
    copy_rows(
        region_ref.data.cast::<u8>(),
        region_ref.bpl.max(0) as usize,
        out,
        width.max(0) as usize * bpp,
        width.max(0) as usize * bpp,
        height.max(0) as usize,
    );
    out
}

#[no_mangle]
pub extern "C" fn vips_region_width(region: *mut VipsRegion) -> libc::c_int {
    unsafe { region.as_ref() }.map_or(0, |region| region.valid.width)
}

#[no_mangle]
pub extern "C" fn vips_region_height(region: *mut VipsRegion) -> libc::c_int {
    unsafe { region.as_ref() }.map_or(0, |region| region.valid.height)
}

#[no_mangle]
pub extern "C" fn vips_region_invalidate(region: *mut VipsRegion) {
    if let Some(region) = unsafe { region.as_mut() } {
        region.invalid = glib_sys::GTRUE;
    }
}

#[allow(dead_code)]
fn _region_type_name(region_type: RegionType) -> &'static str {
    match region_type {
        VIPS_REGION_NONE => "none",
        VIPS_REGION_BUFFER => "buffer",
        VIPS_REGION_OTHER_REGION => "region",
        VIPS_REGION_OTHER_IMAGE => "image",
        _ => "other",
    }
}
