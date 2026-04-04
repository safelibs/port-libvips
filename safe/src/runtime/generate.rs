use std::ptr;

use crate::abi::basic::{VipsGenerateFn, VipsRect, VipsStartFn, VipsStopFn};
use crate::abi::image::{
    VipsDemandStyle, VipsImage, VIPS_DEMAND_STYLE_ANY, VIPS_IMAGE_NONE, VIPS_IMAGE_OPENOUT,
    VIPS_IMAGE_PARTIAL, VIPS_IMAGE_SETBUF, VIPS_IMAGE_SETBUF_FOREIGN,
};
use crate::abi::region::VipsRegion;
use crate::runtime::error::append_message_str;
use crate::runtime::object::object_unref;

type VipsRegionWrite = Option<unsafe extern "C" fn(region: *mut VipsRegion, area: *mut VipsRect, a: *mut libc::c_void) -> libc::c_int>;
type VipsSinkNotify = Option<unsafe extern "C" fn(im: *mut VipsImage, rect: *mut VipsRect, a: *mut libc::c_void)>;

fn attach_generate_callbacks(
    image: *mut VipsImage,
    start_fn: VipsStartFn,
    generate_fn: VipsGenerateFn,
    stop_fn: VipsStopFn,
    a: *mut libc::c_void,
    b: *mut libc::c_void,
) -> Result<(), ()> {
    let Some(image) = (unsafe { image.as_mut() }) else {
        return Err(());
    };
    if image.generate_fn.is_some() || image.start_fn.is_some() || image.stop_fn.is_some() {
        append_message_str("VipsImage", "generate() called twice");
        return Err(());
    }
    image.start_fn = start_fn;
    image.generate_fn = generate_fn;
    image.stop_fn = stop_fn;
    image.client1 = a;
    image.client2 = b;
    image.Bbits = (crate::runtime::image::format_sizeof(image.BandFmt) * 8) as libc::c_int;
    Ok(())
}

#[no_mangle]
pub extern "C" fn vips_sink_disc(
    im: *mut VipsImage,
    write_fn: VipsRegionWrite,
    a: *mut libc::c_void,
) -> libc::c_int {
    if crate::runtime::region::materialize_generated_image(im).is_err() {
        return -1;
    }
    let region = crate::runtime::region::vips_region_new(im);
    if region.is_null() {
        return -1;
    }
    let result = if let (Some(image), Some(write_fn)) = (unsafe { im.as_ref() }, write_fn) {
        let mut area = VipsRect {
            left: 0,
            top: 0,
            width: image.Xsize,
            height: image.Ysize,
        };
        if crate::runtime::region::vips_region_prepare(region, &area) != 0 {
            -1
        } else {
            unsafe { write_fn(region, &mut area, a) }
        }
    } else {
        0
    };
    unsafe {
        object_unref(region);
    }
    result
}

#[no_mangle]
pub extern "C" fn vips_sink(
    im: *mut VipsImage,
    start_fn: VipsStartFn,
    generate_fn: VipsGenerateFn,
    stop_fn: VipsStopFn,
    a: *mut libc::c_void,
    b: *mut libc::c_void,
) -> libc::c_int {
    vips_sink_tile(im, -1, -1, start_fn, generate_fn, stop_fn, a, b)
}

#[no_mangle]
pub extern "C" fn vips_sink_tile(
    im: *mut VipsImage,
    _tile_width: libc::c_int,
    _tile_height: libc::c_int,
    start_fn: VipsStartFn,
    generate_fn: VipsGenerateFn,
    stop_fn: VipsStopFn,
    a: *mut libc::c_void,
    b: *mut libc::c_void,
) -> libc::c_int {
    let Some(generate_fn) = generate_fn else {
        return -1;
    };
    let region = crate::runtime::region::vips_region_new(im);
    if region.is_null() {
        return -1;
    }
    let seq = start_fn.map(|start_fn| unsafe { start_fn(im, a, b) }).unwrap_or(ptr::null_mut());
    let result = if let Some(image) = unsafe { im.as_ref() } {
        let request = VipsRect {
            left: 0,
            top: 0,
            width: image.Xsize,
            height: image.Ysize,
        };
        let _ = crate::runtime::region::vips_region_buffer(region, &request);
        let mut stop = glib_sys::GFALSE;
        unsafe { generate_fn(region, seq, a, b, &mut stop) }
    } else {
        -1
    };
    if let Some(stop_fn) = stop_fn {
        let _ = unsafe { stop_fn(seq, a, b) };
    }
    unsafe {
        object_unref(region);
    }
    result
}

