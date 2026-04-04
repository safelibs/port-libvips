use std::ffi::CStr;
use std::path::{Path, PathBuf};

use crate::abi::image::{
    VipsBandFormat, VipsImage, VipsInterpretation, VIPS_FORMAT_FLOAT, VIPS_FORMAT_INT,
    VIPS_FORMAT_UCHAR, VIPS_FORMAT_USHORT, VIPS_INTERPRETATION_B_W, VIPS_INTERPRETATION_CMYK,
    VIPS_INTERPRETATION_GREY16, VIPS_INTERPRETATION_HISTOGRAM, VIPS_INTERPRETATION_HSV,
    VIPS_INTERPRETATION_LAB, VIPS_INTERPRETATION_LCH, VIPS_INTERPRETATION_MULTIBAND,
    VIPS_INTERPRETATION_RGB16, VIPS_INTERPRETATION_XYZ, VIPS_INTERPRETATION_YXY,
    VIPS_INTERPRETATION_sRGB, VIPS_INTERPRETATION_scRGB,
};
use crate::abi::object::VipsObject;
use crate::pixels::ImageBuffer;
use crate::runtime::header::vips_image_remove;
use crate::runtime::object::object_unref;

use super::{
    argument_assigned, get_image_buffer, get_image_ref, get_string, set_output_blob,
    set_output_image,
};

const ICC_META_NAME: &CStr = c"icc-profile-data";

fn base_bands(space: VipsInterpretation) -> usize {
    match space {
        VIPS_INTERPRETATION_B_W | VIPS_INTERPRETATION_GREY16 => 1,
        VIPS_INTERPRETATION_CMYK => 4,
        _ => 3,
    }
}

fn output_format(space: VipsInterpretation) -> VipsBandFormat {
    match space {
        VIPS_INTERPRETATION_B_W | VIPS_INTERPRETATION_HSV | VIPS_INTERPRETATION_sRGB => {
            VIPS_FORMAT_UCHAR
        }
        VIPS_INTERPRETATION_GREY16 | VIPS_INTERPRETATION_RGB16 => VIPS_FORMAT_USHORT,
        _ => VIPS_FORMAT_FLOAT,
    }
}

fn default_source_space(input: &ImageBuffer) -> VipsInterpretation {
    match input.spec.interpretation {
        VIPS_INTERPRETATION_MULTIBAND => {
            if input.spec.bands <= 1 {
                VIPS_INTERPRETATION_B_W
            } else {
                VIPS_INTERPRETATION_sRGB
            }
        }
        other => other,
    }
}

fn srgb_to_linear(value: f64) -> f64 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(value: f64) -> f64 {
    if value <= 0.003_130_8 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
}

fn rgb_to_xyz(rgb: [f64; 3]) -> [f64; 3] {
    [
        0.412_456_4 * rgb[0] + 0.357_576_1 * rgb[1] + 0.180_437_5 * rgb[2],
        0.212_672_9 * rgb[0] + 0.715_152_2 * rgb[1] + 0.072_175 * rgb[2],
        0.019_333_9 * rgb[0] + 0.119_192 * rgb[1] + 0.950_304_1 * rgb[2],
    ]
}

fn xyz_to_rgb(xyz: [f64; 3]) -> [f64; 3] {
    [
        3.240_454_2 * xyz[0] - 1.537_138_5 * xyz[1] - 0.498_531_4 * xyz[2],
        -0.969_266 * xyz[0] + 1.876_010_8 * xyz[1] + 0.041_556 * xyz[2],
        0.055_643_4 * xyz[0] - 0.204_025_9 * xyz[1] + 1.057_225_2 * xyz[2],
    ]
}

fn xyz_to_lab(xyz: [f64; 3]) -> [f64; 3] {
    let reference = [0.950_47, 1.0, 1.088_83];
    let convert = |value: f64| {
        if value > 0.008_856 {
            value.cbrt()
        } else {
            7.787 * value + 16.0 / 116.0
        }
    };
    let x = convert(xyz[0] / reference[0]);
    let y = convert(xyz[1] / reference[1]);
    let z = convert(xyz[2] / reference[2]);
    [116.0 * y - 16.0, 500.0 * (x - y), 200.0 * (y - z)]
}

fn lab_to_xyz(lab: [f64; 3]) -> [f64; 3] {
    let reference = [0.950_47, 1.0, 1.088_83];
    let fy = (lab[0] + 16.0) / 116.0;
    let fx = fy + lab[1] / 500.0;
    let fz = fy - lab[2] / 200.0;
    let convert = |value: f64| {
        let cube = value.powi(3);
        if cube > 0.008_856 {
            cube
        } else {
            (value - 16.0 / 116.0) / 7.787
        }
    };
    [
        convert(fx) * reference[0],
        convert(fy) * reference[1],
        convert(fz) * reference[2],
    ]
}

