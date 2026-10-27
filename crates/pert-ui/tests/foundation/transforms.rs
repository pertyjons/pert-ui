//! The transform contract, through the facade.
//!
//! These pin the rules in [transforms](../../../docs/rendering-platform.md): affine maps
//! of path geometry, quarter turns of masks and text, and the uniform view
//! transform a container gives its children. Like the coordinate contract they
//! extend, they are written against observable `DisplayList` output and public
//! queries rather than internal geometry.
use pert_ui::testing::HostIntegration;
use pert_ui::*;
use std::{cell::Cell, rc::Rc, time::Duration};

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::new(x, y, w, h).unwrap()
}
fn point(x: f32, y: f32) -> Point {
    Point::new(x, y).unwrap()
}
fn view(scale: f32, x: f32, y: f32) -> ViewTransform {
    ViewTransform::new(ViewScale::new(scale).unwrap(), point(x, y))
}
fn ui(width: f32, height: f32) -> Runtime {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(width, height, 1.0).unwrap())
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui
}

/// A container presenting its children through a bound view transform, placing
/// each at the origin with its measured size.
struct Canvas {
    view: ReadSignal<ViewTransform>,
    current: ViewTransform,
    children: Vec<Element>,
    /// Content extent given to every child instead of its measured size.
    content: Option<Size>,
}
impl Canvas {
    fn new(view: ReadSignal<ViewTransform>, children: Vec<Element>) -> Self {
        Self {
            current: ViewTransform::IDENTITY,
            view,
            children,
            content: None,
        }
    }
    fn content(mut self, size: Size) -> Self {
        self.content = Some(size);
        self
    }
}
impl Component for Canvas {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.current = cx.read(&self.view)?;
        Ok(())
    }
    fn child_transform(&self) -> ViewTransform {
        self.current
    }
    fn measure(
        &self,
        _: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        Ok(Size::new(
            available.width().unwrap_or(0.0),
            available.height().unwrap_or(0.0),
        )?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        for index in 0..cx.len() {
            let size = match self.content {
                Some(size) => size,
                None => cx.measure(index, Constraints::default())?,
            };
            cx.place(index, Rect::from_size(size))?;
        }
        Ok(())
    }
}

/// Reserves a fixed size, fills it, and records what it was told.
#[derive(Default)]
struct Log {
    updates: Cell<usize>,
    paints: Cell<usize>,
    local: Cell<Option<Point>>,
    scroll: Cell<Option<Point>>,
    raster: Cell<f64>,
    view_scale: Cell<f32>,
    size: Cell<Option<Size>>,
}
struct Block {
    size: Size,
    inset: Rect,
    log: Rc<Log>,
}
impl Block {
    fn new(width: f32, height: f32, log: &Rc<Log>) -> Self {
        Self {
            size: Size::new(width, height).unwrap(),
            inset: rect(0.0, 0.0, width, height),
            log: log.clone(),
        }
    }
    fn inset(mut self, inset: Rect) -> Self {
        self.inset = inset;
        self
    }
}
impl Component for Block {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.log.updates.set(self.log.updates.get() + 1);
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(self.size)
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        self.log.paints.set(self.log.paints.get() + 1);
        self.log.raster.set(p.viewport().scale());
        self.log.view_scale.set(p.view_scale());
        self.log.size.set(Some(p.size()));
        p.fill_rect(self.inset, Color::ACCENT);
        Ok(())
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if event.kind() != PointerKind::Leave {
            self.log.local.set(cx.to_local(event.position()));
        }
        if let PointerKind::Scroll(delta) = event.kind() {
            self.log.scroll.set(Some(delta));
        }
        Ok(InputOutcome::Handled)
    }
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

