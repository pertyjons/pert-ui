//! Limits a renderer reports for a surface, capture or scene, shared by every
//! renderer so applications and hosts handle them alike.

/// The kind of scene resource a [`LimitError`] concerns.
///
/// With the `serde` feature it serializes as its variant name, as a
/// [`LimitError`] carries it.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[non_exhaustive]
pub enum ResourceKind {
    /// 8-bit coverage masks ([`MaskImage`](crate::MaskImage)): shaped text, shapes and the vector
    /// coverage fallback.
    Mask,
    /// Premultiplied sRGB color images ([`RasterImage`](crate::RasterImage)).
    Image,
}
impl ResourceKind {
    /// Unique bytes of this kind that one scene may reference: 64 MiB of
    /// coverage masks and 256 MiB of image pixels.
    ///
    /// Content deduplicated by the scene's resource tables counts once, however
    /// often it is drawn.
    #[must_use]
    pub const fn scene_budget_bytes(self) -> usize {
        match self {
            Self::Mask => 64 * 1024 * 1024,
            Self::Image => 256 * 1024 * 1024,
        }
    }
}
impl std::fmt::Display for ResourceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Mask => "coverage mask",
            Self::Image => "image",
        })
    }
}

/// A documented budget or device limit that a surface, capture or scene exceeds.
///
/// These are fixable conditions, not device faults. Device limits are the
/// hardware adapter's own texture dimensions and the GPU renderer's buffer
/// limits. [`FrameImage::MAX_BYTES`](crate::FrameImage::MAX_BYTES) has the one
/// variant [`Self::FrameSize`], whoever reports it: the GPU renderer for a
/// capture, the CPU renderer through its error's `Limit` variant, and
/// [`FrameImage::new`](crate::FrameImage::new) and capture requests through
/// [`CaptureError::Limit`](crate::CaptureError::Limit). Renderers report them,
/// and the native application returns them as its limit error.
///
/// With the `serde` feature it serializes in serde's externally tagged enum
/// form, as [`CaptureError::Limit`](crate::CaptureError::Limit) carries it.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[non_exhaustive]
pub enum LimitError {
    /// A surface or capture extent exceeds the device's maximum 2D texture
    /// dimension, in physical pixels.
    #[error("a {width}×{height} surface exceeds the device texture dimension {max}")]
    SurfaceExtent {
        /// Requested width in physical pixels.
        width: u32,
        /// Requested height in physical pixels.
        height: u32,
        /// The device's maximum 2D texture dimension.
        max: u32,
    },
    /// One scene resource exceeds the device's maximum 2D texture dimension.
    #[error("a {width}×{height} {kind} exceeds the device texture dimension {max}")]
    ResourceExtent {
        /// The resource's kind.
        kind: ResourceKind,
        /// Resource width in pixels.
        width: u32,
        /// Resource height in pixels.
        height: u32,
        /// The device's maximum 2D texture dimension.
        max: u32,
    },
    /// A scene's unique resources of one kind exceed
    /// [`ResourceKind::scene_budget_bytes`].
    #[error("scene {kind} data exceeds its {} MiB budget", kind.scene_budget_bytes() / (1024 * 1024))]
    SceneBudget {
        /// The resource kind whose budget is exceeded.
        kind: ResourceKind,
    },
    /// A scene's encoded vertices exceed the device's maximum buffer size.
    #[error("{bytes} bytes of scene vertices exceed the device buffer limit {max}")]
    VertexBuffer {
        /// Encoded vertex bytes the scene needs.
        bytes: u64,
        /// The device's maximum buffer size in bytes.
        max: u64,
    },
    /// A scene has more vertices than a `u32` draw range can address.
    #[error("the scene has more vertices than a draw range can address")]
    VertexCount,
    /// A frame's 8-bit RGBA pixels would exceed
    /// [`FrameImage::MAX_BYTES`](crate::FrameImage::MAX_BYTES): a capture, a
    /// CPU-rendered scene or a software-presented window. Like
    /// [`Self::SurfaceExtent`] for a GPU surface, a software-presented window
    /// stays unpresented until a smaller size.
    #[error("a {width}×{height} frame exceeds the frame image size limit")]
    FrameSize {
        /// Frame width in physical pixels.
        width: u32,
        /// Frame height in physical pixels.
        height: u32,
    },
}
