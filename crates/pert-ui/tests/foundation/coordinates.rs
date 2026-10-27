//! The frozen coordinate, viewport and clipping contract, through the facade.
//!
//! These pin the rules in [the coordinate contract](../../../docs/rendering-platform.md).
//! They are deliberately written against observable `DisplayList` output rather
//! than internal geometry, because the contract describes visual meaning that any
//! backend must reproduce, not this runtime's implementation.
use pert_ui::*;
use std::time::Duration;

/// Paints one fill and one coverage mask at caller-chosen local coordinates,
/// optionally inside a local clip, so a test can observe how each is recorded.
struct Probe {
    fill: Rect,
    mask: Option<Rect>,
    clip: Option<Rect>,
}
impl Probe {
    fn new(fill: Rect) -> Self {
        Self {
            fill,
            mask: None,
            clip: None,
        }
    }
    fn mask(mut self, rect: Rect) -> Self {
        self.mask = Some(rect);
        self
    }
    fn clip(mut self, rect: Rect) -> Self {
        self.clip = Some(rect);
        self
    }
    fn draw(&self, p: &mut Painter) -> Result<(), DrawError> {
        p.fill_rect(self.fill, Color::ACCENT);
        if let Some(rect) = self.mask {
            // A two-by-two opaque coverage image, so cropping is observable in the
            // command's geometry rather than in rasterization.
            let image = MaskImage::new(2, 2, vec![255; 4])?;
            p.draw_mask(&image, rect, Color::ACCENT);
        }
        Ok(())
    }
}
impl Component for Probe {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    /// Reserves the space it paints, so a container allocates a real rectangle.
    /// A zero-sized child would be clipped away entirely, and composition drops
    /// what an empty clip removes, so its translation would not be observable.
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        let [x, y, width, height] = self.fill.values();
        Ok(Size::new(x + width, y + height)?)
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        match self.clip {
            Some(clip) => p.with_clip(clip, |p| self.draw(p)),
            None => self.draw(p),
        }
    }
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::new(x, y, w, h).unwrap()
}
fn scene(viewport: Viewport, probe: Probe) -> DisplayList {
    let mut ui = Runtime::new();
    ui.set_viewport(viewport).unwrap();
    ui.mount_tree(Element::new("probe", probe)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.paint().unwrap()
}
fn fills(scene: &DisplayList) -> Vec<[f32; 4]> {
    scene
        .commands()
        .iter()
        .filter_map(|command| match command {
            DrawCommand::FillRect { rect, .. } => Some(rect.values()),
            _ => None,
        })
        .collect()
}
fn masks(scene: &DisplayList) -> Vec<([f32; 4], Option<[f32; 4]>)> {
    scene
        .commands()
        .iter()
        .filter_map(|command| match command {
            DrawCommand::DrawMask { rect, clip, .. } => {
                Some((rect.values(), clip.map(Rect::values)))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn zoom_divides_logical_space_and_leaves_the_physical_extent_alone() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.5).unwrap())
        .unwrap();
    ui.mount_tree(Element::new(
        "probe",
        Probe::new(rect(0.0, 0.0, 10.0, 10.0)),
    ))
    .unwrap();
    let settings = ui.ui_settings();
    for factor in [0.75, 1.0, 1.25, 2.0] {
        settings.set_zoom(UiZoom::new(factor).unwrap()).unwrap();
        ui.flush(Duration::ZERO).unwrap();
        let viewport = ui.paint().unwrap().viewport();
        // The effective viewport is the native one divided by zoom, with the scale
        // multiplied by it, so the window keeps its physical size and only the
        // amount of logical space inside it changes.
        assert_eq!(viewport.scale(), 1.5 * factor);
        assert!((f64::from(viewport.width()) - 800.0 / factor).abs() < 0.001);
        assert!((f64::from(viewport.height()) - 600.0 / factor).abs() < 0.001);
        for (logical, native) in [(viewport.width(), 800.0), (viewport.height(), 600.0)] {
            let physical = f64::from(logical) * viewport.scale();
            assert!(
                (physical - native * 1.5).abs() < 0.01,
                "zoom {factor} changed the physical extent to {physical}"
            );
        }
    }
}

#[test]
fn a_native_viewport_keeps_the_exact_physical_extent_through_zoom() {
    // An odd extent at a fractional scale is the case the contract warns about: the
    // logical size is a rounded quotient, so a host must not multiply it back.
    let extent = PhysicalExtent::new(1001, 601);
    let native = Viewport::from_physical(extent, 1.5).unwrap();
    assert_eq!(native.width(), (1001.0_f64 / 1.5) as f32);
    assert_eq!(native.height(), (601.0_f64 / 1.5) as f32);
    assert_eq!((extent.width(), extent.height()), (1001, 601));
    let mut ui = Runtime::new();
    ui.set_viewport(native).unwrap();
    ui.mount_tree(Element::new(
        "probe",
        Probe::new(rect(0.0, 0.0, 10.0, 10.0)),
    ))
    .unwrap();
    let settings = ui.ui_settings();
    for factor in [0.75, 1.0, 1.25, 2.0] {
        settings.set_zoom(UiZoom::new(factor).unwrap()).unwrap();
        ui.flush(Duration::ZERO).unwrap();
        // Both the runtime and the scene a backend receives report the extent the
        // host supplied, not one reconstructed from the zoomed logical size.
        assert_eq!(ui.viewport().physical(), Some(extent));
        assert_eq!(ui.paint().unwrap().viewport().physical(), Some(extent));
    }
    // A viewport built from a logical size has no extent to report, and a zero
    // extent is the suspended viewport a minimized window produces.
    assert_eq!(Viewport::new(800.0, 600.0, 1.5).unwrap().physical(), None);
    let minimized = Viewport::from_physical(PhysicalExtent::new(0, 0), 2.0).unwrap();
    assert!(minimized.physical().is_some_and(PhysicalExtent::is_empty));
    assert_eq!((minimized.width(), minimized.height()), (0.0, 0.0));
    for scale in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(Viewport::from_physical(extent, scale).is_err());
    }
}

#[test]
fn named_components_agree_with_the_destructured_arrays() {
    let rect = Rect::new(-4.0, 8.0, 80.0, 24.0).unwrap();
    assert_eq!(
        [rect.x(), rect.y(), rect.width(), rect.height()],
        rect.values()
    );
    let color = Color::from_linear(0.1, 0.2, 0.3, 0.4).unwrap();
    assert_eq!(
        [color.red(), color.green(), color.blue(), color.alpha()],
        color.channels()
    );
}

#[test]
fn fractional_scales_leave_logical_geometry_exact_and_unsnapped() {
    // 1.25 is the canonical fractional case. Nothing rounds the recorded geometry:
    // scale applies to rasterization and the surface, never to these coordinates.
    let scene = scene(
        Viewport::new(100.0, 100.0, 1.25).unwrap(),
        Probe::new(rect(0.5, 1.5, 10.25, 3.75)),
    );
    assert_eq!(scene.viewport().scale(), 1.25);
    assert_eq!(fills(&scene), vec![[0.5, 1.5, 10.25, 3.75]]);
}

#[test]
fn a_zero_sized_viewport_is_valid_and_suspends_presentation() {
    for viewport in [
        Viewport::new(0.0, 0.0, 1.0).unwrap(),
        Viewport::new(320.0, 0.0, 2.0).unwrap(),
        Viewport::new(0.0, 240.0, 1.25).unwrap(),
    ] {
        let scene = scene(viewport, Probe::new(rect(0.0, 0.0, 10.0, 10.0)));
        assert_eq!(scene.viewport().width(), viewport.width());
        assert_eq!(scene.viewport().height(), viewport.height());
    }
    // A zero or negative scale is not a suspended viewport but invalid input.
    for scale in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(Viewport::new(320.0, 240.0, scale).is_err());
    }
    assert!(Viewport::new(f32::NAN, 240.0, 1.0).is_err());
    assert!(Viewport::new(-1.0, 240.0, 1.0).is_err());
}

