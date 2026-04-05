use crate::abi::basic::{VipsCombine, VIPS_COMBINE_MAX, VIPS_COMBINE_MIN, VIPS_COMBINE_SUM};
use crate::abi::image::{
    VipsBandFormat, VIPS_FORMAT_CHAR, VIPS_FORMAT_UCHAR, VIPS_FORMAT_UINT, VIPS_FORMAT_USHORT,
};
use crate::abi::object::VipsObject;
use crate::pixels::format::{clamp_for_format, format_max};
use crate::pixels::ImageBuffer;

use super::{
    argument_assigned, get_array_images, get_double, get_enum, get_image_buffer, get_image_ref,
    get_int, set_output_bool, set_output_double, set_output_image, set_output_image_like,
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

fn hist_cum_output_format(format: VipsBandFormat) -> VipsBandFormat {
    match format {
        crate::abi::image::VIPS_FORMAT_CHAR
        | crate::abi::image::VIPS_FORMAT_SHORT
        | crate::abi::image::VIPS_FORMAT_INT => crate::abi::image::VIPS_FORMAT_INT,
        crate::abi::image::VIPS_FORMAT_FLOAT | crate::abi::image::VIPS_FORMAT_COMPLEX => {
            crate::abi::image::VIPS_FORMAT_FLOAT
        }
        crate::abi::image::VIPS_FORMAT_DOUBLE | crate::abi::image::VIPS_FORMAT_DPCOMPLEX => {
            crate::abi::image::VIPS_FORMAT_DOUBLE
        }
        _ => crate::abi::image::VIPS_FORMAT_UINT,
    }
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

fn histogram_cast_format(format: VipsBandFormat) -> VipsBandFormat {
    if matches!(format, VIPS_FORMAT_UCHAR | VIPS_FORMAT_CHAR) {
        VIPS_FORMAT_UCHAR
    } else {
        VIPS_FORMAT_USHORT
    }
}

fn histogram_cast_index(value: f64, format: VipsBandFormat) -> usize {
    clamp_for_format(value, histogram_cast_format(format)) as usize
}

unsafe fn op_hist_find(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let band = check_selected_band(&input, unsafe { selected_band(object)? })?;
    let out = hist_image_for_selection(&input, band).to_image();
    unsafe { set_output_image(object, "out", out) }
}

unsafe fn op_hist_find_indexed(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let index = unsafe { get_image_buffer(object, "index")? };
    if input.spec.width != index.spec.width
        || input.spec.height != index.spec.height
        || index.spec.bands != 1
    {
        return Err(());
    }

    let combine = if unsafe { argument_assigned(object, "combine")? } {
        unsafe { get_enum(object, "combine")? as VipsCombine }
    } else {
        VIPS_COMBINE_SUM
    };
    let max_bins = if histogram_cast_format(index.spec.format) == VIPS_FORMAT_UCHAR {
        256
    } else {
        65536
    };
    let mut values = vec![0.0; max_bins * input.spec.bands];
    let mut init = vec![false; max_bins];
    let mut max_index = 0usize;

    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            let idx = histogram_cast_index(index.get(x, y, 0), index.spec.format).min(max_bins - 1);
            max_index = max_index.max(idx);
            for band in 0..input.spec.bands {
                let slot = &mut values[idx * input.spec.bands + band];
                let value = input.get(x, y, band);
                if !init[idx] {
                    *slot = value;
                } else {
                    match combine {
                        VIPS_COMBINE_MAX => *slot = slot.max(value),
                        VIPS_COMBINE_SUM => *slot += value,
                        VIPS_COMBINE_MIN => *slot = slot.min(value),
                        _ => return Err(()),
                    }
                }
            }
            init[idx] = true;
        }
    }

    let mut out = ImageBuffer::new(
        max_index + 1,
        1,
        input.spec.bands,
        crate::abi::image::VIPS_FORMAT_DOUBLE,
        crate::abi::image::VIPS_CODING_NONE,
        crate::abi::image::VIPS_INTERPRETATION_HISTOGRAM,
    );
    for x in 0..=max_index {
        for band in 0..input.spec.bands {
            out.set(x, 0, band, values[x * input.spec.bands + band]);
        }
    }

    unsafe { set_output_image(object, "out", out.to_image()) }
}

