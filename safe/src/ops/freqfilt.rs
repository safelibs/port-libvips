use std::mem::size_of;
use std::sync::OnceLock;

use crate::abi::image::{
    VipsBandFormat, VIPS_FORMAT_DOUBLE, VIPS_FORMAT_DPCOMPLEX, VIPS_INTERPRETATION_B_W,
    VIPS_INTERPRETATION_FOURIER,
};
use crate::abi::object::{
    VipsObject, VipsObjectClass, VIPS_ARGUMENT_CONSTRUCT, VIPS_ARGUMENT_INPUT,
};
use crate::abi::operation::{VipsOperation, VipsOperationClass};
use crate::pixels::format::complex_component_format;
use crate::pixels::iter::pixel_index;
use crate::pixels::{
    complex_image_from_samples, read_complex_image, ComplexSample, ImageBuffer, ImageSpec,
};
use crate::runtime::error::append_message_str;
use crate::runtime::object;

use super::{argument_assigned, get_bool, get_image_ref, set_output_image, set_output_image_like};

fn complex_mul(left: ComplexSample, right: ComplexSample) -> ComplexSample {
    ComplexSample {
        real: left.real * right.real - left.imag * right.imag,
        imag: left.real * right.imag + left.imag * right.real,
    }
}

fn complex_add(left: ComplexSample, right: ComplexSample) -> ComplexSample {
    ComplexSample {
        real: left.real + right.real,
        imag: left.imag + right.imag,
    }
}

fn complex_scale(value: ComplexSample, scale: f64) -> ComplexSample {
    ComplexSample {
        real: value.real * scale,
        imag: value.imag * scale,
    }
}

fn dft2(input: &[ComplexSample], width: usize, height: usize, inverse: bool) -> Vec<ComplexSample> {
    let mut out = vec![ComplexSample::default(); width * height];
    let sign = if inverse { 1.0 } else { -1.0 };
    let norm = if inverse {
        1.0
    } else {
        1.0 / (width * height).max(1) as f64
    };

    for v in 0..height {
        for u in 0..width {
            let mut sum = ComplexSample::default();
            for y in 0..height {
                for x in 0..width {
                    let angle = sign
                        * 2.0
                        * std::f64::consts::PI
                        * ((u * x) as f64 / width.max(1) as f64
                            + (v * y) as f64 / height.max(1) as f64);
                    let (sin, cos) = angle.sin_cos();
                    let twiddle = ComplexSample {
                        real: cos,
                        imag: sin,
                    };
                    sum = complex_add(sum, complex_mul(input[y * width + x], twiddle));
                }
            }
            out[v * width + u] = complex_scale(sum, norm);
        }
    }

    out
}

fn align_complex_bands(
    spec: ImageSpec,
    data: Vec<ComplexSample>,
    bands: usize,
) -> Result<(ImageSpec, Vec<ComplexSample>), ()> {
    if spec.bands == bands {
        return Ok((spec, data));
    }
    if spec.bands != 1 || bands == 0 {
        return Err(());
    }

    let mut aligned = Vec::with_capacity(spec.width * spec.height * bands);
    for y in 0..spec.height {
        for x in 0..spec.width {
            let value = data[pixel_index(spec.width, 1, x, y, 0)];
            for _ in 0..bands {
                aligned.push(value);
            }
        }
    }

    let mut spec = spec;
    spec.bands = bands;
    Ok((spec, aligned))
}

fn output_buffer_from_spec(spec: ImageSpec, format: VipsBandFormat) -> ImageBuffer {
    let mut out = ImageBuffer::new(
        spec.width,
        spec.height,
        spec.bands,
        format,
        spec.coding,
        spec.interpretation,
    );
    out.spec.xres = spec.xres;
    out.spec.yres = spec.yres;
    out.spec.xoffset = spec.xoffset;
    out.spec.yoffset = spec.yoffset;
    out.spec.dhint = spec.dhint;
    out
}

fn ensure_mono(nickname: &str, spec: &ImageSpec) -> Result<(), ()> {
    if spec.bands == 1 {
        Ok(())
    } else {
        append_message_str(nickname, "image must have one band");
        Err(())
    }
}

unsafe fn configure_fft_class(
    klass: glib_sys::gpointer,
    nickname: *const libc::c_char,
    description: *const libc::c_char,
) {
    let class = klass.cast::<VipsObjectClass>();
    unsafe {
        object::prepare_existing_class(class);
        (*class).nickname = nickname;
        (*class).description = description;
        (*class).build = Some(super::generated_operation_build);
    }
}

