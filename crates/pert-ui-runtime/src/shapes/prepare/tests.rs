use super::*;

type Outcome = Result<(), Box<dyn std::error::Error>>;

fn mapping(width: f32, height: f32, scale: f64) -> Result<FrameMapping, GeometryError> {
    let viewport = Viewport::new(width, height, scale)?;
    let extent = viewport.frame_extent();
    FrameMapping::new(extent.width(), extent.height(), viewport)
}

fn length(value: f32) -> Length {
    Length::new(value).expect("a valid length")
}

fn border_of(width: f32) -> Border {
    Border::new(length(width), Paint::Solid(Color::WHITE))
}

fn rect(x: f32, y: f32, w: f32, h: f32, corners: Shape) -> ShapeGeometry {
    ShapeGeometry::Rect {
        rect: Rect::new(x, y, w, h).expect("a valid rectangle"),
        shape: corners,
    }
}

fn prepared(
    geometry: ShapeGeometry,
    border: Option<f32>,
    mapping: &FrameMapping,
) -> Result<PreparedShape, Box<dyn std::error::Error>> {
    PreparedShape::new(&geometry, border.map(border_of).as_ref(), None, mapping)?
        .ok_or_else(|| "the shape prepared to nothing".into())
}

/// The outer boundary's edges in frame pixels.
fn frame_edges(shape: &PreparedShape) -> [f32; 4] {
    let ShapeParameters::Rect { outer, .. } = shape.parameters() else {
        panic!("not a rectangle");
    };
    let [x0, y0, ..] = shape.span();
    [
        outer.edges[0] + x0 as f32,
        outer.edges[1] + y0 as f32,
        outer.edges[2] + x0 as f32,
        outer.edges[3] + y0 as f32,
    ]
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

/// A small deterministic generator for the invariant loops.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 40) as f32) / (1u64 << 24) as f32
    }

    fn range(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.next()
    }
}

#[test]
fn rectangle_edges_snap_like_a_solid_rectangle() -> Outcome {
    // The span rule solid rectangles use in the CPU renderer.
    let solid = |start: f64, end: f64, scale: f64| {
        (
            (start * scale - 0.5).ceil() as f32,
            (end * scale - 0.5).ceil() as f32,
        )
    };
    // A viewport width of 3.4 at scale 1 gives a frame of 3 pixels, so the
    // horizontal factor is not the scale.
    for (width, scale) in [
        (40.0f32, 1.0f64),
        (40.0, 1.25),
        (40.0, 2.0),
        (3.4, 1.0),
        (33.3, 1.5),
    ] {
        let mapping = mapping(width, 30.0, scale)?;
        let viewport = Viewport::new(width, 30.0, scale)?;
        let extent = viewport.frame_extent();
        let (sx, sy) = (
            f64::from(extent.width()) / f64::from(width),
            f64::from(extent.height()) / 30.0,
        );
        for (x, y, w, h) in [
            (1.0f32, 2.0f32, 1.6f32, 9.0f32),
            (0.3, 0.7, 2.2, 3.1),
            (0.49, 0.51, 1.02, 20.0),
            (1.5, 2.5, 1.0, 1.0),
        ] {
            let Some(shape) =
                PreparedShape::new(&rect(x, y, w, h, Shape::RECTANGLE), None, None, &mapping)?
            else {
                let (x0, x1) = solid(f64::from(x), f64::from(x) + f64::from(w), sx);
                let (y0, y1) = solid(f64::from(y), f64::from(y) + f64::from(h), sy);
                assert!(
                    x0 >= x1 || y0 >= y1 || x0 >= extent.width() as f32,
                    "{x} {w}"
                );
                continue;
            };
            let (x0, x1) = solid(f64::from(x), f64::from(x) + f64::from(w), sx);
            let (y0, y1) = solid(f64::from(y), f64::from(y) + f64::from(h), sy);
            assert_eq!(
                frame_edges(&shape),
                [x0, y0, x1, y1],
                "{width} {scale} {x} {w}"
            );
            // Coverage is exactly the solid rectangle's pixels.
            let [sx0, sy0, sx1, sy1] = shape.span();
            for py in sy0..sy1 {
                for px in sx0..sx1 {
                    let inside = (px as f32) >= x0
                        && (px as f32) < x1
                        && (py as f32) >= y0
                        && (py as f32) < y1;
                    let coverage = shape.coverage(px, py);
                    assert_eq!(coverage.outer(), if inside { 1.0 } else { 0.0 });
                    assert!(coverage.exact_arithmetic());
                }
            }
        }
    }
    Ok(())
}

#[test]
fn a_rectangle_between_pixel_centers_collapses() -> Outcome {
    let mapping = mapping(20.0, 20.0, 1.0)?;
    for geometry in [
        rect(0.6, 2.0, 0.3, 5.0, Shape::RECTANGLE),
        rect(2.0, 0.6, 5.0, 0.3, Shape::rounded(length(3.0))),
        rect(2.0, 2.0, 0.0, 5.0, Shape::RECTANGLE),
        // Outside the frame.
        rect(30.0, 2.0, 5.0, 5.0, Shape::RECTANGLE),
    ] {
        assert_eq!(PreparedShape::new(&geometry, None, None, &mapping)?, None);
    }
    Ok(())
}

fn corner_sizes(shape: &PreparedShape) -> [f32; 4] {
    let ShapeParameters::Rect { outer, .. } = shape.parameters() else {
        panic!("not a rectangle");
    };
    outer.corners.map(|corner| corner.size)
}

#[test]
fn the_overlap_rule_scales_corners_that_do_not_fit() -> Outcome {
    let mapping = mapping(200.0, 100.0, 1.0)?;
    // Two round corners share each short side: 30 > 20, so all scale by 2/3.
    let shape = prepared(
        rect(10.0, 10.0, 100.0, 20.0, Shape::rounded(length(15.0))),
        None,
        &mapping,
    )?;
    assert!(corner_sizes(&shape).iter().all(|size| close(*size, 10.0)));
    // Top corners only: each short side holds one corner of 15 ≤ 20, and the
    // top holds 30 ≤ 100, so one corner may exceed half the height.
    let shape = prepared(
        rect(10.0, 10.0, 100.0, 20.0, Shape::rounded_top(length(15.0))),
        None,
        &mapping,
    )?;
    assert_eq!(corner_sizes(&shape), [15.0, 15.0, 0.0, 0.0]);
    // A single corner may take the whole side, round or cut.
    for corner in [Corner::round(length(50.0)), Corner::cut(length(50.0))] {
        let corners = Shape::RECTANGLE.with_corner(CornerPosition::BottomRight, corner);
        let shape = prepared(rect(10.0, 10.0, 100.0, 20.0, corners), None, &mapping)?;
        assert_eq!(corner_sizes(&shape), [0.0, 0.0, 20.0, 0.0]);
    }
    // Sides whose corners are both square impose no limit.
    let shape = prepared(
        rect(10.0, 10.0, 100.0, 20.0, Shape::RECTANGLE),
        None,
        &mapping,
    )?;
    assert_eq!(corner_sizes(&shape), [0.0; 4]);
    Ok(())
}

