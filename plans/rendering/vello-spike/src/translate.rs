//! Part 2: translating a pert-ui `DisplayList` into vello_cpu calls.
//!
//! Depends only on the pert-ui facade and vello_cpu, so the gallery driver
//! (`gallery.sh`) can reuse this file unchanged.
//!
//! Mapping:
//! - background: a full-frame `fill_rect` in the background color;
//! - `FillRect`: `fill_rect` on the rectangle snapped to pert-ui's
//!   pixel-center rule, so edges stay aliased exactly as pert-ui draws them;
//! - `DrawMask`: the coverage mask as an RGBA8 pixmap (vello has no A8 image),
//!   registered once per mask content, painted as an image with an
//!   `AlphaMask` tint of the color; the quarter turn and stretch become the
//!   paint transform; the clip crops the filled rectangle;
//! - `DrawImage`: the raster image re-encoded from pert-ui's
//!   linear-premultiplied form to vello's sRGB-premultiplied form, registered
//!   once, painted the same way with a `Multiply` tint unless the tint is white;
//! - `DrawVector`: the path as a kurbo `BezPath`, filled with its rule or
//!   stroked with its width, caps, joins and miter limit, under a rectangular
//!   clip (`push_clip_rect`) when the command has one.
//!
//! Colors are converted from pert-ui's straight linear RGBA to vello's
//! straight sRGB-encoded RGBA. vello then composites in sRGB-encoded space;
//! there is no switch to composite in linear light, which is the remaining
//! difference from pert-ui's model (see results.md).
use pert_ui::{
    Color, DisplayList, DrawCommand, FillRule, ImageSampling, LineCap, LineJoin, MaskImage,
    PathElement, QuarterTurn, RasterImage, Rect, VectorStyle,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use vello_cpu::color::{AlphaColor, Srgb};
use vello_cpu::kurbo::{self, Affine, BezPath, Cap, Join, Stroke};
use vello_cpu::peniko::{self, ImageQuality, ImageSampler};
use vello_cpu::{Image, ImageId, ImageSource, PixelMetadata, Pixmap, Tint, TintMode};

fn encode(l: f32) -> f32 {
    let l = l.clamp(0.0, 1.0);
    if l <= 0.003_130_8 { l * 12.92 } else { 1.055 * l.powf(1.0 / 2.4) - 0.055 }
}

fn decode(c: u8) -> f32 {
    let c = f32::from(c) / 255.0;
    if c <= 0.040_45 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

/// pert-ui's straight linear color as vello's straight sRGB color.
pub fn srgb(color: Color) -> AlphaColor<Srgb> {
    let [r, g, b, a] = color.channels();
    AlphaColor::new([encode(r), encode(g), encode(b), a])
}

/// What one translated frame cost and needed.
#[derive(Clone, Copy, Debug, Default)]
pub struct Stats {
    /// Walking the commands and issuing vello calls, including converting
    /// and registering masks and images not seen before.
    pub translate: Duration,
    /// `flush` + `render` into the pixmap.
    pub render: Duration,
    /// Masks and images converted and registered in this frame.
    pub new_resources: usize,
    /// Bytes of RGBA8 pixmaps registered in this frame.
    pub new_bytes: usize,
    /// Commands of a kind this translator does not know (none today).
    pub unsupported: usize,
}

pub struct Translator {
    context: Option<vello_cpu::RenderContext>,
    resources: vello_cpu::Resources,
    pixmap: Pixmap,
    threads: u16,
    /// Registered pixmaps by content. Holding the pert-ui resource keeps its
    /// `Arc` alive, so equality's pointer check cannot see a reused address.
    masks: HashMap<MaskImage, ImageId>,
    images: HashMap<RasterImage, (ImageId, bool)>,
}

impl Translator {
    pub fn new(threads: u16) -> Self {
        Self {
            context: None,
            resources: vello_cpu::Resources::new(),
            pixmap: Pixmap::new(1, 1),
            threads,
            masks: HashMap::new(),
            images: HashMap::new(),
        }
    }

    /// Total bytes of every registered pixmap.
    #[allow(dead_code)]
    pub fn resident_bytes(&self) -> usize {
        self.masks.keys().map(|m| m.pixels().len() * 4).sum::<usize>()
            + self.images.keys().map(|i| i.pixels().len()).sum::<usize>()
    }

    /// Translates and renders `scene`; returns opaque-or-not RGBA8 pixels,
    /// premultiplied in sRGB-encoded space as vello stores them.
    pub fn render(&mut self, scene: &DisplayList) -> (Vec<u8>, Stats) {
        let mut stats = Stats::default();
        let extent = scene.viewport().frame_extent();
        let (w, h) = (extent.width() as u16, extent.height() as u16);
        let started = Instant::now();
        let reuse = self.context.as_ref().is_some_and(|c| c.width() == w && c.height() == h);
        if !reuse {
            let settings = vello_cpu::RenderSettings { num_threads: self.threads, ..Default::default() };
            self.context = Some(vello_cpu::RenderContext::new_with(w, h, settings));
            self.pixmap = Pixmap::new(w, h);
        }
        let Some(mut ctx) = self.context.take() else { unreachable!() };
        ctx.reset();
        let viewport = scene.viewport();
        let sx = f64::from(w) / f64::from(viewport.width());
        let sy = f64::from(h) / f64::from(viewport.height());
        ctx.set_paint(srgb(scene.background()));
        ctx.fill_rect(&kurbo::Rect::new(0.0, 0.0, f64::from(w), f64::from(h)));
        for command in scene.commands() {
            match *command {
                DrawCommand::FillRect { rect, color } => {
                    if let Some(r) = snap(rect, None, sx, sy) {
                        ctx.set_paint(srgb(color));
                        ctx.fill_rect(&r);
                    }
                }
                DrawCommand::DrawMask { rect, mask, color, turn, clip } => {
                    let Some(area) = snap(rect, clip, sx, sy) else { continue };
                    let mask = &scene.masks()[mask.index()];
                    let id = match self.masks.get(mask) {
                        Some(id) => *id,
                        None => {
                            let data: Vec<u8> = mask.pixels().iter().flat_map(|&c| [c, c, c, c]).collect();
                            stats.new_bytes += data.len();
                            stats.new_resources += 1;
                            let pixmap = Pixmap::from_parts(
                                data,
                                mask.width() as u16,
                                mask.height() as u16,
                                PixelMetadata {
                                    may_have_transparency: true,
                                    alpha_type: peniko::ImageAlphaType::AlphaPremultiplied,
                                },
                            );
                            let id = self.resources.register_image(Arc::new(pixmap));
                            self.masks.insert(mask.clone(), id);
                            id
                        }
                    };
                    ctx.set_paint(image(id, true, ImageQuality::Medium));
                    ctx.set_tint(Some(Tint { color: srgb(color), mode: TintMode::AlphaMask }));
                    ctx.set_paint_transform(placement(rect, turn, mask.width(), mask.height(), sx, sy));
                    ctx.fill_rect(&area);
                    ctx.reset_tint();
                    ctx.reset_paint_transform();
                }
                DrawCommand::DrawImage { rect, image: id, tint, sampling, turn, clip } => {
                    let Some(area) = snap(rect, clip, sx, sy) else { continue };
                    let raster = &scene.images()[id.index()];
                    let (vid, transparent) = match self.images.get(raster) {
                        Some(v) => *v,
                        None => {
                            let (data, transparent) = straight_srgb(raster.pixels());
                            stats.new_bytes += data.len();
                            stats.new_resources += 1;
                            let pixmap = Pixmap::from_parts(
                                data,
                                raster.width() as u16,
                                raster.height() as u16,
                                PixelMetadata {
                                    may_have_transparency: transparent,
                                    alpha_type: peniko::ImageAlphaType::Alpha,
                                },
                            );
                            let v = (self.resources.register_image(Arc::new(pixmap)), transparent);
                            self.images.insert(raster.clone(), v);
                            v
                        }
                    };
                    let quality = match sampling {
                        ImageSampling::Nearest => ImageQuality::Low,
                        _ => ImageQuality::Medium,
                    };
                    ctx.set_paint(image(vid, transparent, quality));
                    if tint != Color::WHITE {
                        ctx.set_tint(Some(Tint { color: srgb(tint), mode: TintMode::Multiply }));
                    }
                    ctx.set_paint_transform(placement(rect, turn, raster.width(), raster.height(), sx, sy));
                    ctx.fill_rect(&area);
                    ctx.reset_tint();
                    ctx.reset_paint_transform();
                }
                DrawCommand::DrawVector { vector, origin, scale, color, clip } => {
                    let geometry = &scene.vectors()[vector.index()];
                    let (ox, oy, k) = (f64::from(origin.x()), f64::from(origin.y()), f64::from(scale));
                    let mut path = BezPath::new();
                    let p = |q: &pert_ui::Point| kurbo::Point::new((ox + k * f64::from(q.x())) * sx, (oy + k * f64::from(q.y())) * sy);
                    for element in geometry.path().elements() {
                        match element {
                            PathElement::Move(a) => path.move_to(p(a)),
                            PathElement::Line(a) => path.line_to(p(a)),
                            PathElement::Quad(a, b) => path.quad_to(p(a), p(b)),
                            PathElement::Cubic(a, b, c) => path.curve_to(p(a), p(b), p(c)),
                            PathElement::Close => path.close_path(),
                        }
                    }
                    let clipped = match clip {
                        Some(c) => match snap(c, None, sx, sy) {
                            Some(r) => {
                                ctx.push_clip_rect(&r);
                                true
                            }
                            None => continue,
                        },
                        None => false,
                    };
                    ctx.set_paint(srgb(color));
                    match geometry.style() {
                        VectorStyle::Fill(rule) => {
                            ctx.set_fill_rule(match rule {
                                FillRule::EvenOdd => peniko::Fill::EvenOdd,
                                _ => peniko::Fill::NonZero,
                            });
                            ctx.fill_path(&path);
                            ctx.set_fill_rule(peniko::Fill::NonZero);
                        }
                        VectorStyle::Stroke(style) => {
                            let cap = match style.cap() {
                                LineCap::Round => Cap::Round,
                                LineCap::Square => Cap::Square,
                                _ => Cap::Butt,
                            };
                            let join = match style.join() {
                                LineJoin::Round => Join::Round,
                                LineJoin::Bevel => Join::Bevel,
                                _ => Join::Miter,
                            };
                            ctx.set_stroke(
                                Stroke::new(f64::from(style.width().get()) * k * sx)
                                    .with_caps(cap)
                                    .with_join(join)
                                    .with_miter_limit(f64::from(style.miter_limit())),
                            );
                            ctx.stroke_path(&path);
                        }
                    }
                    if clipped {
                        ctx.pop_clip();
                    }
                }
                _ => stats.unsupported += 1,
            }
        }
        ctx.flush();
        stats.translate = started.elapsed();
        let started = Instant::now();
        ctx.render(&mut self.pixmap, &mut self.resources);
        stats.render = started.elapsed();
        self.context = Some(ctx);
        (self.pixmap.data_as_u8_slice().to_vec(), stats)
    }
}

/// Physical pixels that a command with translucent paint (a fill or mask
/// color with alpha < 1, an image with a translucent tint or pixels) covers.
#[allow(dead_code)]
pub fn translucent(scene: &DisplayList) -> Vec<bool> {
    let extent = scene.viewport().frame_extent();
    let (w, h) = (extent.width() as usize, extent.height() as usize);
    let sx = w as f64 / f64::from(scene.viewport().width());
    let sy = h as f64 / f64::from(scene.viewport().height());
    let mut out = vec![false; w * h];
    for command in scene.commands() {
        let area = match *command {
            DrawCommand::FillRect { rect, color } if color.alpha() < 1.0 => snap(rect, None, sx, sy),
            DrawCommand::DrawMask { rect, color, clip, .. } if color.alpha() < 1.0 => snap(rect, clip, sx, sy),
            DrawCommand::DrawImage { rect, image, tint, clip, .. }
                if tint.alpha() < 1.0 || scene.images()[image.index()].pixels().chunks(4).any(|p| p[3] < 255) =>
            {
                snap(rect, clip, sx, sy)
            }
            _ => None,
        };
        if let Some(r) = area {
            for y in (r.y0 as usize)..(r.y1 as usize).min(h) {
                for x in (r.x0 as usize)..(r.x1 as usize).min(w) {
                    out[y * w + x] = true;
                }
            }
        }
    }
    out
}

fn image(id: ImageId, may_have_transparency: bool, quality: ImageQuality) -> Image {
    Image {
        image: ImageSource::OpaqueId { id, may_have_transparency },
        sampler: ImageSampler { quality, ..ImageSampler::default() },
    }
}

/// The physical pixels pert-ui covers for `rect` (∩ `clip`): those whose
/// centers lie inside, left/top edges included. `None` when empty.
fn snap(rect: Rect, clip: Option<Rect>, sx: f64, sy: f64) -> Option<kurbo::Rect> {
    let r = clip.map_or(rect, |c| rect.intersection(c));
    let [x, y, w, h] = r.values().map(f64::from);
    let edge = |v: f64, s: f64| (v * s - 0.5).ceil().max(0.0);
    let out = kurbo::Rect::new(edge(x, sx), edge(y, sy), edge(x + w, sx), edge(y + h, sy));
    (out.width() > 0.0 && out.height() > 0.0).then_some(out)
}

/// Paint transform mapping source pixels onto `rect` (physical) under `turn`.
fn placement(rect: Rect, turn: QuarterTurn, mw: u32, mh: u32, sx: f64, sy: f64) -> Affine {
    let [x, y, w, h] = rect.values().map(f64::from);
    let (x0, y0, w, h) = (x * sx, y * sy, w * sx, h * sy);
    let (mw, mh) = (f64::from(mw), f64::from(mh));
    // kurbo: (x, y) -> (a x + c y + e, b x + d y + f).
    match turn {
        QuarterTurn::Right => Affine::new([0.0, h / mw, -w / mh, 0.0, x0 + w, y0]),
        QuarterTurn::Half => Affine::new([-w / mw, 0.0, 0.0, -h / mh, x0 + w, y0 + h]),
        QuarterTurn::Left => Affine::new([0.0, -h / mw, w / mh, 0.0, x0, y0 + h]),
        _ => Affine::new([w / mw, 0.0, 0.0, h / mh, x0, y0]),
    }
}

/// pert-ui raster pixels (premultiplied in linear light, then sRGB-encoded)
/// as straight sRGB-encoded RGBA8 for vello to premultiply itself.
fn straight_srgb(pixels: &[u8]) -> (Vec<u8>, bool) {
    let mut out = Vec::with_capacity(pixels.len());
    let mut transparent = false;
    for p in pixels.chunks_exact(4) {
        let a = p[3];
        if a == 255 {
            out.extend_from_slice(p);
        } else if a == 0 {
            transparent = true;
            out.extend_from_slice(&[0, 0, 0, 0]);
        } else {
            transparent = true;
            let alpha = f32::from(a) / 255.0;
            for &c in &p[..3] {
                out.push((encode(decode(c) / alpha) * 255.0).round() as u8);
            }
            out.push(a);
        }
    }
    (out, transparent)
}