unsafe fn install_invfft_real_argument(klass: glib_sys::gpointer) {
    let class = klass.cast::<VipsObjectClass>();
    let gobject_class = klass.cast::<gobject_sys::GObjectClass>();
    let pspec = unsafe {
        gobject_sys::g_param_spec_boolean(
            c"real".as_ptr(),
            c"Real".as_ptr(),
            c"Output only the real part".as_ptr(),
            glib_sys::GFALSE,
            gobject_sys::G_PARAM_READWRITE | gobject_sys::G_PARAM_CONSTRUCT,
        )
    };
    unsafe {
        gobject_sys::g_object_class_install_property(
            gobject_class,
            object::vips_argument_get_id() as u32,
            pspec,
        );
        object::vips_object_class_install_argument(
            class,
            pspec,
            VIPS_ARGUMENT_INPUT | VIPS_ARGUMENT_CONSTRUCT,
            2,
            object::DYNAMIC_ARGUMENT_OFFSET,
        );
    }
}

unsafe extern "C" fn fwfft_class_init(klass: glib_sys::gpointer, _data: glib_sys::gpointer) {
    unsafe {
        configure_fft_class(
            klass,
            c"fwfft".as_ptr(),
            c"Transform an image to Fourier space".as_ptr(),
        );
    }
}

unsafe extern "C" fn invfft_class_init(klass: glib_sys::gpointer, _data: glib_sys::gpointer) {
    unsafe {
        configure_fft_class(
            klass,
            c"invfft".as_ptr(),
            c"Transform an image from Fourier space".as_ptr(),
        );
        install_invfft_real_argument(klass);
    }
}

fn freqfilt_parent_type() -> glib_sys::GType {
    unsafe { gobject_sys::g_type_from_name(c"VipsFreqfilt".as_ptr()) }
}

fn register_fft_type(
    storage: &'static OnceLock<glib_sys::GType>,
    type_name: *const libc::c_char,
    class_init: gobject_sys::GClassInitFunc,
) -> glib_sys::GType {
    *storage.get_or_init(|| {
        let existing = unsafe { gobject_sys::g_type_from_name(type_name) };
        if existing != 0 {
            return existing;
        }
        let parent = freqfilt_parent_type();
        if parent == 0 {
            return 0;
        }
        object::register_type(
            parent,
            type_name,
            size_of::<VipsOperationClass>(),
            class_init,
            size_of::<VipsOperation>(),
            None,
            0,
        )
    })
}

pub(crate) fn try_register_operation(name: &str) -> bool {
    static FWFFT_TYPE: OnceLock<glib_sys::GType> = OnceLock::new();
    static INVFFT_TYPE: OnceLock<glib_sys::GType> = OnceLock::new();

    match name {
        "fwfft" => {
            register_fft_type(&FWFFT_TYPE, c"VipsFwfft".as_ptr(), Some(fwfft_class_init)) != 0
        }
        "invfft" => {
            register_fft_type(
                &INVFFT_TYPE,
                c"VipsInvfft".as_ptr(),
                Some(invfft_class_init),
            ) != 0
        }
        _ => false,
    }
}

fn transform_planes(spec: ImageSpec, data: &[ComplexSample], inverse: bool) -> Vec<ComplexSample> {
    let mut out = vec![ComplexSample::default(); data.len()];
    for band in 0..spec.bands {
        let mut plane = vec![ComplexSample::default(); spec.width * spec.height];
        for y in 0..spec.height {
            for x in 0..spec.width {
                let sample = pixel_index(spec.width, spec.bands, x, y, band);
                plane[y * spec.width + x] = data[sample];
            }
        }

        let transformed = dft2(&plane, spec.width, spec.height, inverse);
        for y in 0..spec.height {
            for x in 0..spec.width {
                let sample = pixel_index(spec.width, spec.bands, x, y, band);
                out[sample] = transformed[y * spec.width + x];
            }
        }
    }
    out
}

