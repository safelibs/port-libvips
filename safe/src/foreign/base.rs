use std::collections::BTreeMap;

use crate::abi::image::{
    VIPS_INTERPRETATION_sRGB, VipsAccess, VipsBandFormat, VipsCoding, VipsInterpretation,
    VIPS_CODING_NONE, VIPS_FORMAT_UCHAR, VIPS_FORMAT_USHORT, VIPS_INTERPRETATION_B_W,
    VIPS_INTERPRETATION_GREY16, VIPS_INTERPRETATION_MULTIBAND,
};

pub const CONTAINER_MAGIC: &[u8; 8] = b"SVIPSC01";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForeignKind {
    Unknown,
    Jpeg,
    Png,
    Gif,
    Tiff,
    Webp,
    Heif,
    Svg,
    Pdf,
    Ppm,
    Pfm,
    Csv,
    Matrix,
    Raw,
    Vips,
    Radiance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputKind {
    File,
    Buffer,
    Source,
}

#[derive(Clone, Debug, Default)]
pub struct LoadOptions {
    pub access: Option<VipsAccess>,
    pub autorotate: bool,
    pub dpi: Option<f64>,
    pub fail_on: Option<String>,
    pub memory: bool,
    pub n: Option<i32>,
    pub page: Option<i32>,
    pub revalidate: bool,
    pub scale: Option<f64>,
    pub unlimited: bool,
}

#[derive(Clone, Debug, Default)]
pub struct SaveOptions {
    pub bitdepth: Option<i32>,
    pub keep: Option<String>,
    pub profile: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct ForeignMetadata {
    pub blobs: BTreeMap<String, Vec<u8>>,
    pub ints: BTreeMap<String, i32>,
    pub strings: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct PendingDecode {
    pub bytes: Vec<u8>,
    pub kind: ForeignKind,
    pub options: LoadOptions,
}

#[derive(Clone, Debug)]
pub struct ForeignLoadResult {
    pub width: i32,
    pub height: i32,
    pub bands: i32,
    pub band_format: VipsBandFormat,
    pub coding: VipsCoding,
    pub interpretation: VipsInterpretation,
    pub pixels: Option<Vec<u8>>,
    pub metadata: ForeignMetadata,
    pub pending: Option<PendingDecode>,
    pub loader_name: &'static str,
}

impl ForeignMetadata {
    pub fn with_string(mut self, name: &str, value: impl Into<String>) -> Self {
        self.strings.insert(name.to_owned(), value.into());
        self
    }

    pub fn with_int(mut self, name: &str, value: i32) -> Self {
        self.ints.insert(name.to_owned(), value);
        self
    }

    pub fn insert_blob(&mut self, name: &str, value: Vec<u8>) {
        self.blobs.insert(name.to_owned(), value);
    }
}

pub fn interpretation_for_png(bits: u8, bands: i32) -> VipsInterpretation {
    match (bands, bits) {
        (1, 8) => VIPS_INTERPRETATION_B_W,
        (1, 16) => VIPS_INTERPRETATION_GREY16,
        (3 | 4, 8) => VIPS_INTERPRETATION_sRGB,
        _ => VIPS_INTERPRETATION_MULTIBAND,
    }
}

pub fn band_format_for_bits(bits: u8) -> VipsBandFormat {
    match bits {
        16 => VIPS_FORMAT_USHORT,
        _ => VIPS_FORMAT_UCHAR,
    }
}

pub fn build_load_result(
    width: i32,
    height: i32,
    bands: i32,
    band_format: VipsBandFormat,
    interpretation: VipsInterpretation,
    loader_name: &'static str,
    pixels: Option<Vec<u8>>,
    metadata: ForeignMetadata,
    pending: Option<PendingDecode>,
) -> ForeignLoadResult {
    ForeignLoadResult {
        width,
        height,
        bands,
        band_format,
        coding: VIPS_CODING_NONE,
        interpretation,
        pixels,
        metadata,
        pending,
        loader_name,
    }
}

pub fn parse_option_string(text: &str) -> BTreeMap<String, String> {
    let mut options = BTreeMap::new();
    for item in text.split(',') {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        if let Some((key, value)) = item.split_once('=') {
            options.insert(key.trim().to_owned(), value.trim().to_owned());
        } else {
            options.insert(item.to_owned(), "true".to_owned());
        }
    }
    options
}

pub fn options_from_map(raw: &BTreeMap<String, String>) -> LoadOptions {
    let mut options = LoadOptions::default();
    if let Some(value) = raw.get("access") {
        options.access = match value.to_ascii_lowercase().as_str() {
            "random" => Some(crate::abi::image::VIPS_ACCESS_RANDOM),
            "sequential" => Some(crate::abi::image::VIPS_ACCESS_SEQUENTIAL),
            "sequential-unbuffered" | "sequential_unbuffered" | "sequentialunbuffered" => {
                Some(crate::abi::image::VIPS_ACCESS_SEQUENTIAL_UNBUFFERED)
            }
            _ => Some(crate::abi::image::VIPS_ACCESS_RANDOM),
        };
    }
    if let Some(value) = raw.get("page") {
        options.page = value.parse().ok();
    }
    if let Some(value) = raw.get("n") {
        options.n = value.parse().ok();
    }
    if let Some(value) = raw.get("dpi") {
        options.dpi = value.parse().ok();
    }
    if let Some(value) = raw.get("scale") {
        options.scale = value.parse().ok();
    }
    if let Some(value) = raw.get("fail_on") {
        options.fail_on = Some(value.clone());
    }
    options.autorotate = raw
        .get("autorotate")
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "True"));
    options.revalidate = raw
        .get("revalidate")
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "True"));
    options.memory = raw
        .get("memory")
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "True"));
    options.unlimited = raw
        .get("unlimited")
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "True"));
    options
}

