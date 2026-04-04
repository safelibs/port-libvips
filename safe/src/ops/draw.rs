use std::collections::VecDeque;

use crate::abi::basic::{
    VipsCombineMode, VIPS_COMBINE_MODE_ADD, VIPS_COMBINE_MODE_SET,
};
use crate::abi::object::VipsObject;
use crate::pixels::format::{format_bytes, write_sample};
use crate::pixels::ImageBuffer;
use crate::runtime::image::{image_state, sync_pixels, vips_image_invalidate_all};
use crate::runtime::object::object_unref;

use super::{
    argument_assigned, get_array_double, get_bool, get_enum, get_image_ref, get_int,
};

fn resolve_values(values: &[f64], bands: usize) -> Vec<f64> {
    if values.is_empty() {
        vec![0.0; bands]
    } else {
        (0..bands)
            .map(|band| values.get(band).copied().unwrap_or(values[0]))
            .collect()
    }
}

fn clip_rect(
    width: usize,
    height: usize,
    left: i32,
    top: i32,
    rect_width: i32,
    rect_height: i32,
) -> Option<(usize, usize, usize, usize)> {
    if rect_width <= 0 || rect_height <= 0 {
        return None;
    }
    let x0 = left.max(0) as usize;
    let y0 = top.max(0) as usize;
    let x1 = left.saturating_add(rect_width).max(0) as usize;
    let y1 = top.saturating_add(rect_height).max(0) as usize;
    let x1 = x1.min(width);
    let y1 = y1.min(height);
    if x0 >= x1 || y0 >= y1 {
        None
    } else {
        Some((x0, y0, x1, y1))
    }
}

fn write_back(image: *mut crate::abi::image::VipsImage, buffer: &ImageBuffer) -> Result<(), ()> {
    let Some(image_ref) = (unsafe { image.as_mut() }) else {
        return Err(());
    };
    let Some(state) = (unsafe { image_state(image) }) else {
        return Err(());
    };
    let sample_size = format_bytes(image_ref.BandFmt);
    if sample_size == 0 {
        return Err(());
    }

    state.pixels.resize(buffer.sample_count().saturating_mul(sample_size), 0);
    for (index, value) in buffer.data.iter().copied().enumerate() {
        let offset = index * sample_size;
        let end = offset + sample_size;
        let _ = write_sample(&mut state.pixels[offset..end], image_ref.BandFmt, value);
    }
    sync_pixels(image);
    vips_image_invalidate_all(image);
    Ok(())
}

fn put_pixel(buffer: &mut ImageBuffer, x: i32, y: i32, ink: &[f64]) {
    if x < 0 || y < 0 {
        return;
    }
    let x = x as usize;
    let y = y as usize;
    if x >= buffer.spec.width || y >= buffer.spec.height {
        return;
    }
    for band in 0..buffer.spec.bands {
        buffer.set(x, y, band, ink[band]);
    }
}

fn put_pixel_mode(buffer: &mut ImageBuffer, x: i32, y: i32, values: &[f64], mode: VipsCombineMode) {
    if x < 0 || y < 0 {
        return;
    }
    let x = x as usize;
    let y = y as usize;
    if x >= buffer.spec.width || y >= buffer.spec.height {
        return;
    }
    for band in 0..buffer.spec.bands {
        let current = buffer.get(x, y, band);
        let next = match mode {
            VIPS_COMBINE_MODE_ADD => current + values[band],
            _ => values[band],
        };
        buffer.set(x, y, band, next);
    }
}

fn draw_rect(buffer: &mut ImageBuffer, ink: &[f64], left: i32, top: i32, width: i32, height: i32, fill: bool) {
    let Some((x0, y0, x1, y1)) = clip_rect(buffer.spec.width, buffer.spec.height, left, top, width, height) else {
        return;
    };
    if fill {
        for y in y0..y1 {
            for x in x0..x1 {
                for band in 0..buffer.spec.bands {
                    buffer.set(x, y, band, ink[band]);
                }
            }
        }
    } else {
        for x in x0..x1 {
            for band in 0..buffer.spec.bands {
                buffer.set(x, y0, band, ink[band]);
                buffer.set(x, y1 - 1, band, ink[band]);
            }
        }
        for y in y0..y1 {
            for band in 0..buffer.spec.bands {
                buffer.set(x0, y, band, ink[band]);
                buffer.set(x1 - 1, y, band, ink[band]);
            }
        }
    }
}

