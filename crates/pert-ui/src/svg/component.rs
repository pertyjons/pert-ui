//! The [`Svg`] component, re-exported at the crate root.
use super::{Output, SvgError, image_from, mask_from, parse, render};
use pert_ui_runtime::{
    ArrangeContext, ColorRole, Component, ComponentDetails, Constraints, DrawError, InspectValue,
    LayoutError, MaskImage, MeasureContext, Painter, RasterImage, Rect, RuntimeError, SemanticRole,
    Semantics, Size,
};
use std::cell::RefCell;

/// A drawing placed in the ordinary layout and rasterized for the size it is
/// given.
///
/// By preference the component is as large as the drawing's own size in logical
/// pixels, or as large as an explicit [`Svg::size`]. Given less room it shrinks
/// uniformly rather than overflowing or distorting, and inside its allocation
/// the drawing keeps its aspect ratio and is centered.
///
/// Rasterization happens during layout at [`MeasureContext::raster_scale`],
/// including accumulated ancestor view magnification and its preparation cap.
/// Continuous zoom stretches the existing raster until the view settles, then
/// prepares it at the settled density. A resize or display scale change prepares
/// new coverage when physical dimensions change; panning and tint changes reuse
/// it. The component keeps one
/// rasterization, never a history of the sizes it has had. It is not focusable
/// and handles no input.
///
/// [`Svg::tinted`] keeps only coverage and lets a [`ColorRole`] color it, which
/// is what a monochrome icon wants: one rasterization serves every color role
/// and both theme modes, and it shares the mask table with text and shapes
/// instead of holding an opaque bitmap of its own. Without it the drawing keeps
/// its own colors as an image.
///
/// Inspection reports [`SemanticRole::Image`] with the [`Svg::description`] as
/// its name, the rasterized pixel dimensions, whether it is tinted, and the
/// message of a rasterization that failed after construction.
///
/// Re-describing the image under the same key applies its source, tint, size
/// and description; the same source keeps its parsed tree and prepared raster,
/// and a changed source or a switch between tinted and full color prepares it
/// again.
#[must_use]
pub struct Svg {
    tree: resvg::usvg::Tree,
    key: u64,
    /// The source the tree was parsed from, compared when two keys match so a
    /// hash collision cannot keep a different drawing.
    source: Box<[u8]>,
    intrinsic: Size,
    size: Option<Size>,
    tint: Option<ColorRole>,
    description: Option<String>,
    raster: RefCell<Option<Raster>>,
    failure: RefCell<Option<String>>,
}

struct Raster {
    width: u32,
    height: u32,
    content: Content,
}

enum Content {
    Color(Box<RasterImage>),
    Mask(Box<MaskImage>),
}

impl Svg {
    /// Reads a drawing, keeping the colors it declares.
    ///
    /// The source is parsed once, here, so a malformed drawing is reported
    /// before anything is mounted rather than failing silently during layout.
    ///
    /// # Errors
    /// [`SvgError::Parse`] for a source this build cannot read, and
    /// [`SvgError::InvalidSize`] for one that declares no usable size.
    pub fn new(source: &[u8]) -> Result<Self, SvgError> {
        let tree = parse(source)?;
        let size = tree.size();
        let intrinsic =
            Size::new(size.width(), size.height()).map_err(|_| SvgError::InvalidSize)?;
        Ok(Self {
            tree,
            key: content_key(source),
            source: source.into(),
            intrinsic,
            size: None,
            tint: None,
            description: None,
            raster: RefCell::new(None),
            failure: RefCell::new(None),
        })
    }
    /// Reads a drawing as a coverage mask that the theme's `role` colors.
    ///
    /// # Errors
    /// As [`Svg::new`].
    pub fn tinted(source: &[u8], role: ColorRole) -> Result<Self, SvgError> {
        Ok(Self {
            tint: Some(role),
            ..Self::new(source)?
        })
    }
    /// Sets the preferred logical size, instead of the drawing's own.
    pub const fn size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }
    /// Describes the drawing for inspection and later accessibility, as the text
    /// a person who cannot see it would need.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
    /// The drawing's own logical size, before any [`Svg::size`].
    pub const fn intrinsic_size(&self) -> Size {
        self.intrinsic
    }
    fn preferred(&self) -> Size {
        self.size.unwrap_or(self.intrinsic)
    }
    /// The aspect-preserving rectangle the drawing occupies inside `allocation`.
    fn placement(&self, allocation: Size) -> Result<Rect, crate::GeometryError> {
        let preferred = self.preferred();
        let scale =
            (allocation.width() / preferred.width()).min(allocation.height() / preferred.height());
        let (width, height) = (preferred.width() * scale, preferred.height() * scale);
        Rect::new(
            (allocation.width() - width) / 2.0,
            (allocation.height() - height) / 2.0,
            width,
            height,
        )
    }
}

fn content_key(source: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    source.hash(&mut hasher);
    hasher.finish()
}

