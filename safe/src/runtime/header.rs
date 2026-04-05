use std::ffi::{c_void, CStr, CString};
use std::ptr;

use gobject_sys::{G_TYPE_DOUBLE, G_TYPE_INT, G_TYPE_STRING};

use crate::abi::image::*;
use crate::abi::r#type::*;
use crate::runtime::error::append_message_str;
use crate::runtime::image::{format_sizeof, image_state};

pub(crate) enum MetaValue {
    Int(i32),
    Double(f64),
    String(CString),
    Area(*mut VipsArea),
    Blob(*mut VipsBlob),
    Image(*mut VipsImage),
    ArrayInt(*mut VipsArrayInt),
    ArrayDouble(*mut VipsArrayDouble),
}

impl Drop for MetaValue {
    fn drop(&mut self) {
        match self {
            MetaValue::Area(area) => crate::runtime::r#type::vips_area_unref(*area),
            MetaValue::Blob(blob) => {
                crate::runtime::r#type::vips_area_unref((*blob).cast::<VipsArea>())
            }
            MetaValue::Image(image) => unsafe { crate::runtime::object::object_unref(*image) },
            MetaValue::ArrayInt(array) => {
                crate::runtime::r#type::vips_area_unref((*array).cast::<VipsArea>())
            }
            MetaValue::ArrayDouble(array) => {
                crate::runtime::r#type::vips_area_unref((*array).cast::<VipsArea>())
            }
            _ => {}
        }
    }
}

impl MetaValue {
    fn gtype(&self) -> glib_sys::GType {
        match self {
            MetaValue::Int(_) => G_TYPE_INT,
            MetaValue::Double(_) => G_TYPE_DOUBLE,
            MetaValue::String(_) => G_TYPE_STRING,
            MetaValue::Area(_) => crate::runtime::r#type::vips_area_get_type(),
            MetaValue::Blob(_) => crate::runtime::r#type::vips_blob_get_type(),
            MetaValue::Image(_) => crate::runtime::object::vips_image_get_type(),
            MetaValue::ArrayInt(_) => crate::runtime::r#type::vips_array_int_get_type(),
            MetaValue::ArrayDouble(_) => crate::runtime::r#type::vips_array_double_get_type(),
        }
    }

    fn clone_value(&self) -> MetaValue {
        match self {
            MetaValue::Int(value) => MetaValue::Int(*value),
            MetaValue::Double(value) => MetaValue::Double(*value),
            MetaValue::String(value) => MetaValue::String(value.clone()),
            MetaValue::Area(area) => {
                crate::runtime::r#type::vips_area_copy(*area);
                MetaValue::Area(*area)
            }
            MetaValue::Blob(blob) => {
                crate::runtime::r#type::vips_area_copy((*blob).cast::<VipsArea>());
                MetaValue::Blob(*blob)
            }
            MetaValue::Image(image) => {
                MetaValue::Image(unsafe { crate::runtime::object::object_ref(*image) })
            }
            MetaValue::ArrayInt(array) => {
                crate::runtime::r#type::vips_area_copy((*array).cast::<VipsArea>());
                MetaValue::ArrayInt(*array)
            }
            MetaValue::ArrayDouble(array) => {
                crate::runtime::r#type::vips_area_copy((*array).cast::<VipsArea>());
                MetaValue::ArrayDouble(*array)
            }
        }
    }