#[test]
fn a_local_clip_crops_a_fill_when_recorded_but_only_accompanies_a_mask() {
    let scene = scene(
        Viewport::new(100.0, 100.0, 1.0).unwrap(),
        Probe::new(rect(0.0, 0.0, 40.0, 40.0))
            .mask(rect(0.0, 0.0, 40.0, 40.0))
            .clip(rect(10.0, 10.0, 20.0, 20.0)),
    );
    // A fill carries no clip, because intersecting a rectangle with a rectangle
    // loses nothing.
    assert_eq!(fills(&scene), vec![[10.0, 10.0, 20.0, 20.0]]);
    // A mask keeps its full destination and carries the clip beside it. Shrinking
    // the destination instead would rescale the image rather than crop it.
    assert_eq!(
        masks(&scene),
        vec![([0.0, 0.0, 40.0, 40.0], Some([10.0, 10.0, 20.0, 20.0]))]
    );
}

#[test]
fn nested_local_clips_intersect_and_an_empty_intersection_has_zero_area() {
    struct Nested;
    impl Component for Nested {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
            p.with_clip(rect(0.0, 0.0, 50.0, 50.0), |p| {
                p.with_clip(rect(40.0, 40.0, 50.0, 50.0), |p| {
                    p.fill_rect(rect(0.0, 0.0, 100.0, 100.0), Color::ACCENT);
                });
                // Disjoint clips never produce a wrapped or negative rectangle.
                p.with_clip(rect(60.0, 60.0, 10.0, 10.0), |p| {
                    p.fill_rect(rect(0.0, 0.0, 100.0, 100.0), Color::ACCENT);
                });
                Ok(())
            })
        }
    }
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(100.0, 100.0, 1.0).unwrap())
        .unwrap();
    ui.mount_tree(Element::new("nested", Nested)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let scene = ui.paint().unwrap();
    let recorded = fills(&scene);
    // Only the overlapping group survives. `Rect::intersection` clamps at zero
    // rather than wrapping, and composition drops a command that an empty clip
    // has already removed, so the disjoint group reaches no backend at all.
    assert_eq!(recorded, vec![[40.0, 40.0, 10.0, 10.0]]);
    assert!(
        rect(0.0, 0.0, 50.0, 50.0)
            .intersection(rect(60.0, 60.0, 10.0, 10.0))
            .is_empty()
    );
    assert!(recorded.iter().all(|[_, _, w, h]| *w >= 0.0 && *h >= 0.0));
}

