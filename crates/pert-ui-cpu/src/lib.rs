#![doc = include_str!("../README.md")]

use pert_ui_runtime::host::HostIntegration;
use pert_ui_runtime::{
    CaptureError, Color, DisplayList, DrawCommand, DrawError, FrameImage, FrameMapping,
    ImageSampling, LimitError, MaskImage, Paint, PreparedShape, QuarterTurn, RasterImage, Runtime,
    ShapeCoverage, ShapePrepareError, VectorRasterizer,
};

mod srgb;

/// Why a scene could not be rendered.
///
/// Every refusal happens before any pixel is written, so the renderer stays
/// usable for the next scene.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum CpuRenderError {
    /// Preparing the scene's vectors failed, with the errors of
    /// [`VectorRasterizer::prepare`].
    #[error(transparent)]
    Drawing(#[from] DrawError),
    /// The viewport has no physical pixels, as a minimized window does.
    #[error("the viewport has no physical pixels")]
    EmptyViewport,
    /// The frame would hold more than [`FrameImage::MAX_BYTES`]:
    /// [`LimitError::FrameSize`] with the frame's dimensions, as every
    /// renderer reports that limit.
    #[error(transparent)]
    Limit(LimitError),
    /// The frame fits [`FrameImage::MAX_BYTES`], but the system could not
    /// allocate its pixel buffer of the bytes held here.
    #[error("the {bytes}-byte frame buffer could not be allocated")]
    AllocationFailed {
        /// The bytes the frame's pixels need.
        bytes: u64,
    },
    /// A shape command could not be prepared for the frame: its geometry
    /// lies outside the range this renderer evaluates, or it is malformed,
    /// as a border on an arc or a triangle is.
    #[error(transparent)]
    Shape(#[from] ShapePrepareError),
    /// A command names a mask or image that its scene does not hold, as an ID
    /// copied from another scene would.
    #[error("a command refers to a resource missing from its scene")]
    InvalidReference,
    /// The scene uses a drawing command or sampling mode this renderer does
    /// not know, added to the runtime after it.
    #[error("the scene uses a drawing feature this renderer does not know")]
    Unsupported,
}

impl From<CpuRenderError> for CaptureError {
    /// Drawing failures and a shape that cannot be prepared keep their typed
    /// cause, an oversized frame is [`CaptureError::Limit`] with the same
    /// limit, an empty viewport has no surface to render and the remaining
    /// renderer diagnostics, a failed allocation included, are backend text.
    fn from(error: CpuRenderError) -> Self {
        match error {
            CpuRenderError::Drawing(error) => error.into(),
            CpuRenderError::Limit(limit) => Self::Limit(limit),
            CpuRenderError::Shape(error) => Self::Shape(error),
            CpuRenderError::EmptyViewport => Self::SurfaceUnavailable,
            error => Self::Backend(error.to_string()),
        }
    }
}

/// A retained CPU renderer of [`DisplayList`]s into [`FrameImage`]s.
///
/// It needs no window, GPU, display server or worker thread, so a test, a
/// headless tool or an inspection client can render the same scenes the
/// native renderer presents. Keep one renderer for a sequence of frames: it
/// retains the vector coverage of unchanged geometry between them.
///
/// **Rendering model.** The frame has the scene's
/// [`Viewport::frame_extent`](pert_ui_runtime::Viewport::frame_extent) and starts
/// as the background, premultiplied in linear light. Commands draw in order with
/// source-over blending of linear, premultiplied color, and each command's
/// result is stored as 8-bit sRGB-encoded premultiplied RGBA before the next
/// one blends, as an sRGB attachment does. A command covers the physical
/// pixels whose centers lie inside its clipped rectangle, a center on the left
/// or top edge included and one on the right or bottom edge excluded. Masks and
/// images are sampled at each pixel center through their quarter turn with
/// clamp-to-edge addressing: bilinearly for masks and linear images, and from the
/// nearest texel for nearest sampling. Image texels are decoded from sRGB to
/// linear light before they are filtered. The rendering contract in
/// `docs/rendering-platform.md` records these rules and the tolerances against
/// the GPU renderer.
///
/// **Resources.** A frame holds at most [`FrameImage::MAX_BYTES`]; that is the
/// whole working storage, besides the vector masks the rasterizer bounds.
///
/// ```
/// # mod pert_ui { pub use pert_ui_runtime::*; pub mod cpu { pub use pert_ui_cpu::CpuRenderer; } }
/// use pert_ui::cpu::CpuRenderer;
/// use pert_ui::{Color, Component, DrawError, Element, Painter, Rect, Runtime,
///     RuntimeError, UpdateContext, Viewport};
/// use std::time::Duration;
///
/// struct Swatch;
/// impl Component for Swatch {
///     fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
///         Ok(())
///     }
///     fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
///         painter.fill_rect(Rect::new(0.0, 0.0, 2.0, 1.0)?, Color::WHITE);
///         Ok(())
///     }
/// }
///
/// let mut ui = Runtime::new();
/// ui.set_viewport(Viewport::new(4.0, 2.0, 1.0)?)?;
/// ui.mount_tree(Element::new("swatch", Swatch))?;
/// ui.flush(Duration::ZERO)?;
/// let frame = CpuRenderer::new().render(&ui.paint()?)?;
/// assert_eq!((frame.width(), frame.height()), (4, 2));
/// assert_eq!(&frame.pixels()[..4], &[255, 255, 255, 255]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Default)]
pub struct CpuRenderer {
    vectors: VectorRasterizer,
}

impl std::fmt::Debug for CpuRenderer {
    /// Shows the renderer without its retained coverage.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CpuRenderer")
            .finish_non_exhaustive()
    }
}

