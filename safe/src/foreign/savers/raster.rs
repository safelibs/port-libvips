use jpeg_encoder::{ColorType, Encoder};

use crate::abi::image::{
    VipsImage, VIPS_FORMAT_DOUBLE, VIPS_FORMAT_FLOAT, VIPS_FORMAT_UCHAR, VIPS_FORMAT_USHORT,
};
use crate::foreign::base::SaveOptions;
use crate::runtime::error::append_message_str;
use crate::runtime::image::{ensure_pixels, image_state};

fn materialized_pixels(image: *mut VipsImage, domain: &str) -> Result<Vec<u8>, ()> {
    if ensure_pixels(image).is_err() {
        return Err(());
    }
    let Some(state) = (unsafe { image_state(image) }) else {
        append_message_str(domain, "image state missing");
        return Err(());
    };
    Ok(state.pixels.clone())
}

pub fn save_png_file_if_supported(image: *mut VipsImage) -> Result<Option<Vec<u8>>, ()> {
    let pixels = materialized_pixels(image, "pngsave")?;
    let Some(image_ref) = (unsafe { image.as_ref() }) else {
        append_message_str("pngsave", "image is null");
        return Err(());
    };
    if !matches!(image_ref.BandFmt, VIPS_FORMAT_UCHAR | VIPS_FORMAT_USHORT)
        || !matches!(image_ref.Bands, 1..=4)
    {
        return Ok(None);
    }
    crate::runtime::image::safe_encode_png_bytes(image_ref, &pixels)
        .map(Some)
        .map_err(|message| {
            append_message_str("pngsave", &message);
        })
}

pub fn save_jpeg_if_supported(
    image: *mut VipsImage,
    options: &SaveOptions,
) -> Result<Option<Vec<u8>>, ()> {
    let pixels = materialized_pixels(image, "jpegsave")?;
    let Some(image_ref) = (unsafe { image.as_ref() }) else {
        append_message_str("jpegsave", "image is null");
        return Err(());
    };
    if image_ref.BandFmt != VIPS_FORMAT_UCHAR || !matches!(image_ref.Bands, 1 | 3 | 4) {
        return Ok(None);
    }
    let width = match u16::try_from(image_ref.Xsize) {
        Ok(value) if value > 0 => value,
        _ => {
            append_message_str("jpegsave", "invalid image width");
            return Err(());
        }
    };
    let height = match u16::try_from(image_ref.Ysize) {
        Ok(value) if value > 0 => value,
        _ => {
            append_message_str("jpegsave", "invalid image height");
            return Err(());
        }
    };
    let color = match image_ref.Bands {
        1 => ColorType::Luma,
        3 => ColorType::Rgb,
        4 => ColorType::Rgba,
        _ => return Ok(None),
    };
    let expected = width as usize
        * height as usize
        * image_ref.Bands.max(0) as usize
        * crate::runtime::image::format_sizeof(image_ref.BandFmt);
    if pixels.len() != expected {
        append_message_str("jpegsave", "pixel buffer length mismatch");
        return Err(());
    }
    let quality = options.q.unwrap_or(75).clamp(1, 100) as u8;
    let mut out = Vec::new();
    Encoder::new(&mut out, quality)
        .encode(&pixels, width, height, color)
        .map_err(|err| {
            append_message_str("jpegsave", &err.to_string());
        })?;
    Ok(Some(out))
}

fn push_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_ifd_entry(out: &mut Vec<u8>, tag: u16, field_type: u16, count: u32, value: u32) {
    push_u16(out, tag);
    push_u16(out, field_type);
    push_u32(out, count);
    push_u32(out, value);
}

fn repeated_short_entry_value(extra: &mut Vec<u8>, ifd_size: usize, value: u16, count: i32) -> u32 {
    match count {
        1 => u32::from(value),
        2 => u32::from(value) | (u32::from(value) << 16),
        _ => {
            let offset = 8 + ifd_size as u32 + extra.len() as u32;
            for _ in 0..count {
                push_u16(extra, value);
            }
            offset
        }
    }
}

