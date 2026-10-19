//! Public consumer contracts for retained composition and layout.
use pert_ui::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

#[derive(Default)]
struct Leaf {
    measurements: Rc<Cell<usize>>,
    live: bool,
}
impl Component for Leaf {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        if self.live {
            cx.animate()?;
        }
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        self.measurements.set(self.measurements.get() + 1);
        Ok(Size::new(40.0, 20.0)?)
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        painter.fill_rect(Rect::new(-10.0, -10.0, 200.0, 200.0)?, Color::ACCENT);
        Ok(())
    }
}
struct CustomRow;
impl Component for CustomRow {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        let mut width = 0.0;
        let mut height: f32 = 0.0;
        for i in 0..cx.len() {
            let size = cx.measure(i, available)?;
            width += size.width();
            height = height.max(size.height());
        }
        Ok(Size::new(width, height)?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        let mut x = 0.0;
        for i in 0..cx.len() {
            let child = cx.measure(i, Constraints::bounded(size))?;
            cx.place(i, Rect::new(x, 0.0, child.width(), child.height())?)?;
            x += child.width();
        }
        Ok(())
    }
}
fn leaf(key: &str) -> Element {
    Element::new(key, Leaf::default())
}
fn runtime() -> Runtime {
    let runtime = Runtime::new();
    runtime
        .set_viewport(Viewport::new(60.0, 30.0, 1.0).unwrap())
        .unwrap();
    runtime
}

#[test]
fn public_custom_container_places_and_clips_without_backend_imports() {
    let mut ui = runtime();
    let root = ui
        .mount_tree(Element::new("root", CustomRow).children([leaf("a"), leaf("b")]))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let children = ui.children(&root).unwrap();
    assert_eq!(
        ui.bounds(&children[1]).unwrap().unwrap().values(),
        [40.0, 0.0, 40.0, 20.0]
    );
    let scene = ui.paint().unwrap();
    assert_eq!(
        scene.commands(),
        &[
            DrawCommand::FillRect {
                rect: Rect::new(0.0, 0.0, 40.0, 20.0).unwrap(),
                color: Color::ACCENT
            },
            DrawCommand::FillRect {
                rect: Rect::new(40.0, 0.0, 20.0, 20.0).unwrap(),
                color: Color::ACCENT
            },
        ]
    );
    let snapshot = ui.inspect();
    assert_eq!(
        snapshot.components[1].parent,
        Some(snapshot.components[0].id)
    );
}

/// Counts its own drawing callbacks and fills the allocation it measured.
struct Tile {
    paints: Rc<Cell<usize>>,
}
impl Component for Tile {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(40.0, 20.0)?)
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        self.paints.set(self.paints.get() + 1);
        painter.fill_rect(Rect::new(0.0, 0.0, 40.0, 20.0)?, Color::ACCENT);
        Ok(())
    }
}

#[test]
fn a_fully_clipped_component_composes_nothing_and_still_settles() {
    // Three tiles in a row that is sixty logical pixels wide: the third one is
    // entirely outside the viewport, so its clip has no area.
    let mut ui = runtime();
    let paints = [
        Rc::new(Cell::new(0)),
        Rc::new(Cell::new(0)),
        Rc::new(Cell::new(0)),
    ];
    let root = ui
        .mount_tree(
            Element::new("root", CustomRow).children(["a", "b", "c"].into_iter().zip(&paints).map(
                |(key, paints)| {
                    Element::new(
                        key,
                        Tile {
                            paints: paints.clone(),
                        },
                    )
                },
            )),
        )
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let children = ui.children(&root).unwrap();
    assert_eq!(
        ui.bounds(&children[2]).unwrap().unwrap().values(),
        [80.0, 0.0, 40.0, 20.0]
    );
    // Composition drops what the clip removes instead of encoding an empty
    // rectangle, so the scene is proportional to what is on screen.
    assert_eq!(
        ui.paint().unwrap().commands(),
        &[
            DrawCommand::FillRect {
                rect: Rect::new(0.0, 0.0, 40.0, 20.0).unwrap(),
                color: Color::ACCENT,
            },
            DrawCommand::FillRect {
                rect: Rect::new(40.0, 0.0, 20.0, 20.0).unwrap(),
                color: Color::ACCENT,
            },
        ]
    );
    assert_eq!(paints.each_ref().map(|count| count.get()), [1, 1, 0]);

    // Clipped drawing stays dirty without keeping presentation alive.
    ui.invalidate(&children[2], Invalidation::Paint).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(!ui.needs_paint());
    assert_eq!(ui.paint().unwrap().len(), 2);
    assert_eq!(paints[2].get(), 0);
    assert!(!ui.needs_paint());
    assert_eq!(ui.next_deadline(), None);

    // Leaving the component out of the scene is a composition decision, not a
    // removal: a viewport wide enough to expose it composes it again.
    ui.set_viewport(Viewport::new(200.0, 30.0, 1.0).unwrap())
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(ui.paint().unwrap().len(), 3);
}

