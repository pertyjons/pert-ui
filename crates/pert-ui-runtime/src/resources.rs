//! Scene-local immutable resources: content identity, the deduplicating arena a
//! [`DisplayList`](crate::DisplayList) keeps them in, and color raster images.
//!
//! Pixels are shared through `Arc`, so a resource is never copied between
//! components, scenes or windows. What this module adds is identity: every
//! resource computes a content key once, when it is constructed, and a scene
//! stores equal content once, whichever allocation it came from. A renderer can
//! then keep a GPU copy under that content identity instead of an allocation
//! address. See `docs/rendering-platform.md` for the decisions.
use crate::DrawError;
use std::{collections::HashMap, sync::Arc};

/// Largest accepted raster image, in bytes of RGBA8 pixel data (64 MiB, for
/// example 4096 by 4096 pixels).
const MAX_RASTER_BYTES: u64 = 64 * 1024 * 1024;

// A fast, non-cryptographic 64-bit digest of a resource's dimensions and pixels.
// It only chooses a bucket: equality is always confirmed on the pixels, so a
// collision costs a comparison and never merges different content.
pub(crate) fn content_key(width: u32, height: u32, pixels: &[u8]) -> u64 {
    const K: u64 = 0x517c_c1b7_2722_0a95;
    let step = |hash: u64, word: u64| (hash.rotate_left(5) ^ word).wrapping_mul(K);
    let mut hash = step(step(0, u64::from(width)), u64::from(height));
    let (words, rest) = pixels.as_chunks::<8>();
    for word in words {
        hash = step(hash, u64::from_le_bytes(*word));
    }
    let mut tail = [0u8; 8];
    tail[..rest.len()].copy_from_slice(rest);
    step(step(hash, u64::from_le_bytes(tail)), pixels.len() as u64)
}

// Implemented by the immutable pixel resources a scene can reference.
pub(crate) trait SceneResource: Clone + PartialEq {
    fn content_key(&self) -> u64;
}

// Deduplicating resource table: equal content receives one index. The lookup
// index is derived from the items, so equality and cloning consider the items.
#[derive(Clone, Debug)]
pub(crate) struct Arena<T> {
    items: Vec<T>,
    index: HashMap<u64, Vec<usize>>,
}
impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            index: HashMap::new(),
        }
    }
}
impl<T: PartialEq> PartialEq for Arena<T> {
    fn eq(&self, other: &Self) -> bool {
        self.items == other.items
    }
}
impl<T: SceneResource> Arena<T> {
    // Returns the index of equal content, adding the resource if it is new.
    pub(crate) fn intern(&mut self, item: &T) -> usize {
        let bucket = self.index.entry(item.content_key()).or_default();
        if let Some(&found) = bucket.iter().find(|&&i| self.items[i] == *item) {
            return found;
        }
        let index = self.items.len();
        bucket.push(index);
        self.items.push(item.clone());
        index
    }
    pub(crate) fn items(&self) -> &[T] {
        &self.items
    }
    pub(crate) fn into_items(self) -> Vec<T> {
        self.items
    }
}

/// Opaque index into one owned display list's color images.
///
/// Resolve through [`DisplayList::image`](crate::DisplayList::image). Equal image
/// content drawn several times in one scene shares one ID, whichever allocation
/// each use came from. An ID is a scene identity: never persist it or use it with
/// another display list, where it may resolve to a different image or to nothing.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ImageId(pub(crate) usize);
impl ImageId {
    /// Position of the image in [`DisplayList::images`](crate::DisplayList::images)
    /// of the scene that issued this ID, for an adapter keeping per-resource data
    /// in a parallel table.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

/// How a backend samples an image whose pixels do not map one to one onto the
/// physical pixels it covers.
///
/// Both modes clamp at the image's edges rather than repeating it, so the border
/// pixels extend to the destination's edge instead of blending with the opposite
/// side. Neither promises smooth minification: an image drawn at a small
/// fraction of its pixel size can alias, so prepare a smaller image for a
/// thumbnail rather than relying on the sampler.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ImageSampling {
    /// Bilinear filtering of premultiplied linear-light values. The default,
    /// suited to photographs and to any image drawn at a size other than its own.
    #[default]
    Linear,
    /// The nearest source pixel, with hard edges between pixels. Suited to pixel
    /// art and to images deliberately magnified by whole multiples.
    Nearest,
}

