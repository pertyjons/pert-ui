#![doc = include_str!("../README.md")]

use pert_ui_runtime::{
    CaptureError, Color, DisplayList, DrawCommand, DrawError, FrameImage, ImageSampling,
    LimitError, MaskImage, QuarterTurn, RasterImage, Runtime, VectorRasterizer,
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
    /// Drawing failures keep their typed cause, an oversized frame is
    /// [`CaptureError::Limit`] with the same limit, an empty viewport has no
    /// surface to render and the remaining renderer diagnostics, a failed
    /// allocation included, are backend text.
    fn from(error: CpuRenderError) -> Self {
        match error {
            CpuRenderError::Drawing(error) => error.into(),
            CpuRenderError::Limit(limit) => Self::Limit(limit),
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
/// ui.mount("swatch", Swatch)?;
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
        // Validate every command before preparing or allocating anything.
        for command in scene.commands() {
            check(scene, command)?;
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
    /// use pert_ui::{CaptureRequest, CaptureTarget, Element, Label, Runtime, Viewport};
    /// use std::time::Duration;
    ///
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(200.0, 80.0, 2.0)?)?;
    /// ui.mount_tree(Element::new("title", Label::new("Mixer")))?;
    /// ui.flush(Duration::ZERO)?;
    /// ui.set_capture_support(true);
    /// let pending = ui.request_capture(CaptureRequest::new(CaptureTarget::Path(vec!["title".into()])));
    /// ui.flush(Duration::ZERO)?;
    /// let mut renderer = CpuRenderer::new();
    /// assert_eq!(renderer.serve_captures(&mut ui), 1);
    /// let capture = pending.wait()?.value?;
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
fn check(scene: &DisplayList, command: &DrawCommand) -> Result<(), CpuRenderError> {
    match command {
        DrawCommand::FillRect { .. } => Ok(()),
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

/// The frame being drawn and its logical-to-physical mapping.
struct Frame {
    pixels: Vec<u8>,
    width: usize,
    height: usize,
    scale_x: f64,
    scale_y: f64,
    logical: (f64, f64),
}

impl Frame {
    fn draw(&mut self, scene: &DisplayList, command: &DrawCommand) -> Result<(), CpuRenderError> {
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
            // Every covered pixel gets the same source, so its result depends
            // on the stored bytes alone: reuse the last result while they
            // repeat, as they do across a uniform background.
            let mut last: Option<([u8; 4], [u8; 4])> = None;
            for row in rows {
                for column in columns.clone() {
                    let start = (row * self.width + column) * 4;
                    let stored: [u8; 4] = [
                        self.pixels[start],
                        self.pixels[start + 1],
                        self.pixels[start + 2],
                        self.pixels[start + 3],
                    ];
                    let result = match last {
                        Some((before, after)) if before == stored => after,
                        _ => {
                            let after = blended(stored, tint);
                            last = Some((stored, after));
                            after
                        }
                    };
                    self.pixels[start..start + 4].copy_from_slice(&result);
                }
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
