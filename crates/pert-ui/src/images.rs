//! Decoding image files into [`RasterImage`]s, behind opt-in format features.
//!
//! Nothing here is compiled unless a format feature is enabled, and each format
//! adds only its own decoder: `png`, `jpeg`, `gif`, `webp`, `bmp`, `ico`, `tiff`,
//! `pnm` and `qoi`, or `image-formats` for all of them. Each implies the
//! `images` feature, which provides this module; `images` alone compiles the API
//! with no decoder, so [`decode`] reports [`ImageError::FormatDisabled`] or
//! [`ImageError::UnknownFormat`], and [`load`] does too once it has read the
//! file, or [`ImageError::Io`] when it cannot. [`RasterImage::from_srgba8`] and the
//! [`Image`](crate::Image) component need no feature, so an application with its
//! own decoder or procedurally generated pixels pays for none of this.
//!
//! The format is recognized from the content, never from a file name. Decoding
//! produces straight-alpha sRGB RGBA8, which [`RasterImage::from_srgba8`]
//! premultiplies: grayscale and color without alpha become opaque RGBA, 16-bit
//! channels are reduced to 8 bits, an EXIF orientation is applied, an animated
//! GIF or WebP yields its first frame, and embedded color profiles and gamma
//! metadata are ignored, so the pixels are treated as sRGB.
//!
//! Decoding runs on the calling thread and takes time proportional to the file
//! and the image; a large image can take tens of milliseconds or more. Decode
//! off the UI thread when that matters and hand the `Send` result to the UI
//! thread through a [`Mailbox`](crate::Mailbox), whose handler writes the
//! binding an [`Image::bound`](crate::Image::bound) follows. Never decode in
//! `paint`.
//!
//! ```
//! use pert_ui::{Capacity, Element, Image, RasterImage, Runtime};
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut ui = Runtime::new();
//! let picture = ui.signal(None::<RasterImage>);
//! let target = picture.clone();
//! // Keep the mailbox as long as results matter; dropping it closes it.
//! let mailbox = ui.mailbox(Capacity::new(1)?, move |image: RasterImage| {
//!     target.set(Some(image))
//! })?;
//! ui.mount_tree(Element::new("picture", Image::bound(picture.read_only())))?;
//! let sender = mailbox.sender();
//! std::thread::spawn(move || {
//!     // A worker would call `images::decode` here; this one makes pixels.
//!     let pixels = [40, 120, 220, 255].repeat(16);
//!     if let Ok(image) = RasterImage::from_srgba8(4, 4, pixels) {
//!         let _ = sender.send(image); // Full or Closed: the result is not needed.
//!     }
//! })
//! .join()
//! .expect("the worker finishes");
//! ui.flush(std::time::Duration::ZERO)?;
//! assert!(picture.get()?.is_some());
//! # Ok(()) }
//! ```
//!
//! ```
//! # #[cfg(feature = "png")] {
//! use pert_ui::images::{self, ImageError, ImageFormat};
//! // A PNG this crate would decode needs the `png` feature…
//! assert!(ImageFormat::Png.is_enabled());
//! // …and bytes that are no image at all are reported as such.
//! assert!(matches!(images::decode(b"not an image"), Err(ImageError::UnknownFormat)));
//! # }
//! ```
use crate::RasterImage;
use image::ImageDecoder as _;
use std::{io::Cursor, path::Path};