#[test]
fn a_cut_corner_is_continuous_while_its_size_passes_a_pixel_center() -> Outcome {
    let mapping = mapping(40.0, 40.0, 1.0)?;
    // A cut of 4 at the top right of a rectangle ending at x = 16, from
    // y = 2, runs along x − y = 10, through the center of pixel (13, 3).
    let coverage_at = |size: f32, x: u32, y: u32| -> Result<f32, Box<dyn std::error::Error>> {
        let corners = Shape::cut_top_right(length(size));
        Ok(
            prepared(rect(2.0, 2.0, 14.0, 20.0, corners), None, &mapping)?
                .coverage(x, y)
                .outer(),
        )
    };
    for (x, y) in [(13, 3), (15, 2), (13, 2), (15, 5), (14, 3)] {
        let mut previous = coverage_at(0.0, x, y)?;
        let mut size = 0.0f32;
        while size < 14.0 {
            size += 1.0 / 64.0;
            let coverage = coverage_at(size, x, y)?;
            // A step of 1/64 moves the chamfer by 1/(64·√2) pixel.
            assert!(
                (coverage - previous).abs() <= 0.0125,
                "{x} {y} {size}: {previous} {coverage}"
            );
            assert!(
                coverage <= previous + 1e-6,
                "a larger cut never adds coverage"
            );
            previous = coverage;
        }
    }
    assert!(close(coverage_at(4.0, 13, 3)?, 0.5));
    assert_eq!(coverage_at(4.0, 12, 3)?, 1.0);
    assert_eq!(coverage_at(4.0, 14, 3)?, 0.0);
    Ok(())
}

#[test]
fn a_zero_width_border_is_absent() -> Outcome {
    let mapping = mapping(40.0, 40.0, 1.0)?;
    let geometry = rect(2.0, 2.0, 10.0, 10.0, Shape::rounded(length(3.0)));
    let plain = prepared(geometry, None, &mapping)?;
    let zero = prepared(geometry, Some(0.0), &mapping)?;
    assert_eq!(plain, zero);
    assert!(matches!(
        zero.parameters(),
        ShapeParameters::Rect {
            inner: Inner::NoBorder,
            ..
        }
    ));
    assert_eq!(zero.coverage(6, 6).border(), 0.0);
    Ok(())
}

#[test]
fn border_widths_are_whole_physical_pixels() -> Outcome {
    let inner_inset = |width: f32, scale: f64| -> Result<f32, Box<dyn std::error::Error>> {
        let mapping = mapping(60.0, 60.0, scale)?;
        let shape = prepared(
            rect(2.0, 2.0, 40.0, 40.0, Shape::RECTANGLE),
            Some(width),
            &mapping,
        )?;
        let ShapeParameters::Rect {
            outer,
            inner: Inner::Area(inner),
        } = shape.parameters()
        else {
            return Err("no inner area".into());
        };
        Ok(inner.edges[0] - outer.edges[0])
    };
    assert_eq!(inner_inset(1.0, 1.0)?, 1.0);
    assert_eq!(inner_inset(1.49, 1.0)?, 1.0);
    assert_eq!(inner_inset(1.5, 1.0)?, 2.0);
    assert_eq!(inner_inset(0.2, 1.0)?, 1.0, "at least one pixel");
    assert_eq!(inner_inset(1.0, 1.25)?, 1.0);
    assert_eq!(inner_inset(1.0, 2.0)?, 2.0);
    assert_eq!(inner_inset(1.2, 1.25)?, 2.0);
    Ok(())
}

#[test]
fn inner_corners_are_the_exact_inward_offsets() -> Outcome {
    let mapping = mapping(60.0, 60.0, 1.0)?;
    let corners = Shape::RECTANGLE
        .with_corner(CornerPosition::TopLeft, Corner::round(length(5.0)))
        .with_corner(CornerPosition::TopRight, Corner::cut(length(5.0)))
        .with_corner(CornerPosition::BottomRight, Corner::round(length(1.5)))
        .with_corner(CornerPosition::BottomLeft, Corner::cut(length(1.0)));
    let shape = prepared(rect(2.0, 2.0, 40.0, 40.0, corners), Some(2.0), &mapping)?;
    let ShapeParameters::Rect {
        inner: Inner::Area(inner),
        ..
    } = shape.parameters()
    else {
        return Err("no inner area".into());
    };
    let sizes = inner.corners.map(|corner| corner.size);
    assert!(close(sizes[0], 3.0));
    assert!(close(sizes[1], 5.0 - (2.0 - SQRT_2) * 2.0));
    assert_eq!(sizes[2], 0.0);
    assert_eq!(sizes[3], 0.0);
    assert_eq!(inner.corners[1].kind, CornerKind::Cut);
    Ok(())
}

#[test]
fn a_wide_border_takes_the_whole_silhouette() -> Outcome {
    let mapping = mapping(40.0, 40.0, 1.0)?;
    for (width, filled) in [(1.0, false), (1.4, false), (2.0, true), (5.0, true)] {
        let shape = prepared(
            rect(2.0, 2.0, 10.0, 4.0, Shape::rounded(length(1.0))),
            Some(width),
            &mapping,
        )?;
        let ShapeParameters::Rect { inner, .. } = shape.parameters() else {
            return Err("not a rectangle".into());
        };
        assert_eq!(matches!(inner, Inner::Filled), filled, "{width}");
        if filled {
            let coverage = shape.coverage(6, 4);
            assert_eq!((coverage.outer(), coverage.border()), (1.0, 1.0));
            assert_eq!(coverage.inner_distance(), None);
        }
    }
    Ok(())
}

#[test]
fn far_edges_are_relocated_without_changing_coverage() -> Outcome {
    let mapping = mapping(100.0, 100.0, 1.0)?;
    // A rectangle far larger than the frame, with small corners: every
    // frame pixel is inside, and the parameters stay near the span.
    let corners =
        Shape::rounded(length(6.0)).with_corner(CornerPosition::TopRight, Corner::cut(length(4.0)));
    let shape = prepared(
        rect(-50_000.0, -50_000.0, 100_000.0, 100_000.0, corners),
        Some(3.0),
        &mapping,
    )?;
    assert_eq!(shape.span(), [0, 0, 100, 100]);
    let ShapeParameters::Rect {
        outer,
        inner: Inner::Area(inner),
    } = shape.parameters()
    else {
        return Err("no inner area".into());
    };
    assert_eq!(outer.edges, [-8.0, -8.0, 108.0, 108.0]);
    assert!(inner.edges.iter().all(|edge| edge.abs() <= 108.0));
    for (x, y) in [(0, 0), (99, 0), (0, 99), (99, 99), (50, 50)] {
        let coverage = shape.coverage(x, y);
        assert_eq!((coverage.outer(), coverage.border()), (1.0, 0.0), "{x} {y}");
    }
    // A near edge keeps its place and its corner.
    let shape = prepared(
        rect(10.0, -50_000.0, 100_000.0, 100_000.0, Shape::RECTANGLE),
        None,
        &mapping,
    )?;
    assert_eq!(frame_edges(&shape)[0], 10.0);
    assert_eq!(shape.coverage(9, 50).outer(), 0.0);
    assert_eq!(shape.coverage(10, 50).outer(), 1.0);
    Ok(())
}