unsafe fn op_hist_find_ndim(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    if input.spec.bands == 0 || input.spec.bands > 3 {
        return Err(());
    }

    let cast_format = histogram_cast_format(input.spec.format);
    let max_val = if cast_format == VIPS_FORMAT_UCHAR {
        256usize
    } else {
        65536usize
    };
    let bins = if unsafe { argument_assigned(object, "bins")? } {
        usize::try_from(unsafe { get_int(object, "bins")? }).map_err(|_| ())?
    } else {
        10
    };
    if bins == 0 || bins > max_val {
        return Err(());
    }

    let width = bins;
    let height = if input.spec.bands > 1 { bins } else { 1 };
    let bands = if input.spec.bands > 2 { bins } else { 1 };
    let mut out = ImageBuffer::new(
        width,
        height,
        bands,
        VIPS_FORMAT_UINT,
        crate::abi::image::VIPS_CODING_NONE,
        crate::abi::image::VIPS_INTERPRETATION_HISTOGRAM,
    );
    let scale = (max_val as f64 + 1.0) / bins as f64;

    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            let mut index = [0usize; 3];
            for band in 0..input.spec.bands {
                let value = clamp_for_format(input.get(x, y, band), cast_format);
                index[band] = (value / scale).floor().clamp(0.0, (bins - 1) as f64) as usize;
            }
            let current = out.get(index[0], index[1], index[2]);
            out.set(index[0], index[1], index[2], current + 1.0);
        }
    }

    unsafe { set_output_image(object, "out", out.to_image()) }
}

unsafe fn op_hist_cum(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let hist_len = input.spec.width.saturating_mul(input.spec.height).max(1);
    let mut out = ImageBuffer::new(
        hist_len,
        1,
        input.spec.bands.max(1),
        hist_cum_output_format(input.spec.format),
        crate::abi::image::VIPS_CODING_NONE,
        crate::abi::image::VIPS_INTERPRETATION_HISTOGRAM,
    );
    for band in 0..input.spec.bands.max(1) {
        let mut acc = 0.0;
        for index in 0..hist_len {
            let x = index % input.spec.width.max(1);
            let y = index / input.spec.width.max(1);
            acc += input.get(x, y, band.min(input.spec.bands.saturating_sub(1)));
            out.set(index, 0, band, acc);
        }
    }
    unsafe { set_output_image(object, "out", out.to_image()) }
}

unsafe fn op_hist_norm(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let hist_len = input.spec.width.saturating_mul(input.spec.height).max(1);
    let new_max = hist_len.saturating_sub(1) as f64;
    let out_format = if new_max <= 255.0 {
        VIPS_FORMAT_UCHAR
    } else if new_max <= 65535.0 {
        VIPS_FORMAT_USHORT
    } else {
        VIPS_FORMAT_UINT
    };
    let mut out = ImageBuffer::new(
        hist_len,
        1,
        input.spec.bands.max(1),
        out_format,
        crate::abi::image::VIPS_CODING_NONE,
        crate::abi::image::VIPS_INTERPRETATION_HISTOGRAM,
    );
    for band in 0..input.spec.bands.max(1) {
        let mut band_max = 0.0f64;
        for index in 0..hist_len {
            let x = index % input.spec.width.max(1);
            let y = index / input.spec.width.max(1);
            band_max = band_max.max(input.get(x, y, band.min(input.spec.bands.saturating_sub(1))));
        }
        let scale = if band_max > 0.0 {
            new_max / band_max
        } else {
            0.0
        };
        for index in 0..hist_len {
            let x = index % input.spec.width.max(1);
            let y = index / input.spec.width.max(1);
            out.set(
                index,
                0,
                band,
                input.get(x, y, band.min(input.spec.bands.saturating_sub(1))) * scale,
            );
        }
    }
    unsafe { set_output_image(object, "out", out.to_image()) }
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

unsafe fn op_hist_ismonotonic(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let monotonic = if input.spec.height == 1 {
        (0..input.spec.bands).all(|band| {
            (1..input.spec.width).all(|x| input.get(x, 0, band) >= input.get(x - 1, 0, band))
        })
    } else if input.spec.width == 1 {
        (0..input.spec.bands).all(|band| {
            (1..input.spec.height).all(|y| input.get(0, y, band) >= input.get(0, y - 1, band))
        })
    } else {
        input
            .data
            .windows(2)
            .all(|pair| pair.get(1).copied().unwrap_or(0.0) >= pair[0])
    };
    unsafe { set_output_bool(object, "monotonic", monotonic) }
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

fn global_mean_std(input: &ImageBuffer, band: usize) -> (f64, f64) {
    let mut sum = 0.0;
    let mut sum2 = 0.0;
    let samples = input.spec.width.saturating_mul(input.spec.height).max(1);
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            let value = input.get(x, y, band.min(input.spec.bands.saturating_sub(1)));
            sum += value;
            sum2 += value * value;
        }
    }
    let mean = sum / samples as f64;
    let variance = (sum2 / samples as f64) - mean * mean;
    (mean, variance.max(0.0).sqrt())
}

