use crate::abi::image::{
    VipsBandFormat, VIPS_FORMAT_CHAR, VIPS_FORMAT_COMPLEX, VIPS_FORMAT_DOUBLE,
    VIPS_FORMAT_DPCOMPLEX, VIPS_FORMAT_FLOAT, VIPS_FORMAT_INT, VIPS_FORMAT_SHORT,
    VIPS_FORMAT_UCHAR, VIPS_FORMAT_UINT, VIPS_FORMAT_USHORT,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NumericKind {
    Unsigned,
    Signed,
    Float,
    Complex,
}

pub(crate) fn format_kind(format: VipsBandFormat) -> Option<NumericKind> {
    match format {
        VIPS_FORMAT_UCHAR | VIPS_FORMAT_USHORT | VIPS_FORMAT_UINT => Some(NumericKind::Unsigned),
        VIPS_FORMAT_CHAR | VIPS_FORMAT_SHORT | VIPS_FORMAT_INT => Some(NumericKind::Signed),
        VIPS_FORMAT_FLOAT | VIPS_FORMAT_DOUBLE => Some(NumericKind::Float),
        VIPS_FORMAT_COMPLEX | VIPS_FORMAT_DPCOMPLEX => Some(NumericKind::Complex),
        _ => None,
    }
}

pub(crate) fn format_bytes(format: VipsBandFormat) -> usize {
    match format {
        VIPS_FORMAT_UCHAR | VIPS_FORMAT_CHAR => 1,
        VIPS_FORMAT_USHORT | VIPS_FORMAT_SHORT => 2,
        VIPS_FORMAT_UINT | VIPS_FORMAT_INT | VIPS_FORMAT_FLOAT => 4,
        VIPS_FORMAT_COMPLEX | VIPS_FORMAT_DOUBLE => 8,
        VIPS_FORMAT_DPCOMPLEX => 16,
        _ => 0,
    }
}

pub(crate) fn format_components(format: VipsBandFormat) -> usize {
    match format {
        VIPS_FORMAT_COMPLEX | VIPS_FORMAT_DPCOMPLEX => 2,
        _ => 1,
    }
}

pub(crate) fn format_min(format: VipsBandFormat) -> Option<f64> {
    match format {
        VIPS_FORMAT_UCHAR => Some(u8::MIN as f64),
        VIPS_FORMAT_CHAR => Some(i8::MIN as f64),
        VIPS_FORMAT_USHORT => Some(u16::MIN as f64),
        VIPS_FORMAT_SHORT => Some(i16::MIN as f64),
        VIPS_FORMAT_UINT => Some(u32::MIN as f64),
        VIPS_FORMAT_INT => Some(i32::MIN as f64),
        VIPS_FORMAT_FLOAT => Some(f32::MIN as f64),
        VIPS_FORMAT_DOUBLE => Some(f64::MIN),
        _ => None,
    }
}

pub(crate) fn format_max(format: VipsBandFormat) -> Option<f64> {
    match format {
        VIPS_FORMAT_UCHAR => Some(u8::MAX as f64),
        VIPS_FORMAT_CHAR => Some(i8::MAX as f64),
        VIPS_FORMAT_USHORT => Some(u16::MAX as f64),
        VIPS_FORMAT_SHORT => Some(i16::MAX as f64),
        VIPS_FORMAT_UINT => Some(u32::MAX as f64),
        VIPS_FORMAT_INT => Some(i32::MAX as f64),
        VIPS_FORMAT_FLOAT => Some(f32::MAX as f64),
        VIPS_FORMAT_DOUBLE => Some(f64::MAX),
        _ => None,
    }
}

pub(crate) fn common_format(left: VipsBandFormat, right: VipsBandFormat) -> Option<VipsBandFormat> {
    use crate::abi::image::{
        VIPS_FORMAT_CHAR as C, VIPS_FORMAT_COMPLEX as X, VIPS_FORMAT_DOUBLE as D,
        VIPS_FORMAT_DPCOMPLEX as DX, VIPS_FORMAT_FLOAT as F, VIPS_FORMAT_INT as I,
        VIPS_FORMAT_SHORT as S, VIPS_FORMAT_UCHAR as UC, VIPS_FORMAT_UINT as UI,
        VIPS_FORMAT_USHORT as US,
    };

    if matches!(format_kind(left), Some(NumericKind::Complex))
        || matches!(format_kind(right), Some(NumericKind::Complex))
    {
        return Some(if left == DX || right == DX { DX } else { X });
    }
    if left == D || right == D {
        return Some(D);
    }
    if left == F || right == F {
        return Some(F);
    }

    let index = |format| match format {
        UC => Some(0usize),
        C => Some(1usize),
        US => Some(2usize),
        S => Some(3usize),
        UI => Some(4usize),
        I => Some(5usize),
        _ => None,
    };

    let table = [
        [UC, S, US, S, UI, I],
        [S, C, I, S, I, I],
        [US, I, US, I, UI, I],
        [S, S, I, S, I, I],
        [UI, I, UI, I, UI, I],
        [I, I, I, I, I, I],
    ];

    Some(table[index(left)?][index(right)?])
}

fn trunc_to_i64(value: f64) -> i64 {
    if !value.is_finite() {
        0
    } else {
        value.floor() as i64
    }
}

fn clamp_i64(value: i64, min: i64, max: i64) -> i64 {
    value.clamp(min, max)
}

pub(crate) fn clamp_for_format(value: f64, format: VipsBandFormat) -> f64 {
    match format {
        VIPS_FORMAT_UCHAR => clamp_i64(trunc_to_i64(value), u8::MIN as i64, u8::MAX as i64) as f64,
        VIPS_FORMAT_CHAR => clamp_i64(trunc_to_i64(value), i8::MIN as i64, i8::MAX as i64) as f64,
        VIPS_FORMAT_USHORT => {
            clamp_i64(trunc_to_i64(value), u16::MIN as i64, u16::MAX as i64) as f64
        }
        VIPS_FORMAT_SHORT => {
            clamp_i64(trunc_to_i64(value), i16::MIN as i64, i16::MAX as i64) as f64
        }
        VIPS_FORMAT_UINT => value.floor().clamp(0.0, u32::MAX as f64),
        VIPS_FORMAT_INT => value.floor().clamp(i32::MIN as f64, i32::MAX as f64),
        VIPS_FORMAT_FLOAT => value as f32 as f64,
        VIPS_FORMAT_DOUBLE => value,
        _ => value,
    }
}

pub(crate) fn read_sample(bytes: &[u8], format: VipsBandFormat) -> Option<f64> {
    Some(match format {
        VIPS_FORMAT_UCHAR => *bytes.first()? as f64,
        VIPS_FORMAT_CHAR => i8::from_ne_bytes([*bytes.first()?]) as f64,
        VIPS_FORMAT_USHORT => u16::from_ne_bytes(bytes.get(..2)?.try_into().ok()?) as f64,
        VIPS_FORMAT_SHORT => i16::from_ne_bytes(bytes.get(..2)?.try_into().ok()?) as f64,
        VIPS_FORMAT_UINT => u32::from_ne_bytes(bytes.get(..4)?.try_into().ok()?) as f64,
        VIPS_FORMAT_INT => i32::from_ne_bytes(bytes.get(..4)?.try_into().ok()?) as f64,
        VIPS_FORMAT_FLOAT => f32::from_ne_bytes(bytes.get(..4)?.try_into().ok()?) as f64,
        VIPS_FORMAT_DOUBLE => f64::from_ne_bytes(bytes.get(..8)?.try_into().ok()?),
        _ => return None,
    })
}

pub(crate) fn write_sample(bytes: &mut [u8], format: VipsBandFormat, value: f64) -> bool {
    match format {
        VIPS_FORMAT_UCHAR => bytes
            .get_mut(0)
            .map(|slot| *slot = clamp_for_format(value, format) as u8),
        VIPS_FORMAT_CHAR => bytes
            .get_mut(0)
            .map(|slot| *slot = clamp_for_format(value, format) as i8 as u8),
        VIPS_FORMAT_USHORT => bytes.get_mut(..2).map(|slot| {
            slot.copy_from_slice(&(clamp_for_format(value, format) as u16).to_ne_bytes())
        }),
        VIPS_FORMAT_SHORT => bytes.get_mut(..2).map(|slot| {
            slot.copy_from_slice(&(clamp_for_format(value, format) as i16).to_ne_bytes())
        }),
        VIPS_FORMAT_UINT => bytes.get_mut(..4).map(|slot| {
            slot.copy_from_slice(&(clamp_for_format(value, format) as u32).to_ne_bytes())
        }),
        VIPS_FORMAT_INT => bytes.get_mut(..4).map(|slot| {
            slot.copy_from_slice(&(clamp_for_format(value, format) as i32).to_ne_bytes())
        }),
        VIPS_FORMAT_FLOAT => bytes
            .get_mut(..4)
            .map(|slot| slot.copy_from_slice(&(value as f32).to_ne_bytes())),
        VIPS_FORMAT_COMPLEX => bytes.get_mut(..8).map(|slot| {
            slot[..4].copy_from_slice(&(value as f32).to_ne_bytes());
            slot[4..8].copy_from_slice(&0f32.to_ne_bytes());
        }),
        VIPS_FORMAT_DOUBLE => bytes
            .get_mut(..8)
            .map(|slot| slot.copy_from_slice(&value.to_ne_bytes())),
        VIPS_FORMAT_DPCOMPLEX => bytes.get_mut(..16).map(|slot| {
            slot[..8].copy_from_slice(&value.to_ne_bytes());
            slot[8..16].copy_from_slice(&0f64.to_ne_bytes());
        }),
        _ => None,
    }
    .is_some()
}
