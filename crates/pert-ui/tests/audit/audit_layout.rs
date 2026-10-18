//! Composite minimums and consistent natural layout through the public facade.
use crate::support;
use pert_ui::*;
use support::{frame, widget};

fn ui(width: f32, height: f32) -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(width, height, 1.0).unwrap())
        .unwrap();
    ui
}
fn bounds(ui: &Runtime, root: &WidgetId, key: &str) -> Rect {
    ui.bounds(&widget(ui, root, key)).unwrap().unwrap()
}
fn grid() -> Element {
    Element::new(
        "grid",
        Grid::new(
            [Track::fixed(120.into()); 3],
            (0..3).map(|i| {
                Element::new(
                    format!("cell-{i}"),
                    Container::default().size_limits(SizeLimits::at_least(Size::from([20, 48]))),
                )
            }),
        )
        .unwrap(),
    )
}
#[test]
fn nested_padding_and_descendant_minimums_survive_a_small_preference() {
    let mut ui = ui(300.0, 100.0);
    let root = ui
        .mount_tree(Element::new(
            "scroll",
            ScrollArea::new(Element::new(
                "column",
                Column::new([Element::new(
                    "outer",
                    Container::new(Element::new(
                        "inner",
                        Container::new(grid()).padding(Length::from(8)),
                    ))
                    .padding(Length::from(8))
                    .preferred_size(Size::from([40, 20])),
                )]),
            ))
            .axes(ScrollAxes::Both),
        ))
        .unwrap();
    frame(&mut ui);
    assert_eq!(bounds(&ui, &root, "outer").width(), 408.0);
    assert_eq!(bounds(&ui, &root, "outer").height(), 80.0);
    assert_eq!(bounds(&ui, &root, "inner").width(), 392.0);
    assert_eq!(bounds(&ui, &root, "grid").width(), 376.0);
    assert!(ui.validation_report().issues.is_empty());
    ui.pointer_input(PointerEvent::new(
        Point::new(10.0, 10.0).unwrap(),
        PointerKind::Scroll(Point::new(1000.0, 0.0).unwrap()),
    ))
    .unwrap();
    frame(&mut ui);
    assert_eq!(bounds(&ui, &root, "outer").x(), -108.0);
    assert_eq!(
        (bounds(&ui, &root, "cell-2").x() + bounds(&ui, &root, "cell-2").width()),
        284.0
    );
    ui.set_visible(&widget(&ui, &root, "grid"), false).unwrap();
    frame(&mut ui);
    assert_eq!(bounds(&ui, &root, "outer").width(), 40.0);
    assert_eq!(bounds(&ui, &root, "outer").height(), 32.0);
    ui.set_visible(&widget(&ui, &root, "grid"), true).unwrap();
    frame(&mut ui);
    assert_eq!(bounds(&ui, &root, "outer").width(), 408.0);
}
#[test]
fn explicit_container_maximum_keeps_its_clipping_contract() {
    let mut ui = ui(300.0, 100.0);
    let root = ui
        .mount_tree(Element::new(
            "scroll",
            ScrollArea::new(Element::new(
                "wrapper",
                Container::new(grid())
                    .padding(Length::from(8))
                    .limits(Size::ZERO, Some(Size::from([200, 40])))
                    .unwrap(),
            ))
            .axes(ScrollAxes::Both),
        ))
        .unwrap();
    frame(&mut ui);
    assert_eq!(bounds(&ui, &root, "wrapper").width(), 200.0);
    assert_eq!(bounds(&ui, &root, "wrapper").height(), 40.0);
    assert_eq!(bounds(&ui, &root, "grid").width(), 376.0);
}
#[test]
fn vertical_divider_uses_row_content_height_instead_of_parent_height() {
    for height in [32, 60] {
        let mut ui = ui(300.0, 200.0);
        let root = ui
            .mount_tree(Element::new(
                "row",
                Row::new([
                    Element::new("button", Button::new("Action", |_| Ok(()))),
                    Element::new("divider", Divider::new().vertical()),
                    Element::new(
                        "tall",
                        Container::default().preferred_size(Size::from([20, height])),
                    ),
                ]),
            ))
            .unwrap();
        frame(&mut ui);
        assert_eq!(
            bounds(&ui, &root, "button").y(),
            (f32::from(height) - 32.0) / 2.0
        );
        assert_eq!(bounds(&ui, &root, "divider").y(), 0.0);
        assert_eq!(bounds(&ui, &root, "divider").height(), f32::from(height));
        assert_eq!(bounds(&ui, &root, "tall").y(), 0.0);
    }
}
#[test]
fn unbounded_grid_measure_uses_fixed_track_wrapping_before_arrangement() {
    let mut ui = ui(400.0, 400.0);
    let root = ui
        .mount_tree(Element::new(
            "row",
            Row::new([Element::new(
                "column",
                Column::new([
                    Element::new(
                        "grid",
                        Grid::new(
                            [Track::fixed(50.into())],
                            [Element::new(
                                "label",
                                Label::new("A long label that needs several wrapped lines"),
                            )],
                        )
                        .unwrap(),
                    ),
                    Element::new("after", Label::new("After")),
                ]),
            )]),
        ))
        .unwrap();
    frame(&mut ui);
    let grid = bounds(&ui, &root, "grid");
    let label = bounds(&ui, &root, "label");
    let after = bounds(&ui, &root, "after");
    assert_eq!(grid.width(), 50.0);
    assert!(label.height() > 32.0);
    assert_eq!(grid.height(), label.height());
    assert!(after.y() >= (grid.y() + grid.height()) + 8.0);
    assert!(ui.validation_report().issues.is_empty());
}

