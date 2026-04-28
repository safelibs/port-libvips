mod arithmetic;
mod colour;
mod conversion;
mod convolution;
mod create;
mod draw;
mod freqfilt;
mod histogram;
mod morphology;
mod mosaicing;
mod resample;

use std::ffi::{c_void, CStr, CString};
use std::mem::ManuallyDrop;
use std::ptr;

use libc::c_int;

use crate::abi::connection::{VipsSource, VipsTarget};
use crate::abi::image::VipsImage;
use crate::abi::object::{VipsArgumentInstance, VipsObject};
use crate::abi::r#type::{VipsArrayDouble, VipsArrayImage, VipsBlob};
use crate::pixels::ImageBuffer;
use crate::runtime::error::append_message_str;
use crate::runtime::header::copy_metadata;
use crate::runtime::image::{
    safe_vips_image_new_from_source_internal, safe_vips_image_write_to_target_internal,
};
use crate::runtime::object;
use crate::runtime::r#type::{vips_array_double_get, vips_blob_get};
use crate::runtime::source::{vips_source_new_from_file, vips_source_new_from_memory};
use crate::runtime::target::vips_target_new_to_file;

const SUPPORTED_OPERATIONS: &[&str] = &[
    "abs",
    "add",
    "arrayjoin",
    "avg",
    "bandbool",
    "bandfold",
    "bandjoin",
    "bandjoin_const",
    "bandmean",
    "bandrank",
    "bandunfold",
    "black",
    "blockcache",
    "boolean",
    "boolean_const",
    "buildlut",
    "byteswap",
    "cache",
    "case",
    "cast",
    "colourspace",
    "complex",
    "complexform",
    "complexget",
    "composite2",
    "compass",
    "conv",
    "convsep",
    "copy",
    "countlines",
    "crop",
    "dE00",
    "dE76",
    "dECMC",
    "draw_circle",
    "draw_flood",
    "draw_image",
    "draw_line",
    "draw_mask",
    "draw_rect",
    "draw_smudge",
    "deviate",
    "divide",
    "embed",
    "extract_area",
    "extract_band",
    "eye",
    "falsecolour",
    "fastcor",
    "fill_nearest",
    "flatten",
    "flip",
    "freqmult",
    "gamma",
    "gaussblur",
    "gaussmat",
    "gaussnoise",
    "globalbalance",
    "gravity",
    "grey",
    "grid",
    "hough_circle",
    "hough_line",
    "HSV2sRGB",
    "hist_cum",
    "hist_entropy",
    "hist_equal",
    "hist_find",
    "hist_find_indexed",
    "hist_find_ndim",
    "hist_ismonotonic",
    "hist_local",
    "hist_match",
    "hist_norm",
    "hist_plot",
    "identity",
    "ifthenelse",
    "insert",
    "invert",
    "invertlut",
    "join",
    "labelregions",
    "linear",
    "linecache",
    "logmat",
    "maplut",
    "mask_gaussian",
    "mask_gaussian_band",
    "mask_gaussian_ring",
    "mask_butterworth",
    "mask_butterworth_band",
    "mask_butterworth_ring",
    "mask_fractal",
    "mask_ideal",
    "mask_ideal_band",
    "mask_ideal_ring",
    "math",
    "math2",
    "math2_const",
    "mapim",
    "match",
    "measure",
    "merge",
    "msb",
    "mosaic",
    "mosaic1",
    "matrixinvert",
    "morph",
    "multiply",
    "pngload",
    "pngload_buffer",
    "pngload_source",
    "pngsave",
    "pngsave_buffer",
    "pngsave_target",
    "percent",
    "premultiply",
    "profile",
    "profile_load",
    "project",
    "prewitt",
    "rank",
    "relational",
    "relational_const",
    "reduce",
    "reduceh",
    "reducev",
    "recomb",
    "remainder",
    "remainder_const",
    "resize",
    "rot",
    "rot45",
    "round",
    "sRGB2HSV",
    "sRGB2scRGB",
    "scharr",
    "scRGB2BW",
    "scRGB2sRGB",
    "scRGB2XYZ",
    "sign",
    "scale",
    "sharpen",
    "shrink",
    "shrinkh",
    "shrinkv",
    "smartcrop",
    "sobel",
    "sines",
    "subtract",
    "subsample",
    "sum",
    "switch",
    "stats",
    "stdif",
    "spcor",
    "tilecache",
    "thumbnail",
    "thumbnail_buffer",
    "thumbnail_image",
    "thumbnail_source",
    "tonelut",
    "unpremultiply",
    "wrap",
    "XYZ2Lab",
    "XYZ2scRGB",
    "XYZ2Yxy",
    "Lab2XYZ",
    "Lab2LCh",
    "LCh2Lab",
    "Yxy2XYZ",
    "xyz",
    "zone",
    "zoom",
];