#[test]
fn composition_translates_local_coordinates_into_window_coordinates() {
    // Two probes in a column: the second one's local origin is not the window's,
    // so its recorded geometry must arrive offset by its allocated position.
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 200.0, 1.0).unwrap())
        .unwrap();
    ui.mount_tree(Element::new(
        "column",
        Column::new([
            Element::new("first", Probe::new(rect(0.0, 0.0, 20.0, 20.0))),
            Element::new("second", Probe::new(rect(0.0, 0.0, 20.0, 20.0))),
        ]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let recorded = fills(&ui.paint().unwrap());
    assert_eq!(recorded.len(), 2);
    let [first_x, first_y, ..] = recorded[0];
    let [second_x, second_y, ..] = recorded[1];
    assert_eq!(
        first_x, second_x,
        "a column must not shift children sideways"
    );
    assert!(
        second_y > first_y,
        "the second child must be translated below the first, got {second_y} and {first_y}"
    );
}

#[test]
fn only_translation_exists_so_a_size_change_never_rescales_recorded_geometry() {
    // The contract has no rotation, scale or affine transform. The same painted
    // rectangle keeps its logical size under a different viewport scale; only the
    // scale the backend rasterizes and presents at differs.
    let small = scene(
        Viewport::new(100.0, 100.0, 1.0).unwrap(),
        Probe::new(rect(4.0, 6.0, 12.0, 8.0)),
    );
    let large = scene(
        Viewport::new(100.0, 100.0, 2.0).unwrap(),
        Probe::new(rect(4.0, 6.0, 12.0, 8.0)),
    );
    assert_eq!(fills(&small), fills(&large));
    assert_eq!(fills(&small), vec![[4.0, 6.0, 12.0, 8.0]]);
    assert_eq!(large.viewport().scale(), 2.0);
}

#[test]
fn a_mask_stretches_to_its_destination_and_keeps_its_source_when_cropped() {
    let uncropped = scene(
        Viewport::new(100.0, 100.0, 1.0).unwrap(),
        Probe::new(rect(0.0, 0.0, 1.0, 1.0)).mask(rect(10.0, 10.0, 40.0, 20.0)),
    );
    // The whole image maps to the destination rectangle, whatever its pixel size;
    // only the root's own allocation, the whole window, clips it.
    assert_eq!(
        masks(&uncropped),
        vec![([10.0, 10.0, 40.0, 20.0], Some([0.0, 0.0, 100.0, 100.0]))]
    );
    assert_eq!(uncropped.masks().len(), 1);
    assert_eq!(
        (uncropped.masks()[0].width(), uncropped.masks()[0].height()),
        (2, 2)
    );
    let cropped = scene(
        Viewport::new(100.0, 100.0, 1.0).unwrap(),
        Probe::new(rect(0.0, 0.0, 1.0, 1.0))
            .mask(rect(10.0, 10.0, 40.0, 20.0))
            .clip(rect(10.0, 10.0, 20.0, 20.0)),
    );
    // Cropping leaves the destination alone, so the visible half keeps the sampling
    // it had before the clip instead of compressing the whole image into it.
    assert_eq!(
        masks(&cropped),
        vec![([10.0, 10.0, 40.0, 20.0], Some([10.0, 10.0, 20.0, 20.0]))]
    );
}

/// Records the window position it was handed and its own allocated bounds, so a
/// test can check which space input arrives in and how a component leaves it.
/// The window position handed to the probe, the bounds it was allocated, and the
/// local position it computed with `EventContext::to_local`.
type Observed = Option<([f32; 2], [f32; 4], [f32; 2])>;
#[derive(Clone)]
struct Reporter(std::rc::Rc<std::cell::Cell<Observed>>);
impl Component for Reporter {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(400.0, 400.0)?)
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        p.fill_rect(rect(0.0, 0.0, 1000.0, 1000.0), Color::ACCENT);
        Ok(())
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        e: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if let Some((bounds, local)) = cx.bounds().zip(cx.to_local(e.position())) {
            self.0.set(Some((
                [e.position().x(), e.position().y()],
                bounds.values(),
                [local.x(), local.y()],
            )));
        }
        Ok(InputOutcome::Handled)
    }
}

