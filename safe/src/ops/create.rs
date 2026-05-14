use std::time::{SystemTime, UNIX_EPOCH};

use crate::abi::basic::{VipsPrecision, VIPS_PRECISION_FLOAT, VIPS_PRECISION_INTEGER};
use crate::abi::image::{
    VIPS_INTERPRETATION_sRGB, VIPS_DEMAND_STYLE_THINSTRIP, VIPS_FORMAT_DOUBLE, VIPS_FORMAT_FLOAT,
    VIPS_FORMAT_UCHAR, VIPS_FORMAT_UINT, VIPS_FORMAT_USHORT, VIPS_INTERPRETATION_B_W,
    VIPS_INTERPRETATION_FOURIER, VIPS_INTERPRETATION_HISTOGRAM, VIPS_INTERPRETATION_MULTIBAND,
    VIPS_INTERPRETATION_XYZ,
};
use crate::abi::object::VipsObject;
use crate::pixels::kernel::{gaussian_kernel, log_kernel};
use crate::pixels::ImageBuffer;

use super::{
    argument_assigned, get_bool, get_double, get_enum, get_image_buffer, get_int, get_string,
    set_output_image, set_output_int,
};

fn blank(width: usize, height: usize, bands: usize) -> ImageBuffer {
    ImageBuffer::new(
        width,
        height,
        bands,
        VIPS_FORMAT_UCHAR,
        crate::abi::image::VIPS_CODING_NONE,
        if bands == 1 {
            crate::abi::image::VIPS_INTERPRETATION_B_W
        } else {
            VIPS_INTERPRETATION_MULTIBAND
        },
    )
}

fn point_to_uchar(value: f64) -> f64 {
    (value * 255.0).clamp(0.0, 255.0)
}

fn clamp_point_dimensions(width: usize, height: usize) -> (usize, usize) {
    (width.max(1), height.max(1))
}

fn point_output(mut out: ImageBuffer) -> ImageBuffer {
    out.spec.dhint = VIPS_DEMAND_STYLE_THINSTRIP;
    out
}

fn matrix_rows(buffer: &ImageBuffer) -> Result<Vec<Vec<f64>>, ()> {
    if buffer.spec.bands != 1 {
        return Err(());
    }

    let mut rows = Vec::with_capacity(buffer.spec.height);
    for y in 0..buffer.spec.height {
        let mut row = Vec::with_capacity(buffer.spec.width);
        for x in 0..buffer.spec.width {
            row.push(buffer.get(x, y, 0));
        }
        rows.push(row);
    }
    Ok(rows)
}

fn default_noise_seed() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as u32 ^ duration.subsec_nanos())
        .unwrap_or(0x4d595df4)
}

fn mix_u32(mut value: u32) -> u32 {
    value = value.wrapping_add(0x9e3779b9);
    value ^= value >> 16;
    value = value.wrapping_mul(0x85ebca6b);
    value ^= value >> 13;
    value = value.wrapping_mul(0xc2b2ae35);
    value ^ (value >> 16)
}

fn next_u32(state: &mut u32) -> u32 {
    *state = state.wrapping_mul(1664525).wrapping_add(1013904223);
    *state
}

unsafe fn op_black(object: *mut VipsObject) -> Result<(), ()> {
    let width = usize::try_from(unsafe { get_int(object, "width")? }).map_err(|_| ())?;
    let height = usize::try_from(unsafe { get_int(object, "height")? }).map_err(|_| ())?;
    let bands = if unsafe { argument_assigned(object, "bands")? } {
        usize::try_from(unsafe { get_int(object, "bands")? }).map_err(|_| ())?
    } else {
        1
    };
    let out = blank(width, height, bands).to_image();
    unsafe { set_output_image(object, "out", out) }
}

unsafe fn op_grey(object: *mut VipsObject) -> Result<(), ()> {
    let width = usize::try_from(unsafe { get_int(object, "width")? }).map_err(|_| ())?;
    let height = usize::try_from(unsafe { get_int(object, "height")? }).map_err(|_| ())?;
    let (width, height) = clamp_point_dimensions(width, height);
    let uchar =
        unsafe { argument_assigned(object, "uchar")? } && unsafe { get_bool(object, "uchar")? };
    let mut out = point_output(ImageBuffer::new(
        width,
        height,
        1,
        if uchar {
            VIPS_FORMAT_UCHAR
        } else {
            VIPS_FORMAT_FLOAT
        },
        crate::abi::image::VIPS_CODING_NONE,
        crate::abi::image::VIPS_INTERPRETATION_B_W,
    ));
    let denom = width.saturating_sub(1).max(1) as f64;
    for y in 0..height {
        for x in 0..width {
            let value = x as f64 / denom;
            out.set(x, y, 0, if uchar { point_to_uchar(value) } else { value });
        }
    }
    unsafe { set_output_image(object, "out", out.to_image()) }
}

