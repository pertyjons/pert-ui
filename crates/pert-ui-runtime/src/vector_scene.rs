//! Preserved vector resources and an explicit renderer-side coverage fallback.
use crate::*;
use std::hash::{Hash, Hasher};

/// Opaque scene-local index; never persist or reuse with another display list.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct VectorId(pub(crate) usize);
impl VectorId {
    /// Index into [`DisplayList::vectors`] for this scene.
    pub const fn index(self) -> usize {
        self.0
    }
}
/// How an immutable path covers pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VectorStyle {
    /// Fill implicitly closed subpaths with this winding rule.
    Fill(FillRule),
    /// Stroke with validated width, caps and joins.
    Stroke(StrokeStyle),
}
/// Immutable renderer-independent vector data, shared by cloned scenes.
#[derive(Clone, Debug, PartialEq)]
pub struct VectorGeometry {
    path: Path,
    style: VectorStyle,
    key: u64,
}
impl VectorGeometry {
    pub(crate) fn new(path: Path, style: VectorStyle) -> Self {
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        for element in path.elements() {
            std::mem::discriminant(element).hash(&mut hash);
            let points: &[Point] = match element {
                PathElement::Move(a) | PathElement::Line(a) => std::slice::from_ref(a),
                _ => &[],
            };
            let point = |p: &Point, hash: &mut std::collections::hash_map::DefaultHasher| {
                // Equal positive/negative zeros must hash equally.
                for v in [p.x(), p.y()] {
                    (if v == 0.0 { 0 } else { v.to_bits() }).hash(hash);
                }
            };
            for p in points {
                point(p, &mut hash);
            }
            match element {
                PathElement::Quad(a, b) => {
                    point(a, &mut hash);
                    point(b, &mut hash);
                }
                PathElement::Cubic(a, b, c) => {
                    point(a, &mut hash);
                    point(b, &mut hash);
                    point(c, &mut hash);
                }
                _ => {}
            }
        }
        // Style equality is checked on each hit; the path digest selects a bucket.
        Self {
            path,
            style,
            key: hash.finish(),
        }
    }
    /// Original validated path, including curves and subpath closure.
    pub fn path(&self) -> &Path {
        &self.path
    }
    /// Fill rule or complete stroke style.
    pub const fn style(&self) -> VectorStyle {
        self.style
    }
    /// Process-local cache bucket key. Confirm hits with equality; collisions and
    /// different styles sharing a path are possible. Never persist it.
    pub const fn content_key(&self) -> u64 {
        self.key
    }
}
impl crate::resources::SceneResource for VectorGeometry {
    fn content_key(&self) -> u64 {
        self.key
    }
}
// What a vector's coverage was prepared from. A fill's coverage is in the
// geometry's own units, on the pixel grid `phase` places, and positioned by
// its command; a stroke's is in window coordinates, so every input of its
// preparation is part of the key.
#[derive(Clone, PartialEq)]
enum Key {
    Fill {
        vector: VectorGeometry,
        scale: f64,
        clip: Rect,
        phase: [f64; 2],
    },
    Stroke {
        vector: VectorGeometry,
        origin: Point,
        scale: f32,
        factors: [f64; 3],
        cut: [f64; 4],
    },
}
// The mask, its rectangle and, for a fill, the rectangle's first corner in
// the geometry's units before narrowing.
type Coverage = Option<(MaskImage, Rect, Option<(f64, f64)>)>;
type Cache = std::collections::HashMap<u64, Vec<(Key, Coverage)>>;

/// The coverage prepared for `key` in this preparation or the last
/// successful one.
fn find(cache: &Cache, next: &Cache, key: &Key) -> Option<Coverage> {
    let bucket = match key {
        Key::Fill { vector, .. } | Key::Stroke { vector, .. } => vector.content_key(),
    };
    next.get(&bucket)
        .into_iter()
        .flatten()
        .chain(cache.get(&bucket).into_iter().flatten())
        .find(|(k, _)| k == key)
        .map(|(_, coverage)| coverage.clone())
}

/// A stroke's mask sampled from its reference coverage, one texel per frame
/// pixel of its span, and the span in window coordinates.
fn stroke_coverage(
    vector: &VectorGeometry,
    origin: Point,
    scale: f32,
    clip: Option<Rect>,
    mapping: &crate::FrameMapping,
) -> Result<Coverage, DrawError> {
    let Some(stroke) = crate::PreparedStroke::new(vector, origin, scale, clip, mapping)? else {
        return Ok(None);
    };
    let [x0, y0, x1, y1] = stroke.span();
    let (width, height) = ((x1 - x0) as usize, (y1 - y0) as usize);
    if width.saturating_mul(height) > MaskImage::MAX_PIXELS {
        return Err(DrawError::CoverageTooLarge);
    }
    let mut pixels = vec![0u8; width * height];
    let mut any = false;
    stroke.for_each_covered(|x, y, coverage| {
        let value = (coverage * 255.0).round() as u8;
        any |= value > 0;
        pixels[(y - y0) as usize * width + (x - x0) as usize] = value;
    });
    if !any {
        return Ok(None);
    }
    let [sx, sy, _] = mapping.factors();
    let rect = Rect::new(
        (f64::from(x0) / sx) as f32,
        (f64::from(y0) / sy) as f32,
        (f64::from(x1 - x0) / sx) as f32,
        (f64::from(y1 - y0) / sy) as f32,
    )?;
    Ok(Some((
        MaskImage::new(width as u32, height as u32, pixels)?,
        rect,
        None,
    )))
}
/// The window rectangle of a fill's coverage that was rasterized on the
/// frame's pixels: its first corner `corner`, in the geometry's units and
/// not yet narrowed, placed by `origin` and `scale` and moved onto the whole
/// pixels it was computed for, one texel per pixel. `frame` is the frame's
/// position factors and the window scale.
///
/// `None` when the coverage does not lie on the frame's pixels, and is then
/// placed and stretched as given: where the frame maps positions and lengths
/// apart by more than 1/64 pixel over the mask (a logical size that is not a
/// whole number of pixels), or where even `f64` lost the grid to rounding.
fn fill_on_pixels(
    corner: (f64, f64),
    origin: Point,
    scale: f32,
    (width, height): (u32, u32),
    [sx, sy, window]: [f64; 3],
) -> Option<Rect> {
    const TOLERANCE: f64 = 1.0 / 64.0;
    let scale = f64::from(scale);
    let x = (corner.0 * scale + f64::from(origin.x())) * sx;
    let y = (corner.1 * scale + f64::from(origin.y())) * sy;
    let (width, height) = (f64::from(width), f64::from(height));
    let drift = (width * (sx / window - 1.0))
        .abs()
        .max((height * (sy / window - 1.0)).abs());
    let off = (x - x.round()).abs().max((y - y.round()).abs());
    // A NaN fails the comparison and keeps the given placement.
    if !(drift <= TOLERANCE && off <= TOLERANCE) {
        return None;
    }
    Rect::new(
        (x.round() / sx) as f32,
        (y.round() / sy) as f32,
        (width / sx) as f32,
        (height / sy) as f32,
    )
    .ok()
}
/// Whether every edge of `rect`, a stroke mask's placement, lies within 2⁻⁸
/// of a frame pixel edge, as it does in a frame up to 16 384 pixels per
/// axis. Beyond that, narrowing to `f32` can move an edge across a pixel
/// center.
fn on_pixels(rect: Rect, mapping: &crate::FrameMapping) -> bool {
    let [sx, sy, _] = mapping.factors();
    let [x, y, w, h] = rect.values().map(f64::from);
    [x * sx, y * sy, (x + w) * sx, (y + h) * sy]
        .iter()
        .all(|edge| (edge - edge.round()).abs() <= 2f64.powi(-8))
}

