use crate::abi::image::VipsImage;
use crate::foreign::base::{ForeignMetadata, SaveOptions};
use crate::runtime::header::{
    snapshot_metadata_entries, vips_image_get_blob, vips_image_set_blob_copy,
    vips_image_set_double, vips_image_set_int, vips_image_set_string, MetaValue,
};

pub const PNG_SAFE_XMP_CHUNK: [u8; 4] = *b"vpXm";
pub const PNG_SAFE_ICC_CHUNK: [u8; 4] = *b"vpIc";
pub const PNG_SAFE_EXIF_CHUNK: [u8; 4] = *b"vpEx";

fn copy_blob(image: *mut VipsImage, name: &[u8]) -> Option<Vec<u8>> {
    let mut data = std::ptr::null();
    let mut len = 0usize;
    if vips_image_get_blob(image, name.as_ptr().cast(), &mut data, &mut len) != 0 {
        return None;
    }
    if data.is_null() && len == 0 {
        return Some(Vec::new());
    }
    Some(unsafe { std::slice::from_raw_parts(data.cast::<u8>(), len) }.to_vec())
}

pub fn install_metadata(image: *mut VipsImage, loader_name: &str, metadata: &ForeignMetadata) {
    vips_image_set_string(
        image,
        c"vips-loader".as_ptr(),
        crate::runtime::object::leak_cstring(loader_name),
    );
    for (name, value) in &metadata.blobs {
        if let Ok(name) = std::ffi::CString::new(name.as_str()) {
            vips_image_set_blob_copy(image, name.as_ptr(), value.as_ptr().cast(), value.len());
        }
    }
    for (name, value) in &metadata.ints {
        if let Ok(name) = std::ffi::CString::new(name.as_str()) {
            vips_image_set_int(image, name.as_ptr(), *value);
        }
    }
    for (name, value) in &metadata.doubles {
        if let Ok(name) = std::ffi::CString::new(name.as_str()) {
            vips_image_set_double(image, name.as_ptr(), *value);
        }
    }
    for (name, value) in &metadata.strings {
        if let (Ok(name), Ok(value)) = (
            std::ffi::CString::new(name.as_str()),
            std::ffi::CString::new(value.as_str()),
        ) {
            vips_image_set_string(image, name.as_ptr(), value.as_ptr());
        }
    }
}

pub fn collect_metadata(image: *mut VipsImage, options: &SaveOptions) -> ForeignMetadata {
    let mut metadata = ForeignMetadata::default();
    let keep = options
        .keep
        .as_deref()
        .unwrap_or("all")
        .to_ascii_lowercase();
    let keep_icc = keep == "all" || keep.contains("icc");
    let keep_xmp = keep == "all" || keep.contains("xmp");
    let keep_exif = keep == "all" || keep.contains("exif");

    if keep_icc {
        if let Some(blob) = copy_blob(image, b"icc-profile-data\0") {
            metadata.insert_blob("icc-profile-data", blob);
        }
    }
    if keep_xmp {
        if let Some(blob) = copy_blob(image, b"xmp-data\0") {
            metadata.insert_blob("xmp-data", blob);
        }
    }
    if keep_exif {
        if let Some(blob) = copy_blob(image, b"exif-data\0") {
            metadata.insert_blob("exif-data", blob);
        }
    }
    if let Some(profile) = &options.profile {
        if let Ok(blob) = std::fs::read(profile) {
            metadata.insert_blob("icc-profile-data", blob);
        }
    }

    for (name, value) in snapshot_metadata_entries(image) {
        let Ok(name) = name.into_string() else {
            continue;
        };
        if name == "vips-loader" {
            continue;
        }
        match &value {
            MetaValue::Int(value) => {
                metadata.ints.insert(name, *value);
            }
            MetaValue::Double(value) => {
                metadata.doubles.insert(name, *value);
            }
            MetaValue::String(value) => {
                metadata
                    .strings
                    .insert(name, value.to_string_lossy().into_owned());
            }
            _ => {}
        }
    }

    metadata
}

fn push_jpeg_segment(out: &mut Vec<u8>, marker: u8, payload: &[u8]) {
    if payload.len() + 2 > u16::MAX as usize {
        return;
    }
    out.push(0xff);
    out.push(marker);
    out.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
    out.extend_from_slice(payload);
}