unsafe fn op_xyz(object: *mut VipsObject) -> Result<(), ()> {
    let width = usize::try_from(unsafe { get_int(object, "width")? }).map_err(|_| ())?;
    let height = usize::try_from(unsafe { get_int(object, "height")? }).map_err(|_| ())?;
    let csize = if unsafe { argument_assigned(object, "csize")? } {
        usize::try_from(unsafe { get_int(object, "csize")? }).map_err(|_| ())?
    } else {
        1
    };
    let dsize = if unsafe { argument_assigned(object, "dsize")? } {
        usize::try_from(unsafe { get_int(object, "dsize")? }).map_err(|_| ())?
    } else {
        1
    };
    let esize = if unsafe { argument_assigned(object, "esize")? } {
        usize::try_from(unsafe { get_int(object, "esize")? }).map_err(|_| ())?
    } else {
        1
    };
    let dims = if esize > 1 {
        5
    } else if dsize > 1 {
        4
    } else if csize > 1 {
        3
    } else {
        2
    };
    let out_height = height
        .checked_mul(csize)
        .and_then(|value| value.checked_mul(dsize))
        .and_then(|value| value.checked_mul(esize))
        .ok_or(())?;
    let mut out = ImageBuffer::new(
        width,
        out_height,
        dims,
        VIPS_FORMAT_UINT,
        crate::abi::image::VIPS_CODING_NONE,
        VIPS_INTERPRETATION_XYZ,
    );
    for y in 0..out_height {
        let h4 = height * csize * dsize;
        let dim4 = y / h4.max(1);
        let r4 = y % h4.max(1);
        let h3 = (height * csize).max(1);
        let dim3 = r4 / h3;
        let r3 = r4 % h3;
        let dim2 = r3 / height.max(1);
        let dim1 = r3 % height.max(1);
        for x in 0..width {
            let coords = [x as f64, dim1 as f64, dim2 as f64, dim3 as f64, dim4 as f64];
            for band in 0..dims {
                out.set(x, y, band, coords[band]);
            }
        }
    }
    unsafe { set_output_image(object, "out", out.to_image()) }
}

fn parsed_font_size(font: Option<&str>) -> f64 {
    font.and_then(|font| {
        font.split_whitespace()
            .rev()
            .find_map(|part| part.parse::<f64>().ok())
    })
    .filter(|value| *value > 0.0)
    .unwrap_or(12.0)
}

fn text_lines(text: &str) -> Vec<&str> {
    let lines = text.split('\n').collect::<Vec<_>>();
    if lines.is_empty() {
        vec![text]
    } else {
        lines
    }
}

unsafe fn op_text(object: *mut VipsObject) -> Result<(), ()> {
    let text = unsafe { get_string(object, "text")? }.ok_or(())?;
    if text.is_empty() {
        return Err(());
    }

    let dpi = if unsafe { argument_assigned(object, "dpi")? } {
        unsafe { get_int(object, "dpi")? }.max(1)
    } else {
        72
    };
    let font = if unsafe { argument_assigned(object, "font")? } {
        unsafe { get_string(object, "font")? }
    } else {
        None
    };
    let font_size = parsed_font_size(font.as_deref());
    let scale = dpi as f64 / 72.0;
    let char_width = ((font_size * scale * 0.62).ceil() as usize).max(4);
    let glyph_height = ((font_size * scale * 1.25).ceil() as usize).max(8);
    let line_gap = if unsafe { argument_assigned(object, "spacing")? } {
        unsafe { get_int(object, "spacing")? }.max(0) as usize
    } else {
        0
    };

    let lines = text_lines(&text);
    let content_width = lines
        .iter()
        .map(|line| {
            line.chars()
                .map(|ch| {
                    if ch.is_whitespace() {
                        char_width / 2
                    } else {
                        char_width
                    }
                })
                .sum::<usize>()
        })
        .max()
        .unwrap_or(0);
    if content_width == 0 {
        return Err(());
    }

    let natural_width = content_width.checked_add(4).ok_or(())?;
    let natural_height = lines
        .len()
        .checked_mul(glyph_height.checked_add(line_gap).ok_or(())?)
        .and_then(|height| height.checked_add(4))
        .ok_or(())?;
    let width_limit = if unsafe { argument_assigned(object, "width")? } {
        usize::try_from(unsafe { get_int(object, "width")? }).map_err(|_| ())?
    } else {
        0
    };
    let height_limit = if unsafe { argument_assigned(object, "height")? } {
        usize::try_from(unsafe { get_int(object, "height")? }).map_err(|_| ())?
    } else {
        0
    };
    let width = if width_limit > 0 {
        natural_width.min(width_limit).max(1)
    } else {
        natural_width
    };
    let height = if height_limit > 0 {
        natural_height.min(height_limit).max(1)
    } else {
        natural_height
    };
    let rgba =
        unsafe { argument_assigned(object, "rgba")? } && unsafe { get_bool(object, "rgba")? };

    let mut out = ImageBuffer::new(
        width,
        height,
        if rgba { 4 } else { 1 },
        VIPS_FORMAT_UCHAR,
        crate::abi::image::VIPS_CODING_NONE,
        if rgba {
            VIPS_INTERPRETATION_sRGB
        } else {
            VIPS_INTERPRETATION_B_W
        },
    );
    out.spec.xres = dpi as f64 / 25.4;
    out.spec.yres = dpi as f64 / 25.4;

    for (line_index, line) in lines.iter().enumerate() {
        let top = 2usize.saturating_add(line_index * (glyph_height + line_gap));
        let mut left = 2usize;
        for ch in line.chars() {
            let advance = if ch.is_whitespace() {
                (char_width / 2).max(1)
            } else {
                char_width
            };
            if !ch.is_whitespace() {
                let glyph_width = advance.saturating_sub(2).max(1);
                for y in top..top
                    .saturating_add(glyph_height)
                    .min(height.saturating_sub(1))
                {
                    for x in left..left
                        .saturating_add(glyph_width)
                        .min(width.saturating_sub(1))
                    {
                        let edge = x == left
                            || y == top
                            || x + 1 == left.saturating_add(glyph_width)
                            || y + 1 == top.saturating_add(glyph_height);
                        let alpha = if edge { 180.0 } else { 255.0 };
                        if rgba {
                            out.set(x, y, 0, 0.0);
                            out.set(x, y, 1, 0.0);
                            out.set(x, y, 2, 0.0);
                            out.set(x, y, 3, alpha);
                        } else {
                            out.set(x, y, 0, alpha);
                        }
                    }
                }
            }
            left = left.saturating_add(advance);
            if left >= width {
                break;
            }
        }
    }

    unsafe { set_output_int(object, "autofit_dpi", dpi)? };
    unsafe { set_output_image(object, "out", out.to_image()) }
}

