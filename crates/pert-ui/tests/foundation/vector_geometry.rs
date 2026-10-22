//! Vector path, stroke and ellipse contracts through the public facade, observed
//! as the coverage masks a `DisplayList` carries.
use pert_ui::*;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct Draw<F>(F);
impl<F: Fn(&mut Painter) -> Result<(), DrawError> + 'static> Component for Draw<F> {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        (self.0)(painter)
    }
}

fn scene(
    scale: f64,
    draw: impl Fn(&mut Painter) -> Result<(), DrawError> + 'static,
) -> Result<DisplayList, DrawError> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(64.0, 64.0, scale)?)?;
    ui.mount("draw", Draw(draw))?;
    ui.flush(Duration::ZERO)?;
    ui.paint()?.rasterized()
}

/// The only mask of a scene: its destination, clip and coverage bytes.
fn mask(list: &DisplayList) -> (Rect, Option<Rect>, MaskImage) {
    let masks: Vec<_> = list
        .commands()
        .iter()
        .filter_map(|command| match command {
            DrawCommand::DrawMask {
                rect, mask, clip, ..
            } => Some((*rect, *clip, list.mask(*mask).cloned())),
            _ => None,
        })
        .collect();
    assert_eq!(masks.len(), 1, "expected exactly one mask in {list:?}");
    let (rect, clip, image) = masks[0].clone();
    (rect, clip, image.expect("scene resolves its own mask"))
}

fn at(image: &MaskImage, x: u32, y: u32) -> u8 {
    image.pixels()[(y * image.width() + x) as usize]
}

fn pt(x: f32, y: f32) -> Point {
    Point::new(x, y).expect("finite test point")
}

fn square(x: f32, y: f32, side: f32, clockwise: bool) -> Vec<Point> {
    let mut points = vec![
        pt(x, y),
        pt(x + side, y),
        pt(x + side, y + side),
        pt(x, y + side),
    ];
    if !clockwise {
        points.reverse();
    }
    points
}

fn width(value: f32) -> Length {
    Length::new(value).expect("valid test width")
}

fn color() -> Color {
    Color::from_linear(0.2, 0.4, 0.8, 1.0).expect("valid test color")
}

#[test]
fn a_fill_covers_whole_pixels_fully_and_split_pixels_by_area() -> TestResult {
    let list = scene(1.0, |p| {
        p.fill_path(
            &Path::polygon(square(8.0, 8.0, 16.0, true))?,
            FillRule::NonZero,
            color(),
        )
    })?;
    let (rect, clip, image) = mask(&list);
    assert_eq!(rect.values(), [8.0, 8.0, 16.0, 16.0]);
    assert_eq!(clip, None);
    assert_eq!((image.width(), image.height()), (16, 16));
    assert!(image.pixels().iter().all(|&alpha| alpha == 255));

    // An edge through the middle of a pixel column covers it by half.
    let list = scene(1.0, |p| {
        p.fill_path(
            &Path::polygon([pt(0.5, 0.0), pt(4.0, 0.0), pt(4.0, 4.0), pt(0.5, 4.0)])?,
            FillRule::NonZero,
            color(),
        )
    })?;
    let (rect, _, image) = mask(&list);
    assert_eq!(rect.values(), [0.5, 0.0, 4.0, 4.0]);
    // The grid is anchored at the path's bounds, so the half pixel is the last one.
    assert_eq!(at(&image, 0, 0), 255);
    assert_eq!(at(&image, 3, 0), 128);
    Ok(())
}

#[test]
fn fill_rules_decide_nested_and_opposite_subpaths() -> TestResult {
    let nested = |inner_clockwise: bool| -> Result<Path, PathError> {
        let mut builder = PathBuilder::new();
        for ring in [
            square(0.0, 0.0, 24.0, true),
            square(8.0, 8.0, 8.0, inner_clockwise),
        ] {
            builder = builder.move_to(ring[0]);
            for point in &ring[1..] {
                builder = builder.line_to(*point);
            }
            builder = builder.close();
        }
        builder.build()
    };
    for (inner_clockwise, rule, hole) in [
        (true, FillRule::NonZero, false),
        (false, FillRule::NonZero, true),
        (true, FillRule::EvenOdd, true),
        (false, FillRule::EvenOdd, true),
    ] {
        let path = nested(inner_clockwise)?;
        let (_, _, image) = mask(&scene(1.0, move |p| p.fill_path(&path, rule, color()))?);
        assert_eq!(at(&image, 2, 2), 255);
        assert_eq!(at(&image, 12, 12), if hole { 0 } else { 255 });
    }
    Ok(())
}

