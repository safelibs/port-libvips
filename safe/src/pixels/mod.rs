pub(crate) mod format;
pub(crate) mod iter;
pub(crate) mod kernel;

use crate::abi::image::{
    VipsBandFormat, VipsCoding, VipsDemandStyle, VipsImage, VipsInterpretation,
    VIPS_CODING_NONE, VIPS_DEMAND_STYLE_ANY,
};
use crate::pixels::format::{format_bytes, format_components, read_sample, write_sample};
use crate::pixels::iter::{clamped_sample, expanded_sample, pixel_index, PixelIter};
use crate::runtime::header::{copy_metadata, vips_image_init_fields};
use crate::runtime::image::{ensure_pixels, image_state, sync_pixels, vips_image_new_memory};

#[derive(Clone, Copy, Debug)]
pub(crate) struct ImageSpec {
    pub width: usize,
    pub height: usize,
    pub bands: usize,
    pub format: VipsBandFormat,
    pub coding: VipsCoding,
    pub interpretation: VipsInterpretation,
    pub xres: f64,
    pub yres: f64,
    pub xoffset: i32,
    pub yoffset: i32,
    pub dhint: VipsDemandStyle,
}

#[derive(Clone, Debug)]
pub(crate) struct ImageBuffer {
    pub spec: ImageSpec,
    pub data: Vec<f64>,
}

impl ImageBuffer {
    pub(crate) fn new(
        width: usize,
        height: usize,
        bands: usize,
        format: VipsBandFormat,
        coding: VipsCoding,
        interpretation: VipsInterpretation,
    ) -> Self {
        let samples = width.saturating_mul(height).saturating_mul(bands);
        Self {
            spec: ImageSpec {
                width,
                height,
                bands,
                format,
                coding,
                interpretation,
                xres: 1.0,
                yres: 1.0,
                xoffset: 0,
                yoffset: 0,
                dhint: VIPS_DEMAND_STYLE_ANY,
            },
            data: vec![0.0; samples],
        }
    }

    pub(crate) fn from_image(image: *mut VipsImage) -> Result<Self, ()> {
        ensure_pixels(image)?;
        let image_ref = unsafe { image.as_ref() }.ok_or(())?;
        let state = unsafe { image_state(image) }.ok_or(())?;
        let bytes = &state.pixels;
        let sample_size = format_bytes(image_ref.BandFmt);
        let components = format_components(image_ref.BandFmt);
        if components != 1 || sample_size == 0 {
            return Err(());
        }

        let mut data = Vec::with_capacity(
            image_ref.Xsize.max(0) as usize
                * image_ref.Ysize.max(0) as usize
                * image_ref.Bands.max(0) as usize,
        );
        for chunk in bytes.chunks_exact(sample_size) {
            data.push(read_sample(chunk, image_ref.BandFmt).ok_or(())?);
        }

        Ok(Self {
            spec: ImageSpec {
                width: image_ref.Xsize.max(0) as usize,
                height: image_ref.Ysize.max(0) as usize,
                bands: image_ref.Bands.max(0) as usize,
                format: image_ref.BandFmt,
                coding: image_ref.Coding,
                interpretation: image_ref.Type,
                xres: image_ref.Xres,
                yres: image_ref.Yres,
                xoffset: image_ref.Xoffset,
                yoffset: image_ref.Yoffset,
                dhint: image_ref.dhint,
            },
            data,
        })
    }

    pub(crate) fn sample_count(&self) -> usize {
        self.spec
            .width
            .saturating_mul(self.spec.height)
            .saturating_mul(self.spec.bands)
    }

    pub(crate) fn get(&self, x: usize, y: usize, band: usize) -> f64 {
        self.data[pixel_index(self.spec.width, self.spec.bands, x, y, band)]
    }

    pub(crate) fn set(&mut self, x: usize, y: usize, band: usize, value: f64) {
        let index = pixel_index(self.spec.width, self.spec.bands, x, y, band);
        self.data[index] = value;
    }

    pub(crate) fn sample_or_zero(&self, x: usize, y: usize, band: usize) -> f64 {
        expanded_sample(
            self.spec.width,
            self.spec.height,
            self.spec.bands,
            &self.data,
            x,
            y,
            band,
        )
    }

    pub(crate) fn sample_clamped(&self, x: isize, y: isize, band: usize) -> f64 {
        clamped_sample(
            self.spec.width,
            self.spec.height,
            self.spec.bands,
            &self.data,
            x,
            y,
            band,
        )
    }

    pub(crate) fn with_format(&self, format: VipsBandFormat) -> Self {
        let mut out = self.clone();
        out.spec.format = format;
        out
    }

    pub(crate) fn with_shape(&self, width: usize, height: usize, bands: usize) -> Self {
        let mut out = self.clone();
        out.spec.width = width;
        out.spec.height = height;
        out.spec.bands = bands;
        out.data.resize(width.saturating_mul(height).saturating_mul(bands), 0.0);
        out
    }

    pub(crate) fn replicate_bands(&self, bands: usize) -> Result<Self, ()> {
        if self.spec.bands == bands {
            return Ok(self.clone());
        }
        if self.spec.bands != 1 || bands == 0 {
            return Err(());
        }

        let mut out = self.with_shape(self.spec.width, self.spec.height, bands);
        for coord in PixelIter::new(out.spec.width, out.spec.height, out.spec.bands) {
            out.data[coord.index] = self.get(coord.x, coord.y, 0);
        }
        Ok(out)
    }

    pub(crate) fn zero_extend(&self, width: usize, height: usize) -> Self {
        let mut out = self.with_shape(width, height, self.spec.bands);
        for y in 0..self.spec.height.min(height) {
            for x in 0..self.spec.width.min(width) {
                for band in 0..self.spec.bands {
                    out.set(x, y, band, self.get(x, y, band));
                }
            }
        }
        out
    }

    pub(crate) fn to_image(&self) -> *mut VipsImage {
        let out = vips_image_new_memory();
        vips_image_init_fields(
            out,
            self.spec.width as i32,
            self.spec.height as i32,
            self.spec.bands as i32,
            self.spec.format,
            self.spec.coding,
            self.spec.interpretation,
            self.spec.xres,
            self.spec.yres,
        );

        if let Some(image) = unsafe { out.as_mut() } {
            image.Xoffset = self.spec.xoffset;
            image.Yoffset = self.spec.yoffset;
            image.dhint = self.spec.dhint;
            image.hint_set = glib_sys::GTRUE;
        }

        if let Some(state) = unsafe { image_state(out) } {
            let sample_size = format_bytes(self.spec.format);
            state.pixels = vec![0; self.sample_count().saturating_mul(sample_size)];
            for (index, value) in self.data.iter().enumerate() {
                let offset = index * sample_size;
                let _ = write_sample(
                    &mut state.pixels[offset..offset + sample_size],
                    self.spec.format,
                    *value,
                );
            }
        }
        sync_pixels(out);
        out
    }

    pub(crate) fn into_image_like(self, like: *mut VipsImage) -> *mut VipsImage {
        let out = self.to_image();
        copy_metadata(out, like);
        out
    }
}

impl Default for ImageSpec {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            bands: 0,
            format: crate::abi::image::VIPS_FORMAT_UCHAR,
            coding: VIPS_CODING_NONE,
            interpretation: crate::abi::image::VIPS_INTERPRETATION_MULTIBAND,
            xres: 1.0,
            yres: 1.0,
            xoffset: 0,
            yoffset: 0,
            dhint: VIPS_DEMAND_STYLE_ANY,
        }
    }
}