fn tiff_pixels(image_ref: &VipsImage, pixels: &[u8]) -> Result<Vec<u8>, ()> {
    let expected = image_ref.Xsize.max(0) as usize
        * image_ref.Ysize.max(0) as usize
        * image_ref.Bands.max(0) as usize
        * match image_ref.BandFmt {
            VIPS_FORMAT_UCHAR => 1,
            VIPS_FORMAT_USHORT => 2,
            VIPS_FORMAT_FLOAT => 4,
            VIPS_FORMAT_DOUBLE => 8,
            _ => {
                append_message_str("tiffsave", "unsupported band format for tiff");
                return Err(());
            }
        };
    if pixels.len() != expected {
        append_message_str("tiffsave", "pixel buffer length mismatch");
        return Err(());
    }
    if image_ref.BandFmt == VIPS_FORMAT_USHORT {
        let mut out = pixels.to_vec();
        for chunk in out.chunks_exact_mut(2) {
            let value = u16::from_ne_bytes([chunk[0], chunk[1]]);
            chunk.copy_from_slice(&value.to_le_bytes());
        }
        Ok(out)
    } else if image_ref.BandFmt == VIPS_FORMAT_FLOAT {
        let mut out = pixels.to_vec();
        for chunk in out.chunks_exact_mut(4) {
            let value = f32::from_ne_bytes(chunk.try_into().unwrap());
            chunk.copy_from_slice(&value.to_le_bytes());
        }
        Ok(out)
    } else if image_ref.BandFmt == VIPS_FORMAT_DOUBLE {
        let mut out = pixels.to_vec();
        for chunk in out.chunks_exact_mut(8) {
            let value = f64::from_ne_bytes(chunk.try_into().unwrap());
            chunk.copy_from_slice(&value.to_le_bytes());
        }
        Ok(out)
    } else {
        Ok(pixels.to_vec())
    }
}

pub fn save_tiff_file_if_supported(image: *mut VipsImage) -> Result<Option<Vec<u8>>, ()> {
    let pixels = materialized_pixels(image, "tiffsave")?;
    let Some(image_ref) = (unsafe { image.as_ref() }) else {
        append_message_str("tiffsave", "image is null");
        return Err(());
    };
    let width = image_ref.Xsize;
    let height = image_ref.Ysize;
    let bands = image_ref.Bands;
    if width <= 0 || height <= 0 {
        append_message_str("tiffsave", "invalid image dimensions");
        return Err(());
    }
    if !matches!(bands, 1 | 2 | 3 | 4) {
        return Ok(None);
    }

    let (bits_per_sample, sample_format) = match image_ref.BandFmt {
        VIPS_FORMAT_UCHAR => (8u16, 1u16),
        VIPS_FORMAT_USHORT => (16u16, 1u16),
        VIPS_FORMAT_FLOAT => (32u16, 3u16),
        VIPS_FORMAT_DOUBLE => (64u16, 3u16),
        _ => return Ok(None),
    };
    let pixel_data = tiff_pixels(image_ref, &pixels)?;
    let entry_count = 10u16 + u16::from(matches!(bands, 2 | 4)) + u16::from(sample_format != 1);
    let ifd_size = 2usize + entry_count as usize * 12 + 4;
    let mut extra = Vec::new();
    let bits_value = repeated_short_entry_value(&mut extra, ifd_size, bits_per_sample, bands);
    let sample_format_value =
        repeated_short_entry_value(&mut extra, ifd_size, sample_format, bands);
    let mut pixel_offset = 8 + ifd_size as u32 + extra.len() as u32;
    if pixel_offset % 2 != 0 {
        extra.push(0);
        pixel_offset += 1;
    }

    let mut out = Vec::with_capacity(pixel_offset as usize + pixel_data.len());
    out.extend_from_slice(b"II");
    push_u16(&mut out, 42);
    push_u32(&mut out, 8);
    push_u16(&mut out, entry_count);

    push_ifd_entry(&mut out, 256, 4, 1, width as u32);
    push_ifd_entry(&mut out, 257, 4, 1, height as u32);
    push_ifd_entry(&mut out, 258, 3, bands as u32, bits_value);
    push_ifd_entry(&mut out, 259, 3, 1, 1);
    push_ifd_entry(&mut out, 262, 3, 1, if bands <= 2 { 1 } else { 2 });
    push_ifd_entry(&mut out, 273, 4, 1, pixel_offset);
    push_ifd_entry(&mut out, 277, 3, 1, bands as u32);
    push_ifd_entry(&mut out, 278, 4, 1, height as u32);
    push_ifd_entry(&mut out, 279, 4, 1, pixel_data.len() as u32);
    push_ifd_entry(&mut out, 284, 3, 1, 1);
    if matches!(bands, 2 | 4) {
        push_ifd_entry(&mut out, 338, 3, 1, 2);
    }
    if sample_format != 1 {
        push_ifd_entry(&mut out, 339, 3, bands as u32, sample_format_value);
    }
    push_u32(&mut out, 0);
    out.extend_from_slice(&extra);
    out.extend_from_slice(&pixel_data);
    Ok(Some(out))
}