#[test]
fn caps_end_at_extend_past_or_round_the_endpoints() -> TestResult {
    let line = || Path::polyline([pt(8.0, 16.0), pt(24.0, 16.0)]);
    let stroked = |cap: LineCap| -> Result<(Rect, MaskImage), Box<dyn std::error::Error>> {
        let path = line()?;
        let (rect, _, image) = mask(&scene(1.0, move |p| {
            p.stroke_path(&path, StrokeStyle::new(width(4.0)).with_cap(cap), color())
        })?);
        Ok((rect, image))
    };
    let (butt, image) = stroked(LineCap::Butt)?;
    assert_eq!(butt.values(), [8.0, 14.0, 16.0, 4.0]);
    assert!(image.pixels().iter().all(|&alpha| alpha == 255));

    let (square_cap, image) = stroked(LineCap::Square)?;
    assert_eq!(square_cap.values(), [6.0, 14.0, 20.0, 4.0]);
    assert!(image.pixels().iter().all(|&alpha| alpha == 255));

    let (round, image) = stroked(LineCap::Round)?;
    assert!(round.x() > 5.9 && round.x() < 6.1);
    // The cap rounds off the square cap's corner pixel; its middle is covered.
    let corner = at(&image, 0, 0);
    assert!(
        corner > 0 && corner < 128,
        "rounded corner coverage {corner}"
    );
    assert_eq!(at(&image, 1, 2), 255);
    Ok(())
}

#[test]
fn joins_miter_bevel_or_round_the_outer_corner() -> TestResult {
    // A right angle turning down at (24, 8); the outer corner is (26, 6).
    let corner = |join: LineJoin, limit: f32| -> Result<MaskImage, Box<dyn std::error::Error>> {
        let style = StrokeStyle::new(width(4.0))
            .with_join(join)
            .with_miter_limit(limit)?;
        let (rect, _, image) = mask(&scene(1.0, move |p| {
            p.stroke_polyline(
                &[pt(8.0, 8.0), pt(24.0, 8.0), pt(24.0, 24.0)],
                style,
                color(),
            )
        })?);
        // Every join shares the segments' bounds except where the miter adds a corner.
        assert_eq!((rect.x(), rect.y()), (8.0, 6.0));
        Ok(image)
    };
    // Pixel (17, 0) is the one inside the outer corner, x 25..26 and y 6..7.
    assert_eq!(at(&corner(LineJoin::Miter, 4.0)?, 17, 0), 255);
    assert_eq!(at(&corner(LineJoin::Bevel, 4.0)?, 17, 0), 0);
    // A right angle's miter is sqrt(2) widths long, beyond a limit of 1.2.
    assert_eq!(at(&corner(LineJoin::Miter, 1.2)?, 17, 0), 0);
    let round = at(&corner(LineJoin::Round, 4.0)?, 17, 0);
    assert!(round > 0 && round < 255, "round join covers part: {round}");
    Ok(())
}

#[test]
fn zero_length_subpaths_draw_a_dot_or_square_only_with_those_caps() -> TestResult {
    let dot = |cap: LineCap| {
        scene(1.0, move |p| {
            p.stroke_polyline(
                &[pt(16.0, 16.0)],
                StrokeStyle::new(width(6.0)).with_cap(cap),
                color(),
            )
        })
    };
    assert!(dot(LineCap::Butt)?.commands().is_empty());
    let (rect, _, image) = mask(&dot(LineCap::Square)?);
    assert_eq!(rect.values(), [13.0, 13.0, 6.0, 6.0]);
    assert!(image.pixels().iter().all(|&alpha| alpha == 255));
    let (_, _, image) = mask(&dot(LineCap::Round)?);
    assert!(
        at(&image, 0, 0) < 32,
        "the disc leaves its square's corner almost empty"
    );
    assert_eq!(at(&image, 3, 3), 255);

    // A closed subpath of one point is zero-length too.
    let path = PathBuilder::new().move_to(pt(10.0, 10.0)).close().build()?;
    let list = scene(1.0, move |p| {
        p.stroke_path(
            &path,
            StrokeStyle::new(width(2.0)).with_cap(LineCap::Round),
            color(),
        )
    })?;
    assert_eq!(list.masks().len(), 1);
    Ok(())
}

