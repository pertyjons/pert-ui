//! Screenshots of a window, a view or a widget, from the application's own
//! rendered output.
//!
//! A [`CaptureRequest`] names its target with the same identities inspection
//! uses and is answered with a [`Capture`]: an owned image of the target's
//! visible rendered region at the window's physical scale, with the logical
//! region, clipping and frame it describes. The runtime resolves the target
//! and crops; the host renders the frame, because only its renderer has the
//! pixels. A host declares that it can with
//! [`Runtime::set_capture_support`](crate::Runtime::set_capture_support) and
//! completes each [`CaptureJob`] it is handed; the default native `App` does.
//! Without a host that renders, every request is answered
//! [`CaptureError::Unsupported`], never left waiting.
//! [`docs/contracts.md`] records the contract.
//!
//! [`docs/contracts.md`]: https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md

use crate::resources::{srgb_decode, srgb_encode};
use std::time::Duration;

/// What a capture is of.
///
/// Targets use inspection's identities, so a tool captures what it just
/// queried: a window by [`WindowInspection::id`](crate::WindowInspection::id),
/// a component by [`ComponentInspection::id`](crate::ComponentInspection::id)
/// or by its addressable key path. An incarnation never retargets: a
/// replacement mounted under the same key is a different component and the
/// request for the old one is answered [`CaptureError::UnknownTarget`].
/// Whole-display capture is a separate operating-system service that this
/// library does not provide; a whole window is [`Self::Window`].
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(deny_unknown_fields)
)]
#[non_exhaustive]
pub enum CaptureTarget {
    /// One native presentation root, whole.
    Window(u64),
    /// A rectangle of one native presentation root, in its logical window
    /// coordinates, such as part of a view. The part inside the window is
    /// captured, and [`Capture::clipped`] reports whether the request reached
    /// beyond it. With the `serde` feature its numbers are encoded like the
    /// other diagnostic floats, so a non-finite one survives a round trip and
    /// is then refused as [`CaptureError::InvalidRegion`].
    Region {
        /// The window, [`WindowInspection::id`](crate::WindowInspection::id).
        window: u64,
        /// Left edge in logical pixels.
        #[cfg_attr(feature = "serde", serde(with = "crate::serde_float::f32"))]
        x: f32,
        /// Top edge in logical pixels.
        #[cfg_attr(feature = "serde", serde(with = "crate::serde_float::f32"))]
        y: f32,
        /// Width in logical pixels.
        #[cfg_attr(feature = "serde", serde(with = "crate::serde_float::f32"))]
        width: f32,
        /// Height in logical pixels.
        #[cfg_attr(feature = "serde", serde(with = "crate::serde_float::f32"))]
        height: f32,
    },
    /// One mounted component's visible rendered region.
    Component(u64),
    /// The component at this addressable key path, as
    /// [`Selector::path`](crate::Selector::path) finds it.
    Path(Vec<String>),
}

impl CaptureTarget {
    /// The whole of `window`, by the identity inspection reports for it,
    /// [`NativeWindowId::id`](crate::NativeWindowId::id); for example the main
    /// window, [`Runtime::primary_window`](crate::Runtime::primary_window).
    ///
    /// The request keeps the portable number, not the UI-local handle, so it
    /// can still travel through an
    /// [`InspectionClient`](crate::InspectionClient) and serialize. A window
    /// that has closed by the time the request is served is answered
    /// [`CaptureError::UnknownTarget`]. The number identifies a window of one
    /// runtime only: a request built from another runtime's window can name an
    /// unrelated window of this one.
    ///
    /// ```
    /// use pert_ui_runtime::{CaptureTarget, Runtime};
    /// let ui = Runtime::new();
    /// let main = ui.primary_window();
    /// assert_eq!(CaptureTarget::window(&main), CaptureTarget::Window(main.id()));
    /// ```
    #[must_use]
    pub fn window(window: &crate::NativeWindowId) -> Self {
        Self::Window(window.id())
    }
    /// The rectangle `area` of `window`, in its logical window coordinates, as
    /// [`Self::Region`]; identity as for [`Self::window`].
    #[must_use]
    pub fn region(window: &crate::NativeWindowId, area: crate::Rect) -> Self {
        Self::Region {
            window: window.id(),
            x: area.x(),
            y: area.y(),
            width: area.width(),
            height: area.height(),
        }
    }
}