/// What one command draws: a solid fill, a tinted mask or a tinted image.
enum Source<'a> {
    Fill,
    Mask(&'a MaskImage),
    Image(&'a RasterImage, ImageSampling),
}

impl CpuRenderer {
    /// Creates a renderer with no retained vector coverage.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Renders `scene` into a frame of its viewport's physical extent.
    ///
    /// # Errors
    /// [`CpuRenderError::EmptyViewport`] for a viewport without physical pixels,
    /// [`CpuRenderError::Limit`] with [`LimitError::FrameSize`] beyond
    /// [`FrameImage::MAX_BYTES`], [`CpuRenderError::AllocationFailed`] when the
    /// frame's pixel buffer cannot be allocated,
    /// [`CpuRenderError::Drawing`] when vector preparation fails,
    /// [`CpuRenderError::InvalidReference`] for a command naming a resource its
    /// scene lacks and [`CpuRenderError::Unsupported`] for a command or sampling
    /// mode this renderer does not know. The retained vector coverage then stays
    /// as the last successful scene left it.
    pub fn render(&mut self, scene: &DisplayList) -> Result<FrameImage, CpuRenderError> {
        self.render_frame(scene, false)
    }

    /// Renders `scene` as an opaque native surface shows it: the frame is
    /// cleared with the background's RGB at full alpha instead of
    /// premultiplied by its alpha, so every pixel is opaque. This is what a
    /// window presents; [`Self::render`] is what a capture keeps.
    ///
    /// ```
    /// # mod pert_ui { pub use pert_ui_runtime::*; pub mod cpu { pub use pert_ui_cpu::CpuRenderer; } }
    /// use pert_ui::cpu::CpuRenderer;
    /// use pert_ui::{Runtime, Viewport};
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(4.0, 4.0, 1.0)?)?;
    /// let frame = CpuRenderer::new().render_opaque(&ui.paint()?)?;
    /// assert!(frame.pixels().chunks(4).all(|pixel| pixel[3] == 255));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// As for [`Self::render`].
    pub fn render_opaque(&mut self, scene: &DisplayList) -> Result<FrameImage, CpuRenderError> {
        self.render_frame(scene, true)
    }

    fn render_frame(
        &mut self,
        scene: &DisplayList,
        opaque: bool,
    ) -> Result<FrameImage, CpuRenderError> {
        let viewport = scene.viewport();
        let extent = viewport.frame_extent();
        let (width, height) = (extent.width(), extent.height());
        if width == 0 || height == 0 || viewport.width() <= 0.0 || viewport.height() <= 0.0 {
            return Err(CpuRenderError::EmptyViewport);
        }
        let bytes = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|pixels| pixels.checked_mul(4))
            .filter(|bytes| *bytes <= FrameImage::MAX_BYTES as u64)
            .ok_or(CpuRenderError::Limit(LimitError::FrameSize {
                width,
                height,
            }))?;
        let mapping = FrameMapping::new(width, height, viewport)
            .map_err(|_| CpuRenderError::EmptyViewport)?;
        // Validate every command before preparing or allocating anything.
        for command in scene.commands() {
            check(scene, command, &mapping)?;
        }
        let scene = self.vectors.prepare(scene)?;
        let background = scene.background();
        let clear = encoded(if opaque {
            [background.red(), background.green(), background.blue(), 1.0]
        } else {
            premultiplied(background)
        });
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(bytes as usize)
            .map_err(|_| CpuRenderError::AllocationFailed { bytes })?;
        for _ in 0..u64::from(width) * u64::from(height) {
            pixels.extend_from_slice(&clear);
        }
        let mut frame = Frame {
            pixels,
            width: width as usize,
            height: height as usize,
            scale_x: f64::from(width) / f64::from(viewport.width()),
            scale_y: f64::from(height) / f64::from(viewport.height()),
            logical: (f64::from(viewport.width()), f64::from(viewport.height())),
            mapping,
        };
        for command in scene.commands() {
            frame.draw(&scene, command)?;
        }
        FrameImage::new(width, height, frame.pixels).map_err(|error| match error {
            CaptureError::Limit(limit) => CpuRenderError::Limit(limit),
            // The buffer has exactly the size its dimensions need.
            _ => CpuRenderError::Unsupported,
        })
    }

    /// Completes every capture job `runtime` lists with a frame rendered here,
    /// or with the error that prevented it, and returns how many it completed.
    ///
    /// This makes the renderer the runtime's capture host: declare it once with
    /// [`Runtime::set_capture_support`] and call this after each flush, as a
    /// native host does after each frame. Window, component, path and region
    /// requests, including those an
    /// [`InspectionClient`](pert_ui_runtime::InspectionClient) sends from
    /// another thread, are answered with real pixels and no display or GPU.
    ///
    /// ```
    /// # mod pert_ui { pub use pert_ui_runtime::*; pub mod cpu { pub use pert_ui_cpu::CpuRenderer; } }
    /// use pert_ui::cpu::CpuRenderer;
    /// use pert_ui::{CaptureRequest, CaptureTarget, Element, Label, Runtime, Viewport, WaitError};
    /// use std::time::Duration;
    ///
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(200.0, 80.0, 2.0)?)?;
    /// ui.mount_tree(Element::new("title", Label::new("Mixer")))?;
    /// ui.flush(Duration::ZERO)?;
    /// ui.set_capture_support(true);
    /// let pending = ui.request_capture(CaptureRequest::new(CaptureTarget::Path(vec!["title".into()])));
    /// ui.flush(Duration::ZERO)?;
    /// assert!(matches!(pending.try_reply(), Err(WaitError::Timeout))); // queued, not served
    /// let mut renderer = CpuRenderer::new();
    /// assert_eq!(renderer.serve_captures(&mut ui), 1);
    /// let capture = pending.try_reply()?.value?; // answered by `serve_captures`
    /// assert_eq!(capture.scale, 2.0);
    /// assert!(capture.image.encode_png().starts_with(b"\x89PNG"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn serve_captures(&mut self, runtime: &mut Runtime) -> usize {
        let mut completed = 0;
        for job in runtime.pending_captures() {
            let frame = runtime
                .capture_scene(&job)
                .and_then(|scene| self.render(&scene).map_err(CaptureError::from));
            if runtime.complete_capture(&job, frame) {
                completed += 1;
            }
        }
        completed
    }
}

