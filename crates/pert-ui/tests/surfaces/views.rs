//! Default view, scroll and nested-region layout contracts through the facade.
use crate::support;
use pert_ui::*;
use std::{cell::Cell, rc::Rc, time::Duration};
use support::find;
fn ui(w: f32, h: f32) -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(w, h, 1.0).unwrap()).unwrap();
    ui
}
fn button(key: &str) -> Element {
    Element::new(key, Button::new(key, |_| Ok(())))
}
fn bounds(ui: &Runtime, id: &WidgetId) -> [f32; 4] {
    ui.bounds(id).unwrap().unwrap().values()
}
#[test]
fn scroll_focus_reveals_offscreen_button_and_wheel_reuses_identity() {
    let mut ui = ui(200.0, 80.0);
    let scroll = ui
        .mount_tree(Element::new(
            "scroll",
            ScrollArea::new(Element::new(
                "column",
                Column::new((0..10).map(|n| button(&format!("item-{n}")))),
            )),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let column = ui.children(&scroll).unwrap()[0].clone();
    let buttons = ui.children(&column).unwrap();
    assert!(bounds(&ui, &buttons[9])[1] > 80.0);
    ui.focus(&buttons[9]).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let rect = bounds(&ui, &buttons[9]);
    assert!(rect[1] >= 0.0 && rect[1] + rect[3] <= 80.0);
    ui.paint().unwrap();
    ui.pointer_input(PointerEvent::new(
        Point::new(10.0, 10.0).unwrap(),
        PointerKind::Scroll(Point::new(0.0, -1000.0).unwrap()),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(bounds(&ui, &buttons[0])[1], 0.0);
    assert_eq!(ui.children(&column).unwrap(), buttons);
    ui.focus(&buttons[0]).unwrap();
    for _ in 0..9 {
        ui.focus_next(FocusDirection::Forward).unwrap();
    }
    assert_eq!(ui.focused(), Some(buttons[9].clone()));
    assert!(bounds(&ui, &buttons[9])[1] < 80.0);
}
#[test]
fn nested_region_layout_regions_fill_and_small_windows_keep_center_accessible() {
    let mut ui = ui(500.0, 300.0);
    let region_layout = ui
        .mount_tree(Element::new(
            "shell",
            RegionLayout::new()
                .top(button("toolbar"))
                .top_height(Length::from(40))
                .bottom(button("status"))
                .bottom_height(Length::from(24))
                .left(button("navigation"))
                .left_width(Length::from(100))
                .right(button("tools"))
                .right_width(Length::from(80))
                .center(Element::new(
                    "nested",
                    RegionLayout::new()
                        .top(button("local-tools"))
                        .top_height(Length::from(32))
                        .center(button("editor")),
                )),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    // The nested layout repeats the region keys, so the outer regions are
    // looked up by their path through the layout's splits.
    let region = |path: &[&str]| {
        let path = [&["shell-viewport", "regions", "split-top"], path].concat();
        ui.find_path(&region_layout, &path).unwrap().unwrap()
    };
    let top = region(&["top"]);
    let bottom = region(&["split-bottom", "bottom"]);
    let center = region(&["split-bottom", "split-left", "split-right", "center"]);
    assert_eq!(bounds(&ui, &top), [0.0, 0.0, 500.0, 40.0]);
    assert_eq!(bounds(&ui, &bottom), [0.0, 276.0, 500.0, 24.0]);
    assert_eq!(bounds(&ui, &center), [108.0, 48.0, 304.0, 220.0]);
    let nested = find(&ui, &region_layout, "nested").unwrap();
    let nested_center = find(&ui, &nested, "center").unwrap();
    assert_eq!(bounds(&ui, &nested_center), [108.0, 88.0, 304.0, 180.0]);
    let editor = find(&ui, &nested, "editor").unwrap();
    ui.set_viewport(Viewport::new(120.0, 90.0, 1.0).unwrap())
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.focus(&editor).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let [x, y, _, _] = bounds(&ui, &editor);
    assert!((0.0..120.0).contains(&x));
    assert!((0.0..90.0).contains(&y));
    ui.paint().unwrap();
}
struct Ticker(Rc<Cell<usize>>);
impl Component for Ticker {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.0.set(self.0.get() + 1);
        cx.wake_after(Duration::from_secs(1))?;
        Ok(())
    }
}
#[test]
fn bound_view_hides_work_and_retains_incarnation() {
    let mut ui = ui(300.0, 200.0);
    let visible = ui.signal(true);
    let ticks = Rc::new(Cell::new(0));
    let view = ui
        .mount_tree(Element::new(
            "view",
            View::new(Element::new("live", Ticker(ticks.clone())))
                .visible_signal(visible.read_only()),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let live = ui.children(&view).unwrap()[0].clone();
    visible.set(false).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(ticks.get(), 1);
    assert!(ui.next_deadline().is_none());
    ui.flush(Duration::from_secs(5)).unwrap();
    assert_eq!(ticks.get(), 1);
    visible.set(true).unwrap();
    ui.flush(Duration::from_secs(5)).unwrap();
    assert_eq!(ticks.get(), 2);
    assert_eq!(ui.children(&view).unwrap()[0], live);
}

#[test]
fn scrolling_wrap_preserves_viewport_width_and_exposes_horizontal_overflow() {
    let mut ui = ui(87.0, 100.0);
    let scroll = ui
        .mount_tree(Element::new(
            "scroll",
            ScrollArea::new(Element::new(
                "wrap",
                Wrap::new([40u16, 40, 100].into_iter().enumerate().map(|(i, w)| {
                    Element::new(
                        format!("item-{i}"),
                        Container::default()
                            .preferred_size(Size::from([w, 20]))
                            .background(Color::ACCENT),
                    )
                })),
            ))
            .axes(ScrollAxes::Both),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let wrap = ui.children(&scroll).unwrap()[0].clone();
    let items = ui.children(&wrap).unwrap();
    assert_eq!(bounds(&ui, &items[1])[1], 28.0);
    assert_eq!(bounds(&ui, &items[2])[1], 56.0);
    ui.pointer_input(PointerEvent::new(
        Point::new(10.0, 60.0).unwrap(),
        PointerKind::Scroll(Point::new(100.0, 0.0).unwrap()),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(bounds(&ui, &items[2])[0], -13.0);
    // The oversized item deliberately overflows the viewport-wide wrap. Once
    // scrolled, its visible part leaves the wrap, and development validation
    // says so; only the scroll area clips it.
    if cfg!(debug_assertions) {
        let issues = ui.validation_report().issues;
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert_eq!(issues[0].code, ValidationCode::EscapesParent);
        assert!(issues[0].message.contains("\"item-2\"") && issues[0].message.contains("\"wrap\""));
    }
    assert!(ui.paint().unwrap().commands().iter().any(|command| matches!(command, DrawCommand::FillRect { rect, color } if *color == Color::ACCENT && rect.values() == [0.0, 56.0, 87.0, 20.0])));
}
#[test]
fn a_grid_of_fixed_columns_scrolls_inside_its_own_bounds() {
    // Both-axis scrolling allocates its content at least the viewport width or
    // the content's hard minimum. The grid's fixed columns are that minimum, so
    // it is allocated their full width and every cell stays inside the grid.
    let mut ui = ui(300.0, 100.0);
    let scroll = ui
        .mount_tree(Element::new(
            "scroll",
            ScrollArea::new(Element::new(
                "grid",
                Grid::new(
                    [Track::fixed(120.into()); 3],
                    (0..6).map(|i| {
                        Element::new(
                            format!("cell-{i}"),
                            Container::default().preferred_size(Size::from([40, 48])),
                        )
                    }),
                )
                .unwrap(),
            ))
            .axes(ScrollAxes::Both),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let grid = ui.children(&scroll).unwrap()[0].clone();
    let cells = ui.children(&grid).unwrap();
    // Three 120-pixel columns and two 8-pixel gaps.
    assert_eq!(bounds(&ui, &grid)[2], 376.0);
    assert_eq!(bounds(&ui, &cells[5])[0] + bounds(&ui, &cells[5])[2], 376.0);
    ui.pointer_input(PointerEvent::new(
        Point::new(10.0, 10.0).unwrap(),
        PointerKind::Scroll(Point::new(1000.0, 1000.0).unwrap()),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let [x, _, w, _] = bounds(&ui, &grid);
    assert_eq!(x + w, 300.0, "scrolled to the grid's right edge");
    assert!(ui.validation_report().issues.is_empty());
}
#[test]
fn hidden_view_reconciliation_keeps_new_descendants_suspended() {
    let mut ui = ui(200.0, 100.0);
    let shown = ui.signal(false);
    let ticks = Rc::new(Cell::new(0));
    let view = ui
        .mount_tree(Element::new(
            "view",
            View::new(button("old")).visible_signal(shown.read_only()),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.reconcile_children(&view, vec![Element::new("new", Ticker(ticks.clone()))])
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(ticks.get(), 0);
    assert!(ui.next_deadline().is_none());
    shown.set(true).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(ticks.get(), 1);
}
#[test]
fn scroll_foreground_refreshes_after_content_growth() {
    let mut ui = ui(150.0, 90.0);
    let scroll = ui
        .mount_tree(Element::new(
            "scroll",
            ScrollArea::new(Element::new("column", Column::new([button("first")]))),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let resting = Palette::dark().color(ColorRole::ScrollbarThumb);
    let has_thumb = |scene: DisplayList| {
        scene.commands().iter().any(
            |command| matches!(command, DrawCommand::FillRect { color, .. } if *color == resting),
        )
    };
    assert!(!has_thumb(ui.paint().unwrap()));
    let column = ui.children(&scroll).unwrap()[0].clone();
    ui.reconcile_children(
        &column,
        (0..10).map(|i| button(&format!("item-{i}"))).collect(),
    )
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(has_thumb(ui.paint().unwrap()));
    ui.reconcile_children(&column, vec![button("item-0")])
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(!has_thumb(ui.paint().unwrap()));
}
#[test]
fn unbounded_top_only_region_layout_has_intrinsic_width_and_clipped_focus_is_skipped() {
    let mut ui = ui(300.0, 150.0);
    let row = ui
        .mount_tree(Element::new(
            "row",
            Row::new([Element::new(
                "shell",
                RegionLayout::new().top(button("top")),
            )]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let region_layout = ui.children(&row).unwrap()[0].clone();
    assert!(bounds(&ui, &region_layout)[2] >= 64.0);
    ui.unmount(&row).unwrap();
    let scroll = ui
        .mount_tree(Element::new(
            "scroll",
            ScrollArea::new(Element::new(
                "column",
                Column::new([
                    Element::new(
                        "zero",
                        Container::new(button("clipped"))
                            .limits(Size::ZERO, Some(Size::ZERO))
                            .unwrap(),
                    ),
                    button("visible"),
                ]),
            )),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let column = ui.children(&scroll).unwrap()[0].clone();
    let expected = ui.children(&column).unwrap()[1].clone();
    ui.focus_next(FocusDirection::Forward).unwrap();
    assert_eq!(ui.focused(), Some(expected));
}

#[test]
fn fixed_grid_cells_clip_overflowing_descendants_and_hits() {
    struct Tile(Rc<Cell<usize>>);
    impl Component for Tile {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
            Ok(Size::from([100, 20]))
        }
        fn pointer_input(
            &mut self,
            _: &mut EventContext,
            event: PointerEvent,
        ) -> Result<InputOutcome, RuntimeError> {
            if matches!(event.kind(), PointerKind::Press(_)) {
                self.0.set(self.0.get() + 1);
            }
            Ok(InputOutcome::Handled)
        }
        fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
            painter.fill_rect(Rect::new(0.0, 0.0, 100.0, 20.0)?, Color::ACCENT);
            Ok(())
        }
    }
    let mut ui = ui(200.0, 100.0);
    let presses = Rc::new(Cell::new(0));
    ui.mount_tree(Element::new(
        "grid",
        Grid::new(
            [Track::fixed(Length::from(80)); 2],
            [
                Element::new(
                    "wrap",
                    Wrap::new([Element::new("tile", Tile(presses.clone()))]),
                ),
                Element::new(
                    "other",
                    Container::default().preferred_size(Size::from([80, 20])),
                ),
            ],
        )
        .unwrap(),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(ui.paint().unwrap().commands().iter().any(|command| matches!(command, DrawCommand::FillRect { rect, color } if *color == Color::ACCENT && rect.values() == [0.0, 0.0, 80.0, 20.0])));
    ui.pointer_input(PointerEvent::new(
        Point::new(85.0, 10.0).unwrap(),
        PointerKind::Press(PointerButton::Primary),
    ))
    .unwrap();
    assert_eq!(presses.get(), 0);
}
#[test]
fn column_preserves_nested_wrap_width() {
    let mut ui = ui(87.0, 100.0);
    let column = ui
        .mount_tree(Element::new(
            "column",
            Column::new([Element::new(
                "wrap",
                Wrap::new([40u16, 40, 100].into_iter().enumerate().map(|(i, w)| {
                    Element::new(
                        format!("item-{i}"),
                        Container::default().preferred_size(Size::from([w, 20])),
                    )
                })),
            )]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let wrap = ui.children(&column).unwrap()[0].clone();
    let items = ui.children(&wrap).unwrap();
    assert_eq!(bounds(&ui, &items[1])[1], 28.0);
    assert_eq!(bounds(&ui, &items[2])[1], 56.0);
}

#[test]
fn fixed_view_overflow_starts_at_zero_and_never_scrolls_or_reveals() {
    let mut ui = ui(120.0, 60.0);
    let fixed = ui
        .mount_tree(Element::new(
            "fixed",
            View::new(Element::new(
                "content",
                Column::new((0..10).map(|n| button(&format!("item-{n}")))),
            ))
            .fixed(),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let first = find(&ui, &fixed, "item-0").unwrap();
    let last = find(&ui, &fixed, "item-9").unwrap();
    assert_eq!(bounds(&ui, &first)[1], 0.0);
    ui.pointer_input(PointerEvent::new(
        Point::new(20.0, 20.0).unwrap(),
        PointerKind::Scroll(Point::new(0.0, 400.0).unwrap()),
    ))
    .unwrap();
    ui.focus(&last).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(bounds(&ui, &first)[1], 0.0);
    assert!(bounds(&ui, &last)[1] >= 60.0);
    let scene = ui.paint().unwrap();
    for command in scene.commands() {
        if let DrawCommand::DrawMask {
            clip: Some(clip), ..
        } = command
        {
            let [_, y, w, h] = clip.values();
            assert!(w == 0.0 || h == 0.0 || y + h <= 60.0);
        }
    }
    assert!(ui.next_deadline().is_none());
}
#[test]
fn tiny_fixed_region_layout_keeps_origin_and_clips_below_minimum_sizes() {
    let mut ui = ui(40.0, 20.0);
    let region_layout = ui
        .mount_tree(Element::new(
            "shell",
            RegionLayout::new()
                .fixed()
                .top(button("menu"))
                .center(Element::new(
                    "tabs",
                    Tabs::new([Tab::new("a", "A", button("inside"))]),
                )),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let menu = find(&ui, &region_layout, "menu").unwrap();
    let inside = find(&ui, &region_layout, "inside").unwrap();
    let origin = bounds(&ui, &menu);
    assert_eq!(origin[0], 0.0);
    assert_eq!(origin[1], 0.0);
    ui.focus(&inside).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(bounds(&ui, &menu), origin);
    // Fixed clipping intentionally cannot expose controls below the outer frame.
    assert!(bounds(&ui, &inside)[1] >= 20.0);
    ui.paint().unwrap();
}

#[test]
fn scrollbar_drags_capture_both_axes_above_interactive_nested_content() {
    for (axes, horizontal) in [
        (ScrollAxes::Vertical, false),
        (ScrollAxes::Horizontal, true),
        (ScrollAxes::Both, false),
        (ScrollAxes::Both, true),
    ] {
        let mut ui = ui(100.0, 100.0);
        let clicks = Rc::new(Cell::new(0));
        let actions = clicks.clone();
        let root = ui
            .mount_tree(Element::new(
                "outer",
                ScrollArea::new(Element::new(
                    "scroll",
                    ScrollArea::new(Element::new(
                        "content",
                        Container::new(Element::new(
                            "button",
                            Button::new("Action", move |_| {
                                actions.set(actions.get() + 1);
                                Ok(())
                            }),
                        ))
                        .size_limits(SizeLimits::at_least(Size::from([400, 300]))),
                    ))
                    .axes(axes),
                ))
                .axes(ScrollAxes::None),
            ))
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        let scroll = ui.children(&root).unwrap()[0].clone();
        let content = ui.children(&scroll).unwrap()[0].clone();
        let point = |along, across| {
            if horizontal {
                Point::new(along, across).unwrap()
            } else {
                Point::new(across, along).unwrap()
            }
        };
        let send = |ui: &mut Runtime, position, kind| {
            ui.pointer_input(PointerEvent::new(position, kind)).unwrap();
            ui.flush(Duration::ZERO).unwrap();
        };
        let axis = usize::from(!horizontal);
        send(
            &mut ui,
            point(5.0, 98.0),
            PointerKind::Press(PointerButton::Primary),
        );
        assert_eq!(ui.focused(), Some(scroll.clone()));
        send(&mut ui, point(5.0, 98.0), PointerKind::Move);
        assert_eq!(
            bounds(&ui, &content)[axis],
            0.0,
            "thumb must not jump on grab"
        );
        send(&mut ui, point(40.0, 98.0), PointerKind::Move);
        assert!(bounds(&ui, &content)[axis] < 0.0);
        // The secondary release must not end the primary drag.
        send(
            &mut ui,
            point(40.0, 98.0),
            PointerKind::Release(PointerButton::Secondary),
        );
        send(&mut ui, point(500.0, -50.0), PointerKind::Move);
        let end = bounds(&ui, &content)[axis];
        assert_eq!(end, if horizontal { -300.0 } else { -200.0 });
        send(
            &mut ui,
            point(500.0, -50.0),
            PointerKind::Release(PointerButton::Primary),
        );
        send(&mut ui, point(0.0, 98.0), PointerKind::Move);
        assert_eq!(bounds(&ui, &content)[axis], end);
        // Track clicks also start a drag; Escape retains the new position.
        send(
            &mut ui,
            point(30.0, 98.0),
            PointerKind::Press(PointerButton::Primary),
        );
        let clicked = bounds(&ui, &content)[axis];
        assert!(clicked > end && clicked < 0.0);
        ui.key_input(KeyCode::Escape, KeyAction::Press).unwrap();
        ui.key_input(KeyCode::Escape, KeyAction::Release).unwrap();
        send(&mut ui, point(0.0, 98.0), PointerKind::Move);
        assert_eq!(bounds(&ui, &content)[axis], clicked);
        send(
            &mut ui,
            point(0.0, 98.0),
            PointerKind::Release(PointerButton::Primary),
        );
        assert_eq!(clicks.get(), 0);
        assert!(ui.next_deadline().is_none());
    }
}

#[test]
fn scrollbar_context_loss_and_content_shrink_end_effective_dragging() {
    let mut ui = ui(100.0, 100.0);
    let scroll = ui
        .mount_tree(Element::new(
            "scroll",
            ScrollArea::new(Element::new(
                "content",
                Container::default().size_limits(SizeLimits::at_least(Size::from([100, 500]))),
            )),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let content = ui.children(&scroll).unwrap()[0].clone();
    ui.pointer_input(PointerEvent::new(
        Point::new(98.0, 5.0).unwrap(),
        PointerKind::Press(PointerButton::Primary),
    ))
    .unwrap();
    ui.clear_focus();
    ui.pointer_input(PointerEvent::new(
        Point::new(98.0, 80.0).unwrap(),
        PointerKind::Move,
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(bounds(&ui, &content)[1], 0.0);
    ui.pointer_input(PointerEvent::new(
        Point::new(98.0, 80.0).unwrap(),
        PointerKind::Release(PointerButton::Primary),
    ))
    .unwrap();
    ui.pointer_input(PointerEvent::new(
        Point::new(98.0, 5.0).unwrap(),
        PointerKind::Press(PointerButton::Primary),
    ))
    .unwrap();
    ui.reconcile_children(&scroll, vec![Element::new("small", Label::new("Small"))])
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.pointer_input(PointerEvent::new(
        Point::new(98.0, 80.0).unwrap(),
        PointerKind::Move,
    ))
    .unwrap();
    ui.pointer_input(PointerEvent::new(
        Point::new(98.0, 80.0).unwrap(),
        PointerKind::Release(PointerButton::Primary),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(bounds(&ui, &ui.children(&scroll).unwrap()[0])[1], 0.0);
}

#[test]
fn redescribed_scroll_area_applies_axes_and_keeps_its_offset() {
    fn scroll(axes: ScrollAxes) -> Vec<Element> {
        vec![Element::new(
            "scroll",
            ScrollArea::new(Element::new(
                "column",
                Column::new((0..10).map(|n| button(&format!("item-{n}")))),
            ))
            .axes(axes),
        )]
    }
    let mut ui = ui(200.0, 80.0);
    let root = ui
        .mount_tree(Element::new(
            "root",
            View::new(scroll(ScrollAxes::Vertical).remove(0)),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let area = find(&ui, &root, "scroll").unwrap();
    let column = find(&ui, &root, "column").unwrap();
    ui.focus(&area).unwrap();
    ui.key_input(KeyCode::End, KeyAction::Press).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let offset = bounds(&ui, &column)[1];
    assert!(offset < 0.0);
    ui.reconcile_children(&root, scroll(ScrollAxes::Both))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(bounds(&ui, &column)[1], offset);
    // A fixed viewport clamps the retained offset to the origin.
    ui.reconcile_children(&root, scroll(ScrollAxes::None))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(bounds(&ui, &column)[1], 0.0);
    assert_eq!(find(&ui, &root, "scroll"), Some(area));
}

#[test]
fn redescribed_group_updates_accessible_name_padding_and_frame() {
    fn group(title: &str, padding: Option<u16>) -> Vec<Element> {
        let group = Group::titled(title, Element::new("body", Label::new("Content")));
        vec![Element::new(
            "group",
            match padding {
                Some(padding) => group.padding(Length::from(padding)).frameless(),
                None => group,
            },
        )]
    }
    fn name(ui: &Runtime) -> Option<String> {
        ui.inspect()
            .components
            .into_iter()
            .find(|item| item.key == "group")
            .and_then(|item| item.details.semantics)
            .and_then(|semantics| semantics.name)
    }
    let mut ui = ui(300.0, 200.0);
    let root = ui
        .mount_tree(Element::new(
            "root",
            View::new(group("Mixer", None).remove(0)),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let id = find(&ui, &root, "group").unwrap();
    assert_eq!(name(&ui).as_deref(), Some("Mixer"));
    let heading = find(&ui, &root, "heading").unwrap();
    assert!(bounds(&ui, &heading)[0] > 4.0);
    ui.reconcile_children(&root, group("Master", Some(4)))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(name(&ui).as_deref(), Some("Master"));
    assert_eq!(bounds(&ui, &heading)[0], 4.0);
    assert_eq!(find(&ui, &root, "group"), Some(id));
}

#[test]
fn redescribed_divider_applies_orientation_and_spacing() {
    fn divider(vertical: bool, spacing: Option<u16>) -> Vec<Element> {
        let divider = if vertical {
            Divider::new().vertical()
        } else {
            Divider::new()
        };
        vec![Element::new(
            "divider",
            match spacing {
                Some(spacing) => divider.spacing(Length::from(spacing)),
                None => divider,
            },
        )]
    }
    let mut ui = ui(300.0, 200.0);
    let root = ui
        .mount_tree(Element::new("root", Column::new(divider(false, None))))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let id = find(&ui, &root, "divider").unwrap();
    let [_, _, width, thickness] = bounds(&ui, &id);
    assert_eq!(width, 300.0);
    ui.reconcile_children(&root, divider(true, None)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(bounds(&ui, &id)[2], thickness);
    ui.reconcile_children(&root, divider(true, Some(0)))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(bounds(&ui, &id)[2] < thickness);
    assert_eq!(find(&ui, &root, "divider"), Some(id));
}

#[test]
fn redescribed_grid_applies_tracks_and_gap() {
    fn grid(columns: usize, gap: u16) -> Vec<Element> {
        vec![Element::new(
            "grid",
            Grid::new(
                (0..columns).map(|_| Track::fixed(Length::from(50))),
                [button("a"), button("b")],
            )
            .unwrap()
            .gap(Length::from(gap)),
        )]
    }
    let mut ui = ui(300.0, 200.0);
    let root = ui
        .mount_tree(Element::new("root", View::new(grid(1, 8).remove(0))))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let second = find(&ui, &root, "b").unwrap();
    let [x, y, ..] = bounds(&ui, &second);
    assert_eq!(x, 0.0);
    assert!(y > 0.0);
    ui.reconcile_children(&root, grid(2, 10)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let [x, y, ..] = bounds(&ui, &second);
    let first = bounds(&ui, &find(&ui, &root, "a").unwrap())[2];
    assert_eq!((x, y), (first + 10.0, 0.0));
    assert_eq!(find(&ui, &root, "b"), Some(second));
}

#[test]
fn calling_fixed_again_adds_no_second_viewport() {
    let mut ui = ui(120.0, 60.0);
    let root = ui
        .mount_tree(Element::new(
            "fixed",
            View::new(button("item")).fixed().fixed(),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let viewports = ui
        .inspect()
        .components
        .iter()
        .filter(|component| component.key == "fixed-viewport")
        .count();
    assert_eq!(viewports, 1);
    // The content still sits directly inside that one viewport.
    let item = find(&ui, &root, "item").unwrap();
    assert_eq!(bounds(&ui, &item)[1], 0.0);
}