/// CPU vector fallback for graphics adapters. Retain one per scene stream.
///
/// Lowers vectors to coverage masks. A fill is scan-converted on four
/// sub-scanlines per physical pixel row, with exact horizontal overlap. At
/// its displayed size it is rasterized on the pixels of the frame the
/// scene's viewport maps to: the grid takes the command's position within a
/// pixel, and the mask's rectangle lies on whole frame pixels with one texel
/// each, so a renderer reads every texel at its center and no edge of the
/// rectangle falls beside a pixel center. Two cases keep a mask that is
/// stretched where it is drawn: coverage a magnifying view holds below the
/// displayed size, and a frame whose position factors differ from the
/// viewport's scale, as a logical size that is not a whole number of pixels
/// gives, by more than 1/64 pixel over the mask. A
/// stroke is sampled from its reference coverage,
/// [`PreparedStroke`](crate::PreparedStroke), at the pixel centers of the
/// frame the scene's viewport maps to
/// ([`Viewport::frame_extent`](crate::Viewport::frame_extent)), and quantized
/// to eight bits: a renderer that draws its mask over that frame gets the
/// coverage pert-ui's renderers draw. The mask's rectangle is the stroke's
/// span of whole frame pixels, stored as `f32` logical coordinates, so each
/// texel center lies within 2⁻⁸ physical pixel of its frame pixel's center
/// in a frame up to 16 384 pixels per axis; a renderer that samples the mask
/// bilinearly blends at most that fraction of a neighboring texel. The
/// span is already cut to the command's clip, so such a mask carries no
/// clip; in a larger frame, where narrowing can move an edge of the
/// rectangle by more, the mask keeps the command's clip. A renderer whose
/// frame has another size stretches the mask as it would any other.
///
/// Keeps coverage only for resources used by the last successful preparation.
/// A fill with equal geometry and effective scale reuses its coverage
/// regardless of tint, and at any position that lies at the same place within
/// a frame pixel, when the clip leaves the same part of the geometry visible:
/// a window resize that keeps a fill wholly visible, or a move by whole
/// pixels, does not rasterize it again, while a move by a fraction of a pixel
/// does. Coverage held below the displayed size keeps its reuse wherever it
/// lies. A stroke reuses its mask when every input of its
/// preparation is equal: the geometry, the command's origin, scale and clip
/// as it cuts the frame's pixels, and the viewport's mapping; a moved stroke
/// or a resized window prepares it again. Work and allocations happen after
/// scene recording, outside time-critical callbacks. The cache is bounded by
/// the current/previous scene, not its history.
///
/// Curve subdivision targets a maximum chord deviation of 0.2 physical pixels.
/// Flattening admits at most [`Self::MAX_FLATTENED_POINTS`] generated points
/// per vector, including subpath starts. If the requested subdivision cannot
/// fit, preparation returns [`DrawError::Path`] holding
/// [`PathError::TooLarge`](crate::PathError::TooLarge) instead of silently
/// reducing the subdivision count. A stroke has the limits of
/// [`PreparedStroke::new`](crate::PreparedStroke::new) besides.
///
/// This is a fallback for vectors within a graphics backend, not a software
/// window renderer. Backends can instead consume the original geometry directly.
#[derive(Default)]
pub struct VectorRasterizer {
    cache: Cache,
}

/// Shows the rasterizer without its cached coverage.
impl std::fmt::Debug for VectorRasterizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VectorRasterizer").finish_non_exhaustive()
    }
}
impl VectorRasterizer {
    /// The most points flattening may generate for one vector, subpath starts
    /// included: 1,048,576 (2^20). A vector whose curves need more at the
    /// 0.2-pixel tolerance is rejected with [`DrawError::Path`] holding
    /// [`PathError::TooLarge`](crate::PathError::TooLarge) rather than
    /// flattened more coarsely.
    pub const MAX_FLATTENED_POINTS: usize = crate::geometry::MAX_EDGES;
    /// The most sub-scanline steps covering one fill may take: 16,777,216
    /// (2^24). Coverage is sampled on four sub-scanlines per physical pixel
    /// row, and each edge of a fill's outline costs one step for every
    /// sub-scanline it crosses inside the cropped mask. A fill over the
    /// budget is rejected with [`DrawError::TooComplex`] before any coverage
    /// work, which bounds preparation time. A fill of thousands of points
    /// that crosses a few hundred pixels per point stays well inside it.
    /// Strokes do not count toward it: they have the budgets of
    /// [`PreparedStroke::new`](crate::PreparedStroke::new).
    pub const MAX_COVERAGE_WORK: usize = 1 << 24;

    /// Resolves vectors to masks while preserving command order and clipping.
    ///
    /// A fill is rasterized at the scene's scale times its command's own
    /// `scale`, on the frame's pixels as the type describes, and held at
    /// eight physical pixels per geometry unit when a view magnifies further
    /// (coverage is stretched beyond that). A stroke is
    /// prepared at its displayed size, without that ceiling, and its mask
    /// never extends past the frame. A stroke in a viewport whose frame
    /// extent has no pixels is skipped. On failure both the input and the
    /// last successful cache remain usable.
    ///
    /// # Errors
    /// Returns [`DrawError::MissingVector`], [`DrawError::MissingMask`] or
    /// [`DrawError::MissingImage`] for a reference that is not in `scene`,
    /// [`DrawError::CoverageTooLarge`] for cropped coverage above
    /// [`MaskImage::MAX_PIXELS`](crate::MaskImage::MAX_PIXELS); [`DrawError::Path`] with
    /// [`PathError::TooLarge`](crate::PathError::TooLarge) when flattening exceeds its
    /// segment budget; [`DrawError::TooComplex`] when covering a fill exceeds
    /// [`Self::MAX_COVERAGE_WORK`] or a stroke exceeds a budget of
    /// [`PreparedStroke`](crate::PreparedStroke); [`DrawError::Stroke`] holding
    /// [`StrokePrepareError::OutOfRange`](crate::StrokePrepareError::OutOfRange)
    /// for a stroke beyond the range it is evaluated in; and [`DrawError::Runtime`] holding
    /// [`RuntimeError::Geometry`](crate::RuntimeError::Geometry) with
    /// [`GeometryError::NonFinite`](crate::GeometryError::NonFinite) for a NaN or infinite
    /// command scale or a derived coordinate that is not finite, and with
    /// [`GeometryError::NonPositiveScale`](crate::GeometryError::NonPositiveScale) for a
    /// finite command scale that is zero or negative. A command that is wholly clipped
    /// or fully transparent is skipped before its scale is checked.
    pub fn prepare(&mut self, scene: &DisplayList) -> Result<DisplayList, DrawError> {
        self.lower(scene, false)
    }

