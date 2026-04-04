#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PixelCoord {
    pub x: usize,
    pub y: usize,
    pub band: usize,
    pub index: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct PixelIter {
    width: usize,
    height: usize,
    bands: usize,
    next: usize,
}

impl PixelIter {
    pub(crate) fn new(width: usize, height: usize, bands: usize) -> Self {
        Self {
            width,
            height,
            bands,
            next: 0,
        }
    }
}

impl Iterator for PixelIter {
    type Item = PixelCoord;

    fn next(&mut self) -> Option<Self::Item> {
        let total = self.width.saturating_mul(self.height).saturating_mul(self.bands);
        if self.next >= total {
            return None;
        }

        let index = self.next;
        self.next += 1;

        let pixel = index / self.bands;
        let band = index % self.bands;
        let y = pixel / self.width;
        let x = pixel % self.width;

        Some(PixelCoord { x, y, band, index })
    }
}

pub(crate) fn pixel_index(width: usize, bands: usize, x: usize, y: usize, band: usize) -> usize {
    ((y * width) + x) * bands + band
}

pub(crate) fn expanded_sample(
    width: usize,
    height: usize,
    bands: usize,
    data: &[f64],
    x: usize,
    y: usize,
    band: usize,
) -> f64 {
    if x >= width || y >= height || band >= bands {
        return 0.0;
    }
    data[pixel_index(width, bands, x, y, band)]
}

pub(crate) fn clamped_sample(
    width: usize,
    height: usize,
    bands: usize,
    data: &[f64],
    x: isize,
    y: isize,
    band: usize,
) -> f64 {
    if width == 0 || height == 0 || band >= bands {
        return 0.0;
    }

    let x = x.clamp(0, width.saturating_sub(1) as isize) as usize;
    let y = y.clamp(0, height.saturating_sub(1) as isize) as usize;
    data[pixel_index(width, bands, x, y, band)]
}