unsafe fn op_fwfft(object: *mut VipsObject) -> Result<(), ()> {
    let image = unsafe { get_image_ref(object, "in")? };
    let result = (|| {
        let (mut spec, data) = unsafe { read_complex_image(image)? };
        ensure_mono("fwfft", &spec)?;
        spec.format = VIPS_FORMAT_DPCOMPLEX;
        spec.interpretation = VIPS_INTERPRETATION_FOURIER;
        let samples = transform_planes(spec, &data, false);
        let out = unsafe { complex_image_from_samples(spec, &samples, image)? };
        unsafe { set_output_image(object, "out", out) }
    })();
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

unsafe fn op_invfft(object: *mut VipsObject) -> Result<(), ()> {
    let image = unsafe { get_image_ref(object, "in")? };
    let result = (|| {
        let (spec, data) = unsafe { read_complex_image(image)? };
        ensure_mono("invfft", &spec)?;
        let samples = transform_planes(spec, &data, true);
        let mut out_spec = spec;
        out_spec.interpretation = VIPS_INTERPRETATION_B_W;
        let real = if unsafe { argument_assigned(object, "real").unwrap_or(false) } {
            unsafe { get_bool(object, "real")? }
        } else {
            false
        };
        if real {
            let mut out = output_buffer_from_spec(out_spec, VIPS_FORMAT_DOUBLE);
            out.data = samples.into_iter().map(|sample| sample.real).collect();
            let out = out.into_image_like(image);
            unsafe { set_output_image(object, "out", out) }
        } else {
            out_spec.format = VIPS_FORMAT_DPCOMPLEX;
            let out = unsafe { complex_image_from_samples(out_spec, &samples, image)? };
            unsafe { set_output_image(object, "out", out) }
        }
    })();
    unsafe {
        crate::runtime::object::object_unref(image);
    }
    result
}

unsafe fn op_freqmult(object: *mut VipsObject) -> Result<(), ()> {
    let input_image = unsafe { get_image_ref(object, "in")? };
    let mask_image = unsafe { get_image_ref(object, "mask")? };
    let result = (|| {
        let input_format = unsafe { input_image.as_ref() }.ok_or(())?.BandFmt;
        let mask_format = unsafe { mask_image.as_ref() }.ok_or(())?.BandFmt;
        let input_complex = complex_component_format(input_format).is_some();
        let mask_complex = complex_component_format(mask_format).is_some();

        let (input_spec, input_data) = unsafe { read_complex_image(input_image)? };
        let (mask_spec, mask_data) = unsafe { read_complex_image(mask_image)? };
        if input_spec.width != mask_spec.width || input_spec.height != mask_spec.height {
            return Err(());
        }

        let target_bands = match (input_spec.bands, mask_spec.bands) {
            (left, right) if left == right => left,
            (1, right) => right,
            (left, 1) => left,
            _ => return Err(()),
        };
        let (input_spec, input_data) = align_complex_bands(input_spec, input_data, target_bands)?;
        let (_, mask_data) = align_complex_bands(mask_spec, mask_data, target_bands)?;

        let out_format = if input_complex {
            complex_component_format(input_format).unwrap_or(input_format)
        } else {
            input_format
        };
        let mut out = output_buffer_from_spec(input_spec, out_format);

        for band in 0..out.spec.bands {
            let mut input_plane = vec![ComplexSample::default(); out.spec.width * out.spec.height];
            let mut mask_plane = vec![ComplexSample::default(); out.spec.width * out.spec.height];
            for y in 0..out.spec.height {
                for x in 0..out.spec.width {
                    let sample = pixel_index(out.spec.width, out.spec.bands, x, y, band);
                    let plane = y * out.spec.width + x;
                    input_plane[plane] = input_data[sample];
                    mask_plane[plane] = if mask_complex {
                        mask_data[sample]
                    } else {
                        ComplexSample {
                            real: mask_data[sample].real,
                            imag: 0.0,
                        }
                    };
                }
            }

            let spectrum = if input_complex {
                input_plane
            } else {
                dft2(&input_plane, out.spec.width, out.spec.height, false)
            };
            let filtered = spectrum
                .into_iter()
                .zip(mask_plane)
                .map(|(value, mask)| complex_mul(value, mask))
                .collect::<Vec<_>>();
            let spatial = dft2(&filtered, out.spec.width, out.spec.height, true);

            for y in 0..out.spec.height {
                for x in 0..out.spec.width {
                    let plane = y * out.spec.width + x;
                    out.set(x, y, band, spatial[plane].real);
                }
            }
        }

        unsafe { set_output_image_like(object, "out", out, input_image) }
    })();
    unsafe {
        crate::runtime::object::object_unref(mask_image);
        crate::runtime::object::object_unref(input_image);
    }
    result
}

pub(crate) unsafe fn dispatch(object: *mut VipsObject, nickname: &str) -> Result<bool, ()> {
    match nickname {
        "fwfft" => {
            unsafe { op_fwfft(object)? };
            Ok(true)
        }
        "invfft" => {
            unsafe { op_invfft(object)? };
            Ok(true)
        }
        "freqmult" => {
            unsafe { op_freqmult(object)? };
            Ok(true)
        }
        _ => Ok(false),
    }
}