#[test]
fn a_transformed_path_is_the_path_built_at_the_mapped_points() {
    let bar = Path::polygon([
        point(0.0, 0.0),
        point(10.0, 0.0),
        point(10.0, 4.0),
        point(0.0, 4.0),
    ])
    .unwrap();
    let moved = Transform2D::scale(2.0, 0.5)
        .unwrap()
        .then(Transform2D::translation(5.0, 7.0).unwrap())
        .unwrap();
    let direct = Path::polygon([
        point(5.0, 7.0),
        point(25.0, 7.0),
        point(25.0, 9.0),
        point(5.0, 9.0),
    ])
    .unwrap();
    assert_eq!(bar.transformed(&moved).unwrap(), direct);

    // Curves keep their kind: an ellipse stays four cubics through mapped points.
    let circle = Path::circle(point(0.0, 0.0), Length::new(8.0).unwrap()).unwrap();
    let turned = circle
        .transformed(&Transform2D::rotation_degrees(37.0).unwrap())
        .unwrap();
    assert_eq!(turned.elements().len(), circle.elements().len());
    assert!(
        turned
            .elements()
            .iter()
            .zip(circle.elements())
            .all(|(a, b)| std::mem::discriminant(a) == std::mem::discriminant(b))
    );
}

#[test]
fn a_transformed_path_rejects_coordinates_it_cannot_represent() {
    let far = Path::polyline([point(1.0e30, 0.0), point(2.0e30, 0.0)]).unwrap();
    let huge = Transform2D::scale(1.0e30, 1.0).unwrap();
    assert_eq!(far.transformed(&huge), Err(PathError::InvalidParameter));
}

#[test]
fn transforming_maps_the_outline_and_leaves_the_stroke_width_alone() {
    struct Trace(Path);
    impl Component for Trace {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
            p.stroke_path(&self.0, StrokeStyle::new(Length::new(2.0)?), Color::ACCENT)
        }
    }
    let line = Path::polyline([point(0.0, 10.0), point(10.0, 10.0)]).unwrap();
    // Stretched tenfold along x only, as a plot maps data to its width.
    let stretched = line
        .transformed(&Transform2D::scale(10.0, 1.0).unwrap())
        .unwrap();
    let mut ui = ui(200.0, 40.0);
    ui.mount_tree(Element::new("trace", Trace(stretched)))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let scene = ui.paint().unwrap().rasterized().unwrap();
    let DrawCommand::DrawMask { rect, .. } = scene.commands()[0] else {
        panic!("a stroke lowers to one mask");
    };
    assert_eq!(rect.width(), 100.0, "the outline is ten times as long");
    assert_eq!(rect.height(), 2.0, "and still two pixels wide");
}

#[test]
fn a_quarter_turn_travels_with_the_command_and_shares_the_coverage() {
    struct Turned;
    impl Component for Turned {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
            let image = MaskImage::new(2, 1, vec![255, 0])?;
            p.draw_mask(&image, rect(0.0, 0.0, 20.0, 10.0), Color::ACCENT);
            p.draw_mask_turned(
                &image,
                rect(30.0, 0.0, 10.0, 20.0),
                Color::ACCENT,
                QuarterTurn::Right,
            );
            Ok(())
        }
    }
    let mut ui = ui(100.0, 40.0);
    ui.mount_tree(Element::new("turned", Turned)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let scene = ui.paint().unwrap();
    assert_eq!(scene.masks().len(), 1, "a turn samples the same coverage");
    let turns: Vec<_> = scene
        .commands()
        .iter()
        .filter_map(|command| match command {
            DrawCommand::DrawMask { turn, mask, .. } => Some((*turn, *mask)),
            _ => None,
        })
        .collect();
    assert_eq!(turns[0].0, QuarterTurn::None);
    assert_eq!(turns[1].0, QuarterTurn::Right);
    assert_eq!(turns[0].1, turns[1].1);
}

#[test]
fn quarter_turn_placement_tiles_the_turned_picture() {
    let upright = Size::new(100.0, 20.0).unwrap();
    let left_half = rect(0.0, 0.0, 50.0, 20.0);
    let right_half = rect(50.0, 0.0, 50.0, 20.0);
    // Turned right, the line reads downwards: its start is at the top.
    assert_eq!(
        QuarterTurn::Right
            .place(upright, left_half)
            .unwrap()
            .values(),
        [0.0, 0.0, 20.0, 50.0]
    );
    assert_eq!(
        QuarterTurn::Right
            .place(upright, right_half)
            .unwrap()
            .values(),
        [0.0, 50.0, 20.0, 50.0]
    );
    // Upside down, the halves exchange sides.
    assert_eq!(
        QuarterTurn::Half
            .place(upright, left_half)
            .unwrap()
            .values(),
        [50.0, 0.0, 50.0, 20.0]
    );
    assert_eq!(
        QuarterTurn::None.place(upright, left_half).unwrap(),
        left_half
    );
    assert_eq!(
        QuarterTurn::Left.turned(upright),
        Size::new(20.0, 100.0).unwrap()
    );
}