/// Rejects a command this renderer cannot draw or whose resource is missing.
fn check(
    scene: &DisplayList,
    command: &DrawCommand,
    mapping: &FrameMapping,
) -> Result<(), CpuRenderError> {
    match command {
        DrawCommand::FillRect { .. } => Ok(()),
        DrawCommand::DrawShape {
            geometry,
            fill,
            border,
            clip,
            ..
        } => {
            fill.map(solid).transpose()?;
            border.map(|border| solid(border.paint())).transpose()?;
            PreparedShape::new(geometry, border.as_ref(), *clip, mapping)?;
            Ok(())
        }
        DrawCommand::DrawMask { mask, .. } => scene
            .mask(*mask)
            .map(|_| ())
            .ok_or(CpuRenderError::InvalidReference),
        DrawCommand::DrawImage {
            image, sampling, ..
        } => {
            scene
                .image(*image)
                .ok_or(CpuRenderError::InvalidReference)?;
            match sampling {
                ImageSampling::Linear | ImageSampling::Nearest => Ok(()),
                _ => Err(CpuRenderError::Unsupported),
            }
        }
        DrawCommand::DrawVector { vector, .. } => scene
            .vector(*vector)
            .map(|_| ())
            .ok_or(CpuRenderError::InvalidReference),
        _ => Err(CpuRenderError::Unsupported),
    }
}

