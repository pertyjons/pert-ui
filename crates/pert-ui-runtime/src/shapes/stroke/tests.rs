use super::*;
use crate::{FillRule, Length, Path, PathBuilder, StrokeStyle, Viewport};

type Outcome = Result<(), Box<dyn std::error::Error>>;

fn mapping(width: f32, height: f32, scale: f64) -> Result<FrameMapping, GeometryError> {
    let viewport = Viewport::new(width, height, scale)?;
    let extent = viewport.frame_extent();
    FrameMapping::new(extent.width(), extent.height(), viewport)
}

fn points(values: &[(f32, f32)]) -> Result<Vec<Point>, GeometryError> {
    values.iter().map(|&(x, y)| Point::new(x, y)).collect()
}

fn style(width: f32, cap: LineCap, join: LineJoin) -> Result<StrokeStyle, GeometryError> {
    Ok(StrokeStyle::new(Length::new(width)?)
        .with_cap(cap)
        .with_join(join))
}

fn polyline(
    values: &[(f32, f32)],
    width: f32,
    cap: LineCap,
    join: LineJoin,
) -> Result<VectorGeometry, Box<dyn std::error::Error>> {
    Ok(VectorGeometry::new(
        Path::polyline(points(values)?)?,
        VectorStyle::Stroke(style(width, cap, join)?),
    ))
}

fn polygon(
    values: &[(f32, f32)],
    width: f32,
    join: LineJoin,
) -> Result<VectorGeometry, Box<dyn std::error::Error>> {
    Ok(VectorGeometry::new(
        Path::polygon(points(values)?)?,
        VectorStyle::Stroke(style(width, LineCap::Butt, join)?),
    ))
}

/// Prepared at the window's origin, unscaled and unclipped.
fn prepared(
    vector: &VectorGeometry,
    mapping: &FrameMapping,
) -> Result<PreparedStroke, Box<dyn std::error::Error>> {
    PreparedStroke::new(vector, Point::ZERO, 1.0, None, mapping)?
        .ok_or_else(|| "the stroke prepared to nothing".into())
}

/// The smallest candidate over every segment at the frame point `(x, y)`,
/// without the tile lists.
fn at(stroke: &PreparedStroke, x: f32, y: f32) -> f32 {
    let p = [x - stroke.span[0] as f32, y - stroke.span[1] as f32];
    stroke
        .segments
        .iter()
        .map(|segment| segment.distance(stroke.h, p))
        .fold(f32::INFINITY, f32::min)
}