pub(crate) fn is_manifest_supported_operation(nickname: &str) -> bool {
    SUPPORTED_OPERATIONS.contains(&nickname)
}

struct OwnedValue {
    inner: ManuallyDrop<gobject_sys::GValue>,
}

impl OwnedValue {
    unsafe fn new(value_type: glib_sys::GType) -> Self {
        let mut inner: gobject_sys::GValue = unsafe { std::mem::zeroed() };
        unsafe {
            gobject_sys::g_value_init(&mut inner, value_type);
        }
        Self {
            inner: ManuallyDrop::new(inner),
        }
    }

    fn as_ptr(&mut self) -> *mut gobject_sys::GValue {
        &mut *self.inner
    }

    fn as_ref(&self) -> &gobject_sys::GValue {
        &self.inner
    }
}

impl Drop for OwnedValue {
    fn drop(&mut self) {
        unsafe {
            gobject_sys::g_value_unset(&mut *self.inner);
        }
    }
}

fn cstring(name: &str) -> Result<CString, ()> {
    CString::new(name).map_err(|_| ())
}

unsafe fn argument_info(
    object: *mut VipsObject,
    name: &str,
) -> Result<(*mut gobject_sys::GParamSpec, *mut VipsArgumentInstance), ()> {
    let name = cstring(name)?;
    let class = unsafe { object::object_class(object) };
    if class.is_null() {
        return Err(());
    }
    let fallback =
        unsafe { gobject_sys::g_object_class_find_property(class.cast(), name.as_ptr()) };
    if fallback.is_null() {
        return Err(());
    }
    let mut pspec = ptr::null_mut();
    let mut instance = ptr::null_mut();
    if object::vips_object_get_argument(
        object,
        name.as_ptr(),
        &mut pspec,
        ptr::null_mut(),
        &mut instance,
    ) != 0
    {
        return Ok((fallback, ptr::null_mut()));
    }
    Ok((pspec, instance))
}

pub(crate) unsafe fn argument_assigned(object: *mut VipsObject, name: &str) -> Result<bool, ()> {
    let Ok((_, instance)) = (unsafe { argument_info(object, name) }) else {
        return Ok(false);
    };
    Ok(!instance.is_null() && unsafe { (*instance).assigned != glib_sys::GFALSE })
}

unsafe fn property_value(object: *mut VipsObject, name: &str) -> Result<OwnedValue, ()> {
    let (pspec, _) = unsafe { argument_info(object, name)? };
    let mut value = unsafe { OwnedValue::new((*pspec).value_type) };
    let cname = cstring(name)?;
    unsafe {
        gobject_sys::g_object_get_property(object.cast(), cname.as_ptr(), value.as_ptr());
    }
    Ok(value)
}

unsafe fn set_property(
    object: *mut VipsObject,
    name: &str,
    init: impl FnOnce(*mut gobject_sys::GValue),
) -> Result<(), ()> {
    let (pspec, _) = unsafe { argument_info(object, name)? };
    let mut value = unsafe { OwnedValue::new((*pspec).value_type) };
    init(value.as_ptr());
    let cname = cstring(name)?;
    unsafe {
        gobject_sys::g_object_set_property(object.cast(), cname.as_ptr(), value.as_ptr());
        object::mark_argument_assigned(object, name, true)?;
    }
    Ok(())
}

pub(crate) unsafe fn get_int(object: *mut VipsObject, name: &str) -> Result<c_int, ()> {
    let value = unsafe { property_value(object, name)? };
    Ok(unsafe { gobject_sys::g_value_get_int(value.as_ref()) })
}

pub(crate) unsafe fn get_uint64(object: *mut VipsObject, name: &str) -> Result<u64, ()> {
    let value = unsafe { property_value(object, name)? };
    Ok(unsafe { gobject_sys::g_value_get_uint64(value.as_ref()) })
}

pub(crate) unsafe fn get_double(object: *mut VipsObject, name: &str) -> Result<f64, ()> {
    let value = unsafe { property_value(object, name)? };
    Ok(unsafe { gobject_sys::g_value_get_double(value.as_ref()) })
}