#[test]
fn geometry_beyond_the_domain_is_refused() -> Outcome {
    let mapping = mapping(100.0, 100.0, 1.0)?;
    let out_of_range = |geometry: ShapeGeometry| {
        PreparedShape::new(&geometry, None, None, &mapping) == Err(ShapePrepareError::OutOfRange)
    };
    // A corner far larger than the domain.
    assert!(out_of_range(rect(
        -1.0e6,
        -1.0e6,
        2.0e6,
        2.0e6,
        Shape::rounded(length(5.0e5)),
    )));
    // An arc of radius 10^6 pixels through the frame.
    assert!(out_of_range(ShapeGeometry::Arc(ArcGeometry::new(
        Point::new(1.0e6, 50.0)?,
        length(1.0e6),
        0.0,
        360.0,
        length(2.0),
    )?)));
    // A triangle vertex far outside the frame.
    assert!(out_of_range(ShapeGeometry::Triangle(
        TriangleGeometry::new([
            Point::new(10.0, 10.0)?,
            Point::new(1.0e6, 20.0)?,
            Point::new(10.0, 50.0)?,
        ])
        .ok_or("degenerate")?
    )));
    // A span wider than the domain.
    let wide = FrameMapping::new(70_000, 10, Viewport::new(70_000.0, 10.0, 1.0)?)?;
    assert_eq!(
        PreparedShape::new(
            &rect(0.0, 0.0, 70_000.0, 10.0, Shape::RECTANGLE),
            None,
            None,
            &wide
        ),
        Err(ShapePrepareError::OutOfRange)
    );
    // The same rectangle clipped to a narrow span is fine.
    let clip = Rect::new(100.0, 0.0, 50.0, 10.0)?;
    assert!(
        PreparedShape::new(
            &rect(0.0, 0.0, 70_000.0, 10.0, Shape::RECTANGLE),
            None,
            Some(clip),
            &wide
        )?
        .is_some()
    );
    Ok(())
}

#[test]
fn a_triangle_that_collapses_on_narrowing_draws_nothing() -> Outcome {
    let mapping = mapping(100.0, 100.0, 1.0)?;
    let triangle =
        |points: [(f32, f32); 3]| -> Result<Option<PreparedShape>, Box<dyn std::error::Error>> {
            let [a, b, c] = points;
            let geometry = TriangleGeometry::new([
                Point::new(a.0, a.1)?,
                Point::new(b.0, b.1)?,
                Point::new(c.0, c.1)?,
            ])
            .ok_or("the logical test refused it")?;
            Ok(PreparedShape::new(
                &ShapeGeometry::Triangle(geometry),
                None,
                None,
                &mapping,
            )?)
        };
    // Not collinear logically, but one edge is shorter than 1/256 pixel.
    assert_eq!(
        triangle([(10.0, 10.0), (10.002, 10.0), (10.0, 40.0)])?,
        None
    );
    assert!(triangle([(10.0, 10.0), (10.01, 10.0), (10.0, 40.0)])?.is_some());
    // The logical test itself refuses collinear and coincident points.
    assert!(
        TriangleGeometry::new([
            Point::new(0.0, 0.0)?,
            Point::new(1.0, 1.0)?,
            Point::new(2.0, 2.0)?
        ])
        .is_none()
    );
    assert!(
        TriangleGeometry::new([
            Point::new(3.0, 3.0)?,
            Point::new(3.0, 3.0)?,
            Point::new(2.0, 9.0)?
        ])
        .is_none()
    );
    Ok(())
}

#[test]
fn clip_edges_snap_like_shape_edges() -> Outcome {
    let mapping = mapping(40.0, 40.0, 1.0)?;
    let geometry = rect(0.0, 0.0, 30.0, 30.0, Shape::RECTANGLE);
    let span = |x: f32, w: f32| -> Result<[u32; 4], Box<dyn std::error::Error>> {
        let clip = Rect::new(x, 5.0, w, 10.0)?;
        Ok(PreparedShape::new(&geometry, None, Some(clip), &mapping)?
            .ok_or("clipped away")?
            .span())
    };
    // An edge just before a pixel center keeps the pixel; just after, not.
    assert_eq!(span(1.49, 10.0)?, [1, 5, 11, 15]);
    assert_eq!(span(1.5, 10.0)?, [1, 5, 11, 15]);
    assert_eq!(span(1.51, 10.0)?, [2, 5, 12, 15]);
    // Coverage outside the span is untouched.
    let clip = Rect::new(4.0, 5.0, 3.0, 10.0)?;
    let shape = PreparedShape::new(&geometry, None, Some(clip), &mapping)?.ok_or("clipped away")?;
    let outside = shape.coverage(3, 6);
    assert_eq!(outside.outer(), 0.0);
    assert_eq!(outside.outer_distance(), f32::INFINITY);
    assert_eq!(outside.inner_distance(), None);
    assert!(outside.exact_arithmetic());
    assert_eq!(shape.coverage(4, 6).outer(), 1.0);
    // A clip that misses the shape leaves nothing.
    let clip = Rect::new(35.0, 5.0, 3.0, 10.0)?;
    assert_eq!(
        PreparedShape::new(&geometry, None, Some(clip), &mapping)?,
        None
    );
    Ok(())
}

#[test]
fn arc_angles_are_validated_and_reduced() -> Outcome {
    let arc = |start: f32, sweep: f32| {
        ArcGeometry::new(Point::ZERO, length(5.0), start, sweep, length(1.0))
    };
    assert_eq!(arc(-90.0, 10.0)?.start_degrees(), 270.0);
    assert_eq!(arc(720.0, 10.0)?.start_degrees(), 0.0);
    assert_eq!(arc(450.0, 10.0)?.start_degrees(), 90.0);
    assert_eq!(arc(-1.0e-10, 10.0)?.start_degrees(), 0.0);
    assert_eq!(arc(10.0, 360.0)?.sweep_degrees(), 360.0);
    for (start, sweep) in [
        (f32::NAN, 10.0),
        (0.0, f32::INFINITY),
        (0.0, -1.0),
        (0.0, 360.5),
    ] {
        assert_eq!(arc(start, sweep), Err(PathError::InvalidParameter));
    }
    Ok(())
}

#[test]
fn arcs_and_triangles_take_no_border_and_empty_ones_draw_nothing() -> Outcome {
    let mapping = mapping(40.0, 40.0, 1.0)?;
    let arc = |sweep: f32, width: f32| -> Result<ShapeGeometry, Box<dyn std::error::Error>> {
        Ok(ShapeGeometry::Arc(ArcGeometry::new(
            Point::new(20.0, 20.0)?,
            length(8.0),
            0.0,
            sweep,
            length(width),
        )?))
    };
    let triangle = ShapeGeometry::Triangle(
        TriangleGeometry::new([
            Point::new(1.0, 1.0)?,
            Point::new(9.0, 1.0)?,
            Point::new(1.0, 9.0)?,
        ])
        .ok_or("degenerate")?,
    );
    for geometry in [arc(90.0, 2.0)?, triangle] {
        assert_eq!(
            PreparedShape::new(&geometry, Some(&border_of(1.0)), None, &mapping),
            Err(ShapePrepareError::BorderNotAllowed)
        );
    }
    assert_eq!(
        PreparedShape::new(&arc(0.0, 2.0)?, None, None, &mapping)?,
        None
    );
    assert_eq!(
        PreparedShape::new(&arc(90.0, 0.0)?, None, None, &mapping)?,
        None
    );
    Ok(())
}