    fn to_gvalue(&self, out: *mut gobject_sys::GValue) {
        unsafe {
            match self {
                MetaValue::Int(value) => {
                    gobject_sys::g_value_init(out, G_TYPE_INT);
                    gobject_sys::g_value_set_int(out, *value);
                }
                MetaValue::Double(value) => {
                    gobject_sys::g_value_init(out, G_TYPE_DOUBLE);
                    gobject_sys::g_value_set_double(out, *value);
                }
                MetaValue::String(value) => {
                    gobject_sys::g_value_init(out, G_TYPE_STRING);
                    gobject_sys::g_value_set_string(out, value.as_ptr());
                }
                MetaValue::Area(area) => {
                    gobject_sys::g_value_init(out, crate::runtime::r#type::vips_area_get_type());
                    gobject_sys::g_value_set_boxed(out, (*area).cast::<c_void>());
                }
                MetaValue::Blob(blob) => {
                    gobject_sys::g_value_init(out, crate::runtime::r#type::vips_blob_get_type());
                    gobject_sys::g_value_set_boxed(out, (*blob).cast::<c_void>());
                }
                MetaValue::Image(image) => {
                    gobject_sys::g_value_init(out, crate::runtime::object::vips_image_get_type());
                    gobject_sys::g_value_set_object(out, (*image).cast());
                }
                MetaValue::ArrayInt(array) => {
                    gobject_sys::g_value_init(
                        out,
                        crate::runtime::r#type::vips_array_int_get_type(),
                    );
                    gobject_sys::g_value_set_boxed(out, (*array).cast::<c_void>());
                }
                MetaValue::ArrayDouble(array) => {
                    gobject_sys::g_value_init(
                        out,
                        crate::runtime::r#type::vips_array_double_get_type(),
                    );
                    gobject_sys::g_value_set_boxed(out, (*array).cast::<c_void>());
                }
            }
        }
    }
}

#[derive(Default)]
pub(crate) struct MetaStore {
    entries: Vec<(CString, MetaValue)>,
}

impl MetaStore {
    fn set(&mut self, name: &CStr, value: MetaValue) {
        if let Some((_, existing)) = self
            .entries
            .iter_mut()
            .find(|(key, _)| key.as_c_str() == name)
        {
            *existing = value;
            return;
        }
        self.entries.push((name.to_owned(), value));
    }

    fn get(&self, name: &CStr) -> Option<&MetaValue> {
        self.entries
            .iter()
            .find(|(key, _)| key.as_c_str() == name)
            .map(|(_, value)| value)
    }

    fn remove(&mut self, name: &CStr) -> bool {
        if let Some(index) = self
            .entries
            .iter()
            .position(|(key, _)| key.as_c_str() == name)
        {
            self.entries.remove(index);
            true
        } else {
            false
        }
    }
}

const BUILTIN_FIELDS: &[&str] = &[
    "width",
    "height",
    "bands",
    "format",
    "coding",
    "interpretation",
    "xres",
    "yres",
    "xoffset",
    "yoffset",
    "filename",
    "mode",
];

pub(crate) fn builtin_type(name: &CStr) -> Option<glib_sys::GType> {
    match name.to_bytes() {
        b"width" | b"height" | b"bands" | b"xoffset" | b"yoffset" => Some(G_TYPE_INT),
        b"format" => Some(crate::runtime::r#type::vips_band_format_get_type()),
        b"coding" => Some(crate::runtime::r#type::vips_coding_get_type()),
        b"interpretation" => Some(crate::runtime::r#type::vips_interpretation_get_type()),
        b"xres" | b"yres" => Some(G_TYPE_DOUBLE),
        b"filename" | b"mode" => Some(G_TYPE_STRING),
        _ => None,
    }
}

pub(crate) unsafe fn builtin_get(
    image: *mut VipsImage,
    name: &CStr,
    value: *mut gobject_sys::GValue,
) -> bool {
    let Some(image_ref) = (unsafe { image.as_ref() }) else {
        return false;
    };
    let init_if_needed = |value: *mut gobject_sys::GValue, gtype: glib_sys::GType| unsafe {
        if (*value).g_type == 0 {
            gobject_sys::g_value_init(value, gtype);
        }
    };
    unsafe {
        match name.to_bytes() {
            b"width" => {
                init_if_needed(value, G_TYPE_INT);
                gobject_sys::g_value_set_int(value, image_ref.Xsize);
            }
            b"height" => {
                init_if_needed(value, G_TYPE_INT);
                gobject_sys::g_value_set_int(value, image_ref.Ysize);
            }
            b"bands" => {
                init_if_needed(value, G_TYPE_INT);
                gobject_sys::g_value_set_int(value, image_ref.Bands);
            }
            b"format" => {
                init_if_needed(value, crate::runtime::r#type::vips_band_format_get_type());
                gobject_sys::g_value_set_enum(value, image_ref.BandFmt);
            }
            b"coding" => {
                init_if_needed(value, crate::runtime::r#type::vips_coding_get_type());
                gobject_sys::g_value_set_enum(value, image_ref.Coding);
            }
            b"interpretation" => {
                init_if_needed(
                    value,
                    crate::runtime::r#type::vips_interpretation_get_type(),
                );
                gobject_sys::g_value_set_enum(value, image_ref.Type);
            }
            b"xres" => {
                init_if_needed(value, G_TYPE_DOUBLE);
                gobject_sys::g_value_set_double(value, image_ref.Xres);
            }
            b"yres" => {
                init_if_needed(value, G_TYPE_DOUBLE);
                gobject_sys::g_value_set_double(value, image_ref.Yres);
            }
            b"xoffset" => {
                init_if_needed(value, G_TYPE_INT);
                gobject_sys::g_value_set_int(value, image_ref.Xoffset);
            }
            b"yoffset" => {
                init_if_needed(value, G_TYPE_INT);
                gobject_sys::g_value_set_int(value, image_ref.Yoffset);
            }
            b"filename" => {
                init_if_needed(value, G_TYPE_STRING);
                gobject_sys::g_value_set_string(value, image_ref.filename);
            }
            b"mode" => {
                init_if_needed(value, G_TYPE_STRING);
                gobject_sys::g_value_set_string(value, image_ref.mode);
            }
            _ => return false,
        }
    }
    true
}

pub(crate) unsafe fn builtin_set(
    image: *mut VipsImage,
    name: &CStr,
    value: *mut gobject_sys::GValue,
) -> bool {
    let Some(image_ref) = (unsafe { image.as_mut() }) else {
        return false;
    };
    match name.to_bytes() {
        b"width" => image_ref.Xsize = unsafe { gobject_sys::g_value_get_int(value) },
        b"height" => image_ref.Ysize = unsafe { gobject_sys::g_value_get_int(value) },
        b"bands" => image_ref.Bands = unsafe { gobject_sys::g_value_get_int(value) },
        b"format" => image_ref.BandFmt = unsafe { gobject_sys::g_value_get_enum(value) },
        b"coding" => image_ref.Coding = unsafe { gobject_sys::g_value_get_enum(value) },
        b"interpretation" => image_ref.Type = unsafe { gobject_sys::g_value_get_enum(value) },
        b"xres" => image_ref.Xres = unsafe { gobject_sys::g_value_get_double(value) },
        b"yres" => image_ref.Yres = unsafe { gobject_sys::g_value_get_double(value) },
        b"xoffset" => image_ref.Xoffset = unsafe { gobject_sys::g_value_get_int(value) },
        b"yoffset" => image_ref.Yoffset = unsafe { gobject_sys::g_value_get_int(value) },
        b"filename" => crate::runtime::image::set_filename(
            image,
            if unsafe { gobject_sys::g_value_get_string(value) }.is_null() {
                None
            } else {
                Some(unsafe { CStr::from_ptr(gobject_sys::g_value_get_string(value)) })
            },
        ),
        b"mode" => {
            let ptr = unsafe { gobject_sys::g_value_get_string(value) };
            if !ptr.is_null() {
                crate::runtime::image::set_mode(
                    image,
                    unsafe { CStr::from_ptr(ptr) }.to_str().unwrap_or("p"),
                );
            }
        }
        _ => return false,
    }
    true
}

unsafe fn meta_from_value(value: *mut gobject_sys::GValue) -> Option<MetaValue> {
    let ty = unsafe { (*value).g_type };
    if ty == G_TYPE_INT {
        Some(MetaValue::Int(unsafe {
            gobject_sys::g_value_get_int(value)
        }))
    } else if ty == G_TYPE_DOUBLE {
        Some(MetaValue::Double(unsafe {
            gobject_sys::g_value_get_double(value)
        }))
    } else if ty == G_TYPE_STRING {
        let ptr = unsafe { gobject_sys::g_value_get_string(value) };
        Some(MetaValue::String(if ptr.is_null() {
            CString::new("").expect("empty")
        } else {
            unsafe { CStr::from_ptr(ptr) }.to_owned()
        }))
    } else if ty == crate::runtime::r#type::vips_area_get_type() {
        let area = unsafe { gobject_sys::g_value_get_boxed(value).cast::<VipsArea>() };
        crate::runtime::r#type::vips_area_copy(area);
        Some(MetaValue::Area(area))
    } else if ty == crate::runtime::r#type::vips_blob_get_type() {
        let blob = unsafe { gobject_sys::g_value_get_boxed(value).cast::<VipsBlob>() };
        crate::runtime::r#type::vips_area_copy(blob.cast::<VipsArea>());
        Some(MetaValue::Blob(blob))
    } else if ty == crate::runtime::object::vips_image_get_type() {
        let image = unsafe { gobject_sys::g_value_get_object(value).cast::<VipsImage>() };
        Some(MetaValue::Image(unsafe {
            crate::runtime::object::object_ref(image)
        }))
    } else if ty == crate::runtime::r#type::vips_array_int_get_type() {
        let array = unsafe { gobject_sys::g_value_get_boxed(value).cast::<VipsArrayInt>() };
        crate::runtime::r#type::vips_area_copy(array.cast::<VipsArea>());
        Some(MetaValue::ArrayInt(array))
    } else if ty == crate::runtime::r#type::vips_array_double_get_type() {
        let array = unsafe { gobject_sys::g_value_get_boxed(value).cast::<VipsArrayDouble>() };
        crate::runtime::r#type::vips_area_copy(array.cast::<VipsArea>());
        Some(MetaValue::ArrayDouble(array))
    } else {
        None
    }
}

pub(crate) fn copy_metadata(dst: *mut VipsImage, src: *mut VipsImage) {
    let Some(src_state) = (unsafe { image_state(src) }) else {
        return;
    };
    let Some(dst_state) = (unsafe { image_state(dst) }) else {
        return;
    };
    let snapshot: Vec<(CString, MetaValue)> = src_state
        .meta
        .lock()
        .expect("meta store")
        .entries
        .iter()
        .map(|(name, value)| (name.clone(), value.clone_value()))
        .collect();
    dst_state.meta.lock().expect("meta store").entries = snapshot;
}

pub(crate) fn snapshot_metadata_entries(image: *mut VipsImage) -> Vec<(CString, MetaValue)> {
    let Some(state) = (unsafe { image_state(image) }) else {
        return Vec::new();
    };
    state
        .meta
        .lock()
        .expect("meta store")
        .entries
        .iter()
        .map(|(name, value)| (name.clone(), value.clone_value()))
        .collect()
}

pub(crate) fn snapshot_save_string_metadata(image: *mut VipsImage) -> Vec<(String, String, String)> {
    let mut serialized = Vec::new();
    for (name, value) in snapshot_metadata_entries(image) {
        if name.as_c_str().to_bytes() == b"vips-loader" {
            continue;
        }
        let field_name = name.to_string_lossy().into_owned();
        match &value {
            MetaValue::Int(value) => {
                serialized.push((field_name, "int".to_owned(), value.to_string()));
            }
            MetaValue::String(value) => {
                serialized.push((
                    field_name,
                    "string".to_owned(),
                    value.to_string_lossy().into_owned(),
                ));
            }
            MetaValue::Blob(blob) => {
                let area = unsafe { &(**blob).area };
                let encoded = if area.data.is_null() || area.length == 0 {
                    String::new()
                } else {
                    let encoded = unsafe { glib_sys::g_base64_encode(area.data.cast::<u8>(), area.length) };
                    let text = unsafe { CStr::from_ptr(encoded) }
                        .to_string_lossy()
                        .into_owned();
                    unsafe {
                        glib_sys::g_free(encoded.cast());
                    }
                    text
                };
                serialized.push((field_name, "blob".to_owned(), encoded));
            }
            _ => {}
        }
    }

    serialized
}

pub(crate) fn install_save_string_metadata(
    image: *mut VipsImage,
    name: &str,
    type_name: &str,
    save_string: &str,
) -> Result<(), ()> {
    let name = CString::new(name).map_err(|_| ())?;
    match type_name {
        "int" => {
            let value = save_string.parse::<i32>().map_err(|_| ())?;
            vips_image_set_int(image, name.as_ptr(), value);
        }
        "string" => {
            let value = CString::new(save_string).map_err(|_| ())?;
            vips_image_set_string(image, name.as_ptr(), value.as_ptr());
        }
        "blob" => {
            let save_string = CString::new(save_string).map_err(|_| ())?;
            let mut length = 0usize;
            let data = unsafe { glib_sys::g_base64_decode(save_string.as_ptr(), &mut length) };
            vips_image_set_blob_copy(image, name.as_ptr(), data.cast::<c_void>(), length);
            unsafe {
                glib_sys::g_free(data.cast());
            }
        }
        _ => {}
    }
    Ok(())
}

#[no_mangle]
pub extern "C" fn vips_format_sizeof(format: VipsBandFormat) -> u64 {
    format_sizeof(format) as u64
}

#[no_mangle]
pub extern "C" fn vips_format_sizeof_unsafe(format: VipsBandFormat) -> u64 {
    vips_format_sizeof(format)
}

#[no_mangle]
pub extern "C" fn vips_interpretation_max_alpha(interpretation: VipsInterpretation) -> f64 {
    match interpretation {
        crate::abi::image::VIPS_INTERPRETATION_GREY16
        | crate::abi::image::VIPS_INTERPRETATION_RGB16 => 65535.0,
        _ => 255.0,
    }
}

#[no_mangle]
pub extern "C" fn vips_image_get_width(image: *const VipsImage) -> i32 {
    unsafe { image.as_ref() }.map_or(0, |image| image.Xsize)
}

#[no_mangle]
pub extern "C" fn vips_image_get_height(image: *const VipsImage) -> i32 {
    unsafe { image.as_ref() }.map_or(0, |image| image.Ysize)
}

#[no_mangle]
pub extern "C" fn vips_image_get_bands(image: *const VipsImage) -> i32 {
    unsafe { image.as_ref() }.map_or(0, |image| image.Bands)
}

#[no_mangle]
pub extern "C" fn vips_image_get_format(image: *const VipsImage) -> VipsBandFormat {
    unsafe { image.as_ref() }.map_or(VIPS_FORMAT_NOTSET, |image| image.BandFmt)
}

#[no_mangle]
pub extern "C" fn vips_image_get_format_max(format: VipsBandFormat) -> f64 {
    match format {
        VIPS_FORMAT_UCHAR => u8::MAX as f64,
        VIPS_FORMAT_USHORT => u16::MAX as f64,
        VIPS_FORMAT_UINT => u32::MAX as f64,
        VIPS_FORMAT_CHAR => i8::MAX as f64,
        VIPS_FORMAT_SHORT => i16::MAX as f64,
        VIPS_FORMAT_INT => i32::MAX as f64,
        _ => 1.0,
    }
}

#[no_mangle]
pub extern "C" fn vips_image_guess_format(image: *const VipsImage) -> VipsBandFormat {
    vips_image_get_format(image)
}

#[no_mangle]
pub extern "C" fn vips_image_get_coding(image: *const VipsImage) -> VipsCoding {
    unsafe { image.as_ref() }.map_or(VIPS_CODING_NONE, |image| image.Coding)
}

#[no_mangle]
pub extern "C" fn vips_image_get_interpretation(image: *const VipsImage) -> VipsInterpretation {
    unsafe { image.as_ref() }.map_or(VIPS_INTERPRETATION_MULTIBAND, |image| image.Type)
}

#[no_mangle]
pub extern "C" fn vips_image_guess_interpretation(image: *const VipsImage) -> VipsInterpretation {
    vips_image_get_interpretation(image)
}

#[no_mangle]
pub extern "C" fn vips_image_get_xres(image: *const VipsImage) -> f64 {
    unsafe { image.as_ref() }.map_or(1.0, |image| image.Xres)
}

#[no_mangle]
pub extern "C" fn vips_image_get_yres(image: *const VipsImage) -> f64 {
    unsafe { image.as_ref() }.map_or(1.0, |image| image.Yres)
}

#[no_mangle]
pub extern "C" fn vips_image_get_xoffset(image: *const VipsImage) -> i32 {
    unsafe { image.as_ref() }.map_or(0, |image| image.Xoffset)
}

#[no_mangle]
pub extern "C" fn vips_image_get_yoffset(image: *const VipsImage) -> i32 {
    unsafe { image.as_ref() }.map_or(0, |image| image.Yoffset)
}

#[no_mangle]
pub extern "C" fn vips_image_get_filename(image: *const VipsImage) -> *const libc::c_char {
    unsafe { image.as_ref() }.map_or(ptr::null(), |image| image.filename.cast_const())
}

#[no_mangle]
pub extern "C" fn vips_image_get_mode(image: *const VipsImage) -> *const libc::c_char {
    unsafe { image.as_ref() }.map_or(ptr::null(), |image| image.mode.cast_const())
}

#[no_mangle]
pub extern "C" fn vips_image_get_scale(_image: *const VipsImage) -> f64 {
    1.0
}

#[no_mangle]
pub extern "C" fn vips_image_get_offset(_image: *const VipsImage) -> f64 {
    0.0
}

#[no_mangle]
pub extern "C" fn vips_image_get_page_height(image: *mut VipsImage) -> i32 {
    let mut value = 0;
    if vips_image_get_int(image, c"page-height".as_ptr(), &mut value) == 0 && value > 0 {
        value
    } else {
        unsafe { image.as_ref() }.map_or(0, |image| image.Ysize)
    }
}

#[no_mangle]
pub extern "C" fn vips_image_get_n_pages(image: *mut VipsImage) -> i32 {
    let mut value = 1;
    if vips_image_get_int(image, c"n-pages".as_ptr(), &mut value) == 0 && value > 0 {
        value
    } else {
        1
    }
}

#[no_mangle]
pub extern "C" fn vips_image_get_n_subifds(image: *mut VipsImage) -> i32 {
    let mut value = 0;
    if vips_image_get_int(image, c"n-subifds".as_ptr(), &mut value) == 0 {
        value
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn vips_image_get_orientation(image: *mut VipsImage) -> i32 {
    let mut value = 1;
    if vips_image_get_int(image, c"orientation".as_ptr(), &mut value) == 0 && value > 0 {
        value
    } else {
        1
    }
}

#[no_mangle]
pub extern "C" fn vips_image_get_orientation_swap(image: *mut VipsImage) -> glib_sys::gboolean {
    let orientation = vips_image_get_orientation(image);
    if matches!(orientation, 5 | 6 | 7 | 8) {
        glib_sys::GTRUE
    } else {
        glib_sys::GFALSE
    }
}

#[no_mangle]
pub extern "C" fn vips_image_get_concurrency(
    _image: *mut VipsImage,
    default_concurrency: i32,
) -> i32 {
    default_concurrency
}

#[no_mangle]
pub extern "C" fn vips_image_get_data(image: *mut VipsImage) -> *const c_void {
    let _ = crate::runtime::image::ensure_pixels(image);
    unsafe { image.as_ref() }.map_or(ptr::null(), |image| image.data.cast::<c_void>())
}

#[no_mangle]
pub extern "C" fn vips_image_init_fields(
    image: *mut VipsImage,
    xsize: i32,
    ysize: i32,
    bands: i32,
    format: VipsBandFormat,
    coding: VipsCoding,
    interpretation: VipsInterpretation,
    xres: f64,
    yres: f64,
) {
    let Some(image) = (unsafe { image.as_mut() }) else {
        return;
    };
    image.Xsize = xsize.max(0);
    image.Ysize = ysize.max(0);
    image.Bands = bands.max(0);
    image.BandFmt = format;
    image.Coding = coding;
    image.Type = interpretation;
    image.Xres = xres.max(0.0);
    image.Yres = yres.max(0.0);
    image.Xres_float = image.Xres as f32;
    image.Yres_float = image.Yres as f32;
    image.Bbits = (format_sizeof(format) * 8) as i32;
}

#[no_mangle]
pub extern "C" fn vips_image_set(
    image: *mut VipsImage,
    name: *const libc::c_char,
    value: *mut gobject_sys::GValue,
) {
    let Some(name) = (!name.is_null()).then(|| unsafe { CStr::from_ptr(name) }) else {
        return;
    };
    if unsafe { builtin_set(image, name, value) } {
        return;
    }
    let Some(meta_value) = (unsafe { meta_from_value(value) }) else {
        append_message_str("vips_image_set", "unsupported metadata type");
        return;
    };
    if let Some(state) = unsafe { image_state(image) } {
        state.meta.lock().expect("meta").set(name, meta_value);
    }
}

#[no_mangle]
pub extern "C" fn vips_image_get(
    image: *const VipsImage,
    name: *const libc::c_char,
    value_copy: *mut gobject_sys::GValue,
) -> libc::c_int {
    let Some(name) = (!name.is_null()).then(|| unsafe { CStr::from_ptr(name) }) else {
        return -1;
    };
    if unsafe { builtin_get(image.cast_mut(), name, value_copy) } {
        return 0;
    }
    let Some(state) = (unsafe { image_state(image.cast_mut()) }) else {
        return -1;
    };
    if let Some(value) = state.meta.lock().expect("meta").get(name) {
        value.to_gvalue(value_copy);
        0
    } else {
        append_message_str("vips_image_get", "field not found");
        -1
    }
}

#[no_mangle]
pub extern "C" fn vips_image_get_as_string(
    image: *const VipsImage,
    name: *const libc::c_char,
    out: *mut *mut libc::c_char,
) -> libc::c_int {
    let mut blob = ptr::null();
    let mut blob_len = 0usize;
    if vips_image_get_blob(image, name, &mut blob, &mut blob_len) == 0 {
        let text = if blob.is_null() || blob_len == 0 {
            unsafe { glib_sys::g_strdup(c"".as_ptr()) }
        } else {
            unsafe { glib_sys::g_base64_encode(blob.cast::<u8>(), blob_len) }
        };
        unsafe {
            if !out.is_null() {
                *out = text;
            }
        }
        return 0;
    }
    let mut value: gobject_sys::GValue = unsafe { std::mem::zeroed() };
    if vips_image_get(image, name, &mut value) != 0 {
        return -1;
    }
    let text = unsafe { gobject_sys::g_strdup_value_contents(&value) };
    unsafe {
        if !out.is_null() {
            *out = text;
        }
        gobject_sys::g_value_unset(&mut value);
    }
    0
}

#[no_mangle]
pub extern "C" fn vips_image_get_typeof(
    image: *const VipsImage,
    name: *const libc::c_char,
) -> glib_sys::GType {
    let Some(name) = (!name.is_null()).then(|| unsafe { CStr::from_ptr(name) }) else {
        return 0;
    };
    if let Some(ty) = builtin_type(name) {
        return ty;
    }
    unsafe { image_state(image.cast_mut()) }
        .and_then(|state| state.meta.lock().ok()?.get(name).map(MetaValue::gtype))
        .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn vips_image_remove(
    image: *mut VipsImage,
    name: *const libc::c_char,
) -> glib_sys::gboolean {
    let Some(name) = (!name.is_null()).then(|| unsafe { CStr::from_ptr(name) }) else {
        return glib_sys::GFALSE;
    };
    if let Some(state) = unsafe { image_state(image) } {
        if state.meta.lock().expect("meta").remove(name) {
            return glib_sys::GTRUE;
        }
    }
    glib_sys::GFALSE
}

type VipsImageMapFn = Option<
    unsafe extern "C" fn(
        image: *mut VipsImage,
        name: *const libc::c_char,
        value: *mut gobject_sys::GValue,
        a: *mut c_void,
    ) -> *mut c_void,
>;

#[no_mangle]
pub extern "C" fn vips_image_map(
    image: *mut VipsImage,
    fn_: VipsImageMapFn,
    a: *mut c_void,
) -> *mut c_void {
    let Some(fn_) = fn_ else {
        return ptr::null_mut();
    };
    for field in BUILTIN_FIELDS {
        let mut value: gobject_sys::GValue = unsafe { std::mem::zeroed() };
        let name = CString::new(*field).expect("field");
        let _ = unsafe { builtin_get(image, name.as_c_str(), &mut value) };
        let result = unsafe { fn_(image, name.as_ptr(), &mut value, a) };
        unsafe {
            gobject_sys::g_value_unset(&mut value);
        }
        if !result.is_null() {
            return result;
        }
    }
    if let Some(state) = unsafe { image_state(image) } {
        for (name, meta) in &state.meta.lock().expect("meta").entries {
            let mut value: gobject_sys::GValue = unsafe { std::mem::zeroed() };
            meta.to_gvalue(&mut value);
            let result = unsafe { fn_(image, name.as_ptr(), &mut value, a) };
            unsafe {
                gobject_sys::g_value_unset(&mut value);
            }
            if !result.is_null() {
                return result;
            }
        }
    }
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn vips_image_get_fields(image: *mut VipsImage) -> *mut *mut libc::c_char {
    let mut names: Vec<CString> = BUILTIN_FIELDS
        .iter()
        .map(|field| CString::new(*field).expect("field"))
        .collect();
    if let Some(state) = unsafe { image_state(image) } {
        names.extend(
            state
                .meta
                .lock()
                .expect("meta")
                .entries
                .iter()
                .map(|(name, _)| name.clone()),
        );
    }
    let out = unsafe {
        glib_sys::g_malloc0((names.len() + 1) * std::mem::size_of::<*mut libc::c_char>())
    }
    .cast::<*mut libc::c_char>();
    for (index, name) in names.into_iter().enumerate() {
        unsafe {
            *out.add(index) = glib_sys::g_strdup(name.as_ptr());
        }
    }
    out
}

#[no_mangle]
pub extern "C" fn vips_image_set_area(
    image: *mut VipsImage,
    name: *const libc::c_char,
    free_fn: crate::abi::basic::VipsCallbackFn,
    data: *mut c_void,
) {
    let Some(name) = (!name.is_null()).then(|| unsafe { CStr::from_ptr(name) }) else {
        return;
    };
    let area = crate::runtime::r#type::vips_area_new(free_fn, data);
    if let Some(state) = unsafe { image_state(image) } {
        state
            .meta
            .lock()
            .expect("meta")
            .set(name, MetaValue::Area(area));
    }
}

#[no_mangle]
pub extern "C" fn vips_image_get_area(
    image: *const VipsImage,
    name: *const libc::c_char,
    data: *mut *const c_void,
) -> libc::c_int {
    let Some(name) = (!name.is_null()).then(|| unsafe { CStr::from_ptr(name) }) else {
        return -1;
    };
    let Some(state) = (unsafe { image_state(image.cast_mut()) }) else {
        return -1;
    };
    if let Some(MetaValue::Area(area)) = state.meta.lock().expect("meta").get(name) {
        unsafe {
            if !data.is_null() {
                *data = (**area).data.cast::<c_void>();
            }
        }
        0
    } else {
        -1
    }
}

#[no_mangle]
pub extern "C" fn vips_image_set_blob(
    image: *mut VipsImage,
    name: *const libc::c_char,
    free_fn: crate::abi::basic::VipsCallbackFn,
    data: *const c_void,
    length: usize,
) {
    let Some(name) = (!name.is_null()).then(|| unsafe { CStr::from_ptr(name) }) else {
        return;
    };
    let blob = crate::runtime::r#type::vips_blob_new(free_fn, data, length);
    if let Some(state) = unsafe { image_state(image) } {
        state
            .meta
            .lock()
            .expect("meta")
            .set(name, MetaValue::Blob(blob));
    }
}

#[no_mangle]
pub extern "C" fn vips_image_set_blob_copy(
    image: *mut VipsImage,
    name: *const libc::c_char,
    data: *const c_void,
    length: usize,
) {
    if data.is_null() || length == 0 {
        return;
    }
    let blob = crate::runtime::r#type::vips_blob_copy(data, length);
    let Some(name) = (!name.is_null()).then(|| unsafe { CStr::from_ptr(name) }) else {
        crate::runtime::r#type::vips_area_unref(blob.cast::<VipsArea>());
        return;
    };
    if let Some(state) = unsafe { image_state(image) } {
        state
            .meta
            .lock()
            .expect("meta")
            .set(name, MetaValue::Blob(blob));
    }
}

#[no_mangle]
pub extern "C" fn vips_image_get_blob(
    image: *const VipsImage,
    name: *const libc::c_char,
    data: *mut *const c_void,
    length: *mut usize,
) -> libc::c_int {
    let Some(name) = (!name.is_null()).then(|| unsafe { CStr::from_ptr(name) }) else {
        return -1;
    };
    let Some(state) = (unsafe { image_state(image.cast_mut()) }) else {
        return -1;
    };
    if let Some(MetaValue::Blob(blob)) = state.meta.lock().expect("meta").get(name) {
        unsafe {
            if !data.is_null() {
                *data = (**blob).area.data.cast::<c_void>();
            }
            if !length.is_null() {
                *length = (**blob).area.length;
            }
        }
        0
    } else {
        -1
    }
}

#[no_mangle]
pub extern "C" fn vips_image_get_int(
    image: *const VipsImage,
    name: *const libc::c_char,
    out: *mut i32,
) -> libc::c_int {
    let mut value: gobject_sys::GValue = unsafe { std::mem::zeroed() };
    if vips_image_get(image, name, &mut value) != 0 {
        return -1;
    }
    unsafe {
        if !out.is_null() {
            *out = gobject_sys::g_value_get_int(&value);
        }
        gobject_sys::g_value_unset(&mut value);
    }
    0
}

#[no_mangle]
pub extern "C" fn vips_image_set_int(image: *mut VipsImage, name: *const libc::c_char, value: i32) {
    let mut gvalue: gobject_sys::GValue = unsafe { std::mem::zeroed() };
    unsafe {
        gobject_sys::g_value_init(&mut gvalue, G_TYPE_INT);
        gobject_sys::g_value_set_int(&mut gvalue, value);
    }
    vips_image_set(image, name, &mut gvalue);
    unsafe {
        gobject_sys::g_value_unset(&mut gvalue);
    }
}

#[no_mangle]
pub extern "C" fn vips_image_get_double(
    image: *const VipsImage,
    name: *const libc::c_char,
    out: *mut f64,
) -> libc::c_int {
    let mut value: gobject_sys::GValue = unsafe { std::mem::zeroed() };
    if vips_image_get(image, name, &mut value) != 0 {
        return -1;
    }
    unsafe {
        if !out.is_null() {
            *out = gobject_sys::g_value_get_double(&value);
        }
        gobject_sys::g_value_unset(&mut value);
    }
    0
}

#[no_mangle]
pub extern "C" fn vips_image_set_double(
    image: *mut VipsImage,
    name: *const libc::c_char,
    value: f64,
) {
    let mut gvalue: gobject_sys::GValue = unsafe { std::mem::zeroed() };
    unsafe {
        gobject_sys::g_value_init(&mut gvalue, G_TYPE_DOUBLE);
        gobject_sys::g_value_set_double(&mut gvalue, value);
    }
    vips_image_set(image, name, &mut gvalue);
    unsafe {
        gobject_sys::g_value_unset(&mut gvalue);
    }
}

#[no_mangle]
pub extern "C" fn vips_image_get_string(
    image: *const VipsImage,
    name: *const libc::c_char,
    out: *mut *const libc::c_char,
) -> libc::c_int {
    let Some(name) = (!name.is_null()).then(|| unsafe { CStr::from_ptr(name) }) else {
        return -1;
    };
    if name.to_bytes() == b"filename" {
        unsafe {
            if !out.is_null() {
                *out = glib_sys::g_strdup(vips_image_get_filename(image));
            }
        }
        return 0;
    }
    if name.to_bytes() == b"mode" {
        unsafe {
            if !out.is_null() {
                *out = glib_sys::g_strdup(vips_image_get_mode(image));
            }
        }
        return 0;
    }
    let Some(state) = (unsafe { image_state(image.cast_mut()) }) else {
        return -1;
    };
    if let Some(MetaValue::String(value)) = state.meta.lock().expect("meta").get(name) {
        unsafe {
            if !out.is_null() {
                *out = glib_sys::g_strdup(value.as_ptr());
            }
        }
        0
    } else {
        -1
    }
}

#[no_mangle]
pub extern "C" fn vips_image_set_string(
    image: *mut VipsImage,
    name: *const libc::c_char,
    str_: *const libc::c_char,
) {
    let Some(name) = (!name.is_null()).then(|| unsafe { CStr::from_ptr(name) }) else {
        return;
    };
    if name.to_bytes() == b"filename" {
        crate::runtime::image::set_filename(
            image,
            (!str_.is_null()).then(|| unsafe { CStr::from_ptr(str_) }),
        );
        return;
    }
    if name.to_bytes() == b"mode" {
        if !str_.is_null() {
            crate::runtime::image::set_mode(
                image,
                unsafe { CStr::from_ptr(str_) }.to_str().unwrap_or("p"),
            );
        }
        return;
    }
    if let Some(state) = unsafe { image_state(image) } {
        let text = if str_.is_null() {
            CString::new("").expect("empty")
        } else {
            unsafe { CStr::from_ptr(str_) }.to_owned()
        };
        state
            .meta
            .lock()
            .expect("meta")
            .set(name, MetaValue::String(text));
    }
}

#[no_mangle]
pub extern "C" fn vips_image_print_field(image: *const VipsImage, name: *const libc::c_char) {
    let mut out = ptr::null_mut();
    if vips_image_get_as_string(image, name, &mut out) == 0 && !out.is_null() {
        unsafe {
            libc::puts(out);
            glib_sys::g_free(out.cast::<c_void>());
        }
    }
}

#[no_mangle]
pub extern "C" fn vips_image_get_image(
    image: *const VipsImage,
    name: *const libc::c_char,
    out: *mut *mut VipsImage,
) -> libc::c_int {
    let Some(name) = (!name.is_null()).then(|| unsafe { CStr::from_ptr(name) }) else {
        return -1;
    };
    let Some(state) = (unsafe { image_state(image.cast_mut()) }) else {
        return -1;
    };
    if let Some(MetaValue::Image(value)) = state.meta.lock().expect("meta").get(name) {
        unsafe {
            if !out.is_null() {
                *out = crate::runtime::object::object_ref(*value);
            }
        }
        0
    } else {
        -1
    }
}

#[no_mangle]
pub extern "C" fn vips_image_set_image(
    image: *mut VipsImage,
    name: *const libc::c_char,
    value: *mut VipsImage,
) {
    let Some(name) = (!name.is_null()).then(|| unsafe { CStr::from_ptr(name) }) else {
        return;
    };
    if let Some(state) = unsafe { image_state(image) } {
        state.meta.lock().expect("meta").set(
            name,
            MetaValue::Image(unsafe { crate::runtime::object::object_ref(value) }),
        );
    }
}

#[no_mangle]
pub extern "C" fn vips_image_set_array_int(
    image: *mut VipsImage,
    name: *const libc::c_char,
    array: *const i32,
    n: i32,
) {
    let Some(name) = (!name.is_null()).then(|| unsafe { CStr::from_ptr(name) }) else {
        return;
    };
    let array = crate::runtime::r#type::vips_array_int_new(array, n);
    if let Some(state) = unsafe { image_state(image) } {
        state
            .meta
            .lock()
            .expect("meta")
            .set(name, MetaValue::ArrayInt(array));
    }
}

#[no_mangle]
pub extern "C" fn vips_image_get_array_int(
    image: *mut VipsImage,
    name: *const libc::c_char,
    out: *mut *mut i32,
    n: *mut i32,
) -> libc::c_int {
    let Some(name) = (!name.is_null()).then(|| unsafe { CStr::from_ptr(name) }) else {
        return -1;
    };
    let Some(state) = (unsafe { image_state(image) }) else {
        return -1;
    };
    if let Some(MetaValue::ArrayInt(array)) = state.meta.lock().expect("meta").get(name) {
        unsafe {
            if !out.is_null() {
                *out = (**array).area.data.cast::<i32>();
            }
            if !n.is_null() {
                *n = (**array).area.n;
            }
        }
        0
    } else {
        -1
    }
}

#[no_mangle]
pub extern "C" fn vips_image_get_array_double(
    image: *mut VipsImage,
    name: *const libc::c_char,
    out: *mut *mut f64,
    n: *mut i32,
) -> libc::c_int {
    let Some(name) = (!name.is_null()).then(|| unsafe { CStr::from_ptr(name) }) else {
        return -1;
    };
    let Some(state) = (unsafe { image_state(image) }) else {
        return -1;
    };
    if let Some(MetaValue::ArrayDouble(array)) = state.meta.lock().expect("meta").get(name) {
        unsafe {
            if !out.is_null() {
                *out = (**array).area.data.cast::<f64>();
            }
            if !n.is_null() {
                *n = (**array).area.n;
            }
        }
        0
    } else {
        -1
    }
}

#[no_mangle]
pub extern "C" fn vips_image_set_array_double(
    image: *mut VipsImage,
    name: *const libc::c_char,
    array: *const f64,
    n: i32,
) {
    let Some(name) = (!name.is_null()).then(|| unsafe { CStr::from_ptr(name) }) else {
        return;
    };
    let array = crate::runtime::r#type::vips_array_double_new(array, n);
    if let Some(state) = unsafe { image_state(image) } {
        state
            .meta
            .lock()
            .expect("meta")
            .set(name, MetaValue::ArrayDouble(array));
    }
}

#[no_mangle]
pub extern "C" fn vips_image_get_history(image: *mut VipsImage) -> *const libc::c_char {
    unsafe { image_state(image) }
        .and_then(|state| state.history.as_ref())
        .map_or(ptr::null(), |history| history.as_ptr())
}