pub fn save_options_from_map(raw: &BTreeMap<String, String>) -> SaveOptions {
    SaveOptions {
        bitdepth: raw.get("bitdepth").and_then(|value| value.parse().ok()),
        keep: raw.get("keep").cloned(),
        profile: raw.get("profile").cloned(),
    }
}

pub fn parse_embedded_options(text: &str) -> (String, String) {
    if let Some(start) = text.rfind('[') {
        if text.ends_with(']') && start + 1 < text.len() {
            return (
                text[..start].to_owned(),
                text[start + 1..text.len() - 1].to_owned(),
            );
        }
    }
    (text.to_owned(), String::new())
}

pub fn loader_name(kind: ForeignKind, input_kind: InputKind) -> Option<&'static str> {
    match (kind, input_kind) {
        (ForeignKind::Jpeg, InputKind::File) => Some("jpegload"),
        (ForeignKind::Jpeg, InputKind::Buffer) => Some("jpegload_buffer"),
        (ForeignKind::Jpeg, InputKind::Source) => Some("jpegload_source"),
        (ForeignKind::Png, InputKind::File) => Some("pngload"),
        (ForeignKind::Png, InputKind::Buffer) => Some("pngload_buffer"),
        (ForeignKind::Png, InputKind::Source) => Some("pngload_source"),
        (ForeignKind::Gif, InputKind::File) => Some("gifload"),
        (ForeignKind::Gif, InputKind::Buffer) => Some("gifload_buffer"),
        (ForeignKind::Gif, InputKind::Source) => Some("gifload_source"),
        (ForeignKind::Tiff, InputKind::File) => Some("tiffload"),
        (ForeignKind::Tiff, InputKind::Buffer) => Some("tiffload_buffer"),
        (ForeignKind::Tiff, InputKind::Source) => Some("tiffload_source"),
        (ForeignKind::Vips, InputKind::File) => Some("vipsload"),
        (ForeignKind::Vips, InputKind::Source) => Some("vipsload_source"),
        (ForeignKind::Svg, InputKind::File) => Some("svgload"),
        (ForeignKind::Svg, InputKind::Buffer) => Some("svgload_buffer"),
        (ForeignKind::Svg, InputKind::Source) => Some("svgload_source"),
        (ForeignKind::Pdf, InputKind::File) => Some("pdfload"),
        (ForeignKind::Pdf, InputKind::Buffer) => Some("pdfload_buffer"),
        (ForeignKind::Pdf, InputKind::Source) => Some("pdfload_source"),
        (ForeignKind::Webp, InputKind::File) => Some("webpload"),
        (ForeignKind::Webp, InputKind::Buffer) => Some("webpload_buffer"),
        (ForeignKind::Webp, InputKind::Source) => Some("webpload_source"),
        (ForeignKind::Heif, InputKind::File) => Some("heifload"),
        (ForeignKind::Heif, InputKind::Buffer) => Some("heifload_buffer"),
        (ForeignKind::Heif, InputKind::Source) => Some("heifload_source"),
        (ForeignKind::Ppm, InputKind::File) => Some("ppmload"),
        (ForeignKind::Ppm, InputKind::Source) => Some("ppmload_source"),
        (ForeignKind::Pfm, InputKind::File) => Some("ppmload"),
        (ForeignKind::Pfm, InputKind::Source) => Some("ppmload_source"),
        (ForeignKind::Radiance, InputKind::File) => Some("radload"),
        (ForeignKind::Radiance, InputKind::Buffer) => Some("radload_buffer"),
        (ForeignKind::Radiance, InputKind::Source) => Some("radload_source"),
        (ForeignKind::Csv, InputKind::File) => Some("csvload"),
        (ForeignKind::Csv, InputKind::Source) => Some("csvload_source"),
        (ForeignKind::Matrix, InputKind::File) => Some("matrixload"),
        (ForeignKind::Matrix, InputKind::Source) => Some("matrixload_source"),
        _ => None,
    }
}