/// Declares [`ImageFormat`] and [`ImageFormat::ALL`] from one list of
/// variants, so a format added to the enumeration cannot be left out of `ALL`
/// and therefore out of [`ImageFormat::enabled`].
macro_rules! image_formats {
    ($($(#[$attribute:meta])* $variant:ident,)+) => {
        /// A file format this module can recognize, whether or not its decoder is enabled.
        ///
        /// `Display` writes the format's customary name, such as `PNG` or `WebP`.
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
        #[non_exhaustive]
        pub enum ImageFormat {
            $($(#[$attribute])* $variant,)+
        }
        impl ImageFormat {
            /// Every format in this enumeration, enabled or not, in declaration
            /// order. Adding a format lengthens the slice without changing its
            /// type.
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];
        }
    };
}
image_formats! {
    /// Portable Network Graphics; feature `png`.
    Png,
    /// JPEG; feature `jpeg`.
    Jpeg,
    /// Graphics Interchange Format, first frame only; feature `gif`.
    Gif,
    /// WebP, lossy or lossless, first frame only; feature `webp`.
    WebP,
    /// Windows bitmap; feature `bmp`.
    Bmp,
    /// Windows icon, largest entry; feature `ico`.
    Ico,
    /// Tagged Image File Format; feature `tiff`.
    Tiff,
    /// Portable anymap (PBM, PGM, PPM, PAM); feature `pnm`.
    Pnm,
    /// Quite OK Image format; feature `qoi`.
    Qoi,
}
impl std::fmt::Display for ImageFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Png => "PNG",
            Self::Jpeg => "JPEG",
            Self::Gif => "GIF",
            Self::WebP => "WebP",
            Self::Bmp => "BMP",
            Self::Ico => "ICO",
            Self::Tiff => "TIFF",
            Self::Pnm => "PNM",
            Self::Qoi => "QOI",
        })
    }
}
impl ImageFormat {
    /// Whether this build decodes the format, which its Cargo feature decides.
    #[must_use]
    pub const fn is_enabled(self) -> bool {
        match self {
            Self::Png => cfg!(feature = "png"),
            Self::Jpeg => cfg!(feature = "jpeg"),
            Self::Gif => cfg!(feature = "gif"),
            Self::WebP => cfg!(feature = "webp"),
            Self::Bmp => cfg!(feature = "bmp"),
            Self::Ico => cfg!(feature = "ico"),
            Self::Tiff => cfg!(feature = "tiff"),
            Self::Pnm => cfg!(feature = "pnm"),
            Self::Qoi => cfg!(feature = "qoi"),
        }
    }
    /// The Cargo feature of the `pert-ui` crate that enables this format.
    #[must_use]
    pub const fn feature(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpeg",
            Self::Gif => "gif",
            Self::WebP => "webp",
            Self::Bmp => "bmp",
            Self::Ico => "ico",
            Self::Tiff => "tiff",
            Self::Pnm => "pnm",
            Self::Qoi => "qoi",
        }
    }
    /// Customary lowercase file name extensions, without the dot, for building
    /// a file dialog's filter. Recognition never relies on them.
    #[must_use]
    pub const fn extensions(self) -> &'static [&'static str] {
        match self {
            Self::Png => &["png"],
            Self::Jpeg => &["jpg", "jpeg", "jpe", "jfif"],
            Self::Gif => &["gif"],
            Self::WebP => &["webp"],
            Self::Bmp => &["bmp"],
            Self::Ico => &["ico"],
            Self::Tiff => &["tif", "tiff"],
            Self::Pnm => &["pbm", "pgm", "ppm", "pam", "pnm"],
            Self::Qoi => &["qoi"],
        }
    }
    /// The formats this build decodes.
    pub fn enabled() -> impl Iterator<Item = Self> {
        Self::ALL
            .iter()
            .copied()
            .filter(|format| format.is_enabled())
    }
    /// Recognizes a format from the leading bytes of a file, whether or not its
    /// decoder is enabled; `None` for anything else.
    #[must_use]
    pub fn guess(bytes: &[u8]) -> Option<Self> {
        Some(match image::guess_format(bytes).ok()? {
            image::ImageFormat::Png => Self::Png,
            image::ImageFormat::Jpeg => Self::Jpeg,
            image::ImageFormat::Gif => Self::Gif,
            image::ImageFormat::WebP => Self::WebP,
            image::ImageFormat::Bmp => Self::Bmp,
            image::ImageFormat::Ico => Self::Ico,
            image::ImageFormat::Tiff => Self::Tiff,
            image::ImageFormat::Pnm => Self::Pnm,
            image::ImageFormat::Qoi => Self::Qoi,
            _ => return None,
        })
    }
    const fn backend(self) -> image::ImageFormat {
        match self {
            Self::Png => image::ImageFormat::Png,
            Self::Jpeg => image::ImageFormat::Jpeg,
            Self::Gif => image::ImageFormat::Gif,
            Self::WebP => image::ImageFormat::WebP,
            Self::Bmp => image::ImageFormat::Bmp,
            Self::Ico => image::ImageFormat::Ico,
            Self::Tiff => image::ImageFormat::Tiff,
            Self::Pnm => image::ImageFormat::Pnm,
            Self::Qoi => image::ImageFormat::Qoi,
        }
    }
}

/// Why an image file could not become a [`RasterImage`].
///
/// Unlike most library errors it is neither `Clone` nor `PartialEq`, because
/// [`Self::Io`] keeps the `std::io::Error` as its typed source; match on the
/// variants instead.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ImageError {
    /// The content is not a format listed in [`ImageFormat`].
    #[error("image format not recognized")]
    UnknownFormat,
    /// The content is a known format whose decoder this build does not include.
    #[error("decoding {0} is disabled; enable the pert-ui `{feature}` feature", feature = .0.feature())]
    FormatDisabled(ImageFormat),
    /// The image would exceed [`RasterImage::MAX_BYTES`] as RGBA8. This is
    /// checked from the file's header, before the pixels are decoded.
    #[error("{width}x{height} image exceeds the 64 MiB RasterImage limit")]
    TooLarge {
        /// Width in image pixels, as the file declares it.
        width: u32,
        /// Height in image pixels, as the file declares it.
        height: u32,
    },
    /// Decoding the image would allocate more than the decoder bound that
    /// [`decode`] documents (four times [`RasterImage::MAX_BYTES`]), for
    /// example for a format's intermediate pixel buffers. Unlike
    /// [`Self::TooLarge`], the file's declared dimensions fit the image limit
    /// or were not yet known when the bound was reached.
    #[error("decoding the image would allocate beyond the 256 MiB decoder bound")]
    AllocationLimit,
    /// The data is damaged or truncated, or uses a variant of its format that
    /// the decoder does not support; the text is the decoder's explanation.
    #[error("malformed image: {0}")]
    Malformed(String),
    /// Reading the file failed.
    #[error("reading the image failed")]
    Io(#[from] std::io::Error),
}

