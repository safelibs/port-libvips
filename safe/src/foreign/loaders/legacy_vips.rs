use std::ffi::{CStr, CString};
use std::os::raw::c_int;

use crate::abi::image::{
    VipsImage, VIPS_CODING_LABQ, VIPS_CODING_NONE, VIPS_CODING_RAD, VIPS_IMAGE_OPENIN,
};
use crate::foreign::base::{build_load_result, ForeignLoadResult, ForeignMetadata};
use crate::runtime::error::append_message_str;
use crate::runtime::header::{snapshot_metadata_entries, vips_image_get_history, MetaValue};
use crate::runtime::image::{
    ensure_pixels, format_sizeof, image_state, set_filename, set_history, set_mode, sync_pixels,
};

const HEADER_SIZE: usize = 64;
const LEGACY_METADATA_MAGIC: &[u8; 8] = b"SVIPSMD1";
const NAMESPACE_URI: &str = "http://www.vips.ecs.soton.ac.uk/";
const VIPS_MAGIC_INTEL: u32 = 0xb6a6f208;
const VIPS_MAGIC_SPARC: u32 = 0x08f2a6b6;

#[derive(Default)]
struct DecodedExtension {
    metadata: ForeignMetadata,
    history: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExtensionSection {
    Header,
    Meta,
}

unsafe extern "C" {
    fn vips__read_header_bytes(im: *mut VipsImage, from: *mut u8) -> c_int;
    fn vips__write_header_bytes(im: *mut VipsImage, to: *mut u8) -> c_int;
}

fn parse_header(bytes: &[u8]) -> Result<VipsImage, ()> {
    if bytes.len() < HEADER_SIZE {
        append_message_str("vipsload", "truncated vips header");
        return Err(());
    }

    let mut header = unsafe { std::mem::zeroed::<VipsImage>() };
    header.sizeof_header = HEADER_SIZE as i64;
    if unsafe { vips__read_header_bytes(&mut header, bytes.as_ptr().cast_mut()) } != 0 {
        return Err(());
    }

    Ok(header)
}

fn pixel_length(image: &VipsImage) -> Result<usize, ()> {
    let width = usize::try_from(image.Xsize).map_err(|_| ())?;
    let height = usize::try_from(image.Ysize).map_err(|_| ())?;
    let bands = usize::try_from(image.Bands).map_err(|_| ())?;
    match image.Coding {
        VIPS_CODING_NONE | VIPS_CODING_LABQ | VIPS_CODING_RAD => width
            .checked_mul(height)
            .and_then(|value| value.checked_mul(bands))
            .and_then(|value| value.checked_mul(format_sizeof(image.BandFmt)))
            .ok_or(()),
        _ => usize::try_from(image.Length).map_err(|_| ()),
    }
}

fn extension_error(message: &str) {
    append_message_str("vipsload", message);
}

fn take_u32(bytes: &[u8], offset: &mut usize) -> Result<u32, ()> {
    if *offset + 4 > bytes.len() {
        return Err(());
    }
    let value = u32::from_le_bytes(bytes[*offset..*offset + 4].try_into().unwrap());
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

fn xml_escape_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(ch),
        }
    }
    out
}

fn xml_escape_attr(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

fn decode_xml_entity(entity: &str) -> Result<char, ()> {
    match entity {
        "amp" => Ok('&'),
        "lt" => Ok('<'),
        "gt" => Ok('>'),
        "quot" => Ok('"'),
        "apos" => Ok('\''),
        _ if entity.starts_with("#x") => u32::from_str_radix(&entity[2..], 16)
            .ok()
            .and_then(char::from_u32)
            .ok_or_else(|| {
                extension_error("invalid vips metadata extension");
            }),
        _ if entity.starts_with('#') => entity[1..]
            .parse::<u32>()
            .ok()
            .and_then(char::from_u32)
            .ok_or_else(|| {
                extension_error("invalid vips metadata extension");
            }),
        _ => {
            extension_error("invalid vips metadata extension");
            Err(())
        }
    }
}

fn xml_unescape(text: &str) -> Result<String, ()> {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch != '&' {
            out.push(ch);
            continue;
        }

        let mut entity = String::new();
        let mut terminated = false;
        for next in chars.by_ref() {
            if next == ';' {
                terminated = true;
                break;
            }
            entity.push(next);
        }

        if !terminated {
            extension_error("invalid vips metadata extension");
            return Err(());
        }

        out.push(decode_xml_entity(&entity)?);
    }

    Ok(out)
}