#[test]
fn size_limits_constrain_is_the_clamp_a_container_applies_to_its_preference() {
    let limits = SizeLimits::new(
        Size::new(40.0, 20.0).unwrap(),
        Constraints::new(Some(80.0), None).unwrap(),
    )
    .unwrap();
    // A preferred size is a request; the hard limits decide, minimum first.
    let raised = limits.constrain(Size::new(10.0, 10.0).unwrap());
    assert_eq!((raised.width(), raised.height()), (40.0, 20.0));
    let capped = limits.constrain(Size::new(1000.0, 1000.0).unwrap());
    assert_eq!((capped.width(), capped.height()), (80.0, 1000.0));
    // A built-in container measures to exactly that, so a custom one reproduces
    // the standard behavior by calling the same public method.
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(1000.0, 1000.0, 1.0).unwrap())
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Container::default()
                .size_limits(limits)
                .preferred_size(Size::new(1000.0, 1000.0).unwrap()),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let [_, _, width, height] = ui.bounds(&root).unwrap().unwrap().values();
    assert_eq!((width, height), (capped.width(), capped.height()));
}

#[test]
fn reconcile_preserves_ordered_state_and_rejects_stale_or_ambiguous_keys() {
    let mut ui = runtime();
    let root = ui
        .mount_tree(Element::new("root", CustomRow).children([leaf("a"), leaf("b")]))
        .unwrap();
    let old = ui.children(&root).unwrap();
    ui.focus(&old[0]).unwrap();
    ui.reconcile_children(&root, vec![leaf("b"), leaf("a")])
        .unwrap();
    assert_eq!(
        ui.children(&root).unwrap(),
        vec![old[1].clone(), old[0].clone()]
    );
    assert_eq!(ui.focused(), Some(old[0].clone()));
    assert_eq!(
        ui.reconcile_children(&root, vec![leaf("a"), leaf("a")]),
        Err(RuntimeError::DuplicateKey)
    );
    assert_eq!(
        ui.reconcile_children(&root, vec![Element::new("a", CustomRow)]),
        Err(RuntimeError::ChildTypeChanged)
    );
    ui.reconcile_children(&root, vec![leaf("b")]).unwrap();
    ui.reconcile_children(&root, vec![leaf("a"), leaf("b")])
        .unwrap();
    assert_ne!(ui.children(&root).unwrap()[0], old[0]);
    assert_eq!(ui.event(&old[0], |_| ()), Err(RuntimeError::Unmounted));
    // The same child key under another parent is valid.
    ui.mount_tree(Element::new("other", CustomRow).children([leaf("a")]))
        .unwrap();
}

#[test]
fn hidden_ancestors_suspend_deadlines_and_preserve_local_visibility() {
    let mut ui = runtime();
    let root = ui
        .mount_tree(Element::new("root", CustomRow).children([
            Element::new(
                "live",
                Leaf {
                    live: true,
                    ..Default::default()
                },
            ),
            leaf("hidden"),
        ]))
        .unwrap();
    let children = ui.children(&root).unwrap();
    ui.set_visible(&children[1], false).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(ui.next_deadline().is_some());
    ui.set_visible(&root, false).unwrap();
    assert!(ui.next_deadline().is_none());
    assert_eq!(ui.event(&children[0], |_| ()), Err(RuntimeError::Hidden));
    ui.set_visible(&root, true).unwrap();
    ui.flush(Duration::from_secs(1)).unwrap();
    assert!(ui.next_deadline().is_some());
    assert_eq!(ui.event(&children[1], |_| ()), Err(RuntimeError::Hidden));
    ui.unmount(&root).unwrap();
    assert_eq!(ui.event(&children[0], |_| ()), Err(RuntimeError::Unmounted));
    assert!(ui.inspect().components.is_empty());
    assert!(ui.next_deadline().is_none());
}

#[test]
fn paint_changes_reuse_measurement_and_static_siblings() {
    let mut ui = runtime();
    let measurements = Rc::new(Cell::new(0));
    let root = ui
        .mount_tree(Element::new("root", CustomRow).children([
            Element::new(
                "a",
                Leaf {
                    measurements: measurements.clone(),
                    live: false,
                },
            ),
            leaf("b"),
        ]))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let before = measurements.get();
    ui.invalidate(&ui.children(&root).unwrap()[0], Invalidation::Paint)
        .unwrap();
    assert_eq!(ui.flush(Duration::ZERO).unwrap().components_updated, 1);
    assert_eq!(measurements.get(), before);
    ui.paint().unwrap();
    assert!(ui.next_deadline().is_none());
}

struct Lifecycle {
    name: &'static str,
    log: Rc<RefCell<Vec<String>>>,
    panic_mount: bool,
}
impl Component for Lifecycle {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn mounted(&mut self) {
        assert!(!self.panic_mount, "intentional mount failure");
        self.log.borrow_mut().push(format!("+{}", self.name));
    }
    fn unmounted(&mut self) {
        self.log.borrow_mut().push(format!("-{}", self.name));
    }
}
#[test]
fn mount_panic_rolls_back_new_subtree_and_reconciliation() {
    let mut ui = runtime();
    let root = ui
        .mount_tree(Element::new("root", CustomRow).children([leaf("kept")]))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let kept = ui.children(&root).unwrap();
    let log = Rc::new(RefCell::new(Vec::new()));
    let make = |name, panic_mount| {
        Element::new(
            name,
            Lifecycle {
                name,
                log: log.clone(),
                panic_mount,
            },
        )
    };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ui.reconcile_children(
            &root,
            vec![make("new", false).children([make("bad", true)])],
        )
        .unwrap();
    }));
    assert!(result.is_err());
    assert_eq!(ui.children(&root).unwrap(), kept);
    assert_eq!(&*log.borrow(), &["+new", "-new"]);
    ui.reconcile_children(
        &root,
        vec![make("new", false).children([make("bad", false)])],
    )
    .unwrap();
    ui.unmount(&root).unwrap();
    assert!(log.borrow().ends_with(&["-bad".into(), "-new".into()]));
}