fn lab_to_lch(lab: [f64; 3]) -> [f64; 3] {
    let c = (lab[1] * lab[1] + lab[2] * lab[2]).sqrt();
    let mut h = lab[2].atan2(lab[1]).to_degrees();
    if h < 0.0 {
        h += 360.0;
    }
    [lab[0], c, h]
}

fn lch_to_lab(lch: [f64; 3]) -> [f64; 3] {
    let angle = lch[2].to_radians();
    [lch[0], lch[1] * angle.cos(), lch[1] * angle.sin()]
}

fn xyz_to_yxy(xyz: [f64; 3]) -> [f64; 3] {
    let sum = xyz[0] + xyz[1] + xyz[2];
    if sum.abs() < f64::EPSILON {
        [0.0, 0.0, 0.0]
    } else {
        [xyz[1], xyz[0] / sum, xyz[1] / sum]
    }
}

fn yxy_to_xyz(yxy: [f64; 3]) -> [f64; 3] {
    if yxy[2].abs() < f64::EPSILON {
        [0.0, 0.0, 0.0]
    } else {
        let x = yxy[0] * yxy[1] / yxy[2];
        let z = yxy[0] * (1.0 - yxy[1] - yxy[2]) / yxy[2];
        [x, yxy[0], z]
    }
}

fn rgb_to_cmyk(rgb: [f64; 3]) -> [f64; 4] {
    let k = 1.0 - rgb[0].max(rgb[1]).max(rgb[2]).clamp(0.0, 1.0);
    if k >= 1.0 {
        [0.0, 0.0, 0.0, 1.0]
    } else {
        [
            (1.0 - rgb[0] - k) / (1.0 - k),
            (1.0 - rgb[1] - k) / (1.0 - k),
            (1.0 - rgb[2] - k) / (1.0 - k),
            k,
        ]
    }
}

fn cmyk_to_rgb(cmyk: [f64; 4]) -> [f64; 3] {
    [
        (1.0 - cmyk[0]) * (1.0 - cmyk[3]),
        (1.0 - cmyk[1]) * (1.0 - cmyk[3]),
        (1.0 - cmyk[2]) * (1.0 - cmyk[3]),
    ]
}

fn hsv_to_rgb8(hsv: [f64; 3]) -> [f64; 3] {
    let h = (hsv[0] / 255.0) * 360.0;
    let s = (hsv[1] / 255.0).clamp(0.0, 1.0);
    let v = (hsv[2] / 255.0).clamp(0.0, 1.0);
    if s <= 0.0 {
        return [v * 255.0, v * 255.0, v * 255.0];
    }
    let h = (h / 60.0).rem_euclid(6.0);
    let i = h.floor() as i32;
    let f = h - i as f64;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);
    let rgb = match i {
        0 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        _ => [v, p, q],
    };
    [rgb[0] * 255.0, rgb[1] * 255.0, rgb[2] * 255.0]
}