fn encode_blob_value(blob: *mut crate::abi::r#type::VipsBlob) -> String {
    let area = unsafe { &(*blob).area };
    if area.data.is_null() || area.length == 0 {
        return String::new();
    }

    let encoded = unsafe { glib_sys::g_base64_encode(area.data.cast::<u8>(), area.length) };
    let text = unsafe { CStr::from_ptr(encoded) }
        .to_string_lossy()
        .into_owned();
    unsafe {
        glib_sys::g_free(encoded.cast());
    }
    text
}

fn encode_metadata_value(value: &MetaValue) -> Option<(&'static str, String)> {
    match value {
        MetaValue::Int(value) => Some(("gint", value.to_string())),
        MetaValue::Double(value) => Some(("gdouble", value.to_string())),
        MetaValue::String(value) => Some(("VipsRefString", value.to_string_lossy().into_owned())),
        MetaValue::Blob(blob) => Some(("VipsBlob", encode_blob_value(*blob))),
        _ => None,
    }
}

fn encode_extension_block(image: *mut VipsImage) -> Result<Vec<u8>, ()> {
    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\"?>\n");
    xml.push_str(&format!(
        "<root xmlns=\"{}vips/{}\">\n",
        NAMESPACE_URI,
        env!("CARGO_PKG_VERSION")
    ));
    xml.push_str("  <header>\n");
    let history = vips_image_get_history(image);
    if !history.is_null() {
        let history = unsafe { CStr::from_ptr(history) }.to_string_lossy();
        xml.push_str("    <field type=\"VipsRefString\" name=\"Hist\">");
        xml.push_str(&xml_escape_text(&history));
        xml.push_str("</field>\n");
    }
    xml.push_str("  </header>\n");
    xml.push_str("  <meta>\n");
    for (name, value) in snapshot_metadata_entries(image) {
        if name.as_c_str().to_bytes() == b"vips-loader" {
            continue;
        }
        let Some((type_name, value)) = encode_metadata_value(&value) else {
            continue;
        };
        let field_name = name.to_string_lossy();
        xml.push_str("    <field type=\"");
        xml.push_str(&xml_escape_attr(type_name));
        xml.push_str("\" name=\"");
        xml.push_str(&xml_escape_attr(&field_name));
        xml.push_str("\">");
        xml.push_str(&xml_escape_text(&value));
        xml.push_str("</field>\n");
    }
    xml.push_str("  </meta>\n");
    xml.push_str("</root>\n");
    Ok(xml.into_bytes())
}

fn decode_blob_value(text: &str) -> Result<Vec<u8>, ()> {
    let encoded = CString::new(text.trim()).map_err(|_| {
        extension_error("invalid vips metadata extension");
    })?;
    let mut length = 0usize;
    let data = unsafe { glib_sys::g_base64_decode(encoded.as_ptr(), &mut length) };
    let bytes = if data.is_null() || length == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(data.cast::<u8>(), length) }.to_vec()
    };
    unsafe {
        glib_sys::g_free(data.cast());
    }
    Ok(bytes)
}