#[test]
fn unbounded_weighted_grid_resolves_width_shares_before_row_height() {
    let mut ui = ui(800.0, 400.0);
    let root = ui.mount_tree(Element::new("column", Column::new([
        Element::new("row", Row::new([Element::new("grid", Grid::new(
            [Track::weighted(1.0).unwrap(); 2], [
                Element::new("long", Label::new("A considerably longer label that needs wrapping in its share")),
                Element::new("short", Label::new("B")),
            ]).unwrap())])),
        Element::new("after", Label::new("After")),
    ]))).unwrap();
    frame(&mut ui);
    let grid = bounds(&ui, &root, "grid");
    let long = bounds(&ui, &root, "long");
    let short = bounds(&ui, &root, "short");
    assert!((long.width() - short.width()).abs() < 0.01);
    assert!(long.height() > 20.0);
    assert_eq!(grid.height(), long.height());
    assert!(bounds(&ui, &root, "after").y() >= grid.y() + grid.height() + 8.0);
    assert!(ui.validation_report().issues.is_empty());
}

#[test]
fn explicit_one_axis_maximum_and_large_padding_are_validated_before_narrowing() {
    let mut ui = ui(300.0, 100.0);
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([
                Element::new(
                    "one-axis",
                    Container::new(grid()).padding(Length::from(8)).size_limits(
                        SizeLimits::new(
                            Size::from([180, 70]),
                            Constraints::new(Some(200.0), None).unwrap(),
                        )
                        .unwrap(),
                    ),
                ),
                Element::new(
                    "large-padding",
                    Container::default()
                        .padding(Length::new(f32::MAX).unwrap())
                        .limits(Size::ZERO, Some(Size::from([80, 10])))
                        .unwrap(),
                ),
            ]),
        ))
        .unwrap();
    frame(&mut ui);
    assert_eq!(bounds(&ui, &root, "one-axis").width(), 200.0);
    assert_eq!(bounds(&ui, &root, "one-axis").height(), 70.0);
    assert_eq!(bounds(&ui, &root, "large-padding").width(), 80.0);
    assert_eq!(bounds(&ui, &root, "large-padding").height(), 10.0);
    ui.unmount(&root).unwrap();
    ui.mount_tree(Element::new(
        "uncapped",
        Container::default().padding(Length::new(f32::MAX).unwrap()),
    ))
    .unwrap();
    assert!(matches!(
        ui.flush(ui.now()),
        Err(RuntimeError::Geometry(GeometryError::NonFinite))
    ));
}
#[test]
fn theme_padding_and_redescription_refresh_propagated_minimums() {
    let mut ui = ui(300.0, 100.0);
    let root = ui
        .mount_tree(Element::new(
            "scroll",
            ScrollArea::new(Element::new(
                "wrapper",
                Container::new(grid()).padding(MetricRole::ContentMargin),
            ))
            .axes(ScrollAxes::Both),
        ))
        .unwrap();
    for density in [Density::Standard, Density::Compact, Density::Comfortable] {
        ui.theme().set_density(density).unwrap();
        frame(&mut ui);
        let padding = ui
            .inspect()
            .components
            .into_iter()
            .find(|c| c.key == "wrapper")
            .unwrap()
            .details
            .box_model
            .padding
            .unwrap()[0];
        let grid = bounds(&ui, &root, "grid");
        assert_eq!(
            bounds(&ui, &root, "wrapper").width(),
            grid.width() + 2.0 * padding
        );
    }
    let before = widget(&ui, &root, "wrapper");
    ui.reconcile_children(
        &root,
        vec![Element::new(
            "wrapper",
            Container::new(grid()).padding(Length::from(4)),
        )],
    )
    .unwrap();
    frame(&mut ui);
    assert_eq!(widget(&ui, &root, "wrapper"), before);
    assert_eq!(
        bounds(&ui, &root, "wrapper").width(),
        bounds(&ui, &root, "grid").width() + 8.0
    );
}
#[test]
fn horizontal_scroll_grid_keeps_wrapped_height_and_fixed_content_track_policy() {
    let mut ui = ui(400.0, 400.0);
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([
                Element::new(
                    "scroll",
                    ScrollArea::new(Element::new(
                        "grid",
                        Grid::new(
                            [Track::fixed(50.into()), Track::CONTENT],
                            [
                                Element::new(
                                    "wrapped",
                                    Label::new("A long label that needs several wrapped lines"),
                                ),
                                Element::new("intrinsic", Label::new("Intrinsic column")),
                            ],
                        )
                        .unwrap(),
                    ))
                    .axes(ScrollAxes::Horizontal),
                ),
                Element::new("after", Label::new("After")),
            ]),
        ))
        .unwrap();
    frame(&mut ui);
    let grid = bounds(&ui, &root, "grid");
    let wrapped = bounds(&ui, &root, "wrapped");
    assert_eq!(wrapped.width(), 50.0);
    assert!(bounds(&ui, &root, "intrinsic").width() > 50.0);
    assert_eq!(grid.height(), wrapped.height());
    assert!(bounds(&ui, &root, "after").y() >= grid.y() + grid.height() + 8.0);
    assert!(ui.validation_report().issues.is_empty());
}

