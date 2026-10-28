//! SVG icons and similar graphics, rasterized into the resources the renderer
//! already knows, behind the opt-in `svg` feature.
//!
//! Nothing here is compiled unless the feature is enabled, and it adds only
//! resvg and tiny-skia: no font registry, because SVG `<text>` is deliberately
//! unsupported (see below). There is no SVG drawing command and no live SVG
//! document in the public API. A drawing becomes either a
//! [`RasterImage`], like any decoded picture, or an 8-bit
//! coverage [`MaskImage`] that a theme color tints, which is
//! what a monochrome icon should be: it reuses the mask table the text and shape
//! caches already fill instead of holding one opaque bitmap per color.
//!
//! An SVG has no pixels of its own, so rasterization needs a size. The
//! [`Svg`](crate::Svg) component, at the crate root beside the other widgets,
//! asks for its allocation, rasterizes for the physical size that allocation has
//! on this display, and keeps that one rasterization until its size or the
//! display scale changes. Rasterization happens during layout, never in
//! `paint`. This module holds what works without the component: the
//! rasterizers, the intrinsic size and their errors.
//!
//! The name is `svg_images`, not `svg`: a crate that globs the crate root
//! and depends on the `svg` crate would otherwise find `svg` ambiguous in a
//! `use` path and resolve it to this module elsewhere.
//!
//! # Unsupported: SVG text
//!
//! `<text>` elements are ignored. Rendering them would need a second font
//! registry beside the one the library already uses for its own text, and then a
//! choice between the fonts a machine happens to have and the ones we bundle;
//! that choice is deferred rather than taken. Convert text to paths before
//! shipping an asset, which every drawing tool can do, or draw the text with a
//! [`Label`](crate::Label) beside the graphic.
//!
//! ```
//! # #[cfg(feature = "svg")] {
//! use pert_ui::{ColorRole, Element, Runtime, Svg, Viewport};
//! const CHECK: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
//!   <path d="M3 8 L7 12 L13 4" stroke="black" stroke-width="2" fill="none"/>
//! </svg>"#;
//! let mut ui = Runtime::new();
//! ui.set_viewport(Viewport::new(64.0, 64.0, 2.0)?)?;
//! // A monochrome icon follows the theme, like every other drawn mark.
//! ui.mount_tree(Element::new("check", Svg::tinted(CHECK, ColorRole::Primary)?))?;
//! ui.flush(std::time::Duration::ZERO)?;
//! assert!(!ui.paint()?.masks().is_empty(), "the icon became a coverage mask");
//! # }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
use pert_ui_runtime::{DrawError, MaskImage, RasterImage, Size};

pub(crate) mod component;

/// Why an SVG could not be read or rasterized.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum SvgError {
    /// The source is not SVG this build can read. The message comes from the
    /// parser and names what it found.
    #[error("the SVG could not be parsed: {0}")]
    Parse(String),
    /// The drawing declares no usable size, or a requested size is zero or
    /// larger than the rasterizer accepts.
    #[error("the requested SVG raster size is not usable")]
    InvalidSize,
    /// The requested raster would exceed [`RasterImage::MAX_BYTES`] as RGBA8,
    /// or [`MaskImage::MAX_PIXELS`] as a coverage mask. This is checked before
    /// anything is allocated or rendered.
    #[error("{width}x{height} SVG raster exceeds the image size limit")]
    TooLarge {
        /// Requested width in physical pixels.
        width: u32,
        /// Requested height in physical pixels.
        height: u32,
    },
    /// Converting the rasterized pixels into a [`RasterImage`] or [`MaskImage`]
    /// was refused by one of their resource limits.
    ///
    /// This variant is defensive: rasterization refuses every size those limits
    /// would reject as [`SvgError::TooLarge`] before rendering, so the current
    /// constructors never return it. It remains so that a future limit the
    /// size check does not anticipate is still reported rather than lost.
    #[error(transparent)]
    Resource(#[from] DrawError),
}

/// The deepest element nesting a source may have, counting the root `<svg>`
/// as one; a deeper source is [`SvgError::Parse`].
///
/// The SVG parser and rasterizer recurse once per level, with large frames in
/// an unoptimized build, so a deeply nested source would exhaust the stack of
/// the thread that reads it and abort the process. This bound keeps parsing
/// and rasterizing inside a 2 MiB stack, the Rust default for spawned threads,
/// with room to spare even without optimization, while staying deeper than icons
/// and illustrations nest in practice. Nesting is counted after XML entities
/// are expanded, so an entity cannot hide it.
pub const MAX_NESTING: usize = 64;