fn finalize_xml_field(
    decoded: &mut DecodedExtension,
    section: ExtensionSection,
    name: String,
    type_name: String,
    raw_value: String,
) -> Result<(), ()> {
    let value = xml_unescape(&raw_value)?;
    match section {
        ExtensionSection::Header => {
            if name == "Hist" {
                decoded.history = Some(value);
            }
        }
        ExtensionSection::Meta => match type_name.as_str() {
            "gint" | "gint32" | "guint" | "guint32" | "gboolean" | "int" => {
                let value = value.trim().parse::<i32>().map_err(|_| {
                    extension_error("invalid vips metadata extension");
                })?;
                decoded.metadata.ints.insert(name, value);
            }
            "gdouble" | "gfloat" => {
                let value = value.trim().parse::<f64>().map_err(|_| {
                    extension_error("invalid vips metadata extension");
                })?;
                decoded.metadata.doubles.insert(name, value);
            }
            "VipsRefString" | "gchararray" | "string" => {
                decoded.metadata.strings.insert(name, value);
            }
            "VipsBlob" | "blob" => {
                decoded.metadata.blobs.insert(name, decode_blob_value(&value)?);
            }
            _ => {}
        },
    }
    Ok(())
}

fn parse_attributes(text: &str) -> Result<Vec<(String, String)>, ()> {
    let bytes = text.as_bytes();
    let mut attrs = Vec::new();
    let mut pos = 0usize;

    while pos < bytes.len() {
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= bytes.len() {
            break;
        }

        let name_start = pos;
        while pos < bytes.len() && !bytes[pos].is_ascii_whitespace() && bytes[pos] != b'=' {
            pos += 1;
        }
        let name = &text[name_start..pos];
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= bytes.len() || bytes[pos] != b'=' {
            extension_error("invalid vips metadata extension");
            return Err(());
        }
        pos += 1;
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= bytes.len() || !matches!(bytes[pos], b'"' | b'\'') {
            extension_error("invalid vips metadata extension");
            return Err(());
        }

        let quote = bytes[pos];
        pos += 1;
        let value_start = pos;
        while pos < bytes.len() && bytes[pos] != quote {
            pos += 1;
        }
        if pos >= bytes.len() {
            extension_error("invalid vips metadata extension");
            return Err(());
        }
        let value = xml_unescape(&text[value_start..pos])?;
        pos += 1;

        attrs.push((name.to_owned(), value));
    }

    Ok(attrs)
}

fn find_tag_end(xml: &str, start: usize) -> Result<usize, ()> {
    let bytes = xml.as_bytes();
    let mut pos = start;
    let mut quote = None;

    while pos < bytes.len() {
        match bytes[pos] {
            b'"' | b'\'' if quote.is_none() => quote = Some(bytes[pos]),
            b'"' | b'\'' if quote == Some(bytes[pos]) => quote = None,
            b'>' if quote.is_none() => return Ok(pos),
            _ => {}
        }
        pos += 1;
    }

    extension_error("invalid vips metadata extension");
    Err(())
}