pub(crate) unsafe fn get_bool(object: *mut VipsObject, name: &str) -> Result<bool, ()> {
    let value = unsafe { property_value(object, name)? };
    Ok(unsafe { gobject_sys::g_value_get_boolean(value.as_ref()) != glib_sys::GFALSE })
}

pub(crate) unsafe fn get_enum(object: *mut VipsObject, name: &str) -> Result<c_int, ()> {
    let value = unsafe { property_value(object, name)? };
    Ok(unsafe { gobject_sys::g_value_get_enum(value.as_ref()) })
}

pub(crate) unsafe fn get_flags(object: *mut VipsObject, name: &str) -> Result<u32, ()> {
    let value = unsafe { property_value(object, name)? };
    Ok(unsafe { gobject_sys::g_value_get_flags(value.as_ref()) })
}

pub(crate) unsafe fn get_string(object: *mut VipsObject, name: &str) -> Result<Option<String>, ()> {
    let value = unsafe { property_value(object, name)? };
    let ptr = unsafe { gobject_sys::g_value_get_string(value.as_ref()) };
    if ptr.is_null() {
        Ok(None)
    } else {
        Ok(Some(
            unsafe { CStr::from_ptr(ptr) }
                .to_string_lossy()
                .into_owned(),
        ))
    }
}

pub(crate) unsafe fn get_image_ref(
    object: *mut VipsObject,
    name: &str,
) -> Result<*mut VipsImage, ()> {
    unsafe { get_object_ref(object, name) }
}

pub(crate) unsafe fn get_object_ref<T>(object: *mut VipsObject, name: &str) -> Result<*mut T, ()> {
    let value = unsafe { property_value(object, name)? };
    let ptr = unsafe { gobject_sys::g_value_dup_object(value.as_ref()) }.cast::<T>();
    if ptr.is_null() {
        Err(())
    } else {
        Ok(ptr)
    }
}

pub(crate) unsafe fn get_image_buffer(
    object: *mut VipsObject,
    name: &str,
) -> Result<ImageBuffer, ()> {
    let image = unsafe { get_image_ref(object, name)? };
    let buffer = ImageBuffer::from_image(image);
    unsafe {
        object::object_unref(image);
    }
    buffer
}

pub(crate) unsafe fn get_array_double(object: *mut VipsObject, name: &str) -> Result<Vec<f64>, ()> {
    let (gtype, boxed) = unsafe { object::dynamic_boxed_value(object, name) }.ok_or(())?;
    if gtype != crate::runtime::r#type::vips_array_double_get_type() {
        return Err(());
    }
    let array = boxed.cast::<VipsArrayDouble>();
    if array.is_null() {
        return Err(());
    }
    let mut n = 0;
    let data = vips_array_double_get(array, &mut n);
    if n < 0 || (data.is_null() && n != 0) {
        return Err(());
    }
    if n == 0 {
        return Ok(Vec::new());
    }
    Ok(unsafe { std::slice::from_raw_parts(data, n as usize) }.to_vec())
}

pub(crate) unsafe fn get_array_images(
    object: *mut VipsObject,
    name: &str,
) -> Result<Vec<*mut VipsImage>, ()> {
    let (gtype, boxed) = unsafe { object::dynamic_boxed_value(object, name) }.ok_or(())?;
    if gtype != crate::runtime::r#type::vips_array_image_get_type() {
        return Err(());
    }
    let array = boxed.cast::<VipsArrayImage>();
    if array.is_null() {
        return Err(());
    }
    let area = unsafe { &(*array).area };
    if area.n < 0 || (area.data.is_null() && area.n != 0) {
        return Err(());
    }
    if area.n == 0 {
        return Ok(Vec::new());
    }
    let data = area.data.cast::<*mut VipsImage>();
    Ok(unsafe { std::slice::from_raw_parts(data, area.n as usize) }.to_vec())
}

pub(crate) unsafe fn get_blob_bytes(object: *mut VipsObject, name: &str) -> Result<Vec<u8>, ()> {
    let (gtype, boxed) = unsafe { object::dynamic_boxed_value(object, name) }.ok_or(())?;
    if gtype != crate::runtime::r#type::vips_blob_get_type() {
        return Err(());
    }
    let blob = boxed.cast::<VipsBlob>();
    if blob.is_null() {
        return Err(());
    }
    let mut len = 0usize;
    let ptr = vips_blob_get(blob, &mut len);
    if ptr.is_null() && len != 0 {
        return Err(());
    }
    if len == 0 {
        return Ok(Vec::new());
    }
    Ok(unsafe { std::slice::from_raw_parts(ptr.cast::<u8>(), len) }.to_vec())
}