fn draw_line(buffer: &mut ImageBuffer, ink: &[f64], x1: i32, y1: i32, x2: i32, y2: i32) {
    let mut x = x1;
    let mut y = y1;
    let dx = (x2 - x1).abs();
    let sx = if x1 < x2 { 1 } else { -1 };
    let dy = -(y2 - y1).abs();
    let sy = if y1 < y2 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        put_pixel(buffer, x, y, ink);
        if x == x2 && y == y2 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

fn draw_circle(buffer: &mut ImageBuffer, ink: &[f64], cx: i32, cy: i32, radius: i32, fill: bool) {
    if radius < 0 {
        return;
    }
    let radius_sq = radius * radius;
    for y in (cy - radius)..=(cy + radius) {
        for x in (cx - radius)..=(cx + radius) {
            let dx = x - cx;
            let dy = y - cy;
            let dist_sq = dx * dx + dy * dy;
            let paint = if fill {
                dist_sq <= radius_sq
            } else {
                dist_sq <= radius_sq && dist_sq >= (radius - 1).max(0).pow(2)
            };
            if paint {
                put_pixel(buffer, x, y, ink);
            }
        }
    }
}

fn draw_image(buffer: &mut ImageBuffer, sub: &ImageBuffer, x: i32, y: i32, mode: VipsCombineMode) {
    for sy in 0..sub.spec.height {
        for sx in 0..sub.spec.width {
            let dx = x + sx as i32;
            let dy = y + sy as i32;
            let values = (0..buffer.spec.bands)
                .map(|band| {
                    let sub_band = band.min(sub.spec.bands.saturating_sub(1));
                    sub.get(sx, sy, sub_band)
                })
                .collect::<Vec<_>>();
            put_pixel_mode(buffer, dx, dy, &values, mode);
        }
    }
}

fn mask_alpha(mask: &ImageBuffer, x: usize, y: usize) -> f64 {
    let value = mask.get(x, y, 0);
    match mask.spec.format {
        crate::abi::image::VIPS_FORMAT_UCHAR => (value / 255.0).clamp(0.0, 1.0),
        crate::abi::image::VIPS_FORMAT_USHORT => (value / 65_535.0).clamp(0.0, 1.0),
        _ => value.clamp(0.0, 1.0),
    }
}

fn draw_mask(buffer: &mut ImageBuffer, ink: &[f64], mask: &ImageBuffer, x: i32, y: i32) {
    for my in 0..mask.spec.height {
        for mx in 0..mask.spec.width {
            let dx = x + mx as i32;
            let dy = y + my as i32;
            if dx < 0 || dy < 0 {
                continue;
            }
            let dx = dx as usize;
            let dy = dy as usize;
            if dx >= buffer.spec.width || dy >= buffer.spec.height {
                continue;
            }
            let alpha = mask_alpha(mask, mx, my);
            for band in 0..buffer.spec.bands {
                let current = buffer.get(dx, dy, band);
                buffer.set(dx, dy, band, current * (1.0 - alpha) + ink[band] * alpha);
            }
        }
    }
}

fn draw_smudge(buffer: &mut ImageBuffer, left: i32, top: i32, width: i32, height: i32) {
    let Some((x0, y0, x1, y1)) = clip_rect(buffer.spec.width, buffer.spec.height, left, top, width, height) else {
        return;
    };
    let source = buffer.clone();
    for y in y0..y1 {
        for x in x0..x1 {
            for band in 0..buffer.spec.bands {
                let mut sum = 0.0;
                let mut count = 0.0;
                for oy in -1..=1 {
                    for ox in -1..=1 {
                        sum += source.sample_clamped(x as isize + ox, y as isize + oy, band);
                        count += 1.0;
                    }
                }
                buffer.set(x, y, band, sum / count);
            }
        }
    }
}

fn same_pixel(buffer: &ImageBuffer, x: usize, y: usize, test: &[f64]) -> bool {
    (0..buffer.spec.bands).all(|band| (buffer.get(x, y, band) - test[band]).abs() < f64::EPSILON)
}

fn draw_flood(
    buffer: &mut ImageBuffer,
    ink: &[f64],
    x: i32,
    y: i32,
    test: &[f64],
    bounds: (usize, usize, usize, usize),
) {
    if x < bounds.0 as i32 || y < bounds.1 as i32 {
        return;
    }
    let x = x as usize;
    let y = y as usize;
    if x >= bounds.2 || y >= bounds.3 || !same_pixel(buffer, x, y, test) {
        return;
    }

    let mut queue = VecDeque::new();
    let mut visited = vec![false; buffer.spec.width.saturating_mul(buffer.spec.height)];
    queue.push_back((x, y));
    while let Some((cx, cy)) = queue.pop_front() {
        let index = cy * buffer.spec.width + cx;
        if visited[index] || !same_pixel(buffer, cx, cy, test) {
            continue;
        }
        visited[index] = true;
        for band in 0..buffer.spec.bands {
            buffer.set(cx, cy, band, ink[band]);
        }

        let neighbours = [
            (cx.wrapping_sub(1), cy),
            (cx + 1, cy),
            (cx, cy.wrapping_sub(1)),
            (cx, cy + 1),
        ];
        for (nx, ny) in neighbours {
            if nx >= bounds.0 && nx < bounds.2 && ny >= bounds.1 && ny < bounds.3 {
                queue.push_back((nx, ny));
            }
        }
    }
}

unsafe fn op_draw_rect(object: *mut VipsObject) -> Result<(), ()> {
    let image = unsafe { get_image_ref(object, "image")? };
    let mut buffer = ImageBuffer::from_image(image)?;
    let ink = resolve_values(&unsafe { get_array_double(object, "ink")? }, buffer.spec.bands);
    let left = unsafe { get_int(object, "left")? };
    let top = unsafe { get_int(object, "top")? };
    let width = unsafe { get_int(object, "width")? };
    let height = unsafe { get_int(object, "height")? };
    let fill = unsafe { argument_assigned(object, "fill")? } && unsafe { get_bool(object, "fill")? };
    draw_rect(&mut buffer, &ink, left, top, width, height, fill);
    let result = write_back(image, &buffer);
    unsafe {
        object_unref(image);
    }
    result
}

unsafe fn op_draw_line(object: *mut VipsObject) -> Result<(), ()> {
    let image = unsafe { get_image_ref(object, "image")? };
    let mut buffer = ImageBuffer::from_image(image)?;
    let ink = resolve_values(&unsafe { get_array_double(object, "ink")? }, buffer.spec.bands);
    draw_line(
        &mut buffer,
        &ink,
        unsafe { get_int(object, "x1")? },
        unsafe { get_int(object, "y1")? },
        unsafe { get_int(object, "x2")? },
        unsafe { get_int(object, "y2")? },
    );
    let result = write_back(image, &buffer);
    unsafe {
        object_unref(image);
    }
    result
}

unsafe fn op_draw_circle(object: *mut VipsObject) -> Result<(), ()> {
    let image = unsafe { get_image_ref(object, "image")? };
    let mut buffer = ImageBuffer::from_image(image)?;
    let ink = resolve_values(&unsafe { get_array_double(object, "ink")? }, buffer.spec.bands);
    let fill = unsafe { argument_assigned(object, "fill")? } && unsafe { get_bool(object, "fill")? };
    draw_circle(
        &mut buffer,
        &ink,
        unsafe { get_int(object, "cx")? },
        unsafe { get_int(object, "cy")? },
        unsafe { get_int(object, "radius")? },
        fill,
    );
    let result = write_back(image, &buffer);
    unsafe {
        object_unref(image);
    }
    result
}

unsafe fn op_draw_image(object: *mut VipsObject) -> Result<(), ()> {
    let image = unsafe { get_image_ref(object, "image")? };
    let mut buffer = ImageBuffer::from_image(image)?;
    let sub = unsafe { get_image_ref(object, "sub")? };
    let sub_buffer = ImageBuffer::from_image(sub)?;
    let mode = if unsafe { argument_assigned(object, "mode")? } {
        unsafe { get_enum(object, "mode")? as VipsCombineMode }
    } else {
        VIPS_COMBINE_MODE_SET
    };
    draw_image(
        &mut buffer,
        &sub_buffer,
        unsafe { get_int(object, "x")? },
        unsafe { get_int(object, "y")? },
        mode,
    );
    let result = write_back(image, &buffer);
    unsafe {
        object_unref(sub);
        object_unref(image);
    }
    result
}

unsafe fn op_draw_mask(object: *mut VipsObject) -> Result<(), ()> {
    let image = unsafe { get_image_ref(object, "image")? };
    let mut buffer = ImageBuffer::from_image(image)?;
    let ink = resolve_values(&unsafe { get_array_double(object, "ink")? }, buffer.spec.bands);
    let mask = unsafe { get_image_ref(object, "mask")? };
    let mask_buffer = ImageBuffer::from_image(mask)?;
    draw_mask(
        &mut buffer,
        &ink,
        &mask_buffer,
        unsafe { get_int(object, "x")? },
        unsafe { get_int(object, "y")? },
    );
    let result = write_back(image, &buffer);
    unsafe {
        object_unref(mask);
        object_unref(image);
    }
    result
}

unsafe fn op_draw_smudge(object: *mut VipsObject) -> Result<(), ()> {
    let image = unsafe { get_image_ref(object, "image")? };
    let mut buffer = ImageBuffer::from_image(image)?;
    draw_smudge(
        &mut buffer,
        unsafe { get_int(object, "left")? },
        unsafe { get_int(object, "top")? },
        unsafe { get_int(object, "width")? },
        unsafe { get_int(object, "height")? },
    );
    let result = write_back(image, &buffer);
    unsafe {
        object_unref(image);
    }
    result
}

unsafe fn op_draw_flood(object: *mut VipsObject) -> Result<(), ()> {
    let image = unsafe { get_image_ref(object, "image")? };
    let mut buffer = ImageBuffer::from_image(image)?;
    let ink = resolve_values(&unsafe { get_array_double(object, "ink")? }, buffer.spec.bands);
    let x = unsafe { get_int(object, "x")? };
    let y = unsafe { get_int(object, "y")? };
    let test = if unsafe { argument_assigned(object, "test")? } {
        resolve_values(&unsafe { get_array_double(object, "test")? }, buffer.spec.bands)
    } else if x >= 0
        && y >= 0
        && (x as usize) < buffer.spec.width
        && (y as usize) < buffer.spec.height
    {
        (0..buffer.spec.bands)
            .map(|band| buffer.get(x as usize, y as usize, band))
            .collect::<Vec<_>>()
    } else {
        vec![0.0; buffer.spec.bands]
    };
    let left = if unsafe { argument_assigned(object, "left")? } {
        unsafe { get_int(object, "left")? }
    } else {
        0
    };
    let top = if unsafe { argument_assigned(object, "top")? } {
        unsafe { get_int(object, "top")? }
    } else {
        0
    };
    let width = if unsafe { argument_assigned(object, "width")? } {
        unsafe { get_int(object, "width")? }
    } else {
        buffer.spec.width as i32
    };
    let height = if unsafe { argument_assigned(object, "height")? } {
        unsafe { get_int(object, "height")? }
    } else {
        buffer.spec.height as i32
    };
    if let Some(bounds) = clip_rect(buffer.spec.width, buffer.spec.height, left, top, width, height) {
        draw_flood(&mut buffer, &ink, x, y, &test, bounds);
    }
    let result = write_back(image, &buffer);
    unsafe {
        object_unref(image);
    }
    result
}

pub(crate) unsafe fn dispatch(object: *mut VipsObject, nickname: &str) -> Result<bool, ()> {
    match nickname {
        "draw_rect" => {
            unsafe { op_draw_rect(object)? };
            Ok(true)
        }
        "draw_line" => {
            unsafe { op_draw_line(object)? };
            Ok(true)
        }
        "draw_circle" => {
            unsafe { op_draw_circle(object)? };
            Ok(true)
        }
        "draw_image" => {
            unsafe { op_draw_image(object)? };
            Ok(true)
        }
        "draw_mask" => {
            unsafe { op_draw_mask(object)? };
            Ok(true)
        }
        "draw_smudge" => {
            unsafe { op_draw_smudge(object)? };
            Ok(true)
        }
        "draw_flood" => {
            unsafe { op_draw_flood(object)? };
            Ok(true)
        }
        _ => Ok(false),
    }
}
