use crate::abi::image::{
    VIPS_INTERPRETATION_sRGB, VIPS_FORMAT_DOUBLE, VIPS_FORMAT_FLOAT, VIPS_FORMAT_UCHAR,
    VIPS_FORMAT_USHORT, VIPS_INTERPRETATION_B_W, VIPS_INTERPRETATION_GREY16,
};
use crate::foreign::base::{build_load_result, ForeignMetadata};
use crate::runtime::error::append_message_str;

#[derive(Clone, Copy)]
enum ByteOrder {
    Little,
    Big,
}

fn read_u16(bytes: &[u8], offset: usize, order: ByteOrder) -> Option<u16> {
    let raw: [u8; 2] = bytes.get(offset..offset.checked_add(2)?)?.try_into().ok()?;
    Some(match order {
        ByteOrder::Little => u16::from_le_bytes(raw),
        ByteOrder::Big => u16::from_be_bytes(raw),
    })
}

fn read_u32(bytes: &[u8], offset: usize, order: ByteOrder) -> Option<u32> {
    let raw: [u8; 4] = bytes.get(offset..offset.checked_add(4)?)?.try_into().ok()?;
    Some(match order {
        ByteOrder::Little => u32::from_le_bytes(raw),
        ByteOrder::Big => u32::from_be_bytes(raw),
    })
}

fn read_i32(bytes: &[u8], offset: &mut usize) -> Option<i32> {
    let raw: [u8; 4] = bytes
        .get(*offset..(*offset).checked_add(4)?)?
        .try_into()
        .ok()?;
    *offset += 4;
    Some(i32::from_le_bytes(raw))
}

fn read_u64_le(bytes: &[u8], offset: &mut usize) -> Option<u64> {
    let raw: [u8; 8] = bytes
        .get(*offset..(*offset).checked_add(8)?)?
        .try_into()
        .ok()?;
    *offset += 8;
    Some(u64::from_le_bytes(raw))
}

fn read_u32_le_at(bytes: &[u8], offset: &mut usize) -> Option<u32> {
    let raw: [u8; 4] = bytes
        .get(*offset..(*offset).checked_add(4)?)?
        .try_into()
        .ok()?;
    *offset += 4;
    Some(u32::from_le_bytes(raw))
}

#[derive(Clone, Copy)]
struct IfdEntry {
    tag: u16,
    field_type: u16,
    count: u32,
    value_offset: u32,
}

fn read_entry(bytes: &[u8], offset: usize, order: ByteOrder) -> Option<IfdEntry> {
    Some(IfdEntry {
        tag: read_u16(bytes, offset, order)?,
        field_type: read_u16(bytes, offset + 2, order)?,
        count: read_u32(bytes, offset + 4, order)?,
        value_offset: read_u32(bytes, offset + 8, order)?,
    })
}

fn type_width(field_type: u16) -> Option<usize> {
    match field_type {
        1 | 2 | 7 => Some(1),
        3 => Some(2),
        4 => Some(4),
        _ => None,
    }
}

fn entry_bytes(entry: IfdEntry, bytes: &[u8], order: ByteOrder) -> Option<Vec<u8>> {
    let width = type_width(entry.field_type)?;
    let len = (entry.count as usize).checked_mul(width)?;
    let inline = match order {
        ByteOrder::Little => entry.value_offset.to_le_bytes(),
        ByteOrder::Big => entry.value_offset.to_be_bytes(),
    };
    let source = if len <= 4 {
        &inline[..len]
    } else {
        let offset = entry.value_offset as usize;
        bytes.get(offset..offset.checked_add(len)?)?
    };
    Some(source.to_vec())
}