    /// As [`Self::prepare`], but leaves in the scene every stroke, for a
    /// renderer that covers strokes itself: a [`DrawCommand::DrawVector`]
    /// whose vector
    /// [`PreparedStroke::accepts`](crate::PreparedStroke::accepts). Fills
    /// become masks as in [`Self::prepare`]. The renderer prepares each remaining
    /// stroke with [`PreparedStroke::new`](crate::PreparedStroke::new),
    /// which checks its scale and reports its own errors; this method
    /// checks only that its vector is in `scene`.
    ///
    /// Such a stroke needs no mask.
    ///
    /// # Errors
    /// As for [`Self::prepare`], for the vectors that become masks, and
    /// [`DrawError::MissingVector`] for any vector missing from `scene`.
    pub fn prepare_fills(&mut self, scene: &DisplayList) -> Result<DisplayList, DrawError> {
        self.lower(scene, true)
    }

    fn lower(&mut self, scene: &DisplayList, keep_strokes: bool) -> Result<DisplayList, DrawError> {
        let mut result = DisplayList {
            gradients: scene.gradients.clone(),
            background: scene.background,
            viewport: scene.viewport,
            ..Default::default()
        };
        let mut next = Cache::new();
        let viewport = Rect::new(0.0, 0.0, scene.viewport.width(), scene.viewport.height())?;
        let mut mapping: Option<crate::FrameMapping> = None;
        for command in scene.commands() {
            let command = match *command {
                DrawCommand::DrawVector {
                    vector,
                    origin,
                    scale,
                    color,
                    clip,
                } => {
                    let vector = scene
                        .vector(vector)
                        .ok_or(DrawError::MissingVector(vector))?;
                    if keep_strokes && crate::PreparedStroke::accepts(vector) {
                        result.fills.push(DrawCommand::DrawVector {
                            vector: VectorId(result.vectors.intern(vector)),
                            origin,
                            scale,
                            color,
                            clip,
                        });
                        continue;
                    }
                    let command_clip = clip;
                    let clip = clip.map_or(viewport, |c| c.intersection(viewport));
                    if clip.is_empty() || color.alpha() == 0.0 {
                        continue;
                    }
                    if !scale.is_finite() {
                        return Err(crate::GeometryError::NonFinite.into());
                    }
                    if scale <= 0.0 {
                        return Err(crate::GeometryError::NonPositiveScale.into());
                    }
                    let mut placed = None;
                    let mut frame = None;
                    // Built at the first vector; a frame without pixels has
                    // nothing to cover.
                    if mapping.is_none() {
                        let extent = scene.viewport.frame_extent();
                        mapping = crate::FrameMapping::new(
                            extent.width(),
                            extent.height(),
                            scene.viewport,
                        )
                        .ok();
                    }
                    let (key, coverage) = if let VectorStyle::Stroke(_) = vector.style() {
                        let Some(mapping) = &mapping else {
                            continue;
                        };
                        let key = Key::Stroke {
                            vector: vector.clone(),
                            origin,
                            scale,
                            factors: mapping.factors(),
                            cut: mapping.cut(command_clip),
                        };
                        let coverage = match find(&self.cache, &next, &key) {
                            Some(coverage) => coverage,
                            None => stroke_coverage(vector, origin, scale, command_clip, mapping)?,
                        };
                        placed = coverage
                            .as_ref()
                            .map(|(_, rect, _)| on_pixels(*rect, mapping));
                        (key, coverage)
                    } else {
                        let VectorStyle::Fill(rule) = vector.style() else {
                            unreachable!("strokes take the branch above");
                        };
                        // The clip in the geometry's own units, which a view
                        // transform magnifies by `scale` on the way to the window.
                        let local = Rect::new(
                            (clip.x() - origin.x()) / scale,
                            (clip.y() - origin.y()) / scale,
                            clip.width() / scale,
                            clip.height() / scale,
                        )?;
                        // Prepared at the displayed size up to the ceiling every
                        // magnifying view shares; the destination below still uses
                        // the full factor, so coverage beyond it is stretched.
                        let window = scene.viewport.scale();
                        let raster_scale = crate::runtime::raster_scale(window, scale);
                        // At the displayed size the coverage is rasterized on
                        // the frame's own pixels: the grid takes the origin's
                        // position within a pixel. Coverage held below that
                        // size is stretched wherever it lies, so it keeps one
                        // phase and its reuse while the view moves.
                        let mut phase = [0.0; 2];
                        if let Some(mapping) = &mapping
                            && raster_scale == window * f64::from(scale)
                        {
                            let [sx, sy, _] = mapping.factors();
                            phase = [
                                (f64::from(origin.x()) * sx).rem_euclid(1.0),
                                (f64::from(origin.y()) * sy).rem_euclid(1.0),
                            ];
                            frame = Some([sx, sy, window]);
                        }
                        // Only the visible part of the coverage matters, so a clip
                        // change that leaves it whole (a window resize) still hits.
                        let clip = crate::geometry::coverage_bounds(vector.path(), raster_scale)
                            .map_or(local, |bounds| local.intersection(bounds));
                        let key = Key::Fill {
                            vector: vector.clone(),
                            scale: raster_scale,
                            clip,
                            phase,
                        };
                        let coverage = match find(&self.cache, &next, &key) {
                            Some(coverage) => coverage,
                            None => crate::geometry::rasterize_fill(
                                vector.path(),
                                rule,
                                raster_scale,
                                Some(clip),
                                (phase[0], phase[1]),
                            )?
                            .map(|(mask, rect, corner)| (mask, rect, Some(corner))),
                        };
                        (key, coverage)
                    };
                    let key_is_fill = matches!(key, Key::Fill { .. });
                    let entries = next.entry(vector.content_key()).or_default();
                    if !entries.iter().any(|(k, _)| k == &key) {
                        entries.push((key, coverage.clone()));
                    }
                    let Some((mask, rect, corner)) = coverage else {
                        continue;
                    };
                    // A fill's coverage is in its own units, placed here and
                    // clipped by its command's clip; rasterized on the frame's
                    // pixels, it is placed on them exactly. A stroke's mask is its
                    // span, already cut to the clip on the frame's pixels;
                    // clipping it again would apply a renderer's own rounding
                    // of the clip's edges, which can differ by a row where an
                    // edge lies on pixel centers. A mask whose placement left
                    // the pixel edges, in a frame beyond 16 384 pixels, keeps
                    // the clip, so that it never draws outside it.
                    let (rect, clip) = if key_is_fill {
                        let on_frame = frame.zip(corner).and_then(|(frame, corner)| {
                            fill_on_pixels(
                                corner,
                                origin,
                                scale,
                                (mask.width(), mask.height()),
                                frame,
                            )
                        });
                        (
                            match on_frame {
                                Some(rect) => rect,
                                None => Rect::new(
                                    rect.x() * scale + origin.x(),
                                    rect.y() * scale + origin.y(),
                                    rect.width() * scale,
                                    rect.height() * scale,
                                )?,
                            },
                            command_clip,
                        )
                    } else if placed == Some(true) {
                        (rect, None)
                    } else {
                        (rect, command_clip)
                    };
                    DrawCommand::DrawMask {
                        mask: MaskId(result.masks.intern(&mask)),
                        rect,
                        color,
                        turn: crate::QuarterTurn::None,
                        clip,
                    }
                }
                DrawCommand::DrawMask {
                    mask,
                    rect,
                    color,
                    turn,
                    clip,
                } => DrawCommand::DrawMask {
                    mask: MaskId(
                        result
                            .masks
                            .intern(scene.mask(mask).ok_or(DrawError::MissingMask(mask))?),
                    ),
                    rect,
                    color,
                    turn,
                    clip,
                },
                DrawCommand::DrawImage {
                    image,
                    rect,
                    tint,
                    sampling,
                    turn,
                    clip,
                } => DrawCommand::DrawImage {
                    image: ImageId(
                        result
                            .images
                            .intern(scene.image(image).ok_or(DrawError::MissingImage(image))?),
                    ),
                    rect,
                    tint,
                    sampling,
                    turn,
                    clip,
                },
                other => other,
            };
            result.fills.push(command);
        }
        self.cache = next;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scene(color: Color, size: f32) -> DisplayList {
        let path = Path::polygon([
            Point::new(0.0, 0.0).unwrap(),
            Point::new(size, 0.0).unwrap(),
            Point::new(size, size).unwrap(),
            Point::new(0.0, size).unwrap(),
        ])
        .unwrap();
        let mut scene = DisplayList {
            viewport: Viewport::new(size, size, 1.0).unwrap(),
            ..Default::default()
        };
        let id = VectorId(scene.vectors.intern(&VectorGeometry::new(
            path,
            VectorStyle::Fill(FillRule::NonZero),
        )));
        scene.fills.push(DrawCommand::DrawVector {
            vector: id,
            origin: Point::default(),
            scale: 1.0,
            color,
            clip: None,
        });
        scene
    }
    #[test]
    fn preparing_fills_leaves_strokes_and_masks_fills() -> Result<(), Box<dyn std::error::Error>> {
        let style = StrokeStyle::new(Length::new(2.0)?);
        let line = Path::polyline([Point::new(2.0, 2.0)?, Point::new(14.0, 9.0)?])?;
        let curve = PathBuilder::new()
            .move_to(Point::new(2.0, 2.0)?)
            .quad_to(Point::new(8.0, 14.0)?, Point::new(14.0, 2.0)?)
            .build()?;
        let mut scene = DisplayList {
            viewport: Viewport::new(16.0, 16.0, 1.0)?,
            ..Default::default()
        };
        let clip = Some(Rect::new(1.0, 1.0, 12.0, 12.0)?);
        for (path, style) in [
            (curve.clone(), VectorStyle::Fill(FillRule::NonZero)),
            (line.clone(), VectorStyle::Stroke(style)),
            (curve, VectorStyle::Stroke(style)),
            (line, VectorStyle::Stroke(style)),
        ] {
            let id = VectorId(scene.vectors.intern(&VectorGeometry::new(path, style)));
            scene.fills.push(DrawCommand::DrawVector {
                vector: id,
                origin: Point::new(1.0, 0.5)?,
                scale: 1.5,
                color: Color::WHITE,
                clip,
            });
        }
        let prepared = VectorRasterizer::default().prepare_fills(&scene)?;
        let kept: Vec<bool> = prepared
            .commands()
            .iter()
            .map(|command| matches!(command, DrawCommand::DrawVector { .. }))
            .collect();
        // The fill becomes a mask; the strokes stay, curved or not.
        assert_eq!(kept, [false, true, true, true]);
        assert!(matches!(
            prepared.commands()[0],
            DrawCommand::DrawMask { .. }
        ));
        // The kept strokes keep their placement, and equal ones share a
        // vector.
        assert_eq!(prepared.vectors().len(), 2);
        assert_eq!(prepared.commands()[1], prepared.commands()[3]);
        for command in prepared.commands()[1..].iter().copied() {
            let DrawCommand::DrawVector {
                vector,
                origin,
                scale,
                color,
                clip: kept_clip,
            } = command
            else {
                unreachable!("checked above");
            };
            assert!(crate::PreparedStroke::accepts(
                prepared.vector(vector).ok_or("the kept vector")?
            ));
            assert_eq!(
                (origin, scale, color, kept_clip),
                (Point::new(1.0, 0.5)?, 1.5, Color::WHITE, clip)
            );
        }
        // `prepare` lowers every vector.
        let lowered = scene.rasterized()?;
        assert!(lowered.vectors().is_empty());
        assert!(
            lowered
                .commands()
                .iter()
                .all(|command| matches!(command, DrawCommand::DrawMask { .. }))
        );
        // A stroke with a non-finite scale is the renderer's to refuse.
        let Some(DrawCommand::DrawVector { scale, .. }) = scene.fills.get_mut(2) else {
            unreachable!("recorded above");
        };
        *scale = f32::NAN;
        assert!(VectorRasterizer::default().prepare_fills(&scene).is_ok());
        assert!(VectorRasterizer::default().prepare(&scene).is_err());
        Ok(())
    }

    #[test]
    fn a_reference_missing_from_its_scene_names_the_id() {
        let mut cache = VectorRasterizer::default();
        let mut foreign = scene(Color::WHITE, 16.0);
        foreign.vectors = Default::default();
        assert_eq!(
            cache.prepare(&foreign),
            Err(DrawError::MissingVector(VectorId(0)))
        );
        let mut masked = DisplayList {
            viewport: Viewport::new(16.0, 16.0, 1.0).unwrap(),
            ..Default::default()
        };
        masked.fills.push(DrawCommand::DrawMask {
            rect: Rect::new(0.0, 0.0, 4.0, 4.0).unwrap(),
            mask: MaskId(3),
            color: Color::WHITE,
            turn: crate::QuarterTurn::None,
            clip: None,
        });
        assert_eq!(
            cache.prepare(&masked),
            Err(DrawError::MissingMask(MaskId(3)))
        );
        // The message names the missing ID's index.
        assert_eq!(
            DrawError::MissingVector(VectorId(0)).to_string(),
            "a command refers to vector 0 missing from its scene"
        );
        assert_eq!(
            DrawError::MissingMask(MaskId(3)).to_string(),
            "a command refers to mask 3 missing from its scene"
        );
        assert_eq!(
            DrawError::MissingImage(crate::ImageId(7)).to_string(),
            "a command refers to image 7 missing from its scene"
        );
    }
    #[test]
    fn persistent_fallback_reuses_tinted_coverage_recovers_and_evicts() {
        let mut cache = VectorRasterizer::default();
        let first = cache.prepare(&scene(Color::WHITE, 16.0)).unwrap();
        let pixels = first.masks()[0].pixels().as_ptr();
        let tinted = cache.prepare(&scene(Color::ACCENT, 16.0)).unwrap();
        assert_eq!(
            pixels,
            tinted.masks()[0].pixels().as_ptr(),
            "same immutable coverage allocation"
        );
        assert_eq!(
            cache.prepare(&scene(Color::WHITE, 5000.0)),
            Err(DrawError::CoverageTooLarge)
        );
        let recovered = cache.prepare(&scene(Color::WHITE, 16.0)).unwrap();
        assert_eq!(first, recovered);
        assert_eq!(pixels, recovered.masks()[0].pixels().as_ptr());
        cache.prepare(&DisplayList::default()).unwrap();
        assert!(
            cache.cache.is_empty(),
            "empty scenes evict obsolete coverage"
        );
    }
    #[test]
    fn resizing_around_wholly_visible_geometry_reuses_coverage() {
        let mut cache = VectorRasterizer::default();
        let mut first = scene(Color::WHITE, 16.0);
        first.viewport = Viewport::new(32.0, 32.0, 1.0).unwrap();
        let first = cache.prepare(&first).unwrap();
        let mut wider = scene(Color::WHITE, 16.0);
        wider.viewport = Viewport::new(64.0, 48.0, 1.0).unwrap();
        let resized = cache.prepare(&wider).unwrap();
        assert_eq!(
            first.masks()[0].pixels().as_ptr(),
            resized.masks()[0].pixels().as_ptr(),
            "a clip change outside the geometry keeps its coverage"
        );
        let mut cropped = scene(Color::WHITE, 16.0);
        cropped.viewport = Viewport::new(8.0, 16.0, 1.0).unwrap();
        let cropped = cache.prepare(&cropped).unwrap();
        assert_eq!(
            cropped.masks()[0].width(),
            8,
            "a crop still rasterizes anew"
        );
    }
    #[test]
    fn coverage_bounds_crop_matches_the_plain_clip() {
        let points = [(0.0, 0.0), (20.0, 2.0), (4.0, 12.0), (18.0, 18.0)]
            .map(|(x, y)| Point::new(x, y).unwrap());
        let path = PathBuilder::new()
            .move_to(points[0])
            .line_to(points[1])
            .line_to(points[2])
            .cubic_to(points[3], points[0], points[1])
            .build()
            .unwrap();
        let rules = [FillRule::EvenOdd, FillRule::NonZero];
        let clips = [
            Rect::new(-100.0, -100.0, 400.0, 400.0).unwrap(),
            Rect::new(-50.0, 3.0, 400.0, 7.5).unwrap(),
            Rect::new(5.25, -50.0, 3.5, 400.0).unwrap(),
            Rect::new(30.0, 30.0, 10.0, 10.0).unwrap(),
        ];
        for rule in rules {
            for scale in [1.0, 1.5, 2.0] {
                for phase in [(0.0, 0.0), (0.3, 0.75), (0.999, 0.001)] {
                    let fill = |clip| {
                        crate::geometry::rasterize_fill(&path, rule, scale, clip, phase)
                            .unwrap()
                            .map(|(mask, rect, _)| (mask, rect))
                    };
                    let bounds = crate::geometry::coverage_bounds(&path, scale).unwrap();
                    let whole = fill(None);
                    assert_eq!(fill(Some(bounds)), whole, "the bounds contain all coverage");
                    for clip in clips {
                        assert_eq!(fill(Some(clip.intersection(bounds))), fill(Some(clip)));
                    }
                }
            }
        }
    }
    /// A ten-unit square filled at `origin` under the view scale `view`.
    fn square(viewport: Viewport, origin: (f32, f32), view: f32) -> DisplayList {
        let mut scene = scene(Color::WHITE, 10.0);
        scene.viewport = viewport;
        let DrawCommand::DrawVector {
            origin: o, scale, ..
        } = &mut scene.fills[0]
        else {
            unreachable!()
        };
        *o = p(origin.0, origin.1);
        *scale = view;
        scene
    }
    /// The one mask of a prepared scene with its rectangle in frame pixels.
    fn placed(prepared: &DisplayList) -> (&MaskImage, [f64; 4]) {
        let [DrawCommand::DrawMask { mask, rect, .. }] = prepared.commands() else {
            panic!("one mask expected");
        };
        let scale = prepared.viewport().scale();
        (
            prepared.mask(*mask).unwrap(),
            rect.values().map(|v| f64::from(v) * scale),
        )
    }
    #[test]
    fn a_fill_is_rasterized_on_the_frames_pixels_at_any_position() {
        for scale in [1.0, 1.25, 2.0] {
            for origin in [(3.3, 4.7), (0.0, 0.0), (17.999, 0.001), (-2.6, 5.5)] {
                let viewport = Viewport::new(64.0, 48.0, scale).unwrap();
                let prepared = VectorRasterizer::default()
                    .prepare(&square(viewport, origin, 1.0))
                    .unwrap();
                let (mask, rect) = placed(&prepared);
                let name = format!("{scale} {origin:?}");
                // The rectangle lies on whole pixels, one texel each, within
                // what its `f32` logical coordinates hold.
                for edge in [rect[0], rect[1]] {
                    assert!((edge - edge.round()).abs() < 1e-4, "{name}: {rect:?}");
                }
                assert!((rect[2] - f64::from(mask.width())).abs() < 1e-4, "{name}");
                assert!((rect[3] - f64::from(mask.height())).abs() < 1e-4, "{name}");
                // Each texel holds the area of the square inside its pixel:
                // exactly across, and to the four sub-scanlines of a row down.
                let (left, top) = (rect[0].round(), rect[1].round());
                let low = |v: f32| (f64::from(v).max(0.0)) * scale;
                let high = |v: f32, limit: f32| f64::from(v + 10.0).min(f64::from(limit)) * scale;
                let (x0, x1) = (low(origin.0), high(origin.0, 64.0));
                let (y0, y1) = (low(origin.1), high(origin.1, 48.0));
                let overlap =
                    |a: f64, b: f64, cell: f64| (b.min(cell + 1.0) - a.max(cell)).clamp(0.0, 1.0);
                for row in 0..mask.height() {
                    for column in 0..mask.width() {
                        let across = overlap(x0, x1, left + f64::from(column));
                        let wanted = across * overlap(y0, y1, top + f64::from(row));
                        let got = f64::from(mask.pixels()[(row * mask.width() + column) as usize]);
                        assert!(
                            (got / 255.0 - wanted).abs() <= across * 0.125 + 1.0 / 255.0,
                            "{name}: ({column}, {row}) {got} for {wanted}"
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn a_fill_far_from_its_origin_is_still_placed_on_its_pixels() {
        // Local coordinates beyond 2^24 hold no fraction in `f32`, so the
        // mask's own rectangle cannot say where its grid lies. The left edge
        // is at frame x 23.5: the mask starts at pixel 23, half covered.
        let left = 16_777_222.0f32;
        let path = Path::polygon([
            p(left, 0.0),
            p(left + 8.0, 0.0),
            p(left + 8.0, 4.0),
            p(left, 4.0),
        ])
        .unwrap();
        let mut scene = DisplayList {
            viewport: Viewport::new(64.0, 48.0, 1.0).unwrap(),
            ..Default::default()
        };
        let id = VectorId(scene.vectors.intern(&VectorGeometry::new(
            path,
            VectorStyle::Fill(FillRule::NonZero),
        )));
        scene.fills.push(DrawCommand::DrawVector {
            vector: id,
            origin: p(-20_971_504.0, 2.0),
            scale: 1.25,
            color: Color::WHITE,
            clip: None,
        });
        let prepared = VectorRasterizer::default().prepare(&scene).unwrap();
        let (mask, rect) = placed(&prepared);
        assert_eq!((rect[0], rect[1]), (23.0, 2.0));
        assert_eq!(rect[2], f64::from(mask.width()));
        assert_eq!(mask.pixels()[0], 128);
        assert_eq!(mask.pixels()[1], 255);
    }
    #[test]
    fn a_fill_reuses_its_coverage_at_the_same_position_within_a_pixel() {
        let viewport = Viewport::new(64.0, 48.0, 2.0).unwrap();
        let mut cache = VectorRasterizer::default();
        let pixels = |prepared: &DisplayList| placed(prepared).0.pixels().as_ptr();
        let first = cache.prepare(&square(viewport, (3.25, 4.75), 1.0)).unwrap();
        // Whole frame pixels away, at scale 2: the same phase.
        let moved = cache
            .prepare(&square(viewport, (10.75, 20.25), 1.0))
            .unwrap();
        assert_eq!(pixels(&first), pixels(&moved));
        assert_eq!(placed(&moved).1[0], placed(&first).1[0] + 15.0);
        // A quarter of a pixel further its edges fall elsewhere in their pixels.
        let shifted = cache
            .prepare(&square(viewport, (10.875, 20.25), 1.0))
            .unwrap();
        assert_ne!(pixels(&moved), pixels(&shifted));
        assert_ne!(placed(&moved).0.pixels(), placed(&shifted).0.pixels());
    }
    #[test]
    fn coverage_held_below_the_displayed_size_keeps_its_reuse_and_stretch() {
        // A view factor of 32 is rasterized at eight pixels per unit.
        let viewport = Viewport::new(640.0, 480.0, 1.0).unwrap();
        let mut cache = VectorRasterizer::default();
        let first = cache
            .prepare(&square(viewport, (13.3, 14.7), 32.0))
            .unwrap();
        let moved = cache
            .prepare(&square(viewport, (19.1, 12.2), 32.0))
            .unwrap();
        let (mask, rect) = placed(&first);
        assert_eq!((mask.width(), mask.height()), (80, 80));
        assert!((rect[2] - 320.0).abs() < 1e-3 && (rect[0] - 13.3).abs() < 1e-3);
        assert_eq!(mask.pixels().as_ptr(), placed(&moved).0.pixels().as_ptr());
    }
    #[test]
    fn a_frame_that_maps_positions_and_lengths_apart_stretches_large_fills() {
        // 64.4 logical pixels on 64: a ten-pixel mask drifts by 0.06 pixel,
        // more than the placement allows, so it is placed as given: eleven
        // logical units wide, which the frame draws a little narrower.
        let viewport = Viewport::new(64.4, 48.0, 1.0).unwrap();
        let prepared = VectorRasterizer::default()
            .prepare(&square(viewport, (3.3, 4.0), 1.0))
            .unwrap();
        let (mask, rect) = placed(&prepared);
        assert_eq!(mask.width(), 11);
        assert!((rect[2] - 11.0).abs() < 1e-4, "{rect:?}");
    }

    /// One stroke command: its path, style, origin, scale and clip.
    type Stroke = (Path, StrokeStyle, Point, f32, Option<Rect>);

    /// A scene of strokes over `viewport`, in order.
    fn strokes(viewport: Viewport, strokes: &[Stroke]) -> DisplayList {
        let mut scene = DisplayList {
            viewport,
            ..Default::default()
        };
        for (path, style, origin, scale, clip) in strokes.iter().cloned() {
            let id = VectorId(
                scene
                    .vectors
                    .intern(&VectorGeometry::new(path, VectorStyle::Stroke(style))),
            );
            scene.fills.push(DrawCommand::DrawVector {
                vector: id,
                origin,
                scale,
                color: Color::WHITE,
                clip,
            });
        }
        scene
    }

    fn p(x: f32, y: f32) -> Point {
        Point::new(x, y).unwrap()
    }

    fn width(value: f32) -> StrokeStyle {
        StrokeStyle::new(Length::new(value).unwrap())
    }

    /// The parity corpus: each case a viewport and its strokes.
    fn corpus() -> Result<Vec<(&'static str, DisplayList)>, Box<dyn std::error::Error>> {
        let unit = Viewport::new(64.0, 48.0, 1.0)?;
        let curve = PathBuilder::new()
            .move_to(p(6.0, 30.0))
            .quad_to(p(20.0, 2.0), p(34.0, 30.0))
            .cubic_to(p(40.0, 44.0), p(52.0, 4.0), p(58.0, 24.0))
            .build()?;
        let zigzag = Path::polyline([p(4.0, 40.0), p(20.0, 8.0), p(30.0, 40.0), p(44.0, 10.0)])?;
        let closed = Path::polygon([p(10.0, 10.0), p(50.0, 14.0), p(30.0, 38.0)])?;
        let dot = PathBuilder::new().move_to(p(20.5, 20.25)).close().build()?;
        // The review's near reversal: a tangent neighbor outside the hull.
        let reversal = PathBuilder::new()
            .move_to(p(0.0, 0.0))
            .quad_to(p(-0.001, 0.0), p(100.0, 0.0))
            .build()?;
        let none = Point::ZERO;
        Ok(vec![
            (
                "miter zigzag",
                strokes(unit, &[(zigzag.clone(), width(3.0), none, 1.0, None)]),
            ),
            (
                "round curves",
                strokes(
                    unit,
                    &[(
                        curve.clone(),
                        width(5.0)
                            .with_cap(LineCap::Round)
                            .with_join(LineJoin::Round),
                        none,
                        1.0,
                        None,
                    )],
                ),
            ),
            (
                "square bevel curves",
                strokes(
                    unit,
                    &[(
                        curve.clone(),
                        width(4.0)
                            .with_cap(LineCap::Square)
                            .with_join(LineJoin::Bevel),
                        none,
                        1.0,
                        None,
                    )],
                ),
            ),
            (
                "hairline",
                strokes(unit, &[(zigzag.clone(), width(0.4), none, 1.0, None)]),
            ),
            (
                "clipped",
                strokes(
                    unit,
                    &[(
                        curve.clone(),
                        width(6.0),
                        none,
                        1.0,
                        Some(Rect::new(12.3, 7.6, 25.1, 20.2)?),
                    )],
                ),
            ),
            (
                "fractional origin",
                strokes(
                    unit,
                    &[(closed.clone(), width(2.5), p(0.37, 0.61), 1.0, None)],
                ),
            ),
            (
                "fractional scale and size",
                strokes(
                    Viewport::new(63.3, 47.7, 1.25)?,
                    &[
                        (
                            closed,
                            width(2.0).with_join(LineJoin::Round),
                            none,
                            1.0,
                            None,
                        ),
                        (zigzag, width(1.5), p(0.2, 0.1), 1.0, None),
                    ],
                ),
            ),
            (
                "magnifying view",
                strokes(
                    unit,
                    &[(
                        curve,
                        width(0.5).with_cap(LineCap::Round),
                        // The quadratic's top, (20, 16), lands at (40, 12).
                        p(-200.0, -180.0),
                        12.0,
                        None,
                    )],
                ),
            ),
            (
                "points",
                strokes(
                    unit,
                    &[
                        (
                            dot.clone(),
                            width(5.0).with_cap(LineCap::Round),
                            none,
                            1.0,
                            None,
                        ),
                        (
                            dot,
                            width(4.0).with_cap(LineCap::Square),
                            p(20.0, 4.0),
                            1.0,
                            None,
                        ),
                    ],
                ),
            ),
            (
                "near reversal",
                strokes(
                    unit,
                    &[(
                        reversal,
                        width(2.0)
                            .with_cap(LineCap::Square)
                            .with_join(LineJoin::Round),
                        p(10.2, 10.0),
                        0.4,
                        None,
                    )],
                ),
            ),
            (
                "far in a large frame",
                strokes(
                    Viewport::new(13_107.0, 40.0, 1.25)?,
                    &[(
                        Path::polyline([p(0.0, 0.0), p(9.0, 20.0), p(17.0, 3.0)])?,
                        width(2.0),
                        p(13_080.3, 6.7),
                        1.0,
                        None,
                    )],
                ),
            ),
        ])
    }

    /// Every texel of a stroke's mask is its reference coverage at the frame
    /// pixel, rounded to eight bits, and the mask lies on the stroke's span.
    #[test]
    fn stroke_masks_hold_the_reference_coverage_on_its_span()
    -> Result<(), Box<dyn std::error::Error>> {
        for (name, scene) in corpus()? {
            let lowered = scene.rasterized()?;
            let extent = scene.viewport().frame_extent();
            let mapping =
                crate::FrameMapping::new(extent.width(), extent.height(), scene.viewport())?;
            let [sx, sy, _] = mapping.factors();
            assert_eq!(lowered.len(), scene.len(), "{name}: every stroke draws");
            for (command, drawn) in scene.commands().iter().zip(lowered.commands()) {
                let DrawCommand::DrawVector {
                    vector,
                    origin,
                    scale,
                    clip,
                    ..
                } = *command
                else {
                    unreachable!("the corpus holds strokes");
                };
                let DrawCommand::DrawMask {
                    mask,
                    rect,
                    clip: kept,
                    ..
                } = *drawn
                else {
                    panic!("{name}: a stroke lowers to a mask");
                };
                assert_eq!(kept, None, "{name}: the span is already clipped");
                let stroke = crate::PreparedStroke::new(
                    scene.vector(vector).ok_or("the corpus's vector")?,
                    origin,
                    scale,
                    clip,
                    &mapping,
                )?
                .ok_or("the corpus's strokes draw")?;
                let [x0, y0, x1, y1] = stroke.span();
                let mask = lowered
                    .mask(mask)
                    .ok_or("the lowered scene holds its masks")?;
                assert_eq!(
                    (mask.width(), mask.height()),
                    (x1 - x0, y1 - y0),
                    "{name}: one texel per pixel of the span"
                );
                for y in y0..y1 {
                    for x in x0..x1 {
                        let expected = (stroke.coverage(x, y).coverage() * 255.0).round() as u8;
                        let texel = mask.pixels()[((y - y0) * mask.width() + x - x0) as usize];
                        assert_eq!(texel, expected, "{name}: pixel ({x}, {y})");
                    }
                }
                // The rectangle maps back onto the span's pixel edges.
                let [rx, ry, rw, rh] = rect.values().map(f64::from);
                let bound = 2f64.powi(-8);
                for (actual, edge) in [
                    (rx * sx, x0),
                    (ry * sy, y0),
                    ((rx + rw) * sx, x1),
                    ((ry + rh) * sy, y1),
                ] {
                    assert!(
                        (actual - f64::from(edge)).abs() <= bound,
                        "{name}: edge {actual} against {edge}"
                    );
                }
            }
        }
        Ok(())
    }

    /// The allocation of a scene's only mask, which a reused mask shares.
    fn allocation(scene: &DisplayList) -> *const u8 {
        assert_eq!(scene.masks().len(), 1);
        scene.masks()[0].pixels().as_ptr()
    }

    /// A stroke's mask is reused when every input of its preparation is
    /// equal, and prepared again, as a fresh rasterizer prepares it, when
    /// any one of them changes.
    #[test]
    fn a_stroke_mask_is_reused_only_when_every_input_is_equal()
    -> Result<(), Box<dyn std::error::Error>> {
        let viewport = Viewport::new(32.0, 32.0, 1.0)?;
        let path = Path::polyline([p(4.0, 20.0), p(14.0, 6.0), p(26.0, 22.0)])?;
        let style = width(2.0);
        let base: Stroke = (path, style, p(1.0, 1.0), 1.0, None);
        let scene_of =
            |viewport: Viewport, stroke: &Stroke| strokes(viewport, std::slice::from_ref(stroke));
        let mut cache = VectorRasterizer::default();
        let first = cache.prepare(&scene_of(viewport, &base))?;
        let again = cache.prepare(&scene_of(viewport, &base))?;
        assert_eq!(
            allocation(&first),
            allocation(&again),
            "an unchanged stroke"
        );
        assert_eq!(first, again);

        let mut moved_path = base.clone();
        moved_path.0 = Path::polyline([p(4.0, 20.0), p(14.0, 7.0), p(26.0, 22.0)])?;
        let mut capped = base.clone();
        capped.1 = style.with_cap(LineCap::Round);
        let mut moved = base.clone();
        moved.2 = p(2.0, 1.0);
        let mut scaled = base.clone();
        scaled.3 = 1.001;
        let mut clipped = base.clone();
        clipped.4 = Some(Rect::new(0.0, 0.0, 12.0, 32.0)?);
        let changes = [
            ("path", viewport, moved_path),
            ("style", viewport, capped),
            ("origin", viewport, moved),
            ("command scale", viewport, scaled),
            ("clip", viewport, clipped),
            // The frame extent stays 32 by 32 pixels: one factor changes.
            ("sx", Viewport::new(32.2, 32.0, 1.0)?, base.clone()),
            ("sy", Viewport::new(32.0, 32.2, 1.0)?, base.clone()),
            (
                "viewport scale",
                Viewport::new(32.0, 32.0, 1.001)?,
                base.clone(),
            ),
            ("frame", Viewport::new(40.0, 32.0, 1.0)?, base.clone()),
        ];
        for (name, changed_viewport, changed) in changes {
            // A cache holding the base alone, as after a frame of it.
            let mut cache = VectorRasterizer::default();
            let reference = cache.prepare(&scene_of(viewport, &base))?;
            let scene = scene_of(changed_viewport, &changed);
            let prepared = cache.prepare(&scene)?;
            assert_ne!(
                allocation(&prepared),
                allocation(&reference),
                "{name}: prepared again"
            );
            assert_eq!(
                prepared,
                VectorRasterizer::default().prepare(&scene)?,
                "{name}: as a fresh preparation"
            );
        }
        Ok(())
    }

    /// In a frame far beyond 16 384 pixels, narrowing the mask's rectangle
    /// to `f32` can move an edge across a pixel center; such a mask keeps
    /// the command's clip, so it never draws outside it, where a mask
    /// placed on the pixels carries none.
    #[test]
    fn a_mask_off_the_pixel_edges_keeps_its_clip() -> Result<(), Box<dyn std::error::Error>> {
        let wide = Viewport::from_physical(
            crate::PhysicalExtent::new(16_777_216, 1),
            2.672_406_447_681_879_6,
        )?;
        let clip = Rect::new(5_319_706.5, 0.0, 89.976_59, 1.0)?;
        let line = Path::polyline([p(5_319_690.0, 0.187), p(5_319_810.0, 0.187)])?;
        let lowered = strokes(
            wide,
            &[(line.clone(), width(1.0), Point::ZERO, 1.0, Some(clip))],
        )
        .rasterized()?;
        let DrawCommand::DrawMask {
            rect, clip: kept, ..
        } = lowered.commands()[0]
        else {
            panic!("a stroke lowers to a mask");
        };
        let extent = wide.frame_extent();
        let mapping = crate::FrameMapping::new(extent.width(), extent.height(), wide)?;
        assert!(
            !on_pixels(rect, &mapping),
            "the placement leaves the pixel edges"
        );
        assert_eq!(kept, Some(clip));
        // Within the range the same stroke's mask lies on the pixels.
        let near = strokes(
            Viewport::new(64.0, 8.0, 2.672_406_447_681_879_6)?,
            &[(
                line,
                width(1.0),
                p(-5_319_680.0, 0.0),
                1.0,
                Some(Rect::new(10.5, 0.0, 30.0, 1.0)?),
            )],
        )
        .rasterized()?;
        let DrawCommand::DrawMask { clip: kept, .. } = near.commands()[0] else {
            panic!("a stroke lowers to a mask");
        };
        assert_eq!(kept, None);
        Ok(())
    }

    /// A refused stroke leaves the last successful cache, which the next
    /// preparation reuses.
    #[test]
    fn a_refused_stroke_keeps_the_previous_cache() -> Result<(), Box<dyn std::error::Error>> {
        let viewport = Viewport::new(32.0, 32.0, 1.0)?;
        let stroke: Stroke = (
            Path::polyline([p(4.0, 4.0), p(28.0, 20.0)])?,
            width(3.0),
            Point::ZERO,
            1.0,
            None,
        );
        let mut cache = VectorRasterizer::default();
        let first = cache.prepare(&strokes(viewport, std::slice::from_ref(&stroke)))?;
        let mut refused = stroke.clone();
        refused.3 = f32::NAN;
        assert!(
            cache
                .prepare(&strokes(viewport, &[stroke.clone(), refused]))
                .is_err()
        );
        let after = cache.prepare(&strokes(viewport, &[stroke]))?;
        assert_eq!(allocation(&after), allocation(&first));
        Ok(())
    }

    /// Each way a stroke can be refused reaches `prepare` with its error.
    #[test]
    fn stroke_refusals_reach_prepare() -> Result<(), Box<dyn std::error::Error>> {
        let refusal = |viewport: Viewport, path: Path, style: StrokeStyle| {
            VectorRasterizer::default()
                .prepare(&strokes(viewport, &[(path, style, Point::ZERO, 1.0, None)]))
                .err()
        };
        // Five thousand segments within one tile exceed its list.
        let crowded =
            (0..5000).map(|i| p(2.0 + i as f32 * 0.002, if i % 2 == 0 { 2.0 } else { 12.0 }));
        assert_eq!(
            refusal(
                Viewport::new(32.0, 32.0, 1.0)?,
                Path::polyline(crowded)?,
                width(1.0)
            ),
            Some(DrawError::TooComplex)
        );
        // A frame taller than the evaluation range.
        assert_eq!(
            refusal(
                Viewport::new(8.0, 70_000.0, 1.0)?,
                Path::polyline([p(4.0, -10.0), p(4.0, 70_010.0)])?,
                width(1.0)
            ),
            Some(DrawError::Stroke(crate::StrokePrepareError::OutOfRange))
        );
        // A curve whose flattening exceeds its budget.
        let wild = PathBuilder::new()
            .move_to(Point::ZERO)
            .quad_to(p(0.0, f32::MAX), p(1.0, 0.0))
            .build()?;
        assert_eq!(
            refusal(Viewport::new(32.0, 32.0, 1.0)?, wild, width(1.0)),
            Some(DrawError::Path(PathError::TooLarge))
        );
        // A span whose mask would exceed the mask limit.
        assert_eq!(
            refusal(
                Viewport::new(4100.0, 4100.0, 1.0)?,
                Path::polyline([p(-10.0, 2050.0), p(4110.0, 2050.0)])?,
                width(4200.0)
            ),
            Some(DrawError::CoverageTooLarge)
        );
        Ok(())
    }

    /// A viewport whose frame has no pixels draws no stroke and builds no
    /// mapping; scenes of only fills or only strokes prepare as before.
    #[test]
    fn a_frame_without_pixels_skips_its_strokes() -> Result<(), Box<dyn std::error::Error>> {
        let stroke: Stroke = (
            Path::polyline([p(0.0, 0.0), p(0.3, 0.3)])?,
            width(1.0),
            Point::ZERO,
            1.0,
            None,
        );
        let tiny = Viewport::new(0.3, 0.3, 1.0)?;
        assert_eq!(tiny.frame_extent().width(), 0);
        let lowered = strokes(tiny, std::slice::from_ref(&stroke)).rasterized()?;
        assert!(lowered.is_empty());
        assert_eq!(
            strokes(Viewport::new(8.0, 8.0, 1.0)?, &[stroke])
                .rasterized()?
                .len(),
            1
        );
        assert_eq!(scene(Color::WHITE, 16.0).rasterized()?.len(), 1);
        Ok(())
    }

    /// A long trace whose outline fill would exceed the coverage budget is
    /// within the stroke's budgets; the same points filled are still refused.
    #[test]
    fn the_coverage_budget_bounds_fills_only() -> Result<(), Box<dyn std::error::Error>> {
        let count = 12_000;
        let trace = || {
            Path::polyline((0..count).map(|i| {
                p(
                    i as f32 * 2000.0 / count as f32,
                    if i % 2 == 0 { 0.0 } else { 500.0 },
                )
            }))
        };
        let viewport = Viewport::new(2000.0, 500.0, 1.0)?;
        assert!(
            strokes(viewport, &[(trace()?, width(1.0), Point::ZERO, 1.0, None)])
                .rasterized()
                .is_ok()
        );
        let mut filled = DisplayList {
            viewport,
            ..Default::default()
        };
        let id = VectorId(filled.vectors.intern(&VectorGeometry::new(
            trace()?,
            VectorStyle::Fill(FillRule::NonZero),
        )));
        filled.fills.push(DrawCommand::DrawVector {
            vector: id,
            origin: Point::ZERO,
            scale: 1.0,
            color: Color::WHITE,
            clip: None,
        });
        assert_eq!(filled.rasterized().err(), Some(DrawError::TooComplex));
        Ok(())
    }

    #[test]
    fn a_non_finite_scale_is_non_finite_and_a_non_positive_one_is_not_positive() {
        use crate::{GeometryError, RuntimeError};
        let with_scale = |scale: f32| {
            let mut scene = scene(Color::WHITE, 16.0);
            if let Some(DrawCommand::DrawVector { scale: s, .. }) = scene.fills.first_mut() {
                *s = scale;
            }
            scene
        };
        let mut cache = VectorRasterizer::default();
        for (scale, expected) in [
            (f32::NAN, GeometryError::NonFinite),
            (f32::INFINITY, GeometryError::NonFinite),
            (f32::NEG_INFINITY, GeometryError::NonFinite),
            (0.0, GeometryError::NonPositiveScale),
            (-1.0, GeometryError::NonPositiveScale),
        ] {
            assert_eq!(
                cache.prepare(&with_scale(scale)),
                Err(DrawError::Runtime(RuntimeError::Geometry(expected))),
                "scale {scale}"
            );
        }
    }
}