/// Owned, immutable, decoded color image, shared cheaply between scenes.
///
/// Pixels are row-major RGBA8 without padding, in the sRGB color space with the
/// sRGB transfer function, and stored with **premultiplied** alpha: each color
/// channel holds the sRGB encoding of its linear-light value multiplied by alpha,
/// and the alpha byte is linear coverage. Premultiplication happens once, here,
/// because filtered sampling of straight-alpha pixels darkens the edges of
/// transparent regions. Construct from the straight alpha that decoders and
/// most image files provide with [`RasterImage::from_srgba8`].
///
/// Construction admits at most 64 MiB of pixel data and computes a content key
/// once, in time linear in the pixel count, so equal images from different
/// allocations are stored once per scene and uploaded once by a caching backend.
/// Construct images outside `paint`: decoding, conversion and file I/O never
/// belong in a paint callback. Clones share storage, and the type is `Send` and
/// `Sync`, so a worker thread may prepare an image and hand it to the UI thread.
/// This is not a callback-safe endpoint; allocation and final deallocation
/// belong outside time-critical callbacks.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// # use pert_ui::RasterImage;
/// // A 2x1 image: opaque red, then half-transparent white.
/// let image = RasterImage::from_srgba8(2, 1, vec![255, 0, 0, 255, 255, 255, 255, 128])?;
/// assert_eq!((image.width(), image.height()), (2, 1));
/// // Opaque pixels are unchanged; the translucent one is premultiplied.
/// assert_eq!(&image.pixels()[..4], &[255, 0, 0, 255]);
/// assert!(image.pixels()[4] < 255);
/// assert_eq!(image.pixels()[7], 128);
/// # Ok::<(), pert_ui::DrawError>(())
/// ```
#[derive(Clone, Debug)]
pub struct RasterImage {
    width: u32,
    height: u32,
    key: u64,
    pixels: Arc<[u8]>,
}
impl RasterImage {
    /// Largest accepted pixel data in bytes: 64 MiB of RGBA8.
    pub const MAX_BYTES: usize = MAX_RASTER_BYTES as usize;

    /// Creates an image from straight-alpha sRGB-encoded RGBA8 pixels, row-major
    /// without padding, as image decoders produce them, and premultiplies them.
    ///
    /// The conversion happens in place. Opaque pixels are left unchanged and
    /// fully transparent ones become zero, discarding their color; only
    /// partially transparent pixels are converted to linear light, multiplied
    /// and encoded again.
    ///
    /// # Errors
    /// Returns [`DrawError::InvalidImage`], holding the dimensions and byte
    /// count, for a zero dimension, a byte count
    /// other than `width * height * 4`, or more than [`RasterImage::MAX_BYTES`].
    pub fn from_srgba8(width: u32, height: u32, mut pixels: Vec<u8>) -> Result<Self, DrawError> {
        let count = u64::from(width) * u64::from(height);
        if count == 0 || count > MAX_RASTER_BYTES / 4 || count * 4 != pixels.len() as u64 {
            return Err(DrawError::InvalidImage {
                width,
                height,
                bytes: pixels.len(),
            });
        }
        for pixel in pixels.as_chunks_mut::<4>().0 {
            premultiply(pixel);
        }
        Ok(Self {
            width,
            height,
            key: content_key(width, height, &pixels),
            pixels: pixels.into(),
        })
    }
    /// Width in image pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }
    /// Height in image pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }
    /// The image's own size in logical pixels: one logical pixel per image pixel,
    /// which is how [`Image`](crate::Image) sizes itself by default. At a
    /// viewport scale above one the image is therefore magnified; supply an
    /// image with more pixels, drawn into the same logical rectangle, for a
    /// sharper result.
    #[must_use]
    pub fn logical_size(&self) -> crate::Size {
        crate::Size::new(self.width as f32, self.height as f32).unwrap_or_default()
    }
    /// Borrowed premultiplied sRGB-encoded RGBA8 bytes, row-major without padding.
    #[must_use]
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
    /// A 64-bit digest of the dimensions and pixels, computed at construction.
    ///
    /// Equal images always have equal keys, so an adapter can cache an upload
    /// under this key and find it again for equal content from another
    /// allocation, scene or window. Different images can share a key, rarely, so
    /// confirm a cache hit with `==`, which compares the pixels only when the
    /// allocations differ. The value is stable within one process and not across
    /// versions; never persist it.
    #[must_use]
    pub const fn content_key(&self) -> u64 {
        self.key
    }
}
impl PartialEq for RasterImage {
    fn eq(&self, other: &Self) -> bool {
        self.width == other.width
            && self.height == other.height
            && self.key == other.key
            && (Arc::ptr_eq(&self.pixels, &other.pixels) || self.pixels == other.pixels)
    }
}
impl Eq for RasterImage {}
impl std::hash::Hash for RasterImage {
    /// Hashes the [`content key`](Self::content_key), in constant time. Equal
    /// images have equal keys, so the hash agrees with `==`.
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}
impl SceneResource for RasterImage {
    fn content_key(&self) -> u64 {
        self.key
    }
}