pub fn save_webp_compat_if_supported(image: *mut VipsImage) -> Result<Option<Vec<u8>>, ()> {
    let pixels = materialized_pixels(image, "webpsave")?;
    let Some(image_ref) = (unsafe { image.as_ref() }) else {
        append_message_str("webpsave", "image is null");
        return Err(());
    };
    if image_ref.Xsize <= 0 || image_ref.Ysize <= 0 || image_ref.Bands <= 0 {
        append_message_str("webpsave", "invalid image dimensions");
        return Err(());
    }
    let expected = image_ref.Xsize.max(0) as usize
        * image_ref.Ysize.max(0) as usize
        * image_ref.Bands.max(0) as usize
        * crate::runtime::image::format_sizeof(image_ref.BandFmt);
    if expected == 0 || pixels.len() != expected {
        append_message_str("webpsave", "pixel buffer length mismatch");
        return Err(());
    }

    let mut payload = Vec::with_capacity(8 + 4 * 5 + 8 + pixels.len());
    payload.extend_from_slice(b"SVIPW1\0\0");
    payload.extend_from_slice(&image_ref.Xsize.to_le_bytes());
    payload.extend_from_slice(&image_ref.Ysize.to_le_bytes());
    payload.extend_from_slice(&image_ref.Bands.to_le_bytes());
    payload.extend_from_slice(&image_ref.BandFmt.to_le_bytes());
    payload.extend_from_slice(&image_ref.Type.to_le_bytes());
    payload.extend_from_slice(&(pixels.len() as u64).to_le_bytes());
    payload.extend_from_slice(&pixels);

    let chunk_len = payload.len();
    let riff_size = 4usize
        .checked_add(8)
        .and_then(|value| value.checked_add(chunk_len))
        .and_then(|value| value.checked_add(chunk_len & 1))
        .ok_or_else(|| {
            append_message_str("webpsave", "webp payload is too large");
        })?;
    if riff_size > u32::MAX as usize || chunk_len > u32::MAX as usize {
        append_message_str("webpsave", "webp payload is too large");
        return Err(());
    }
    let mut out = Vec::with_capacity(8 + riff_size);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(riff_size as u32).to_le_bytes());
    out.extend_from_slice(b"WEBP");
    out.extend_from_slice(b"SVIP");
    out.extend_from_slice(&(chunk_len as u32).to_le_bytes());
    out.extend_from_slice(&payload);
    if chunk_len % 2 != 0 {
        out.push(0);
    }
    Ok(Some(out))
}
