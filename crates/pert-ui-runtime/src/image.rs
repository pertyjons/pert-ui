//! Presentation of a decoded color image inside the ordinary layout.
//!
//! The component only places and draws a [`RasterImage`]; it never decodes,
//! loads or scales pixels. Decoding belongs to whoever supplies the image, off
//! the paint path, and is available through the `pert-ui` facade's image
//! format features.
use crate::{
    ArrangeContext, Component, ComponentDetails, Constraints, DrawError, ImageSampling,
    InspectValue, LayoutError, MeasureContext, Painter, RasterImage, ReadSignal, Rect,
    RuntimeError, SemanticRole, Semantics, Size, UpdateContext,
};

/// How an [`Image`] fills the rectangle it is allocated when their aspect ratios
/// differ.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ImageFit {
    /// Scales the image uniformly until it touches the allocation on one axis,
    /// centered, so all of it is visible and the rest of the allocation stays
    /// empty. The default.
    #[default]
    Contain,
    /// Scales the image uniformly until it covers the whole allocation, centered,
    /// and clips what overflows, so nothing of the allocation stays empty.
    Cover,
    /// Stretches the image to the allocation on both axes, changing its aspect
    /// ratio when the allocation's differs.
    Fill,
}

/// A decoded color image placed in the layout.
///
/// By preference the component is as large as the image in logical pixels, one
/// per image pixel (see [`RasterImage::logical_size`]), or as large as an
/// explicit [`Image::size`]. Given less room, it shrinks uniformly to fit rather
/// than overflowing or distorting. Inside its allocation the image is placed by
/// its [`ImageFit`], [`ImageFit::Contain`] by default, and drawn with
/// [`ImageSampling::Linear`] unless [`Image::sampling`] selects another mode. It
/// is not focusable and handles no input.
///
/// A bound image follows a `ReadSignal<Option<RasterImage>>`, for example one
/// that a worker thread's decoded result is written to. `None` draws nothing and
/// keeps the explicit size, or no size at all without one. A change of image
/// repaints; it requests layout only when the component's preferred size depends
/// on the image and the image's pixel dimensions changed.
///
/// Inspection reports [`SemanticRole::Image`] with the [`Image::description`] as
/// its name, the text a person who cannot see the image would need, and the
/// pixel dimensions and fit as properties.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{DrawCommand, Element, Image, ImageFit, RasterImage, Runtime, Viewport};
/// let mut ui = Runtime::new();
/// ui.set_viewport(Viewport::new(100.0, 100.0, 1.0)?)?;
/// // A 4x2 opaque gray image. As the root it is allocated the whole 100x100
/// // window, and Contain scales it uniformly and centers it there.
/// let pixels = [128, 128, 128, 255].repeat(8);
/// let image = RasterImage::from_srgba8(4, 2, pixels)?;
/// ui.mount_tree(Element::new(
///     "logo",
///     Image::new(image)
///         .fit(ImageFit::Contain)
///         .description("Company logo"),
/// ))?;
/// ui.flush(std::time::Duration::ZERO)?;
/// let scene = ui.paint()?;
/// let DrawCommand::DrawImage { rect, .. } = scene.commands()[0] else { panic!() };
/// assert_eq!(rect.values(), [0.0, 25.0, 100.0, 50.0]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// Re-describing the image under the same key applies its pixels or binding,
/// fit, sampling, size and description; an unchanged description keeps its
/// prepared caches.
#[must_use]
pub struct Image {
    image: Option<RasterImage>,
    source: Option<ReadSignal<Option<RasterImage>>>,
    fit: ImageFit,
    sampling: ImageSampling,
    size: Option<Size>,
    description: String,
}
description_debug!(Image);
impl Image {
    /// Presents a fixed image.
    pub fn new(image: RasterImage) -> Self {
        Self {
            image: Some(image),
            ..Self::empty()
        }
    }
    /// Follows an application-owned image binding; `None` presents nothing.
    pub fn bound(source: impl Into<ReadSignal<Option<RasterImage>>>) -> Self {
        let source: ReadSignal<Option<RasterImage>> = source.into();
        Self {
            source: Some(source),
            ..Self::empty()
        }
    }
    fn empty() -> Self {
        Self {
            image: None,
            source: None,
            fit: ImageFit::default(),
            sampling: ImageSampling::default(),
            size: None,
            description: String::new(),
        }
    }
    /// Selects how the image fills its allocation; default [`ImageFit::Contain`].
    pub fn fit(mut self, fit: ImageFit) -> Self {
        self.fit = fit;
        self
    }
    /// Selects how pixels are filtered; default [`ImageSampling::Linear`].
    pub fn sampling(mut self, sampling: ImageSampling) -> Self {
        self.sampling = sampling;
        self
    }
    /// Prefers this logical size instead of the image's own. The fit still
    /// decides how the image occupies it, so a size of another aspect ratio does
    /// not distort the image unless the fit is [`ImageFit::Fill`].
    pub fn size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }
    /// Sets the text alternative: what the image shows, for a person or a tool
    /// that cannot see it. Leave it empty for a purely decorative image.
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = text.into();
        self
    }
    fn preferred(&self) -> Size {
        self.size
            .or_else(|| self.image.as_ref().map(RasterImage::logical_size))
            .unwrap_or(Size::ZERO)
    }
    // Where the image goes inside an allocation of `size`, before clipping.
    fn placement(&self, image: &RasterImage, size: Size) -> Result<Rect, crate::GeometryError> {
        let natural = image.logical_size();
        let (width, height) = (size.width(), size.height());
        let scale = match self.fit {
            ImageFit::Fill => return Ok(Rect::from_size(size)),
            ImageFit::Contain => (width / natural.width()).min(height / natural.height()),
            ImageFit::Cover => (width / natural.width()).max(height / natural.height()),
        };
        let (w, h) = (natural.width() * scale, natural.height() * scale);
        Rect::new((width - w) / 2.0, (height - h) / 2.0, w, h)
    }
}
impl Component for Image {
    /// Applies a new description: the fixed image or binding, fit, sampling,
    /// explicit size and description.
    fn reconcile(&mut self, next: Self) -> Option<crate::Invalidation> {
        let rebound = match (&self.source, &next.source) {
            (Some(old), Some(new)) => !old.same_source(new),
            (None, None) => false,
            _ => true,
        };
        let layout = self.size != next.size
            || (self.size.is_none()
                && (rebound
                    || (next.source.is_none()
                        && self.image.as_ref().map(|i| (i.width(), i.height()))
                            != next.image.as_ref().map(|i| (i.width(), i.height())))));
        let paint = rebound
            || (next.source.is_none() && self.image != next.image)
            || self.fit != next.fit
            || self.sampling != next.sampling
            || self.description != next.description;
        if rebound || next.source.is_none() {
            // A bound image is read again by the next update; a fixed one is
            // the description's.
            self.image = next.image;
        }
        self.source = next.source;
        self.fit = next.fit;
        self.sampling = next.sampling;
        self.size = next.size;
        self.description = next.description;
        if layout {
            Some(crate::Invalidation::Layout)
        } else {
            paint.then_some(crate::Invalidation::Paint)
        }
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        if let Some(source) = &self.source {
            let image = cx.read_paint(source)?;
            let dimensions = |image: &Option<RasterImage>| {
                image.as_ref().map(|image| (image.width(), image.height()))
            };
            if self.size.is_none() && dimensions(&image) != dimensions(&self.image) {
                cx.request_layout();
            }
            self.image = image;
        }
        Ok(())
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
    fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        if cx.is_empty() {
            Ok(())
        } else {
            Err(LayoutError::InvalidChild.into())
        }
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let (size, Some(image)) = (painter.size(), &self.image) else {
            return Ok(());
        };
        if size.width() == 0.0 || size.height() == 0.0 {
            return Ok(());
        }
        let rect = self.placement(image, size)?;
        if self.fit == ImageFit::Cover {
            painter.with_clip(Rect::from_size(size), |painter| {
                painter.draw_image(image, rect, self.sampling);
            });
        } else {
            painter.draw_image(image, rect, self.sampling);
        }
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        let fit = match self.fit {
            ImageFit::Contain => "contain",
            ImageFit::Cover => "cover",
            ImageFit::Fill => "fill",
        };
        details
            .properties
            .insert("fit".into(), InspectValue::Text(fit.into()));
        if let Some(image) = &self.image {
            details.properties.insert(
                "pixels".into(),
                InspectValue::Text(format!("{}x{}", image.width(), image.height())),
            );
        }
        details.semantics = Some(Semantics::new(SemanticRole::Image).name(&*self.description));
        details
    }
}