#[test]
fn empty_zero_width_and_degenerate_geometry_record_nothing() -> TestResult {
    assert!(
        scene(1.0, |p| p.stroke_polyline(
            &[],
            StrokeStyle::new(width(2.0)),
            color()
        ))?
        .is_empty()
    );
    assert!(
        scene(1.0, |p| {
            p.stroke_polyline(
                &[pt(0.0, 0.0), pt(10.0, 10.0)],
                StrokeStyle::new(width(0.0)),
                color(),
            )
        })?
        .is_empty()
    );
    // Collinear fills and a zero-height ellipse enclose no area.
    assert!(
        scene(1.0, |p| {
            p.fill_path(
                &Path::polygon([pt(0.0, 0.0), pt(5.0, 5.0), pt(10.0, 10.0)])?,
                FillRule::NonZero,
                color(),
            )
        })?
        .is_empty()
    );
    assert!(
        scene(1.0, |p| p
            .fill_ellipse(Rect::new(4.0, 4.0, 10.0, 0.0)?, color()))?
        .is_empty()
    );
    // Repeated points inside a stroke are skipped rather than producing NaN joins.
    let list = scene(1.0, |p| {
        p.stroke_polyline(
            &[pt(4.0, 4.0), pt(4.0, 4.0), pt(20.0, 4.0), pt(20.0, 4.0)],
            StrokeStyle::new(width(2.0)),
            color(),
        )
    })?;
    assert_eq!(mask(&list).0.values(), [4.0, 3.0, 16.0, 2.0]);
    Ok(())
}

#[test]
fn builders_and_styles_reject_invalid_input() {
    assert_eq!(
        PathBuilder::new().line_to(pt(1.0, 1.0)).build(),
        Err(PathError::MissingMoveTo)
    );
    assert_eq!(
        PathBuilder::new().close().build(),
        Err(PathError::MissingMoveTo)
    );
    assert_eq!(PathBuilder::new().build(), Err(PathError::Empty));
    assert_eq!(
        PathBuilder::new().move_to(pt(1.0, 1.0)).build(),
        Err(PathError::Empty)
    );
    assert_eq!(Path::polyline([]), Err(PathError::Empty));
    for (start, sweep) in [
        (f32::NAN, 90.0),
        (0.0, 360.5),
        (0.0, -361.0),
        (f32::INFINITY, 1.0),
    ] {
        assert_eq!(
            PathBuilder::new()
                .arc(pt(0.0, 0.0), width(4.0), start, sweep)
                .build(),
            Err(PathError::InvalidParameter)
        );
    }
    for limit in [0.99, f32::NAN, f32::INFINITY, -1.0] {
        assert_eq!(
            StrokeStyle::new(width(1.0)).with_miter_limit(limit),
            Err(PathError::InvalidParameter)
        );
    }
    let too_many = (0..=Path::MAX_POINTS).map(|i| pt(i as f32, 0.0));
    assert_eq!(Path::polyline(too_many), Err(PathError::TooLarge));
    let limit = (0..Path::MAX_POINTS).map(|i| pt(i as f32, 0.0));
    assert!(Path::polyline(limit).is_ok());
    // A rectangle whose far edge overflows f32 cannot be an ellipse.
    let huge = Rect::new(f32::MAX, 0.0, f32::MAX, 1.0).expect("finite rectangle");
    assert_eq!(Path::ellipse(huge), Err(PathError::InvalidParameter));
    // A segment after close starts a new subpath at the closed one's start.
    let reopened = PathBuilder::new()
        .move_to(pt(2.0, 2.0))
        .line_to(pt(6.0, 2.0))
        .close()
        .line_to(pt(2.0, 9.0))
        .build()
        .expect("valid path");
    assert_eq!(reopened.bounds().values(), [2.0, 2.0, 4.0, 7.0]);
}

