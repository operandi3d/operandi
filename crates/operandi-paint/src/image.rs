//! A paintable image: width × height pixels of one [`Format`] at one
//! [`Depth`] (PNT-01..05).
//!
//! Pixels are stored at the image's own depth (8-bit, 16-bit or 32-bit
//! float), so saving writes back what was loaded. Every operation reads
//! and writes through normalised `[f32; 4]` RGBA values: grey reads as
//! three equal channels, an image without alpha reads alpha 1, and writes
//! to whole-number depths clamp to 0..1.

/// Smallest and largest side New Image and Set Resolution accept (PNT-02).
pub const MIN_SIZE: u32 = 64;
pub const MAX_SIZE: u32 = 8192;
/// New Image's starting resolution (PNT-02).
pub const DEFAULT_SIZE: u32 = 2048;
/// The one-time note for images painting handles less well (PNT-05).
pub const SQUARE_POW2_NOTE: &str = "Painting works best on square and power of 2 images.";

/// Which channels an image has (PNT-03).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Format {
    Gray,
    Rgb,
    #[default]
    Rgba,
}

impl Format {
    pub const ALL: [Format; 3] = [Format::Gray, Format::Rgb, Format::Rgba];
    pub fn label(self) -> &'static str {
        match self {
            Format::Gray => "Gray",
            Format::Rgb => "RGB",
            Format::Rgba => "RGBA",
        }
    }
    /// The format a typed word names (`gray`, `rgb`, `rgba`, any case).
    pub fn named(word: &str) -> Option<Format> {
        Format::ALL
            .into_iter()
            .find(|f| f.label().eq_ignore_ascii_case(word.trim()))
            .or_else(|| {
                word.trim()
                    .eq_ignore_ascii_case("grey")
                    .then_some(Format::Gray)
            })
    }
    pub fn channels(self) -> usize {
        match self {
            Format::Gray => 1,
            Format::Rgb => 3,
            Format::Rgba => 4,
        }
    }
    pub fn has_alpha(self) -> bool {
        self == Format::Rgba
    }
}

/// How each channel is stored.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Depth {
    #[default]
    U8,
    U16,
    /// Floating Point (PNT-04): 32-bit floats, saved as OpenEXR by default.
    F32,
}

impl Depth {
    pub fn bits(self) -> u32 {
        match self {
            Depth::U8 => 8,
            Depth::U16 => 16,
            Depth::F32 => 32,
        }
    }
    pub fn is_float(self) -> bool {
        self == Depth::F32
    }
}

/// The stored samples, row by row from the top, channels interleaved.
#[derive(Clone, Debug, PartialEq)]
pub enum Pixels {
    U8(Vec<u8>),
    U16(Vec<u16>),
    F32(Vec<f32>),
}

impl Pixels {
    fn len(&self) -> usize {
        match self {
            Pixels::U8(v) => v.len(),
            Pixels::U16(v) => v.len(),
            Pixels::F32(v) => v.len(),
        }
    }
    fn get(&self, i: usize) -> f32 {
        match self {
            Pixels::U8(v) => f32::from(v[i]) / 255.,
            Pixels::U16(v) => f32::from(v[i]) / 65535.,
            Pixels::F32(v) => v[i],
        }
    }
    fn set(&mut self, i: usize, value: f32) {
        match self {
            Pixels::U8(v) => v[i] = (value.clamp(0., 1.) * 255.).round() as u8,
            Pixels::U16(v) => v[i] = (value.clamp(0., 1.) * 65535.).round() as u16,
            Pixels::F32(v) => v[i] = value,
        }
    }
}

/// An image's size and storage, without its pixels: what New Image asks
/// for and what the Images list shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Layout {
    pub width: u32,
    pub height: u32,
    pub format: Format,
    pub depth: Depth,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PaintImage {
    pub width: u32,
    pub height: u32,
    pub format: Format,
    pub depth: Depth,
    pub pixels: Pixels,
}

/// Why an image can't be made or changed.
pub type Result<T> = std::result::Result<T, String>;

/// Checks a side against New Image's range (PNT-02).
pub fn check_side(side: u32) -> Result<()> {
    if (MIN_SIZE..=MAX_SIZE).contains(&side) {
        Ok(())
    } else {
        Err(format!(
            "Resolution must be {MIN_SIZE} to {MAX_SIZE} pixels (got {side})"
        ))
    }
}