pub fn inject_jpeg_metadata_segments(jpeg: Vec<u8>, metadata: &ForeignMetadata) -> Vec<u8> {
    if jpeg.get(0..2) != Some(&[0xff, 0xd8]) {
        return jpeg;
    }

    let mut segments = Vec::new();
    if let Some(exif) = metadata.blobs.get("exif-data") {
        push_jpeg_segment(&mut segments, 0xe1, exif);
    }
    if let Some(xmp) = metadata.blobs.get("xmp-data") {
        let mut payload = b"http://ns.adobe.com/xap/1.0/\0".to_vec();
        payload.extend_from_slice(xmp);
        push_jpeg_segment(&mut segments, 0xe1, &payload);
    }
    if let Some(icc) = metadata.blobs.get("icc-profile-data") {
        let max_chunk = u16::MAX as usize - 2 - 14;
        let total = icc.len().div_ceil(max_chunk);
        if (1..=255).contains(&total) {
            for (index, chunk) in icc.chunks(max_chunk).enumerate() {
                let mut payload = b"ICC_PROFILE\0".to_vec();
                payload.push((index + 1) as u8);
                payload.push(total as u8);
                payload.extend_from_slice(chunk);
                push_jpeg_segment(&mut segments, 0xe2, &payload);
            }
        }
    }

    if segments.is_empty() {
        return jpeg;
    }

    let mut out = Vec::with_capacity(jpeg.len() + segments.len());
    out.extend_from_slice(&jpeg[..2]);
    out.extend_from_slice(&segments);
    out.extend_from_slice(&jpeg[2..]);
    out
}

pub fn png_metadata_chunks(metadata: &ForeignMetadata) -> Vec<([u8; 4], Vec<u8>)> {
    let mut chunks = Vec::new();
    if let Some(xmp) = metadata.blobs.get("xmp-data") {
        chunks.push((PNG_SAFE_XMP_CHUNK, xmp.clone()));
    }
    if let Some(icc) = metadata.blobs.get("icc-profile-data") {
        chunks.push((PNG_SAFE_ICC_CHUNK, icc.clone()));
    }
    if let Some(exif) = metadata.blobs.get("exif-data") {
        chunks.push((PNG_SAFE_EXIF_CHUNK, exif.clone()));
    }
    chunks
}

pub fn extract_jpeg_metadata(bytes: &[u8]) -> ForeignMetadata {
    let mut metadata = ForeignMetadata::default();
    let mut offset = 2usize;
    let mut icc_parts: Vec<Option<Vec<u8>>> = Vec::new();

    while offset + 4 <= bytes.len() {
        if bytes[offset] != 0xff {
            break;
        }
        let marker = bytes[offset + 1];
        offset += 2;
        if marker == 0xd9 || marker == 0xda {
            break;
        }
        let segment_len = u16::from_be_bytes([bytes[offset], bytes[offset + 1]]) as usize;
        offset += 2;
        if segment_len < 2 || offset + segment_len - 2 > bytes.len() {
            break;
        }
        let segment = &bytes[offset..offset + segment_len - 2];
        if marker == 0xe1 && segment.starts_with(b"Exif\0\0") {
            metadata.insert_blob("exif-data", segment.to_vec());
        } else if marker == 0xe1
            && segment.starts_with(b"http://ns.adobe.com/xap/1.0/\0")
            && segment.len() > 29
        {
            metadata.insert_blob("xmp-data", segment[29..].to_vec());
        } else if marker == 0xe2 && segment.starts_with(b"ICC_PROFILE\0") && segment.len() > 14 {
            let index = segment[12] as usize;
            let total = segment[13] as usize;
            if icc_parts.len() < total {
                icc_parts.resize(total, None);
            }
            if index > 0 && index <= total {
                icc_parts[index - 1] = Some(segment[14..].to_vec());
            }
        }
        offset += segment_len - 2;
    }

    if !icc_parts.is_empty() && icc_parts.iter().all(Option::is_some) {
        let icc = icc_parts
            .into_iter()
            .flatten()
            .flatten()
            .collect::<Vec<_>>();
        metadata.insert_blob("icc-profile-data", icc);
    }

    metadata
}

pub fn extract_png_metadata(bytes: &[u8]) -> ForeignMetadata {
    const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

    let mut metadata = ForeignMetadata::default();
    if bytes.get(0..8) != Some(&PNG_SIGNATURE[..]) {
        return metadata;
    }

    let mut offset = 8usize;
    while offset + 12 <= bytes.len() {
        let length = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        let chunk_name = &bytes[offset + 4..offset + 8];
        let data_start = offset + 8;
        let Some(data_end) = data_start.checked_add(length) else {
            break;
        };
        if data_end + 4 > bytes.len() {
            break;
        }
        let data = &bytes[data_start..data_end];
        match chunk_name {
            name if name == PNG_SAFE_XMP_CHUNK => {
                metadata.insert_blob("xmp-data", data.to_vec());
            }
            name if name == PNG_SAFE_ICC_CHUNK => {
                metadata.insert_blob("icc-profile-data", data.to_vec());
            }
            name if name == PNG_SAFE_EXIF_CHUNK => {
                metadata.insert_blob("exif-data", data.to_vec());
            }
            b"IEND" => break,
            _ => {}
        }
        offset = data_end + 4;
    }

    metadata
}
