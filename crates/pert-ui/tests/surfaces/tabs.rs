//! Keyed tab retention, metadata and overflow behavior.
use pert_ui::*;
use std::{cell::Cell, rc::Rc, time::Duration};
fn ui(width: f32) -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(width, 120.0, 1.0).unwrap())
        .unwrap();
    ui
}
fn tab(key: &str) -> Tab {
    Tab::new(
        key,
        format!("View {key}"),
        Element::new("button", Button::new("Action", |_| Ok(()))),
    )
}
fn selected(ui: &Runtime) -> String {
    ui.inspect()
        .components
        .iter()
        .find_map(|item| match item.details.properties.get("selected") {
            Some(InspectValue::Text(value)) => Some(value.clone()),
            _ => None,
        })
        .unwrap()
}

#[test]
fn overflow_arrow_has_neutral_hover_background() {
    let mut ui = ui(100.0);
    ui.mount_tree(Element::new(
        "tabs",
        Tabs::new([tab("a"), tab("b"), tab("c")]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let color = ui
        .theme()
        .palette(ThemeMode::Dark)
        .unwrap()
        .color(ColorRole::SurfaceHover);
    let count = |ui: &mut Runtime| {
        ui.paint()
            .unwrap()
            .commands()
            .iter()
            .filter(|command| {
                matches!(command,
                    DrawCommand::FillRect { color: fill, .. }
                    | DrawCommand::DrawMask { color: fill, .. }
                    if *fill == color)
            })
            .count()
    };
    let resting = count(&mut ui);
    ui.pointer_input(PointerEvent::new(
        Point::new(90.0, 12.0).unwrap(),
        PointerKind::Move,
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(count(&mut ui) > resting);
}
#[test]
fn fixed_tab_reveals_nested_scroll_content_without_moving_the_viewport() {
    let mut ui = ui(200.0);
    let tabs = ui
        .mount_tree(Element::new(
            "tabs",
            Tabs::new([
                Tab::new(
                    "fixed",
                    "Fixed",
                    Element::new(
                        "scroll",
                        ScrollArea::new(Element::new(
                            "rows",
                            Column::new((0..30).map(|index| {
                                Element::new(format!("row-{index}"), Button::new("Row", |_| Ok(())))
                            })),
                        )),
                    ),
                )
                .fixed(),
                tab("other"),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let view = ui.children(&tabs).unwrap()[0].clone();
    let viewport = ui.children(&view).unwrap()[0].clone();
    let scroll = ui.children(&viewport).unwrap()[0].clone();
    let rows = ui.children(&scroll).unwrap()[0].clone();
    let last = ui.children(&rows).unwrap().last().unwrap().clone();
    let scroll_bounds = ui.bounds(&scroll).unwrap();
    assert_eq!(scroll_bounds, ui.bounds(&viewport).unwrap());
    ui.focus(&last).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let last_bounds = ui.bounds(&last).unwrap().unwrap().values();
    assert!(last_bounds[1] >= 0.0 && last_bounds[1] + last_bounds[3] <= 120.0);
    assert_eq!(ui.bounds(&scroll).unwrap(), scroll_bounds);
    ui.focus(&tabs).unwrap();
    for key in [KeyCode::End, KeyCode::Home] {
        ui.key_input(key, KeyAction::Press).unwrap();
        ui.key_input(key, KeyAction::Release).unwrap();
        ui.flush(Duration::ZERO).unwrap();
    }
    assert_eq!(ui.bounds(&last).unwrap().unwrap().values(), last_bounds);
    let report = ui.inspect();
    assert!(
        report
            .components
            .iter()
            .filter(|item| item.key == "tab-viewport")
            .all(|item| { item.details.properties["scroll_y"] == InspectValue::Number(0.0) })
    );
}

#[test]
fn arrows_select_reorder_preserves_identity_and_removal_chooses_neighbor() {
    let mut ui = ui(200.0);
    let tabs = ui
        .mount_tree(Element::new(
            "tabs",
            Tabs::new([tab("a"), tab("b"), tab("c")]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let original = ui.children(&tabs).unwrap();
    assert_eq!(selected(&ui), "a");
    ui.focus(&tabs).unwrap();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Press).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(selected(&ui), "b");
    ui.reconcile_children(
        &tabs,
        [tab("c"), tab("b"), tab("a")]
            .into_iter()
            .map(Tab::into_element)
            .collect(),
    )
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(selected(&ui), "b");
    assert_eq!(ui.children(&tabs).unwrap()[1], original[1]);
    ui.reconcile_children(
        &tabs,
        [tab("c"), tab("a")]
            .into_iter()
            .map(Tab::into_element)
            .collect(),
    )
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(selected(&ui), "a");
    ui.reconcile_children(
        &tabs,
        vec![
            Tab::new(
                "a",
                "Renamed",
                Element::new("button", Button::new("Action", |_| Ok(()))),
            )
            .into_element(),
        ],
    )
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(ui.children(&tabs).unwrap()[0], original[0]);
    assert_eq!(selected(&ui), "a");
    ui.paint().unwrap();
}
struct Live(Rc<Cell<usize>>);
impl Component for Live {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.0.set(self.0.get() + 1);
        cx.animate()
    }
}
#[test]
fn inactive_views_suspend_deadlines_and_resume_same_state() {
    let mut ui = ui(200.0);
    let ticks = Rc::new(Cell::new(0));
    let tabs = ui
        .mount_tree(Element::new(
            "tabs",
            Tabs::new([
                tab("static"),
                Tab::new("live", "Live", Element::new("meter", Live(ticks.clone()))),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(ticks.get(), 0);
    assert!(ui.next_deadline().is_none());
    ui.focus(&tabs).unwrap();
    ui.key_input(KeyCode::End, KeyAction::Press).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(ticks.get(), 1);
    assert!(ui.next_deadline().is_some());
    ui.key_input(KeyCode::Home, KeyAction::Press).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(ui.next_deadline().is_none());
    ui.flush(Duration::from_secs(5)).unwrap();
    assert_eq!(ticks.get(), 1);
    ui.key_input(KeyCode::End, KeyAction::Press).unwrap();
    ui.flush(Duration::from_secs(5)).unwrap();
    assert_eq!(ticks.get(), 2);
}
#[test]
fn ctrl_tab_bubbles_from_content_and_narrow_strip_reveals_selection() {
    let mut ui = ui(100.0);
    let tabs = ui
        .mount_tree(Element::new(
            "tabs",
            Tabs::new([tab("a"), tab("b"), tab("c")]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.focus(&tabs).unwrap();
    ui.focus_next(false).unwrap();
    assert_ne!(ui.focused(), Some(tabs.clone()));
    ui.key_input(KeyCode::ControlLeft, KeyAction::Press)
        .unwrap();
    ui.key_input(KeyCode::Tab, KeyAction::Press).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(selected(&ui), "b");
    assert_eq!(ui.focused(), Some(tabs.clone()));
    let scene = ui.paint().unwrap();
    assert!(scene.commands().iter().any(|command| matches!(command, DrawCommand::FillRect { rect, color } if *color == Color::ACCENT && rect.width() > 0.0 && rect.x() >= 28.0 && rect.x() + rect.width() <= 72.0)));
}

fn pointer(ui: &mut Runtime, x: f32, kind: PointerKind) {
    ui.pointer_input(PointerEvent::new(Point::new(x, 18.0).unwrap(), kind))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
}
fn compact_tab(key: &str) -> Tab {
    Tab::new(key, key, Element::new("content", Container::default()))
}
#[test]
fn escape_cancels_title_and_overflow_arrow_presses() {
    for width in [300.0, 100.0] {
        let mut ui = ui(width);
        ui.mount_tree(Element::new(
            "tabs",
            Tabs::new([compact_tab("a"), compact_tab("b"), compact_tab("c")]),
        ))
        .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        let before = ui.paint().unwrap();
        let x = if width == 300.0 { 90.0 } else { 86.0 };
        pointer(&mut ui, x, PointerKind::Press(PointerButton::Primary));
        ui.key_input(KeyCode::Escape, KeyAction::Press).unwrap();
        pointer(&mut ui, x, PointerKind::Release(PointerButton::Primary));
        assert_eq!(selected(&ui), "a");
        // Arrow cancellation keeps the selected underline at the original offset.
        let accents = |scene: &DisplayList| {
            scene
                .commands()
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::FillRect { rect, color }
                        if *color == Color::ACCENT && rect.y() == 33.0 && rect.height() == 3.0 =>
                    {
                        Some(*rect)
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(accents(&before), accents(&ui.paint().unwrap()));
    }
}
#[test]
fn removed_and_recreated_tab_does_not_receive_old_release() {
    let mut ui = ui(300.0);
    let tabs = ui
        .mount_tree(Element::new(
            "tabs",
            Tabs::new([compact_tab("a"), compact_tab("b")]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let original = ui.children(&tabs).unwrap()[1].clone();
    pointer(&mut ui, 90.0, PointerKind::Press(PointerButton::Primary));
    ui.reconcile_children(&tabs, vec![compact_tab("a").into_element()])
        .unwrap();
    ui.reconcile_children(
        &tabs,
        vec![
            compact_tab("a").into_element(),
            compact_tab("b").into_element(),
        ],
    )
    .unwrap();
    assert_ne!(ui.children(&tabs).unwrap()[1], original);
    pointer(&mut ui, 90.0, PointerKind::Release(PointerButton::Primary));
    assert_eq!(selected(&ui), "a");
}
#[test]
fn ctrl_tab_restores_local_focus_with_an_earlier_toolbar() {
    let mut ui = ui(300.0);
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new("toolbar", Button::new("Global action", |_| Ok(()))),
                Element::new("tabs", Tabs::new([tab("a"), tab("b"), tab("c")])),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let tabs = ui.children(&root).unwrap()[1].clone();
    ui.focus(&tabs).unwrap();
    ui.focus_next(false).unwrap();
    assert_ne!(ui.focused(), Some(tabs.clone()));
    ui.key_input(KeyCode::ControlLeft, KeyAction::Press)
        .unwrap();
    ui.key_input(KeyCode::Tab, KeyAction::Press).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(selected(&ui), "b");
    assert_eq!(ui.focused(), Some(tabs.clone()));
    ui.key_input(KeyCode::Tab, KeyAction::Release).unwrap();
    ui.key_input(KeyCode::Tab, KeyAction::Press).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(selected(&ui), "c");
}
struct PartialClip(Option<Element>);
impl Component for PartialClip {
    fn take_children(&mut self) -> Vec<Element> {
        self.0.take().into_iter().collect()
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        cx.place_clipped(
            0,
            Rect::new(0.0, 0.0, 300.0, 120.0).unwrap(),
            Rect::new(0.0, 0.0, 80.0, 120.0).unwrap(),
        )
    }
}
#[test]
fn captured_release_respects_ancestor_clip() {
    let mut ui = ui(300.0);
    ui.mount_tree(Element::new(
        "clip",
        PartialClip(Some(Element::new(
            "tabs",
            Tabs::new([compact_tab("a"), compact_tab("b")]),
        ))),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    pointer(&mut ui, 70.0, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, 90.0, PointerKind::Release(PointerButton::Primary));
    assert_eq!(selected(&ui), "a");
}

#[test]
fn newly_exposed_bound_content_is_updated_before_its_first_measurement() {
    let mut ui = ui(400.0);
    let value = ui.signal("Initial content".to_owned());
    let tabs = ui
        .mount_tree(Element::new(
            "tabs",
            Tabs::new([
                compact_tab("a"),
                Tab::new(
                    "b",
                    "b",
                    Element::new(
                        "split",
                        SplitPane::new(
                            SplitAxis::LeftRight,
                            Element::new("first", Label::new(value.read_only())),
                            Element::new("second", Label::new("Other pane")),
                        )
                        .sizing(SplitSizing::CONTENT_FIRST),
                    ),
                ),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    value
        .set("Fresh content changed while hidden".to_owned())
        .unwrap();
    ui.focus(&tabs).unwrap();
    ui.key_input(KeyCode::End, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::End, KeyAction::Release).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let split = ui.children(&tabs).unwrap()[1].clone();
    let label = ui.children(&split).unwrap()[0].clone();
    let width = ui.bounds(&label).unwrap().unwrap().width();
    assert!(width > 150.0, "stale hidden content measured as {width}");
    assert!(ui.next_deadline().is_none());
}

#[test]
fn horizontal_metric_edits_reveal_the_selected_header_without_changing_height() {
    for (role, value) in [
        (MetricRole::ControlPaddingX, 24),
        (MetricRole::ControlWidth, 150),
        (MetricRole::BodySize, 15),
    ] {
        let mut ui = ui(200.0);
        let tabs = ui
            .mount_tree(Element::new(
                "tabs",
                Tabs::new([tab("a"), tab("b"), tab("c")]),
            ))
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        ui.focus(&tabs).unwrap();
        ui.key_input(KeyCode::End, KeyAction::Press).unwrap();
        ui.key_input(KeyCode::End, KeyAction::Release).unwrap();
        ui.flush(Duration::ZERO).unwrap();
        let metrics = ui
            .theme()
            .metrics(Density::Standard)
            .unwrap()
            .with_metric(role, value.into())
            .unwrap();
        ui.theme().set_metrics(Density::Standard, metrics).unwrap();
        ui.flush(Duration::ZERO).unwrap();
        let scene = ui.paint().unwrap();
        assert_eq!(selected(&ui), "c");
        assert!(scene.commands().iter().any(|command| matches!(command, DrawCommand::DrawMask { rect, color, .. } | DrawCommand::FillRect { rect, color } if *color == Palette::dark().color(ColorRole::TabSelected) && rect.x() >= 32.0 && rect.x() < 168.0)), "{role:?}");
    }
}
#[test]
fn redescribed_strip_applies_detach_and_reorder_settings_and_keeps_selection() {
    fn strip(enabled: bool, moves: &Rc<Cell<usize>>) -> Vec<Element> {
        let tabs = Tabs::new([tab("a"), tab("b"), tab("c")]);
        let tabs = if enabled {
            let moves = moves.clone();
            tabs.detachable(true).reorderable(move |_, _| {
                moves.set(moves.get() + 1);
                Ok(false)
            })
        } else {
            tabs
        };
        vec![Element::new("tabs", tabs)]
    }
    fn flag(ui: &Runtime, name: &str) -> bool {
        ui.inspect()
            .components
            .iter()
            .find(|item| item.key == "tabs")
            .map(|item| item.details.properties[name] == InspectValue::Bool(true))
            .unwrap()
    }
    fn alt_right(ui: &mut Runtime) {
        ui.key_input(KeyCode::AltLeft, KeyAction::Press).unwrap();
        ui.key_input(KeyCode::ArrowRight, KeyAction::Press).unwrap();
        ui.key_input(KeyCode::ArrowRight, KeyAction::Release)
            .unwrap();
        ui.key_input(KeyCode::AltLeft, KeyAction::Release).unwrap();
        ui.flush(Duration::ZERO).unwrap();
    }
    let moves = Rc::new(Cell::new(0));
    let mut ui = ui(400.0);
    let root = ui
        .mount_tree(Element::new(
            "root",
            View::new(strip(false, &moves).remove(0)),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let tabs = ui.children(&root).unwrap()[0].clone();
    let views = ui.children(&tabs).unwrap();
    ui.focus(&tabs).unwrap();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Release)
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(selected(&ui), "b");
    assert!(!flag(&ui, "detachable") && !flag(&ui, "reorderable"));

    ui.reconcile_children(&root, strip(true, &moves)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(flag(&ui, "detachable") && flag(&ui, "reorderable"));
    assert_eq!(selected(&ui), "b");
    assert_eq!(ui.children(&tabs).unwrap(), views);
    alt_right(&mut ui);
    assert_eq!(moves.get(), 1);

    ui.reconcile_children(&root, strip(false, &moves)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(!flag(&ui, "detachable") && !flag(&ui, "reorderable"));
    assert_eq!(selected(&ui), "b");
    alt_right(&mut ui);
    assert_eq!(moves.get(), 1);
    assert_eq!(ui.children(&root).unwrap()[0], tabs);
}

#[test]
fn restore_uses_children_reconciled_before_the_next_update() {
    let mut ui = ui(200.0);
    let tabs = ui
        .mount_tree(Element::new("tabs", Tabs::new([tab("a")])))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let selected_b = Workspace::new(
        WorkspaceSettings::default(),
        [(
            WorkspacePath::new(["tabs"]).unwrap(),
            ViewState::new("tabs")
                .unwrap()
                .with("selected", WorkspaceValue::text("b").unwrap())
                .unwrap(),
        )],
    )
    .unwrap();
    ui.reconcile_children(
        &tabs,
        [tab("b"), tab("c")]
            .into_iter()
            .map(Tab::into_element)
            .collect(),
    )
    .unwrap();
    assert!(ui.restore_workspace(&selected_b).unwrap().is_complete());
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(selected(&ui), "b");
    ui.reconcile_children(&tabs, vec![tab("c").into_element()])
        .unwrap();
    let before = ui.capture_workspace().unwrap();
    let report = ui.restore_workspace(&selected_b).unwrap();
    assert!(
        report
            .rejected()
            .any(|(_, rejection)| *rejection == RestoreRejection::Unresolved("selected".into()))
    );
    assert_eq!(ui.capture_workspace().unwrap(), before);
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(selected(&ui), "c");
}

#[test]
fn restore_observes_detach_and_redock_before_parent_update() {
    let mut ui = ui(300.0);
    let tabs = ui
        .mount_tree(Element::new(
            "tabs",
            Tabs::new([tab("a"), tab("b")]).detachable(true),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let saved = Workspace::new(
        WorkspaceSettings::default(),
        [(
            WorkspacePath::new(["tabs"]).unwrap(),
            ViewState::new("tabs")
                .unwrap()
                .with("selected", WorkspaceValue::text("b").unwrap())
                .unwrap(),
        )],
    )
    .unwrap();
    let b = ui.children(&tabs).unwrap()[1].clone();
    assert!(
        ui.inspect()
            .components
            .iter()
            .any(|component| component.key == "b" && !component.visible)
    );
    assert!(ui.restore_workspace(&saved).unwrap().is_complete());
    ui.flush(Duration::ZERO).unwrap();
    let prepared = ui
        .prepare_detach(&b, NativeWindowOptions::new("B").unwrap())
        .unwrap();
    let window = ui.commit_detach(prepared).unwrap();
    let select_a = Workspace::new(
        WorkspaceSettings::default(),
        [(
            WorkspacePath::new(["tabs"]).unwrap(),
            ViewState::new("tabs")
                .unwrap()
                .with("selected", WorkspaceValue::text("a").unwrap())
                .unwrap(),
        )],
    )
    .unwrap();
    assert!(ui.restore_workspace(&select_a).unwrap().is_complete());
    let before = ui.capture_workspace().unwrap();
    assert!(
        ui.restore_workspace(&saved)
            .unwrap()
            .rejected()
            .any(|(_, reason)| *reason == RestoreRejection::Unresolved("selected".into()))
    );
    assert_eq!(ui.capture_workspace().unwrap(), before);
    ui.redock(&window).unwrap();
    assert!(ui.restore_workspace(&saved).unwrap().is_complete());
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(selected(&ui), "b");
}

#[test]
fn armed_tab_rejects_restore_to_a_fresh_child() {
    let mut ui = ui(300.0);
    let tabs = ui
        .mount_tree(Element::new("tabs", Tabs::new([tab("a")])))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    pointer(&mut ui, 40.0, PointerKind::Press(PointerButton::Primary));
    ui.reconcile_children(
        &tabs,
        vec![tab("a").into_element(), tab("b").into_element()],
    )
    .unwrap();
    let before = ui.capture_workspace().unwrap();
    let saved = Workspace::new(
        WorkspaceSettings::default(),
        [(
            WorkspacePath::new(["tabs"]).unwrap(),
            ViewState::new("tabs")
                .unwrap()
                .with("selected", WorkspaceValue::text("b").unwrap())
                .unwrap(),
        )],
    )
    .unwrap();
    assert!(
        ui.restore_workspace(&saved)
            .unwrap()
            .rejected()
            .any(|(_, reason)| *reason == RestoreRejection::Busy)
    );
    assert_eq!(ui.capture_workspace().unwrap(), before);
}

#[test]
fn invalid_tabs_are_rejected_before_anything_mounts_or_changes() {
    let content = || Element::new("button", Button::new("Action", |_| Ok(())));
    let cases: [(Vec<Tab>, LayoutError); 3] = [
        (vec![Tab::new("a", " ", content())], LayoutError::BlankLabel),
        (vec![Tab::new(" ", "A", content())], LayoutError::BlankKey),
        (
            vec![Tab::new("a", "A", content()), Tab::new("a", "B", content())],
            LayoutError::DuplicateKey,
        ),
    ];
    for (tabs, expected) in cases {
        let mut ui = ui(400.0);
        assert_eq!(
            ui.mount_tree(Element::new("tabs", Tabs::new(tabs))).err(),
            Some(RuntimeError::Layout(expected))
        );
        assert!(ui.inspect().components.is_empty());
    }

    let mut ui = ui(400.0);
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([Element::new("tabs", Tabs::new([tab("a"), tab("b")]))]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let tabs = ui.find_descendant(&root, "tabs").unwrap().unwrap();
    ui.reveal_child(&tabs, "b").unwrap();
    let children = ui.children(&tabs).unwrap();
    assert_eq!(selected(&ui), "b");
    let invalid = Element::new(
        "tabs",
        Tabs::new([tab("a"), Tab::new("b", "  ", content())]),
    );
    assert_eq!(
        ui.reconcile_children(&root, vec![invalid]),
        Err(RuntimeError::Layout(LayoutError::BlankLabel))
    );
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(ui.children(&tabs).unwrap(), children);
    assert_eq!(selected(&ui), "b");
    let visible = |key: &str| {
        ui.inspect()
            .components
            .iter()
            .any(|c| c.key == key && c.visible)
    };
    assert!(visible("b"));
    assert!(!visible("a"));
}