#[test]
fn oversized_rasters_are_rejected_and_the_clip_bounds_the_work() -> TestResult {
    let huge = || Path::polygon(square(0.0, 0.0, 5000.0, true));
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(5000.0, 5000.0, 1.0)?)?;
    ui.mount(
        "large",
        Draw(move |p: &mut Painter| p.fill_path(&huge()?, FillRule::NonZero, color())),
    )?;
    ui.flush(Duration::ZERO)?;
    let preserved = ui.paint()?;
    assert_eq!(preserved.vectors().len(), 1);
    assert!(
        preserved.masks().is_empty(),
        "recording does not allocate pixels"
    );
    assert_eq!(preserved.rasterized(), Err(DrawError::InvalidImage));
    // The same path clipped to a small group rasterizes only what is visible.
    let list = scene(1.0, move |p| {
        p.with_clip(Rect::new(10.0, 10.0, 20.0, 20.0)?, |p| {
            p.fill_path(&huge()?, FillRule::NonZero, color())
        })
    })?;
    let (rect, clip, image) = mask(&list);
    assert_eq!(rect.values(), [10.0, 10.0, 20.0, 20.0]);
    assert_eq!(clip.map(Rect::values), Some([10.0, 10.0, 20.0, 20.0]));
    assert_eq!((image.width(), image.height()), (20, 20));
    Ok(())
}

#[test]
fn a_clip_crops_the_raster_on_the_unclipped_grid() -> TestResult {
    let disc = || Path::circle(pt(20.0, 20.0), width(10.25));
    let (full_rect, _, full) = mask(&scene(1.0, move |p| {
        p.fill_path(&disc()?, FillRule::NonZero, color())
    })?);
    let list = scene(1.0, move |p| {
        p.with_clip(Rect::new(20.0, 0.0, 64.0, 64.0)?, |p| {
            p.fill_path(&disc()?, FillRule::NonZero, color())
        })
    })?;
    let (rect, clip, cropped) = mask(&list);
    assert!(clip.is_some());
    // The cropped raster starts on a whole pixel of the uncropped grid.
    let offset = (
        (rect.x() - full_rect.x()) as u32,
        (rect.y() - full_rect.y()) as u32,
    );
    assert_eq!(offset.1, 0);
    assert!(offset.0 > 0 && cropped.width() < full.width());
    for y in 0..cropped.height() {
        for x in 0..cropped.width() {
            assert_eq!(at(&cropped, x, y), at(&full, x + offset.0, y + offset.1));
        }
    }
    // A clip away from the path records nothing.
    let list = scene(1.0, move |p| {
        p.with_clip(Rect::new(40.0, 40.0, 8.0, 8.0)?, |p| {
            p.fill_path(&disc()?, FillRule::NonZero, color())
        })
    })?;
    assert!(list.is_empty());
    Ok(())
}

#[test]
fn rasters_follow_the_effective_scale_and_keep_logical_placement() -> TestResult {
    for scale in [1.0, 1.25, 2.0] {
        let (rect, _, image) = mask(&scene(scale, |p| {
            p.fill_path(
                &Path::polygon(square(4.0, 4.0, 8.0, true))?,
                FillRule::NonZero,
                color(),
            )
        })?);
        assert_eq!(rect.values(), [4.0, 4.0, 8.0, 8.0]);
        assert_eq!(image.width(), (8.0 * scale).ceil() as u32);
    }
    Ok(())
}