fn premultiply(pixel: &mut [u8; 4]) {
    let alpha = pixel[3];
    if alpha == 255 {
        return;
    }
    if alpha == 0 {
        *pixel = [0; 4];
        return;
    }
    let coverage = f32::from(alpha) / 255.0;
    for channel in &mut pixel[..3] {
        *channel = srgb_encode(srgb_decode(*channel) * coverage);
    }
}

/// The linear-light value, `0..=1`, of an sRGB-encoded byte.
pub(crate) fn srgb_decode(byte: u8) -> f32 {
    let encoded = f32::from(byte) / 255.0;
    if encoded <= 0.04045 {
        encoded / 12.92
    } else {
        ((encoded + 0.055) / 1.055).powf(2.4)
    }
}

/// The nearest sRGB-encoded byte of a linear-light value, clamped to `0..=1`.
pub(crate) fn srgb_encode(linear: f32) -> u8 {
    let linear = linear.clamp(0.0, 1.0);
    let encoded = if linear <= 0.003_130_8 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    };
    (encoded * 255.0).round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn premultiplication_happens_in_linear_light() {
        // Straight sRGB 255 at half coverage is linear 0.5, which encodes to 188,
        // not the 128 that premultiplying the encoded byte would give.
        let image = RasterImage::from_srgba8(1, 1, vec![255, 255, 255, 128]).unwrap();
        let expected = (1.055 * (128.0f32 / 255.0).powf(1.0 / 2.4) - 0.055) * 255.0;
        assert!((f32::from(image.pixels()[0]) - expected).abs() <= 1.0);
        assert_eq!(image.pixels()[3], 128);
        let clear = RasterImage::from_srgba8(1, 1, vec![9, 9, 9, 0]).unwrap();
        assert_eq!(clear.pixels(), &[0, 0, 0, 0]);
    }

    #[test]
    fn arena_deduplicates_equal_content_across_allocations() {
        let a = RasterImage::from_srgba8(1, 1, vec![1, 2, 3, 255]).unwrap();
        let b = RasterImage::from_srgba8(1, 1, vec![1, 2, 3, 255]).unwrap();
        let c = RasterImage::from_srgba8(1, 1, vec![3, 2, 1, 255]).unwrap();
        assert!(!Arc::ptr_eq(&a.pixels, &b.pixels));
        let mut arena = Arena::default();
        assert_eq!(arena.intern(&a), 0);
        assert_eq!(arena.intern(&b), 0);
        assert_eq!(arena.intern(&c), 1);
        assert_eq!(arena.items().len(), 2);
    }

    #[test]
    fn a_key_collision_never_merges_different_content() {
        let a = RasterImage::from_srgba8(1, 1, vec![1, 2, 3, 255]).unwrap();
        let mut b = RasterImage::from_srgba8(1, 1, vec![4, 5, 6, 255]).unwrap();
        b.key = a.key;
        let mut arena = Arena::default();
        assert_eq!(arena.intern(&a), 0);
        assert_eq!(arena.intern(&b), 1);
    }

    #[test]
    fn construction_rejects_inconsistent_or_oversized_data() {
        assert!(RasterImage::from_srgba8(0, 1, vec![]).is_err());
        assert!(RasterImage::from_srgba8(1, 1, vec![0; 3]).is_err());
        assert!(RasterImage::from_srgba8(4097, 4096, Vec::new()).is_err());
        assert_eq!(RasterImage::MAX_BYTES, 64 * 1024 * 1024);
    }
}
