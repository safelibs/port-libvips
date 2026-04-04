use crate::abi::image::{VIPS_FORMAT_DOUBLE, VIPS_FORMAT_FLOAT};
use crate::abi::object::VipsObject;

use super::{get_image_buffer, get_image_ref, set_output_image_like};

pub(crate) unsafe fn dispatch(object: *mut VipsObject, nickname: &str) -> Result<bool, ()> {
    match nickname {
        "freqmult" => {
            let input = unsafe { get_image_buffer(object, "in")? };
            let mask = unsafe { get_image_buffer(object, "mask")? };
            if input.spec.width != mask.spec.width || input.spec.height != mask.spec.height {
                return Err(());
            }
            let mask = if mask.spec.bands == input.spec.bands {
                mask
            } else if mask.spec.bands == 1 {
                mask.replicate_bands(input.spec.bands)?
            } else {
                return Err(());
            };
            let mut out = input.with_format(if matches!(input.spec.format, VIPS_FORMAT_DOUBLE) {
                VIPS_FORMAT_DOUBLE
            } else {
                VIPS_FORMAT_FLOAT
            });
            for index in 0..out.data.len() {
                out.data[index] = input.data[index] * mask.data[index];
            }
            let image = unsafe { get_image_ref(object, "in")? };
            let result = unsafe { set_output_image_like(object, "out", out, image) };
            unsafe { crate::runtime::object::object_unref(image) };
            result?;
            Ok(true)
        }
        _ => Ok(false),
    }
}
