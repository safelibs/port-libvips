use std::fmt::Write;

use crate::abi::image::{VipsImage, VIPS_FORMAT_FLOAT, VIPS_FORMAT_UCHAR};
use crate::runtime::error::append_message_str;
use crate::runtime::image::{ensure_pixels, image_state};

pub fn save_raw(image: *mut VipsImage) -> Result<Vec<u8>, ()> {
    if ensure_pixels(image).is_err() {
        return Err(());
    }
    let Some(state) = (unsafe { image_state(image) }) else {
        return Err(());
    };
    Ok(state.pixels.clone())
}

pub fn save_csv(image: *mut VipsImage) -> Result<Vec<u8>, ()> {
    if ensure_pixels(image).is_err() {
        return Err(());
    }
    let Some(image_ref) = (unsafe { image.as_ref() }) else {
        return Err(());
    };
    let Some(state) = (unsafe { image_state(image) }) else {
        return Err(());
    };
    if image_ref.Bands != 1 || image_ref.BandFmt != VIPS_FORMAT_UCHAR {
        append_message_str("csvsave", "only mono uchar csv images are supported");
        return Err(());
    }
    let mut out = String::new();
    for row in 0..image_ref.Ysize.max(0) as usize {
        let start = row * image_ref.Xsize.max(0) as usize;
        let end = start + image_ref.Xsize.max(0) as usize;
        for (index, value) in state.pixels[start..end].iter().enumerate() {
            if index != 0 {
                out.push(',');
            }
            let _ = write!(&mut out, "{value}");
        }
        out.push('\n');
    }
    Ok(out.into_bytes())
}

pub fn save_matrix(image: *mut VipsImage) -> Result<Vec<u8>, ()> {
    if ensure_pixels(image).is_err() {
        return Err(());
    }
    let Some(image_ref) = (unsafe { image.as_ref() }) else {
        return Err(());
    };
    let Some(state) = (unsafe { image_state(image) }) else {
        return Err(());
    };
    if image_ref.Bands != 1 {
        append_message_str("matrixsave", "only single-band matrix images are supported");
        return Err(());
    }

    let mut out = String::new();
    let _ = writeln!(
        &mut out,
        "{} {} 1 0",
        image_ref.Xsize.max(0),
        image_ref.Ysize.max(0)
    );
    match image_ref.BandFmt {
        VIPS_FORMAT_UCHAR => {
            for row in 0..image_ref.Ysize.max(0) as usize {
                let start = row * image_ref.Xsize.max(0) as usize;
                let end = start + image_ref.Xsize.max(0) as usize;
                for value in &state.pixels[start..end] {
                    let _ = write!(&mut out, "{} ", *value as f64);
                }
                out.push('\n');
            }
        }
        VIPS_FORMAT_FLOAT => {
            for row in 0..image_ref.Ysize.max(0) as usize {
                let start = row * image_ref.Xsize.max(0) as usize * 4;
                let end = start + image_ref.Xsize.max(0) as usize * 4;
                for chunk in state.pixels[start..end].chunks_exact(4) {
                    let value = f32::from_ne_bytes(chunk.try_into().unwrap()) as f64;
                    let _ = write!(&mut out, "{value} ");
                }
                out.push('\n');
            }
        }
        _ => {
            append_message_str("matrixsave", "unsupported matrix format");
            return Err(());
        }
    }
    Ok(out.into_bytes())
}

pub fn save_ppm(image: *mut VipsImage, pfm: bool) -> Result<Vec<u8>, ()> {
    if ensure_pixels(image).is_err() {
        return Err(());
    }
    let Some(image_ref) = (unsafe { image.as_ref() }) else {
        return Err(());
    };
    let Some(state) = (unsafe { image_state(image) }) else {
        return Err(());
    };
    if pfm {
        if image_ref.Bands != 1 || image_ref.BandFmt != VIPS_FORMAT_FLOAT {
            append_message_str("ppmsave", "pfm save requires single-band float input");
            return Err(());
        }
        let mut out = Vec::new();
        out.extend_from_slice(
            format!("Pf\n{} {}\n-1.0\n", image_ref.Xsize, image_ref.Ysize).as_bytes(),
        );
        out.extend_from_slice(&state.pixels);
        return Ok(out);
    }

    let magic = match image_ref.Bands {
        1 => "P5",
        3 => "P6",
        _ => {
            append_message_str("ppmsave", "only mono and rgb ppm images are supported");
            return Err(());
        }
    };
    let mut out = Vec::new();
    out.extend_from_slice(
        format!("{magic}\n{} {}\n255\n", image_ref.Xsize, image_ref.Ysize).as_bytes(),
    );
    out.extend_from_slice(&state.pixels);
    Ok(out)
}