#[test]
fn invalid_nested_keys_are_rejected_before_callbacks() {
    let mut ui = runtime();
    let result = ui.mount_tree(
        Element::new("root", CustomRow)
            .children([Element::new("nested", CustomRow).children([leaf("")])]),
    );
    assert_eq!(result.unwrap_err(), RuntimeError::EmptyKey);
    assert!(ui.inspect().components.is_empty());
    assert!(Size::new(f32::NAN, 1.0).is_err());
    assert!(Constraints::new(Some(f32::INFINITY), None).is_err());
}

#[test]
fn rollback_discovers_completed_descendants_before_unmounting_ancestors() {
    let mut ui = runtime();
    let log = Rc::new(RefCell::new(Vec::new()));
    let make = |name, panic_mount| {
        Element::new(
            name,
            Lifecycle {
                name,
                log: log.clone(),
                panic_mount,
            },
        )
    };
    let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ui.mount_tree(
            make("root", false).children([make("parent", false).children([make("failure", true)])]),
        )
        .unwrap();
    }));
    assert!(failed.is_err());
    assert_eq!(&*log.borrow(), &["+root", "+parent", "-parent", "-root"]);
    assert!(ui.inspect().components.is_empty());
    let root = ui.mount_tree(Element::new("root", CustomRow)).unwrap();
    log.borrow_mut().clear();
    let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ui.reconcile_children(
            &root,
            vec![
                make("parent", false).children([make("child", false)]),
                make("failure", true),
            ],
        )
        .unwrap();
    }));
    assert!(failed.is_err());
    assert_eq!(&*log.borrow(), &["+parent", "+child", "-child", "-parent"]);
    assert!(ui.children(&root).unwrap().is_empty());
}

struct FallibleLayout {
    fail: Rc<Cell<bool>>,
    signal: Signal<bool>,
}
impl Component for FallibleLayout {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn arrange(&self, _: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        assert_eq!(self.signal.set(true), Err(RuntimeError::WriteDuringLayout));
        if self.fail.get() {
            return Err(LayoutError::InvalidChild.into());
        }
        Ok(())
    }
}
struct Stretch;
impl Component for Stretch {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        let width = size.width() / cx.len().max(1) as f32;
        for index in 0..cx.len() {
            cx.place(
                index,
                Rect::new(index as f32 * width, 0.0, width, size.height())?,
            )?;
        }
        Ok(())
    }
}
#[test]
fn layout_failure_keeps_previous_geometry_and_phase_is_restored() {
    let mut ui = runtime();
    let fail = Rc::new(Cell::new(false));
    let signal = ui.signal(false);
    let root = ui
        .mount_tree(Element::new("root", Stretch).children([
            leaf("first"),
            Element::new(
                "last",
                FallibleLayout {
                    fail: fail.clone(),
                    signal: signal.clone(),
                },
            ),
        ]))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let children = ui.children(&root).unwrap();
    let before = ui.bounds(&children[0]).unwrap();
    let observed = ui.inspect();
    assert_eq!(
        observed.components[1].details.box_model.border_box,
        before.map(Rect::values)
    );
    assert!(ui.validation_report().issues.is_empty());
    fail.set(true);
    ui.set_viewport(Viewport::new(300.0, 100.0, 1.0).unwrap())
        .unwrap();
    assert_eq!(
        ui.flush(Duration::ZERO),
        Err(RuntimeError::Layout(LayoutError::InvalidChild))
    );
    assert_eq!(ui.bounds(&children[0]).unwrap(), before);
    assert_eq!(ui.paint(), Err(DrawError::PendingUpdates));
    assert!(ui.validation_report().pending || !cfg!(debug_assertions));
    signal.set(true).unwrap();
    fail.set(false);
    ui.flush(Duration::ZERO).unwrap();
    assert_ne!(ui.bounds(&children[0]).unwrap(), before);
    ui.paint().unwrap();
}