#[test]
fn curves_and_ellipses_approximate_their_exact_area() -> TestResult {
    let (rect, _, image) = mask(&scene(2.0, |p| {
        p.fill_ellipse(Rect::new(4.0, 4.0, 40.0, 40.0)?, color())
    })?);
    assert_eq!(rect.values(), [4.0, 4.0, 40.0, 40.0]);
    let area: f64 = image
        .pixels()
        .iter()
        .map(|&a| f64::from(a) / 255.0)
        .sum::<f64>()
        / 4.0;
    let exact = std::f64::consts::PI * 20.0 * 20.0;
    assert!(
        (area - exact).abs() / exact < 0.005,
        "area {area} vs {exact}"
    );
    assert_eq!(at(&image, 40, 40), 255);
    assert_eq!(at(&image, 1, 1), 0);

    // A centered stroke extends half its width outside the ellipse's rectangle.
    let (rect, _, _) = mask(&scene(1.0, |p| {
        p.stroke_ellipse(
            Rect::new(10.0, 10.0, 20.0, 10.0)?,
            StrokeStyle::new(width(4.0)),
            color(),
        )
    })?);
    // Within the 0.2-pixel flattening tolerance of the exact outline.
    for (actual, exact) in rect.values().into_iter().zip([8.0, 8.0, 24.0, 14.0]) {
        assert!((actual - exact).abs() < 0.2, "{:?}", rect.values());
    }

    // An arc built into a path ends where its sweep says.
    let path = PathBuilder::new()
        .arc(pt(20.0, 20.0), width(10.0), 0.0, 90.0)
        .build()?;
    let bounds = path.bounds();
    assert!((bounds.x() + bounds.width() - 30.0).abs() < 1e-4);
    assert!((bounds.y() + bounds.height() - 30.0).abs() < 1e-4);
    Ok(())
}

#[test]
fn equal_geometry_shares_one_mask_and_polylines_match_paths() -> TestResult {
    let style = StrokeStyle::new(width(3.0)).with_join(LineJoin::Round);
    let points = [pt(2.0, 2.0), pt(12.0, 18.0), pt(22.0, 4.0)];
    let list = scene(1.0, move |p| {
        p.stroke_polyline(&points, style, color())?;
        p.stroke_path(&Path::polyline(points)?, style, color())?;
        // The same outline shifted by whole pixels rasterizes identically.
        let shifted = points.map(|q| Point::new(q.x() + 30.0, q.y() + 30.0).expect("finite"));
        p.stroke_polyline(&shifted, style, color())
    })?;
    assert_eq!(list.len(), 3);
    assert_eq!(list.masks().len(), 1);
    Ok(())
}

#[test]
fn a_self_overlapping_stroke_is_one_mask_so_translucency_never_doubles() -> TestResult {
    // The trace crosses itself; the whole outline is one coverage mask, so the
    // crossing is blended once instead of once per overlapping piece.
    let list = scene(1.0, |p| {
        p.stroke_polyline(
            &[pt(0.0, 8.0), pt(16.0, 8.0), pt(8.0, 0.0), pt(8.0, 16.0)],
            StrokeStyle::new(width(2.0)),
            Color::from_linear(1.0, 1.0, 1.0, 0.5)?,
        )
    })?;
    assert_eq!(list.len(), 1);
    let (rect, _, image) = mask(&list);
    let crossing = ((8.0 - rect.x()) as u32, (8.0 - rect.y()) as u32);
    assert_eq!(at(&image, crossing.0, crossing.1), 255);
    Ok(())
}

#[test]
fn joins_follow_the_exact_tangents_not_the_flattened_segments() -> TestResult {
    let style = StrokeStyle::new(width(4.0));
    // A line meeting a curve at a right angle is a corner and receives the miter.
    let path = PathBuilder::new()
        .move_to(pt(8.0, 8.0))
        .line_to(pt(24.0, 8.0))
        .quad_to(pt(24.0, 24.0), pt(40.0, 24.0))
        .build()?;
    let (rect, _, image) = mask(&scene(1.0, move |p| p.stroke_path(&path, style, color()))?);
    assert_eq!((rect.x(), rect.y()), (8.0, 6.0));
    assert_eq!(at(&image, 17, 0), 255);
    // Where lines continue into corner arcs tangentially, nothing overshoots the
    // stroke's exact outline by more than the flattening tolerance.
    let (rect, _, _) = mask(&scene(1.0, move |p| {
        p.stroke_path(
            &Path::rounded_rect(Rect::new(10.0, 10.0, 30.0, 20.0)?, width(6.0))?,
            style,
            color(),
        )
    })?);
    assert!(rect.x() > 7.8 && rect.y() > 7.8, "{:?}", rect.values());
    Ok(())
}