unsafe fn op_identity(object: *mut VipsObject) -> Result<(), ()> {
    let bands = if unsafe { argument_assigned(object, "bands")? } {
        usize::try_from(unsafe { get_int(object, "bands")? }).map_err(|_| ())?
    } else {
        1
    };
    let ushort =
        unsafe { argument_assigned(object, "ushort")? } && unsafe { get_bool(object, "ushort")? };
    let size = if ushort && unsafe { argument_assigned(object, "size")? } {
        usize::try_from(unsafe { get_int(object, "size")? }).map_err(|_| ())?
    } else if ushort {
        65536
    } else {
        256
    };
    let mut out = ImageBuffer::new(
        size,
        1,
        bands,
        if ushort {
            VIPS_FORMAT_USHORT
        } else {
            VIPS_FORMAT_UCHAR
        },
        crate::abi::image::VIPS_CODING_NONE,
        VIPS_INTERPRETATION_HISTOGRAM,
    );
    for x in 0..size {
        for band in 0..bands {
            out.set(x, 0, band, x as f64);
        }
    }
    unsafe { set_output_image(object, "out", out.to_image()) }
}

unsafe fn op_eye(object: *mut VipsObject) -> Result<(), ()> {
    let width = usize::try_from(unsafe { get_int(object, "width")? }).map_err(|_| ())?;
    let height = usize::try_from(unsafe { get_int(object, "height")? }).map_err(|_| ())?;
    let (width, height) = clamp_point_dimensions(width, height);
    let factor = if unsafe { argument_assigned(object, "factor")? } {
        unsafe { get_double(object, "factor")? }
    } else {
        0.5
    };
    let uchar =
        unsafe { argument_assigned(object, "uchar")? } && unsafe { get_bool(object, "uchar")? };
    let mut out = point_output(ImageBuffer::new(
        width,
        height,
        1,
        if uchar {
            VIPS_FORMAT_UCHAR
        } else {
            VIPS_FORMAT_FLOAT
        },
        crate::abi::image::VIPS_CODING_NONE,
        crate::abi::image::VIPS_INTERPRETATION_B_W,
    ));
    let max_x = width.saturating_sub(1).max(1) as f64;
    let max_y = height.saturating_sub(1).max(1) as f64;
    let c = factor * std::f64::consts::PI / (2.0 * max_x);
    let h = max_y * max_y;
    for y in 0..height {
        for x in 0..width {
            let value = (y as f64 * y as f64) * (c * x as f64 * x as f64).cos() / h;
            out.set(x, y, 0, if uchar { point_to_uchar(value) } else { value });
        }
    }
    unsafe { set_output_image(object, "out", out.to_image()) }
}

unsafe fn op_matrix_kernel(
    object: *mut VipsObject,
    build: impl FnOnce(f64, f64, bool, VipsPrecision) -> Result<crate::pixels::kernel::Kernel, ()>,
) -> Result<(), ()> {
    let sigma = unsafe { get_double(object, "sigma")? };
    let min_ampl = unsafe { get_double(object, "min_ampl")? };
    let separable = unsafe { argument_assigned(object, "separable")? }
        && unsafe { get_bool(object, "separable")? };
    let precision = if unsafe { argument_assigned(object, "precision")? } {
        unsafe { get_enum(object, "precision")? as VipsPrecision }
    } else if unsafe { argument_assigned(object, "integer")? }
        && !unsafe { get_bool(object, "integer")? }
    {
        VIPS_PRECISION_FLOAT
    } else {
        VIPS_PRECISION_INTEGER
    };
    let kernel = build(sigma, min_ampl, separable, precision)?;
    unsafe { set_output_image(object, "out", kernel.to_image()) }
}

