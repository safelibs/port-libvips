use crate::abi::image::VIPS_FORMAT_LAST;
use crate::foreign::base::{
    build_load_result, ForeignKind, ForeignMetadata, LoadOptions, CONTAINER_MAGIC,
};
use crate::runtime::error::append_message_str;

const FLAG_PACKED_GRAY: u32 = 1 << 0;
const HEADER_LEN: usize = 8 + 1 + 1 + 1 + 4 + 4 + 4 + 4 + 4 + 4 + 4 + 4 + 8;

struct ContainerParts<'a> {
    kind: ForeignKind,
    bands: i32,
    format_tag: i32,
    width: i32,
    height: i32,
    interpretation: i32,
    flags: u32,
    metadata: ForeignMetadata,
    payload: &'a [u8],
}

fn take_u32(bytes: &[u8], offset: &mut usize) -> Result<u32, ()> {
    if *offset + 4 > bytes.len() {
        return Err(());
    }
    let value = u32::from_le_bytes(bytes[*offset..*offset + 4].try_into().unwrap());
    *offset += 4;
    Ok(value)
}

fn take_i32(bytes: &[u8], offset: &mut usize) -> Result<i32, ()> {
    if *offset + 4 > bytes.len() {
        return Err(());
    }
    let value = i32::from_le_bytes(bytes[*offset..*offset + 4].try_into().unwrap());
    *offset += 4;
    Ok(value)
}

fn take_u64(bytes: &[u8], offset: &mut usize) -> Result<u64, ()> {
    if *offset + 8 > bytes.len() {
        return Err(());
    }
    let value = u64::from_le_bytes(bytes[*offset..*offset + 8].try_into().unwrap());
    *offset += 8;
    Ok(value)
}

fn kind_from_tag(tag: u8) -> ForeignKind {
    match tag {
        1 => ForeignKind::Jpeg,
        2 => ForeignKind::Png,
        3 => ForeignKind::Gif,
        4 => ForeignKind::Tiff,
        5 => ForeignKind::Webp,
        6 => ForeignKind::Svg,
        7 => ForeignKind::Pdf,
        8 => ForeignKind::Ppm,
        9 => ForeignKind::Pfm,
        10 => ForeignKind::Csv,
        11 => ForeignKind::Matrix,
        12 => ForeignKind::Raw,
        13 => ForeignKind::Vips,
        14 => ForeignKind::Radiance,
        15 => ForeignKind::Heif,
        _ => ForeignKind::Vips,
    }
}

fn parse_container_parts(bytes: &[u8]) -> Result<ContainerParts<'_>, ()> {
    if !is_container(bytes) || bytes.len() < HEADER_LEN {
        append_message_str("vipsload", "invalid vips container");
        return Err(());
    }

    let mut offset = CONTAINER_MAGIC.len();
    let kind = kind_from_tag(bytes[offset]);
    offset += 1;
    let bands = bytes[offset] as i32;
    offset += 1;
    let format_tag = bytes[offset] as i32;
    offset += 1;
    let width = take_i32(bytes, &mut offset)?;
    let height = take_i32(bytes, &mut offset)?;
    let _coding = take_i32(bytes, &mut offset)?;
    let interpretation = take_i32(bytes, &mut offset)?;
    let flags = take_u32(bytes, &mut offset)?;
    let blob_count = take_u32(bytes, &mut offset)? as usize;
    let int_count = take_u32(bytes, &mut offset)? as usize;
    let string_count = take_u32(bytes, &mut offset)? as usize;
    let pixel_len = take_u64(bytes, &mut offset)? as usize;

    let mut metadata = ForeignMetadata::default();

    for _ in 0..blob_count {
        let name_len = take_u32(bytes, &mut offset)? as usize;
        let value_len = take_u32(bytes, &mut offset)? as usize;
        if offset + name_len + value_len > bytes.len() {
            append_message_str("vipsload", "truncated metadata");
            return Err(());
        }
        let name = String::from_utf8_lossy(&bytes[offset..offset + name_len]).into_owned();
        offset += name_len;
        metadata.insert_blob(&name, bytes[offset..offset + value_len].to_vec());
        offset += value_len;
    }

    for _ in 0..int_count {
        let name_len = take_u32(bytes, &mut offset)? as usize;
        let value = take_i32(bytes, &mut offset)?;
        if offset + name_len > bytes.len() {
            append_message_str("vipsload", "truncated metadata");
            return Err(());
        }
        let name = String::from_utf8_lossy(&bytes[offset..offset + name_len]).into_owned();
        offset += name_len;
        metadata.ints.insert(name, value);
    }

    for _ in 0..string_count {
        let name_len = take_u32(bytes, &mut offset)? as usize;
        let value_len = take_u32(bytes, &mut offset)? as usize;
        if offset + name_len + value_len > bytes.len() {
            append_message_str("vipsload", "truncated metadata");
            return Err(());
        }
        let name = String::from_utf8_lossy(&bytes[offset..offset + name_len]).into_owned();
        offset += name_len;
        let value = String::from_utf8_lossy(&bytes[offset..offset + value_len]).into_owned();
        offset += value_len;
        metadata.strings.insert(name, value);
    }

    if offset + pixel_len > bytes.len() {
        append_message_str("vipsload", "truncated pixel payload");
        return Err(());
    }

    Ok(ContainerParts {
        kind,
        bands,
        format_tag,
        width,
        height,
        interpretation,
        flags,
        metadata,
        payload: &bytes[offset..offset + pixel_len],
    })
}

