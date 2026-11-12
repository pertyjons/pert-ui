//! General `Painter::stroke_arc` geometry through the public facade.
use pert_ui::*;
use std::time::Duration;
struct Arc {
    start: f32,
    sweep: f32,
    radius: f32,
    width: f32,
}
impl Component for Arc {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        p.stroke_arc(
            Point::new(32.0, 32.0)?,
            Length::new(self.radius)?,
            self.start,
            self.sweep,
            Length::new(self.width)?,
            p.color(ColorRole::Primary),
        )
    }
}
fn arc(
    start: f32,
    sweep: f32,
    radius: f32,
    width: f32,
    scale: f64,
) -> Result<DisplayList, DrawError> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(64.0, 64.0, scale)?).unwrap();
    ui.mount_tree(Element::new(
        "arc",
        Arc {
            start,
            sweep,
            radius,
            width,
        },
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.paint()
}
/// The scene's one arc, prepared for a frame of the viewport's physical size.
fn prepared(scene: &DisplayList) -> Result<Option<PreparedShape>, ShapePrepareError> {
    let [
        DrawCommand::DrawShape {
            geometry: geometry @ ShapeGeometry::Arc(_),
            border,
            clip,
            ..
        },
    ] = scene.commands()
    else {
        panic!("not one arc: {:?}", scene.commands());
    };
    assert!(scene.masks().is_empty());
    let viewport = scene.viewport();
    let side = (64.0 * viewport.scale()).round() as u32;
    let mapping = FrameMapping::new(side, side, viewport).unwrap();
    PreparedShape::new(geometry, border.as_ref(), *clip, &mapping)
}
/// The arc's coverage of every frame pixel, by rows, and the frame's side.
fn pixels(scene: &DisplayList) -> (Vec<f32>, usize) {
    let side = (64.0 * scene.viewport().scale()).round() as u32;
    let shape = prepared(scene).unwrap().expect("an arc to draw");
    let pixels = (0..side)
        .flat_map(|y| (0..side).map(move |x| (x, y)))
        .map(|(x, y)| shape.coverage(x, y).outer())
        .collect();
    (pixels, side as usize)
}
#[test]
fn circular_arc_handles_empty_complete_invalid_and_oversized_geometry() {
    assert!(arc(0.0, 0.0, 24.0, 2.0, 1.0).unwrap().commands().is_empty());
    assert!(
        arc(0.0, 90.0, 24.0, 0.0, 1.0)
            .unwrap()
            .commands()
            .is_empty()
    );
    for (start, sweep) in [
        (f32::NAN, 90.0),
        (0.0, -1.0),
        (0.0, 361.0),
        (0.0, f32::INFINITY),
    ] {
        assert!(matches!(
            arc(start, sweep, 24.0, 2.0, 1.0),
            Err(DrawError::Path(PathError::InvalidParameter))
        ));
    }
    // An arc beyond the evaluation range is recorded, and refused with a
    // typed cause when a renderer prepares it.
    let oversized = arc(0.0, 90.0, 100000.0, 2.0, 1.0).unwrap();
    assert!(matches!(
        prepared(&oversized),
        Err(ShapePrepareError::OutOfRange)
    ));
    let ring = arc(0.0, 360.0, 24.0, 2.0, 1.0).unwrap();
    let (coverage, w) = pixels(&ring);
    // The center is empty; the left and the top of the ring are covered.
    assert_eq!(coverage[w / 2 * w + w / 2], 0.0);
    assert!(coverage[w / 2 * w + 8] > 0.0 && coverage[8 * w + w / 2] > 0.0);
    // A full ring covers the same pixels from any start angle.
    assert_eq!(
        coverage,
        pixels(&arc(360.0, 360.0, 24.0, 2.0, 1.0).unwrap()).0
    );
}
#[test]
fn clockwise_arc_covers_start_to_endpoint_with_antialiasing_at_fractional_dpi() {
    for scale in [1.0, 1.25, 2.0] {
        let scene = arc(135.0, 135.0, 24.0, 2.0, scale).unwrap();
        let (coverage, w) = pixels(&scene);
        // The stroke center lies 24 logical pixels from the middle.
        let radius = (24.0 * scale) as usize;
        // A lower-left -> top sweep passes the left radius, leaving the right empty.
        assert!(coverage[w / 2 * w + w / 2 - radius] > 0.0);
        assert_eq!(coverage[w / 2 * w + w / 2 + radius], 0.0);
        assert_eq!(coverage[w / 2 * w + w / 2 + radius - 1], 0.0);
        assert!(coverage.iter().any(|p| *p > 0.0 && *p < 1.0));
        let full = arc(135.0, 270.0, 24.0, 2.0, scale).unwrap();
        let sum = |s: &DisplayList| pixels(s).0.iter().map(|p| f64::from(*p)).sum::<f64>();
        assert!(sum(&full) > sum(&scene));
    }
}