/// Rec. 709 luma, for writing a colour into a grey image.
pub fn luma(c: [f32; 3]) -> f32 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

impl PaintImage {
    /// All samples 0: colour 0, alpha 0 (PNT-06, Set Color off).
    pub fn new(layout: Layout) -> Result<PaintImage> {
        let n = (layout.width as usize)
            .checked_mul(layout.height as usize)
            .and_then(|p| p.checked_mul(layout.format.channels()))
            .ok_or("The image is too large")?;
        let pixels = match layout.depth {
            Depth::U8 => Pixels::U8(vec![0; n]),
            Depth::U16 => Pixels::U16(vec![0; n]),
            Depth::F32 => Pixels::F32(vec![0.; n]),
        };
        Ok(PaintImage {
            width: layout.width,
            height: layout.height,
            format: layout.format,
            depth: layout.depth,
            pixels,
        })
    }

    /// New Image (PNT-01..07): blank, or every pixel `color` with full
    /// alpha when Set Color is on.
    pub fn filled(layout: Layout, color: Option<[f32; 3]>) -> Result<PaintImage> {
        let mut image = PaintImage::new(layout)?;
        if let Some(c) = color {
            image.fill([c[0], c[1], c[2], 1.]);
        }
        Ok(image)
    }

    pub fn layout(&self) -> Layout {
        Layout {
            width: self.width,
            height: self.height,
            format: self.format,
            depth: self.depth,
        }
    }

    pub fn pixel_count(&self) -> usize {
        self.width as usize * self.height as usize
    }

    /// 0 × 0 images can't be painted (PNT-15).
    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// Square with power-of-two sides (PNT-05).
    pub fn is_square_pow2(&self) -> bool {
        is_square_pow2(self.width, self.height)
    }

    /// Pixel `i` (row-major from the top) as normalised RGBA.
    pub fn get(&self, i: usize) -> [f32; 4] {
        let c = self.format.channels();
        let at = i * c;
        match self.format {
            Format::Gray => {
                let v = self.pixels.get(at);
                [v, v, v, 1.]
            }
            Format::Rgb => [
                self.pixels.get(at),
                self.pixels.get(at + 1),
                self.pixels.get(at + 2),
                1.,
            ],
            Format::Rgba => [
                self.pixels.get(at),
                self.pixels.get(at + 1),
                self.pixels.get(at + 2),
                self.pixels.get(at + 3),
            ],
        }
    }

    /// Sets pixel `i`; a grey image takes the colour's luma, an image
    /// without alpha ignores it.
    pub fn set(&mut self, i: usize, rgba: [f32; 4]) {
        let c = self.format.channels();
        let at = i * c;
        match self.format {
            Format::Gray => self.pixels.set(at, luma([rgba[0], rgba[1], rgba[2]])),
            Format::Rgb | Format::Rgba => {
                for (k, v) in rgba.iter().take(c).enumerate() {
                    self.pixels.set(at + k, *v);
                }
            }
        }
    }

    /// Copies pixel `from` onto pixel `to` exactly (no conversion).
    pub fn copy_pixel(&mut self, from: usize, to: usize) {
        let c = self.format.channels();
        let (from, to) = (from * c, to * c);
        match &mut self.pixels {
            Pixels::U8(v) => v.copy_within(from..from + c, to),
            Pixels::U16(v) => v.copy_within(from..from + c, to),
            Pixels::F32(v) => v.copy_within(from..from + c, to),
        }
    }

    pub fn fill(&mut self, rgba: [f32; 4]) {
        for i in 0..self.pixel_count() {
            self.set(i, rgba);
        }
    }

    /// Clear Image (PNT-24): every sample 0, as a new blank image.
    pub fn clear(&mut self) {
        match &mut self.pixels {
            Pixels::U8(v) => v.fill(0),
            Pixels::U16(v) => v.fill(0),
            Pixels::F32(v) => v.fill(0.),
        }
    }

    /// Invert Image (PNT-23): colour becomes 1 − colour, alpha is kept.
    /// Float values clamp to 0..1 first.
    pub fn invert(&mut self) {
        let c = self.format.channels();
        let colour = if self.format.has_alpha() { 3 } else { c };
        for i in 0..self.pixels.len() {
            if i % c < colour {
                let v = self.pixels.get(i).clamp(0., 1.);
                self.pixels.set(i, 1. - v);
            }
        }
    }