fn tile(key: &str, width: f32, height: f32) -> Element {
    Element::new(
        key,
        Container::default()
            .preferred_size(Size::new(width, height).unwrap())
            .background(Color::ACCENT),
    )
}
#[test]
fn wrap_exact_fit_oversize_and_resize_preserve_identity() {
    let mut ui = runtime();
    ui.set_viewport(Viewport::new(88.0, 200.0, 1.0).unwrap())
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "wrap",
            Wrap::new([
                tile("a", 40.0, 20.0),
                tile("b", 40.0, 30.0),
                tile("wide", 100.0, 10.0),
                tile("c", 40.0, 20.0),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let ids = ui.children(&root).unwrap();
    let boxes: Vec<_> = ids
        .iter()
        .map(|id| ui.bounds(id).unwrap().unwrap().values())
        .collect();
    assert_eq!(
        boxes,
        vec![
            [0.0, 5.0, 40.0, 20.0],
            [48.0, 0.0, 40.0, 30.0],
            [0.0, 38.0, 100.0, 10.0],
            [0.0, 56.0, 40.0, 20.0]
        ]
    );
    ui.set_viewport(Viewport::new(87.0, 200.0, 1.25).unwrap())
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(ui.children(&root).unwrap(), ids);
    assert_eq!(
        ui.bounds(&ids[1]).unwrap().unwrap().values(),
        [0.0, 28.0, 40.0, 30.0]
    );
    ui.set_visible(&ids[0], false).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(
        ui.bounds(&ids[1]).unwrap().unwrap().values(),
        [0.0, 0.0, 40.0, 30.0]
    );
}
#[test]
fn grid_tracks_share_columns_and_weighted_spare_space() {
    let mut ui = runtime();
    ui.set_viewport(Viewport::new(208.0, 100.0, 1.0).unwrap())
        .unwrap();
    let grid = Grid::new(
        [Track::CONTENT, Track::weighted(1.0).unwrap()],
        [
            tile("a", 40.0, 20.0),
            tile("b", 30.0, 30.0),
            tile("c", 60.0, 10.0),
            tile("d", 40.0, 10.0),
        ],
    )
    .unwrap();
    let root = ui.mount_tree(Element::new("grid", grid)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let ids = ui.children(&root).unwrap();
    assert_eq!(
        ui.bounds(&ids[0]).unwrap().unwrap().values(),
        [0.0, 0.0, 60.0, 30.0]
    );
    assert_eq!(
        ui.bounds(&ids[1]).unwrap().unwrap().values(),
        [68.0, 0.0, 140.0, 30.0]
    );
    assert_eq!(
        ui.bounds(&ids[3]).unwrap().unwrap().values(),
        [68.0, 38.0, 140.0, 10.0]
    );
    assert!(ui.validation_report().issues.is_empty());
    assert!(Grid::new([], []).is_err());
    assert!(Track::weighted(0.0).is_err());
    assert!(Length::new(f32::INFINITY).is_err());
}

#[test]
fn a_content_column_counts_toward_the_grid_minimum() {
    let mut ui = runtime();
    // Narrower than the content column's intrinsic width.
    ui.set_viewport(Viewport::new(50.0, 100.0, 1.0).unwrap())
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([Element::new(
                "grid",
                Grid::new(
                    [Track::CONTENT, Track::weighted(1.0).unwrap()],
                    [tile("wide", 60.0, 20.0), tile("rest", 10.0, 20.0)],
                )
                .unwrap(),
            )]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let grid = ui.children(&root).unwrap()[0].clone();
    let wide = ui.children(&grid).unwrap()[0].clone();
    // The grid keeps the content column and the gap instead of letting the
    // column overflow a narrower allocation.
    let [_, _, width, _] = ui.bounds(&grid).unwrap().unwrap().values();
    assert_eq!(width, 68.0);
    let [x, _, content, _] = ui.bounds(&wide).unwrap().unwrap().values();
    assert_eq!((x, content), (0.0, 60.0));
}

#[test]
fn composed_wrapping_uses_viewport_not_unbounded_preference() {
    for track in [
        Track::fixed(Length::new(80.0).unwrap()),
        Track::weighted(1.0).unwrap(),
    ] {
        let mut ui = runtime();
        ui.set_viewport(Viewport::new(80.0, 200.0, 1.0).unwrap())
            .unwrap();
        let root = ui
            .mount_tree(Element::new(
                "grid",
                Grid::new(
                    [track],
                    [Element::new(
                        "flow",
                        Wrap::new([
                            tile("a", 40.0, 20.0),
                            tile("b", 40.0, 20.0),
                            tile("c", 40.0, 20.0),
                        ]),
                    )],
                )
                .unwrap(),
            ))
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        let flow = ui.children(&root).unwrap()[0].clone();
        let children = ui.children(&flow).unwrap();
        assert_eq!(
            ui.bounds(&children[2]).unwrap().unwrap().values(),
            [0.0, 56.0, 40.0, 20.0]
        );
    }
    let mut ui = runtime();
    ui.set_viewport(Viewport::new(87.0, 200.0, 1.0).unwrap())
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "container",
            Container::new(Element::new(
                "flow",
                Wrap::new([
                    tile("a", 40.0, 20.0),
                    tile("b", 40.0, 20.0),
                    tile("wide", 100.0, 20.0),
                ]),
            )),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let flow = ui.children(&root).unwrap()[0].clone();
    let second = ui.children(&flow).unwrap()[1].clone();
    assert_eq!(
        ui.bounds(&second).unwrap().unwrap().values(),
        [0.0, 28.0, 40.0, 20.0]
    );
}
#[test]
fn hard_limits_constrain_root_and_grid_allocations() {
    let bounded = || {
        Container::default()
            .limits(Size::ZERO, Some(Size::new(40.0, 20.0).unwrap()))
            .unwrap()
    };
    let mut ui = runtime();
    let root = ui.mount_tree(Element::new("root", bounded())).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(
        ui.bounds(&root).unwrap().unwrap().values(),
        [0.0, 0.0, 40.0, 20.0]
    );
    ui.unmount(&root).unwrap();
    ui.set_viewport(Viewport::new(208.0, 100.0, 1.0).unwrap())
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "grid",
            Grid::new(
                [Track::weighted(1.0).unwrap(), Track::weighted(1.0).unwrap()],
                [
                    Element::new("limited", bounded()),
                    tile("tall", 30.0, 100.0),
                ],
            )
            .unwrap(),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let child = ui.children(&root).unwrap()[0].clone();
    assert_eq!(
        ui.bounds(&child).unwrap().unwrap().values(),
        [0.0, 0.0, 40.0, 20.0]
    );
}

#[test]
fn hard_width_limit_constrains_measurement_before_wrapped_height() {
    let mut ui = runtime();
    ui.set_viewport(Viewport::new(200.0, 200.0, 1.0).unwrap())
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([Element::new(
                "limited",
                Container::new(Element::new(
                    "wrap",
                    Wrap::new([
                        tile("a", 40.0, 20.0),
                        tile("b", 40.0, 20.0),
                        tile("c", 40.0, 20.0),
                    ]),
                ))
                .limits(Size::ZERO, Some(Size::new(80.0, 200.0).unwrap()))
                .unwrap(),
            )]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let limited = ui.children(&root).unwrap()[0].clone();
    assert_eq!(
        ui.bounds(&limited).unwrap().unwrap().values(),
        [0.0, 0.0, 40.0, 76.0]
    );
    let wrap = ui.children(&limited).unwrap()[0].clone();
    let last = ui.children(&wrap).unwrap()[2].clone();
    assert_eq!(
        ui.bounds(&last).unwrap().unwrap().values(),
        [0.0, 56.0, 40.0, 20.0]
    );
}

#[test]
fn translated_leaf_keeps_arrangement_and_font_metric_dependencies() {
    struct Positioned {
        geometry: ReadSignal<[f32; 3]>,
        current: [f32; 3],
    }
    impl Component for Positioned {
        fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
            self.current = cx.read(&self.geometry)?;
            Ok(())
        }
        fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
            let [x, width, clip_width] = self.current;
            cx.place_clipped(
                0,
                Rect::new(x, 0.0, width, 24.0).unwrap(),
                Rect::new(0.0, 0.0, clip_width, 24.0).unwrap(),
            )
        }
    }
    struct Prepared {
        arrangements: Rc<Cell<usize>>,
    }
    impl Component for Prepared {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
            self.arrangements.set(self.arrangements.get() + 1);
            // Dependencies are collected only during arrangement in this custom
            // leaf; translation must not erase them when it skips the callback.
            let measure = cx.measurement();
            let _gap = measure.metric(MetricRole::IconGap);
            let _text = measure.text("Retained", Some(size.width()), TextStyle::Body)?;
            Ok(())
        }
        fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
            let size = painter.size().unwrap();
            painter.fill_rect(Rect::from_size(size), Color::ACCENT);
            Ok(())
        }
    }
    let mut ui = runtime();
    ui.set_viewport(Viewport::new(300.0, 100.0, 1.0).unwrap())
        .unwrap();
    let geometry = ui.signal([0.0, 100.0, 200.0]);
    let arrangements = Rc::new(Cell::new(0));
    let root = ui
        .mount_tree(
            Element::new(
                "positioned",
                Positioned {
                    geometry: geometry.read_only(),
                    current: [0.0; 3],
                },
            )
            .children([Element::new(
                "prepared",
                Prepared {
                    arrangements: arrangements.clone(),
                },
            )]),
        )
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.paint().unwrap();
    assert_eq!(arrangements.get(), 1);
    let leaf = ui.children(&root).unwrap()[0].clone();
    geometry.set([40.0, 100.0, 200.0]).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(arrangements.get(), 1);
    assert_eq!(
        ui.bounds(&leaf).unwrap().unwrap().values(),
        [40.0, 0.0, 100.0, 24.0]
    );
    geometry.set([40.0, 100.0, 80.0]).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(arrangements.get(), 1);
    assert_eq!(
        ui.paint().unwrap().commands(),
        &[DrawCommand::FillRect {
            rect: Rect::new(40.0, 0.0, 40.0, 24.0).unwrap(),
            color: Color::ACCENT,
        }]
    );
    let fonts = ui.fonts();
    fonts
        .set_default(
            FontRole::Body,
            fonts.default_font(FontRole::Monospace).unwrap(),
        )
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(arrangements.get(), 2);
    // Skip once more, then change a metric read only during arrangement.
    geometry.set([30.0, 100.0, 80.0]).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(arrangements.get(), 2);
    ui.theme().set_density(Density::Comfortable).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(arrangements.get(), 3);
    geometry.set([30.0, 120.0, 80.0]).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(arrangements.get(), 4);
    ui.set_viewport(Viewport::new(300.0, 100.0, 1.25).unwrap())
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(arrangements.get(), 5);
    ui.invalidate(&leaf, Invalidation::Layout).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(arrangements.get(), 6);
}

#[test]
fn bordered_container_paints_a_rounded_panel_above_its_child() {
    let mut ui = runtime();
    ui.mount_tree(Element::new(
        "panel",
        Container::new(leaf("content"))
            .background_role(ColorRole::Surface)
            .border_role(ColorRole::PanelBorder),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let scene = ui.paint().unwrap();
    let commands = scene.commands();
    let border = Palette::dark().color(ColorRole::PanelBorder);
    let surface = Palette::dark().color(ColorRole::Surface);
    let top_edge = commands
        .iter()
        .position(|c| matches!(c, DrawCommand::FillRect { rect, color } if *color == border && rect.y() == 0.0 && rect.height() == 1.0))
        .expect("top border edge at border width");
    let child = commands
        .iter()
        .position(|c| matches!(c, DrawCommand::FillRect { color, .. } if *color == Color::ACCENT))
        .expect("child fill");
    assert!(top_edge > child, "border is painted above the child");
    assert!(
        commands
            .iter()
            .any(|c| matches!(c, DrawCommand::DrawMask { color, .. } if *color == surface)),
        "the fill uses rounded corners"
    );
    assert!(
        commands
            .iter()
            .any(|c| matches!(c, DrawCommand::DrawMask { color, .. } if *color == border)),
        "the border uses rounded corners"
    );
}

#[test]
fn inspection_reports_the_painted_border_stroke_and_omits_an_unpainted_one() {
    let mut ui = runtime();
    ui.set_viewport(Viewport::new(200.0, 200.0, 1.0).unwrap())
        .unwrap();
    ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new(
                "panel",
                Container::new(leaf("content")).border_role(ColorRole::PanelBorder),
            ),
            Element::new("plain", Container::new(leaf("other"))),
        ]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let unpainted = ui.inspect();
    assert_eq!(
        unpainted.components[1].details.box_model.border, None,
        "a border is absent until it is painted"
    );
    ui.paint().unwrap();
    let painted = ui.inspect();
    let width = ui
        .theme()
        .metrics(Density::Standard)
        .unwrap()
        .get(MetricRole::BorderWidth);
    assert_eq!(
        painted.components[1].details.box_model.border,
        Some(BorderInspection {
            width,
            color: Palette::dark().color(ColorRole::PanelBorder).channels(),
        })
    );
    assert_eq!(
        painted.components[3].details.box_model.border, None,
        "a container without a border role reports none"
    );
}

/// A focusable leaf with private state that no binding restores.
struct Counter {
    presses: u32,
    updates: Rc<Cell<usize>>,
}
impl Component for Counter {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.updates.set(self.updates.get() + 1);
        Ok(())
    }
    fn focusable(&self) -> bool {
        true
    }
    fn keyboard_event(
        &mut self,
        _: &mut EventContext,
        event: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        if event.transition == KeyTransition::Pressed {
            self.presses += 1;
        }
        Ok(InputOutcome::Handled)
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(10.0, 10.0)?)
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        details
            .properties
            .insert("presses".into(), InspectValue::Number(self.presses.into()));
        details
    }
}