/// Decodes an image file held in memory.
///
/// # Errors
/// [`ImageError::UnknownFormat`] or [`ImageError::FormatDisabled`] when the
/// content cannot be decoded by this build, [`ImageError::TooLarge`] before any
/// pixel is decoded for an image over the size limit,
/// [`ImageError::AllocationLimit`] for a file whose decoding would allocate
/// beyond the bound below, and [`ImageError::Malformed`] for damaged or
/// truncated data or a declared zero width or height.
///
/// Decoder allocations, including the header and ancillary data such as a
/// compressed color profile, are bounded to four times
/// [`RasterImage::MAX_BYTES`] (256 MiB), besides the copy of the file the caller
/// holds. Metadata that would inflate beyond that bound is dropped, not
/// decoded, since the pixels are treated as sRGB regardless.
pub fn decode(bytes: &[u8]) -> Result<RasterImage, ImageError> {
    let format = ImageFormat::guess(bytes).ok_or(ImageError::UnknownFormat)?;
    if !format.is_enabled() {
        return Err(ImageError::FormatDisabled(format));
    }
    let mut reader = image::ImageReader::with_format(Cursor::new(bytes), format.backend());
    // Only an allocation limit, and the one decoding keeps: the PNG decoder
    // inflates ancillary chunks such as an embedded color profile while it
    // reads the header, and cannot change its limit afterwards. No dimension
    // limit is set, so the size check below sees the declared dimensions and
    // reports them. A decoder may hold wider intermediate pixels, such as
    // 16-bit channels, before conversion to RGBA8.
    let mut limits = image::Limits::no_limits();
    limits.max_alloc = Some(MAX_DECODER_ALLOCATION);
    reader.limits(limits.clone());
    let mut decoder = reader.into_decoder().map_err(backend)?;
    let (width, height) = decoder.dimensions();
    if width == 0 || height == 0 {
        return Err(ImageError::Malformed(format!(
            "the file declares a {width}x{height} image"
        )));
    }
    if !fits_raster(width, height) {
        return Err(ImageError::TooLarge { width, height });
    }
    decoder.set_limits(limits).map_err(backend)?;
    let orientation = decoder.orientation().map_err(backend)?;
    let mut decoded = image::DynamicImage::from_decoder(decoder).map_err(backend)?;
    decoded.apply_orientation(orientation);
    let pixels = decoded.into_rgba8();
    let (width, height) = pixels.dimensions();
    RasterImage::from_srgba8(width, height, pixels.into_raw())
        .map_err(|_| ImageError::TooLarge { width, height })
}

/// Reads and decodes an image file.
///
/// The whole file is read into memory on the calling thread, then decoded as
/// [`decode`] describes; the file name's extension is not consulted.
///
/// # Errors
/// [`ImageError::Io`] when the file cannot be read, otherwise those of [`decode`].
pub fn load(path: impl AsRef<Path>) -> Result<RasterImage, ImageError> {
    decode(&std::fs::read(path)?)
}

/// What a decoder may allocate for one image, header and metadata included.
const MAX_DECODER_ALLOCATION: u64 = 4 * RasterImage::MAX_BYTES as u64;

/// Whether declared dimensions fit [`RasterImage::MAX_BYTES`] as RGBA8.
///
/// Compares the pixel count, which cannot overflow `u64` for any pair of
/// `u32` dimensions, rather than the byte count, which can.
fn fits_raster(width: u32, height: u32) -> bool {
    u64::from(width) * u64::from(height) <= RasterImage::MAX_BYTES as u64 / 4
}

fn backend(error: image::ImageError) -> ImageError {
    match error {
        // Only an allocation limit is set, so a limit error is that bound.
        image::ImageError::Limits(_) => ImageError::AllocationLimit,
        // The format was recognized and its decoder is enabled, so an
        // unsupported result is a variant inside the format, and the bytes are
        // already in memory, so a read error means truncated data: both are
        // malformed input.
        error => ImageError::Malformed(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::{ImageError, backend, fits_raster};
    use image::error::{LimitError, LimitErrorKind};

    #[test]
    fn a_decoder_limit_is_reported_as_the_allocation_bound_not_malformed_data() {
        let limit =
            image::ImageError::Limits(LimitError::from_kind(LimitErrorKind::InsufficientMemory));
        assert!(matches!(backend(limit), ImageError::AllocationLimit));
        let truncated = image::ImageError::IoError(std::io::ErrorKind::UnexpectedEof.into());
        assert!(matches!(backend(truncated), ImageError::Malformed(_)));
    }

    #[test]
    fn maximal_declared_dimensions_are_too_large_without_overflow() {
        assert!(!fits_raster(u32::MAX, u32::MAX));
        assert!(!fits_raster(u32::MAX, 1));
        let side = 1 << 12;
        assert!(fits_raster(side, side));
        assert!(!fits_raster(side, side + 1));
    }
}