/// As [`at`], at the center of a frame pixel that may lie outside the span.
fn at_pixel(stroke: &PreparedStroke, x: i64, y: i64) -> f32 {
    let p = [
        (x - i64::from(stroke.span[0])) as f32 + 0.5,
        (y - i64::from(stroke.span[1])) as f32 + 0.5,
    ];
    stroke
        .segments
        .iter()
        .map(|segment| segment.distance(stroke.h, p))
        .fold(f32::INFINITY, f32::min)
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

const CAPS: [LineCap; 3] = [LineCap::Butt, LineCap::Round, LineCap::Square];
const JOINS: [LineJoin; 3] = [LineJoin::Round, LineJoin::Bevel, LineJoin::Miter];

/// A small deterministic generator for the invariant loops.
struct Lcg(u64);

impl Lcg {
    /// The next value in `0..1`.
    fn next(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
}

#[test]
fn a_horizontal_line_covers_its_rows() -> Outcome {
    let mapping = mapping(64.0, 64.0, 1.0)?;
    let line = polyline(
        &[(8.0, 20.0), (40.0, 20.0)],
        2.0,
        LineCap::Butt,
        LineJoin::Miter,
    )?;
    let stroke = prepared(&line, &mapping)?;
    assert_eq!((stroke.half_width(), stroke.thin()), (1.0, 1.0));
    let coverage = |x, y| stroke.coverage(x, y).coverage();
    assert_eq!(coverage(20, 19), 1.0);
    assert_eq!(coverage(20, 20), 1.0);
    assert_eq!(coverage(20, 18), 0.0);
    assert_eq!(coverage(20, 21), 0.0);
    // The butt ends at x = 8 and x = 40.
    assert_eq!(coverage(7, 19), 0.0);
    assert_eq!(coverage(8, 19), 1.0);
    assert_eq!(coverage(39, 19), 1.0);
    assert_eq!(coverage(40, 19), 0.0);
    // Half a pixel above the edge row's centers: distance 0.5 and -0.5.
    assert_eq!(stroke.coverage(20, 18).distance(), 0.5);
    assert_eq!(stroke.coverage(20, 19).distance(), -0.5);
    Ok(())
}

#[test]
fn caps_have_their_hand_computed_distances() -> Outcome {
    let mapping = mapping(64.0, 64.0, 1.0)?;
    // h = 3, from (8, 20) to (40, 20).
    let line = |cap| -> Result<PreparedStroke, Box<dyn std::error::Error>> {
        prepared(
            &polyline(&[(8.0, 20.0), (40.0, 20.0)], 6.0, cap, LineJoin::Miter)?,
            &mapping,
        )
    };
    let (butt, round, square) = (
        line(LineCap::Butt)?,
        line(LineCap::Round)?,
        line(LineCap::Square)?,
    );
    // Two pixels beyond the start, on the line.
    assert_eq!(at(&butt, 6.0, 20.0), 2.0);
    assert_eq!(at(&round, 6.0, 20.0), -1.0);
    assert_eq!(at(&square, 6.0, 20.0), -1.0);
    // Two beyond and four across: outside all three.
    assert_eq!(at(&butt, 6.0, 24.0), 2.0);
    assert!(close(at(&round, 6.0, 24.0), 20f32.sqrt() - 3.0));
    assert_eq!(at(&square, 6.0, 24.0), 1.0);
    // The same at the end.
    assert_eq!(at(&butt, 42.0, 20.0), 2.0);
    assert_eq!(at(&round, 42.0, 20.0), -1.0);
    assert_eq!(at(&square, 42.0, 24.0), 1.0);
    // Inside the body the across distance decides.
    for stroke in [&butt, &round, &square] {
        assert_eq!(at(stroke, 24.0, 22.0), -1.0);
        assert_eq!(at(stroke, 24.0, 16.0), 1.0);
    }
    Ok(())
}

#[test]
fn joins_have_their_hand_computed_distances() -> Outcome {
    let mapping = mapping(64.0, 64.0, 1.0)?;
    // h = 2, a right-angle turn at (30, 10) whose outer corner is (32, 8).
    let corner = |join| -> Result<PreparedStroke, Box<dyn std::error::Error>> {
        prepared(
            &polyline(
                &[(10.0, 10.0), (30.0, 10.0), (30.0, 30.0)],
                4.0,
                LineCap::Butt,
                join,
            )?,
            &mapping,
        )
    };
    let (round, bevel, miter) = (
        corner(LineJoin::Round)?,
        corner(LineJoin::Bevel)?,
        corner(LineJoin::Miter)?,
    );
    // In the wedge, 1.5 out along both axes: q = (1.5, -1.5).
    let diagonal = 4.5f32.sqrt();
    assert!(close(at(&round, 31.5, 8.5), diagonal - 2.0));
    assert!(close(at(&bevel, 31.5, 8.5), diagonal - 2f32.sqrt()));
    assert!(close(at(&miter, 31.5, 8.5), -0.5));
    // Beyond the miter's tip.
    assert!(close(at(&miter, 32.5, 7.0), 1.0));
    // On the inner side both bodies cover, the nearer edge deciding.
    for stroke in [&round, &bevel, &miter] {
        assert!(close(at(stroke, 29.0, 11.5), -1.0));
        assert!(close(at(stroke, 20.0, 9.0), -1.0));
    }
    // The opposite turn mirrors it: the outer corner is (32, 32).
    let mirrored = prepared(
        &polyline(
            &[(10.0, 30.0), (30.0, 30.0), (30.0, 10.0)],
            4.0,
            LineCap::Butt,
            LineJoin::Miter,
        )?,
        &mapping,
    )?;
    assert!(close(at(&mirrored, 31.5, 31.5), -0.5));
    assert!(close(at(&mirrored, 32.5, 33.0), 1.0));
    Ok(())
}

#[test]
fn a_miter_beyond_its_limit_is_a_bevel() -> Outcome {
    let mapping = mapping(64.0, 64.0, 1.0)?;
    // A turn of about 152 degrees: 1 / cos of its half is about 4.1.
    let sharp = [(10.0, 20.0), (40.0, 20.0), (12.0, 35.0)];
    let with_limit = |limit: f32| -> Result<PreparedStroke, Box<dyn std::error::Error>> {
        let style = style(4.0, LineCap::Butt, LineJoin::Miter)?.with_miter_limit(limit)?;
        prepared(
            &VectorGeometry::new(Path::polyline(points(&sharp)?)?, VectorStyle::Stroke(style)),
            &mapping,
        )
    };
    let bevel = prepared(
        &polyline(&sharp, 4.0, LineCap::Butt, LineJoin::Bevel)?,
        &mapping,
    )?;
    assert_eq!(with_limit(4.0)?, bevel);
    let miter = with_limit(5.0)?;
    assert_ne!(miter, bevel);
    // Three pixels past the vertex along the first direction.
    assert!(at(&bevel, 43.0, 19.0) > 0.5);
    assert!(at(&miter, 43.0, 19.0) < 0.0);
    Ok(())
}

#[test]
fn a_reversal_has_a_half_disc_or_nothing() -> Outcome {
    let mapping = mapping(64.0, 64.0, 1.0)?;
    let reversal = [(10.0, 20.0), (40.0, 20.0), (20.0, 20.0)];
    for join in JOINS {
        let stroke = prepared(&polyline(&reversal, 6.0, LineCap::Butt, join)?, &mapping)?;
        let beyond = at(&stroke, 42.0, 20.0);
        if join == LineJoin::Round {
            assert_eq!(beyond, -1.0, "{join:?}");
            assert!(close(at(&stroke, 42.0, 22.0), 8f32.sqrt() - 3.0));
        } else {
            assert!(beyond >= 2.0, "{join:?}: {beyond}");
        }
        // One pixel short of the vertex: the round join's disc reaches
        // on, the others end at the vertex's line.
        let short = if join == LineJoin::Round { -2.0 } else { -1.0 };
        assert_eq!(at(&stroke, 39.0, 21.0), short, "{join:?}");
    }
    Ok(())
}

#[test]
fn points_follow_their_cap() -> Outcome {
    let mapping = mapping(64.0, 64.0, 1.0)?;
    let dot = |cap| -> Result<Option<PreparedStroke>, Box<dyn std::error::Error>> {
        let path = PathBuilder::new()
            .move_to(Point::new(20.0, 20.0)?)
            .close()
            .build()?;
        let vector =
            VectorGeometry::new(path, VectorStyle::Stroke(style(6.0, cap, LineJoin::Miter)?));
        Ok(PreparedStroke::new(
            &vector,
            Point::ZERO,
            1.0,
            None,
            &mapping,
        )?)
    };
    assert_eq!(dot(LineCap::Butt)?, None);
    let round = dot(LineCap::Round)?.ok_or("no disc")?;
    let square = dot(LineCap::Square)?.ok_or("no square")?;
    assert_eq!(at(&round, 24.0, 20.0), 1.0);
    assert!(close(at(&round, 22.0, 22.0), 8f32.sqrt() - 3.0));
    assert_eq!(at(&square, 22.0, 22.0), -1.0);
    assert_eq!(at(&square, 24.0, 21.0), 1.0);
    Ok(())
}

#[test]
fn a_stroke_narrower_than_a_pixel_fades_by_its_width() -> Outcome {
    let mapping = mapping(64.0, 64.0, 1.0)?;
    let thin = prepared(
        &polyline(
            &[(8.0, 20.5), (40.0, 20.5)],
            0.25,
            LineCap::Butt,
            LineJoin::Miter,
        )?,
        &mapping,
    )?;
    assert_eq!((thin.half_width(), thin.thin()), (0.5, 0.25));
    assert_eq!(thin.coverage(20, 20).coverage(), 0.25);
    assert_eq!(thin.coverage(20, 19).coverage(), 0.0);
    assert_eq!(thin.coverage(20, 21).coverage(), 0.0);
    // Half way between two rows each takes half of it.
    let between = prepared(
        &polyline(
            &[(8.0, 20.0), (40.0, 20.0)],
            0.25,
            LineCap::Butt,
            LineJoin::Miter,
        )?,
        &mapping,
    )?;
    assert_eq!(between.coverage(20, 19).coverage(), 0.125);
    assert_eq!(between.coverage(20, 20).coverage(), 0.125);
    Ok(())
}

/// A random open or closed path whose segments are long against its width,
/// so that neither of the two discontinuities the reference keeps occurs.
fn long_path(random: &mut Lcg, closed: bool) -> Vec<(f32, f32)> {
    let mut path: Vec<(f32, f32)> = Vec::new();
    while path.len() < 6 {
        let p = (20.0 + 88.0 * random.next(), 20.0 + 88.0 * random.next());
        let far = |q: &(f32, f32)| (p.0 - q.0).hypot(p.1 - q.1) >= 24.0;
        let closes = !closed || path.len() < 5 || far(&path[0]);
        if path.last().is_none_or(far) && closes {
            path.push(p);
        }
    }
    path
}

#[test]
fn the_distance_is_continuous_across_every_region_boundary() -> Outcome {
    let mapping = mapping(128.0, 128.0, 1.0)?;
    let mut random = Lcg(7);
    for case in 0..60 {
        let closed = case % 3 == 0;
        let path = long_path(&mut random, closed);
        for join in JOINS {
            for cap in CAPS {
                let vector = if closed {
                    polygon(&path, 5.0, join)?
                } else {
                    polyline(&path, 5.0, cap, join)?
                };
                let stroke = prepared(&vector, &mapping)?;
                // Every candidate is 1-Lipschitz, so a jump of the smallest
                // one is a boundary where two regions disagree.
                for _ in 0..4000 {
                    let (x, y) = (128.0 * random.next(), 128.0 * random.next());
                    let turn = std::f32::consts::TAU * random.next();
                    let step = 1.0 / 64.0;
                    let (near, far) = (
                        at(&stroke, x, y),
                        at(&stroke, x + step * turn.cos(), y + step * turn.sin()),
                    );
                    assert!(
                        (near - far).abs() <= step + 1e-3,
                        "case {case} {join:?} {cap:?} at ({x}, {y}): {near} then {far}"
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn each_boundary_of_a_right_angle_agrees_from_both_sides() -> Outcome {
    let mapping = mapping(64.0, 64.0, 1.0)?;
    let step = 1.0 / 256.0;
    for join in JOINS {
        for cap in CAPS {
            let stroke = prepared(
                &polyline(&[(10.0, 10.0), (30.0, 10.0), (30.0, 30.0)], 4.0, cap, join)?,
                &mapping,
            )?;
            let across = |x0: f32, y0: f32, x1: f32, y1: f32| {
                let (before, after) = (at(&stroke, x0, y0), at(&stroke, x1, y1));
                assert!(
                    (before - after).abs() <= 2.0 * step,
                    "{join:?} {cap:?} ({x0}, {y0}) {before}, ({x1}, {y1}) {after}"
                );
            };
            for offset in [0.25f32, 1.0, 1.75, 2.0, 2.5] {
                // The first body's end line, on the outer side: body, join.
                across(30.0 - step, 10.0 - offset, 30.0 + step, 10.0 - offset);
                // The second body's start line, on the outer side.
                across(30.0 + offset, 10.0 - step, 30.0 + offset, 10.0 + step);
                // Both end lines on the inner side, where the other body
                // covers.
                across(30.0 - step, 10.0 + offset, 30.0 + step, 10.0 + offset);
                across(30.0 - offset, 10.0 - step, 30.0 - offset, 10.0 + step);
                // The cap lines at both ends, inside and beside the stroke.
                across(10.0 - step, 10.0 + offset, 10.0 + step, 10.0 + offset);
                across(30.0 + offset, 30.0 - step, 30.0 + offset, 30.0 + step);
            }
            // The bisector through the wedge, where the join's halves meet.
            across(31.0 - step, 9.0 - step, 31.0 + step, 9.0 + step);
        }
    }
    Ok(())
}

#[test]
fn tile_lists_hold_every_segment_that_covers_a_pixel() -> Outcome {
    let mapping = mapping(96.0, 96.0, 1.0)?;
    let threshold = 0.5 + PARITY_MARGIN;
    let mut random = Lcg(11);
    for case in 0..24 {
        // A dense wave, a sparse zigzag and sharp turns in turn.
        let path: Vec<(f32, f32)> = match case % 3 {
            0 => (0..60)
                .map(|i| {
                    let t = i as f32;
                    (4.0 + t * 1.4, 48.0 + 30.0 * (t * 0.9).sin())
                })
                .collect(),
            1 => (0..8)
                .map(|_| (96.0 * random.next(), 96.0 * random.next()))
                .collect(),
            _ => (0..12)
                .map(|i| (20.0 + 60.0 * (i % 2) as f32, 10.0 + 6.5 * i as f32))
                .collect(),
        };
        let width = [0.4, 1.0, 3.0, 9.0][case % 4];
        for join in JOINS {
            for cap in CAPS {
                let style = style(width, cap, join)?.with_miter_limit(12.0)?;
                let vector = VectorGeometry::new(
                    Path::polyline(points(&path)?)?,
                    VectorStyle::Stroke(style),
                );
                let stroke = prepared(&vector, &mapping)?;
                let [x0, y0, x1, y1] = stroke.span().map(i64::from);
                for y in -8..104 {
                    for x in -8..104 {
                        let all = at_pixel(&stroke, x, y);
                        if x < x0 || y < y0 || x >= x1 || y >= y1 {
                            // Outside the span nothing is covered. The frame
                            // cuts the span, so only its inside is claimed.
                            if (0..96).contains(&x) && (0..96).contains(&y) {
                                assert!(all >= 0.5, "case {case} {join:?} {cap:?} ({x}, {y})");
                            }
                            continue;
                        }
                        let listed = stroke.coverage(x as u32, y as u32).distance();
                        assert!(
                            listed == all || (all >= threshold && listed >= threshold),
                            "case {case} {join:?} {cap:?} ({x}, {y}): listed {listed}, all {all}"
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

#[test]
fn visiting_the_covered_pixels_gives_the_reference_coverage() -> Outcome {
    let mapping = mapping(96.0, 96.0, 1.25)?;
    let mut random = Lcg(29);
    for case in 0..30 {
        // Dense short segments, long diagonals, sharp turns and curves.
        let path: Vec<(f32, f32)> = match case % 3 {
            0 => (0..300)
                .map(|i| {
                    let t = i as f32;
                    (
                        2.0 + t * 0.3,
                        48.0 + 30.0 * (t * 0.21).sin() + 3.0 * random.next(),
                    )
                })
                .collect(),
            1 => (0..6)
                .map(|_| (96.0 * random.next(), 96.0 * random.next()))
                .collect(),
            _ => (0..14)
                .map(|i| (20.0 + 60.0 * (i % 2) as f32, 8.0 + 6.0 * i as f32))
                .collect(),
        };
        let width = [0.3, 1.0, 2.5, 7.0, 14.0][case % 5];
        for join in JOINS {
            for cap in CAPS {
                let style = style(width, cap, join)?.with_miter_limit(9.0)?;
                let vector = if case % 4 == 3 {
                    VectorGeometry::new(
                        Path::ellipse(Rect::new(10.3, 20.6, 70.0, 40.0)?)?,
                        VectorStyle::Stroke(style),
                    )
                } else {
                    VectorGeometry::new(Path::polyline(points(&path)?)?, VectorStyle::Stroke(style))
                };
                let stroke = prepared(&vector, &mapping)?;
                let mut visited = std::collections::HashMap::new();
                stroke.for_each_covered(|x, y, coverage| {
                    assert!(
                        visited.insert((x, y), coverage).is_none(),
                        "case {case} {join:?} {cap:?}: ({x}, {y}) twice"
                    );
                });
                let [x0, y0, x1, y1] = stroke.span();
                let mut covered = 0;
                for y in y0..y1 {
                    for x in x0..x1 {
                        let reference = stroke.coverage(x, y).coverage();
                        assert_eq!(
                            visited.get(&(x, y)).copied().unwrap_or(0.0),
                            reference,
                            "case {case} {join:?} {cap:?} width {width} at ({x}, {y})"
                        );
                        covered += usize::from(reference > 0.0);
                    }
                }
                assert_eq!(covered, visited.len());
                assert!(covered > 0);
            }
        }
    }
    Ok(())
}

#[test]
fn visiting_covers_far_geometry_a_long_miter_and_a_point_as_the_reference() -> Outcome {
    let mapping = mapping(96.0, 96.0, 1.25)?;
    let far = VectorGeometry::new(
        PathBuilder::new()
            .move_to(Point::new(-1.0e6, 30.0)?)
            .line_to(Point::new(70.0, 40.0)?)
            .line_to(Point::new(-40.0, 44.0)?)
            .move_to(Point::new(80.5, 80.25)?)
            .close()
            .build()?,
        VectorStyle::Stroke(style(3.0, LineCap::Round, LineJoin::Miter)?.with_miter_limit(40.0)?),
    );
    let stroke = prepared(&far, &mapping)?;
    let mut visited = std::collections::HashMap::new();
    stroke.for_each_covered(|x, y, coverage| {
        assert!(
            visited.insert((x, y), coverage).is_none(),
            "({x}, {y}) twice"
        );
    });
    let [x0, y0, x1, y1] = stroke.span();
    for y in y0..y1 {
        for x in x0..x1 {
            assert_eq!(
                visited.get(&(x, y)).copied().unwrap_or(0.0),
                stroke.coverage(x, y).coverage(),
                "({x}, {y})"
            );
        }
    }
    // The line, the miter between its two segments and the point's disc.
    assert!(visited.len() > 300, "{}", visited.len());
    assert!(visited.contains_key(&(100, 100)));
    Ok(())
}

#[test]
fn no_pixel_beyond_the_support_of_a_long_miter_is_covered() -> Outcome {
    // A frame wide enough for miters thousands of pixels long.
    let mapping = FrameMapping::new(40_000, 128, Viewport::new(40_000.0, 128.0, 1.0)?)?;
    let mut random = Lcg(3);
    let mut longest = 0.0f32;
    for case in 0..160 {
        // The path runs right to left into the vertex and almost straight
        // back; the miter points on to the left.
        let width = 1.0 + 7.0 * random.next();
        let ratio = 10f32.powf(1.0 + 2.6 * random.next());
        let vertex = (36_000.0 + random.next(), 60.0 + 8.0 * random.next());
        // 1 / cos of half the turn is `ratio` for a turn of pi - 2 / ratio.
        let back = 2.0 / ratio * if case % 2 == 0 { 1.0 } else { -1.0 };
        let path = [
            (vertex.0 + 300.0, vertex.1),
            vertex,
            (vertex.0 + 300.0 * back.cos(), vertex.1 + 300.0 * back.sin()),
        ];
        let style = style(width, LineCap::Butt, LineJoin::Miter)?.with_miter_limit(30_000.0)?;
        let vector =
            VectorGeometry::new(Path::polyline(points(&path)?)?, VectorStyle::Stroke(style));
        let stroke = prepared(&vector, &mapping)?;
        let [x0, y0, x1, y1] = stroke.span().map(i64::from);
        assert!(x0 > 0, "case {case}: the frame cut the miter");
        // The tip is covered, so the test reaches it.
        let tip = (x0..x0 + 64)
            .flat_map(|x| (y0..y1).map(move |y| (x, y)))
            .map(|(x, y)| at_pixel(&stroke, x, y))
            .fold(f32::INFINITY, f32::min);
        longest = longest.max((vertex.0 - x0 as f32) / width);
        assert!(tip < 64.0, "case {case}: nothing near the span's end");
        // Columns left of the span, and the rows above and below it.
        for x in x0 - 48..x0 {
            for y in 0..128 {
                let distance = at_pixel(&stroke, x, y);
                assert!(
                    distance >= 0.5,
                    "case {case} ratio {ratio} ({x}, {y}): {distance}"
                );
            }
        }
        for x in (x0..x1).step_by(7) {
            for y in (0..y0).chain(y1..128) {
                let distance = at_pixel(&stroke, x, y);
                assert!(
                    distance >= 0.5,
                    "case {case} ratio {ratio} ({x}, {y}): {distance}"
                );
            }
        }
    }
    // The cases include miters over a thousand widths long.
    assert!(longest > 1000.0, "{longest}");
    Ok(())
}

#[test]
fn a_miter_too_long_for_the_domain_is_refused() -> Outcome {
    let mapping = FrameMapping::new(40_000, 128, Viewport::new(40_000.0, 128.0, 1.0)?)?;
    // 1 / cos of half the turn is about 20 000; with h = 4 the support
    // exceeds the domain.
    let back = 1.0e-4f32;
    let path = [
        (36_300.0, 64.0),
        (36_000.0, 64.0),
        (36_000.0 + 300.0 * back.cos(), 64.0 + 300.0 * back.sin()),
    ];
    let style = style(8.0, LineCap::Butt, LineJoin::Miter)?.with_miter_limit(30_000.0)?;
    let vector = VectorGeometry::new(Path::polyline(points(&path)?)?, VectorStyle::Stroke(style));
    assert_eq!(
        PreparedStroke::new(&vector, Point::ZERO, 1.0, None, &mapping),
        Err(StrokePrepareError::OutOfRange.into())
    );
    // The same turn within the default limit is a bevel, and prepares.
    let bevel = polyline(&path, 8.0, LineCap::Butt, LineJoin::Miter)?;
    assert!(PreparedStroke::new(&bevel, Point::ZERO, 1.0, None, &mapping)?.is_some());
    // A width beyond the domain is refused too.
    let wide = polyline(
        &[(10.0, 10.0), (50.0, 10.0)],
        140_000.0,
        LineCap::Butt,
        LineJoin::Miter,
    )?;
    assert_eq!(
        PreparedStroke::new(&wide, Point::ZERO, 1.0, None, &mapping),
        Err(StrokePrepareError::OutOfRange.into())
    );
    Ok(())
}

#[test]
fn subpaths_are_normalized_as_the_mask_coverage_normalizes_them() -> Outcome {
    let mapping = mapping(64.0, 64.0, 1.0)?;
    let plain = [(10.0, 10.0), (30.0, 10.0), (30.0, 30.0)];
    for join in JOINS {
        for cap in CAPS {
            // Repeated points are dropped.
            let repeated = [
                (10.0, 10.0),
                (10.0, 10.0),
                (30.0, 10.0),
                (30.0, 10.0),
                (30.0, 10.0),
                (30.0, 30.0),
            ];
            assert_eq!(
                prepared(&polyline(&repeated, 4.0, cap, join)?, &mapping)?,
                prepared(&polyline(&plain, 4.0, cap, join)?, &mapping)?,
                "{join:?} {cap:?}"
            );
            // A closed subpath of two points is open, with its caps.
            let two = [(10.0, 20.0), (40.0, 20.0)];
            let closed = VectorGeometry::new(
                Path::polygon(points(&two)?)?,
                VectorStyle::Stroke(style(4.0, cap, join)?),
            );
            let open = prepared(&polyline(&two, 4.0, cap, join)?, &mapping)?;
            let closed = prepared(&closed, &mapping)?;
            for (x, y) in [(8.5, 20.5), (20.5, 21.5), (41.5, 19.5), (43.5, 23.5)] {
                assert_eq!(
                    at(&closed, x, y),
                    at(&open, x, y),
                    "{join:?} {cap:?} ({x}, {y})"
                );
            }
        }
        // A closing point equal to the first is dropped: three vertices,
        // three joins and no cap.
        let explicit = PathBuilder::new()
            .move_to(Point::new(10.0, 10.0)?)
            .line_to(Point::new(30.0, 10.0)?)
            .line_to(Point::new(30.0, 30.0)?)
            .line_to(Point::new(10.0, 10.0)?)
            .close()
            .build()?;
        let explicit = VectorGeometry::new(
            explicit,
            VectorStyle::Stroke(style(4.0, LineCap::Butt, join)?),
        );
        assert_eq!(
            prepared(&explicit, &mapping)?,
            prepared(&polygon(&plain, 4.0, join)?, &mapping)?,
            "{join:?}"
        );
        assert_eq!(
            prepared(&polygon(&plain, 4.0, join)?, &mapping)?
                .segments()
                .len(),
            3
        );
    }
    Ok(())
}

#[test]
fn the_mapping_scales_positions_and_widths() -> Outcome {
    let path = [(10.0, 10.0), (30.0, 10.0), (30.0, 30.0)];
    let doubled: Vec<(f32, f32)> = path.iter().map(|&(x, y)| (2.0 * x, 2.0 * y)).collect();
    let single = mapping(128.0, 128.0, 1.0)?;
    for join in JOINS {
        let small = polyline(&path, 3.0, LineCap::Round, join)?;
        let large = prepared(&polyline(&doubled, 6.0, LineCap::Round, join)?, &single)?;
        // The command's scale magnifies points and width alike.
        assert_eq!(
            PreparedStroke::new(&small, Point::ZERO, 2.0, None, &single)?,
            Some(large.clone()),
            "{join:?}"
        );
        // So does the viewport's scale.
        let dense = mapping(64.0, 64.0, 2.0)?;
        assert_eq!(prepared(&small, &dense)?, large, "{join:?}");
        // The origin translates before the scale: whole pixels move the
        // span and nothing else.
        let moved = PreparedStroke::new(&small, Point::new(8.0, 4.0)?, 1.0, None, &single)?
            .ok_or("nothing prepared")?;
        let home = prepared(&small, &single)?;
        let [x0, y0, x1, y1] = home.span();
        assert_eq!(moved.span(), [x0 + 8, y0 + 4, x1 + 8, y1 + 4]);
        assert_eq!(moved.segments(), home.segments());
    }
    // No ceiling on the magnification: twenty times is twenty times as wide.
    let line = polyline(
        &[(2.0, 3.0), (5.0, 3.0)],
        0.5,
        LineCap::Butt,
        LineJoin::Miter,
    )?;
    let magnified =
        PreparedStroke::new(&line, Point::ZERO, 20.0, None, &single)?.ok_or("nothing prepared")?;
    assert_eq!(magnified.half_width(), 5.0);
    assert_eq!(at(&magnified, 70.0, 64.0), -1.0);
    Ok(())
}

#[test]
fn a_clip_and_the_frame_cut_the_span_not_the_coverage() -> Outcome {
    let mapping = mapping(96.0, 96.0, 1.25)?;
    let mut random = Lcg(5);
    let path: Vec<(f32, f32)> = (0..30)
        .map(|_| (96.0 * random.next(), 96.0 * random.next()))
        .collect();
    for join in JOINS {
        let vector = polyline(&path, 2.0, LineCap::Round, join)?;
        let whole = prepared(&vector, &mapping)?;
        let clip = Rect::new(20.25, 15.75, 41.5, 50.5)?;
        let cut = PreparedStroke::new(&vector, Point::ZERO, 1.0, Some(clip), &mapping)?
            .ok_or("nothing prepared")?;
        // The clip's edges snap to the first pixel center at or beyond them.
        assert_eq!(cut.span(), [25, 20, 77, 83]);
        // Segments beyond the clip's support are dropped or cut.
        assert!(cut.segments().len() < whole.segments().len());
        for y in 0..120 {
            for x in 0..120 {
                let inside = (25..77).contains(&x) && (20..83).contains(&y);
                let (full, part) = (whole.coverage(x, y), cut.coverage(x, y));
                if inside {
                    assert!(
                        (full.coverage() - part.coverage()).abs() < 1e-3,
                        "{join:?} ({x}, {y}): {full:?}, {part:?}"
                    );
                } else {
                    assert_eq!(part.coverage(), 0.0);
                }
            }
        }
    }
    // A clip beside the stroke leaves nothing.
    let line = polyline(
        &[(10.0, 10.0), (30.0, 10.0)],
        2.0,
        LineCap::Butt,
        LineJoin::Miter,
    )?;
    let beside = Rect::new(10.0, 40.0, 20.0, 20.0)?;
    assert_eq!(
        PreparedStroke::new(&line, Point::ZERO, 1.0, Some(beside), &mapping)?,
        None
    );
    Ok(())
}

#[test]
fn geometry_far_from_the_frame_covers_as_near_it() -> Outcome {
    let mapping = mapping(64.0, 64.0, 1.0)?;
    // The same sloped line through the frame, with its ends near and a
    // million pixels away.
    let slope = 0.3f32;
    let through = |reach: f32| -> Result<PreparedStroke, Box<dyn std::error::Error>> {
        prepared(
            &polyline(
                &[
                    (32.0 - reach, 32.0 - slope * reach),
                    (32.0 + reach, 32.0 + slope * reach),
                ],
                3.0,
                LineCap::Square,
                LineJoin::Miter,
            )?,
            &mapping,
        )
    };
    let (near, far) = (through(100.0)?, through(1.0e6)?);
    assert_eq!(far.segments().len(), 1);
    for y in 0..64 {
        for x in 0..64 {
            let (a, b) = (near.coverage(x, y), far.coverage(x, y));
            assert!(
                (a.coverage() - b.coverage()).abs() < 2e-3,
                "({x}, {y}): {a:?}, {b:?}"
            );
        }
    }
    // A segment that ends inside the frame keeps its cap; the far end has
    // none to draw.
    let ray = prepared(
        &polyline(
            &[(-1.0e6, 32.0), (32.0, 32.0)],
            4.0,
            LineCap::Round,
            LineJoin::Miter,
        )?,
        &mapping,
    )?;
    assert_eq!(at(&ray, 33.0, 32.0), -1.0);
    assert_eq!(at(&ray, 1.0, 33.0), -1.0);
    Ok(())
}

#[test]
fn remote_end_points_keep_the_line_through_the_frame() -> Outcome {
    let mapping = mapping(64.0, 64.0, 1.0)?;
    let threshold = 0.5 + PARITY_MARGIN;
    // Lines whose end points lie 1e20 pixels away, on both sides or on one:
    // a parameter along such a segment cannot resolve the frame.
    type Case = (&'static [(f32, f32)], (u32, u32));
    let cases: [Case; 5] = [
        (&[(-1e20, -1e20), (1e20, 1e20)], (32, 32)),
        (&[(1e20, 2e20), (-1e20, -2e20)], (16, 32)),
        (&[(-1e20, 20.0), (1e20, 20.0)], (50, 20)),
        (&[(-1e20, -3e20), (30.0, 40.0)], (29, 37)),
        (&[(10.0, 50.0), (3e20, -1e20), (20.0, 20.0)], (22, 19)),
    ];
    for (path, (x, y)) in cases {
        for cap in CAPS {
            let stroke = prepared(&polyline(path, 3.0, cap, LineJoin::Round)?, &mapping)?;
            assert_eq!(
                stroke.coverage(x, y).coverage(),
                1.0,
                "{path:?} {cap:?} at ({x}, {y})"
            );
            let mut covered = 0;
            for y in 0..64 {
                for x in 0..64 {
                    let all = at_pixel(&stroke, x, y);
                    let listed = stroke.coverage(x as u32, y as u32).distance();
                    assert!(
                        listed == all || (all >= threshold && listed >= threshold),
                        "{path:?} {cap:?} ({x}, {y}): listed {listed}, all {all}"
                    );
                    covered += u32::from(all < 0.0);
                }
            }
            // A line three pixels wide through the frame, not the frame.
            assert!((60..600).contains(&covered), "{path:?} {cap:?}: {covered}");
        }
    }
    // The diagonal passes through the pixel corners it should.
    let diagonal = prepared(
        &polyline(
            &[(-1e20, -1e20), (1e20, 1e20)],
            2.0,
            LineCap::Butt,
            LineJoin::Miter,
        )?,
        &mapping,
    )?;
    assert_eq!(at(&diagonal, 40.0, 40.0), -1.0);
    assert!(close(at(&diagonal, 41.0, 39.0), 2f32.sqrt() - 1.0));
    Ok(())
}

#[test]
fn tiles_reach_the_last_pixel_of_the_largest_frame() -> Outcome {
    // A frame as wide and as tall as a `u32` holds.
    for (wide, tall) in [(u32::MAX, 64), (64, u32::MAX)] {
        let mapping = FrameMapping::new(wide, tall, Viewport::new(64.0, 64.0, 1.0)?)?;
        // Mapped onto the last pixels of the long axis.
        let path = if wide > tall {
            [(64.0, 20.0), (64.0, 40.0)]
        } else {
            [(20.0, 64.0), (40.0, 64.0)]
        };
        let stroke = prepared(
            &polyline(&path, 2.0, LineCap::Butt, LineJoin::Miter)?,
            &mapping,
        )?;
        let [x0, y0, x1, y1] = stroke.span();
        assert_eq!(x1.max(y1), u32::MAX);
        let mut tiles = 0;
        for [left, top, right, bottom] in stroke.tiles() {
            assert!(x0 <= left && left < right && right <= x1, "{left}..{right}");
            assert!(y0 <= top && top < bottom && bottom <= y1, "{top}..{bottom}");
            tiles += 1;
        }
        assert!(tiles > 0);
    }
    Ok(())
}

#[test]
fn unequal_axes_move_points_and_keep_the_width() -> Outcome {
    // Twice as many pixels per unit across as down, at a viewport scale of
    // one: positions follow each axis, the width follows the scale.
    let mapping = FrameMapping::new(128, 64, Viewport::new(64.0, 64.0, 1.0)?)?;
    let upright = prepared(
        &polyline(
            &[(10.0, 10.0), (10.0, 50.0)],
            2.0,
            LineCap::Butt,
            LineJoin::Miter,
        )?,
        &mapping,
    )?;
    assert_eq!(upright.half_width(), 1.0);
    let row = |x| upright.coverage(x, 30).coverage();
    assert_eq!([row(18), row(19), row(20), row(21)], [0.0, 1.0, 1.0, 0.0]);
    assert_eq!(upright.coverage(19, 9).coverage(), 0.0);
    assert_eq!(upright.coverage(19, 10).coverage(), 1.0);
    Ok(())
}

#[test]
fn curves_are_flattened_within_a_fifth_of_a_pixel() -> Outcome {
    // A circle of radius 20 around (32, 32), four pixels wide, at three
    // pixel densities and under a command scale: its distance is that of
    // the ring, within the chord tolerance.
    let ring = VectorGeometry::new(
        Path::circle(Point::new(32.0, 32.0)?, Length::new(20.0)?)?,
        VectorStyle::Stroke(style(4.0, LineCap::Butt, LineJoin::Round)?),
    );
    assert!(PreparedStroke::accepts(&ring));
    for (density, command) in [(1.0, 1.0), (2.0, 1.0), (1.0, 3.0), (0.5, 1.0)] {
        let mapping = mapping(64.0 * command as f32, 64.0 * command as f32, density)?;
        let stroke = PreparedStroke::new(&ring, Point::ZERO, command as f32, None, &mapping)?
            .ok_or("nothing prepared")?;
        let factor = (density * command) as f32;
        let mut random = Lcg(17);
        let mut farthest = 0.0f32;
        for _ in 0..4000 {
            let (x, y) = (64.0 * factor * random.next(), 64.0 * factor * random.next());
            let radius = (x - 32.0 * factor).hypot(y - 32.0 * factor);
            let expected = (radius - 20.0 * factor).abs() - 2.0 * factor;
            // Far inside the ring's hole the nearest chord is not the
            // nearest part of the stroke's region; near the stroke it is.
            if expected.abs() < 3.0 {
                farthest = farthest.max((at(&stroke, x, y) - expected).abs());
            }
        }
        assert!(
            farthest <= 0.2 + 1e-3,
            "density {density}, scale {command}: {farthest}"
        );
        // More pixels per unit take more chords.
        assert!(stroke.segments().len() >= 16, "{}", stroke.segments().len());
    }
    let coarse = PreparedStroke::new(&ring, Point::ZERO, 1.0, None, &mapping(64.0, 64.0, 1.0)?)?
        .ok_or("nothing prepared")?;
    let fine = PreparedStroke::new(&ring, Point::ZERO, 1.0, None, &mapping(64.0, 64.0, 4.0)?)?
        .ok_or("nothing prepared")?;
    assert!(fine.segments().len() > coarse.segments().len());
    // An open curve keeps its caps and joins its chords.
    for join in JOINS {
        let arch = PathBuilder::new()
            .move_to(Point::new(10.0, 50.0)?)
            .cubic_to(
                Point::new(10.0, 10.0)?,
                Point::new(54.0, 10.0)?,
                Point::new(54.0, 50.0)?,
            )
            .build()?;
        let arch =
            VectorGeometry::new(arch, VectorStyle::Stroke(style(6.0, LineCap::Round, join)?));
        let stroke = prepared(&arch, &mapping(64.0, 64.0, 1.0)?)?;
        // The top of the arch is at y = 20; the caps reach below the ends.
        assert!((at(&stroke, 32.0, 20.0) + 3.0).abs() < 0.2, "{join:?}");
        assert!((at(&stroke, 32.0, 24.0) - 1.0).abs() < 0.2, "{join:?}");
        assert_eq!(at(&stroke, 10.0, 52.0), -1.0, "{join:?}");
        assert_eq!(at(&stroke, 54.0, 54.0), 1.0, "{join:?}");
        // Along the curve the distance is continuous across its chords.
        let mut random = Lcg(23);
        for _ in 0..4000 {
            let (x, y) = (64.0 * random.next(), 64.0 * random.next());
            let step = 1.0 / 64.0;
            let (near, far) = (at(&stroke, x, y), at(&stroke, x + step, y));
            assert!((near - far).abs() <= step + 1e-3, "{join:?} ({x}, {y})");
        }
    }
    Ok(())
}

#[test]
fn a_curves_joins_are_round_and_its_ends_follow_its_tangents() -> Outcome {
    let curve = |build: PathBuilder,
                 width: f32,
                 join|
     -> Result<VectorGeometry, Box<dyn std::error::Error>> {
        Ok(VectorGeometry::new(
            build.build()?,
            VectorStyle::Stroke(style(width, LineCap::Butt, join)?),
        ))
    };
    // A quadratic that runs out to x = 24 and back along the same line:
    // its flattened reversal is inside the curve, so the stroke's end is
    // round there whatever the join, as the mask coverage's is.
    for join in JOINS {
        let back = curve(
            PathBuilder::new()
                .move_to(Point::new(20.0, 20.0)?)
                .quad_to(Point::new(28.0, 20.0)?, Point::new(20.0, 20.0)?),
            10.0,
            join,
        )?;
        let stroke = prepared(&back, &mapping(64.0, 64.0, 1.0)?)?;
        assert!(at(&stroke, 27.5, 20.5) < -1.0, "{join:?}");
        assert!(at(&stroke, 29.5, 20.5) > 0.4, "{join:?}");
    }
    // A wide stroke along a quarter turn: its butt caps are perpendicular
    // to the curve's tangents at its ends, not to its first and last chord.
    let turn = curve(
        PathBuilder::new()
            .move_to(Point::new(100.0, 100.0)?)
            .quad_to(Point::new(100.0, 110.0)?, Point::new(110.0, 110.0)?),
        100.0,
        LineJoin::Miter,
    )?;
    let stroke = prepared(&turn, &mapping(256.0, 256.0, 1.0)?)?;
    // The start's cap line is y = 100, the end's x = 110.
    assert!(at(&stroke, 60.5, 103.5) < -3.0);
    assert!((at(&stroke, 60.5, 99.0) - 1.0).abs() < 0.05);
    assert!((at(&stroke, 80.0, 99.0) - 1.0).abs() < 0.05);
    assert!((at(&stroke, 111.0, 150.0) - 1.0).abs() < 0.05);
    assert!(at(&stroke, 106.5, 150.0) < -3.0);
    // A control point almost on the start, on the far side of it: the
    // tangent points away from the curve's run. The square cap lies behind
    // the tangent, inside the stroke, not ahead of the first chord.
    for cap in [LineCap::Butt, LineCap::Square] {
        let away = VectorGeometry::new(
            PathBuilder::new()
                .move_to(Point::new(20.0, 20.0)?)
                .quad_to(Point::new(19.99, 20.0)?, Point::new(30.0, 20.0)?)
                .build()?,
            VectorStyle::Stroke(style(10.0, cap, LineJoin::Miter)?),
        );
        let stroke = prepared(&away, &mapping(64.0, 64.0, 1.0)?)?;
        assert!(at(&stroke, 17.5, 20.5) > 0.5, "{cap:?}");
        assert!(at(&stroke, 22.5, 20.5) < -1.0, "{cap:?}");
    }
    // A control point a millionth of a pixel beside the start: the tangent
    // is perpendicular to a curve that runs along y = 20, and the point
    // added on it stays within a tenth of a pixel of the curve's chord.
    let beside = curve(
        PathBuilder::new().move_to(Point::new(20.0, 20.0)?).quad_to(
            Point::new(20.0, 20.0 + 2f32.powi(-19))?,
            Point::new(120.0, 20.0)?,
        ),
        10.0,
        LineJoin::Round,
    )?;
    let stroke = prepared(&beside, &mapping(128.0, 64.0, 1.0)?)?;
    for x in [21.0, 25.0, 40.0, 90.0] {
        assert!((at(&stroke, x, 14.5) - 0.5).abs() < 0.11, "({x}, 14.5)");
        assert!((at(&stroke, x, 25.5) - 0.5).abs() < 0.11, "({x}, 25.5)");
    }
    // A curve meeting a line at an angle is joined in the stroke's style
    // with the curve's tangent: a miter's tip where the tangents say.
    let corner = curve(
        PathBuilder::new()
            .move_to(Point::new(20.0, 40.0)?)
            .quad_to(Point::new(40.0, 40.0)?, Point::new(40.0, 20.0)?)
            .line_to(Point::new(20.0, 20.0)?),
        4.0,
        LineJoin::Miter,
    )?;
    let stroke = prepared(&corner, &mapping(64.0, 64.0, 1.0)?)?;
    // The curve arrives going up, the line leaves going left: the outer
    // corner is (42, 18).
    assert!((at(&stroke, 41.5, 18.5) + 0.5).abs() < 0.05);
    assert!((at(&stroke, 42.5, 17.5) - 0.5).abs() < 0.05);
    Ok(())
}

#[test]
fn an_invisible_curve_is_not_flattened() -> Outcome {
    let mapping = mapping(64.0, 64.0, 1.0)?;
    let arc = PathBuilder::new()
        .move_to(Point::new(1.0, 1.0)?)
        .quad_to(Point::new(1.0, 2.0)?, Point::new(2.0, 2.0)?)
        .build()?;
    let stroke = VectorGeometry::new(
        arc.clone(),
        VectorStyle::Stroke(style(2.0, LineCap::Butt, LineJoin::Miter)?),
    );
    // Magnified a million million times it needs more chords than
    // flattening admits, where any of it can show.
    assert_eq!(
        PreparedStroke::new(&stroke, Point::ZERO, 1.0e12, None, &mapping),
        Err(crate::PathError::TooLarge.into())
    );
    // Clipped away, or moved off the frame, it prepares nothing instead.
    let beside = Rect::new(40.0, 40.0, 0.0, 0.0)?;
    assert_eq!(
        PreparedStroke::new(&stroke, Point::ZERO, 1.0e12, Some(beside), &mapping),
        Ok(None)
    );
    assert_eq!(
        PreparedStroke::new(&stroke, Point::new(-1.0e20, 0.0)?, 1.0e12, None, &mapping),
        Ok(None)
    );
    // So does a stroke without width.
    let hairless = VectorGeometry::new(
        arc,
        VectorStyle::Stroke(style(0.0, LineCap::Butt, LineJoin::Miter)?),
    );
    assert_eq!(
        PreparedStroke::new(&hairless, Point::ZERO, 1.0e12, None, &mapping),
        Ok(None)
    );
    Ok(())
}

#[test]
fn what_cannot_be_prepared_is_refused_and_nothing_draws_nothing() -> Outcome {
    let mapping = mapping(64.0, 64.0, 1.0)?;
    let line = [(10.0, 10.0), (30.0, 10.0)];
    let new = |vector: &VectorGeometry, scale: f32| {
        PreparedStroke::new(vector, Point::ZERO, scale, None, &mapping)
    };
    let fill = VectorGeometry::new(
        Path::polygon(points(&[(10.0, 10.0), (30.0, 10.0), (30.0, 30.0)])?)?,
        VectorStyle::Fill(FillRule::NonZero),
    );
    assert_eq!(new(&fill, 1.0), Err(StrokePrepareError::Unsupported.into()));
    let stroke = polyline(&line, 2.0, LineCap::Butt, LineJoin::Miter)?;
    assert_eq!(new(&stroke, f32::NAN), Err(GeometryError::NonFinite.into()));
    assert_eq!(
        new(&stroke, 0.0),
        Err(GeometryError::NonPositiveScale.into())
    );
    // A zero width and a stroke outside the frame draw nothing.
    let zero = polyline(&line, 0.0, LineCap::Round, LineJoin::Miter)?;
    assert_eq!(new(&zero, 1.0), Ok(None));
    let outside = polyline(
        &[(200.0, 10.0), (300.0, 10.0)],
        2.0,
        LineCap::Round,
        LineJoin::Miter,
    )?;
    assert_eq!(new(&outside, 1.0), Ok(None));
    Ok(())
}

#[test]
fn a_tile_listing_too_many_segments_is_refused() -> Outcome {
    let mapping = mapping(64.0, 64.0, 1.0)?;
    // A zigzag inside one tile.
    let zigzag = |count: usize| -> Result<VectorGeometry, Box<dyn std::error::Error>> {
        let path: Vec<(f32, f32)> = (0..count)
            .map(|i| {
                (
                    34.0 + 4.0 * (i % 2) as f32,
                    34.0 + 8.0 * i as f32 / count as f32,
                )
            })
            .collect();
        polyline(&path, 1.0, LineCap::Butt, LineJoin::Bevel)
    };
    let most = prepared(&zigzag(PreparedStroke::MAX_TILE_SEGMENTS + 1)?, &mapping)?;
    assert_eq!(most.list(36, 38).len(), PreparedStroke::MAX_TILE_SEGMENTS);
    assert_eq!(
        PreparedStroke::new(
            &zigzag(PreparedStroke::MAX_TILE_SEGMENTS + 2)?,
            Point::ZERO,
            1.0,
            None,
            &mapping
        ),
        Err(DrawError::TooComplex)
    );
    Ok(())
}

#[test]
fn too_many_tile_references_are_refused() -> Outcome {
    // Long segments across a large frame: each crosses many tiles.
    let mapping = FrameMapping::new(8192, 8192, Viewport::new(8192.0, 8192.0, 1.0)?)?;
    let sweep = |count: usize| -> Result<VectorGeometry, Box<dyn std::error::Error>> {
        let path: Vec<(f32, f32)> = (0..count)
            .map(|i| (8192.0 * (i % 2) as f32, 8192.0 * i as f32 / count as f32))
            .collect();
        polyline(&path, 1.0, LineCap::Butt, LineJoin::Bevel)
    };
    assert!(PreparedStroke::new(&sweep(400)?, Point::ZERO, 1.0, None, &mapping)?.is_some());
    // One point can exceed the budget alone: a disc over a frame of more
    // tiles than a stroke may reference.
    let mapping = FrameMapping::new(40_000, 40_000, Viewport::new(40_000.0, 40_000.0, 1.0)?)?;
    let disc = PathBuilder::new()
        .move_to(Point::new(20_000.0, 20_000.0)?)
        .close()
        .build()?;
    let disc = VectorGeometry::new(
        disc,
        VectorStyle::Stroke(style(40_000.0, LineCap::Round, LineJoin::Miter)?),
    );
    assert_eq!(
        PreparedStroke::new(&disc, Point::ZERO, 1.0, None, &mapping),
        Err(DrawError::TooComplex)
    );
    let mapping = FrameMapping::new(8192, 8192, Viewport::new(8192.0, 8192.0, 1.0)?)?;
    assert_eq!(
        PreparedStroke::new(&sweep(4000)?, Point::ZERO, 1.0, None, &mapping),
        Err(DrawError::TooComplex)
    );
    Ok(())
}