fn parse(source: &[u8]) -> Result<resvg::usvg::Tree, SvgError> {
    use resvg::usvg::{self, roxmltree};
    let text = std::str::from_utf8(source)
        .map_err(|_| SvgError::Parse("the source is not UTF-8 text".into()))?;
    let too_deep = || {
        SvgError::Parse(format!(
            "elements are nested more than {MAX_NESTING} levels deep"
        ))
    };
    // The XML tokenizer recurses per element too, so the text is measured
    // before it runs, counting markup in entities conservatively. The expanded
    // document is checked again before usvg's much deeper recursion.
    if textual_nesting_exceeds(source, MAX_NESTING) {
        return Err(too_deep());
    }
    let options = roxmltree::ParsingOptions {
        allow_dtd: true,
        ..roxmltree::ParsingOptions::default()
    };
    let document = roxmltree::Document::parse_with_options(text, options)
        .map_err(|error| SvgError::Parse(error.to_string()))?;
    let expanded_too_deep = document.descendants().any(|node| {
        node.is_element() && node.ancestors().filter(|node| node.is_element()).count() > MAX_NESTING
    });
    if expanded_too_deep {
        return Err(too_deep());
    }
    // A drawing is the bytes it was given: `<image>` loads neither files nor
    // embedded data. usvg's default resolver would read any path from disk.
    let options = usvg::Options {
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_data: Box::new(|_, _, _| None),
            resolve_string: Box::new(|_, _| None),
        },
        ..usvg::Options::default()
    };
    usvg::Tree::from_xmltree(&document, &options)
        .map_err(|error| SvgError::Parse(error.to_string()))
}

/// Whether element tags in `text` nest deeper than `limit`, measured without
/// recursion and without parsing. Comments, CDATA sections and processing
/// instructions are skipped. Every start tag in a document type declaration's
/// quoted values, such as an entity's replacement text, counts as one level
/// for the whole document, because entity references can nest those levels
/// inside each other. The measure errs toward counting too much, never too
/// little.
fn textual_nesting_exceeds(text: &[u8], limit: usize) -> bool {
    let skip_to = |from: usize, end: &[u8]| {
        text[from..]
            .windows(end.len())
            .position(|window| window == end)
            .map_or(text.len(), |at| from + at + end.len())
    };
    let mut depth = 0usize;
    let mut index = 0;
    while index < text.len() {
        if text[index] != b'<' {
            index += 1;
            continue;
        }
        let rest = &text[index..];
        if rest.starts_with(b"<!--") {
            index = skip_to(index + 4, b"-->");
        } else if rest.starts_with(b"<![CDATA[") {
            index = skip_to(index + 9, b"]]>");
        } else if rest.starts_with(b"<?") {
            index = skip_to(index + 2, b"?>");
        } else if rest.starts_with(b"</") {
            depth = depth.saturating_sub(1);
            index += 2;
        } else if rest.starts_with(b"<!DOCTYPE") {
            // Entities can nest inside each other where they are expanded, so
            // every start tag in a quoted value adds to the depth for good.
            index += 9;
            let (mut quote, mut subset) = (None, false);
            while index < text.len() {
                let byte = text[index];
                match quote {
                    Some(open) if byte == open => quote = None,
                    Some(_) => {
                        let next = text.get(index + 1).copied();
                        if byte == b'<' && !matches!(next, Some(b'/' | b'!' | b'?') | None) {
                            depth += 1;
                            if depth > limit {
                                return true;
                            }
                        }
                    }
                    None if byte == b'"' || byte == b'\'' => quote = Some(byte),
                    None if text[index..].starts_with(b"<!--") => {
                        index = skip_to(index + 4, b"-->");
                        continue;
                    }
                    None if byte == b'[' => subset = true,
                    None if byte == b']' => subset = false,
                    None if byte == b'>' && !subset => break,
                    None => {}
                }
                index += 1;
            }
            index += 1;
        } else if rest.starts_with(b"<!") {
            index += 2;
        } else {
            depth += 1;
            if depth > limit {
                return true;
            }
            // Find the end of the start tag outside quoted attribute values,
            // and whether it closes itself.
            index += 1;
            let mut quote = None;
            while index < text.len() {
                let byte = text[index];
                match quote {
                    Some(open) if byte == open => quote = None,
                    Some(_) => {}
                    None if byte == b'"' || byte == b'\'' => quote = Some(byte),
                    None if byte == b'>' => {
                        if text[index - 1] == b'/' {
                            depth -= 1;
                        }
                        break;
                    }
                    None if byte == b'<' => break,
                    None => {}
                }
                index += 1;
            }
            if text.get(index) == Some(&b'>') {
                index += 1;
            }
        }
    }
    false
}