/// A container that promotes one child to a popover surface.
struct SurfaceHost;
impl Component for SurfaceHost {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn child_surface(&self, key: &str) -> Option<SurfaceKind> {
        (key == "popover").then_some(SurfaceKind::Popover)
    }
}

fn incarnation(ui: &Runtime, key: &str) -> u64 {
    ui.inspect()
        .components
        .into_iter()
        .find(|component| component.key == key)
        .expect("mounted component")
        .id
}

fn presses(ui: &Runtime, key: &str) -> InspectValue {
    ui.inspect()
        .components
        .into_iter()
        .find(|component| component.key == key)
        .expect("mounted component")
        .details
        .properties["presses"]
        .clone()
}

#[test]
fn reparenting_moves_retained_state_focus_and_navigation_order() {
    let mut ui = runtime();
    ui.set_viewport(Viewport::new(200.0, 200.0, 1.0).unwrap())
        .unwrap();
    let updates = Rc::new(Cell::new(0));
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new("left", Column::new([])).children([
                    Element::new(
                        "item",
                        Counter {
                            presses: 0,
                            updates: updates.clone(),
                        },
                    ),
                    leaf("tail"),
                ]),
                Element::new("right", Column::new([])).children([leaf("other")]),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let [left, right] = <[WidgetId; 2]>::try_from(ui.children(&root).unwrap()).unwrap();
    let item = ui.children(&left).unwrap()[0].clone();
    ui.focus(&item).unwrap();
    ui.key_input(KeyCode::KeyA, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::KeyA, KeyAction::Release).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(presses(&ui, "item"), InspectValue::Number(1.0));
    let before = updates.get();
    let original = incarnation(&ui, "item");

    ui.reparent(&item, &right, 0).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(
        ui.children(&right).unwrap(),
        [item.clone(), ui.children(&right).unwrap()[1].clone()],
        "the moved child takes the requested position"
    );
    assert_eq!(ui.children(&left).unwrap().len(), 1);
    assert_eq!(
        ui.focused(),
        Some(item.clone()),
        "focus follows the subtree"
    );
    ui.key_input(KeyCode::KeyA, KeyAction::Press).unwrap();
    assert_eq!(
        presses(&ui, "item"),
        InspectValue::Number(2.0),
        "private state survives the move"
    );
    assert_eq!(
        updates.get() - before,
        1,
        "the move schedules one update, not a remount"
    );
    assert_eq!(
        incarnation(&ui, "item"),
        original,
        "the incarnation is unchanged"
    );

    // The freed key is available again under the old owner.
    ui.reconcile_children(
        &left,
        vec![
            leaf("tail"),
            Element::new(
                "item",
                Counter {
                    presses: 0,
                    updates,
                },
            ),
        ],
    )
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(
        ui.reparent(&item, &left, 0),
        Err(RuntimeError::DuplicateKey),
        "the new owner already uses the key"
    );
    assert_eq!(
        ui.reparent(&root, &item, 0),
        Err(RuntimeError::Layout(LayoutError::InvalidChild)),
        "an owner cannot move into its own subtree"
    );
}