#[test]
fn pointer_positions_arrive_in_effective_logical_window_coordinates() {
    let seen = std::rc::Rc::new(std::cell::Cell::new(None));
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 400.0, 2.0).unwrap())
        .unwrap();
    ui.mount_tree(Element::new("reporter", Reporter(seen.clone())))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.paint().unwrap();
    ui.pointer_input(PointerEvent::new(
        Point::new(120.0, 80.0).unwrap(),
        PointerKind::Move,
    ))
    .unwrap();
    let (position, bounds, _) = seen.get().expect("the probe must receive the move");
    // Input shares the window-logical space with layout, so a position is directly
    // comparable with the bounds the same component was allocated. A component
    // reaches its own local space by subtracting its origin, which is the only
    // conversion it ever needs.
    assert_eq!(position, [120.0, 80.0]);
    assert_eq!([bounds[0], bounds[1]], [0.0, 0.0]);

    // Under zoom the space is the effective one, not the native one: the same
    // logical position stays valid while the viewport shrinks around it.
    ui.ui_settings()
        .set_zoom(UiZoom::new(2.0).unwrap())
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(ui.paint().unwrap().viewport().width(), 200.0);
    ui.pointer_input(PointerEvent::new(
        Point::new(120.0, 80.0).unwrap(),
        PointerKind::Move,
    ))
    .unwrap();
    let (position, _, _) = seen.get().unwrap();
    assert_eq!(position, [120.0, 80.0]);
}