#[test]
fn empty_nested_clips_omit_vector_commands_and_resources_before_rasterization() -> TestResult {
    for visible in [false, true] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(64.0, 64.0, 1.25)?)?;
        ui.mount(
            "draw",
            Draw(move |p: &mut Painter| {
                p.with_clip(Rect::new(1.0, 1.0, 4.0, 4.0)?, |p| {
                    p.with_clip(Rect::new(20.0, 20.0, 4.0, 4.0)?, |p| {
                        p.fill_path(
                            &Path::polygon(square(0.0, 0.0, 32.0, true))?,
                            FillRule::NonZero,
                            color(),
                        )
                    })
                })?;
                if visible {
                    p.fill_path(
                        &Path::polygon(square(16.0, 16.0, 10.0, true))?,
                        FillRule::NonZero,
                        color(),
                    )?;
                }
                Ok(())
            }),
        )?;
        ui.flush(ui.now())?;
        let scene = ui.paint()?;
        assert_eq!(scene.commands().len(), usize::from(visible));
        assert_eq!(scene.vectors().len(), usize::from(visible));
        assert!(scene.images().is_empty());
        assert!(scene.masks().is_empty());
        if visible {
            let DrawCommand::DrawVector { vector, clip, .. } = scene.commands()[0] else {
                panic!("visible vector must survive")
            };
            assert!(scene.vector(vector).is_some());
            assert!(clip.is_none());
        }
    }
    Ok(())
}

#[test]
fn huge_uncropped_strokes_render_all_visible_rows_without_index_overflow() -> TestResult {
    for height in [100.0, 100000.0] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(10.0, height, 1.0)?)?;
        ui.mount(
            "draw",
            Draw(|painter: &mut Painter| {
                painter.stroke_polyline(
                    &[pt(1.0, -1e20), pt(1.0, 100000.0)],
                    StrokeStyle::new(Length::new(1.0)?),
                    painter.color(ColorRole::Primary),
                )
            }),
        )?;
        ui.flush(Duration::ZERO)?;
        let list = ui.paint()?.rasterized()?;
        let (rect, _, image) = mask(&list);
        assert_eq!(rect.y(), 0.0);
        assert_eq!(image.height(), height as u32);
        assert!(
            image.pixels()[image.pixels().len() - image.width() as usize..]
                .iter()
                .any(|p| *p > 0)
        );
    }
    Ok(())
}

#[test]
fn remote_diagonal_stroke_preserves_its_near_endpoint_intercept() -> TestResult {
    for scale in [1.0, 1.25, 1.5] {
        let list = scene(scale, |painter| {
            painter.stroke_polyline(
                &[pt(-1e20, -1e20), pt(50.0, 60.0)],
                StrokeStyle::new(Length::new(2.0)?),
                painter.color(ColorRole::Primary),
            )
        })?;
        let (rect, _, image) = mask(&list);
        let sample = |x: f64, y: f64| {
            let x = ((x - f64::from(rect.x())) * scale).floor() as u32;
            let y = ((y - f64::from(rect.y())) * scale).floor() as u32;
            if x < image.width() && y < image.height() {
                at(&image, x, y)
            } else {
                0
            }
        };
        assert!(sample(20.0, 30.0) > 0);
        assert_eq!(sample(20.0, 20.0), 0);
    }
    Ok(())
}

#[test]
fn a_curve_that_cannot_meet_the_tolerance_returns_a_typed_budget_error() -> TestResult {
    let error = scene(1.0, |painter| {
        let path = PathBuilder::new()
            .move_to(Point::ZERO)
            .quad_to(pt(0.0, f32::MAX), pt(1.0, 0.0))
            .build()?;
        painter.fill_path(&path, FillRule::NonZero, color())
    })
    .expect_err("the curve must not silently flatten to a coarse polygon");
    assert_eq!(error, DrawError::Path(PathError::TooLarge));
    Ok(())
}

#[test]
fn a_large_asymmetric_curve_covers_its_analytically_known_visible_apex() -> TestResult {
    // x(t) = 200e6 * (t - .37), y(t) = 32 - x(t)^2 / 200e6.
    // The apex is deliberately off the old 1024-segment grid. Over this small
    // viewport the exact top differs from y=32 by less than .000021 units.
    // Closing the curve fills the region below it. These expectations follow
    // the equation, independently of the fallback's subdivision algorithm.
    for scale in [1.0, 1.25, 1.5, 8.0] {
        let list = scene(scale, |painter| {
            let path = PathBuilder::new()
                .move_to(pt(-74e6, -27_379_968.0))
                .quad_to(pt(26e6, 46_620_032.0), pt(126e6, -79_379_968.0))
                .close()
                .build()?;
            painter.fill_path(&path, FillRule::NonZero, color())
        })?;
        let (rect, _, image) = mask(&list);
        assert_eq!(rect.y(), 0.0);
        assert_eq!(image.height(), (32.0 * scale) as u32);
        let apex_row = image.height() - 1;
        for x in 0..image.width() {
            assert_eq!(at(&image, x, apex_row - 1), 255);
            // A .2-physical-pixel chord error may omit the last of four
            // sub-scanlines, but cannot omit the first three (191/255).
            assert!(at(&image, x, apex_row) >= 191, "scale {scale}, column {x}");
        }
    }
    Ok(())
}