#[test]
fn a_promoted_surface_root_cannot_move_to_another_owner() {
    let mut ui = runtime();
    let root = ui
        .mount_tree(Element::new("root", Column::new([])).children([
            Element::new("host", SurfaceHost).children([leaf("popover")]),
            Element::new("other", Column::new([])),
        ]))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let [host, other] = <[WidgetId; 2]>::try_from(ui.children(&root).unwrap()).unwrap();
    let popover = ui.children(&host).unwrap()[0].clone();
    assert_eq!(
        ui.reparent(&popover, &other, 0),
        Err(RuntimeError::NotMovable)
    );
    assert_eq!(ui.children(&host).unwrap(), [popover]);
}

#[test]
fn a_child_moved_inside_its_describing_parent_stays_described_by_it() {
    let mut ui = runtime();
    let root = ui
        .mount_tree(Element::new("root", Column::new([])).children([
            Element::new("home", Column::new([])).children([
                leaf("item"),
                Element::new("inner", Column::new([])).children([leaf("fixed")]),
            ]),
            Element::new("away", Column::new([])),
        ]))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let home = ui.find_path(&root, &["home"]).unwrap().unwrap();
    let inner = ui.find_path(&home, &["inner"]).unwrap().unwrap();
    let away = ui.find_path(&root, &["away"]).unwrap().unwrap();
    let item = ui.find_path(&home, &["item"]).unwrap().unwrap();
    let described = || {
        vec![
            leaf("item"),
            Element::new("inner", Column::new([])).children([leaf("fixed")]),
        ]
    };

    // Moved deeper into `home`, the item is still described by `home`.
    ui.reparent(&item, &inner, usize::MAX).unwrap();
    ui.reconcile_children(&home, described()).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(ui.children(&home).unwrap(), std::slice::from_ref(&inner));
    let held = ui.children(&inner).unwrap();
    assert_eq!(held.len(), 2);
    assert_eq!(held[1], item, "matched where it is, not remounted");
    ui.reconcile_children(&inner, vec![leaf("fixed")]).unwrap();
    assert!(
        ui.children(&inner).unwrap().contains(&item),
        "the owner holding it does not describe it"
    );
    assert_eq!(
        ui.reconcile_children(&inner, vec![leaf("fixed"), leaf("item")]),
        Err(RuntimeError::DuplicateKey),
        "nor may the owner describe the same key"
    );
    assert_eq!(
        ui.reparent(&inner, &away, 0),
        Err(RuntimeError::NotMovable),
        "the item cannot leave the parent describing it with its owner"
    );
    ui.reconcile_children(
        &home,
        vec![Element::new("inner", Column::new([])).children([leaf("fixed")])],
    )
    .unwrap();
    assert!(ui.key(&item).is_err(), "omitting it unmounts it");
    assert_eq!(ui.children(&inner).unwrap().len(), 1);

    // Moved anywhere else, the new owner describes it.
    ui.reconcile_children(&home, described()).unwrap();
    let item = ui.find_path(&home, &["item"]).unwrap().unwrap();
    ui.reparent(&item, &away, 0).unwrap();
    ui.reconcile_children(&home, described()).unwrap();
    let fresh = ui.find_path(&home, &["item"]).unwrap().unwrap();
    assert_ne!(fresh, item, "the former parent mounts its own again");
    assert_eq!(ui.children(&away).unwrap(), std::slice::from_ref(&item));
    ui.reconcile_children(&away, Vec::new()).unwrap();
    assert!(ui.key(&item).is_err());
    assert!(
        ui.validation_report().issues.is_empty(),
        "{:?}",
        ui.validation_report().issues
    );
}

