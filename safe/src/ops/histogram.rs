use crate::abi::image::{VipsBandFormat, VIPS_FORMAT_FLOAT, VIPS_FORMAT_UCHAR, VIPS_FORMAT_UINT};
use crate::abi::object::VipsObject;
use crate::pixels::format::format_max;
use crate::pixels::ImageBuffer;

use super::{
    argument_assigned, get_double, get_image_buffer, get_image_ref, get_int, set_output_double,
    set_output_image, set_output_image_like,
};

fn histogram_bins(format: VipsBandFormat) -> usize {
    match format {
        crate::abi::image::VIPS_FORMAT_USHORT => 65536,
        _ => 256,
    }
}

fn hist_for_band(input: &ImageBuffer, band: usize) -> Vec<f64> {
    let bins = histogram_bins(input.spec.format);
    let max = format_max(input.spec.format)
        .unwrap_or((bins - 1) as f64)
        .max(1.0);
    let mut hist = vec![0.0; bins];
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            let value = input.get(x, y, band.min(input.spec.bands - 1));
            let index = value.round().clamp(0.0, max) as usize;
            hist[index.min(bins - 1)] += 1.0;
        }
    }
    hist
}

fn hist_image(values: &[f64], format: VipsBandFormat) -> ImageBuffer {
    let mut out = ImageBuffer::new(
        values.len(),
        1,
        1,
        format,
        crate::abi::image::VIPS_CODING_NONE,
        crate::abi::image::VIPS_INTERPRETATION_HISTOGRAM,
    );
    out.data = values.to_vec();
    out
}

unsafe fn selected_band(object: *mut VipsObject) -> Result<Option<usize>, ()> {
    if !unsafe { argument_assigned(object, "band")? } {
        return Ok(None);
    }
    let band = unsafe { get_int(object, "band")? };
    if band < 0 {
        Ok(None)
    } else {
        usize::try_from(band).map(Some).map_err(|_| ())
    }
}

fn check_selected_band(input: &ImageBuffer, band: Option<usize>) -> Result<Option<usize>, ()> {
    match band {
        Some(band) if band >= input.spec.bands => Err(()),
        _ => Ok(band),
    }
}

fn hist_image_for_selection(input: &ImageBuffer, band: Option<usize>) -> ImageBuffer {
    let bins = histogram_bins(input.spec.format);
    let output_bands = band.map(|_| 1).unwrap_or(input.spec.bands.max(1));
    let mut out = ImageBuffer::new(
        bins,
        1,
        output_bands,
        VIPS_FORMAT_UINT,
        crate::abi::image::VIPS_CODING_NONE,
        crate::abi::image::VIPS_INTERPRETATION_HISTOGRAM,
    );
    match band {
        Some(band) => {
            for (x, value) in hist_for_band(input, band).into_iter().enumerate() {
                out.set(x, 0, 0, value);
            }
        }
        None => {
            for band in 0..input.spec.bands {
                for (x, value) in hist_for_band(input, band).into_iter().enumerate() {
                    out.set(x, 0, band, value);
                }
            }
        }
    }
    out
}

unsafe fn op_hist_find(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let band = check_selected_band(&input, unsafe { selected_band(object)? })?;
    let out = hist_image_for_selection(&input, band).to_image();
    unsafe { set_output_image(object, "out", out) }
}

unsafe fn op_hist_cum(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let mut values = input.data.clone();
    for index in 1..values.len() {
        values[index] += values[index - 1];
    }
    let out = hist_image(&values, input.spec.format).to_image();
    unsafe { set_output_image(object, "out", out) }
}

unsafe fn op_hist_norm(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let sum = input.data.iter().sum::<f64>().max(1.0);
    let values = input
        .data
        .iter()
        .map(|value| value / sum)
        .collect::<Vec<_>>();
    let out = hist_image(&values, VIPS_FORMAT_FLOAT).to_image();
    unsafe { set_output_image(object, "out", out) }
}

unsafe fn op_hist_plot(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let max = input.data.iter().copied().fold(0.0, f64::max).max(1.0);
    let height = 256usize;
    let mut out = ImageBuffer::new(
        input.spec.width,
        height,
        1,
        VIPS_FORMAT_UCHAR,
        crate::abi::image::VIPS_CODING_NONE,
        crate::abi::image::VIPS_INTERPRETATION_HISTOGRAM,
    );
    for x in 0..input.spec.width {
        let level = (input.get(x, 0, 0) / max * (height - 1) as f64).round() as usize;
        for y in 0..height {
            out.set(x, height - 1 - y, 0, if y <= level { 255.0 } else { 0.0 });
        }
    }
    unsafe { set_output_image(object, "out", out.to_image()) }
}