#[test]
fn a_remote_diagonal_fill_preserves_the_visible_edge_intercepts() -> TestResult {
    for scale in [1.0, 1.25, 1.5] {
        let list = scene(scale, |painter| {
            painter.fill_path(
                &Path::polygon([pt(-1e20, -1e20), pt(50.0, 60.0), pt(50.0, 61.0)])?,
                FillRule::NonZero,
                color(),
            )
        })?;
        let (rect, _, image) = mask(&list);
        let sample = |x: f64, y: f64| {
            at(
                &image,
                ((x - f64::from(rect.x())) * scale).floor() as u32,
                ((y - f64::from(rect.y())) * scale).floor() as u32,
            )
        };
        assert!(sample(20.0, 30.0) > 0);
        assert_eq!(sample(20.0, 20.0), 0);
    }
    Ok(())
}

#[test]
fn remote_path_bounds_contain_the_near_endpoints_before_coverage_clipping() -> TestResult {
    let path = Path::polygon([pt(-1e20, -1e20), pt(50.0, 60.0), pt(50.0, 61.0)])?;
    let [x, y, w, h] = path.bounds().values();
    assert!(f64::from(x) + f64::from(w) >= 50.0);
    assert!(f64::from(y) + f64::from(h) >= 61.0);
    // The preceding representable extents would miss the endpoints: this
    // fixture needs exactly one outward step, not arbitrary padding.
    assert!(f64::from(x) + f64::from(w.next_down()) < 50.0);
    assert!(f64::from(y) + f64::from(h.next_down()) < 61.0);
    assert!(
        !path
            .bounds()
            .intersection(Rect::new(0.0, 0.0, 64.0, 64.0)?)
            .is_empty()
    );
    assert_eq!(
        Path::polyline([pt(-f32::MAX, 0.0), pt(1.0, 0.0)]),
        Err(PathError::InvalidParameter),
        "outward rounding must not turn a finite extent into an infinite bound",
    );
    Ok(())
}

#[test]
fn positive_remote_stroke_endpoints_match_near_visible_geometry() -> TestResult {
    for scale in [1.0, 1.25, 1.5, 8.0] {
        for cap in [LineCap::Butt, LineCap::Round, LineCap::Square] {
            for join in [LineJoin::Round, LineJoin::Bevel, LineJoin::Miter] {
                for reversed in [false, true] {
                    let render = |far: f32| -> Result<DisplayList, DrawError> {
                        let mut points = vec![
                            pt(far, if far > 1e10 { far } else { far - 5.0 }),
                            pt(10.0, 5.0),
                            pt(0.0, 0.0),
                        ];
                        if reversed {
                            points.reverse();
                        }
                        let path = Path::polyline(points)?;
                        let style = StrokeStyle::new(width(2.0)).with_cap(cap).with_join(join);
                        scene(scale, move |p| p.stroke_path(&path, style, color()))
                    };
                    let (actual_rect, actual_clip, actual) = mask(&render(1e20)?);
                    let (expected_rect, expected_clip, expected) = mask(&render(100.0)?);
                    assert_eq!(actual_rect, expected_rect);
                    assert_eq!(actual_clip, expected_clip);
                    assert_eq!(actual.pixels().len(), expected.pixels().len());
                    for (&a, &b) in actual.pixels().iter().zip(expected.pixels()) {
                        assert!(
                            a.abs_diff(b) <= 1,
                            "{scale} {cap:?} {join:?} reversed={reversed}: {a} vs {b}"
                        );
                    }
                }
            }
        }
    }
    Ok(())
}