/// The premultiplied color of a paint this renderer knows.
fn solid(paint: Paint) -> Result<[f32; 4], CpuRenderError> {
    match paint {
        Paint::Solid(color) => Ok(premultiplied(color)),
        _ => Err(CpuRenderError::Unsupported),
    }
}

/// A straight linear color premultiplied by its alpha.
fn premultiplied(color: Color) -> [f32; 4] {
    let [red, green, blue, alpha] = color.channels();
    [red * alpha, green * alpha, blue * alpha, alpha]
}

/// Linear premultiplied color stored as sRGB-encoded premultiplied bytes.
fn encoded([red, green, blue, alpha]: [f32; 4]) -> [u8; 4] {
    [
        srgb::encode(red),
        srgb::encode(green),
        srgb::encode(blue),
        srgb::alpha(alpha),
    ]
}

/// The last blend's stored bytes, source and result.
#[derive(Clone, Copy)]
struct Blend {
    stored: [u8; 4],
    source: [f32; 4],
    result: [u8; 4],
}

/// The frame being drawn and its logical-to-physical mapping.
struct Frame {
    pixels: Vec<u8>,
    width: usize,
    height: usize,
    scale_x: f64,
    scale_y: f64,
    logical: (f64, f64),
    mapping: FrameMapping,
}

impl Frame {
    /// Draws a shape from its prepared coverage: with the silhouette's
    /// coverage `c_o`, the border's share `c_b` and premultiplied paints `F`
    /// and `B`, the source is `B·c_b + F·(c_o − B.a·c_b)`, blended once.
    fn draw_shape(&mut self, shape: &PreparedShape, fill: [f32; 4], border: [f32; 4]) {
        let source = |coverage: ShapeCoverage| -> [f32; 4] {
            let (outer, band) = (coverage.outer(), coverage.border());
            let inside = outer - border[3] * band;
            std::array::from_fn(|channel| border[channel] * band + fill[channel] * inside)
        };
        let [x0, y0, x1, y1] = shape.span();
        let mut memo = None;
        for row in y0..y1 {
            // The solid run inside a rectangle has one source, so it is
            // filled as a run; only the pixels outside it evaluate the
            // distance.
            let [first, past] = shape.solid_run(row).unwrap_or([x1, x1]);
            for column in (x0..first).chain(past..x1) {
                let coverage = shape.coverage(column, row);
                if coverage.outer() > 0.0 {
                    let column = column as usize;
                    self.fill_run(
                        row as usize,
                        column..column + 1,
                        source(coverage),
                        &mut memo,
                    );
                }
            }
            if past > first {
                let source = source(shape.coverage(first, row));
                self.fill_run(
                    row as usize,
                    first as usize..past as usize,
                    source,
                    &mut memo,
                );
            }
        }
    }