#[test]
fn a_bounded_row_keeps_virtual_list_work_bounded() {
    struct ListViewport(Vec<Element>);
    impl Component for ListViewport {
        fn take_children(&mut self) -> Vec<Element> {
            std::mem::take(&mut self.0)
        }
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn measure(
            &self,
            cx: &MeasureContext<'_>,
            available: Constraints,
        ) -> Result<Size, RuntimeError> {
            cx.measure(0, Constraints::new(Some(200.0), available.height())?)
        }
        fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
            cx.place(0, Rect::from_size(size))
        }
    }
    struct Rows(std::rc::Rc<std::cell::Cell<usize>>);
    impl ListItems for Rows {
        fn len(&self) -> usize {
            1000
        }
        fn key(&self, index: usize) -> Option<String> {
            (index < self.len()).then(|| format!("item-{index}"))
        }
        fn build(&self, index: usize) -> Option<Element> {
            let key = self.key(index)?;
            self.0.set(self.0.get() + 1);
            Some(Element::new(key, Label::new("Row")))
        }
    }
    for height in [20.0, 200.0] {
        let mut ui = ui(300.0, height);
        let builds = std::rc::Rc::new(std::cell::Cell::new(0));
        let source = ui.signal(ListSource::new(Rows(builds.clone())));
        let root = ui
            .mount_tree(Element::new(
                "row",
                Row::new([
                    Element::new(
                        "viewport",
                        ListViewport(vec![Element::new(
                            "list",
                            VirtualList::new(source.read_only(), 40.0).unwrap(),
                        )]),
                    ),
                    Element::new("divider", Divider::new().vertical()),
                ]),
            ))
            .unwrap();
        frame(&mut ui);
        assert_eq!(bounds(&ui, &root, "list").height(), height);
        assert_eq!(bounds(&ui, &root, "divider").height(), height);
        assert!(builds.get() < 20, "built {} rows", builds.get());
        assert!(ui.validation_report().issues.is_empty());
    }
}
#[test]
fn mixed_fixed_and_weighted_tracks_keep_fractional_text_height_contained() {
    for text in [
        "A fractional natural width",
        "iiiiii WWW",
        "An ordinary short label",
    ] {
        let mut ui = ui(800.0, 400.0);
        let root = ui
            .mount_tree(Element::new(
                "row",
                Row::new([Element::new(
                    "column",
                    Column::new([
                        Element::new(
                            "grid",
                            Grid::new(
                                [
                                    Track::fixed(Length::new(120.3).unwrap()),
                                    Track::weighted(1.0).unwrap(),
                                ],
                                [
                                    Element::new("fixed", Label::new("Fixed")),
                                    Element::new("weighted", Label::new(text)),
                                ],
                            )
                            .unwrap(),
                        ),
                        Element::new("after", Label::new("After")),
                    ]),
                )]),
            ))
            .unwrap();
        frame(&mut ui);
        let grid = bounds(&ui, &root, "grid");
        let label = bounds(&ui, &root, "weighted");
        assert_eq!(grid.height(), label.height());
        assert!(bounds(&ui, &root, "after").y() >= grid.y() + grid.height() + 8.0);
        assert!(ui.validation_report().issues.is_empty());
    }
}
