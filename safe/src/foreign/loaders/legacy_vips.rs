use std::ffi::CStr;
use std::os::raw::c_int;

use crate::abi::image::{
    VipsImage, VIPS_CODING_LABQ, VIPS_CODING_NONE, VIPS_CODING_RAD, VIPS_IMAGE_OPENIN,
};
use crate::foreign::base::{build_load_result, ForeignLoadResult, ForeignMetadata};
use crate::runtime::error::append_message_str;
use crate::runtime::image::{ensure_pixels, format_sizeof, image_state, set_filename, set_mode, sync_pixels};

const HEADER_SIZE: usize = 64;
const VIPS_MAGIC_INTEL: u32 = 0xb6a6f208;
const VIPS_MAGIC_SPARC: u32 = 0x08f2a6b6;

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
    let payload = extract_pixel_payload(bytes)?;
    let mut result = build_load_result(
        header.Xsize,
        header.Ysize,
        header.Bands,
        header.BandFmt,
        header.Type,
        "vipsload",
        Some(payload),
        ForeignMetadata::default(),
        None,
    );
    result.coding = header.Coding;
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
    Ok(bytes)
}

pub fn write_file(image: *mut VipsImage, filename: &str) -> Result<(), ()> {
    let bytes = save_bytes(image)?;
    std::fs::write(filename, bytes).map_err(|err| {
        append_message_str("vipssave", &err.to_string());
    })
}
