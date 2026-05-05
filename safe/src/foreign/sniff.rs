use crate::foreign::base::{ForeignKind, CONTAINER_MAGIC};

const VIPS_MAGIC_INTEL_BYTES: &[u8; 4] = &[0x08, 0xf2, 0xa6, 0xb6];
const VIPS_MAGIC_SPARC_BYTES: &[u8; 4] = &[0xb6, 0xa6, 0xf2, 0x08];

pub fn kind_from_suffix(filename: &str) -> ForeignKind {
    let lower = filename.to_ascii_lowercase();
    if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        ForeignKind::Jpeg
    } else if lower.ends_with(".png") {
        ForeignKind::Png
    } else if lower.ends_with(".gif") {
        ForeignKind::Gif
    } else if lower.ends_with(".tif") || lower.ends_with(".tiff") {
        ForeignKind::Tiff
    } else if lower.ends_with(".webp") {
        ForeignKind::Webp
    } else if lower.ends_with(".avif")
        || lower.ends_with(".heif")
        || lower.ends_with(".heic")
        || lower.ends_with(".hif")
    {
        ForeignKind::Heif
    } else if lower.ends_with(".svg") || lower.ends_with(".svgz") || lower.ends_with(".svg.gz") {
        ForeignKind::Svg
    } else if lower.ends_with(".pdf") {
        ForeignKind::Pdf
    } else if lower.ends_with(".ppm") || lower.ends_with(".pgm") || lower.ends_with(".pbm") {
        ForeignKind::Ppm
    } else if lower.ends_with(".pfm") {
        ForeignKind::Pfm
    } else if lower.ends_with(".csv") {
        ForeignKind::Csv
    } else if lower.ends_with(".mat")
        || lower.ends_with(".mtx")
        || lower.ends_with(".matrix")
        || lower.ends_with(".con")
    {
        ForeignKind::Matrix
    } else if lower.ends_with(".v") {
        ForeignKind::Vips
    } else if lower.ends_with(".hdr") {
        ForeignKind::Radiance
    } else {
        ForeignKind::Unknown
    }
}

pub fn kind_from_bytes(bytes: &[u8], filename_hint: Option<&str>) -> ForeignKind {
    if bytes.len() >= CONTAINER_MAGIC.len() && &bytes[..CONTAINER_MAGIC.len()] == CONTAINER_MAGIC {
        return match bytes
            .get(CONTAINER_MAGIC.len())
            .copied()
            .unwrap_or_default()
        {
            1 => ForeignKind::Jpeg,
            2 => ForeignKind::Png,
            3 => ForeignKind::Gif,
            4 => ForeignKind::Tiff,
            5 => ForeignKind::Webp,
            15 => ForeignKind::Heif,
            6 => ForeignKind::Svg,
            7 => ForeignKind::Pdf,
            8 => ForeignKind::Ppm,
            9 => ForeignKind::Pfm,
            10 => ForeignKind::Csv,
            11 => ForeignKind::Matrix,
            12 => ForeignKind::Raw,
            13 => ForeignKind::Vips,
            14 => ForeignKind::Radiance,
            _ => ForeignKind::Vips,
        };
    }

    if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        return ForeignKind::Jpeg;
    }
    if bytes.starts_with(VIPS_MAGIC_INTEL_BYTES) || bytes.starts_with(VIPS_MAGIC_SPARC_BYTES) {
        return ForeignKind::Vips;
    }
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return ForeignKind::Png;
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return ForeignKind::Gif;
    }
    if bytes.starts_with(b"II*\0") || bytes.starts_with(b"MM\0*") {
        return ForeignKind::Tiff;
    }
    if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        return ForeignKind::Webp;
    }
    if bytes.len() >= 12 && bytes.get(4..8) == Some(b"ftyp") {
        let brand = &bytes[8..12];
        if matches!(
            brand,
            b"avif" | b"avis" | b"heic" | b"heix" | b"hevc" | b"hevx" | b"mif1" | b"msf1"
        ) {
            return ForeignKind::Heif;
        }
    }
    if bytes.starts_with(b"%PDF-") {
        return ForeignKind::Pdf;
    }
    if bytes.starts_with(b"P5")
        || bytes.starts_with(b"P6")
        || bytes.starts_with(b"P2")
        || bytes.starts_with(b"P3")
    {
        return ForeignKind::Ppm;
    }
    if bytes.starts_with(b"PF") || bytes.starts_with(b"Pf") {
        return ForeignKind::Pfm;
    }
    if bytes.starts_with(b"#?RADIANCE") || bytes.starts_with(b"#?RGBE") {
        return ForeignKind::Radiance;
    }
    let trimmed = String::from_utf8_lossy(&bytes[..bytes.len().min(256)]);
    let trimmed = trimmed.trim_start_matches('\u{feff}').trim_start();
    if trimmed.starts_with("<svg")
        || (trimmed.starts_with("<?xml") && trimmed.contains("<svg"))
        || trimmed.starts_with("<!DOCTYPE svg")
    {
        return ForeignKind::Svg;
    }

    filename_hint
        .map(kind_from_suffix)
        .unwrap_or(ForeignKind::Unknown)
}

pub fn is_public_operation(name: &str) -> bool {
    matches!(
        name,
        "jpegload"
            | "jpegload_buffer"
            | "jpegload_source"
            | "jpegsave_buffer"
            | "rawload"
            | "rawsave"
            | "ppmload_buffer"
            | "csvload_source"
            | "csvsave_target"
            | "matrixload_source"
            | "matrixsave_target"
            | "ppmload_source"
            | "ppmsave_target"
    )
}