pub(crate) unsafe fn set_output_int(
    object: *mut VipsObject,
    name: &str,
    value: c_int,
) -> Result<(), ()> {
    unsafe {
        set_property(object, name, |gvalue| {
            gobject_sys::g_value_set_int(gvalue, value);
        })
    }
}

pub(crate) unsafe fn set_output_double(
    object: *mut VipsObject,
    name: &str,
    value: f64,
) -> Result<(), ()> {
    unsafe {
        set_property(object, name, |gvalue| {
            gobject_sys::g_value_set_double(gvalue, value);
        })
    }
}

pub(crate) unsafe fn set_output_bool(
    object: *mut VipsObject,
    name: &str,
    value: bool,
) -> Result<(), ()> {
    unsafe {
        set_property(object, name, |gvalue| {
            gobject_sys::g_value_set_boolean(
                gvalue,
                if value {
                    glib_sys::GTRUE
                } else {
                    glib_sys::GFALSE
                },
            );
        })
    }
}

pub(crate) unsafe fn set_output_image(
    object: *mut VipsObject,
    name: &str,
    image: *mut VipsImage,
) -> Result<(), ()> {
    unsafe {
        set_property(object, name, |gvalue| {
            gobject_sys::g_value_set_object(gvalue, image.cast());
        })
    }
}

pub(crate) unsafe fn set_output_blob(
    object: *mut VipsObject,
    name: &str,
    bytes: Vec<u8>,
) -> Result<(), ()> {
    let blob = crate::runtime::r#type::vips_blob_copy(bytes.as_ptr().cast::<c_void>(), bytes.len());
    let result = unsafe {
        set_property(object, name, |gvalue| {
            gobject_sys::g_value_set_boxed(gvalue, blob.cast::<c_void>());
        })
    };
    crate::runtime::r#type::vips_area_unref(blob.cast::<crate::abi::r#type::VipsArea>());
    result
}

pub(crate) unsafe fn set_output_array_double(
    object: *mut VipsObject,
    name: &str,
    values: &[f64],
) -> Result<(), ()> {
    let array =
        crate::runtime::r#type::vips_array_double_new(values.as_ptr(), values.len() as c_int);
    let result = unsafe {
        set_property(object, name, |gvalue| {
            gobject_sys::g_value_set_boxed(gvalue, array.cast::<c_void>());
        })
    };
    crate::runtime::r#type::vips_area_unref(array.cast::<crate::abi::r#type::VipsArea>());
    result
}

pub(crate) unsafe fn set_output_image_like(
    object: *mut VipsObject,
    name: &str,
    buffer: ImageBuffer,
    like: *mut VipsImage,
) -> Result<(), ()> {
    let out = buffer.into_image_like(like);
    unsafe { set_output_image(object, name, out) }
}

pub(crate) fn copy_output_metadata(out: *mut VipsImage, like: *mut VipsImage) {
    copy_metadata(out, like);
}

fn nickname(object: *mut VipsObject) -> Result<String, ()> {
    let class = unsafe { object::object_class(object) };
    if class.is_null() || unsafe { (*class).nickname.is_null() } {
        return Err(());
    }
    Ok(unsafe { CStr::from_ptr((*class).nickname) }
        .to_string_lossy()
        .into_owned())
}

unsafe fn dispatch_operation(object: *mut VipsObject, nickname: &str) -> Result<bool, ()> {
    if unsafe { arithmetic::dispatch(object, nickname)? } {
        return Ok(true);
    }
    if unsafe { colour::dispatch(object, nickname)? } {
        return Ok(true);
    }
    if unsafe { conversion::dispatch(object, nickname)? } {
        return Ok(true);
    }
    if unsafe { convolution::dispatch(object, nickname)? } {
        return Ok(true);
    }
    if unsafe { create::dispatch(object, nickname)? } {
        return Ok(true);
    }
    if unsafe { draw::dispatch(object, nickname)? } {
        return Ok(true);
    }
    if unsafe { histogram::dispatch(object, nickname)? } {
        return Ok(true);
    }
    if unsafe { mosaicing::dispatch(object, nickname)? } {
        return Ok(true);
    }
    if unsafe { morphology::dispatch(object, nickname)? } {
        return Ok(true);
    }
    if unsafe { freqfilt::dispatch(object, nickname)? } {
        return Ok(true);
    }
    if unsafe { resample::dispatch(object, nickname)? } {
        return Ok(true);
    }
    if crate::foreign::dispatch_operation(object, nickname)? {
        return Ok(true);
    }
    Ok(false)
}

