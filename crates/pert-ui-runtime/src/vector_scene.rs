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
#[derive(Clone, PartialEq)]
struct Key {
    vector: VectorGeometry,
    scale: f64,
    clip: Rect,
}
type Coverage = Option<(MaskImage, Rect)>;
/// CPU vector fallback for graphics adapters. Retain one per scene stream.
///
/// Keeps coverage only for resources used by the last successful preparation.
/// Equal geometry and effective scale reuse coverage regardless of tint when the
/// clip leaves the same part of the geometry visible, so a window resize that
/// keeps a shape wholly visible does not rasterize it again. Work and
/// allocations happen after scene recording, outside time-critical callbacks.
/// The cache is bounded by the current/previous scene, not its history.
///
/// Curve subdivision targets a maximum chord deviation of 0.2 physical pixels.
/// Flattening admits at most [`Self::MAX_FLATTENED_POINTS`] generated points
/// per vector, including subpath starts. If the requested subdivision cannot
/// fit, preparation returns [`DrawError::Path`] holding
/// [`PathError::TooLarge`](crate::PathError::TooLarge) instead of silently
/// reducing the subdivision count. The stroke outline has a separate budget of
/// [`Self::MAX_STROKE_EDGES`] edges when expanded to filled polygons.
///
/// This is a fallback for vectors within a graphics backend, not a software
/// window renderer. Backends can instead consume the original geometry directly.
#[derive(Default)]
pub struct VectorRasterizer {
    cache: std::collections::HashMap<u64, Vec<(Key, Coverage)>>,
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
    /// The most edges one stroke's outline may expand to as filled polygons:
    /// 1,048,576 (2^20), separate from [`Self::MAX_FLATTENED_POINTS`]. A
    /// stroke over it is rejected with [`DrawError::Path`] holding
    /// [`PathError::TooLarge`](crate::PathError::TooLarge).
    pub const MAX_STROKE_EDGES: usize = crate::geometry::MAX_EDGES;
    /// The most sub-scanline steps covering one vector may take: 16,777,216
    /// (2^24). Coverage is
    /// sampled on four sub-scanlines per physical pixel row, and each edge of
    /// a fill's outline, or each piece of a stroke of straight segments, costs
    /// one step for every sub-scanline it crosses inside the cropped mask. A
    /// vector over the budget is rejected with [`DrawError::TooComplex`]
    /// before any coverage work, which bounds preparation time and the memory
    /// held for a stroke's spans: at most one span (24 bytes on 64-bit
    /// targets) per step, besides the mask itself, however tall and narrow the
    /// mask is. A trace of thousands of points that crosses
    /// a few hundred pixels per point stays well inside it.
    pub const MAX_COVERAGE_WORK: usize = 1 << 24;