pub fn file_save_name(kind: ForeignKind) -> Option<&'static str> {
    match kind {
        ForeignKind::Jpeg => Some("jpegsave"),
        ForeignKind::Png => Some("pngsave"),
        ForeignKind::Tiff => Some("tiffsave"),
        ForeignKind::Webp => Some("webpsave"),
        ForeignKind::Heif => Some("heifsave"),
        ForeignKind::Vips => Some("vipssave"),
        ForeignKind::Ppm | ForeignKind::Pfm => Some("ppmsave"),
        ForeignKind::Csv => Some("csvsave"),
        ForeignKind::Matrix => Some("matrixsave"),
        ForeignKind::Radiance => Some("radsave"),
        _ => None,
    }
}

pub fn buffer_save_name(kind: ForeignKind) -> Option<&'static str> {
    match kind {
        ForeignKind::Jpeg => Some("jpegsave_buffer"),
        ForeignKind::Png => Some("pngsave_buffer"),
        ForeignKind::Tiff => Some("tiffsave_buffer"),
        ForeignKind::Webp => Some("webpsave_buffer"),
        ForeignKind::Heif => Some("heifsave_buffer"),
        ForeignKind::Radiance => Some("radsave_buffer"),
        _ => None,
    }
}

pub fn target_save_name(kind: ForeignKind) -> Option<&'static str> {
    match kind {
        ForeignKind::Png => Some("pngsave_target"),
        ForeignKind::Jpeg => Some("jpegsave_target"),
        ForeignKind::Tiff => Some("tiffsave_target"),
        ForeignKind::Webp => Some("webpsave_target"),
        ForeignKind::Heif => Some("heifsave_target"),
        ForeignKind::Vips => Some("vipssave_target"),
        ForeignKind::Ppm | ForeignKind::Pfm => Some("ppmsave_target"),
        ForeignKind::Csv => Some("csvsave_target"),
        ForeignKind::Matrix => Some("matrixsave_target"),
        ForeignKind::Radiance => Some("radsave_target"),
        _ => None,
    }
}