fn entry_values(entry: IfdEntry, bytes: &[u8], order: ByteOrder) -> Option<Vec<u32>> {
    let width = type_width(entry.field_type)?;
    let len = (entry.count as usize).checked_mul(width)?;
    let inline = match order {
        ByteOrder::Little => entry.value_offset.to_le_bytes(),
        ByteOrder::Big => entry.value_offset.to_be_bytes(),
    };
    let source = if len <= 4 {
        &inline[..len]
    } else {
        let offset = entry.value_offset as usize;
        bytes.get(offset..offset.checked_add(len)?)?
    };
    let mut values = Vec::with_capacity(entry.count as usize);
    for index in 0..entry.count as usize {
        let offset = index * width;
        values.push(match entry.field_type {
            3 => read_u16(source, offset, order)? as u32,
            4 => read_u32(source, offset, order)?,
            _ => return None,
        });
    }
    Some(values)
}

fn scalar(entries: &[IfdEntry], tag: u16, bytes: &[u8], order: ByteOrder) -> Option<u32> {
    let entry = entries.iter().copied().find(|entry| entry.tag == tag)?;
    entry_values(entry, bytes, order)?.first().copied()
}

fn bits_per_sample(entries: &[IfdEntry], bytes: &[u8], order: ByteOrder) -> Option<u16> {
    let entry = entries.iter().copied().find(|entry| entry.tag == 258)?;
    let values = entry_values(entry, bytes, order)?;
    let first = *values.first()? as u16;
    if values.iter().all(|value| *value as u16 == first) {
        Some(first)
    } else {
        None
    }
}

fn sample_format(entries: &[IfdEntry], bytes: &[u8], order: ByteOrder) -> Option<u16> {
    let Some(entry) = entries.iter().copied().find(|entry| entry.tag == 339) else {
        return Some(1);
    };
    let values = entry_values(entry, bytes, order)?;
    let first = *values.first()? as u16;
    if values.iter().all(|value| *value as u16 == first) {
        Some(first)
    } else {
        None
    }
}