/// A request for a screenshot.
///
/// ```
/// use pert_ui_runtime::{CaptureRequest, CaptureTarget};
/// let request = CaptureRequest::new(CaptureTarget::Path(vec!["mixer".into()])).include_overlay(true);
/// assert!(request.overlay_included());
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(deny_unknown_fields)
)]
#[must_use]
pub struct CaptureRequest {
    target: CaptureTarget,
    include_overlay: bool,
}

impl CaptureRequest {
    /// A request for `target` without development overlays.
    pub const fn new(target: CaptureTarget) -> Self {
        Self {
            target,
            include_overlay: false,
        }
    }
    /// Whether the active [`DebugOverlay`](crate::DebugOverlay), if any, is
    /// drawn into the image. Off by default: a screenshot shows what the
    /// user sees without diagnostic marks, whatever the overlay setting.
    pub const fn include_overlay(mut self, include: bool) -> Self {
        self.include_overlay = include;
        self
    }
    /// The target.
    #[must_use]
    pub const fn target(&self) -> &CaptureTarget {
        &self.target
    }
    /// Whether overlays are drawn.
    #[must_use]
    pub const fn overlay_included(&self) -> bool {
        self.include_overlay
    }
}

/// Why a capture was not produced. Each is a distinct outcome; a request is
/// never answered with a different target or a stale image.
///
/// With the `serde` feature, portable outcomes and backend diagnostics retain
/// their serialized representation. [`Self::Runtime`] and [`Self::Drawing`]
/// contain owned Rust causes, including application errors, and deliberately
/// reject serialization and deserialization. Applications define their own
/// transport DTO when they need to send these causes over a protocol.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[non_exhaustive]
pub enum CaptureError {
    /// No host has declared that it renders captures; a headless runtime, or
    /// a custom host that has not called `Runtime::set_capture_support`.
    #[error("this host does not render captures")]
    Unsupported,
    /// The window, component or path does not name a mounted target.
    #[error("the capture target is not mounted")]
    UnknownTarget,
    /// A [`CaptureTarget::Region`] is not a finite rectangle with a
    /// nonnegative size.
    #[error("the capture region is not a valid rectangle")]
    InvalidRegion,
    /// The target is not presented: hidden, in a window that is not
    /// presentable, or clipped away entirely.
    #[error("the capture target is not visible")]
    Hidden,
    /// Layout has not produced the target's bounds yet.
    #[error("the capture target has not been measured")]
    Unmeasured,
    /// The window's surface was not available to render: occluded, lost or
    /// not yet created. Ask again once it presents.
    #[error("the window's surface is unavailable")]
    SurfaceUnavailable,
    /// The image would exceed [`FrameImage::MAX_BYTES`].
    #[error("the capture exceeds the image size limit")]
    TooLarge,
    /// [`Runtime::MAX_PENDING_CAPTURES`](crate::Runtime::MAX_PENDING_CAPTURES)
    /// captures are already waiting for the host; wait for one, then ask
    /// again.
    #[error("too many captures are pending")]
    Busy,
    /// A runtime operation failed. The owned cause preserves application errors
    /// and can be matched without parsing diagnostic text.
    #[cfg_attr(feature = "serde", serde(skip))]
    #[error(transparent)]
    Runtime(#[from] crate::RuntimeError),
    /// Drawing the scene failed. Conversion from [`crate::DrawError`] moves
    /// its runtime causes into [`Self::Runtime`]; other drawing failures stay here.
    /// Custom hosts should use `From` or `?` for this normalization. The library
    /// never produces `Drawing(DrawError::Runtime(_))`.
    #[cfg_attr(feature = "serde", serde(skip))]
    #[error(transparent)]
    Drawing(crate::DrawError),
    /// A frame the host supplied does not fit what was asked of it: a
    /// renderer's mistake that the library detects. The cause keeps the
    /// dimensions involved.
    #[error(transparent)]
    Frame(FrameError),
    /// Backend-specific diagnostics from rendering.
    #[error("capture rendering failed: {0}")]
    Backend(String),
}

/// Why the pixels a host supplied cannot form the requested image.
///
/// Each variant names the dimensions involved, in physical pixels, so a host
/// can tell its own read-back mistake apart without parsing text.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[non_exhaustive]
pub enum FrameError {
    /// The byte count is not `width * height * 4`, or a dimension is zero.
    #[error("{width}×{height} pixels need {expected} bytes, {actual} given")]
    PixelCount {
        /// Declared width.
        width: u32,
        /// Declared height.
        height: u32,
        /// The bytes those dimensions need.
        expected: u64,
        /// The bytes supplied.
        actual: u64,
    },
    /// A crop region reaches outside the frame, or its far edge overflows.
    #[error("crop {x},{y} {width}×{height} leaves the {frame_width}×{frame_height} frame")]
    CropOutside {
        /// Region left edge.
        x: u32,
        /// Region top edge.
        y: u32,
        /// Region width.
        width: u32,
        /// Region height.
        height: u32,
        /// Frame width.
        frame_width: u32,
        /// Frame height.
        frame_height: u32,
    },
    /// A completed frame does not have the window's physical extent, so its
    /// crop would show other pixels than the region claims.
    #[error(
        "the frame is {frame_width}×{frame_height} but the window is {window_width}×{window_height}"
    )]
    ExtentMismatch {
        /// Supplied frame width.
        frame_width: u32,
        /// Supplied frame height.
        frame_height: u32,
        /// The window's physical width.
        window_width: u32,
        /// The window's physical height.
        window_height: u32,
    },
}

