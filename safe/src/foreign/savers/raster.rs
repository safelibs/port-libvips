use crate::abi::image::{VipsImage, VIPS_FORMAT_UCHAR, VIPS_FORMAT_USHORT};
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

fn tiff_pixels(image_ref: &VipsImage, pixels: &[u8]) -> Result<Vec<u8>, ()> {
    let expected = image_ref.Xsize.max(0) as usize
        * image_ref.Ysize.max(0) as usize
        * image_ref.Bands.max(0) as usize
        * match image_ref.BandFmt {
            VIPS_FORMAT_UCHAR => 1,
            VIPS_FORMAT_USHORT => 2,
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
    if !matches!(bands, 1 | 3 | 4) {
        return Ok(None);
    }

    let bits_per_sample = match image_ref.BandFmt {
        VIPS_FORMAT_UCHAR => 8u16,
        VIPS_FORMAT_USHORT => 16u16,
        _ => return Ok(None),
    };
    let pixel_data = tiff_pixels(image_ref, &pixels)?;
    let entry_count = 10u16 + u16::from(bands == 4);
    let ifd_size = 2usize + entry_count as usize * 12 + 4;
    let mut extra = Vec::new();
    let bits_offset = if bands > 1 {
        let offset = 8 + ifd_size as u32;
        for _ in 0..bands {
            push_u16(&mut extra, bits_per_sample);
        }
        Some(offset)
    } else {
        None
    };
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
    push_ifd_entry(
        &mut out,
        258,
        3,
        bands as u32,
        bits_offset.unwrap_or(bits_per_sample as u32),
    );
    push_ifd_entry(&mut out, 259, 3, 1, 1);
    push_ifd_entry(&mut out, 262, 3, 1, if bands == 1 { 1 } else { 2 });
    push_ifd_entry(&mut out, 273, 4, 1, pixel_offset);
    push_ifd_entry(&mut out, 277, 3, 1, bands as u32);
    push_ifd_entry(&mut out, 278, 4, 1, height as u32);
    push_ifd_entry(&mut out, 279, 4, 1, pixel_data.len() as u32);
    push_ifd_entry(&mut out, 284, 3, 1, 1);
    if bands == 4 {
        push_ifd_entry(&mut out, 338, 3, 1, 2);
    }
    push_u32(&mut out, 0);
    out.extend_from_slice(&extra);
    out.extend_from_slice(&pixel_data);
    Ok(Some(out))
}