#[test]
fn command_order_is_preserved_so_later_drawing_covers_earlier_drawing() {
    struct Stack;
    impl Component for Stack {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
            p.fill_rect(rect(0.0, 0.0, 10.0, 10.0), Color::BACKGROUND);
            p.fill_rect(rect(0.0, 0.0, 10.0, 10.0), Color::ACCENT);
            Ok(())
        }
    }
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(50.0, 50.0, 1.0).unwrap())
        .unwrap();
    ui.mount_tree(Element::new("stack", Stack)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let scene = ui.paint().unwrap();
    let colors: Vec<_> = scene
        .commands()
        .iter()
        .filter_map(|command| match command {
            DrawCommand::FillRect { color, .. } => Some(*color),
            _ => None,
        })
        .collect();
    // Overlapping geometry is resolved by order alone; there is no z index.
    assert_eq!(colors, vec![Color::BACKGROUND, Color::ACCENT]);
}

#[test]
fn coverage_is_rasterized_at_the_effective_scale_and_follows_a_scale_change() {
    struct Rounded;
    impl Component for Rounded {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
            p.fill_shape(
                rect(0.0, 0.0, 20.0, 20.0),
                Shape::Rounded(Length::new(4.0)?),
                Color::ACCENT,
            )
        }
    }
    let pixels = |scale: f64| {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(50.0, 50.0, scale).unwrap())
            .unwrap();
        ui.mount_tree(Element::new("rounded", Rounded)).unwrap();
        ui.flush(Duration::ZERO).unwrap();
        let scene = ui.paint().unwrap();
        let image = &scene.masks()[0];
        let destination = masks(&scene)[0].0;
        ((image.width(), image.height()), destination)
    };
    // A rounded fill spends coverage only on its corners, so the destination here
    // is a corner and not the whole rectangle. What matters for the contract is
    // that the destination is logical and scale-independent, while the coverage
    // behind it is rasterized at physical resolution, so a scale change
    // re-rasterizes instead of stretching the previous pixels.
    let (small, small_rect) = pixels(1.0);
    let (large, large_rect) = pixels(2.0);
    assert_eq!(small_rect, large_rect);
    assert_eq!(large.0, small.0 * 2);
    assert_eq!(large.1, small.1 * 2);
}