#[test]
fn fully_clipped_animation_pauses_and_revealing_restarts_without_remount()
-> Result<(), Box<dyn std::error::Error>> {
    struct Clip {
        width: ReadSignal<f32>,
        current: f32,
    }
    impl Component for Clip {
        fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
            self.current = cx.read(&self.width)?;
            Ok(())
        }
        fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
            Ok(Size::new(60.0, 30.0)?)
        }
        fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
            cx.place_clipped(
                0,
                Rect::new(0.0, 0.0, 40.0, 20.0).unwrap(),
                Rect::new(0.0, 0.0, self.current, 30.0).unwrap(),
            )
        }
    }
    let mut ui = runtime();
    ui.set_presentation_rate(Some(PresentationRate::new(120.0)?))?;
    let width = ui.signal(60.0);
    let root = ui.mount_tree(
        Element::new(
            "clip",
            Clip {
                width: width.read_only(),
                current: 60.0,
            },
        )
        .children([Element::new(
            "live",
            Leaf {
                live: true,
                ..Default::default()
            },
        )]),
    )?;
    ui.flush(Duration::ZERO)?;
    let child = ui.children(&root)?[0].clone();
    ui.paint()?;
    assert_eq!(ui.next_deadline(), Some(Duration::from_nanos(8_333_333)));
    width.set(0.0)?;
    ui.flush(Duration::ZERO)?;
    ui.paint()?;
    assert_eq!(ui.next_deadline(), None);
    assert!(!ui.needs_paint());
    assert_eq!(ui.flush(Duration::from_secs(5))?.components_updated, 0);
    width.set(60.0)?;
    ui.flush(Duration::from_secs(5))?;
    assert_eq!(ui.children(&root)?[0], child);
    assert_eq!(
        ui.next_deadline(),
        Some(Duration::from_secs(5) + Duration::from_nanos(8_333_333))
    );
    Ok(())
}