fn rgb8_to_hsv(rgb: [f64; 3]) -> [f64; 3] {
    let r = (rgb[0] / 255.0).clamp(0.0, 1.0);
    let g = (rgb[1] / 255.0).clamp(0.0, 1.0);
    let b = (rgb[2] / 255.0).clamp(0.0, 1.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let hue = if delta <= 0.0 {
        0.0
    } else if (max - r).abs() < f64::EPSILON {
        60.0 * ((g - b) / delta).rem_euclid(6.0)
    } else if (max - g).abs() < f64::EPSILON {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };
    let sat = if max <= 0.0 { 0.0 } else { delta / max };
    [(hue / 360.0) * 255.0, sat * 255.0, max * 255.0]
}

fn to_linear_rgb(space: VipsInterpretation, values: &[f64]) -> [f64; 3] {
    match space {
        VIPS_INTERPRETATION_B_W => {
            let grey = srgb_to_linear((values.first().copied().unwrap_or(0.0) / 255.0).clamp(0.0, 1.0));
            [grey, grey, grey]
        }
        VIPS_INTERPRETATION_GREY16 => {
            let grey = (values.first().copied().unwrap_or(0.0) / 65_535.0).clamp(0.0, 1.0);
            [grey, grey, grey]
        }
        VIPS_INTERPRETATION_RGB16 => {
            let get = |index| {
                let value = values.get(index).copied().unwrap_or(0.0_f64) / 65_535.0_f64;
                srgb_to_linear(value.clamp(0.0_f64, 1.0_f64))
            };
            [get(0), get(1), get(2)]
        }
        VIPS_INTERPRETATION_HSV => {
            let rgb = hsv_to_rgb8([
                values.first().copied().unwrap_or(0.0),
                values.get(1).copied().unwrap_or(0.0),
                values.get(2).copied().unwrap_or(0.0),
            ]);
            [
                srgb_to_linear((rgb[0] / 255.0).clamp(0.0, 1.0)),
                srgb_to_linear((rgb[1] / 255.0).clamp(0.0, 1.0)),
                srgb_to_linear((rgb[2] / 255.0).clamp(0.0, 1.0)),
            ]
        }
        VIPS_INTERPRETATION_scRGB => [
            values.first().copied().unwrap_or(0.0),
            values.get(1).copied().unwrap_or(0.0),
            values.get(2).copied().unwrap_or(0.0),
        ],
        VIPS_INTERPRETATION_XYZ => xyz_to_rgb([
            values.first().copied().unwrap_or(0.0),
            values.get(1).copied().unwrap_or(0.0),
            values.get(2).copied().unwrap_or(0.0),
        ]),
        VIPS_INTERPRETATION_LAB => xyz_to_rgb(lab_to_xyz([
            values.first().copied().unwrap_or(0.0),
            values.get(1).copied().unwrap_or(0.0),
            values.get(2).copied().unwrap_or(0.0),
        ])),
        VIPS_INTERPRETATION_LCH => xyz_to_rgb(lab_to_xyz(lch_to_lab([
            values.first().copied().unwrap_or(0.0),
            values.get(1).copied().unwrap_or(0.0),
            values.get(2).copied().unwrap_or(0.0),
        ]))),
        VIPS_INTERPRETATION_YXY => xyz_to_rgb(yxy_to_xyz([
            values.first().copied().unwrap_or(0.0),
            values.get(1).copied().unwrap_or(0.0),
            values.get(2).copied().unwrap_or(0.0),
        ])),
        VIPS_INTERPRETATION_CMYK => cmyk_to_rgb([
            values.first().copied().unwrap_or(0.0),
            values.get(1).copied().unwrap_or(0.0),
            values.get(2).copied().unwrap_or(0.0),
            values.get(3).copied().unwrap_or(0.0),
        ]),
        _ => {
            let get = |index| {
                let value =
                    values.get(index).copied().unwrap_or(values.first().copied().unwrap_or(0.0_f64))
                        / 255.0_f64;
                srgb_to_linear(value.clamp(0.0_f64, 1.0_f64))
            };
            [get(0), get(1), get(2)]
        }
    }
}

fn from_linear_rgb(space: VipsInterpretation, rgb: [f64; 3]) -> Vec<f64> {
    match space {
        VIPS_INTERPRETATION_B_W => {
            let grey = 0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2];
            vec![linear_to_srgb(grey.clamp(0.0, 1.0)) * 255.0]
        }
        VIPS_INTERPRETATION_GREY16 => {
            let grey = 0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2];
            vec![grey.clamp(0.0, 1.0) * 65_535.0]
        }
        VIPS_INTERPRETATION_RGB16 => vec![
            linear_to_srgb(rgb[0].clamp(0.0, 1.0)) * 65_535.0,
            linear_to_srgb(rgb[1].clamp(0.0, 1.0)) * 65_535.0,
            linear_to_srgb(rgb[2].clamp(0.0, 1.0)) * 65_535.0,
        ],
        VIPS_INTERPRETATION_HSV => {
            let rgb8 = [
                linear_to_srgb(rgb[0].clamp(0.0, 1.0)) * 255.0,
                linear_to_srgb(rgb[1].clamp(0.0, 1.0)) * 255.0,
                linear_to_srgb(rgb[2].clamp(0.0, 1.0)) * 255.0,
            ];
            rgb8_to_hsv(rgb8).to_vec()
        }
        VIPS_INTERPRETATION_scRGB => vec![rgb[0], rgb[1], rgb[2]],
        VIPS_INTERPRETATION_XYZ => rgb_to_xyz(rgb).to_vec(),
        VIPS_INTERPRETATION_LAB => xyz_to_lab(rgb_to_xyz(rgb)).to_vec(),
        VIPS_INTERPRETATION_LCH => lab_to_lch(xyz_to_lab(rgb_to_xyz(rgb))).to_vec(),
        VIPS_INTERPRETATION_YXY => xyz_to_yxy(rgb_to_xyz(rgb)).to_vec(),
        VIPS_INTERPRETATION_CMYK => rgb_to_cmyk(rgb).to_vec(),
        _ => vec![
            linear_to_srgb(rgb[0].clamp(0.0, 1.0)) * 255.0,
            linear_to_srgb(rgb[1].clamp(0.0, 1.0)) * 255.0,
            linear_to_srgb(rgb[2].clamp(0.0, 1.0)) * 255.0,
        ],
    }
}