#[test]
fn to_local_is_the_conversion_the_contract_describes() {
    let seen = std::rc::Rc::new(std::cell::Cell::new(None));
    let probe = Reporter(seen.clone());
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 400.0, 1.0).unwrap())
        .unwrap();
    // A second child, so the reporter is allocated at a nonzero origin.
    ui.mount_tree(Element::new(
        "column",
        Column::new([
            Element::new("spacer", Probe::new(rect(0.0, 0.0, 1.0, 1.0))),
            Element::new("reporter", probe),
        ]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.paint().unwrap();
    ui.pointer_input(PointerEvent::new(
        Point::new(30.0, 60.0).unwrap(),
        PointerKind::Move,
    ))
    .unwrap();
    let (position, bounds, local) = seen.get().expect("the probe must receive the move");
    assert!(bounds[1] > 0.0, "the reporter must sit below the spacer");
    // `to_local` must equal the hand-written subtraction the contract documents,
    // which is the whole reason built-in widgets may stop writing it out.
    assert_eq!(
        local,
        [position[0] - bounds[0], position[1] - bounds[1]],
        "to_local must subtract exactly the component's origin"
    );
    assert_eq!(local[1], 60.0 - bounds[1]);
    assert_ne!(local[1], position[1], "the origin must actually be removed");
}

#[test]
fn rect_primitives_agree_with_the_validated_constructor_and_tile_a_surface() {
    let size = Size::new(40.0, 20.0).unwrap();
    // Validated inputs cannot produce invalid geometry, so neither helper is
    // fallible, and both must land exactly where `Rect::new` would.
    assert_eq!(Rect::from_size(size), rect(0.0, 0.0, 40.0, 20.0));
    assert_eq!(
        Rect::from_origin_size(Point::new(-10.0, 5.0).unwrap(), size),
        rect(-10.0, 5.0, 40.0, 20.0)
    );
    // Containment is half-open, so adjacent bands neither overlap nor leave a gap:
    // every position along the shared boundary belongs to exactly one of them.
    let left = rect(0.0, 0.0, 10.0, 10.0);
    let right = rect(10.0, 0.0, 10.0, 10.0);
    for x in [0.0, 5.0, 9.999, 10.0, 15.0, 19.999] {
        let point = Point::new(x, 5.0).unwrap();
        assert_eq!(
            u8::from(left.contains(point)) + u8::from(right.contains(point)),
            1,
            "{x} must belong to exactly one band"
        );
    }
    assert!(!left.contains(Point::new(-0.001, 5.0).unwrap()));
    assert!(!left.contains(Point::new(5.0, 10.0).unwrap()));
    // A zero-extent rectangle contains nothing, not even its own origin.
    assert!(!rect(4.0, 4.0, 0.0, 8.0).contains(Point::new(4.0, 6.0).unwrap()));
}

#[test]
fn a_local_hit_test_is_to_local_followed_by_contains() {
    let seen = std::rc::Rc::new(std::cell::Cell::new(None));
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 400.0, 1.0).unwrap())
        .unwrap();
    ui.mount_tree(Element::new(
        "column",
        Column::new([
            Element::new("spacer", Probe::new(rect(0.0, 0.0, 1.0, 1.0))),
            Element::new("reporter", Reporter(seen.clone())),
        ]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.pointer_input(PointerEvent::new(
        Point::new(30.0, 60.0).unwrap(),
        PointerKind::Move,
    ))
    .unwrap();
    let (position, bounds, local) = seen.get().expect("the probe must receive the move");
    let [x, y, width, height] = bounds;
    assert!(y > 0.0, "the reporter must sit below the spacer");
    // A widget owns rectangles in its own space, so it tests the converted point
    // against them. That must answer exactly as the window-space rectangle does.
    let window = rect(x, y, width, height);
    let allocation = Rect::from_size(Size::new(width, height).unwrap());
    assert!(window.contains(Point::new(position[0], position[1]).unwrap()));
    assert!(allocation.contains(Point::new(local[0], local[1]).unwrap()));
    // A position past the trailing edge misses in both spaces alike.
    let outside = Point::new(x + width, y + height).unwrap();
    assert!(!window.contains(outside));
    assert!(!allocation.contains(Point::new(width, height).unwrap()));
}

#[test]
fn a_viewport_converts_physical_host_input_using_its_effective_scale() {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 400.0, 2.0).unwrap())
        .unwrap();
    // A host divides by the viewport it reads back, never by the native scale it
    // supplied, because zoom multiplies into that scale.
    let native = ui.viewport();
    assert_eq!(native.to_logical(240.0, 160.0).unwrap().x(), 120.0);
    ui.ui_settings()
        .set_zoom(UiZoom::new(1.25).unwrap())
        .unwrap();
    let effective = ui.viewport();
    assert_eq!(effective.scale(), 2.5);
    let converted = effective.to_logical(240.0, 160.0).unwrap();
    assert_eq!([converted.x(), converted.y()], [96.0, 64.0]);
    // Using the unzoomed scale would have kept the earlier answer and been wrong.
    assert_ne!(converted.x(), native.to_logical(240.0, 160.0).unwrap().x());
    assert!(effective.to_logical(f64::NAN, 0.0).is_err());
}
