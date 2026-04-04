use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::foreign::base::{build_load_result, ForeignKind, ForeignMetadata, LoadOptions};
use crate::runtime::error::append_message_str;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(1);

fn temp_path(suffix: &str) -> PathBuf {
    let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("safe-vips-{}-{}{}", std::process::id(), id, suffix))
}

pub fn decode_with_convert(
    bytes: &[u8],
    kind: ForeignKind,
    options: &LoadOptions,
) -> Result<crate::foreign::base::ForeignLoadResult, ()> {
    let suffix = match kind {
        ForeignKind::Jpeg => ".jpg",
        ForeignKind::Png => ".png",
        ForeignKind::Gif => ".gif",
        ForeignKind::Tiff => ".tif",
        ForeignKind::Webp => ".webp",
        ForeignKind::Heif => ".avif",
        ForeignKind::Svg => ".svg",
        ForeignKind::Pdf => ".pdf",
        ForeignKind::Radiance => ".hdr",
        ForeignKind::Ppm => ".ppm",
        ForeignKind::Pfm => ".pfm",
        ForeignKind::Matrix => ".mat",
        _ => ".dat",
    };
    let input = temp_path(suffix);
    let png_output = temp_path(".png");
    if std::fs::write(&input, bytes).is_err() {
        append_message_str("foreign", "unable to create convert input");
        return Err(());
    }

    let mut command = Command::new("convert");
    if matches!(kind, ForeignKind::Svg | ForeignKind::Pdf) {
        let density = options
            .dpi
            .or_else(|| options.scale.map(|value| value * 72.0))
            .unwrap_or(72.0);
        command.arg("-density").arg(format!("{density:.4}"));
    }

    let mut input_arg = input.to_string_lossy().into_owned();
    if matches!(
        kind,
        ForeignKind::Gif | ForeignKind::Tiff | ForeignKind::Pdf
    ) {
        match (options.page, options.n) {
            (Some(page), Some(n)) if n > 1 => {
                input_arg.push_str(&format!("[{}-{}]", page, page + n - 1));
            }
            (Some(page), Some(-1)) => {
                input_arg.push_str(&format!("[{}-]", page));
            }
            (Some(page), _) => {
                input_arg.push_str(&format!("[{page}]"));
            }
            (None, Some(-1)) => {}
            _ => {
                input_arg.push_str("[0]");
            }
        }
    }
    command.arg(input_arg);
    if matches!(kind, ForeignKind::Gif) {
        command.args(["-background", "none"]);
    }
    if matches!(kind, ForeignKind::Pdf) {
        command.args(["-background", "white", "-alpha", "remove", "-alpha", "off"]);
    }
    if matches!(options.n, Some(value) if value == -1 || value > 1) {
        command.arg("-append");
    }
    if matches!(kind, ForeignKind::Gif | ForeignKind::Svg | ForeignKind::Pdf) {
        command.arg(format!("PNG32:{}", png_output.to_string_lossy()));
    } else {
        command.arg(png_output.as_os_str());
    }

    let status = command.status().map_err(|err| {
        append_message_str("foreign", &format!("convert failed: {err}"));
    })?;
    if !status.success() {
        append_message_str("foreign", "convert failed");
        let _ = std::fs::remove_file(&input);
        let _ = std::fs::remove_file(&png_output);
        return Err(());
    }

    let png_bytes = std::fs::read(&png_output).map_err(|err| {
        append_message_str("foreign", &format!("unable to read convert output: {err}"));
    })?;
    let _ = std::fs::remove_file(&input);
    let _ = std::fs::remove_file(&png_output);

    let (mut pixels, width, height, bands, band_format, interpretation, _) =
        crate::runtime::image::safe_decode_png_bytes(&png_bytes).map_err(|message| {
            append_message_str("foreign", &message);
        })?;

    if bands == 4 && band_format == crate::abi::image::VIPS_FORMAT_UCHAR {
        for chunk in pixels.chunks_exact_mut(4) {
            if chunk[3] == 0 {
                chunk[0] = 0;
                chunk[1] = 0;
                chunk[2] = 0;
            }
        }
    }

    Ok(build_load_result(
        width as i32,
        height as i32,
        bands,
        band_format,
        interpretation,
        match kind {
            ForeignKind::Gif => "gifload",
            ForeignKind::Tiff => "tiffload",
            ForeignKind::Svg => "svgload",
            ForeignKind::Pdf => "pdfload",
            ForeignKind::Webp => "webpload",
            ForeignKind::Heif => "heifload",
            ForeignKind::Radiance => "radload",
            _ => "foreignload",
        },
        Some(pixels),
        ForeignMetadata::default(),
        None,
    ))
}