impl From<CaptureError> for crate::RuntimeError {
    /// Carries a capture failure out of an application callback with `?`, as
    /// [`RuntimeError::Application`](crate::RuntimeError::Application) holding
    /// the whole error, downcastable through
    /// [`ApplicationError::downcast_ref`](crate::ApplicationError::downcast_ref).
    /// [`CaptureError::Runtime`] becomes the runtime error it holds, so a runtime
    /// failure keeps one form. This is the error policy's callback-boundary
    /// exception; see [`RuntimeError`](crate::RuntimeError).
    fn from(error: CaptureError) -> Self {
        match error {
            CaptureError::Runtime(error) => error,
            error => Self::Application(crate::ApplicationError::new(error)),
        }
    }
}
impl From<FrameError> for CaptureError {
    fn from(error: FrameError) -> Self {
        Self::Frame(error)
    }
}

impl From<crate::DrawError> for CaptureError {
    fn from(error: crate::DrawError) -> Self {
        match error {
            crate::DrawError::Runtime(error) => Self::Runtime(error),
            error => Self::Drawing(error),
        }
    }
}

/// Pixels of one rendered frame or a region of one: 8-bit sRGB RGBA, rows
/// top to bottom, with scene alpha preserved independently of an opaque native
/// presentation surface. RGB is premultiplied in linear light before sRGB
/// encoding.
/// The default palettes paint an opaque background, so every pixel is then
/// opaque and the distinction does not arise; a palette with a translucent
/// background yields premultiplied color, not straight.
///
/// Built by a host from its renderer's read-back and cropped by the runtime.
/// [`FrameImage::encode_png`] encodes it without any dependency, converting to
/// the straight alpha PNG stores.
///
/// # Serialization
///
/// With the `serde` feature an image is `{ width, height, rgba }`, `rgba`
/// being the list of bytes [`FrameImage::pixels`] returns. Reading one rejects
/// an unknown field and validates as [`FrameImage::new`] does: a zero
/// dimension, a byte count other than `width * height * 4` and an image
/// beyond [`FrameImage::MAX_BYTES`] are errors.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(try_from = "FrameImageRepr", into = "FrameImageRepr")
)]
pub struct FrameImage {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

#[cfg(feature = "serde")]
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct FrameImageRepr {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

#[cfg(feature = "serde")]
impl TryFrom<FrameImageRepr> for FrameImage {
    type Error = CaptureError;
    fn try_from(repr: FrameImageRepr) -> Result<Self, Self::Error> {
        Self::new(repr.width, repr.height, repr.rgba)
    }
}

#[cfg(feature = "serde")]
impl From<FrameImage> for FrameImageRepr {
    fn from(image: FrameImage) -> Self {
        Self {
            width: image.width,
            height: image.height,
            rgba: image.rgba,
        }
    }
}

impl FrameImage {
    /// The most bytes an image holds, 64 MiB: a 4096×4096 frame.
    pub const MAX_BYTES: usize = 64 << 20;

    /// Wraps read-back pixels: `width * height * 4` bytes, nonzero dimensions.
    ///
    /// # Errors
    /// [`CaptureError::TooLarge`] beyond [`Self::MAX_BYTES`] and
    /// [`CaptureError::Frame`] with [`FrameError::PixelCount`] for a zero
    /// dimension or a byte count that does not match, which is a renderer's
    /// mistake rather than a size.
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Result<Self, CaptureError> {
        let bytes = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(CaptureError::TooLarge)?;
        if bytes > Self::MAX_BYTES as u64 {
            return Err(CaptureError::TooLarge);
        }
        if bytes == 0 || bytes != rgba.len() as u64 {
            return Err(FrameError::PixelCount {
                width,
                height,
                expected: bytes,
                actual: rgba.len() as u64,
            }
            .into());
        }
        Ok(Self {
            width,
            height,
            rgba,
        })
    }
    /// Width in physical pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }
    /// Height in physical pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }
    /// Borrowed premultiplied sRGB-encoded RGBA8 bytes, row-major from the top
    /// without padding: `width * height * 4` bytes, like
    /// [`RasterImage::pixels`](crate::RasterImage::pixels).
    #[must_use]
    pub fn pixels(&self) -> &[u8] {
        &self.rgba
    }
    /// The `x, y, width, height` region in physical pixels, which must lie
    /// inside the image and be nonempty.
    ///
    /// # Errors
    /// [`CaptureError::Hidden`] for an empty region and
    /// [`CaptureError::Frame`] with [`FrameError::CropOutside`] for one outside
    /// the image, including one whose far edge overflows `u32`.
    pub fn crop(&self, x: u32, y: u32, width: u32, height: u32) -> Result<Self, CaptureError> {
        if width == 0 || height == 0 {
            return Err(CaptureError::Hidden);
        }
        let outside = || FrameError::CropOutside {
            x,
            y,
            width,
            height,
            frame_width: self.width,
            frame_height: self.height,
        };
        let (Some(right), Some(bottom)) = (x.checked_add(width), y.checked_add(height)) else {
            return Err(outside().into());
        };
        if right > self.width || bottom > self.height {
            return Err(outside().into());
        }
        let stride = self.width as usize * 4;
        let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
        for row in y..bottom {
            let start = row as usize * stride + x as usize * 4;
            rgba.extend_from_slice(&self.rgba[start..start + width as usize * 4]);
        }
        Self::new(width, height, rgba)
    }
    /// Encodes the image as a PNG: 8-bit RGBA, one IDAT chunk of stored
    /// (uncompressed) deflate blocks, so the encoder needs no dependency and
    /// the output is deterministic. The MIME type is `image/png`. An
    /// application that wants smaller files re-encodes with its own codec.
    ///
    /// PNG stores straight alpha, so translucent pixels are un-premultiplied
    /// on the way out, in linear light as [`RasterImage`](crate::RasterImage)
    /// premultiplies them: each color channel is decoded, divided by the
    /// alpha's coverage, clamped to full intensity and encoded to the nearest
    /// sRGB byte. A fully transparent pixel is written as all zeros, and
    /// opaque pixels are written unchanged. A translucent image is converted
    /// into a copy first, so encoding it briefly holds its pixels twice.
    /// Low alpha leaves little color precision, as premultiplied storage
    /// already did; decoding the file and premultiplying it again reproduces
    /// a pixel whose color does not exceed its alpha to within one step per
    /// channel.
    #[must_use]
    pub fn encode_png(&self) -> Vec<u8> {
        if self
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .all(|pixel| pixel[3] == u8::MAX)
        {
            return png::encode(self.width, self.height, &self.rgba);
        }
        let mut straight = self.rgba.clone();
        let decode: [f32; 256] = std::array::from_fn(|byte| srgb_decode(byte as u8));
        for pixel in straight.as_chunks_mut::<4>().0 {
            match pixel[3] {
                u8::MAX => {}
                0 => pixel.fill(0),
                alpha => {
                    let coverage = f32::from(alpha) / 255.0;
                    for channel in &mut pixel[..3] {
                        *channel = srgb_encode(decode[usize::from(*channel)] / coverage);
                    }
                }
            }
        }
        png::encode(self.width, self.height, &straight)
    }
}

/// A finished screenshot.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(deny_unknown_fields)
)]
pub struct Capture {
    /// The requested target.
    pub target: CaptureTarget,
    /// The window incarnation the image comes from,
    /// [`WindowInspection::id`](crate::WindowInspection::id).
    pub window: u64,
    /// The component the image shows, for component and path targets.
    pub component: Option<u64>,
    /// The captured logical region in window coordinates, `x, y, width,
    /// height`: the target's visible rendered intersection with its clip and
    /// the window. With the `serde` feature it is encoded like the other
    /// diagnostic floats: a non-finite value is explicit text, and a finite
    /// number too large for an `f32` is rejected.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_float::array4"))]
    pub region: [f32; 4],
    /// Whether the target's own bounds extend beyond `region`, so the image
    /// shows only part of it.
    pub clipped: bool,
    /// Physical pixels per logical pixel, including application zoom. The
    /// image's size is `region` mapped onto the frame, with each edge rounded
    /// outward to the pixel grid and then clamped to the frame. The mapping is
    /// the frame's physical extent over the window's logical size on each axis:
    /// this scale, except that a host-supplied physical extent can make it
    /// differ in the last digits. Serialized like
    /// [`Self::region`].
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_float::f64"))]
    pub scale: f64,
    /// The runtime clock of the flush whose state the frame shows.
    pub at: Duration,
    /// Whether a development overlay was drawn into the image: the request
    /// asked for it, one was active, and the build has debug assertions.
    pub overlay: bool,
    /// The pixels.
    pub image: FrameImage,
}