#[test]
fn a_frame_mapping_needs_an_extent() -> Outcome {
    let viewport = Viewport::new(10.0, 10.0, 1.0)?;
    assert_eq!(
        FrameMapping::new(0, 10, viewport),
        Err(GeometryError::Empty)
    );
    assert_eq!(
        FrameMapping::new(10, 0, viewport),
        Err(GeometryError::Empty)
    );
    assert!(FrameMapping::new(10, 10, viewport).is_ok());
    Ok(())
}

#[test]
fn round_corner_pixels_match_hand_computation() -> Outcome {
    // A 20×20 rectangle at the origin with radius 4: the corner's center is
    // (4, 4), and a pixel center p has distance |p − c| − 4.
    let hand = |px: f32, py: f32| {
        (0.5 - (((px - 4.0).powi(2) + (py - 4.0).powi(2)).sqrt() - 4.0)).clamp(0.0, 1.0)
    };
    let mapping = mapping(40.0, 40.0, 1.0)?;
    let shape = prepared(
        rect(0.0, 0.0, 20.0, 20.0, Shape::rounded(length(4.0))),
        None,
        &mapping,
    )?;
    for (x, y) in [(0u32, 0u32), (1, 1), (0, 1), (1, 0), (2, 0), (0, 3), (3, 3)] {
        let coverage = shape.coverage(x, y);
        assert!(
            close(coverage.outer(), hand(x as f32 + 0.5, y as f32 + 0.5)),
            "{x} {y}"
        );
        assert!(!coverage.exact_arithmetic(), "inside the corner's region");
    }
    assert!(close(shape.coverage(1, 1).outer(), 0.964_466));
    assert_eq!(shape.coverage(0, 0).outer(), 0.0);
    // Beyond the region the straight edge decides, exactly.
    let edge = shape.coverage(0, 10);
    assert_eq!((edge.outer(), edge.exact_arithmetic()), (1.0, true));
    // The mirrored corners agree.
    for (x, y) in [(0u32, 1u32), (1, 1), (2, 0)] {
        let expected = shape.coverage(x, y).outer();
        assert_eq!(shape.coverage(19 - x, y).outer(), expected);
        assert_eq!(shape.coverage(19 - x, 19 - y).outer(), expected);
        assert_eq!(shape.coverage(x, 19 - y).outer(), expected);
    }
    // At scale 1.25 the radius is 5 physical pixels and at 2 it is 8.
    for (scale, radius) in [(1.25f64, 5.0f32), (2.0, 8.0)] {
        let mapping = self::mapping(40.0, 40.0, scale)?;
        let shape = prepared(
            rect(0.0, 0.0, 20.0, 20.0, Shape::rounded(length(4.0))),
            None,
            &mapping,
        )?;
        let hand = |px: f32, py: f32| {
            (0.5 - (((px - radius).powi(2) + (py - radius).powi(2)).sqrt() - radius))
                .clamp(0.0, 1.0)
        };
        for (x, y) in [(0u32, 0u32), (1, 1), (0, 2), (2, 1)] {
            assert!(
                close(
                    shape.coverage(x, y).outer(),
                    hand(x as f32 + 0.5, y as f32 + 0.5)
                ),
                "{scale} {x} {y}"
            );
        }
    }
    Ok(())
}

#[test]
fn small_and_oversized_corners_match_hand_computation() -> Outcome {
    let mapping = mapping(40.0, 40.0, 1.0)?;
    // A radius below one pixel: only the corner pixel loses coverage.
    let shape = prepared(
        rect(0.0, 0.0, 20.0, 20.0, Shape::rounded(length(0.75))),
        None,
        &mapping,
    )?;
    let expected = 0.5 - ((2.0f32 * 0.25 * 0.25).sqrt() - 0.75);
    assert!(close(
        shape.coverage(0, 0).outer(),
        expected.clamp(0.0, 1.0)
    ));
    assert_eq!(shape.coverage(1, 0).outer(), 1.0);
    // A zero-size corner is square.
    let shape = prepared(
        rect(0.0, 0.0, 20.0, 20.0, Shape::rounded(length(0.0))),
        None,
        &mapping,
    )?;
    assert_eq!(shape.coverage(0, 0).outer(), 1.0);
    // One round corner of the full height on a 30×10 rectangle: center
    // (10, 10), radius 10.
    let corners =
        Shape::RECTANGLE.with_corner(CornerPosition::TopLeft, Corner::round(length(10.0)));
    let shape = prepared(rect(0.0, 0.0, 30.0, 10.0, corners), None, &mapping)?;
    let hand = |px: f32, py: f32| {
        (0.5 - (((px - 10.0).powi(2) + (py - 10.0).powi(2)).sqrt() - 10.0)).clamp(0.0, 1.0)
    };
    for (x, y) in [(0u32, 5u32), (2, 3), (4, 1), (0, 9), (6, 0)] {
        assert!(
            close(
                shape.coverage(x, y).outer(),
                hand(x as f32 + 0.5, y as f32 + 0.5)
            ),
            "{x} {y}"
        );
    }
    // One cut corner of the full height: the chamfer runs from (20, 0) to
    // (30, 10), and a pixel center's distance is (x − y − 20) / √2.
    let corners = Shape::RECTANGLE.with_corner(CornerPosition::TopRight, Corner::cut(length(10.0)));
    let shape = prepared(rect(0.0, 0.0, 30.0, 10.0, corners), None, &mapping)?;
    for (x, y) in [(20u32, 0u32), (21, 0), (25, 5), (29, 9), (28, 2), (19, 0)] {
        let d = (x as f32 - y as f32 - 20.0) / SQRT_2;
        assert!(
            close(shape.coverage(x, y).outer(), (0.5 - d).clamp(0.0, 1.0)),
            "{x} {y}"
        );
    }
    Ok(())
}

#[test]
fn border_pixels_match_hand_computation() -> Outcome {
    let mapping = mapping(40.0, 40.0, 1.0)?;
    let shape = prepared(
        rect(0.0, 0.0, 20.0, 20.0, Shape::RECTANGLE),
        Some(1.0),
        &mapping,
    )?;
    // The outermost column is border, the next is fill.
    let band = shape.coverage(0, 5);
    assert_eq!((band.outer(), band.border()), (1.0, 1.0));
    assert_eq!(band.inner_distance(), Some(0.5));
    assert!(band.exact_arithmetic());
    let fill = shape.coverage(1, 5);
    assert_eq!((fill.outer(), fill.border()), (1.0, 0.0));
    // A round corner of 4 with a border of 2: the inner radius is 2 around
    // the same center, so the band is where 2 ≤ |p − c| ≤ 4, ramped.
    let shape = prepared(
        rect(0.0, 0.0, 20.0, 20.0, Shape::rounded(length(4.0))),
        Some(2.0),
        &mapping,
    )?;
    for (x, y) in [(1u32, 1u32), (2, 2), (3, 3), (1, 2), (0, 2), (2, 1)] {
        let r = ((x as f32 - 3.5).powi(2) + (y as f32 - 3.5).powi(2)).sqrt();
        let outer = (0.5 - (r - 4.0)).clamp(0.0, 1.0);
        let inner = (0.5 - (r - 2.0)).clamp(0.0, 1.0).min(outer);
        let coverage = shape.coverage(x, y);
        assert!(close(coverage.outer(), outer), "{x} {y}");
        assert!(close(coverage.border(), outer - inner), "{x} {y}");
    }
    Ok(())
}