/// A container that counts its `arrange` calls and insets its one child.
struct Counted {
    arranges: std::rc::Rc<std::cell::Cell<usize>>,
    children: Vec<Element>,
}
impl Component for Counted {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(
        &self,
        _: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        Ok(Size::new(available.width().unwrap_or(200.0), 100.0)?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        self.arranges.set(self.arranges.get() + 1);
        for index in 0..cx.len() {
            let inset = Rect::new(10.0, 5.0, size.width() - 20.0, 20.0)?;
            cx.place(index, inset)?;
        }
        Ok(())
    }
}

#[test]
fn scrolling_moves_a_composite_component_without_arranging_it_again() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 150.0, 1.0).unwrap())
        .unwrap();
    let arranges = std::rc::Rc::new(std::cell::Cell::new(0));
    let rows = (0..10).map(|index| {
        Element::new(
            format!("row-{index}"),
            Counted {
                arranges: arranges.clone(),
                children: vec![Element::new("label", Label::new(format!("Row {index}")))],
            },
        )
    });
    let scroll = ui
        .mount_tree(Element::new(
            "scroll",
            ScrollArea::new(Element::new("rows", Column::new(rows))),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let column = ui.children(&scroll).unwrap()[0].clone();
    let row = ui.children(&column).unwrap()[3].clone();
    let label = ui.children(&row).unwrap()[0].clone();
    let before = ui.bounds(&label).unwrap().unwrap();
    let arranged = arranges.get();

    ui.pointer_input(PointerEvent::new(
        Point::new(50.0, 50.0).unwrap(),
        PointerKind::Scroll(Point::new(0.0, 30.0).unwrap()),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let after = ui.bounds(&label).unwrap().unwrap();
    assert_eq!(
        arranges.get(),
        arranged,
        "a translation reuses the placements"
    );
    assert_eq!(
        after.values(),
        [
            before.x(),
            before.y() - 30.0,
            before.width(),
            before.height()
        ]
    );

    // A new size still arranges them.
    ui.set_viewport(Viewport::new(280.0, 150.0, 1.0).unwrap())
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(arranges.get() > arranged);
    assert_eq!(
        ui.bounds(&label).unwrap().unwrap().width(),
        before.width() - 20.0
    );
}

/// A leaf that fails its arrangement while `fail` is set.
struct Failing {
    fail: std::rc::Rc<std::cell::Cell<bool>>,
}
impl Component for Failing {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(
        &self,
        _: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        Ok(Size::new(available.width().unwrap_or(10.0), 10.0)?)
    }
    fn arrange(&self, _: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        if self.fail.get() {
            return Err(LayoutError::InvalidChild.into());
        }
        Ok(())
    }
}

/// Places its children at a width it reads from a signal, so a change
/// reallocates them without invalidating their own layout.
struct Sized {
    width: ReadSignal<f32>,
    current: f32,
    children: Vec<Element>,
}
impl Component for Sized {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.current = cx.read(&self.width)?;
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(400.0, 400.0)?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        for index in 0..cx.len() {
            let y = 100.0 * index as f32;
            let slot = Rect::new(0.0, y, self.current, 100.0)?;
            cx.place(index, slot)?;
        }
        Ok(())
    }
}

#[test]
fn a_failed_layout_pass_leaves_no_placement_to_reuse() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 150.0, 1.0).unwrap())
        .unwrap();
    let arranges = std::rc::Rc::new(std::cell::Cell::new(0));
    let fail = std::rc::Rc::new(std::cell::Cell::new(false));
    let width = ui.signal(200.0_f32);
    let scroll = ui
        .mount_tree(Element::new(
            "scroll",
            ScrollArea::new(Element::new(
                "sized",
                Sized {
                    width: width.read_only(),
                    current: 0.0,
                    children: vec![
                        Element::new(
                            "counted",
                            Counted {
                                arranges: arranges.clone(),
                                children: vec![Element::new("label", Label::new("Row"))],
                            },
                        ),
                        Element::new("failing", Failing { fail: fail.clone() }),
                    ],
                },
            )),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let sized = ui.children(&scroll).unwrap()[0].clone();
    let counted = ui.children(&sized).unwrap()[0].clone();
    let label = ui.children(&counted).unwrap()[0].clone();
    assert_eq!(ui.bounds(&label).unwrap().unwrap().width(), 180.0);

    // A wider allocation fails after the first child was arranged for it.
    fail.set(true);
    width.set(300.0).unwrap();
    assert!(ui.flush(Duration::ZERO).is_err());
    fail.set(false);
    width.set(200.0).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    // Scrolling moves the child at its committed size: its placements must be
    // the committed ones, not the failed pass's.
    ui.pointer_input(PointerEvent::new(
        Point::new(50.0, 50.0).unwrap(),
        PointerKind::Scroll(Point::new(0.0, 20.0).unwrap()),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(ui.bounds(&label).unwrap().unwrap().width(), 180.0);
}
