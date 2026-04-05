use crate::abi::image::VipsBandFormat;
use crate::abi::object::VipsObject;
use crate::pixels::format::complex_component_format;
use crate::pixels::iter::pixel_index;
use crate::pixels::{read_complex_image, ComplexSample, ImageBuffer, ImageSpec};

use super::{get_image_ref, set_output_image_like};

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
        1.0 / (width * height).max(1) as f64
    } else {
        1.0
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
        "freqmult" => {
            unsafe { op_freqmult(object)? };
            Ok(true)
        }
        _ => Ok(false),
    }
}