#[no_mangle]
pub extern "C" fn vips_sink_screen(
    _in: *mut VipsImage,
    _out: *mut VipsImage,
    _mask: *mut VipsImage,
    _tile_width: libc::c_int,
    _tile_height: libc::c_int,
    _max_tiles: libc::c_int,
    _priority: libc::c_int,
    notify_fn: VipsSinkNotify,
    a: *mut libc::c_void,
) -> libc::c_int {
    if let Some(notify_fn) = notify_fn {
        let mut rect = VipsRect {
            left: 0,
            top: 0,
            width: 0,
            height: 0,
        };
        unsafe {
            notify_fn(ptr::null_mut(), &mut rect, a);
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn vips_sink_memory(image: *mut VipsImage) -> libc::c_int {
    if crate::runtime::region::materialize_generated_image(image).is_ok() {
        0
    } else {
        -1
    }
}

#[no_mangle]
pub extern "C" fn vips_start_one(
    _out: *mut VipsImage,
    a: *mut libc::c_void,
    _b: *mut libc::c_void,
) -> *mut libc::c_void {
    crate::runtime::region::vips_region_new(a.cast::<VipsImage>()).cast::<libc::c_void>()
}

#[no_mangle]
pub extern "C" fn vips_stop_one(
    seq: *mut libc::c_void,
    _a: *mut libc::c_void,
    _b: *mut libc::c_void,
) -> libc::c_int {
    unsafe {
        object_unref(seq.cast::<VipsRegion>());
    }
    0
}

#[no_mangle]
pub extern "C" fn vips_start_many(
    _out: *mut VipsImage,
    a: *mut libc::c_void,
    _b: *mut libc::c_void,
) -> *mut libc::c_void {
    let images = a.cast::<*mut VipsImage>();
    if images.is_null() {
        return ptr::null_mut();
    }
    let mut count = 0usize;
    while unsafe { *images.add(count) }.is_null() == false {
        count += 1;
    }
    let bytes = (count + 1) * std::mem::size_of::<*mut VipsRegion>();
    let array = unsafe { glib_sys::g_malloc0(bytes) }.cast::<*mut VipsRegion>();
    if array.is_null() {
        return ptr::null_mut();
    }
    for index in 0..count {
        let image = unsafe { *images.add(index) };
        let region = crate::runtime::region::vips_region_new(image);
        if region.is_null() {
            let _ = vips_stop_many(array.cast::<libc::c_void>(), ptr::null_mut(), ptr::null_mut());
            return ptr::null_mut();
        }
        unsafe {
            *array.add(index) = region;
        }
    }
    array.cast::<libc::c_void>()
}

#[no_mangle]
pub extern "C" fn vips_stop_many(
    seq: *mut libc::c_void,
    _a: *mut libc::c_void,
    _b: *mut libc::c_void,
) -> libc::c_int {
    if seq.is_null() {
        return 0;
    }
    let array = seq.cast::<*mut VipsRegion>();
    let mut index = 0usize;
    loop {
        let region = unsafe { *array.add(index) };
        if region.is_null() {
            break;
        }
        unsafe {
            object_unref(region);
        }
        index += 1;
    }
    unsafe {
        glib_sys::g_free(seq);
    }
    0
}

#[no_mangle]
pub extern "C" fn vips_image_generate(
    image: *mut VipsImage,
    start_fn: VipsStartFn,
    generate_fn: VipsGenerateFn,
    stop_fn: VipsStopFn,
    a: *mut libc::c_void,
    b: *mut libc::c_void,
) -> libc::c_int {
    let Some(image_ref) = (unsafe { image.as_mut() }) else {
        return -1;
    };
    if generate_fn.is_none() {
        return -1;
    }
    if image_ref.hint_set == glib_sys::GFALSE {
        append_message_str("vips_image_generate", "demand hint not set");
        return -1;
    }
    if attach_generate_callbacks(image, start_fn, generate_fn, stop_fn, a, b).is_err() {
        return -1;
    }

    match image_ref.dtype {
        VIPS_IMAGE_NONE => {
            image_ref.dtype = VIPS_IMAGE_PARTIAL;
            0
        }
        VIPS_IMAGE_PARTIAL => 0,
        VIPS_IMAGE_SETBUF | VIPS_IMAGE_SETBUF_FOREIGN | VIPS_IMAGE_OPENOUT => {
            let _ = crate::runtime::image::vips_image_write_prepare(image);
            vips_sink_memory(image)
        }
        _ => {
            image_ref.dtype = VIPS_IMAGE_PARTIAL;
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn vips_image_pipeline_array(
    image: *mut VipsImage,
    hint: VipsDemandStyle,
    _in: *mut *mut VipsImage,
) -> libc::c_int {
    if let Some(image) = unsafe { image.as_mut() } {
        image.dhint = if hint < 0 { VIPS_DEMAND_STYLE_ANY } else { hint };
        image.hint_set = glib_sys::GTRUE;
        0
    } else {
        -1
    }
}