fn arc_shape(
    start: f32,
    sweep: f32,
    width: f32,
    mapping: &FrameMapping,
) -> Result<PreparedShape, Box<dyn std::error::Error>> {
    prepared(
        ShapeGeometry::Arc(ArcGeometry::new(
            Point::new(10.5, 10.5)?,
            length(5.0),
            start,
            sweep,
            length(width),
        )?),
        None,
        mapping,
    )
}

#[test]
fn arc_pixels_match_hand_computation() -> Outcome {
    let mapping = mapping(24.0, 24.0, 1.0)?;
    // A full ring of radius 5 and width 2 around the center of pixel (10, 10).
    let ring = arc_shape(0.0, 360.0, 2.0, &mapping)?;
    let hand = |x: u32, y: u32| {
        let r = ((x as f32 - 10.0).powi(2) + (y as f32 - 10.0).powi(2)).sqrt();
        (0.5 - ((r - 5.0).abs() - 1.0)).clamp(0.0, 1.0)
    };
    for (x, y) in [
        (15u32, 10u32),
        (16, 10),
        (17, 10),
        (13, 14),
        (10, 10),
        (14, 14),
        (11, 4),
    ] {
        assert!(close(ring.coverage(x, y).outer(), hand(x, y)), "{x} {y}");
        assert_eq!(ring.coverage(x, y).border(), 0.0);
    }
    assert_eq!(ring.coverage(15, 10).outer(), 1.0);
    assert_eq!(ring.coverage(10, 10).outer(), 0.0);
    // A quarter from 0° (toward +x) clockwise to 90° (toward +y, down).
    let quarter = arc_shape(0.0, 90.0, 2.0, &mapping)?;
    assert_eq!(
        quarter.coverage(15, 10).outer(),
        1.0,
        "the start, on the stroke"
    );
    assert_eq!(quarter.coverage(10, 15).outer(), 1.0, "the end");
    assert_eq!(quarter.coverage(14, 14).outer(), hand(14, 14));
    assert_eq!(quarter.coverage(5, 10).outer(), 0.0, "the opposite side");
    assert_eq!(quarter.coverage(10, 5).outer(), 0.0);
    // Round caps: one pixel before the start, the cap's edge passes through
    // the pixel center, one pixel beyond it nothing is left.
    assert!(close(quarter.coverage(15, 9).outer(), 0.5));
    assert_eq!(quarter.coverage(15, 8).outer(), 0.0);
    assert!(close(quarter.coverage(9, 15).outer(), 0.5));
    // The start angle rotates the same arc.
    let turned = arc_shape(90.0, 90.0, 2.0, &mapping)?;
    assert_eq!(turned.coverage(10, 15).outer(), 1.0);
    assert_eq!(turned.coverage(5, 10).outer(), 1.0);
    assert_eq!(turned.coverage(15, 10).outer(), 0.0, "before the start");
    assert!(close(turned.coverage(11, 15).outer(), 0.5));
    Ok(())
}

#[test]
fn a_thin_arc_fades_with_its_width() -> Outcome {
    let mapping = mapping(24.0, 24.0, 1.0)?;
    // On the center line a stroke of one pixel or more covers the pixel.
    assert_eq!(
        arc_shape(0.0, 360.0, 1.0, &mapping)?
            .coverage(15, 10)
            .outer(),
        1.0
    );
    for width in [0.75f32, 0.5, 0.25, 0.01] {
        let coverage = arc_shape(0.0, 360.0, width, &mapping)?.coverage(15, 10);
        assert!(close(coverage.outer(), width), "{width}");
        assert_eq!(
            coverage.outer_distance(),
            -0.5,
            "the distance precedes the factor"
        );
        // The caps fade the same way. With the center a quarter pixel
        // lower, the center of pixel (15, 10) lies before the start, a
        // quarter pixel from the cap's center: inside the half-pixel cap
        // by a quarter, so three quarters covered before the factor.
        let cap = prepared(
            ShapeGeometry::Arc(ArcGeometry::new(
                Point::new(10.5, 10.75)?,
                length(5.0),
                0.0,
                90.0,
                length(width),
            )?),
            None,
            &mapping,
        )?
        .coverage(15, 10);
        assert!(close(cap.outer_distance(), -0.25));
        assert!(close(cap.outer(), 0.75 * width), "{width}");
    }
    // A sweep approaching zero leaves the two caps, which coincide.
    let sliver = arc_shape(0.0, 0.001, 2.0, &mapping)?;
    assert_eq!(sliver.coverage(15, 10).outer(), 1.0);
    assert_eq!(sliver.coverage(13, 10).outer(), 0.0);
    Ok(())
}

#[test]
fn triangle_pixels_match_hand_computation() -> Outcome {
    let mapping = mapping(24.0, 24.0, 1.0)?;
    let triangle = |points: [(f32, f32); 3]| -> Result<PreparedShape, Box<dyn std::error::Error>> {
        let [a, b, c] = points;
        prepared(
            ShapeGeometry::Triangle(
                TriangleGeometry::new([
                    Point::new(a.0, a.1)?,
                    Point::new(b.0, b.1)?,
                    Point::new(c.0, c.1)?,
                ])
                .ok_or("degenerate")?,
            ),
            None,
            &mapping,
        )
    };
    // A right triangle with legs of 10 on the axes; both windings agree.
    let one = triangle([(2.0, 2.0), (12.0, 2.0), (2.0, 12.0)])?;
    let other = triangle([(2.0, 2.0), (2.0, 12.0), (12.0, 2.0)])?;
    for shape in [one, other] {
        assert_eq!(shape.coverage(4, 4).outer(), 1.0);
        assert_eq!(shape.coverage(4, 4).outer_distance(), -2.5);
        // The center of pixel (7, 6) lies on the hypotenuse x + y = 14.
        assert!(close(shape.coverage(7, 6).outer(), 0.5));
        // One pixel along the normal: (8, 7) is √2/2·2 = 1.414 outside.
        assert_eq!(shape.coverage(8, 7).outer(), 0.0);
        // Left of the vertical leg by half a pixel.
        assert_eq!(shape.coverage(1, 6).outer(), 0.0);
        assert_eq!(shape.coverage(2, 6).outer(), 1.0);
        // Beyond the right-angle vertex the nearest point is the vertex.
        let corner = shape.coverage(1, 1);
        assert!(close(corner.outer_distance(), 0.5 * SQRT_2));
        assert_eq!(corner.border(), 0.0);
    }
    Ok(())
}