fn parse_xml_extension(bytes: &[u8]) -> Result<DecodedExtension, ()> {
    let xml = std::str::from_utf8(bytes).map_err(|_| {
        extension_error("invalid vips metadata extension");
    })?;
    let mut decoded = DecodedExtension::default();
    let mut section = None;
    let mut current_field: Option<(ExtensionSection, String, String, String)> = None;
    let mut pos = 0usize;

    while let Some(relative) = xml[pos..].find('<') {
        let tag_start = pos + relative;
        if let Some((_, _, _, content)) = current_field.as_mut() {
            content.push_str(&xml[pos..tag_start]);
        }

        if xml[tag_start..].starts_with("<!--") {
            let comment_end = xml[tag_start + 4..]
                .find("-->")
                .map(|offset| tag_start + 4 + offset + 3)
                .ok_or_else(|| {
                    extension_error("invalid vips metadata extension");
                })?;
            pos = comment_end;
            continue;
        }

        let tag_end = find_tag_end(xml, tag_start + 1)?;
        let mut tag = xml[tag_start + 1..tag_end].trim();
        pos = tag_end + 1;

        if tag.is_empty() || tag.starts_with('?') || tag.starts_with('!') {
            continue;
        }

        let is_end = tag.starts_with('/');
        if is_end {
            tag = tag[1..].trim_start();
        }

        let self_closing = !is_end && tag.ends_with('/');
        if self_closing {
            tag = tag[..tag.len() - 1].trim_end();
        }

        let (name, attrs_text) = if let Some(index) = tag.find(char::is_whitespace) {
            (&tag[..index], tag[index + 1..].trim_start())
        } else {
            (tag, "")
        };

        match (is_end, name) {
            (false, "header") => section = Some(ExtensionSection::Header),
            (true, "header") => section = None,
            (false, "meta") => section = Some(ExtensionSection::Meta),
            (true, "meta") => section = None,
            (false, "field") => {
                let attrs = parse_attributes(attrs_text)?;
                let field_name = attrs
                    .iter()
                    .find_map(|(attr, value)| (attr == "name").then_some(value.clone()))
                    .unwrap_or_default();
                let type_name = attrs
                    .iter()
                    .find_map(|(attr, value)| (attr == "type").then_some(value.clone()))
                    .unwrap_or_default();
                let field_section = section.unwrap_or(ExtensionSection::Meta);
                current_field = Some((field_section, field_name, type_name, String::new()));
                if self_closing {
                    let field = current_field.take().expect("field");
                    finalize_xml_field(&mut decoded, field.0, field.1, field.2, field.3)?;
                }
            }
            (true, "field") => {
                let Some(field) = current_field.take() else {
                    extension_error("invalid vips metadata extension");
                    return Err(());
                };
                finalize_xml_field(&mut decoded, field.0, field.1, field.2, field.3)?;
            }
            _ => {}
        }
    }

    if let Some((_, _, _, content)) = current_field.as_mut() {
        content.push_str(&xml[pos..]);
    }
    if let Some(field) = current_field.take() {
        extension_error("invalid vips metadata extension");
        let _ = field;
        return Err(());
    }

    Ok(decoded)
}

fn decode_legacy_binary_extension(bytes: &[u8]) -> Result<DecodedExtension, ()> {
    let mut decoded = DecodedExtension::default();
    let mut offset = LEGACY_METADATA_MAGIC.len();
    let count = take_u32(bytes, &mut offset)? as usize;
    for _ in 0..count {
        let name_len = take_u32(bytes, &mut offset)? as usize;
        let type_len = take_u32(bytes, &mut offset)? as usize;
        let value_len = take_u64(bytes, &mut offset)? as usize;
        if offset + name_len + type_len + value_len > bytes.len() {
            extension_error("truncated vips metadata extension");
            return Err(());
        }

        let name = String::from_utf8_lossy(&bytes[offset..offset + name_len]).into_owned();
        offset += name_len;
        let type_name = String::from_utf8_lossy(&bytes[offset..offset + type_len]).into_owned();
        offset += type_len;
        let value = String::from_utf8_lossy(&bytes[offset..offset + value_len]).into_owned();
        offset += value_len;

        match type_name.as_str() {
            "int" => {
                let value = value.parse::<i32>().map_err(|_| {
                    extension_error("invalid vips metadata extension");
                })?;
                decoded.metadata.ints.insert(name, value);
            }
            "double" => {
                let value = value.parse::<f64>().map_err(|_| {
                    extension_error("invalid vips metadata extension");
                })?;
                decoded.metadata.doubles.insert(name, value);
            }
            "string" => {
                decoded.metadata.strings.insert(name, value);
            }
            "blob" => {
                decoded.metadata.blobs.insert(name, decode_blob_value(&value)?);
            }
            _ => {}
        }
    }

    Ok(decoded)
}