impl Component for Svg {
    /// Nothing follows a binding, so an update has nothing to collect: the
    /// drawing changes through reconciliation, and its pixels through layout.
    fn update(&mut self, _: &mut pert_ui_runtime::UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    /// Replaces the drawing and its presentation without remounting. A source
    /// with the same content keeps the rasterization it already has.
    fn reconcile(&mut self, next: Self) -> Option<pert_ui_runtime::Invalidation> {
        // The key rejects most differing sources cheaply; equal keys still
        // compare the bytes, since a 64-bit hash can collide.
        let source_changed = self.key != next.key || self.source != next.source;
        // A mask and a color image are different rasterizations, which only
        // layout prepares; a new role for the same mask is a repaint.
        let output_changed = self.tint.is_some() != next.tint.is_some();
        let layout_changed = source_changed || output_changed || self.size != next.size;
        if source_changed || output_changed {
            self.raster.replace(None);
            self.failure.replace(None);
        }
        if source_changed {
            self.tree = next.tree;
            self.key = next.key;
            self.source = next.source;
            self.intrinsic = next.intrinsic;
        }
        let paint_changed = self.tint != next.tint || self.description != next.description;
        self.size = next.size;
        self.tint = next.tint;
        self.description = next.description;
        match (layout_changed, paint_changed) {
            (true, _) => Some(pert_ui_runtime::Invalidation::Layout),
            (false, true) => Some(pert_ui_runtime::Invalidation::Paint),
            (false, false) => None,
        }
    }
    fn measure(
        &self,
        _: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        let preferred = self.preferred();
        let limit = |available: Option<f32>, preferred: f32| {
            available
                .filter(|&available| preferred > available)
                .map_or(1.0, |available| available / preferred)
        };
        let scale = limit(available.width(), preferred.width())
            .min(limit(available.height(), preferred.height()));
        Ok(Size::new(
            preferred.width() * scale,
            preferred.height() * scale,
        )?)
    }
    /// Rasterizes for the physical size this allocation covers, which is where
    /// the work belongs: layout knows the size and the scale, and `paint` may
    /// not rasterize.
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        if !cx.is_empty() {
            return Err(LayoutError::InvalidChild.into());
        }
        let scale = cx.measurement().raster_scale();
        let Ok(rect) = self.placement(size) else {
            self.raster.replace(None);
            return Ok(());
        };
        let physical = |extent: f32| (f64::from(extent) * scale).round().max(0.0) as u32;
        let (width, height) = (physical(rect.width()), physical(rect.height()));
        if width == 0 || height == 0 {
            self.raster.replace(None);
            return Ok(());
        }
        if self
            .raster
            .borrow()
            .as_ref()
            .is_some_and(|raster| raster.width == width && raster.height == height)
        {
            return Ok(());
        }
        let output = match self.tint {
            Some(_) => Output::Mask,
            None => Output::Color,
        };
        let rasterized = render(&self.tree, width, height, output).and_then(|pixmap| {
            Ok(match output {
                Output::Mask => Content::Mask(Box::new(mask_from(&pixmap)?)),
                Output::Color => Content::Color(Box::new(image_from(&pixmap)?)),
            })
        });
        match rasterized {
            Ok(content) => {
                self.failure.replace(None);
                self.raster.replace(Some(Raster {
                    width,
                    height,
                    content,
                }));
            }
            // A failure here is a resource limit at this size, not a broken
            // drawing: the component draws nothing and inspection keeps why.
            Err(error) => {
                self.raster.replace(None);
                self.failure.replace(Some(error.to_string()));
            }
        }
        Ok(())
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let borrowed = self.raster.borrow();
        let (Some(size), Some(raster)) = (painter.size(), borrowed.as_ref()) else {
            return Ok(());
        };
        let rect = self.placement(size)?;
        match (&raster.content, self.tint) {
            (Content::Mask(mask), Some(role)) => {
                painter.draw_mask(mask, rect, painter.color(role));
            }
            (Content::Color(image), _) => {
                painter.draw_image(image, rect, pert_ui_runtime::ImageSampling::Linear);
            }
            (Content::Mask(_), None) => {}
        }
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        let mut semantics = Semantics::new(SemanticRole::Image);
        if let Some(description) = &self.description {
            semantics = semantics.name(description.clone());
        }
        details.semantics = Some(semantics);
        if let Some(raster) = self.raster.borrow().as_ref() {
            details.properties.insert(
                "pixel_width".into(),
                InspectValue::Number(f64::from(raster.width)),
            );
            details.properties.insert(
                "pixel_height".into(),
                InspectValue::Number(f64::from(raster.height)),
            );
        }
        details
            .properties
            .insert("tinted".into(), InspectValue::Bool(self.tint.is_some()));
        if let Some(failure) = self.failure.borrow().as_ref() {
            details
                .properties
                .insert("failure".into(), InspectValue::Text(failure.clone()));
        }
        details
    }
}

#[cfg(test)]
mod tests {
    use super::Svg;
    use pert_ui_runtime::{Component, Invalidation};

    const SQUARE: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8"><rect width="8" height="8"/></svg>"#;
    const WIDE: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="8"><rect width="16" height="8"/></svg>"#;

    #[test]
    fn a_different_source_with_a_colliding_key_replaces_the_drawing() -> Result<(), super::SvgError>
    {
        let mut current = Svg::new(SQUARE)?;
        let mut next = Svg::new(WIDE)?;
        // Simulate a 64-bit hash collision between two different sources.
        next.key = current.key;
        assert_eq!(current.reconcile(next), Some(Invalidation::Layout));
        assert_eq!(&*current.source, WIDE);
        assert_eq!(current.intrinsic_size().width(), 16.0);
        Ok(())
    }

    #[test]
    fn the_same_source_keeps_the_drawing_without_invalidation() -> Result<(), super::SvgError> {
        let mut current = Svg::new(SQUARE)?;
        assert_eq!(current.reconcile(Svg::new(SQUARE)?), None);
        Ok(())
    }
}