fn mask_base(width: usize, height: usize, x: usize, y: usize, optical: bool) -> (f64, f64, bool) {
    let half_width = (width / 2).max(1);
    let half_height = (height / 2).max(1);
    let mut xx = x as isize;
    let mut yy = y as isize;
    if !optical {
        xx = (xx + half_width as isize) % width.max(1) as isize;
        yy = (yy + half_height as isize) % height.max(1) as isize;
    }
    xx -= half_width as isize;
    yy -= half_height as isize;
    let is_dc = xx == 0 && yy == 0;
    (
        xx as f64 / half_width as f64,
        yy as f64 / half_height as f64,
        is_dc,
    )
}

unsafe fn op_mask_from_point(
    object: *mut VipsObject,
    point: impl Fn(f64, f64) -> f64,
) -> Result<(), ()> {
    let width = usize::try_from(unsafe { get_int(object, "width")? }).map_err(|_| ())?;
    let height = usize::try_from(unsafe { get_int(object, "height")? }).map_err(|_| ())?;
    let (width, height) = clamp_point_dimensions(width, height);
    let optical =
        unsafe { argument_assigned(object, "optical")? } && unsafe { get_bool(object, "optical")? };
    let reject =
        unsafe { argument_assigned(object, "reject")? } && unsafe { get_bool(object, "reject")? };
    let nodc =
        unsafe { argument_assigned(object, "nodc")? } && unsafe { get_bool(object, "nodc")? };
    let uchar =
        unsafe { argument_assigned(object, "uchar")? } && unsafe { get_bool(object, "uchar")? };
    let mut out = point_output(ImageBuffer::new(
        width,
        height,
        1,
        if uchar {
            VIPS_FORMAT_UCHAR
        } else {
            VIPS_FORMAT_FLOAT
        },
        crate::abi::image::VIPS_CODING_NONE,
        VIPS_INTERPRETATION_FOURIER,
    ));
    for y in 0..height {
        for x in 0..width {
            let (dx, dy, is_dc) = mask_base(width, height, x, y, optical);
            let value = if !nodc && is_dc {
                1.0
            } else {
                let value = point(dx, dy);
                if reject {
                    1.0 - value
                } else {
                    value
                }
            };
            out.set(x, y, 0, if uchar { point_to_uchar(value) } else { value });
        }
    }
    unsafe { set_output_image(object, "out", out.to_image()) }
}