pub fn parse_tiff(bytes: &[u8]) -> Result<crate::foreign::base::ForeignLoadResult, ()> {
    let order = match bytes.get(0..2) {
        Some(b"II") => ByteOrder::Little,
        Some(b"MM") => ByteOrder::Big,
        _ => {
            append_message_str("tiffload", "missing tiff byte order");
            return Err(());
        }
    };
    if read_u16(bytes, 2, order) != Some(42) {
        append_message_str("tiffload", "invalid tiff magic");
        return Err(());
    }
    let Some(ifd_offset) = read_u32(bytes, 4, order).map(|value| value as usize) else {
        append_message_str("tiffload", "missing tiff ifd");
        return Err(());
    };
    let Some(entry_count) = read_u16(bytes, ifd_offset, order).map(|value| value as usize) else {
        append_message_str("tiffload", "truncated tiff ifd");
        return Err(());
    };
    let entries_start = ifd_offset + 2;
    let entries_len = entry_count.checked_mul(12).ok_or_else(|| {
        append_message_str("tiffload", "tiff ifd is too large");
    })?;
    if bytes
        .get(entries_start..entries_start.saturating_add(entries_len))
        .is_none()
    {
        append_message_str("tiffload", "truncated tiff entries");
        return Err(());
    }

    let mut entries = Vec::with_capacity(entry_count);
    for index in 0..entry_count {
        let Some(entry) = read_entry(bytes, entries_start + index * 12, order) else {
            append_message_str("tiffload", "truncated tiff entry");
            return Err(());
        };
        entries.push(entry);
    }

    let width = scalar(&entries, 256, bytes, order).ok_or_else(|| {
        append_message_str("tiffload", "missing image width");
    })? as i32;
    let height = scalar(&entries, 257, bytes, order).ok_or_else(|| {
        append_message_str("tiffload", "missing image height");
    })? as i32;
    let compression = scalar(&entries, 259, bytes, order).unwrap_or(1);
    if compression != 1 {
        append_message_str("tiffload", "compressed tiff is not supported");
        return Err(());
    }
    let bands = scalar(&entries, 277, bytes, order).unwrap_or(1) as i32;
    if !matches!(bands, 1 | 2 | 3 | 4) {
        append_message_str("tiffload", "unsupported samples per pixel");
        return Err(());
    }
    let bits = bits_per_sample(&entries, bytes, order).ok_or_else(|| {
        append_message_str("tiffload", "unsupported bits per sample");
    })?;
    let sample_format = sample_format(&entries, bytes, order).ok_or_else(|| {
        append_message_str("tiffload", "unsupported sample format");
    })?;
    let band_format = match (bits, sample_format) {
        (8, 1) => VIPS_FORMAT_UCHAR,
        (16, 1) => VIPS_FORMAT_USHORT,
        (32, 3) => VIPS_FORMAT_FLOAT,
        (64, 3) => VIPS_FORMAT_DOUBLE,
        _ => {
            append_message_str("tiffload", "unsupported sample format");
            return Err(());
        }
    };
    let strip_offset = scalar(&entries, 273, bytes, order).ok_or_else(|| {
        append_message_str("tiffload", "missing strip offset");
    })? as usize;
    let bytes_per_sample = match bits {
        8 => 1,
        16 => 2,
        32 => 4,
        64 => 8,
        _ => {
            append_message_str("tiffload", "unsupported bits per sample");
            return Err(());
        }
    };
    let expected =
        width.max(0) as usize * height.max(0) as usize * bands.max(0) as usize * bytes_per_sample;
    let byte_count = scalar(&entries, 279, bytes, order).unwrap_or(expected as u32) as usize;
    let Some(strip_end) = strip_offset.checked_add(expected) else {
        append_message_str("tiffload", "truncated strip data");
        return Err(());
    };
    if byte_count < expected || bytes.get(strip_offset..strip_end).is_none() {
        append_message_str("tiffload", "truncated strip data");
        return Err(());
    }
    let mut pixels = bytes[strip_offset..strip_end].to_vec();
    if bits == 16 {
        for chunk in pixels.chunks_exact_mut(2) {
            let value = match order {
                ByteOrder::Little => u16::from_le_bytes([chunk[0], chunk[1]]),
                ByteOrder::Big => u16::from_be_bytes([chunk[0], chunk[1]]),
            };
            chunk.copy_from_slice(&value.to_ne_bytes());
        }
    } else if bits == 32 && sample_format == 3 {
        for chunk in pixels.chunks_exact_mut(4) {
            let value = match order {
                ByteOrder::Little => f32::from_le_bytes(chunk.try_into().unwrap()),
                ByteOrder::Big => f32::from_be_bytes(chunk.try_into().unwrap()),
            };
            chunk.copy_from_slice(&value.to_ne_bytes());
        }
    } else if bits == 64 && sample_format == 3 {
        for chunk in pixels.chunks_exact_mut(8) {
            let value = match order {
                ByteOrder::Little => f64::from_le_bytes(chunk.try_into().unwrap()),
                ByteOrder::Big => f64::from_be_bytes(chunk.try_into().unwrap()),
            };
            chunk.copy_from_slice(&value.to_ne_bytes());
        }
    }
    let mut metadata = ForeignMetadata::default().with_string("vips-loader", "tiffload");
    for entry in &entries {
        match entry.tag {
            700 => {
                if let Some(value) = entry_bytes(*entry, bytes, order) {
                    metadata.insert_blob("xmp-data", value);
                }
            }
            34675 => {
                if let Some(value) = entry_bytes(*entry, bytes, order) {
                    metadata.insert_blob("icc-profile-data", value);
                }
            }
            _ => {}
        }
    }

    Ok(build_load_result(
        width,
        height,
        bands,
        band_format,
        if bands <= 2 {
            if bits == 16 {
                VIPS_INTERPRETATION_GREY16
            } else {
                VIPS_INTERPRETATION_B_W
            }
        } else {
            VIPS_INTERPRETATION_sRGB
        },
        "tiffload",
        Some(pixels),
        metadata,
        None,
    ))
}

