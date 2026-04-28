use std::io::Cursor;

use jpeg_decoder::{Decoder, PixelFormat};

fn pixel_format_size(format: PixelFormat) -> usize {
    match format {
        PixelFormat::L8 => 1,
        PixelFormat::L16 => 2,
        PixelFormat::RGB24 => 3,
        PixelFormat::CMYK32 => 4,
    }
}

pub fn blank_pixels_from_header(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut decoder = Decoder::new(Cursor::new(bytes));
    decoder.read_info().map_err(|err| err.to_string())?;
    let info = decoder
        .info()
        .ok_or_else(|| "missing jpeg frame info".to_owned())?;
    let len = usize::from(info.width)
        .saturating_mul(usize::from(info.height))
        .saturating_mul(pixel_format_size(info.pixel_format));
    Ok(vec![0; len])
}

pub fn decode_pixels(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut decoder = Decoder::new(Cursor::new(bytes));
    let pixels = decoder.decode().map_err(|err| err.to_string())?;

    let Some(info) = decoder.info() else {
        return Err("missing jpeg frame info".to_owned());
    };

    let expected = usize::from(info.width)
        .saturating_mul(usize::from(info.height))
        .saturating_mul(pixel_format_size(info.pixel_format));
    if pixels.len() != expected {
        return Err("decoded jpeg pixel payload size mismatch".to_owned());
    }

    Ok(pixels)
}
