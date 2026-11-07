//! Pixel parity between two RGBA8 frames of the same size, and PNG output.
use std::path::Path;

/// Differences over the R, G and B channels (every workload is opaque, so
/// alpha is 255 everywhere and premultiplied equals straight color).
#[derive(Clone, Copy, Debug, Default)]
pub struct Diff {
    pub max: u8,
    pub mean: f64,
    /// Share of pixels with any channel differing by more than 2/255.
    pub over2: f64,
    pub pixels: usize,
}

/// Per-region parity: all pixels, "interior" pixels whose 3×3 neighborhood
/// in the reference is one uniform color (flat fills, away from any edge),
/// and "edge" pixels (the rest: antialiased edges, the line, gradient
/// texels that change from row to row).
#[derive(Clone, Copy, Debug, Default)]
pub struct Parity {
    pub all: Diff,
    pub interior: Diff,
    pub edge: Diff,
    /// Uniform pixels a translucent command covers (only when classified;
    /// they are then left out of `interior`).
    pub translucent: Diff,
}

#[derive(Default)]
struct Acc {
    max: u8,
    sum: u64,
    over: usize,
    n: usize,
}
impl Acc {
    fn add(&mut self, a: &[u8], b: &[u8]) {
        let mut over = false;
        for c in 0..3 {
            let d = a[c].abs_diff(b[c]);
            self.max = self.max.max(d);
            self.sum += u64::from(d);
            over |= d > 2;
        }
        self.over += usize::from(over);
        self.n += 1;
    }
    fn done(&self) -> Diff {
        Diff {
            max: self.max,
            mean: if self.n == 0 { 0.0 } else { self.sum as f64 / (self.n * 3) as f64 },
            over2: if self.n == 0 { 0.0 } else { self.over as f64 / self.n as f64 },
            pixels: self.n,
        }
    }
}

pub fn parity(reference: &[u8], other: &[u8], width: usize, height: usize) -> Parity {
    classified(reference, other, width, height, None)
}

/// As [`parity`], with uniform pixels under a translucent command (`true` in
/// `translucent`) counted separately from opaque interiors.
pub fn classified(
    reference: &[u8],
    other: &[u8],
    width: usize,
    height: usize,
    translucent: Option<&[bool]>,
) -> Parity {
    assert_eq!(reference.len(), other.len());
    let px = |x: usize, y: usize| &reference[(y * width + x) * 4..][..4];
    let (mut all, mut interior, mut edge, mut glass) =
        (Acc::default(), Acc::default(), Acc::default(), Acc::default());
    for y in 0..height {
        for x in 0..width {
            let i = (y * width + x) * 4;
            let (a, b) = (&reference[i..i + 4], &other[i..i + 4]);
            all.add(a, b);
            let uniform = x > 0
                && y > 0
                && x + 1 < width
                && y + 1 < height
                && (y - 1..=y + 1).all(|yy| (x - 1..=x + 1).all(|xx| px(xx, yy) == a));
            if !uniform {
                edge.add(a, b)
            } else if translucent.is_some_and(|t| t[y * width + x]) {
                glass.add(a, b)
            } else {
                interior.add(a, b)
            }
        }
    }
    Parity { all: all.done(), interior: interior.done(), edge: edge.done(), translucent: glass.done() }
}

/// Writes RGBA8 pixels as a PNG.
pub fn write_png(path: &Path, rgba: &[u8], width: u32, height: u32) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::High);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(rgba)?;
    Ok(())
}

/// A `w`×`h` crop at (`x`, `y`), each pixel repeated `zoom` times per axis.
pub fn crop(rgba: &[u8], width: usize, x: usize, y: usize, w: usize, h: usize, zoom: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(w * h * zoom * zoom * 4);
    for yy in 0..h * zoom {
        for xx in 0..w * zoom {
            let i = ((y + yy / zoom) * width + x + xx / zoom) * 4;
            out.extend_from_slice(&rgba[i..i + 4]);
        }
    }
    out
}

fn to_linear(c: u8) -> f64 {
    let c = f64::from(c) / 255.0;
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

fn to_srgb8(l: f64) -> u8 {
    let c = if l <= 0.0031308 { l * 12.92 } else { 1.055 * l.powf(1.0 / 2.4) - 0.055 };
    (c * 255.0).round().clamp(0.0, 255.0) as u8
}

/// Opaque RGBA8 of `foreground` over `background` with per-pixel coverage
/// `alpha`, blended in linear light and encoded to sRGB8 once.
pub fn composite_linear(alpha: &[u8], background: [u8; 4], foreground: [u8; 4]) -> Vec<u8> {
    let bg: [f64; 3] = std::array::from_fn(|c| to_linear(background[c]));
    let fg: [f64; 3] = std::array::from_fn(|c| to_linear(foreground[c]));
    let mut out = Vec::with_capacity(alpha.len() * 4);
    for &a in alpha {
        let a = f64::from(a) / 255.0;
        for c in 0..3 {
            out.push(to_srgb8(fg[c] * a + bg[c] * (1.0 - a)));
        }
        out.push(255);
    }
    out
}

/// The pixel with the largest channel difference.
pub fn worst(a: &[u8], b: &[u8], width: usize) -> (usize, usize) {
    let mut best = (0, 0);
    for (i, (p, q)) in a.chunks(4).zip(b.chunks(4)).enumerate() {
        let d = (0..3).map(|c| p[c].abs_diff(q[c])).max().unwrap_or(0);
        if d > best.0 {
            best = (d, i);
        }
    }
    (best.1 % width, best.1 / width)
}
