use crate::abi::image::{VipsImage, VIPS_FORMAT_UCHAR, VIPS_FORMAT_USHORT};
use crate::foreign::base::{ForeignKind, SaveOptions, CONTAINER_MAGIC};
use crate::foreign::metadata::collect_metadata;
use crate::runtime::error::append_message_str;
use crate::runtime::image::{ensure_pixels, image_state};

const FLAG_PACKED_GRAY: u32 = 1 << 0;

fn kind_tag(kind: ForeignKind) -> u8 {
    match kind {
        ForeignKind::Jpeg => 1,
        ForeignKind::Png => 2,
        ForeignKind::Gif => 3,
        ForeignKind::Tiff => 4,
        ForeignKind::Webp => 5,
        ForeignKind::Heif => 15,
        ForeignKind::Svg => 6,
        ForeignKind::Pdf => 7,
        ForeignKind::Ppm => 8,
        ForeignKind::Pfm => 9,
        ForeignKind::Csv => 10,
        ForeignKind::Matrix => 11,
        ForeignKind::Raw => 12,
        ForeignKind::Vips => 13,
        ForeignKind::Radiance => 14,
        ForeignKind::Unknown => 0,
    }
}

fn png_bits_per_sample(image: &VipsImage, options: &SaveOptions) -> i32 {
    match image.BandFmt {
        VIPS_FORMAT_USHORT => 16,
        _ => match options.bitdepth.unwrap_or(8) {
            1 | 2 | 4 | 8 => options.bitdepth.unwrap_or(8),
            _ => 8,
        },
    }
}

fn quantize_gray_sample(value: u8, bits_per_sample: i32) -> u8 {
    match bits_per_sample {
        1 => u8::from(value >= 128),
        2 => ((value as u16 * 3 + 127) / 255) as u8,
        4 => ((value as u16 * 15 + 127) / 255) as u8,
        _ => value,
    }
}

fn pack_grayscale_pixels(pixels: &[u8], bits_per_sample: i32) -> Vec<u8> {
    let samples_per_byte = 8 / bits_per_sample as usize;
    let mut out = Vec::with_capacity(pixels.len().div_ceil(samples_per_byte));
    let mut byte = 0u8;
    let mut in_byte = 0usize;
    for &value in pixels {
        let quantized = quantize_gray_sample(value, bits_per_sample);
        let shift = 8 - bits_per_sample as usize * (in_byte + 1);
        byte |= quantized << shift;
        in_byte += 1;
        if in_byte == samples_per_byte {
            out.push(byte);
            byte = 0;
            in_byte = 0;
        }
    }
    if in_byte != 0 {
        out.push(byte);
    }
    out
}

pub fn write_container(
    image: *mut VipsImage,
    kind: ForeignKind,
    options: &SaveOptions,
) -> Result<Vec<u8>, ()> {
    if ensure_pixels(image).is_err() {
        return Err(());
    }
    let Some(image_ref) = (unsafe { image.as_ref() }) else {
        return Err(());
    };
    let Some(state) = (unsafe { image_state(image) }) else {
        return Err(());
    };

    let mut metadata = collect_metadata(image, options);
    let mut flags = 0u32;
    let payload = if kind == ForeignKind::Png {
        let bits_per_sample = png_bits_per_sample(image_ref, options);
        metadata
            .ints
            .insert("bits-per-sample".to_owned(), bits_per_sample);
        if image_ref.Bands == 1
            && image_ref.BandFmt == VIPS_FORMAT_UCHAR
            && matches!(bits_per_sample, 1 | 2 | 4)
        {
            flags |= FLAG_PACKED_GRAY;
            pack_grayscale_pixels(&state.pixels, bits_per_sample)
        } else {
            state.pixels.clone()
        }
    } else {
        state.pixels.clone()
    };

    let mut out = Vec::new();
    out.extend_from_slice(CONTAINER_MAGIC);
    out.push(kind_tag(kind));
    out.push(image_ref.Bands.clamp(0, 255) as u8);
    out.push(image_ref.BandFmt.clamp(0, u8::MAX as i32) as u8);
    out.extend_from_slice(&image_ref.Xsize.to_le_bytes());
    out.extend_from_slice(&image_ref.Ysize.to_le_bytes());
    out.extend_from_slice(&image_ref.Coding.to_le_bytes());
    out.extend_from_slice(&image_ref.Type.to_le_bytes());
    out.extend_from_slice(&flags.to_le_bytes());
    out.extend_from_slice(&(metadata.blobs.len() as u32).to_le_bytes());
    out.extend_from_slice(&(metadata.ints.len() as u32).to_le_bytes());
    out.extend_from_slice(&(metadata.strings.len() as u32).to_le_bytes());
    out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    for (name, value) in &metadata.blobs {
        out.extend_from_slice(&(name.len() as u32).to_le_bytes());
        out.extend_from_slice(&(value.len() as u32).to_le_bytes());
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(value);
    }
    for (name, value) in &metadata.ints {
        out.extend_from_slice(&(name.len() as u32).to_le_bytes());
        out.extend_from_slice(&value.to_le_bytes());
        out.extend_from_slice(name.as_bytes());
    }
    for (name, value) in &metadata.strings {
        out.extend_from_slice(&(name.len() as u32).to_le_bytes());
        out.extend_from_slice(&(value.len() as u32).to_le_bytes());
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(value.as_bytes());
    }
    out.extend_from_slice(&payload);
    if out.is_empty() {
        append_message_str("foreignsave", "empty container output");
        return Err(());
    }
    Ok(out)
}