    /// Source-over of one linear premultiplied `source` onto the pixels
    /// `columns` of `row`. A source that adds nothing leaves them unread, an
    /// opaque one replaces them unread, and any other is blended, reusing
    /// `memo`'s result while the source and the stored bytes repeat, as they
    /// do across a uniform background.
    fn fill_run(
        &mut self,
        row: usize,
        columns: std::ops::Range<usize>,
        source: [f32; 4],
        memo: &mut Option<Blend>,
    ) {
        if source.iter().all(|channel| *channel <= 0.0) {
            return;
        }
        let start = (row * self.width + columns.start) * 4;
        let pixels = &mut self.pixels[start..start + columns.len() * 4];
        if source[3] >= 1.0 {
            // Nothing of the destination remains: `blended` would return
            // these bytes for any stored pixel.
            let bytes = encoded(source);
            for pixel in pixels.as_chunks_mut::<4>().0 {
                *pixel = bytes;
            }
            return;
        }
        for pixel in pixels.as_chunks_mut::<4>().0 {
            let stored = *pixel;
            let result = match *memo {
                Some(Blend {
                    stored: before,
                    source: with,
                    result,
                }) if before == stored && with == source => result,
                _ => {
                    let result = blended(stored, source);
                    *memo = Some(Blend {
                        stored,
                        source,
                        result,
                    });
                    result
                }
            };
            *pixel = result;
        }
    }

