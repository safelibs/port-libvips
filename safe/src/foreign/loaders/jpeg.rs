use std::io::Cursor;

use jpeg_decoder::{Decoder, PixelFormat};

use crate::abi::image::{
    VIPS_INTERPRETATION_sRGB, VipsBandFormat, VipsInterpretation, VIPS_FORMAT_UCHAR,
    VIPS_FORMAT_USHORT, VIPS_INTERPRETATION_B_W, VIPS_INTERPRETATION_CMYK,
};

#[derive(Clone, Copy, Debug)]
pub struct JpegInfo {
    pub width: i32,
    pub height: i32,
    pub bands: i32,
    pub band_format: VipsBandFormat,
    pub interpretation: VipsInterpretation,
}

fn pixel_format_size(format: PixelFormat) -> usize {
    match format {
        PixelFormat::L8 => 1,
        PixelFormat::L16 => 2,
        PixelFormat::RGB24 => 3,
        PixelFormat::CMYK32 => 4,
    }
}

fn image_info(width: u16, height: u16, format: PixelFormat) -> JpegInfo {
    match format {
        PixelFormat::L8 => JpegInfo {
            width: i32::from(width),
            height: i32::from(height),
            bands: 1,
            band_format: VIPS_FORMAT_UCHAR,
            interpretation: VIPS_INTERPRETATION_B_W,
        },
        PixelFormat::L16 => JpegInfo {
            width: i32::from(width),
            height: i32::from(height),
            bands: 1,
            band_format: VIPS_FORMAT_USHORT,
            interpretation: VIPS_INTERPRETATION_B_W,
        },
        PixelFormat::RGB24 => JpegInfo {
            width: i32::from(width),
            height: i32::from(height),
            bands: 3,
            band_format: VIPS_FORMAT_UCHAR,
            interpretation: VIPS_INTERPRETATION_sRGB,
        },
        PixelFormat::CMYK32 => JpegInfo {
            width: i32::from(width),
            height: i32::from(height),
            bands: 4,
            band_format: VIPS_FORMAT_UCHAR,
            interpretation: VIPS_INTERPRETATION_CMYK,
        },
    }
}

pub fn read_info(bytes: &[u8]) -> Result<JpegInfo, String> {
    let mut decoder = Decoder::new(Cursor::new(bytes));
    decoder.read_info().map_err(|err| err.to_string())?;
    let info = decoder
        .info()
        .ok_or_else(|| "missing jpeg frame info".to_owned())?;
    Ok(image_info(info.width, info.height, info.pixel_format))
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