fn transform_buffer(
    input: &ImageBuffer,
    source_space: VipsInterpretation,
    target_space: VipsInterpretation,
) -> Result<ImageBuffer, ()> {
    let source_bands = base_bands(source_space).min(input.spec.bands.max(1));
    let target_bands = base_bands(target_space);
    let extras = input.spec.bands.saturating_sub(source_bands);

    let mut out = ImageBuffer::new(
        input.spec.width,
        input.spec.height,
        target_bands + extras,
        output_format(target_space),
        input.spec.coding,
        target_space,
    );
    out.spec.xres = input.spec.xres;
    out.spec.yres = input.spec.yres;
    out.spec.xoffset = input.spec.xoffset;
    out.spec.yoffset = input.spec.yoffset;
    out.spec.dhint = input.spec.dhint;

    for y in 0..input.spec.height {
        for x in 0..input.spec.width {
            let samples = (0..source_bands)
                .map(|band| input.get(x, y, band))
                .collect::<Vec<_>>();
            let rgb = to_linear_rgb(source_space, &samples);
            let converted = from_linear_rgb(target_space, rgb);
            for band in 0..target_bands {
                out.set(x, y, band, converted.get(band).copied().unwrap_or(0.0));
            }
            for extra in 0..extras {
                out.set(
                    x,
                    y,
                    target_bands + extra,
                    input.get(x, y, source_bands + extra),
                );
            }
        }
    }

    Ok(out)
}

fn repo_profile_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repo root")
        .join("original/libvips/colour/profiles")
}

fn resolve_profile_path(name: &str) -> Option<PathBuf> {
    let candidate = Path::new(name);
    if candidate.exists() {
        return Some(candidate.to_path_buf());
    }

    let normalized = name.to_ascii_lowercase();
    let filename = match normalized.as_str() {
        "srgb" | "srgb.icm" | "srgb.icc" => "sRGB.icm",
        "sgrey" | "sgrey.icm" | "sgrey.icc" => "sGrey.icm",
        "p3" | "p3.icm" | "p3.icc" => "p3.icm",
        "cmyk" | "cmyk.icm" | "cmyk.icc" => "cmyk.icm",
        _ => return None,
    };
    Some(repo_profile_dir().join(filename))
}

unsafe fn set_transformed_output(
    object: *mut VipsObject,
    buffer: ImageBuffer,
    like: *mut VipsImage,
) -> Result<(), ()> {
    let out = buffer.into_image_like(like);
    vips_image_remove(out, ICC_META_NAME.as_ptr());
    unsafe { set_output_image(object, "out", out) }
}

unsafe fn op_colourspace(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let like = unsafe { get_image_ref(object, "in")? };
    let target_space = unsafe { super::get_enum(object, "space")? } as VipsInterpretation;
    let source_space = if unsafe { argument_assigned(object, "source_space")? } {
        unsafe { super::get_enum(object, "source_space")? as VipsInterpretation }
    } else {
        default_source_space(&input)
    };
    let result = transform_buffer(&input, source_space, target_space);
    let status = match result {
        Ok(buffer) => unsafe { set_transformed_output(object, buffer, like) },
        Err(()) => Err(()),
    };
    unsafe {
        object_unref(like);
    }
    status
}

unsafe fn op_named_transform(
    object: *mut VipsObject,
    source_space: VipsInterpretation,
    target_space: VipsInterpretation,
) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let like = unsafe { get_image_ref(object, "in")? };
    let result = transform_buffer(&input, source_space, target_space);
    let status = match result {
        Ok(buffer) => unsafe { set_transformed_output(object, buffer, like) },
        Err(()) => Err(()),
    };
    unsafe {
        object_unref(like);
    }
    status
}