fn decode_extension(bytes: &[u8]) -> Result<DecodedExtension, ()> {
    if bytes.is_empty() || bytes.iter().all(u8::is_ascii_whitespace) {
        return Ok(DecodedExtension::default());
    }
    if bytes.starts_with(LEGACY_METADATA_MAGIC) {
        return decode_legacy_binary_extension(bytes);
    }
    parse_xml_extension(bytes)
}

fn decode_extension_lossy(bytes: &[u8]) -> DecodedExtension {
    decode_extension(bytes).unwrap_or_default()
}

fn open_image_fd(filename: &CStr) -> Result<c_int, ()> {
    let fd = crate::runtime::memory::vips_tracked_open(filename.as_ptr(), libc::O_RDWR, 0);
    if fd >= 0 {
        return Ok(fd);
    }

    let fd = crate::runtime::memory::vips_tracked_open(filename.as_ptr(), libc::O_RDONLY, 0);
    if fd >= 0 {
        return Ok(fd);
    }

    append_message_str(
        "vipsload",
        &format!("unable to open {}", filename.to_string_lossy()),
    );
    Err(())
}

pub fn extract_pixel_payload(bytes: &[u8]) -> Result<Vec<u8>, ()> {
    let header = parse_header(bytes)?;
    let payload_len = pixel_length(&header)?;
    if bytes.len() < HEADER_SIZE + payload_len {
        append_message_str("vipsload", "truncated vips pixel payload");
        return Err(());
    }

    Ok(bytes[HEADER_SIZE..HEADER_SIZE + payload_len].to_vec())
}

pub fn parse_bytes(bytes: &[u8]) -> Result<ForeignLoadResult, ()> {
    let header = parse_header(bytes)?;
    let payload_len = pixel_length(&header)?;
    if bytes.len() < HEADER_SIZE + payload_len {
        append_message_str("vipsload", "truncated vips pixel payload");
        return Err(());
    }
    let decoded = decode_extension_lossy(&bytes[HEADER_SIZE + payload_len..]);
    let mut result = build_load_result(
        header.Xsize,
        header.Ysize,
        header.Bands,
        header.BandFmt,
        header.Type,
        "vipsload",
        Some(bytes[HEADER_SIZE..HEADER_SIZE + payload_len].to_vec()),
        decoded.metadata,
        None,
    );
    result.coding = header.Coding;
    result.history = decoded.history;
    Ok(result)
}

pub fn load_file_into_image(filename: &CStr, image: *mut VipsImage) -> *mut VipsImage {
    let Ok(bytes) = std::fs::read(filename.to_string_lossy().as_ref()) else {
        append_message_str(
            "vipsload",
            &format!("unable to read {}", filename.to_string_lossy()),
        );
        return std::ptr::null_mut();
    };
    if bytes.len() < HEADER_SIZE {
        append_message_str("vipsload", "truncated vips header");
        return std::ptr::null_mut();
    }

    set_filename(image, Some(filename));
    set_mode(image, "r");

    let Some(image_ref) = (unsafe { image.as_mut() }) else {
        return std::ptr::null_mut();
    };
    image_ref.sizeof_header = HEADER_SIZE as i64;
    if unsafe { vips__read_header_bytes(image, bytes.as_ptr().cast_mut()) } != 0 {
        return std::ptr::null_mut();
    }

    let Ok(payload_len) = pixel_length(image_ref) else {
        append_message_str("vipsload", "invalid vips image dimensions");
        return std::ptr::null_mut();
    };
    if bytes.len() < HEADER_SIZE + payload_len {
        append_message_str("vipsload", "truncated vips pixel payload");
        return std::ptr::null_mut();
    }
    let decoded = decode_extension_lossy(&bytes[HEADER_SIZE + payload_len..]);

    let Ok(fd) = open_image_fd(filename) else {
        return std::ptr::null_mut();
    };

    if let Some(state) = unsafe { image_state(image) } {
        if let Some(old_fd) = state.fd.replace(fd) {
            crate::runtime::memory::vips_tracked_close(old_fd);
        }
        state.pixels = bytes[HEADER_SIZE..HEADER_SIZE + payload_len].to_vec();
        state.pending_load = None;
    } else {
        crate::runtime::memory::vips_tracked_close(fd);
        return std::ptr::null_mut();
    }

    image_ref.fd = fd;
    image_ref.dtype = VIPS_IMAGE_OPENIN;
    image_ref.file_length = bytes.len() as i64;
    image_ref.sizeof_header = HEADER_SIZE as i64;
    sync_pixels(image);
    set_history(image, decoded.history.as_deref());
    crate::foreign::metadata::install_metadata(image, "vipsload", &decoded.metadata);

    image
}