/// Every pixel of `shape`'s span, checked against the coverage invariants.
fn check_invariants(shape: &PreparedShape, context: &str) {
    let [x0, y0, x1, y1] = shape.span();
    let inner_area = matches!(
        shape.parameters(),
        ShapeParameters::Rect {
            inner: Inner::Area(_),
            ..
        }
    );
    // A solid box is inside the span, and every row's run covers it.
    if let Some([left, top, right, bottom]) = shape.solid_box() {
        assert!(
            x0 <= left && left < right && right <= x1 && y0 <= top && top < bottom && bottom <= y1,
            "{context}: box {left} {top} {right} {bottom}"
        );
        for y in top..bottom {
            let run = shape.solid_run(y);
            assert!(
                run.is_some_and(|[first, past]| first <= left && right <= past),
                "{context}: row {y} of the box has the run {run:?}"
            );
        }
    }
    for y in y0..y1 {
        // A solid run is inside the span, saturated and uniform.
        if let Some([first, past]) = shape.solid_run(y) {
            assert!(
                x0 <= first && first < past && past <= x1,
                "{context}: run {first}..{past}"
            );
            let expected = shape.coverage(first, y);
            assert_eq!(expected.outer(), 1.0, "{context}: run at {first} {y}");
            assert!(
                expected.border() == 0.0 || expected.border() == 1.0,
                "{context}"
            );
            for x in first..past {
                let coverage = shape.coverage(x, y);
                assert_eq!(
                    (coverage.outer(), coverage.border()),
                    (expected.outer(), expected.border()),
                    "{context}: run pixel {x} {y}"
                );
            }
        }
        for x in x0..x1 {
            let coverage = shape.coverage(x, y);
            let (outer, border) = (coverage.outer(), coverage.border());
            assert!(
                (0.0..=1.0).contains(&outer),
                "{context}: outer {outer} at {x} {y}"
            );
            assert!(
                border >= 0.0 && border <= outer,
                "{context}: border {border} of {outer} at {x} {y}"
            );
            assert!(coverage.outer_distance().is_finite(), "{context}");
            assert_eq!(coverage.inner_distance().is_some(), inner_area, "{context}");
            if coverage.exact_arithmetic() {
                // Where exact arithmetic decides, coverage is saturated.
                assert!(
                    outer == 0.0 || outer == 1.0,
                    "{context}: {outer} at {x} {y}"
                );
                assert!(
                    border == 0.0 || border == 1.0,
                    "{context}: {border} at {x} {y}"
                );
            }
        }
    }
}

#[test]
fn rectangle_coverage_keeps_its_invariants_over_the_domain() -> Outcome {
    let mut random = Lcg(0x5eed_1234);
    let corner = |random: &mut Lcg| match (random.next() * 3.0) as u32 {
        0 => Corner::SQUARE,
        1 => Corner::round(length(random.range(0.0, 30.0))),
        _ => Corner::cut(length(random.range(0.0, 30.0))),
    };
    for case in 0..400 {
        let scale = [1.0, 1.25, 1.5, 2.0][case % 4];
        let mapping = mapping(48.0, 40.0, scale)?;
        let corners = Shape([
            corner(&mut random),
            corner(&mut random),
            corner(&mut random),
            corner(&mut random),
        ]);
        let geometry = rect(
            random.range(-10.0, 30.0),
            random.range(-10.0, 30.0),
            random.range(0.0, 40.0),
            random.range(0.0, 40.0),
            corners,
        );
        let border = (random.next() < 0.6).then(|| border_of(random.range(0.0, 8.0)));
        let clip = (random.next() < 0.3)
            .then(|| Rect::new(random.range(0.0, 20.0), random.range(0.0, 20.0), 17.3, 11.9))
            .transpose()?;
        let Some(shape) = PreparedShape::new(&geometry, border.as_ref(), clip, &mapping)? else {
            continue;
        };
        check_invariants(&shape, &format!("case {case}: {geometry:?} {border:?}"));
    }
    Ok(())
}

#[test]
fn arc_and_triangle_coverage_keep_their_invariants() -> Outcome {
    let mut random = Lcg(0xa5c3_0001);
    for case in 0..300 {
        let scale = [1.0, 1.25, 2.0][case % 3];
        let mapping = mapping(48.0, 40.0, scale)?;
        let geometry = if case % 2 == 0 {
            ShapeGeometry::Arc(ArcGeometry::new(
                Point::new(random.range(-5.0, 50.0), random.range(-5.0, 45.0))?,
                length(random.range(0.0, 25.0)),
                random.range(-720.0, 720.0),
                random.range(0.0, 360.0),
                length(random.range(0.0, 6.0)),
            )?)
        } else {
            let mut point = || Point::new(random.range(-10.0, 55.0), random.range(-10.0, 50.0));
            let Some(triangle) = TriangleGeometry::new([point()?, point()?, point()?]) else {
                continue;
            };
            ShapeGeometry::Triangle(triangle)
        };
        let Some(shape) = PreparedShape::new(&geometry, None, None, &mapping)? else {
            continue;
        };
        check_invariants(&shape, &format!("case {case}: {geometry:?}"));
        let [x0, y0, x1, y1] = shape.span();
        for y in y0..y1 {
            for x in x0..x1 {
                assert_eq!(shape.coverage(x, y).border(), 0.0);
            }
        }
    }
    Ok(())
}

#[test]
fn nothing_outside_the_span_is_covered() -> Outcome {
    // The span is the bounding box grown by one pixel, so the pixels on its
    // rim are already uncovered: no coverage is cut off by the span.
    let mut random = Lcg(0x0bad_cafe);
    let mapping = mapping(64.0, 64.0, 1.0)?;
    for case in 0..200 {
        let geometry = match case % 3 {
            0 => rect(
                random.range(8.0, 20.0),
                random.range(8.0, 20.0),
                random.range(1.0, 30.0),
                random.range(1.0, 30.0),
                Shape::rounded(length(random.range(0.0, 12.0))),
            ),
            1 => ShapeGeometry::Arc(ArcGeometry::new(
                Point::new(random.range(20.0, 40.0), random.range(20.0, 40.0))?,
                length(random.range(0.0, 12.0)),
                random.range(0.0, 360.0),
                random.range(0.1, 360.0),
                length(random.range(0.1, 5.0)),
            )?),
            _ => {
                let mut point = || Point::new(random.range(8.0, 56.0), random.range(8.0, 56.0));
                let Some(triangle) = TriangleGeometry::new([point()?, point()?, point()?]) else {
                    continue;
                };
                ShapeGeometry::Triangle(triangle)
            }
        };
        let Some(shape) = PreparedShape::new(&geometry, None, None, &mapping)? else {
            continue;
        };
        let [x0, y0, x1, y1] = shape.span();
        for x in x0..x1 {
            assert_eq!(
                shape.coverage(x, y0).outer(),
                0.0,
                "case {case}: top rim at {x}"
            );
            assert_eq!(
                shape.coverage(x, y1 - 1).outer(),
                0.0,
                "case {case}: bottom rim at {x}"
            );
        }
        for y in y0..y1 {
            assert_eq!(
                shape.coverage(x0, y).outer(),
                0.0,
                "case {case}: left rim at {y}"
            );
            assert_eq!(
                shape.coverage(x1 - 1, y).outer(),
                0.0,
                "case {case}: right rim at {y}"
            );
        }
    }
    Ok(())
}