/// The drawing's own size in logical pixels, from its `width`/`height` or its
/// `viewBox`.
///
/// # Errors
/// [`SvgError::Parse`] for a source this build cannot read, and
/// [`SvgError::InvalidSize`] for one that declares no usable size.
pub fn intrinsic_size(source: &[u8]) -> Result<Size, SvgError> {
    let size = parse(source)?.size();
    Size::new(size.width(), size.height()).map_err(|_| SvgError::InvalidSize)
}

/// What a rasterization produces, which decides its size limit.
#[derive(Clone, Copy)]
enum Output {
    Color,
    Mask,
}

fn render(
    tree: &resvg::usvg::Tree,
    width: u32,
    height: u32,
    output: Output,
) -> Result<resvg::tiny_skia::Pixmap, SvgError> {
    if width == 0 || height == 0 {
        return Err(SvgError::InvalidSize);
    }
    // Checked before the pixmap exists: rendering first and refusing the copy
    // afterwards would allocate and fill the oversized pixmap anyway.
    let pixels = u64::from(width) * u64::from(height);
    let fits = match output {
        Output::Color => pixels <= RasterImage::MAX_BYTES as u64 / 4,
        Output::Mask => pixels <= MaskImage::MAX_PIXELS as u64,
    };
    if !fits {
        return Err(SvgError::TooLarge { width, height });
    }
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height).ok_or(SvgError::InvalidSize)?;
    let size = tree.size();
    if size.width() <= 0.0 || size.height() <= 0.0 {
        return Err(SvgError::InvalidSize);
    }
    // Uniform, so a drawing whose aspect ratio differs from the target keeps its
    // proportions instead of being stretched by the rasterizer.
    let scale = (width as f32 / size.width()).min(height as f32 / size.height());
    let x = (width as f32 - size.width() * scale) / 2.0;
    let y = (height as f32 - size.height() * scale) / 2.0;
    let transform = resvg::tiny_skia::Transform::from_row(scale, 0.0, 0.0, scale, x, y);
    resvg::render(tree, transform, &mut pixmap.as_mut());
    Ok(pixmap)
}

/// Rasterizes a drawing into a color image of exactly `width` by `height`
/// physical pixels.
///
/// The drawing keeps its aspect ratio and is centered, so a target whose
/// proportions differ leaves transparent margins. Rasterization takes time
/// proportional to the pixels; never call it in `paint`.
///
/// # Errors
/// [`SvgError::Parse`], [`SvgError::InvalidSize`] for a zero or unusable size,
/// and [`SvgError::TooLarge`], before anything is rendered, when the image would
/// exceed [`RasterImage::MAX_BYTES`].
pub fn rasterize(source: &[u8], width: u32, height: u32) -> Result<RasterImage, SvgError> {
    let pixmap = render(&parse(source)?, width, height, Output::Color)?;
    Ok(image_from(&pixmap)?)
}

/// Rasterizes a drawing into an 8-bit coverage mask of exactly `width` by
/// `height` physical pixels, keeping only how opaque each pixel is.
///
/// This is what a monochrome icon wants: the mask carries the shape and the
/// theme supplies the color at draw time, so one rasterization serves every
/// color role and every theme mode. Colors in the source are discarded.
///
/// # Errors
/// As [`rasterize`], except that [`SvgError::TooLarge`] follows the mask limit,
/// [`MaskImage::MAX_PIXELS`].
pub fn rasterize_mask(source: &[u8], width: u32, height: u32) -> Result<MaskImage, SvgError> {
    let pixmap = render(&parse(source)?, width, height, Output::Mask)?;
    Ok(mask_from(&pixmap)?)
}

fn image_from(pixmap: &resvg::tiny_skia::Pixmap) -> Result<RasterImage, DrawError> {
    // tiny-skia stores premultiplied pixels; RasterImage premultiplies itself.
    let mut pixels = Vec::with_capacity(pixmap.pixels().len() * 4);
    for pixel in pixmap.pixels() {
        let color = pixel.demultiply();
        pixels.extend_from_slice(&[color.red(), color.green(), color.blue(), color.alpha()]);
    }
    RasterImage::from_srgba8(pixmap.width(), pixmap.height(), pixels)
}

fn mask_from(pixmap: &resvg::tiny_skia::Pixmap) -> Result<MaskImage, DrawError> {
    let coverage = pixmap.pixels().iter().map(|pixel| pixel.alpha()).collect();
    MaskImage::new(pixmap.width(), pixmap.height(), coverage)
}