fn unpack_packed_gray(
    payload: &[u8],
    width: i32,
    height: i32,
    bits_per_sample: i32,
) -> Result<Vec<u8>, ()> {
    let expected = width.max(0) as usize * height.max(0) as usize;
    let max_value = match bits_per_sample {
        1 => 1usize,
        2 => 3usize,
        4 => 15usize,
        _ => return Err(()),
    };
    let mut out = Vec::with_capacity(expected);
    let mask = max_value as u8;
    let samples_per_byte = 8 / bits_per_sample as usize;
    for &byte in payload {
        for sample_index in 0..samples_per_byte {
            if out.len() == expected {
                break;
            }
            let shift = 8 - bits_per_sample as usize * (sample_index + 1);
            let value = (byte >> shift) & mask;
            out.push(((value as usize * 255) / max_value) as u8);
        }
    }
    if out.len() != expected {
        append_message_str("vipsload", "truncated packed pixel payload");
        return Err(());
    }
    Ok(out)
}

pub fn is_container(bytes: &[u8]) -> bool {
    bytes.len() >= CONTAINER_MAGIC.len() && &bytes[..CONTAINER_MAGIC.len()] == CONTAINER_MAGIC
}

pub fn extract_pixel_payload(bytes: &[u8]) -> Result<Vec<u8>, ()> {
    let parts = parse_container_parts(bytes)?;
    if parts.flags & FLAG_PACKED_GRAY != 0 {
        let bits_per_sample = parts
            .metadata
            .ints
            .get("bits-per-sample")
            .copied()
            .unwrap_or(8);
        unpack_packed_gray(parts.payload, parts.width, parts.height, bits_per_sample)
    } else {
        Ok(parts.payload.to_vec())
    }
}

pub fn parse_container(
    bytes: &[u8],
    _options: LoadOptions,
) -> Result<crate::foreign::base::ForeignLoadResult, ()> {
    let parts = parse_container_parts(bytes)?;
    let pixels = if parts.flags & FLAG_PACKED_GRAY != 0 {
        let bits_per_sample = parts
            .metadata
            .ints
            .get("bits-per-sample")
            .copied()
            .unwrap_or(8);
        unpack_packed_gray(parts.payload, parts.width, parts.height, bits_per_sample)?
    } else {
        parts.payload.to_vec()
    };

    Ok(build_load_result(
        parts.width,
        parts.height,
        parts.bands,
        if (0..VIPS_FORMAT_LAST).contains(&parts.format_tag) {
            parts.format_tag
        } else {
            crate::foreign::base::band_format_for_bits(
                parts
                    .metadata
                    .ints
                    .get("bits-per-sample")
                    .copied()
                    .unwrap_or(8) as u8,
            )
        },
        if parts.interpretation == 0 {
            crate::foreign::base::interpretation_for_png(
                parts
                    .metadata
                    .ints
                    .get("bits-per-sample")
                    .copied()
                    .unwrap_or(8) as u8,
                parts.bands,
            )
        } else {
            parts.interpretation
        },
        match parts.kind {
            ForeignKind::Jpeg => "jpegload",
            ForeignKind::Png => "pngload",
            ForeignKind::Gif => "gifload",
            ForeignKind::Tiff => "tiffload",
            ForeignKind::Webp => "webpload",
            ForeignKind::Heif => "heifload",
            ForeignKind::Radiance => "radload",
            _ => "vipsload",
        },
        Some(pixels),
        parts.metadata,
        None,
    ))
}