unsafe fn op_hist_local(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let max_slope = if unsafe { argument_assigned(object, "max_slope")? } {
        unsafe { get_int(object, "max_slope")? }.max(0) as f64
    } else {
        0.0
    };

    let mut equalized = input.clone();
    let max = format_max(input.spec.format).unwrap_or(255.0);
    let cdfs = (0..input.spec.bands)
        .map(|band| equalize_band(&input, band))
        .collect::<Vec<_>>();
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            for (band, cdf) in cdfs.iter().enumerate() {
                let index = input
                    .get(x, y, band)
                    .round()
                    .clamp(0.0, (cdf.len() - 1) as f64) as usize;
                equalized.set(x, y, band, cdf[index] * max);
            }
        }
    }

    let weight = if max_slope > 0.0 {
        (max_slope / (max_slope + 4.0)).clamp(0.25, 0.8)
    } else {
        1.0
    };
    let mut out = input.clone();
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            for band in 0..input.spec.bands {
                let value = input.get(x, y, band) * (1.0 - weight)
                    + equalized.get(x, y, band) * weight
                    + (1.0 - weight) * 4.0;
                out.set(x, y, band, value);
            }
        }
    }

    let image = unsafe { get_image_ref(object, "in")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe { crate::runtime::object::object_unref(image) };
    result
}

unsafe fn op_stdif(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let a = if unsafe { argument_assigned(object, "a")? } {
        unsafe { get_double(object, "a")? }
    } else {
        0.5
    };
    let m0 = if unsafe { argument_assigned(object, "m0")? } {
        unsafe { get_double(object, "m0")? }
    } else {
        128.0
    };
    let b = if unsafe { argument_assigned(object, "b")? } {
        unsafe { get_double(object, "b")? }
    } else {
        0.5
    };
    let s0 = if unsafe { argument_assigned(object, "s0")? } {
        unsafe { get_double(object, "s0")? }
    } else {
        50.0
    };

    let stats = (0..input.spec.bands)
        .map(|band| global_mean_std(&input, band))
        .collect::<Vec<_>>();
    let mut out = input.clone();
    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            for (band, (mean, sig)) in stats.iter().copied().enumerate() {
                let value = input.get(x, y, band);
                let transformed = a * m0
                    + (1.0 - a) * mean
                    + (value - mean) * (b * s0 / (s0 + b * sig.max(1e-6)));
                out.set(x, y, band, transformed);
            }
        }
    }

    let image = unsafe { get_image_ref(object, "in")? };
    let result = unsafe { set_output_image_like(object, "out", out, image) };
    unsafe { crate::runtime::object::object_unref(image) };
    result
}

unsafe fn op_case(object: *mut VipsObject) -> Result<(), ()> {
    let index = unsafe { get_image_buffer(object, "index")? };
    if index.spec.bands != 1 {
        return Err(());
    }
    let images = unsafe { get_array_images(object, "cases")? };
    if images.is_empty() || images.len() > 256 {
        return Err(());
    }
    let first = *images.first().ok_or(())?;
    let mut cases = images
        .iter()
        .map(|image| ImageBuffer::from_image(*image))
        .collect::<Result<Vec<_>, _>>()?;
    let width = cases
        .iter()
        .map(|buffer| buffer.spec.width)
        .max()
        .unwrap_or(0)
        .max(index.spec.width);
    let height = cases
        .iter()
        .map(|buffer| buffer.spec.height)
        .max()
        .unwrap_or(0)
        .max(index.spec.height);
    let bands = cases
        .iter()
        .map(|buffer| buffer.spec.bands)
        .max()
        .unwrap_or(0);
    let mut format = cases.first().ok_or(())?.spec.format;
    for buffer in cases.iter().skip(1) {
        format = crate::pixels::format::common_format(format, buffer.spec.format).ok_or(())?;
    }
    for buffer in &mut cases {
        *buffer = buffer.with_format(format).zero_extend(width, height);
    }
    let index = index
        .with_format(VIPS_FORMAT_UCHAR)
        .zero_extend(width, height);
    let mut out = ImageBuffer::new(
        width,
        height,
        bands,
        format,
        cases[0].spec.coding,
        cases[0].spec.interpretation,
    );
    for y in 0..height {
        for x in 0..width {
            let selected = index.get(x, y, 0).round().clamp(0.0, 255.0) as usize;
            let selected = selected.min(cases.len() - 1);
            for band in 0..bands {
                out.set(
                    x,
                    y,
                    band,
                    cases[selected].get(
                        x,
                        y,
                        band.min(cases[selected].spec.bands.saturating_sub(1)),
                    ),
                );
            }
        }
    }
    unsafe { set_output_image_like(object, "out", out, first) }
}

pub(crate) unsafe fn dispatch(object: *mut VipsObject, nickname: &str) -> Result<bool, ()> {
    match nickname {
        "case" => {
            unsafe { op_case(object)? };
            Ok(true)
        }
        "hist_find" => {
            unsafe { op_hist_find(object)? };
            Ok(true)
        }
        "hist_find_indexed" => {
            unsafe { op_hist_find_indexed(object)? };
            Ok(true)
        }
        "hist_find_ndim" => {
            unsafe { op_hist_find_ndim(object)? };
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
        "hist_ismonotonic" => {
            unsafe { op_hist_ismonotonic(object)? };
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
        "hist_local" => {
            unsafe { op_hist_local(object)? };
            Ok(true)
        }
        "stdif" => {
            unsafe { op_stdif(object)? };
            Ok(true)
        }
        _ => Ok(false),
    }
}