unsafe fn op_buildlut(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let mut rows = matrix_rows(&input)?;
    if rows.is_empty() || rows[0].len() < 2 {
        return Err(());
    }
    let mut xlow = 0i32;
    let mut xhigh = 0i32;
    for (index, row) in rows.iter().enumerate() {
        let rounded = row[0].round();
        if !rounded.is_finite()
            || rounded < i32::MIN as f64
            || rounded > i32::MAX as f64
            || (row[0] - rounded).abs() > 0.001
        {
            let _ = index;
            return Err(());
        }
        let rounded = rounded as i32;
        if index == 0 {
            xlow = rounded;
            xhigh = rounded;
        } else {
            xlow = xlow.min(rounded);
            xhigh = xhigh.max(rounded);
        }
    }
    let lut_size = usize::try_from(
        xhigh
            .checked_sub(xlow)
            .and_then(|value| value.checked_add(1))
            .ok_or(())?,
    )
    .map_err(|_| ())?;
    if lut_size < 1 {
        return Err(());
    }
    rows.sort_by(|left, right| {
        left[0]
            .partial_cmp(&right[0])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let bands = rows[0].len() - 1;
    let mut out = ImageBuffer::new(
        lut_size,
        1,
        bands,
        VIPS_FORMAT_DOUBLE,
        crate::abi::image::VIPS_CODING_NONE,
        VIPS_INTERPRETATION_HISTOGRAM,
    );
    for band in 0..bands {
        for pair in rows.windows(2) {
            let x1 = pair[0][0].round() as i32;
            let x2 = pair[1][0].round() as i32;
            let dx = x2 - x1;
            if dx <= 0 {
                continue;
            }
            let y1 = pair[0][band + 1];
            let y2 = pair[1][band + 1];
            let dy = y2 - y1;
            for x in 0..dx {
                let out_x = usize::try_from(x + x1 - xlow).map_err(|_| ())?;
                out.set(out_x, 0, band, y1 + x as f64 * dy / dx as f64);
            }
        }
        let xlast = rows.last().ok_or(())?[0].round() as i32;
        let out_x = usize::try_from(xlast - xlow).map_err(|_| ())?;
        out.set(out_x, 0, band, rows.last().ok_or(())?[band + 1]);
    }
    unsafe { set_output_image(object, "out", out.to_image()) }
}

unsafe fn op_gaussnoise(object: *mut VipsObject) -> Result<(), ()> {
    let width = usize::try_from(unsafe { get_int(object, "width")? }).map_err(|_| ())?;
    let height = usize::try_from(unsafe { get_int(object, "height")? }).map_err(|_| ())?;
    let mean = if unsafe { argument_assigned(object, "mean")? } {
        unsafe { get_double(object, "mean")? }
    } else {
        128.0
    };
    let sigma = if unsafe { argument_assigned(object, "sigma")? } {
        unsafe { get_double(object, "sigma")? }
    } else {
        30.0
    };
    let base_seed = if unsafe { argument_assigned(object, "seed")? } {
        unsafe { get_int(object, "seed")? as u32 }
    } else {
        default_noise_seed()
    };

    let mut out = ImageBuffer::new(
        width,
        height,
        1,
        VIPS_FORMAT_FLOAT,
        crate::abi::image::VIPS_CODING_NONE,
        VIPS_INTERPRETATION_MULTIBAND,
    );
    for y in 0..height {
        for x in 0..width {
            let mut state = mix_u32(
                base_seed
                    ^ (x as u32).wrapping_mul(0x9e3779b9)
                    ^ (y as u32).wrapping_mul(0x85ebca6b),
            );
            if state == 0 {
                state = 0x6d2b79f5;
            }
            let mut sum = 0.0;
            for _ in 0..12 {
                sum += next_u32(&mut state) as f64 / u32::MAX as f64;
            }
            out.set(x, y, 0, (sum - 6.0) * sigma + mean);
        }
    }
    unsafe { set_output_image(object, "out", out.to_image()) }
}

unsafe fn op_invertlut(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let size = if unsafe { argument_assigned(object, "size")? } {
        usize::try_from(unsafe { get_int(object, "size")? }).map_err(|_| ())?
    } else {
        256
    };
    if size < 1 {
        return Err(());
    }

    let mut rows = matrix_rows(&input)?;
    if rows.is_empty() || rows[0].len() < 2 {
        return Err(());
    }
    if rows
        .iter()
        .flat_map(|row| row.iter())
        .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
    {
        return Err(());
    }
    rows.sort_by(|left, right| {
        left[0]
            .partial_cmp(&right[0])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let bands = rows[0].len() - 1;
    let last_index = size - 1;
    let mut out = ImageBuffer::new(
        size,
        1,
        bands,
        VIPS_FORMAT_DOUBLE,
        crate::abi::image::VIPS_CODING_NONE,
        VIPS_INTERPRETATION_HISTOGRAM,
    );
    for band in 0..bands {
        let first = (rows[0][band + 1] * last_index as f64).clamp(0.0, last_index as f64) as usize;
        let last = (rows.last().ok_or(())?[band + 1] * last_index as f64)
            .clamp(0.0, last_index as f64) as usize;

        for k in 0..first {
            let fac = if first == 0 {
                0.0
            } else {
                rows[0][0] / first as f64
            };
            out.set(k, 0, band, k as f64 * fac);
        }

        if rows.len() > 1 && last > first {
            for k in first..last {
                let ki = if last_index == 0 {
                    0.0
                } else {
                    k as f64 / last_index as f64
                };
                let mut j = rows.len() as isize - 1;
                while j >= 0 && rows[j as usize][band + 1] >= ki {
                    j -= 1;
                }
                let j = if j < 0 { 0usize } else { j as usize };
                let next = (j + 1).min(rows.len() - 1);
                let irange = rows[next][band + 1] - rows[j][band + 1];
                let orange = rows[next][0] - rows[j][0];
                let value = if irange.abs() <= f64::EPSILON {
                    rows[j][0]
                } else {
                    rows[j][0] + orange * ((ki - rows[j][band + 1]) / irange)
                };
                out.set(k, 0, band, value);
            }
        }

        for k in last..size {
            let fac = if last >= last_index {
                0.0
            } else {
                (1.0 - rows.last().ok_or(())?[0]) / (last_index - last) as f64
            };
            out.set(
                k,
                0,
                band,
                rows.last().ok_or(())?[0] + (k.saturating_sub(last)) as f64 * fac,
            );
        }
    }
    unsafe { set_output_image(object, "out", out.to_image()) }
}

fn tone_curve_weight(start: f64, peak: f64, end: f64, x: f64) -> f64 {
    if peak <= start || end <= peak {
        return 0.0;
    }
    let smooth = |value: f64| 3.0 * value * value - 2.0 * value * value * value;
    if x < start {
        0.0
    } else if x < peak {
        smooth((x - start) / (peak - start))
    } else if x < end {
        1.0 - smooth((x - peak) / (end - peak))
    } else {
        0.0
    }
}

fn tone_curve_value(
    lb: f64,
    lw: f64,
    ls: f64,
    lm: f64,
    lh: f64,
    s: f64,
    m: f64,
    h: f64,
    x: f64,
) -> f64 {
    x + s * tone_curve_weight(lb, ls, lm, x)
        + m * tone_curve_weight(ls, lm, lh, x)
        + h * tone_curve_weight(lm, lh, lw, x)
}

unsafe fn op_tonelut(object: *mut VipsObject) -> Result<(), ()> {
    let in_max = if unsafe { argument_assigned(object, "in_max")? } {
        usize::try_from(unsafe { get_int(object, "in_max")? }).map_err(|_| ())?
    } else {
        32767
    };
    let out_max = if unsafe { argument_assigned(object, "out_max")? } {
        usize::try_from(unsafe { get_int(object, "out_max")? }).map_err(|_| ())?
    } else {
        32767
    };
    if in_max == 0 || in_max > 65535 || out_max == 0 || out_max > 65535 {
        return Err(());
    }
    let lb = if unsafe { argument_assigned(object, "Lb")? } {
        unsafe { get_double(object, "Lb")? }
    } else {
        0.0
    };
    let lw = if unsafe { argument_assigned(object, "Lw")? } {
        unsafe { get_double(object, "Lw")? }
    } else {
        100.0
    };
    let ps = if unsafe { argument_assigned(object, "Ps")? } {
        unsafe { get_double(object, "Ps")? }
    } else {
        0.2
    };
    let pm = if unsafe { argument_assigned(object, "Pm")? } {
        unsafe { get_double(object, "Pm")? }
    } else {
        0.5
    };
    let ph = if unsafe { argument_assigned(object, "Ph")? } {
        unsafe { get_double(object, "Ph")? }
    } else {
        0.8
    };
    let s = if unsafe { argument_assigned(object, "S")? } {
        unsafe { get_double(object, "S")? }
    } else {
        0.0
    };
    let m = if unsafe { argument_assigned(object, "M")? } {
        unsafe { get_double(object, "M")? }
    } else {
        0.0
    };
    let h = if unsafe { argument_assigned(object, "H")? } {
        unsafe { get_double(object, "H")? }
    } else {
        0.0
    };
    let ls = lb + ps * (lw - lb);
    let lm = lb + pm * (lw - lb);
    let lh = lb + ph * (lw - lb);
    let mut out = ImageBuffer::new(
        in_max + 1,
        1,
        1,
        VIPS_FORMAT_USHORT,
        crate::abi::image::VIPS_CODING_NONE,
        VIPS_INTERPRETATION_HISTOGRAM,
    );
    for index in 0..=in_max {
        let lightness = 100.0 * index as f64 / in_max as f64;
        let value =
            (out_max as f64 / 100.0) * tone_curve_value(lb, lw, ls, lm, lh, s, m, h, lightness);
        out.set(index, 0, 0, value.clamp(0.0, out_max as f64));
    }
    unsafe { set_output_image(object, "out", out.to_image()) }
}

unsafe fn op_zone(object: *mut VipsObject) -> Result<(), ()> {
    let width = usize::try_from(unsafe { get_int(object, "width")? }).map_err(|_| ())?;
    let height = usize::try_from(unsafe { get_int(object, "height")? }).map_err(|_| ())?;
    let uchar =
        unsafe { argument_assigned(object, "uchar")? } && unsafe { get_bool(object, "uchar")? };
    let mut out = ImageBuffer::new(
        width,
        height,
        1,
        if uchar {
            VIPS_FORMAT_UCHAR
        } else {
            VIPS_FORMAT_FLOAT
        },
        crate::abi::image::VIPS_CODING_NONE,
        crate::abi::image::VIPS_INTERPRETATION_B_W,
    );
    let c = std::f64::consts::PI / width.max(1) as f64;
    let hwidth = width as isize / 2;
    let hheight = height as isize / 2;
    for y in 0..height {
        for x in 0..width {
            let h2 = (x as isize - hwidth).pow(2) as f64;
            let v2 = (y as isize - hheight).pow(2) as f64;
            let value = (c * (v2 + h2)).cos();
            out.set(
                x,
                y,
                0,
                if uchar {
                    ((value + 1.0) * 0.5 * 255.0).clamp(0.0, 255.0)
                } else {
                    value
                },
            );
        }
    }
    unsafe { set_output_image(object, "out", out.to_image()) }
}

unsafe fn op_sines(object: *mut VipsObject) -> Result<(), ()> {
    let width = usize::try_from(unsafe { get_int(object, "width")? }).map_err(|_| ())?;
    let height = usize::try_from(unsafe { get_int(object, "height")? }).map_err(|_| ())?;
    let hfreq = if unsafe { argument_assigned(object, "hfreq")? } {
        unsafe { get_double(object, "hfreq")? }
    } else {
        0.5
    };
    let vfreq = if unsafe { argument_assigned(object, "vfreq")? } {
        unsafe { get_double(object, "vfreq")? }
    } else {
        0.5
    };
    let uchar =
        unsafe { argument_assigned(object, "uchar")? } && unsafe { get_bool(object, "uchar")? };
    let theta = if hfreq == 0.0 {
        std::f64::consts::PI / 2.0
    } else {
        (vfreq / hfreq).atan()
    };
    let factor = (hfreq * hfreq + vfreq * vfreq).sqrt();
    let c = factor * std::f64::consts::PI * 2.0 / width.max(1) as f64;
    let costheta = theta.cos();
    let sintheta = theta.sin();
    let mut out = ImageBuffer::new(
        width,
        height,
        1,
        if uchar {
            VIPS_FORMAT_UCHAR
        } else {
            VIPS_FORMAT_FLOAT
        },
        crate::abi::image::VIPS_CODING_NONE,
        crate::abi::image::VIPS_INTERPRETATION_B_W,
    );
    for y in 0..height {
        for x in 0..width {
            let value = (c * (x as f64 * costheta - y as f64 * sintheta)).cos();
            out.set(
                x,
                y,
                0,
                if uchar {
                    ((value + 1.0) * 0.5 * 255.0).clamp(0.0, 255.0)
                } else {
                    value
                },
            );
        }
    }
    unsafe { set_output_image(object, "out", out.to_image()) }
}

unsafe fn op_mask_ideal(object: *mut VipsObject) -> Result<(), ()> {
    let fc = unsafe { get_double(object, "frequency_cutoff")? };
    let fc2 = fc * fc;
    unsafe {
        op_mask_from_point(
            object,
            move |dx, dy| {
                if dx * dx + dy * dy <= fc2 {
                    0.0
                } else {
                    1.0
                }
            },
        )
    }
}

unsafe fn op_mask_ideal_band(object: *mut VipsObject) -> Result<(), ()> {
    let fcx = unsafe { get_double(object, "frequency_cutoff_x")? };
    let fcy = unsafe { get_double(object, "frequency_cutoff_y")? };
    let r2 = unsafe { get_double(object, "radius")? }.powi(2);
    unsafe {
        op_mask_from_point(object, move |dx, dy| {
            let d1 = (dx - fcx) * (dx - fcx) + (dy - fcy) * (dy - fcy);
            let d2 = (dx + fcx) * (dx + fcx) + (dy + fcy) * (dy + fcy);
            if d1 < r2 || d2 < r2 {
                1.0
            } else {
                0.0
            }
        })
    }
}

unsafe fn op_mask_ideal_ring(object: *mut VipsObject) -> Result<(), ()> {
    let fc = unsafe { get_double(object, "frequency_cutoff")? };
    let df = unsafe { get_double(object, "ringwidth")? } / 2.0;
    let fc2_1 = (fc - df) * (fc - df);
    let fc2_2 = (fc + df) * (fc + df);
    unsafe {
        op_mask_from_point(object, move |dx, dy| {
            let dist2 = dx * dx + dy * dy;
            if dist2 > fc2_1 && dist2 < fc2_2 {
                1.0
            } else {
                0.0
            }
        })
    }
}

unsafe fn op_mask_gaussian(object: *mut VipsObject) -> Result<(), ()> {
    let fc = unsafe { get_double(object, "frequency_cutoff")? };
    let ac = unsafe { get_double(object, "amplitude_cutoff")? };
    let fc2 = (fc * fc).max(f64::MIN_POSITIVE);
    let cnst = ac.max(f64::MIN_POSITIVE).ln();
    unsafe {
        op_mask_from_point(object, move |dx, dy| {
            1.0 - (cnst * ((dx * dx + dy * dy) / fc2)).exp()
        })
    }
}

unsafe fn op_mask_gaussian_band(object: *mut VipsObject) -> Result<(), ()> {
    let fcx = unsafe { get_double(object, "frequency_cutoff_x")? };
    let fcy = unsafe { get_double(object, "frequency_cutoff_y")? };
    let r2 = unsafe { get_double(object, "radius")? }
        .powi(2)
        .max(f64::MIN_POSITIVE);
    let ac = unsafe { get_double(object, "amplitude_cutoff")? };
    let cnst = ac.max(f64::MIN_POSITIVE).ln();
    let cnsta = 1.0 / (1.0 + (cnst * 4.0 * (fcx * fcx + fcy * fcy) / r2).exp());
    unsafe {
        op_mask_from_point(object, move |dx, dy| {
            let d1 = (dx - fcx) * (dx - fcx) + (dy - fcy) * (dy - fcy);
            let d2 = (dx + fcx) * (dx + fcx) + (dy + fcy) * (dy + fcy);
            cnsta * ((cnst * d1 / r2).exp() + (cnst * d2 / r2).exp())
        })
    }
}

unsafe fn op_mask_gaussian_ring(object: *mut VipsObject) -> Result<(), ()> {
    let fc = unsafe { get_double(object, "frequency_cutoff")? };
    let df = unsafe { get_double(object, "ringwidth")? } / 2.0;
    let df2 = (df * df).max(f64::MIN_POSITIVE);
    let cnst = unsafe { get_double(object, "amplitude_cutoff")? }
        .max(f64::MIN_POSITIVE)
        .ln();
    unsafe {
        op_mask_from_point(object, move |dx, dy| {
            let dist = (dx * dx + dy * dy).sqrt();
            (cnst * (dist - fc) * (dist - fc) / df2).exp()
        })
    }
}

unsafe fn op_mask_butterworth(object: *mut VipsObject) -> Result<(), ()> {
    let order = unsafe { get_double(object, "order")? };
    let fc = unsafe { get_double(object, "frequency_cutoff")? };
    let ac = unsafe { get_double(object, "amplitude_cutoff")? };
    let cnst = (1.0 / ac.max(f64::MIN_POSITIVE)) - 1.0;
    let fc2 = fc * fc;
    unsafe {
        op_mask_from_point(object, move |dx, dy| {
            let d = dx * dx + dy * dy;
            if d <= f64::EPSILON {
                0.0
            } else {
                1.0 / (1.0 + cnst * (fc2 / d).powf(order))
            }
        })
    }
}

unsafe fn op_mask_butterworth_band(object: *mut VipsObject) -> Result<(), ()> {
    let order = unsafe { get_double(object, "order")? };
    let fcx = unsafe { get_double(object, "frequency_cutoff_x")? };
    let fcy = unsafe { get_double(object, "frequency_cutoff_y")? };
    let r2 = unsafe { get_double(object, "radius")? }
        .powi(2)
        .max(f64::MIN_POSITIVE);
    let ac = unsafe { get_double(object, "amplitude_cutoff")? };
    let cnst = (1.0 / ac.max(f64::MIN_POSITIVE)) - 1.0;
    let cnsta = 1.0 / (1.0 + 1.0 / (1.0 + cnst * (4.0 * (fcx * fcx + fcy * fcy) / r2).powf(order)));
    unsafe {
        op_mask_from_point(object, move |dx, dy| {
            let d1 = (dx - fcx) * (dx - fcx) + (dy - fcy) * (dy - fcy);
            let d2 = (dx + fcx) * (dx + fcx) + (dy + fcy) * (dy + fcy);
            cnsta
                * (1.0 / (1.0 + cnst * (d1 / r2).powf(order))
                    + 1.0 / (1.0 + cnst * (d2 / r2).powf(order)))
        })
    }
}

unsafe fn op_mask_butterworth_ring(object: *mut VipsObject) -> Result<(), ()> {
    let order = unsafe { get_double(object, "order")? };
    let fc = unsafe { get_double(object, "frequency_cutoff")? };
    let ac = unsafe { get_double(object, "amplitude_cutoff")? };
    let df = unsafe { get_double(object, "ringwidth")? } / 2.0;
    let cnst = (1.0 / ac.max(f64::MIN_POSITIVE)) - 1.0;
    let df2 = (df * df).max(f64::MIN_POSITIVE);
    unsafe {
        op_mask_from_point(object, move |dx, dy| {
            let dist = (dx * dx + dy * dy).sqrt();
            1.0 / (1.0 + cnst * (((dist - fc) * (dist - fc)) / df2).powf(order))
        })
    }
}

unsafe fn op_mask_fractal(object: *mut VipsObject) -> Result<(), ()> {
    let fd = (unsafe { get_double(object, "fractal_dimension")? } - 4.0) / 2.0;
    unsafe {
        op_mask_from_point(object, move |dx, dy| {
            let d2 = dx * dx + dy * dy;
            if d2 <= f64::EPSILON {
                0.0
            } else {
                d2.powf(fd)
            }
        })
    }
}

pub(crate) unsafe fn dispatch(object: *mut VipsObject, nickname: &str) -> Result<bool, ()> {
    match nickname {
        "black" => {
            unsafe { op_black(object)? };
            Ok(true)
        }
        "grey" => {
            unsafe { op_grey(object)? };
            Ok(true)
        }
        "xyz" => {
            unsafe { op_xyz(object)? };
            Ok(true)
        }
        "text" => {
            unsafe { op_text(object)? };
            Ok(true)
        }
        "identity" => {
            unsafe { op_identity(object)? };
            Ok(true)
        }
        "eye" => {
            unsafe { op_eye(object)? };
            Ok(true)
        }
        "buildlut" => {
            unsafe { op_buildlut(object)? };
            Ok(true)
        }
        "gaussmat" => {
            unsafe { op_matrix_kernel(object, gaussian_kernel)? };
            Ok(true)
        }
        "gaussnoise" => {
            unsafe { op_gaussnoise(object)? };
            Ok(true)
        }
        "invertlut" => {
            unsafe { op_invertlut(object)? };
            Ok(true)
        }
        "logmat" => {
            unsafe { op_matrix_kernel(object, log_kernel)? };
            Ok(true)
        }
        "mask_fractal" => {
            unsafe { op_mask_fractal(object)? };
            Ok(true)
        }
        "mask_butterworth" => {
            unsafe { op_mask_butterworth(object)? };
            Ok(true)
        }
        "mask_butterworth_band" => {
            unsafe { op_mask_butterworth_band(object)? };
            Ok(true)
        }
        "mask_butterworth_ring" => {
            unsafe { op_mask_butterworth_ring(object)? };
            Ok(true)
        }
        "mask_gaussian" => {
            unsafe { op_mask_gaussian(object)? };
            Ok(true)
        }
        "mask_gaussian_band" => {
            unsafe { op_mask_gaussian_band(object)? };
            Ok(true)
        }
        "mask_gaussian_ring" => {
            unsafe { op_mask_gaussian_ring(object)? };
            Ok(true)
        }
        "mask_ideal" => {
            unsafe { op_mask_ideal(object)? };
            Ok(true)
        }
        "mask_ideal_band" => {
            unsafe { op_mask_ideal_band(object)? };
            Ok(true)
        }
        "mask_ideal_ring" => {
            unsafe { op_mask_ideal_ring(object)? };
            Ok(true)
        }
        "sines" => {
            unsafe { op_sines(object)? };
            Ok(true)
        }
        "tonelut" => {
            unsafe { op_tonelut(object)? };
            Ok(true)
        }
        "zone" => {
            unsafe { op_zone(object)? };
            Ok(true)
        }
        _ => Ok(false),
    }
}