/// Work a host does for one accepted capture: render this window's current
/// scene, with or without overlays, and hand the frame back.
///
/// `Runtime::pending_captures` lists them; the host renders each with the
/// scene `Runtime::capture_scene` gives it and completes it with
/// `Runtime::complete_capture`. A job stays listed until completed, so a host
/// that cannot render a window right now (occluded, lost surface) completes
/// it with [`CaptureError::SurfaceUnavailable`] rather than leaving it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaptureJob {
    /// Identifies the job for `complete_capture`; never reused.
    pub id: u64,
    /// The window to render, [`WindowInspection::id`](crate::WindowInspection::id).
    pub window: u64,
    /// Whether the scene for it includes overlays.
    pub include_overlay: bool,
}

/// A dependency-free PNG writer: stored deflate blocks, zlib framing, CRC-32.
mod png {
    const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1A, b'\n'];

    fn crc_table() -> [u32; 256] {
        let mut table = [0u32; 256];
        let mut n = 0;
        while n < 256 {
            let mut c = n as u32;
            let mut k = 0;
            while k < 8 {
                c = if c & 1 != 0 {
                    0xEDB8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
                k += 1;
            }
            table[n] = c;
            n += 1;
        }
        table
    }

    fn crc(table: &[u32; 256], bytes: &[u8]) -> u32 {
        let mut c = 0xFFFF_FFFFu32;
        for byte in bytes {
            c = table[((c ^ u32::from(*byte)) & 0xFF) as usize] ^ (c >> 8);
        }
        c ^ 0xFFFF_FFFF
    }

    fn adler(bytes: &[u8]) -> u32 {
        let (mut a, mut b) = (1u32, 0u32);
        for chunk in bytes.chunks(5552) {
            for byte in chunk {
                a += u32::from(*byte);
                b += a;
            }
            a %= 65521;
            b %= 65521;
        }
        (b << 16) | a
    }

    fn chunk(out: &mut Vec<u8>, table: &[u32; 256], kind: &[u8; 4], data: &[u8]) {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let start = out.len();
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        let sum = crc(table, &out[start..]);
        out.extend_from_slice(&sum.to_be_bytes());
    }

    pub(super) fn encode(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
        let table = crc_table();
        let stride = width as usize * 4;
        // Each scanline is prefixed with filter type 0 (none).
        let mut raw = Vec::with_capacity((stride + 1) * height as usize);
        for row in rgba.chunks(stride) {
            raw.push(0);
            raw.extend_from_slice(row);
        }
        // zlib: CMF/FLG for deflate with a 32 KiB window and no preset
        // dictionary, then stored blocks of at most 65535 bytes.
        let mut zlib = Vec::with_capacity(raw.len() + raw.len() / 65535 * 5 + 11);
        zlib.extend_from_slice(&[0x78, 0x01]);
        let mut blocks = raw.chunks(65535).peekable();
        if blocks.peek().is_none() {
            zlib.extend_from_slice(&[1, 0, 0, 0xFF, 0xFF]);
        }
        while let Some(block) = blocks.next() {
            let last = blocks.peek().is_none();
            zlib.push(u8::from(last));
            let len = block.len() as u16;
            zlib.extend_from_slice(&len.to_le_bytes());
            zlib.extend_from_slice(&(!len).to_le_bytes());
            zlib.extend_from_slice(block);
        }
        zlib.extend_from_slice(&adler(&raw).to_be_bytes());

        let mut out = Vec::with_capacity(zlib.len() + 64);
        out.extend_from_slice(&SIGNATURE);
        let mut header = Vec::with_capacity(13);
        header.extend_from_slice(&width.to_be_bytes());
        header.extend_from_slice(&height.to_be_bytes());
        // Bit depth 8, color type 6 (RGBA), deflate, no filter, no interlace.
        header.extend_from_slice(&[8, 6, 0, 0, 0]);
        chunk(&mut out, &table, b"IHDR", &header);
        chunk(&mut out, &table, b"IDAT", &zlib);
        chunk(&mut out, &table, b"IEND", &[]);
        out
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn crc_matches_the_reference_value() {
            let table = crc_table();
            assert_eq!(crc(&table, b"123456789"), 0xCBF4_3926);
            assert_eq!(adler(b"Wikipedia"), 0x11E6_0398);
        }

        #[test]
        fn a_stored_stream_inflates_to_the_scanlines() {
            let rgba: Vec<u8> = (0..2 * 3 * 4).map(|i| i as u8).collect();
            let png = encode(2, 3, &rgba);
            assert_eq!(&png[..8], &SIGNATURE);
            assert_eq!(&png[12..16], b"IHDR");
            let idat = 8 + 25;
            assert_eq!(&png[idat + 4..idat + 8], b"IDAT");
            let len = u32::from_be_bytes([png[idat], png[idat + 1], png[idat + 2], png[idat + 3]])
                as usize;
            let zlib = &png[idat + 8..idat + 8 + len];
            assert_eq!(&zlib[..2], &[0x78, 0x01]);
            // One final stored block holding three filtered scanlines.
            assert_eq!(zlib[2], 1);
            let stored = u16::from_le_bytes([zlib[3], zlib[4]]) as usize;
            assert_eq!(stored, 3 * (2 * 4 + 1));
            assert_eq!(!stored as u16, u16::from_le_bytes([zlib[5], zlib[6]]));
            let raw = &zlib[7..7 + stored];
            assert_eq!(raw[0], 0);
            assert_eq!(&raw[1..9], &rgba[..8]);
            assert_eq!(raw[9], 0);
            let sum = u32::from_be_bytes([
                zlib[7 + stored],
                zlib[8 + stored],
                zlib[9 + stored],
                zlib[10 + stored],
            ]);
            assert_eq!(sum, adler(raw));
            assert_eq!(&png[png.len() - 8..png.len() - 4], b"IEND");
        }

        #[test]
        fn a_large_image_spans_several_stored_blocks() {
            let width = 300u32;
            let height = 100u32;
            let rgba = vec![7u8; (width * height * 4) as usize];
            let png = encode(width, height, &rgba);
            let raw_len = (width as usize * 4 + 1) * height as usize;
            assert!(raw_len > 65535);
            // 8 signature + 25 IHDR + 12 chunk framing + zlib(2 + blocks*5 + raw + 4) + 12 IEND
            let blocks = raw_len.div_ceil(65535);
            assert_eq!(png.len(), 8 + 25 + 12 + 2 + blocks * 5 + raw_len + 4 + 12);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Premultiplies one straight channel the way `RasterImage` does.
    fn premultiply(straight: u8, alpha: u8) -> u8 {
        srgb_encode(srgb_decode(straight) * (f32::from(alpha) / 255.0))
    }

    /// The scanline bytes of an image `encode_png` wrote as one stored block.
    fn scanlines(png: &[u8]) -> &[u8] {
        let zlib = &png[8 + 25 + 8..];
        let stored = usize::from(u16::from_le_bytes([zlib[3], zlib[4]]));
        &zlib[7..7 + stored]
    }

    #[test]
    fn png_pixels_are_straight_alpha() {
        // A half-covered white written premultiplied in linear light, a clear
        // pixel with leftover color, and an opaque one.
        let half = premultiply(255, 128);
        assert_ne!(half, 255);
        let frame =
            FrameImage::new(3, 1, vec![half, half, 0, 128, 7, 8, 9, 0, 1, 2, 3, 255]).unwrap();
        let png = frame.encode_png();
        assert_eq!(
            scanlines(&png),
            &[0, 255, 255, 0, 128, 0, 0, 0, 0, 1, 2, 3, 255]
        );
        // The frame keeps its premultiplied pixels.
        assert_eq!(frame.pixels()[0], half);
    }

    #[test]
    fn premultiplying_the_straight_png_pixel_again_is_within_one_step() {
        for alpha in 1..u8::MAX {
            let coverage = f32::from(alpha) / 255.0;
            for color in 0..=u8::MAX {
                // Only colors that do not exceed their alpha are premultiplied.
                if srgb_decode(color) > coverage {
                    break;
                }
                let frame = FrameImage::new(1, 1, vec![color, 0, 0, alpha]).unwrap();
                let straight = scanlines(&frame.encode_png())[1];
                let again = premultiply(straight, alpha);
                assert!(
                    again.abs_diff(color) <= 1,
                    "alpha {alpha}: {color} became {straight}, then {again}"
                );
            }
        }
    }

    #[test]
    fn frames_validate_and_crop() {
        assert_eq!(
            FrameImage::new(0, 4, Vec::new()),
            Err(CaptureError::Frame(FrameError::PixelCount {
                width: 0,
                height: 4,
                expected: 0,
                actual: 0,
            }))
        );
        assert_eq!(
            FrameImage::new(4, 3, vec![0; 5]),
            Err(CaptureError::Frame(FrameError::PixelCount {
                width: 4,
                height: 3,
                expected: 48,
                actual: 5,
            }))
        );
        assert_eq!(
            FrameImage::new(4097, 4096, Vec::new()),
            Err(CaptureError::TooLarge)
        );
        // Dimensions whose product overflows are too large, not wrapped.
        assert_eq!(
            FrameImage::new(2_147_529_989, 2_147_437_308, vec![0; 166_832]),
            Err(CaptureError::TooLarge)
        );
        let pixels: Vec<u8> = (0..4 * 3 * 4).map(|i| i as u8).collect();
        let frame = FrameImage::new(4, 3, pixels.clone()).unwrap();
        let crop = frame.crop(1, 1, 2, 2).unwrap();
        assert_eq!((crop.width(), crop.height()), (2, 2));
        assert_eq!(&crop.pixels()[..8], &pixels[(4 + 1) * 4..(4 + 3) * 4]);
        assert_eq!(&crop.pixels()[8..], &pixels[(8 + 1) * 4..(8 + 3) * 4]);
        assert_eq!(
            frame.crop(3, 0, 2, 1),
            Err(CaptureError::Frame(FrameError::CropOutside {
                x: 3,
                y: 0,
                width: 2,
                height: 1,
                frame_width: 4,
                frame_height: 3,
            }))
        );
        assert_eq!(frame.crop(0, 0, 0, 1), Err(CaptureError::Hidden));
        assert!(matches!(
            frame.crop(u32::MAX, 0, 2, 1),
            Err(CaptureError::Frame(FrameError::CropOutside {
                x: u32::MAX,
                ..
            }))
        ));
    }
}
