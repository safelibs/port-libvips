use std::io::Cursor;

use jpeg_decoder::{Decoder, PixelFormat};

use crate::runtime::error::append_message_str;

pub fn decode_pixels(bytes: &[u8]) -> Result<Vec<u8>, ()> {
    let mut decoder = Decoder::new(Cursor::new(bytes));
    let pixels = decoder.decode().map_err(|err| {
        append_message_str("jpegload", &err.to_string());
    })?;

    let Some(info) = decoder.info() else {
        append_message_str("jpegload", "missing jpeg frame info");
        return Err(());
    };

    let channels = match info.pixel_format {
        PixelFormat::L8 => 1,
        PixelFormat::L16 => 2,
        PixelFormat::RGB24 => 3,
        PixelFormat::CMYK32 => 4,
    };
    let expected = usize::from(info.width)
        .saturating_mul(usize::from(info.height))
        .saturating_mul(channels);
    if pixels.len() != expected {
        append_message_str("jpegload", "decoded jpeg pixel payload size mismatch");
        return Err(());
    }

    Ok(pixels)
}