pub fn parse_webp_compat(bytes: &[u8]) -> Result<crate::foreign::base::ForeignLoadResult, ()> {
    if bytes.len() < 20 || bytes.get(0..4) != Some(b"RIFF") || bytes.get(8..12) != Some(b"WEBP") {
        append_message_str("webpload", "invalid webp header");
        return Err(());
    }
    let mut offset = 12usize;
    while offset + 8 <= bytes.len() {
        let tag = &bytes[offset..offset + 4];
        let size = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        offset += 8;
        let Some(payload) = bytes.get(offset..offset.saturating_add(size)) else {
            append_message_str("webpload", "truncated webp chunk");
            return Err(());
        };
        if tag == b"SVIP" {
            return parse_webp_payload(payload);
        }
        offset = offset.saturating_add(size + (size & 1));
    }
    append_message_str("webpload", "unsupported webp payload");
    Err(())
}

fn parse_webp_payload(payload: &[u8]) -> Result<crate::foreign::base::ForeignLoadResult, ()> {
    if payload.get(0..8) != Some(b"SVIPW1\0\0") {
        append_message_str("webpload", "unsupported webp payload");
        return Err(());
    }
    let mut offset = 8usize;
    let width = read_i32(payload, &mut offset).ok_or_else(|| {
        append_message_str("webpload", "truncated webp payload");
    })?;
    let height = read_i32(payload, &mut offset).ok_or_else(|| {
        append_message_str("webpload", "truncated webp payload");
    })?;
    let bands = read_i32(payload, &mut offset).ok_or_else(|| {
        append_message_str("webpload", "truncated webp payload");
    })?;
    let band_format = read_i32(payload, &mut offset).ok_or_else(|| {
        append_message_str("webpload", "truncated webp payload");
    })?;
    let interpretation = read_i32(payload, &mut offset).ok_or_else(|| {
        append_message_str("webpload", "truncated webp payload");
    })?;
    let pixel_len = read_u64_le(payload, &mut offset).ok_or_else(|| {
        append_message_str("webpload", "truncated webp payload");
    })? as usize;
    let Some(pixels) = payload.get(offset..offset.saturating_add(pixel_len)) else {
        append_message_str("webpload", "truncated webp pixels");
        return Err(());
    };
    offset += pixel_len;
    let mut metadata = ForeignMetadata::default().with_string("vips-loader", "webpload");
    if offset < payload.len() {
        parse_blob_metadata_payload(payload, &mut offset, &mut metadata)?;
    }
    Ok(build_load_result(
        width,
        height,
        bands,
        band_format,
        interpretation,
        "webpload",
        Some(pixels.to_vec()),
        metadata,
        None,
    ))
}

fn parse_blob_metadata_payload(
    payload: &[u8],
    offset: &mut usize,
    metadata: &mut ForeignMetadata,
) -> Result<(), ()> {
    let count = read_u32_le_at(payload, offset).ok_or_else(|| {
        append_message_str("webpload", "truncated webp metadata");
    })?;
    for _ in 0..count {
        let name_len = read_u32_le_at(payload, offset).ok_or_else(|| {
            append_message_str("webpload", "truncated webp metadata");
        })? as usize;
        let data_len = read_u64_le(payload, offset).ok_or_else(|| {
            append_message_str("webpload", "truncated webp metadata");
        })? as usize;
        let name_end = offset.checked_add(name_len).ok_or_else(|| {
            append_message_str("webpload", "webp metadata is too large");
        })?;
        let Some(name_bytes) = payload.get(*offset..name_end) else {
            append_message_str("webpload", "truncated webp metadata");
            return Err(());
        };
        *offset = name_end;
        let data_end = offset.checked_add(data_len).ok_or_else(|| {
            append_message_str("webpload", "webp metadata is too large");
        })?;
        let Some(data) = payload.get(*offset..data_end) else {
            append_message_str("webpload", "truncated webp metadata");
            return Err(());
        };
        *offset = data_end;
        if let Ok(name) = std::str::from_utf8(name_bytes) {
            metadata.insert_blob(name, data.to_vec());
        }
    }
    Ok(())
}