    /// Bilinear sample at pixel coordinates (centres at +0.5), edges clamped.
    pub fn sample(&self, x: f32, y: f32) -> [f32; 4] {
        let (w, h) = (self.width as i64, self.height as i64);
        let fx = (x - 0.5).max(0.);
        let fy = (y - 0.5).max(0.);
        let x0 = (fx.floor() as i64).min(w - 1);
        let y0 = (fy.floor() as i64).min(h - 1);
        let x1 = (x0 + 1).min(w - 1);
        let y1 = (y0 + 1).min(h - 1);
        let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
        let at = |x: i64, y: i64| self.get((y * w + x) as usize);
        let (a, b, c, d) = (at(x0, y0), at(x1, y0), at(x0, y1), at(x1, y1));
        std::array::from_fn(|k| {
            let top = a[k] + (b[k] - a[k]) * tx;
            let bottom = c[k] + (d[k] - c[k]) * tx;
            top + (bottom - top) * ty
        })
    }

    /// Set Resolution (PNT-25): the same picture at a new size, filtered
    /// bilinearly, keeping format and depth.
    pub fn resized(&self, width: u32, height: u32) -> Result<PaintImage> {
        check_side(width)?;
        check_side(height)?;
        if self.is_empty() {
            return Err("This image is empty (0 × 0).".into());
        }
        let mut out = PaintImage::new(Layout {
            width,
            height,
            ..self.layout()
        })?;
        let (sx, sy) = (
            self.width as f32 / width as f32,
            self.height as f32 / height as f32,
        );
        for y in 0..height {
            for x in 0..width {
                let v = self.sample((x as f32 + 0.5) * sx, (y as f32 + 0.5) * sy);
                out.set((y * width + x) as usize, v);
            }
        }
        Ok(out)
    }
}

pub fn is_square_pow2(width: u32, height: u32) -> bool {
    width == height && width.is_power_of_two()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(format: Format, depth: Depth) -> Layout {
        Layout {
            width: 64,
            height: 64,
            format,
            depth,
        }
    }

    #[test]
    fn new_image_is_zero_colour_zero_alpha() {
        let image = PaintImage::filled(layout(Format::Rgba, Depth::U8), None).unwrap();
        assert!((0..image.pixel_count()).all(|i| image.get(i) == [0.; 4]));
    }

    #[test]
    fn set_color_fills_with_full_alpha() {
        let image =
            PaintImage::filled(layout(Format::Rgba, Depth::F32), Some([0.25, 0.5, 2.])).unwrap();
        assert_eq!(image.get(17), [0.25, 0.5, 2., 1.]);
        let grey = PaintImage::filled(layout(Format::Gray, Depth::U8), Some([1., 1., 1.])).unwrap();
        assert_eq!(grey.get(0), [1., 1., 1., 1.]);
    }

    #[test]
    fn invert_keeps_alpha_and_clamps_floats() {
        let mut image = PaintImage::new(layout(Format::Rgba, Depth::F32)).unwrap();
        image.set(0, [2., -1., 0.25, 0.5]);
        image.invert();
        assert_eq!(image.get(0), [0., 1., 0.75, 0.5]);
        let mut eight = PaintImage::new(layout(Format::Rgb, Depth::U8)).unwrap();
        eight.invert();
        assert_eq!(eight.get(3), [1., 1., 1., 1.]);
    }

    #[test]
    fn resize_is_bilinear_and_checks_range() {
        let mut image = PaintImage::new(Layout {
            width: 64,
            height: 64,
            format: Format::Gray,
            depth: Depth::F32,
        })
        .unwrap();
        for y in 0..64 {
            for x in 0..64 {
                image.set(y * 64 + x, [x as f32 / 63.; 4]);
            }
        }
        let big = image.resized(128, 128).unwrap();
        assert_eq!(big.layout().format, Format::Gray);
        // A ramp stays a ramp, edges clamped.
        assert!((big.get(0)[0] - 0.).abs() < 1e-6);
        assert!((big.get(127)[0] - 1.).abs() < 1e-6);
        assert!((big.get(64)[0] - 0.5).abs() < 0.02);
        assert!(image.resized(32, 32).is_err());
        assert!(image.resized(8193, 64).is_err());
    }

    #[test]
    fn square_pow2() {
        assert!(is_square_pow2(2048, 2048));
        assert!(!is_square_pow2(2048, 1024));
        assert!(!is_square_pow2(1000, 1000));
    }
}