unsafe fn op_profile(object: *mut VipsObject) -> Result<(), ()> {
    let input = unsafe { get_image_buffer(object, "in")? };
    let like = unsafe { get_image_ref(object, "in")? };

    let mut columns = ImageBuffer::new(
        input.spec.width,
        1,
        input.spec.bands,
        VIPS_FORMAT_INT,
        input.spec.coding,
        VIPS_INTERPRETATION_HISTOGRAM,
    );
    let mut rows = ImageBuffer::new(
        1,
        input.spec.height,
        input.spec.bands,
        VIPS_FORMAT_INT,
        input.spec.coding,
        VIPS_INTERPRETATION_HISTOGRAM,
    );

    for x in 0..input.spec.width {
        for band in 0..input.spec.bands {
            let edge = (0..input.spec.height)
                .find(|&y| input.get(x, y, band) != 0.0)
                .unwrap_or(input.spec.height);
            columns.set(x, 0, band, edge as f64);
        }
    }
    for y in 0..input.spec.height {
        for band in 0..input.spec.bands {
            let edge = (0..input.spec.width)
                .find(|&x| input.get(x, y, band) != 0.0)
                .unwrap_or(input.spec.width);
            rows.set(0, y, band, edge as f64);
        }
    }

    let columns_image = columns.into_image_like(like);
    let rows_image = rows.into_image_like(like);
    let result = unsafe {
        set_output_image(object, "columns", columns_image)
            .and_then(|_| set_output_image(object, "rows", rows_image))
    };
    unsafe {
        object_unref(like);
    }
    result
}

unsafe fn op_profile_load(object: *mut VipsObject) -> Result<(), ()> {
    let name = unsafe { get_string(object, "name")? }.ok_or(())?;
    if name.eq_ignore_ascii_case("none") {
        return unsafe { set_output_blob(object, "profile", Vec::new()) };
    }
    let path = resolve_profile_path(&name).ok_or(())?;
    let bytes = std::fs::read(path).map_err(|_| ())?;
    unsafe { set_output_blob(object, "profile", bytes) }
}

pub(crate) unsafe fn dispatch(object: *mut VipsObject, nickname: &str) -> Result<bool, ()> {
    match nickname {
        "colourspace" => {
            unsafe { op_colourspace(object)? };
            Ok(true)
        }
        "profile" => {
            unsafe { op_profile(object)? };
            Ok(true)
        }
        "profile_load" => {
            unsafe { op_profile_load(object)? };
            Ok(true)
        }
        "sRGB2HSV" => {
            unsafe { op_named_transform(object, VIPS_INTERPRETATION_sRGB, VIPS_INTERPRETATION_HSV)? };
            Ok(true)
        }
        "HSV2sRGB" => {
            unsafe { op_named_transform(object, VIPS_INTERPRETATION_HSV, VIPS_INTERPRETATION_sRGB)? };
            Ok(true)
        }
        "sRGB2scRGB" => {
            unsafe { op_named_transform(object, VIPS_INTERPRETATION_sRGB, VIPS_INTERPRETATION_scRGB)? };
            Ok(true)
        }
        "scRGB2sRGB" => {
            unsafe { op_named_transform(object, VIPS_INTERPRETATION_scRGB, VIPS_INTERPRETATION_sRGB)? };
            Ok(true)
        }
        "scRGB2BW" => {
            unsafe { op_named_transform(object, VIPS_INTERPRETATION_scRGB, VIPS_INTERPRETATION_B_W)? };
            Ok(true)
        }
        "XYZ2Lab" => {
            unsafe { op_named_transform(object, VIPS_INTERPRETATION_XYZ, VIPS_INTERPRETATION_LAB)? };
            Ok(true)
        }
        "Lab2XYZ" => {
            unsafe { op_named_transform(object, VIPS_INTERPRETATION_LAB, VIPS_INTERPRETATION_XYZ)? };
            Ok(true)
        }
        "Lab2LCh" => {
            unsafe { op_named_transform(object, VIPS_INTERPRETATION_LAB, VIPS_INTERPRETATION_LCH)? };
            Ok(true)
        }
        "LCh2Lab" => {
            unsafe { op_named_transform(object, VIPS_INTERPRETATION_LCH, VIPS_INTERPRETATION_LAB)? };
            Ok(true)
        }
        "XYZ2Yxy" => {
            unsafe { op_named_transform(object, VIPS_INTERPRETATION_XYZ, VIPS_INTERPRETATION_YXY)? };
            Ok(true)
        }
        "Yxy2XYZ" => {
            unsafe { op_named_transform(object, VIPS_INTERPRETATION_YXY, VIPS_INTERPRETATION_XYZ)? };
            Ok(true)
        }
        "XYZ2scRGB" => {
            unsafe { op_named_transform(object, VIPS_INTERPRETATION_XYZ, VIPS_INTERPRETATION_scRGB)? };
            Ok(true)
        }
        "scRGB2XYZ" => {
            unsafe { op_named_transform(object, VIPS_INTERPRETATION_scRGB, VIPS_INTERPRETATION_XYZ)? };
            Ok(true)
        }
        _ => Ok(false),
    }
}