    fn draw(&mut self, scene: &DisplayList, command: &DrawCommand) -> Result<(), CpuRenderError> {
        if let DrawCommand::DrawShape {
            geometry,
            fill,
            border,
            clip,
            ..
        } = command
        {
            let fill = fill.map(solid).transpose()?.unwrap_or([0.0; 4]);
            let paint = border
                .map(|border| solid(border.paint()))
                .transpose()?
                .unwrap_or([0.0; 4]);
            if fill[3] == 0.0 && paint[3] == 0.0 {
                return Ok(());
            }
            if let Some(shape) =
                PreparedShape::new(geometry, border.as_ref(), *clip, &self.mapping)?
            {
                self.draw_shape(&shape, fill, paint);
            }
            return Ok(());
        }
        let (rect, color, clip, source, turn) = match *command {
            DrawCommand::FillRect { rect, color } => {
                (rect, color, None, Source::Fill, QuarterTurn::None)
            }
            DrawCommand::DrawMask {
                rect,
                mask,
                color,
                turn,
                clip,
            } => {
                let mask = scene.mask(mask).ok_or(CpuRenderError::InvalidReference)?;
                (rect, color, clip, Source::Mask(mask), turn)
            }
            DrawCommand::DrawImage {
                rect,
                image,
                tint,
                sampling,
                turn,
                clip,
            } => {
                let image = scene.image(image).ok_or(CpuRenderError::InvalidReference)?;
                (rect, tint, clip, Source::Image(image, sampling), turn)
            }
            // Prepared scenes hold no vectors; anything else was refused.
            _ => return Err(CpuRenderError::Unsupported),
        };
        let [x, y, w, h] = rect.values().map(f64::from);
        let (width, height) = self.logical;
        let [cx, cy, cw, ch] = clip.map_or([0.0, 0.0, width, height], |clip| {
            clip.values().map(f64::from)
        });
        let left = x.max(0.0).max(cx);
        let right = (x + w).min(width).min(cx + cw);
        let top = y.max(0.0).max(cy);
        let bottom = (y + h).min(height).min(cy + ch);
        if right <= left || bottom <= top || color.alpha() == 0.0 {
            return Ok(());
        }
        // Pixel `i` is covered when its center `i + 0.5` lies in the half-open
        // physical span: a center on the leading edge is in, one on the
        // trailing edge out.
        let span = |start: f64, end: f64, scale: f64, limit: usize| {
            let first = (start * scale - 0.5).ceil().clamp(0.0, limit as f64) as usize;
            let past = (end * scale - 0.5).ceil().clamp(0.0, limit as f64) as usize;
            first..past
        };
        let columns = span(left, right, self.scale_x, self.width);
        let rows = span(top, bottom, self.scale_y, self.height);
        let tint = premultiplied(color);
        if matches!(source, Source::Fill) {
            let mut memo = None;
            for row in rows {
                self.fill_run(row, columns.clone(), tint, &mut memo);
            }
            return Ok(());
        }
        for row in rows {
            let center_y = (row as f64 + 0.5) / self.scale_y;
            let v = ((center_y - y) / h) as f32;
            for column in columns.clone() {
                let source_color = match &source {
                    Source::Fill => tint,
                    Source::Mask(mask) => {
                        let center_x = (column as f64 + 0.5) / self.scale_x;
                        let u = ((center_x - x) / w) as f32;
                        let [su, sv] = turn.source(u, v);
                        let coverage = sample_mask(mask, su, sv);
                        tint.map(|channel| channel * coverage)
                    }
                    Source::Image(image, sampling) => {
                        let center_x = (column as f64 + 0.5) / self.scale_x;
                        let u = ((center_x - x) / w) as f32;
                        let [su, sv] = turn.source(u, v);
                        let texel = sample_image(image, *sampling, su, sv);
                        [
                            texel[0] * tint[0],
                            texel[1] * tint[1],
                            texel[2] * tint[2],
                            texel[3] * tint[3],
                        ]
                    }
                };
                self.blend(row * self.width + column, source_color);
            }
        }
        Ok(())
    }

    /// Source-over of linear premultiplied `source` onto one stored pixel.
    fn blend(&mut self, pixel: usize, source: [f32; 4]) {
        if source[3] <= 0.0 && source[..3].iter().all(|channel| *channel <= 0.0) {
            return;
        }
        let stored = &mut self.pixels[pixel * 4..pixel * 4 + 4];
        let result = blended([stored[0], stored[1], stored[2], stored[3]], source);
        stored.copy_from_slice(&result);
    }
}

/// Source-over of linear premultiplied `source` onto stored sRGB-encoded
/// premultiplied bytes, stored again.
fn blended(stored: [u8; 4], source: [f32; 4]) -> [u8; 4] {
    let keep = 1.0 - source[3];
    let destination = [
        srgb::decode(stored[0]),
        srgb::decode(stored[1]),
        srgb::decode(stored[2]),
        f32::from(stored[3]) / 255.0,
    ];
    encoded([
        source[0] + destination[0] * keep,
        source[1] + destination[1] * keep,
        source[2] + destination[2] * keep,
        source[3] + destination[3] * keep,
    ])
}

/// The texel coordinates and weights of bilinear filtering at `(u, v)` with
/// clamp-to-edge addressing: two columns, two rows and their fractions.
fn bilinear(width: u32, height: u32, u: f32, v: f32) -> ([usize; 2], [usize; 2], f32, f32) {
    let axis = |position: f32, size: u32| {
        let texel = position * size as f32 - 0.5;
        let floor = texel.floor();
        let fraction = texel - floor;
        let last = size as f32 - 1.0;
        let first = floor.clamp(0.0, last) as usize;
        let second = (floor + 1.0).clamp(0.0, last) as usize;
        ([first, second], fraction)
    };
    let (columns, fx) = axis(u, width);
    let (rows, fy) = axis(v, height);
    (columns, rows, fx, fy)
}

