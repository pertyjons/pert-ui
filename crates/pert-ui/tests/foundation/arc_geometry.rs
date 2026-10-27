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
    assert!(matches!(
        arc(0.0, 90.0, 100000.0, 2.0, 1.0),
        Err(DrawError::CoverageTooLarge)
    ));
    let ring = arc(0.0, 360.0, 24.0, 2.0, 1.0).unwrap();
    let mask = &ring.masks()[0];
    let w = mask.width() as usize;
    assert_eq!(mask.pixels()[w / 2 * w + w / 2], 0);
    assert!(mask.pixels()[w / 2 * w] > 0 && mask.pixels()[w / 2] > 0);
    assert_eq!(
        ring.masks(),
        arc(360.0, 360.0, 24.0, 2.0, 1.0).unwrap().masks()
    );
}
#[test]
fn clockwise_arc_covers_start_to_endpoint_with_antialiasing_at_fractional_dpi() {
    for scale in [1.0, 1.25, 2.0] {
        let scene = arc(135.0, 135.0, 24.0, 2.0, scale).unwrap();
        let mask = &scene.masks()[0];
        let w = mask.width() as usize;
        // A lower-left -> top sweep passes the left radius, leaving the right empty.
        assert!(mask.pixels()[w / 2 * w] > 0);
        assert_eq!(mask.pixels()[w / 2 * w + w - 1], 0);
        assert!(mask.pixels().iter().any(|p| *p > 0 && *p < 255));
        let full = arc(135.0, 270.0, 24.0, 2.0, scale).unwrap();
        let sum = |s: &DisplayList| {
            s.masks()[0]
                .pixels()
                .iter()
                .map(|p| u64::from(*p))
                .sum::<u64>()
        };
        assert!(sum(&full) > sum(&scene));
    }
}