#[test]
fn a_turned_label_exchanges_its_extents_and_keeps_its_coverage() {
    let mut runtime = ui(400.0, 400.0);
    let root = runtime
        .mount_tree(Element::new(
            "row",
            Row::new([
                Element::new("upright", Label::new("Amplitude")),
                Element::new("turned", Label::new("Amplitude").turned(QuarterTurn::Left)),
            ]),
        ))
        .unwrap();
    runtime.flush(Duration::ZERO).unwrap();
    let children = runtime.children(&root).unwrap();
    let upright = runtime.bounds(&children[0]).unwrap().unwrap();
    let turned = runtime.bounds(&children[1]).unwrap().unwrap();
    assert!(upright.width() > upright.height());
    assert_eq!(turned.width(), upright.height());
    assert_eq!(turned.height(), upright.width());
    let scene = runtime.paint().unwrap();
    assert_eq!(
        scene.masks().len(),
        1,
        "the turned label draws the upright label's glyph coverage"
    );
}

#[test]
fn children_are_shown_through_the_view_transform() {
    let mut ui = ui(400.0, 300.0);
    let log = Rc::new(Log::default());
    let transform = ui.signal(view(2.0, 30.0, 10.0));
    let root = ui
        .mount_tree(Element::new(
            "canvas",
            Canvas::new(
                transform.read_only(),
                vec![Element::new(
                    "block",
                    Block::new(50.0, 40.0, &log).inset(rect(5.0, 5.0, 10.0, 10.0)),
                )],
            ),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let block = ui.children(&root).unwrap()[0].clone();
    // offset + scale * (child_origin + p), with the child placed at the origin.
    assert_eq!(
        ui.bounds(&block).unwrap().unwrap().values(),
        [30.0, 10.0, 100.0, 80.0]
    );
    let scene = ui.paint().unwrap();
    assert_eq!(fills(&scene), [[40.0, 20.0, 20.0, 20.0]]);
    // The child still paints in its own 50 × 40 pixels and only learns the
    // scale at which its coverage is prepared.
    assert_eq!(log.size.get(), Some(Size::new(50.0, 40.0).unwrap()));
    assert_eq!(log.view_scale.get(), 2.0);
    assert_eq!(log.raster.get(), 2.0);
}

#[test]
fn view_transforms_nest_by_multiplication() {
    let mut ui = ui(800.0, 600.0);
    let log = Rc::new(Log::default());
    let outer = ui.signal(view(2.0, 10.0, 0.0));
    let inner = ui.signal(view(1.5, 4.0, 0.0));
    let root = ui
        .mount_tree(Element::new(
            "outer",
            Canvas::new(
                outer.read_only(),
                vec![Element::new(
                    "inner",
                    Canvas::new(
                        inner.read_only(),
                        vec![Element::new("block", Block::new(10.0, 10.0, &log))],
                    ),
                )],
            )
            .content(Size::from([200, 200])),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let inner_id = ui.children(&root).unwrap()[0].clone();
    let block = ui.children(&inner_id).unwrap()[0].clone();
    // The inner offset is in the inner canvas's local pixels, which the outer
    // view doubles: 10 + 2 * 4 = 18. The block is shown at 2 * 1.5 = 3.
    assert_eq!(
        ui.bounds(&block).unwrap().unwrap().values(),
        [18.0, 0.0, 30.0, 30.0]
    );
    ui.paint().unwrap();
    assert_eq!(log.view_scale.get(), 3.0);
}

#[test]
fn a_clip_is_mapped_with_the_content_it_clips() {
    struct Clipped;
    impl Component for Clipped {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
            Ok(Size::new(40.0, 40.0)?)
        }
        fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
            let image = MaskImage::new(2, 2, vec![255; 4])?;
            p.with_clip(rect(10.0, 10.0, 5.0, 5.0), |p| {
                p.draw_mask(&image, rect(0.0, 0.0, 40.0, 40.0), Color::ACCENT);
            });
            Ok(())
        }
    }
    let mut ui = ui(400.0, 300.0);
    let transform = ui.signal(view(4.0, 100.0, 0.0));
    ui.mount_tree(Element::new(
        "canvas",
        Canvas::new(
            transform.read_only(),
            vec![Element::new("clipped", Clipped)],
        ),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let scene = ui.paint().unwrap();
    let DrawCommand::DrawMask {
        rect: shown, clip, ..
    } = scene.commands()[0]
    else {
        panic!("one mask");
    };
    assert_eq!(shown.values(), [100.0, 0.0, 160.0, 160.0]);
    assert_eq!(clip.unwrap().values(), [140.0, 40.0, 20.0, 20.0]);
}

#[test]
fn content_outside_the_container_is_clipped_like_any_child() {
    let mut ui = ui(100.0, 100.0);
    let log = Rc::new(Log::default());
    // Magnified until only its top-left quarter fits the window.
    let transform = ui.signal(view(4.0, 0.0, 0.0));
    ui.mount_tree(Element::new(
        "canvas",
        Canvas::new(
            transform.read_only(),
            vec![Element::new("block", Block::new(50.0, 50.0, &log))],
        ),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(fills(&ui.paint().unwrap()), [[0.0, 0.0, 100.0, 100.0]]);
}

#[test]
fn vector_geometry_carries_its_scale_to_the_backend() {
    struct Disc;
    impl Component for Disc {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
            Ok(Size::new(20.0, 20.0)?)
        }
        fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
            p.fill_ellipse(rect(0.0, 0.0, 20.0, 20.0), Color::ACCENT)
        }
    }
    let mut ui = ui(400.0, 300.0);
    let transform = ui.signal(view(3.0, 50.0, 60.0));
    ui.mount_tree(Element::new(
        "canvas",
        Canvas::new(transform.read_only(), vec![Element::new("disc", Disc)]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let scene = ui.paint().unwrap();
    let DrawCommand::DrawVector { origin, scale, .. } = scene.commands()[0] else {
        panic!("one vector");
    };
    assert_eq!((origin.x(), origin.y(), scale), (50.0, 60.0, 3.0));
    // The fallback prepares it at the displayed size: sixty pixels across.
    let lowered = scene.rasterized().unwrap();
    let DrawCommand::DrawMask {
        rect: shown, mask, ..
    } = lowered.commands()[0]
    else {
        panic!("lowered to a mask");
    };
    assert_eq!(shown.values(), [50.0, 60.0, 60.0, 60.0]);
    assert_eq!(lowered.mask(mask).unwrap().width(), 60);
}

#[test]
fn pointer_positions_and_scroll_distances_reach_local_space() {
    let mut ui = ui(400.0, 300.0);
    let log = Rc::new(Log::default());
    let transform = ui.signal(view(2.0, 30.0, 10.0));
    ui.mount_tree(Element::new(
        "canvas",
        Canvas::new(
            transform.read_only(),
            vec![Element::new("block", Block::new(50.0, 40.0, &log))],
        ),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.pointer_input(PointerEvent::new(point(50.0, 30.0), PointerKind::Move))
        .unwrap();
    assert_eq!(log.local.get(), Some(point(10.0, 10.0)));
    // The block's last shown pixel is local (49.5, 39.5); one further misses it.
    log.local.set(None);
    ui.pointer_input(PointerEvent::new(point(130.0, 30.0), PointerKind::Move))
        .unwrap();
    assert_eq!(log.local.get(), None, "outside the scaled allocation");
    ui.pointer_input(PointerEvent::new(
        point(50.0, 30.0),
        PointerKind::Scroll(point(0.0, 12.0)),
    ))
    .unwrap();
    assert_eq!(
        log.scroll.get(),
        Some(point(0.0, 6.0)),
        "twelve window pixels are six content pixels at 200%"
    );
}

#[test]
fn a_pan_moves_retained_drawing_and_a_zoom_prepares_the_subtree_again() {
    let mut ui = ui(400.0, 300.0);
    let log = Rc::new(Log::default());
    let transform = ui.signal(view(1.0, 0.0, 0.0));
    ui.mount_tree(Element::new(
        "canvas",
        Canvas::new(
            transform.read_only(),
            vec![Element::new("block", Block::new(50.0, 40.0, &log))],
        ),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.paint().unwrap();
    let (updates, paints) = (log.updates.get(), log.paints.get());

    transform.set(view(1.0, 25.0, 5.0)).unwrap();
    let report = ui.flush(Duration::ZERO).unwrap();
    assert_eq!(report.components_updated, 1, "only the container updates");
    assert_eq!(fills(&ui.paint().unwrap()), [[25.0, 5.0, 50.0, 40.0]]);
    assert_eq!(
        (log.updates.get(), log.paints.get()),
        (updates, paints),
        "a pan neither updates nor repaints the content"
    );

    transform.set(view(2.0, 25.0, 5.0)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(fills(&ui.paint().unwrap()), [[25.0, 5.0, 100.0, 80.0]]);
    assert_eq!(log.updates.get(), updates + 1, "a zoom prepares it again");
    assert_eq!(log.paints.get(), paints + 1);
    assert_eq!(ui.next_deadline(), None, "and leaves nothing scheduled");
}

#[test]
fn a_continuous_zoom_stretches_prepared_drawing_until_the_scale_rests() {
    let mut ui = ui(800.0, 600.0);
    let log = Rc::new(Log::default());
    let transform = ui.signal(view(1.0, 0.0, 0.0));
    ui.mount_tree(Element::new(
        "canvas",
        Canvas::new(
            transform.read_only(),
            // The block at the origin, the label below it.
            vec![Element::new(
                "content",
                Column::new([
                    Element::new("block", Block::new(50.0, 40.0, &log)),
                    Element::new("label", Label::new("Sharp")),
                ]),
            )],
        ),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let issues = ui.validation_report().issues;
    assert!(issues.is_empty(), "{issues:?}");
    let coverage = |scene: &DisplayList| {
        scene
            .commands()
            .iter()
            .find_map(|command| match *command {
                DrawCommand::DrawMask { rect, mask, .. } => {
                    Some((rect.width(), scene.mask(mask).unwrap().width()))
                }
                _ => None,
            })
            .expect("the label's mask")
    };
    let (_, plain) = coverage(&ui.paint().unwrap());

    // A single change is prepared at once.
    let frame = Duration::from_millis(16);
    transform.set(view(1.5, 0.0, 0.0)).unwrap();
    ui.flush(frame).unwrap();
    let (_, single) = coverage(&ui.paint().unwrap());
    assert!(single > plain, "prepared for 150%");
    assert_eq!(log.view_scale.get(), 1.5);
    let updates = log.updates.get();

    // A change a frame later continues a zoom: shown larger, drawn from the
    // coverage prepared at 150%, and nothing under the view updates.
    transform.set(view(2.0, 0.0, 0.0)).unwrap();
    ui.flush(frame * 2).unwrap();
    let (shown, stretched) = coverage(&ui.paint().unwrap());
    assert_eq!(stretched, single, "the same coverage, stretched");
    assert!(shown > stretched as f32);
    assert_eq!(fills(&ui.paint().unwrap()), [[0.0, 0.0, 100.0, 80.0]]);
    assert_eq!(log.updates.get(), updates);
    let settle = ui.next_deadline().expect("the rest is scheduled");
    assert!(settle > frame * 2);
    assert_eq!(
        ui.application_deadline(),
        Some(settle),
        "a custom host sees it"
    );
    assert_eq!(
        ui.flush_windows(frame * 2).unwrap().next_deadline,
        Some(settle)
    );

    // Once the scale has rested, it is prepared at its displayed size.
    ui.flush(settle).unwrap();
    let (shown, sharp) = coverage(&ui.paint().unwrap());
    assert!(sharp > single);
    assert!((shown - sharp as f32).abs() <= 1.0);
    assert_eq!(log.updates.get(), updates + 1);
    assert_eq!(log.view_scale.get(), 2.0);
    assert_eq!(ui.next_deadline(), None, "and nothing stays scheduled");
}

#[test]
fn a_component_removed_during_a_zoom_leaves_nothing_to_settle() {
    let mut ui = ui(800.0, 600.0);
    let log = Rc::new(Log::default());
    let transform = ui.signal(view(1.0, 0.0, 0.0));
    let root = ui
        .mount_tree(Element::new(
            "canvas",
            Canvas::new(
                transform.read_only(),
                vec![Element::new("block", Block::new(50.0, 40.0, &log))],
            ),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let frame = Duration::from_millis(16);
    transform.set(view(1.5, 0.0, 0.0)).unwrap();
    ui.flush(frame).unwrap();
    transform.set(view(2.0, 0.0, 0.0)).unwrap();
    ui.flush(frame * 2).unwrap();
    let settle = ui.next_deadline().expect("stretched");
    ui.unmount(&root).unwrap();
    assert_eq!(
        ui.next_deadline(),
        None,
        "no wake is left for removed content"
    );
    assert_eq!(ui.application_deadline(), None);
    ui.flush(settle).unwrap();
    assert_eq!(ui.next_deadline(), None);
}

#[test]
fn shutdown_during_a_zoom_leaves_nothing_scheduled() {
    let mut ui = ui(800.0, 600.0);
    let log = Rc::new(Log::default());
    let transform = ui.signal(view(1.0, 0.0, 0.0));
    ui.mount_tree(Element::new(
        "canvas",
        Canvas::new(
            transform.read_only(),
            vec![Element::new("block", Block::new(50.0, 40.0, &log))],
        ),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let frame = Duration::from_millis(16);
    transform.set(view(1.5, 0.0, 0.0)).unwrap();
    ui.flush(frame).unwrap();
    transform.set(view(2.0, 0.0, 0.0)).unwrap();
    ui.flush(frame * 2).unwrap();
    assert!(ui.application_deadline().is_some());
    ui.shutdown();
    assert_eq!(ui.application_deadline(), None);
}

#[test]
fn text_inside_a_magnified_view_is_prepared_at_its_displayed_size() {
    let mut ui = ui(800.0, 600.0);
    let transform = ui.signal(view(1.0, 0.0, 0.0));
    ui.mount_tree(Element::new(
        "canvas",
        Canvas::new(
            transform.read_only(),
            vec![Element::new("label", Label::new("Sharp"))],
        ),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let scene = ui.paint().unwrap();
    let plain = scene.masks()[0].width();

    transform.set(view(2.0, 0.0, 0.0)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let scene = ui.paint().unwrap();
    let DrawCommand::DrawMask {
        rect: shown, mask, ..
    } = scene.commands()[0]
    else {
        panic!("one text mask");
    };
    let coverage = scene.mask(mask).unwrap();
    assert!(coverage.width() > plain, "rasterized again, not stretched");
    assert!(
        (shown.width() - coverage.width() as f32).abs() <= 1.0,
        "one coverage pixel per window pixel: {} vs {}",
        shown.width(),
        coverage.width()
    );
}

#[test]
fn magnification_beyond_the_raster_ceiling_stretches_instead_of_failing() {
    let mut ui = ui(800.0, 600.0);
    let log = Rc::new(Log::default());
    let transform = ui.signal(view(ViewScale::MAX.get(), 0.0, 0.0));
    ui.mount_tree(Element::new(
        "canvas",
        Canvas::new(
            transform.read_only(),
            vec![Element::new(
                "content",
                Column::new([
                    Element::new("block", Block::new(4.0, 4.0, &log)),
                    Element::new("label", Label::new("Huge").style(TextStyle::Heading)),
                ]),
            )],
        ),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let issues = ui.validation_report().issues;
    assert!(issues.is_empty(), "{issues:?}");
    ui.paint().unwrap();
    assert_eq!(log.view_scale.get(), ViewScale::MAX.get());
    assert_eq!(log.raster.get(), 8.0, "coverage is prepared at the ceiling");
}

#[test]
fn a_surface_opened_inside_a_view_is_not_scaled() {
    let mut ui = ui(800.0, 600.0);
    let open = ui.signal(true);
    let transform = ui.signal(view(3.0, 0.0, 0.0));
    let root = ui
        .mount_tree(Element::new(
            "canvas",
            Canvas::new(
                transform.read_only(),
                vec![Element::new(
                    "popover",
                    Popover::new(
                        open,
                        Element::new("trigger", Button::new("Open", |_| Ok(()))),
                        Element::new("content", Label::new("Readable")),
                    ),
                )],
            ),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let popover = ui.children(&root).unwrap()[0].clone();
    let parts = ui.children(&popover).unwrap();
    let trigger = ui.bounds(&parts[0]).unwrap().unwrap();
    let content = ui.bounds(&parts[1]).unwrap().unwrap();

    assert!(trigger.height() >= 3.0 * 24.0, "the trigger is magnified");
    assert!(
        content.height() < trigger.height() / 2.0,
        "the surface keeps the window's scale: {content:?} vs {trigger:?}"
    );
    assert!(
        content.y() >= trigger.y() + trigger.height() - 0.5,
        "and is anchored below the trigger as displayed"
    );
}

#[test]
fn a_floating_window_follows_the_pointer_inside_a_magnified_layer() {
    let mut ui = ui(800.0, 600.0);
    let transform = ui.signal(view(2.0, 0.0, 0.0));
    let window = FloatingWindow::new(
        &ui,
        "tools",
        "Tools",
        Element::new("body", Label::new("Body")),
    )
    .position(point(10.0, 10.0))
    .unwrap()
    .initial_size(Size::from([150, 100]))
    .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "canvas",
            Canvas::new(
                transform.read_only(),
                vec![Element::new("layer", WindowLayer::new([window]))],
            )
            .content(Size::from([300, 200])),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let layer = ui.children(&root).unwrap()[0].clone();
    let frame = ui.children(&layer).unwrap()[0].clone();
    assert_eq!(
        ui.bounds(&frame).unwrap().unwrap().values(),
        [20.0, 20.0, 300.0, 200.0],
        "placed at content (10, 10), shown at 200%"
    );
    // Press the title bar and drag forty window pixels right: twenty content
    // pixels, so the window is shown forty pixels further right.
    let mut pointer = |x: f32, kind: PointerKind| {
        ui.pointer_input(PointerEvent::new(point(x, 36.0), kind))
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
    };
    pointer(60.0, PointerKind::Press(PointerButton::Primary));
    pointer(100.0, PointerKind::Move);
    pointer(100.0, PointerKind::Release(PointerButton::Primary));
    assert_eq!(
        ui.bounds(&frame).unwrap().unwrap().values(),
        [60.0, 20.0, 300.0, 200.0]
    );
}

#[test]
fn vector_coverage_honors_the_raster_ceiling_too() {
    struct Dot;
    impl Component for Dot {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
            Ok(Size::new(4.0, 4.0)?)
        }
        fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
            p.fill_ellipse(rect(0.0, 0.0, 4.0, 4.0), Color::ACCENT)
        }
    }
    let mut ui = ui(400.0, 300.0);
    let transform = ui.signal(view(ViewScale::MAX.get(), 0.0, 0.0));
    ui.mount_tree(Element::new(
        "canvas",
        Canvas::new(transform.read_only(), vec![Element::new("dot", Dot)]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let lowered = ui.paint().unwrap().rasterized().unwrap();
    let DrawCommand::DrawMask {
        rect: shown, mask, ..
    } = lowered.commands()[0]
    else {
        panic!("lowered to a mask");
    };
    assert_eq!(
        shown.width(),
        4.0 * ViewScale::MAX.get(),
        "shown at the full factor"
    );
    assert_eq!(
        lowered.mask(mask).unwrap().width(),
        32,
        "prepared at eight pixels per unit and stretched"
    );
}

#[test]
fn the_public_mapping_agrees_with_placement_far_from_the_origin() {
    // In `f32` the offset and the product cancel to zero; the runtime places
    // the child in `f64`, and the public mapping must say the same.
    let far = view(1.1, -110_000_000.0, 0.0);
    let shown = far.apply(point(100_000_000.0, 0.0)).unwrap();
    assert!((shown.x() - 2.384_185_8).abs() < 1e-3, "{}", shown.x());
    let mut ui = ui(400.0, 300.0);
    let log = Rc::new(Log::default());
    let transform = ui.signal(far);
    struct Far(Vec<Element>, ReadSignal<ViewTransform>, ViewTransform);
    impl Component for Far {
        fn take_children(&mut self) -> Vec<Element> {
            std::mem::take(&mut self.0)
        }
        fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
            self.2 = cx.read(&self.1)?;
            Ok(())
        }
        fn child_transform(&self) -> ViewTransform {
            self.2
        }
        fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
            cx.place(0, rect(100_000_000.0, 0.0, 10.0, 10.0))
        }
    }
    let root = ui
        .mount_tree(Element::new(
            "far",
            Far(
                vec![Element::new("block", Block::new(10.0, 10.0, &log))],
                transform.read_only(),
                ViewTransform::IDENTITY,
            ),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let block = ui.children(&root).unwrap()[0].clone();
    assert_eq!(ui.bounds(&block).unwrap().unwrap().x(), shown.x());
}

#[test]
fn a_child_promoted_to_a_surface_leaves_the_view_transform() {
    struct Promoter {
        promote: ReadSignal<bool>,
        promoted: bool,
        children: Vec<Element>,
    }
    impl Component for Promoter {
        fn take_children(&mut self) -> Vec<Element> {
            std::mem::take(&mut self.children)
        }
        fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
            self.promoted = cx.read(&self.promote)?;
            Ok(())
        }
        fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
            Ok(Size::new(60.0, 20.0)?)
        }
        fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
            cx.place(0, rect(0.0, 0.0, 50.0, 40.0))
        }
        fn child_surface(&self, _: &str) -> Option<SurfaceKind> {
            self.promoted.then_some(SurfaceKind::Tooltip)
        }
    }
    let mut ui = ui(800.0, 600.0);
    let log = Rc::new(Log::default());
    let promote = ui.signal(false);
    let transform = ui.signal(view(2.0, 0.0, 0.0));
    let root = ui
        .mount_tree(Element::new(
            "canvas",
            Canvas::new(
                transform.read_only(),
                vec![Element::new(
                    "promoter",
                    Promoter {
                        promote: promote.read_only(),
                        promoted: false,
                        children: vec![Element::new("block", Block::new(50.0, 40.0, &log))],
                    },
                )],
            ),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let promoter = ui.children(&root).unwrap()[0].clone();
    let block = ui.children(&promoter).unwrap()[0].clone();
    assert_eq!(ui.bounds(&block).unwrap().unwrap().width(), 100.0);
    promote.set(true).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(
        ui.bounds(&block).unwrap().unwrap().width(),
        50.0,
        "a surface is shown at the window's own scale"
    );
    promote.set(false).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(ui.bounds(&block).unwrap().unwrap().width(), 100.0);
}

#[test]
fn view_scale_rejects_what_it_cannot_honor() {
    assert_eq!(ViewScale::new(0.0), Err(GeometryError::ViewScaleOutOfRange));
    assert_eq!(
        ViewScale::new(ViewScale::MAX.get() * 2.0),
        Err(GeometryError::ViewScaleOutOfRange)
    );
    assert_eq!(ViewScale::new(f32::INFINITY), Err(GeometryError::NonFinite));
    // The message names the range, so a reader need not look it up.
    assert_eq!(
        GeometryError::ViewScaleOutOfRange.to_string(),
        "view scale is outside 0.03125..=32"
    );
    assert_eq!(ViewScale::new(ViewScale::MIN.get()), Ok(ViewScale::MIN));
    assert_eq!(ViewScale::try_from(1.0), Ok(ViewScale::ONE));
}

#[test]
fn a_tab_drag_out_under_a_reducing_view_waits_for_the_window_threshold() {
    let mut ui = ui(400.0, 300.0);
    let scale = ui.signal(view(0.25, 0.0, 0.0));
    let root = ui
        .mount_tree(Element::new(
            "canvas",
            Canvas::new(
                scale.read_only(),
                vec![Element::new(
                    "tabs",
                    Tabs::new([
                        Tab::new("first", "First", Element::new("a", Label::new("A"))),
                        Tab::new("second", "Second", Element::new("b", Label::new("B"))),
                    ])
                    .detachable(true),
                )],
            )
            .content(Size::new(800.0, 600.0).unwrap()),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let tabs = ui.children(&root).unwrap()[0].clone();
    let body = ui.bounds(&ui.children(&tabs).unwrap()[0]).unwrap().unwrap();
    // The strip's bottom edge, in window pixels, a quarter of its local height.
    let bottom = body.y();
    let pointer = |ui: &mut Runtime, kind: PointerKind, y: f32| {
        ui.pointer_input(PointerEvent::new(point(2.0, y), kind))
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
    };
    pointer(
        &mut ui,
        PointerKind::Press(PointerButton::Primary),
        bottom - 0.25,
    );
    // Nine local pixels below the strip, beyond its margin, but only 2.5
    // window pixels of travel: still a click.
    pointer(&mut ui, PointerKind::Move, bottom + 2.25);
    assert!(!ui.drag_active());
    pointer(&mut ui, PointerKind::Move, bottom + 10.0);
    assert!(ui.drag_active());
}