pub fn save_bytes(image: *mut VipsImage) -> Result<Vec<u8>, ()> {
    ensure_pixels(image)?;
    let Some(image_ref) = (unsafe { image.as_mut() }) else {
        return Err(());
    };
    let Some(state) = (unsafe { image_state(image) }) else {
        return Err(());
    };

    let payload_len = pixel_length(image_ref)?;
    if state.pixels.len() < payload_len {
        append_message_str("vipssave", "in-memory image payload is truncated");
        return Err(());
    }

    image_ref.magic = match image_ref.magic {
        VIPS_MAGIC_SPARC => VIPS_MAGIC_SPARC,
        VIPS_MAGIC_INTEL => VIPS_MAGIC_INTEL,
        _ => VIPS_MAGIC_INTEL,
    };
    image_ref.sizeof_header = HEADER_SIZE as i64;
    image_ref.Bbits = (format_sizeof(image_ref.BandFmt) * 8) as i32;
    image_ref.Length = payload_len as i32;

    let mut header = [0u8; HEADER_SIZE];
    if unsafe { vips__write_header_bytes(image, header.as_mut_ptr()) } != 0 {
        return Err(());
    }

    let mut bytes = Vec::with_capacity(HEADER_SIZE + payload_len);
    bytes.extend_from_slice(&header);
    bytes.extend_from_slice(&state.pixels[..payload_len]);
    let extension = encode_extension_block(image)?;
    bytes.extend_from_slice(&extension);
    Ok(bytes)
}

pub fn write_file(image: *mut VipsImage, filename: &str) -> Result<(), ()> {
    let bytes = save_bytes(image)?;
    std::fs::write(filename, bytes).map_err(|err| {
        append_message_str("vipssave", &err.to_string());
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_xml_extension_parses_common_types() {
        let xml = concat!(
            "<?xml version=\"1.0\"?>\n",
            "<root xmlns=\"http://www.vips.ecs.soton.ac.uk/vips/8.15.1\">\n",
            "  <header>\n",
            "    <field type=\"VipsRefString\" name=\"Hist\">history line 1\nhistory line 2</field>\n",
            "  </header>\n",
            "  <meta>\n",
            "    <field type=\"VipsBlob\" name=\"exif-data\">QUJD</field>\n",
            "    <field type=\"VipsRefString\" name=\"comment\">edited-by-setext</field>\n",
            "    <field type=\"gint\" name=\"page-height\">7</field>\n",
            "    <field type=\"gdouble\" name=\"scale\">1.5</field>\n",
            "  </meta>\n",
            "</root>\n"
        );

        let decoded = parse_xml_extension(xml.as_bytes()).expect("decoded extension");
        assert_eq!(decoded.history.as_deref(), Some("history line 1\nhistory line 2"));
        assert_eq!(
            decoded.metadata.strings.get("comment").map(String::as_str),
            Some("edited-by-setext")
        );
        assert_eq!(decoded.metadata.ints.get("page-height").copied(), Some(7));
        assert_eq!(decoded.metadata.doubles.get("scale").copied(), Some(1.5));
        assert_eq!(
            decoded.metadata.blobs.get("exif-data").map(Vec::as_slice),
            Some(&b"ABC"[..])
        );
    }
}