    /// Resolves vectors to masks while preserving command order and clipping.
    ///
    /// Rasterization uses the scene's scale times each command's own `scale`,
    /// held at eight physical pixels per geometry unit when a view magnifies
    /// further (coverage is stretched beyond that), and the viewport.
    /// On failure
    /// both the input and the last successful cache remain usable.
    ///
    /// # Errors
    /// Returns [`DrawError::InvalidImage`] for a vector, mask or image reference that is
    /// not in `scene`, or for cropped coverage above
    /// [`MaskImage::MAX_PIXELS`](crate::MaskImage::MAX_PIXELS); [`DrawError::Path`] with
    /// [`PathError::TooLarge`](crate::PathError::TooLarge) when flattening exceeds its
    /// segment budget; [`DrawError::TooComplex`] when covering a vector exceeds
    /// [`Self::MAX_COVERAGE_WORK`]; and [`DrawError::Runtime`] holding
    /// [`RuntimeError::Geometry`](crate::RuntimeError::Geometry) with
    /// [`GeometryError::NonFinite`](crate::GeometryError::NonFinite) for a NaN or infinite
    /// command scale or a derived coordinate that is not finite, and with
    /// [`GeometryError::NonPositiveScale`](crate::GeometryError::NonPositiveScale) for a
    /// finite command scale that is zero or negative. A command that is wholly clipped
    /// or fully transparent is skipped before its scale is checked.
    pub fn prepare(&mut self, scene: &DisplayList) -> Result<DisplayList, DrawError> {
        let mut result = DisplayList {
            background: scene.background,
            viewport: scene.viewport,
            ..Default::default()
        };
        let mut next: std::collections::HashMap<u64, Vec<(Key, Coverage)>> =
            std::collections::HashMap::new();
        let viewport = Rect::new(0.0, 0.0, scene.viewport.width(), scene.viewport.height())?;
        for command in scene.commands() {
            let command = match *command {
                DrawCommand::DrawVector {
                    vector,
                    origin,
                    scale,
                    color,
                    clip,
                } => {
                    let vector = scene.vector(vector).ok_or(DrawError::InvalidImage)?;
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
                    // The clip in the geometry's own units, which a view transform
                    // magnifies by `scale` on the way to the window.
                    let local = Rect::new(
                        (clip.x() - origin.x()) / scale,
                        (clip.y() - origin.y()) / scale,
                        clip.width() / scale,
                        clip.height() / scale,
                    )?;
                    // Prepared at the displayed size up to the ceiling every
                    // magnifying view shares; the destination below still uses
                    // the full factor, so coverage beyond it is stretched.
                    let raster_scale = crate::runtime::raster_scale(scene.viewport.scale(), scale);
                    // Only the visible part of the coverage matters, so a clip
                    // change that leaves it whole (a window resize) still hits.
                    let clip = crate::geometry::coverage_bounds(vector, raster_scale)
                        .map_or(local, |bounds| local.intersection(bounds));
                    let key = Key {
                        vector: vector.clone(),
                        scale: raster_scale,
                        clip,
                    };
                    let bucket = vector.content_key();
                    let cached = next
                        .get(&bucket)
                        .into_iter()
                        .flatten()
                        .chain(self.cache.get(&bucket).into_iter().flatten())
                        .find(|(k, _)| k == &key)
                        .map(|(_, v)| v.clone());
                    let coverage = match cached {
                        Some(c) => c,
                        None => {
                            crate::geometry::rasterize_vector(vector, key.scale, Some(key.clip))?
                        }
                    };
                    let entries = next.entry(bucket).or_default();
                    if !entries.iter().any(|(k, _)| k == &key) {
                        entries.push((key, coverage.clone()));
                    }
                    let Some((mask, rect)) = coverage else {
                        continue;
                    };
                    let rect = Rect::new(
                        rect.x() * scale + origin.x(),
                        rect.y() * scale + origin.y(),
                        rect.width() * scale,
                        rect.height() * scale,
                    )?;
                    DrawCommand::DrawMask {
                        mask: MaskId(result.masks.intern(&mask)),
                        rect,
                        color,
                        turn: crate::QuarterTurn::None,
                        clip: command_clip,
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
                            .intern(scene.mask(mask).ok_or(DrawError::InvalidImage)?),
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
                            .intern(scene.image(image).ok_or(DrawError::InvalidImage)?),
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
            Err(DrawError::InvalidImage)
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
        let width = crate::Length::new(3.0).unwrap();
        let styles = [
            VectorStyle::Fill(FillRule::EvenOdd),
            VectorStyle::Stroke(StrokeStyle::new(width)),
            VectorStyle::Stroke(
                StrokeStyle::new(width)
                    .with_cap(LineCap::Square)
                    .with_miter_limit(10.0)
                    .unwrap(),
            ),
            VectorStyle::Stroke(
                StrokeStyle::new(width)
                    .with_cap(LineCap::Round)
                    .with_join(LineJoin::Round),
            ),
        ];
        let clips = [
            Rect::new(-100.0, -100.0, 400.0, 400.0).unwrap(),
            Rect::new(-50.0, 3.0, 400.0, 7.5).unwrap(),
            Rect::new(5.25, -50.0, 3.5, 400.0).unwrap(),
            Rect::new(30.0, 30.0, 10.0, 10.0).unwrap(),
        ];
        for style in styles {
            let vector = VectorGeometry::new(path.clone(), style);
            for scale in [1.0, 1.5, 2.0] {
                let bounds = crate::geometry::coverage_bounds(&vector, scale).unwrap();
                let whole = crate::geometry::rasterize_vector(&vector, scale, None).unwrap();
                assert_eq!(
                    crate::geometry::rasterize_vector(&vector, scale, Some(bounds)).unwrap(),
                    whole,
                    "the bounds contain all coverage"
                );
                for clip in clips {
                    assert_eq!(
                        crate::geometry::rasterize_vector(
                            &vector,
                            scale,
                            Some(clip.intersection(bounds))
                        )
                        .unwrap(),
                        crate::geometry::rasterize_vector(&vector, scale, Some(clip)).unwrap()
                    );
                }
            }
        }
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