unsafe fn op_maplut(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let lut = unsafe { get_image_buffer(object, "lut")? };
    let band = check_selected_band(&input, unsafe { selected_band(object)? })?;
    let lut_len = lut.spec.width.max(1);
    let mut out = input.with_format(lut.spec.format);
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            for b in 0..input.spec.bands {
                let source_band = band.unwrap_or(b).min(input.spec.bands - 1);
                let lut_band = b.min(lut.spec.bands - 1);
                let index = input
                    .get(x, y, source_band)
                    .round()
                    .clamp(0.0, (lut_len - 1) as f64) as usize;
                out.set(x, y, b, lut.get(index, 0, lut_band));
            }
        }
    }
    let image = unsafe { get_image_ref(object, "in")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe { crate::runtime::object::object_unref(image) };
    result
}

unsafe fn op_percent(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let percent = unsafe { get_double(object, "percent")? }.clamp(0.0, 100.0);
    let hist = hist_for_band(&input, 0);
    let total = hist.iter().sum::<f64>().max(1.0);
    let target = total * percent / 100.0;
    let mut acc = 0.0;
    let mut threshold = 0;
    for (index, value) in hist.iter().copied().enumerate() {
        acc += value;
        if acc >= target {
            threshold = index as i32;
            break;
        }
    }
    unsafe { super::set_output_int(object, "threshold", threshold) }
}

unsafe fn op_hist_entropy(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let sum = input.data.iter().sum::<f64>().max(1.0);
    let entropy = input
        .data
        .iter()
        .copied()
        .filter(|value| *value > 0.0)
        .map(|value| {
            let p = value / sum;
            -p * p.log2()
        })
        .sum();
    unsafe { set_output_double(object, "out", entropy) }
}

fn equalize_band(input: &ImageBuffer, band: usize) -> Vec<f64> {
    let hist = hist_for_band(input, band);
    let total = hist.iter().sum::<f64>().max(1.0);
    let mut cdf = vec![0.0; hist.len()];
    let mut acc = 0.0;
    for (index, value) in hist.iter().copied().enumerate() {
        acc += value;
        cdf[index] = acc / total;
    }
    cdf
}

unsafe fn op_hist_equal(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let band = check_selected_band(&input, unsafe { selected_band(object)? })?;
    let max = format_max(input.spec.format).unwrap_or(255.0);
    let mut out = input.clone();
    match band {
        Some(selected) => {
            let cdf = equalize_band(&input, selected);
            for y in 0..input.spec.height {
                for x in 0..input.spec.width {
                    let index = input
                        .get(x, y, selected)
                        .round()
                        .clamp(0.0, (cdf.len() - 1) as f64)
                        as usize;
                    out.set(x, y, selected, cdf[index] * max);
                }
            }
        }
        None => {
            let cdfs = (0..input.spec.bands)
                .map(|band| equalize_band(&input, band))
                .collect::<Vec<_>>();
            for y in 0..input.spec.height {
                for x in 0..input.spec.width {
                    for (band, cdf) in cdfs.iter().enumerate() {
                        let index = input
                            .get(x, y, band)
                            .round()
                            .clamp(0.0, (cdf.len() - 1) as f64)
                            as usize;
                        out.set(x, y, band, cdf[index] * max);
                    }
                }
            }
        }
    }
    let image = unsafe { get_image_ref(object, "in")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe { crate::runtime::object::object_unref(image) };
    result
}

unsafe fn op_hist_match(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let reference = unsafe { get_image_buffer(object, "ref")? };
    let src = equalize_band(&input, 0);
    let dst = equalize_band(&reference, 0);
    let mut lut = vec![0.0; src.len()];
    for (index, src_value) in src.iter().copied().enumerate() {
        let target = dst
            .iter()
            .position(|value| *value >= src_value)
            .unwrap_or(dst.len() - 1);
        lut[index] = target as f64;
    }
    let mut out = input.clone();
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            for b in 0..input.spec.bands {
                let index = input
                    .get(x, y, b)
                    .round()
                    .clamp(0.0, (lut.len() - 1) as f64) as usize;
                out.set(x, y, b, lut[index]);
            }
        }
    }
    let image = unsafe { get_image_ref(object, "in")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe { crate::runtime::object::object_unref(image) };
    result
}

pub(crate) unsafe fn dispatch(object: *mut VipsObject, nickname: &str) -> Result<bool, ()> {
    match nickname {
        "hist_find" => {
            unsafe { op_hist_find(object)? };
            Ok(true)
        }
        "hist_cum" => {
            unsafe { op_hist_cum(object)? };
            Ok(true)
        }
        "hist_norm" => {
            unsafe { op_hist_norm(object)? };
            Ok(true)
        }
        "hist_plot" => {
            unsafe { op_hist_plot(object)? };
            Ok(true)
        }
        "maplut" => {
            unsafe { op_maplut(object)? };
            Ok(true)
        }
        "percent" => {
            unsafe { op_percent(object)? };
            Ok(true)
        }
        "hist_entropy" => {
            unsafe { op_hist_entropy(object)? };
            Ok(true)
        }
        "hist_match" => {
            unsafe { op_hist_match(object)? };
            Ok(true)
        }
        "hist_equal" => {
            unsafe { op_hist_equal(object)? };
            Ok(true)
        }
        _ => Ok(false),
    }
}
