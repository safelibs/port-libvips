use crate::abi::basic::{VipsPrecision, VIPS_PRECISION_FLOAT, VIPS_PRECISION_INTEGER};
use crate::abi::image::VIPS_INTERPRETATION_MATRIX;
use crate::pixels::ImageBuffer;
use crate::runtime::header::{vips_image_get_double, vips_image_set_double};

#[derive(Clone, Debug)]
pub(crate) struct Kernel {
    pub width: usize,
    pub height: usize,
    pub data: Vec<f64>,
    pub scale: f64,
    pub offset: f64,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct KernelSample {
    pub dx: isize,
    pub dy: isize,
    pub value: f64,
}

pub(crate) struct KernelIter<'a> {
    kernel: &'a Kernel,
    next: usize,
}

impl Kernel {
    pub(crate) fn new(
        width: usize,
        height: usize,
        data: Vec<f64>,
        scale: f64,
        offset: f64,
    ) -> Self {
        Self {
            width,
            height,
            data,
            scale,
            offset,
        }
    }

    pub(crate) fn at(&self, x: usize, y: usize) -> f64 {
        self.data[y * self.width + x]
    }

    pub(crate) fn scale_or_one(&self) -> f64 {
        if self.scale == 0.0 {
            1.0
        } else {
            self.scale
        }
    }

    pub(crate) fn origin(&self) -> (isize, isize) {
        (self.width as isize / 2, self.height as isize / 2)
    }

    pub(crate) fn iter(&self) -> KernelIter<'_> {
        KernelIter {
            kernel: self,
            next: 0,
        }
    }

    pub(crate) fn from_image(image: *mut crate::abi::image::VipsImage) -> Result<Self, ()> {
        let buffer = ImageBuffer::from_image(image)?;
        if buffer.spec.bands != 1 {
            return Err(());
        }

        let mut scale = 1.0;
        let mut offset = 0.0;
        let _ = vips_image_get_double(image, c"scale".as_ptr(), &mut scale);
        let _ = vips_image_get_double(image, c"offset".as_ptr(), &mut offset);

        Ok(Self::new(
            buffer.spec.width,
            buffer.spec.height,
            buffer.data,
            scale,
            offset,
        ))
    }

    pub(crate) fn to_image(&self) -> *mut crate::abi::image::VipsImage {
        let mut image = ImageBuffer::new(
            self.width,
            self.height,
            1,
            crate::abi::image::VIPS_FORMAT_DOUBLE,
            crate::abi::image::VIPS_CODING_NONE,
            VIPS_INTERPRETATION_MATRIX,
        );
        image.data = self.data.clone();
        let out = image.to_image();
        vips_image_set_double(out, c"scale".as_ptr(), self.scale);
        vips_image_set_double(out, c"offset".as_ptr(), self.offset);
        out
    }

    pub(crate) fn rotate_45(&self) -> Self {
        let mut rotated = vec![0.0; self.data.len()];
        let cx = (self.width as f64 - 1.0) / 2.0;
        let cy = (self.height as f64 - 1.0) / 2.0;
        let angle = std::f64::consts::FRAC_PI_4;
        let sin = angle.sin();
        let cos = angle.cos();

        for y in 0..self.height {
            for x in 0..self.width {
                let dx = x as f64 - cx;
                let dy = y as f64 - cy;
                let src_x = (cos * dx + sin * dy + cx).round() as isize;
                let src_y = (-sin * dx + cos * dy + cy).round() as isize;
                if src_x >= 0
                    && src_y >= 0
                    && (src_x as usize) < self.width
                    && (src_y as usize) < self.height
                {
                    rotated[y * self.width + x] =
                        self.data[src_y as usize * self.width + src_x as usize];
                }
            }
        }

        Self::new(self.width, self.height, rotated, self.scale, self.offset)
    }
}

impl Iterator for KernelIter<'_> {
    type Item = KernelSample;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next >= self.kernel.data.len() {
            return None;
        }

        let index = self.next;
        self.next += 1;

        let x = index % self.kernel.width;
        let y = index / self.kernel.width;
        let (cx, cy) = self.kernel.origin();
        Some(KernelSample {
            dx: x as isize - cx,
            dy: y as isize - cy,
            value: self.kernel.at(x, y),
        })
    }
}

fn gaussian_radius(sigma: f64, min_ampl: f64) -> usize {
    let sig2 = 2.0 * sigma * sigma;
    let max_x = (8.0 * sigma).clamp(0.0, 5000.0) as usize;
    for x in 0..max_x {
        let value = (-((x * x) as f64) / sig2).exp();
        if value < min_ampl {
            return x.saturating_sub(1);
        }
    }
    max_x
}

pub(crate) fn gaussian_kernel(
    sigma: f64,
    min_ampl: f64,
    separable: bool,
    precision: VipsPrecision,
) -> Result<Kernel, ()> {
    if !(sigma.is_finite() && sigma > 0.0 && min_ampl.is_finite() && min_ampl > 0.0) {
        return Err(());
    }

    let radius = gaussian_radius(sigma, min_ampl);
    let width = radius.saturating_mul(2).saturating_add(1);
    let height = if separable { 1 } else { width };
    let sig2 = 2.0 * sigma * sigma;
    let mut data = Vec::with_capacity(width * height);
    let mut sum = 0.0;

    for y in 0..height {
        for x in 0..width {
            let xo = x as isize - width as isize / 2;
            let yo = y as isize - height as isize / 2;
            let distance = (xo * xo + yo * yo) as f64;
            let mut value = (-distance / sig2).exp();
            if precision != VIPS_PRECISION_FLOAT {
                value = (20.0 * value).round();
            }
            data.push(value);
            sum += value;
        }
    }

    if sum == 0.0 {
        sum = 1.0;
    }

    Ok(Kernel::new(width, height, data, sum, 0.0))
}

pub(crate) fn log_kernel(
    sigma: f64,
    min_ampl: f64,
    separable: bool,
    precision: VipsPrecision,
) -> Result<Kernel, ()> {
    if !(sigma.is_finite() && sigma > 0.0 && min_ampl.is_finite() && min_ampl > 0.0) {
        return Err(());
    }

    let sig2 = sigma * sigma;
    let mut last = 0.0;
    let mut radius = None;
    for x in 0..5000usize {
        let distance = (x * x) as f64;
        let value = 0.5 * (2.0 - distance / sig2) * (-distance / (2.0 * sig2)).exp();
        if value - last >= 0.0 && value.abs() < min_ampl {
            radius = Some(x);
            break;
        }
        last = value;
    }
    let radius = radius.ok_or(())?;
    let width = radius.saturating_mul(2).saturating_add(1);
    let height = if separable { 1 } else { width };
    let mut data = Vec::with_capacity(width * height);
    let mut sum = 0.0;

    for y in 0..height {
        for x in 0..width {
            let xo = x as isize - width as isize / 2;
            let yo = y as isize - height as isize / 2;
            let distance = (xo * xo + yo * yo) as f64;
            let mut value = 0.5 * (2.0 - distance / sig2) * (-distance / (2.0 * sig2)).exp();
            if precision == VIPS_PRECISION_INTEGER {
                value = (20.0 * value).round();
            }
            data.push(value);
            sum += value;
        }
    }

    Ok(Kernel::new(width, height, data, sum, 0.0))
}