unsafe fn dispatch_png(object: *mut VipsObject, nickname: &str) -> Result<bool, ()> {
    match nickname {
        "pngload" => {
            let filename = unsafe { get_string(object, "filename")? }.ok_or(())?;
            let cfilename = cstring(&filename)?;
            let source = vips_source_new_from_file(cfilename.as_ptr());
            if source.is_null() {
                return Err(());
            }
            let out = safe_vips_image_new_from_source_internal(source, ptr::null(), 0);
            unsafe {
                object::object_unref(source);
            }
            if out.is_null() {
                return Err(());
            }
            unsafe { set_output_image(object, "out", out)? };
            Ok(true)
        }
        "pngload_buffer" => {
            let bytes = unsafe { get_blob_bytes(object, "buffer")? };
            let source = vips_source_new_from_memory(bytes.as_ptr().cast::<c_void>(), bytes.len());
            if source.is_null() {
                return Err(());
            }
            let out = safe_vips_image_new_from_source_internal(source, ptr::null(), 0);
            unsafe {
                object::object_unref(source);
            }
            if out.is_null() {
                return Err(());
            }
            unsafe { set_output_image(object, "out", out)? };
            Ok(true)
        }
        "pngload_source" => {
            let source = unsafe { get_object_ref::<VipsSource>(object, "source")? };
            let out = safe_vips_image_new_from_source_internal(source, ptr::null(), 0);
            unsafe {
                object::object_unref(source);
            }
            if out.is_null() {
                return Err(());
            }
            unsafe { set_output_image(object, "out", out)? };
            Ok(true)
        }
        "pngsave" => {
            let image = unsafe { get_image_ref(object, "in")? };
            let filename = unsafe { get_string(object, "filename")? }.ok_or(())?;
            let cfilename = cstring(&filename)?;
            let target = vips_target_new_to_file(cfilename.as_ptr());
            if target.is_null() {
                unsafe {
                    object::object_unref(image);
                }
                return Err(());
            }
            let result = safe_vips_image_write_to_target_internal(image, c".png".as_ptr(), target);
            unsafe {
                object::object_unref(target);
                object::object_unref(image);
            }
            if result != 0 {
                return Err(());
            }
            Ok(true)
        }
        "pngsave_buffer" => {
            let image = unsafe { get_image_ref(object, "in")? };
            let mut len = 0usize;
            let ptr = crate::runtime::image::vips_image_write_to_memory(image, &mut len);
            unsafe {
                object::object_unref(image);
            }
            if ptr.is_null() && len != 0 {
                return Err(());
            }
            let bytes = unsafe { std::slice::from_raw_parts(ptr.cast::<u8>(), len) }.to_vec();
            unsafe {
                glib_sys::g_free(ptr);
            }
            unsafe { set_output_blob(object, "buffer", bytes)? };
            Ok(true)
        }
        "pngsave_target" => {
            let image = unsafe { get_image_ref(object, "in")? };
            let value = unsafe { property_value(object, "target")? };
            let target =
                unsafe { gobject_sys::g_value_dup_object(value.as_ref()) }.cast::<VipsTarget>();
            if target.is_null() {
                unsafe {
                    object::object_unref(image);
                }
                return Err(());
            }
            let result = safe_vips_image_write_to_target_internal(image, c".png".as_ptr(), target);
            unsafe {
                object::object_unref(target);
                object::object_unref(image);
            }
            if result != 0 {
                return Err(());
            }
            Ok(true)
        }
        _ => Ok(false),
    }
}

pub(crate) unsafe extern "C" fn generated_operation_build(object: *mut VipsObject) -> c_int {
    if object.is_null() {
        append_message_str("generated_operation_build", "object is NULL");
        return -1;
    }
    if unsafe { object::default_object_build(object) } != 0 {
        return -1;
    }
    let Ok(nickname) = nickname(object) else {
        append_message_str("generated_operation_build", "operation nickname missing");
        return -1;
    };
    let result = unsafe { dispatch_operation(object, &nickname) }.and_then(|handled| {
        if handled {
            Ok(true)
        } else {
            unsafe { dispatch_png(object, &nickname) }
        }
    });
    match result {
        Ok(true) => 0,
        Ok(false) => {
            append_message_str(&nickname, "operation not implemented");
            -1
        }
        Err(()) => {
            append_message_str(&nickname, "operation failed");
            -1
        }
    }
}