#[test]
fn geometry_values_report_what_they_were_given() -> Outcome {
    let corner = Corner::cut(length(3.0));
    assert_eq!(
        (corner.kind(), corner.size()),
        (CornerKind::Cut, length(3.0))
    );
    assert_eq!(Corner::SQUARE.size(), length(0.0));
    let corners = Shape::rounded_top(length(2.0)).with_corner(CornerPosition::BottomLeft, corner);
    assert_eq!(
        corners.corner(CornerPosition::TopRight),
        Corner::round(length(2.0))
    );
    assert_eq!(corners.corner(CornerPosition::BottomRight), Corner::SQUARE);
    assert_eq!(corners.corner(CornerPosition::BottomLeft), corner);
    assert_eq!(Shape::default(), Shape::RECTANGLE);
    let center = Point::new(4.0, 5.0)?;
    let arc = ArcGeometry::new(center, length(6.0), 30.0, 40.0, length(2.0))?;
    assert_eq!(
        (arc.center(), arc.radius(), arc.width()),
        (center, length(6.0), length(2.0))
    );
    let vertices = [Point::ZERO, Point::new(1.0, 0.0)?, Point::new(0.0, 1.0)?];
    assert_eq!(
        TriangleGeometry::new(vertices)
            .ok_or("degenerate")?
            .vertices(),
        vertices
    );
    Ok(())
}

type Prepared = Result<Option<PreparedShape>, ShapePrepareError>;

#[test]
fn a_triangle_whose_vertices_merge_on_narrowing_draws_nothing() -> Outcome {
    // At a horizontal factor of 0.75, neighbouring `f32` positions in
    // [1.9, 2) map closer together than `f32` can tell apart.
    let mapping = mapping(40.0, 40.0, 0.75)?;
    let narrowed = |x: f32| (f64::from(x) * 0.75) as f32;
    let mut x = 1.9f32;
    while narrowed(x) != narrowed(x.next_up()) {
        x = x.next_up();
        assert!(x < 2.0, "no pair merges");
    }
    let next = x.next_up();
    let triangle = |points: [(f32, f32); 3]| -> Result<Prepared, Box<dyn std::error::Error>> {
        let [a, b, c] = points;
        let geometry = TriangleGeometry::new([
            Point::new(a.0, a.1)?,
            Point::new(b.0, b.1)?,
            Point::new(c.0, c.1)?,
        ])
        .ok_or("the logical test refused it")?;
        Ok(PreparedShape::new(
            &ShapeGeometry::Triangle(geometry),
            None,
            None,
            &mapping,
        ))
    };
    // Two vertices coincide after narrowing.
    assert_eq!(triangle([(x, 4.0), (next, 4.0), (x, 20.0)])?, Ok(None));
    // All three become collinear after narrowing.
    assert_eq!(triangle([(x, 4.0), (next, 12.0), (x, 20.0)])?, Ok(None));
    // The same triangle a few pixels wide is drawn.
    assert!(matches!(
        triangle([(x, 4.0), (x + 4.0, 12.0), (x, 20.0)])?,
        Ok(Some(_))
    ));
    // The shortest edge may be exactly 1/256 pixel, not shorter.
    let unit = self::mapping(40.0, 40.0, 1.0)?;
    let edge = |length: f32| -> Result<bool, Box<dyn std::error::Error>> {
        let geometry = TriangleGeometry::new([
            Point::new(10.0, 10.0)?,
            Point::new(10.0 + length, 10.0)?,
            Point::new(10.0, 30.0)?,
        ])
        .ok_or("the logical test refused it")?;
        Ok(PreparedShape::new(&ShapeGeometry::Triangle(geometry), None, None, &unit)?.is_some())
    };
    assert!(edge(1.0 / 256.0)?);
    assert!(!edge(1.0 / 512.0)?);
    Ok(())
}

#[test]
fn the_domain_bounds_what_an_evaluation_forms() -> Outcome {
    let mapping = mapping(100.0, 100.0, 1.0)?;
    // The span is the triangle's box within the frame, 91 pixels wide. A
    // vertex 65 445 pixels left of it is 65 536 from its far side: the last
    // accepted position.
    let triangle = |x: f32| -> Result<Prepared, Box<dyn std::error::Error>> {
        let geometry = TriangleGeometry::new([
            Point::new(x, 10.0)?,
            Point::new(90.0, 20.0)?,
            Point::new(90.0, 60.0)?,
        ])
        .ok_or("degenerate")?;
        Ok(PreparedShape::new(
            &ShapeGeometry::Triangle(geometry),
            None,
            None,
            &mapping,
        ))
    };
    assert!(matches!(triangle(-65_445.0)?, Ok(Some(_))));
    assert_eq!(triangle(-65_446.0)?, Err(ShapePrepareError::OutOfRange));
    // An arc whose parameters each fit, but whose rotated pixel offsets and
    // cap subtraction would not.
    let arc = |center: f32, radius: f32| -> Result<Prepared, Box<dyn std::error::Error>> {
        let geometry = ArcGeometry::new(
            Point::new(center, center)?,
            length(radius),
            0.0,
            90.0,
            length(40_000.0),
        )?;
        Ok(PreparedShape::new(
            &ShapeGeometry::Arc(geometry),
            None,
            None,
            &mapping,
        ))
    };
    assert_eq!(
        arc(-60_000.0, 65_536.0)?,
        Err(ShapePrepareError::OutOfRange)
    );
    // Center 10 000 px away on both axes: the far corner is √2 · 10 100
    // away, and with the radius that stays below the limit.
    assert!(matches!(arc(-10_000.0, 40_000.0)?, Ok(Some(_))));
    assert_eq!(
        arc(-10_000.0, 52_000.0)?,
        Err(ShapePrepareError::OutOfRange)
    );
    Ok(())
}

/// Coverage of `shape`, a rectangle, evaluated from its snapped edges
/// without relocation: the reference a relocated boundary must reproduce.
fn unrelocated(shape: &PreparedShape, edges: [f32; 4], border: f32, x: u32, y: u32) -> (f32, f32) {
    let ShapeParameters::Rect { outer, inner } = shape.parameters() else {
        panic!("not a rectangle");
    };
    let [x0, y0, ..] = shape.span();
    let origin = [x0 as f32, y0 as f32];
    let place = |inset: f32, corners: [PreparedCorner; 4]| Boundary {
        edges: [
            edges[0] + inset - origin[0],
            edges[1] + inset - origin[1],
            edges[2] - inset - origin[0],
            edges[3] - inset - origin[1],
        ],
        corners,
    };
    let p = [(x - x0) as f32 + 0.5, (y - y0) as f32 + 0.5];
    let outer_coverage = ramp(place(0.0, outer.corners).distance(p).0);
    let inside = match inner {
        Inner::NoBorder => outer_coverage,
        Inner::Filled => 0.0,
        Inner::Area(inner) => ramp(place(border, inner.corners).distance(p).0).min(outer_coverage),
    };
    (outer_coverage, outer_coverage - inside)
}