/// Linear coverage of `mask` at `(u, v)`, filtered bilinearly.
fn sample_mask(mask: &MaskImage, u: f32, v: f32) -> f32 {
    let (width, height) = (mask.width(), mask.height());
    let pixels = mask.pixels();
    let (columns, rows, fx, fy) = bilinear(width, height, u, v);
    let at = |column: usize, row: usize| f32::from(pixels[row * width as usize + column]) / 255.0;
    let top = at(columns[0], rows[0]) * (1.0 - fx) + at(columns[1], rows[0]) * fx;
    let bottom = at(columns[0], rows[1]) * (1.0 - fx) + at(columns[1], rows[1]) * fx;
    top * (1.0 - fy) + bottom * fy
}

/// Linear premultiplied color of `image` at `(u, v)`.
fn sample_image(image: &RasterImage, sampling: ImageSampling, u: f32, v: f32) -> [f32; 4] {
    let (width, height) = (image.width(), image.height());
    let pixels = image.pixels();
    let texel = |column: usize, row: usize| {
        let start = (row * width as usize + column) * 4;
        let bytes = &pixels[start..start + 4];
        [
            srgb::decode(bytes[0]),
            srgb::decode(bytes[1]),
            srgb::decode(bytes[2]),
            f32::from(bytes[3]) / 255.0,
        ]
    };
    if sampling == ImageSampling::Nearest {
        let index = |position: f32, size: u32| {
            (position * size as f32)
                .floor()
                .clamp(0.0, size as f32 - 1.0) as usize
        };
        return texel(index(u, width), index(v, height));
    }
    let (columns, rows, fx, fy) = bilinear(width, height, u, v);
    let [a, b, c, d] = [
        texel(columns[0], rows[0]),
        texel(columns[1], rows[0]),
        texel(columns[0], rows[1]),
        texel(columns[1], rows[1]),
    ];
    std::array::from_fn(|channel| {
        let top = a[channel] * (1.0 - fx) + b[channel] * fx;
        let bottom = c[channel] * (1.0 - fx) + d[channel] * fx;
        top * (1.0 - fy) + bottom * fy
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pert_ui_runtime::Viewport;

    #[test]
    fn a_filled_run_is_the_per_pixel_blend() -> Result<(), Box<dyn std::error::Error>> {
        // Stored pixels that repeat and vary: opaque, translucent and clear.
        let stored: Vec<[u8; 4]> = (0..64u8)
            .map(|index| match index % 4 {
                0 | 1 => [40, 90, 200, 255],
                2 => [index, 255 - index, index * 3, 255],
                _ => [index / 2, index / 3, index / 4, index],
            })
            .collect();
        let width = stored.len();
        let viewport = Viewport::new(width as f32, 1.0, 1.0)?;
        let sources = [
            // Nothing, opaque, translucent, and coverage-scaled paints.
            [0.0, 0.0, 0.0, 0.0],
            [0.2, 0.4, 0.6, 1.0],
            [0.1, 0.2, 0.3, 0.5],
            [0.25, 0.0, 0.0, 0.25],
            [0.999, 0.999, 0.999, 0.999],
        ];
        for source in sources {
            let mut frame = Frame {
                pixels: stored.iter().flatten().copied().collect(),
                width,
                height: 1,
                scale_x: 1.0,
                scale_y: 1.0,
                logical: (width as f64, 1.0),
                mapping: FrameMapping::new(width as u32, 1, viewport)?,
            };
            // The run leaves the first and last pixel alone.
            frame.fill_run(0, 1..width - 1, source, &mut None);
            let expected: Vec<u8> = stored
                .iter()
                .enumerate()
                .flat_map(|(index, pixel)| {
                    if index == 0 || index == width - 1 {
                        *pixel
                    } else {
                        blended(*pixel, source)
                    }
                })
                .collect();
            assert_eq!(frame.pixels, expected, "{source:?}");
        }
        Ok(())
    }
}