#[test]
fn relocated_boundaries_cover_what_the_unrelocated_ones_do() -> Outcome {
    let mapping = mapping(100.0, 100.0, 1.0)?;
    let all = Shape::RECTANGLE
        .with_corner(CornerPosition::TopLeft, Corner::round(length(9.0)))
        .with_corner(CornerPosition::TopRight, Corner::cut(length(7.0)))
        .with_corner(CornerPosition::BottomRight, Corner::round(length(5.0)))
        .with_corner(CornerPosition::BottomLeft, Corner::cut(length(11.0)));
    // (x, y, width, height, border, corners): edges near and far on every
    // side, borders that are partly in view, and a wide border on a large
    // square rectangle, whose inner edges are relocated on their own.
    let cases: [(f32, f32, f32, f32, f32, Shape); 6] = [
        (10.0, 12.0, 90_000.0, 90_000.0, 3.0, all),
        (-90_000.0, -90_000.0, 90_060.0, 90_050.0, 3.0, all),
        (-40_000.0, 20.0, 80_000.0, 60.0, 4.0, all),
        (30.0, -40_000.0, 40.0, 80_000.0, 4.0, all),
        (
            -30_000.0,
            -30_000.0,
            60_000.0,
            60_000.0,
            29_960.0,
            Shape::RECTANGLE,
        ),
        (
            -30_000.0,
            -30_000.0,
            30_070.0,
            30_080.0,
            40.0,
            Shape::rounded(length(30.0)),
        ),
    ];
    for (index, (x, y, w, h, border, corners)) in cases.into_iter().enumerate() {
        let shape = prepared(rect(x, y, w, h, corners), Some(border), &mapping)?;
        check_invariants(&shape, &format!("relocated case {index}"));
        // Whole logical coordinates at scale 1: the snapped edges are the
        // rectangle's own.
        let edges = [x, y, x + w, y + h];
        let [x0, y0, x1, y1] = shape.span();
        let mut border_pixels = 0;
        for py in y0..y1 {
            for px in x0..x1 {
                let coverage = shape.coverage(px, py);
                let expected = unrelocated(&shape, edges, border, px, py);
                assert_eq!(
                    (coverage.outer(), coverage.border()),
                    expected,
                    "case {index} at {px} {py}"
                );
                border_pixels += usize::from(coverage.border() > 0.0);
            }
        }
        assert!(border_pixels > 0, "case {index} shows no border");
    }
    Ok(())
}

#[test]
fn solid_runs_cover_a_rectangles_interior() -> Outcome {
    let mapping = mapping(60.0, 40.0, 1.0)?;
    // No border: the run is the row between the corners.
    let shape = prepared(
        rect(4.0, 4.0, 40.0, 20.0, Shape::rounded(length(6.0))),
        None,
        &mapping,
    )?;
    assert_eq!(shape.solid_run(14), Some([4, 44]));
    assert_eq!(
        shape.solid_run(5),
        Some([10, 38]),
        "inside the corners' band"
    );
    assert_eq!(shape.solid_run(3), None, "the rim row");
    assert_eq!(
        shape.solid_box(),
        Some([4, 10, 44, 18]),
        "clear of the corners"
    );
    assert_eq!(shape.solid_run(2), None, "outside the span");
    // With a border the run is the inner area, all fill.
    let shape = prepared(
        rect(4.0, 4.0, 40.0, 20.0, Shape::RECTANGLE),
        Some(3.0),
        &mapping,
    )?;
    assert_eq!(shape.solid_run(14), Some([7, 41]));
    assert_eq!(shape.coverage(7, 14).border(), 0.0);
    assert_eq!(shape.solid_run(5), None, "a row of the border band");
    assert_eq!(shape.solid_box(), Some([7, 7, 41, 21]), "the inner area");
    // A border that takes the whole silhouette: the run is all border.
    let shape = prepared(
        rect(4.0, 4.0, 40.0, 6.0, Shape::RECTANGLE),
        Some(5.0),
        &mapping,
    )?;
    assert_eq!(shape.solid_run(6), Some([4, 44]));
    assert_eq!(shape.coverage(4, 6).border(), 1.0);
    // A chamfer shortens the run on its rows, and a clip crops it.
    let shape = prepared(
        rect(4.0, 4.0, 40.0, 20.0, Shape::cut_top_right(length(8.0))),
        None,
        &mapping,
    )?;
    assert_eq!(shape.solid_run(20), Some([4, 44]));
    let [first, past] = shape.solid_run(6).ok_or("no run")?;
    assert!(first == 4 && past < 40, "{past}");
    let clip = Rect::new(10.0, 0.0, 5.0, 40.0)?;
    let clipped = PreparedShape::new(
        &rect(4.0, 4.0, 40.0, 20.0, Shape::RECTANGLE),
        None,
        Some(clip),
        &mapping,
    )?
    .ok_or("clipped away")?;
    assert_eq!(clipped.solid_run(14), Some([10, 15]));
    assert_eq!(clipped.solid_box(), Some([10, 4, 15, 24]));
    // A pill has runs but no box: its corners reach every row.
    let pill = prepared(
        rect(4.0, 4.0, 40.0, 10.0, Shape::rounded(length(5.0))),
        None,
        &mapping,
    )?;
    assert!(pill.solid_run(9).is_some());
    assert_eq!(pill.solid_box(), None);
    // Arcs and triangles have none.
    let arc = arc_shape(0.0, 360.0, 2.0, &mapping)?;
    assert_eq!(arc.solid_run(10), None);
    assert_eq!(arc.solid_box(), None);
    Ok(())
}

#[test]
fn an_arc_is_not_dropped_where_the_frame_maps_positions_and_lengths_apart() -> Outcome {
    // 64.4 logical pixels on a 64-pixel frame: positions map by 64 / 64.4
    // and lengths by the scale, 1. An arc centered 1000 to the left with a
    // radius of 1000 ends at 0.5 in logical terms, left of a clip from 2,
    // but its center maps to about -993.8, so it reaches pixel 6.
    let viewport = Viewport::new(64.4, 64.0, 1.0)?;
    let mapping = FrameMapping::new(64, 64, viewport)?;
    let clip = Rect::new(2.0, 0.0, 60.0, 64.0)?;
    let arc = |x: f32| -> Result<ShapeGeometry, Box<dyn std::error::Error>> {
        Ok(ShapeGeometry::Arc(ArcGeometry::new(
            Point::new(x, 32.0)?,
            length(1000.0),
            0.0,
            360.0,
            length(1.0),
        )?))
    };
    let near = arc(-1000.0)?;
    let shape = PreparedShape::new(&near, None, Some(clip), &mapping)?.ok_or("nothing")?;
    assert!(shape.coverage(6, 32).outer() > 0.5);
    assert!(!near.misses(clip, viewport));
    // What preparation leaves nothing of may be dropped, and far enough it is.
    let far = arc(-1010.0)?;
    assert!(PreparedShape::new(&far, None, Some(clip), &mapping)?.is_none());
    assert!(far.misses(clip, viewport));
    // With whole logical pixels the logical bounds decide, as before.
    let whole = Viewport::new(64.0, 64.0, 1.0)?;
    assert!(arc(-1000.0)?.misses(clip, whole));
    assert!(!arc(-998.0)?.misses(clip, whole));
    Ok(())
}
