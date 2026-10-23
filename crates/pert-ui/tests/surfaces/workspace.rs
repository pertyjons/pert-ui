//! Workspace persistence through the public facade: what a capture addresses,
//! what a document rejects, and what a restore does with every entry.
use pert_ui::*;
use std::{cell::Cell, rc::Rc, time::Duration};

#[test]
fn ambiguous_addresses_reject_capture_and_restore_before_settings_change() {
    let mut ui = runtime();
    ui.mount_tree(Element::new(
        "main",
        Column::new([
            Element::new(
                "scroll",
                ScrollArea::new(Element::new("body", Label::new("Body"))),
            )
            .part("same"),
            Element::new("label", Label::new("Other")).part("same"),
        ]),
    ))
    .unwrap();
    let error = WorkspaceError::DuplicatePath("main/same".into());
    assert_eq!(ui.capture_workspace(), Err(error.clone()));
    let saved = Workspace::new(
        WorkspaceSettings::default().with_theme_mode(Some(ThemeMode::Light)),
        [],
    )
    .unwrap();
    assert_eq!(ui.restore_workspace(&saved), Err(error));
    assert_eq!(ui.theme().mode().unwrap(), ThemeMode::Dark);
}

#[test]
fn malformed_duplicate_fields_and_invalid_public_maps_cannot_round_trip() {
    for (body, reason) in [
        ("settings mode=dark mode=light", "duplicate setting"),
        ("settings zoom=1.0 zoom=1.5", "duplicate setting"),
        (
            "settings density=compact density=standard",
            "duplicate setting",
        ),
        (
            "entry \"tabs\" tabs selected=\"a\" selected=\"b\"",
            "duplicate field",
        ),
        (
            "entry \"table\" table widths={\"a\"=1.0 \"a\"=2.0}",
            "duplicate map key",
        ),
    ] {
        assert_eq!(
            Workspace::decode(&format!("pert-ui-workspace 1\n{body}\n")),
            Err(WorkspaceError::Malformed(reason.into())),
            "{body}"
        );
    }
    assert!(matches!(
        WorkspaceValue::map([(String::new(), WorkspaceValue::flag(true))]),
        Err(WorkspaceError::InvalidName(_))
    ));
    let state = ViewState::new("custom")
        .unwrap()
        .with(
            "a",
            WorkspaceValue::text("a".repeat(Workspace::MAX_STRING_BYTES)).unwrap(),
        )
        .unwrap()
        .with(
            "b",
            WorkspaceValue::text("b".repeat(Workspace::MAX_STRING_BYTES)).unwrap(),
        )
        .unwrap();
    let entries = (0..Workspace::MAX_ENTRIES).map(|i| {
        (
            WorkspacePath::new([format!("item-{i}")]).unwrap(),
            state.clone(),
        )
    });
    assert_eq!(
        Workspace::new(WorkspaceSettings::default(), entries),
        Err(WorkspaceError::TooLarge)
    );
}

/// A one-entry document whose encoding is `room` bytes short of
/// `Workspace::MAX_DOCUMENT_BYTES` with default settings.
fn document_short_of_the_limit_by(room: usize) -> Workspace {
    let fill = "f".repeat(Workspace::MAX_STRING_BYTES - 9);
    let build = |last: usize| {
        let mut state = ViewState::new("k").unwrap();
        for index in 0..4095 {
            state = state
                .with(format!("k{index:04}"), WorkspaceValue::text(&fill).unwrap())
                .unwrap();
        }
        state = state
            .with("k4095", WorkspaceValue::text("l".repeat(last)).unwrap())
            .unwrap();
        Workspace::new(
            WorkspaceSettings::default(),
            [(WorkspacePath::new(["x"]).unwrap(), state)],
        )
        .unwrap()
    };
    let last = Workspace::MAX_DOCUMENT_BYTES - room - build(0).encode().len();
    let workspace = build(last);
    assert_eq!(
        workspace.encode().len(),
        Workspace::MAX_DOCUMENT_BYTES - room
    );
    workspace
}

#[test]
fn settings_that_would_overflow_the_document_are_refused_and_the_rest_round_trips() {
    let light = WorkspaceSettings::default().with_theme_mode(Some(ThemeMode::Light));
    let line = "settings mode=light\n".len();

    // At the limit, the settings line would make the document unloadable.
    let mut full = document_short_of_the_limit_by(0);
    assert_eq!(full.set_settings(light), Err(WorkspaceError::TooLarge));
    assert_eq!(*full.settings(), WorkspaceSettings::default(), "kept");
    assert_eq!(Workspace::decode(&full.encode()), Ok(full.clone()));

    // With exactly the line's room, the settings are accepted and the
    // document at the limit still loads.
    let mut fits = document_short_of_the_limit_by(line);
    fits.set_settings(light).unwrap();
    assert_eq!(fits.encode().len(), Workspace::MAX_DOCUMENT_BYTES);
    let decoded = Workspace::decode(&fits.encode()).unwrap();
    assert_eq!(decoded, fits);
    assert_eq!(*decoded.settings(), light);
}

#[test]
fn failed_mount_and_reconcile_do_not_consume_pending_restore() {
    struct Fail;
    impl Component for Fail {
        fn mounted(&mut self) {
            panic!("mount failure");
        }
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
    }
    fn child(fail: bool) -> Element {
        let body = if fail {
            Element::new("body", Fail)
        } else {
            Element::new("body", Label::new("OK"))
        };
        Element::new("scroll", ScrollArea::new(body))
    }
    for reconcile in [false, true] {
        let mut ui = runtime();
        let parent = if reconcile {
            Some(
                ui.mount_tree(Element::new("main", Column::default()))
                    .unwrap(),
            )
        } else {
            None
        };
        let path = if reconcile {
            vec!["main", "scroll"]
        } else {
            vec!["scroll"]
        };
        let saved = Workspace::new(
            WorkspaceSettings::default(),
            [(
                WorkspacePath::new(path).unwrap(),
                ViewState::new("scroll")
                    .unwrap()
                    .with("x", WorkspaceValue::number(12.0).unwrap())
                    .unwrap()
                    .with("y", WorkspaceValue::number(34.0).unwrap())
                    .unwrap(),
            )],
        )
        .unwrap();
        let initial = ui.restore_workspace(&saved).unwrap();
        assert_eq!(initial.pending().count(), 1);
        let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if let Some(parent) = &parent {
                ui.reconcile_children(parent, vec![child(true)]).unwrap();
            } else {
                ui.mount_tree(child(true)).unwrap();
            }
        }));
        assert!(failed.is_err());
        assert_eq!(ui.workspace_report(), &initial);
        if let Some(parent) = &parent {
            ui.reconcile_children(parent, vec![child(false)]).unwrap();
        } else {
            ui.mount_tree(child(false)).unwrap();
        }
        assert!(ui.workspace_report().is_complete());
        assert_eq!(
            ui.capture_workspace()
                .unwrap()
                .entries()
                .next()
                .unwrap()
                .1
                .number("y"),
            Ok(34.0)
        );
    }
}

#[test]
fn empty_tabs_restore_their_own_capture() {
    let mut ui = runtime();
    ui.mount_tree(Element::new("tabs", Tabs::new([]))).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let saved = ui.capture_workspace().unwrap();
    assert!(ui.restore_workspace(&saved).unwrap().is_complete());
}

#[test]
fn a_busy_split_captures_only_committed_preferences() {
    let mut ui = runtime();
    let split = ui
        .mount_tree(Element::new(
            "split",
            SplitPane::new(
                SplitAxis::LeftRight,
                Element::new("a", Label::new("A")),
                Element::new("b", Label::new("B")),
            ),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let original = ui.capture_workspace().unwrap();
    let pointer = |ui: &mut Runtime, x, kind| {
        ui.pointer_input(PointerEvent::new(Point::new(x, 20.0).unwrap(), kind))
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
    };
    pointer(&mut ui, 200.0, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, 250.0, PointerKind::Move);
    assert_eq!(ui.capture_workspace().unwrap(), original);
    assert!(
        ui.restore_workspace(&original)
            .unwrap()
            .rejected()
            .any(|(_, reason)| *reason == RestoreRejection::Busy)
    );
    pointer(&mut ui, 250.0, PointerKind::Release(PointerButton::Primary));
    let committed = ui.capture_workspace().unwrap();
    assert_ne!(committed, original);
    ui.focus(&split).unwrap();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Repeat)
        .unwrap();
    assert_eq!(ui.capture_workspace().unwrap(), committed);
    assert!(
        ui.restore_workspace(&original)
            .unwrap()
            .rejected()
            .any(|(_, reason)| *reason == RestoreRejection::Busy)
    );
    ui.key_input(KeyCode::Escape, KeyAction::Press).unwrap();
    assert_eq!(ui.capture_workspace().unwrap(), committed);
}

#[test]
fn floating_window_restore_observes_authoritative_status_before_flush() {
    let mut ui = runtime();
    let window = FloatingWindow::new(
        &ui,
        "window",
        "Window",
        Element::new("body", Label::new("Body")),
    )
    .initial_size(Size::from([200, 100]))
    .unwrap()
    .minimizable(true);
    let state = window.controller();
    ui.mount_tree(Element::new("layer", WindowLayer::new([window])))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let saved = ui.capture_workspace().unwrap();
    state.minimize().unwrap();
    assert_ne!(
        ui.capture_workspace().unwrap(),
        saved,
        "capture reads authoritative status"
    );
    assert!(ui.restore_workspace(&saved).unwrap().is_complete());
    assert_eq!(state.status().get().unwrap(), WindowStatus::Open);
    state.set_position(Point::new(70.0, 80.0).unwrap()).unwrap();
    state.close().unwrap();
    let closed = ui.capture_workspace().unwrap();
    assert!(
        ui.restore_workspace(&saved)
            .unwrap()
            .rejected()
            .any(|(_, reason)| matches!(reason, RestoreRejection::Unresolved(_)))
    );
    assert_eq!(
        ui.capture_workspace().unwrap(),
        closed,
        "rejection must not mutate placement"
    );
}

fn runtime() -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 400.0, 1.0).unwrap())
        .unwrap();
    ui
}

#[test]
fn tabs_and_scroll_refuse_restore_during_pointer_ownership() {
    let mut ui = runtime();
    ui.mount_tree(tabs()).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let saved = ui.capture_workspace().unwrap();
    ui.pointer_input(PointerEvent::new(
        Point::new(20.0, 10.0).unwrap(),
        PointerKind::Press(PointerButton::Primary),
    ))
    .unwrap();
    assert!(
        ui.restore_workspace(&saved)
            .unwrap()
            .rejected()
            .any(|(_, reason)| *reason == RestoreRejection::Busy)
    );

    let mut ui = runtime();
    ui.mount_tree(Element::new(
        "scroll",
        ScrollArea::new(Element::new(
            "content",
            Container::default().size_limits(SizeLimits::at_least(Size::from([400, 1000]))),
        )),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let saved = ui.capture_workspace().unwrap();
    ui.pointer_input(PointerEvent::new(
        Point::new(398.0, 5.0).unwrap(),
        PointerKind::Press(PointerButton::Primary),
    ))
    .unwrap();
    assert!(
        ui.restore_workspace(&saved)
            .unwrap()
            .rejected()
            .any(|(_, reason)| *reason == RestoreRejection::Busy)
    );
    ui.pointer_input(PointerEvent::new(
        Point::new(398.0, 5.0).unwrap(),
        PointerKind::Release(PointerButton::Primary),
    ))
    .unwrap();
    assert!(ui.restore_workspace(&saved).unwrap().is_complete());
}

#[test]
fn an_ambiguous_late_mount_never_consumes_state_into_one_arbitrary_node() {
    let mut ui = runtime();
    let state = ViewState::new("scroll")
        .unwrap()
        .with("x", WorkspaceValue::number(12.0).unwrap())
        .unwrap()
        .with("y", WorkspaceValue::number(34.0).unwrap())
        .unwrap();
    let saved = Workspace::new(
        WorkspaceSettings::default(),
        [(WorkspacePath::new(["main", "same"]).unwrap(), state)],
    )
    .unwrap();
    let _report = ui.restore_workspace(&saved).unwrap();
    ui.mount_tree(Element::new(
        "main",
        Column::new([
            Element::new(
                "a",
                ScrollArea::new(Element::new("body", Label::new("Body"))),
            )
            .part("same"),
            Element::new("b", Label::new("Other")).part("same"),
        ]),
    ))
    .unwrap();
    assert_eq!(ui.workspace_report().applied().count(), 0);
    assert_eq!(ui.workspace_report().ignored().count(), 1);
    assert!(
        ui.workspace_report()
            .ignored()
            .all(|(_, reason)| *reason == IgnoredEntry::AmbiguousPath)
    );
}

fn paths(workspace: &Workspace) -> Vec<String> {
    workspace
        .entries()
        .map(|(path, _)| path.to_string())
        .collect()
}

fn entry(workspace: &Workspace, path: &str) -> ViewState {
    workspace
        .entries()
        .find(|(candidate, _)| candidate.to_string() == path)
        .unwrap_or_else(|| panic!("no entry at {path} among {:?}", paths(workspace)))
        .1
        .clone()
}

fn tabs() -> Element {
    Element::new(
        "tabs",
        Tabs::new([
            Tab::new("overview", "Overview", Element::new("a", Label::new("A"))),
            Tab::new("scope", "Scope", Element::new("b", Label::new("B"))),
        ]),
    )
}

fn visible(ui: &Runtime, key: &str) -> bool {
    ui.inspect()
        .components
        .iter()
        .find(|component| component.key == key)
        .map(|component| component.visible)
        .unwrap_or_else(|| panic!("no component keyed {key}"))
}

#[test]
fn captures_only_addressable_participants_and_repeats_byte_for_byte() {
    let mut ui = runtime();
    ui.mount_tree(Element::new(
        "main",
        Column::new([
            tabs(),
            Element::new(
                "split",
                SplitPane::new(
                    SplitAxis::LeftRight,
                    Element::new("left", Label::new("Left")),
                    Element::new("right", Label::new("Right")),
                ),
            ),
            Element::new(
                "scroll",
                ScrollArea::new(Element::new("c", Label::new("C"))),
            ),
        ]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let workspace = ui.capture_workspace().unwrap();
    assert_eq!(
        paths(&workspace),
        [
            "main/scroll",
            "main/split",
            "main/tabs",
            // Each tab's own scrolling viewport, named by Tabs rather than
            // carrying its internal key.
            "main/tabs/overview/viewport",
            "main/tabs/scope/viewport",
        ],
        "labels and layout containers are not participants",
    );
    assert_eq!(entry(&workspace, "main/tabs").kind(), "tabs");
    assert_eq!(
        entry(&workspace, "main/tabs").text("selected"),
        Ok("overview")
    );
    assert_eq!(workspace.encode(), ui.capture_workspace().unwrap().encode());
    assert_eq!(Workspace::decode(&workspace.encode()).unwrap(), workspace);
}

#[test]
fn library_internal_structure_stays_out_of_addresses() {
    fn frame(top: bool) -> Element {
        let mut layout = RegionLayout::new()
            .left(Element::new("panel", Label::new("Left")))
            .center(Element::new("editor", Label::new("Center")));
        if top {
            layout = layout.top(Element::new("bar", Label::new("Top")));
        }
        Element::new("frame", layout)
    }
    let capture = |top: bool| {
        let mut ui = runtime();
        ui.mount_tree(frame(top)).unwrap();
        ui.flush(Duration::ZERO).unwrap();
        paths(&ui.capture_workspace().unwrap())
    };
    let without_top = capture(false);
    let with_top = capture(true);
    for path in without_top.iter().chain(&with_top) {
        assert!(
            !path.contains("split-") && !path.contains("lone-region") && !path.contains("regions"),
            "internal key in {path}",
        );
    }
    assert!(
        without_top.contains(&"frame/left-divider".to_owned()),
        "{without_top:?}"
    );
    assert!(
        with_top.contains(&"frame/left-divider".to_owned()),
        "a divider's address must not depend on how many other regions exist: {with_top:?}",
    );
    assert!(
        with_top.contains(&"frame/top-divider".to_owned()),
        "{with_top:?}"
    );
    assert!(
        with_top.contains(&"frame/center/viewport".to_owned()),
        "{with_top:?}"
    );
}

#[test]
fn slots_keep_two_regions_from_colliding_on_one_child_key() {
    let mut ui = runtime();
    ui.mount_tree(Element::new(
        "frame",
        RegionLayout::new()
            .left(Element::new(
                "panel",
                ScrollArea::new(Element::new("a", Label::new("A"))),
            ))
            .center(Element::new(
                "panel",
                ScrollArea::new(Element::new("b", Label::new("B"))),
            )),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let captured = paths(&ui.capture_workspace().unwrap());
    assert!(
        captured.contains(&"frame/left/panel".to_owned()),
        "{captured:?}"
    );
    assert!(
        captured.contains(&"frame/center/panel".to_owned()),
        "{captured:?}"
    );
}

#[test]
fn restores_a_selection_a_proportion_and_an_offset() {
    let mut ui = runtime();
    ui.mount_tree(Element::new(
        "main",
        Column::new([
            tabs(),
            Element::new(
                "split",
                SplitPane::new(
                    SplitAxis::LeftRight,
                    Element::new("left", Label::new("Left")),
                    Element::new("right", Label::new("Right")),
                ),
            ),
        ]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(visible(&ui, "a") && !visible(&ui, "b"));
    let document = concat!(
        "pert-ui-workspace 1\n",
        "entry \"main/split\" split extent=0.25 rule=\"ratio\"\n",
        "entry \"main/tabs\" tabs selected=\"scope\"\n",
    );
    let report = ui
        .restore_workspace(&Workspace::decode(document).unwrap())
        .unwrap();
    assert!(report.is_complete(), "{report:?}");
    assert_eq!(report.applied().count(), 2);
    ui.flush(Duration::ZERO).unwrap();
    assert!(
        !visible(&ui, "a") && visible(&ui, "b"),
        "the saved tab is selected"
    );
    let captured = ui.capture_workspace().unwrap();
    assert_eq!(entry(&captured, "main/split").number("extent"), Ok(0.25));
    assert_eq!(entry(&captured, "main/tabs").text("selected"), Ok("scope"));
}

#[test]
fn a_restored_split_extent_captures_back_exactly() {
    // Neither value is representable in f32; the preference must not widen
    // 0.65 to 0.6499999761581421 on the way back into the document.
    for (rule, extent) in [("ratio", 0.65), ("first", 120.3), ("second", 0.1)] {
        let mut ui = runtime();
        ui.mount_tree(Element::new(
            "main",
            SplitPane::new(
                SplitAxis::LeftRight,
                Element::new("left", Label::new("Left")),
                Element::new("right", Label::new("Right")),
            ),
        ))
        .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        let document =
            format!("pert-ui-workspace 1\nentry \"main\" split extent={extent} rule=\"{rule}\"\n");
        let report = ui
            .restore_workspace(&Workspace::decode(&document).unwrap())
            .unwrap();
        assert!(report.is_complete(), "{report:?}");
        ui.flush(Duration::ZERO).unwrap();
        let captured = ui.capture_workspace().unwrap();
        assert_eq!(entry(&captured, "main").number("extent"), Ok(extent));
        assert_eq!(entry(&captured, "main").text("rule"), Ok(rule));
    }
}

#[test]
fn structure_drift_is_reported_per_entry_while_the_rest_applies() {
    let mut ui = runtime();
    ui.mount_tree(Element::new("main", Column::new([tabs()])))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let document = concat!(
        "pert-ui-workspace 1\n",
        "entry \"main/tabs\" tabs selected=\"scope\"\n",
        "entry \"main/gone\" scroll x=0.0 y=12.0\n",
        "entry \"main\" split extent=0.5 rule=\"ratio\"\n",
    );
    let report = ui
        .restore_workspace(&Workspace::decode(document).unwrap())
        .unwrap();
    assert_eq!(
        report
            .applied()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["main/tabs"],
    );
    assert_eq!(
        report
            .pending()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["main/gone"],
        "a path that is not mounted may still arrive",
    );
    let ignored: Vec<_> = report
        .ignored()
        .map(|(path, reason)| (path.to_string(), reason.clone()))
        .collect();
    assert_eq!(
        ignored,
        [("main".to_owned(), IgnoredEntry::NotAParticipant)]
    );
}

#[test]
fn a_participant_rejects_what_it_cannot_resolve_without_changing_itself() {
    let mut ui = runtime();
    ui.mount_tree(Element::new("main", Column::new([tabs()])))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    for (document, expected) in [
        (
            "entry \"main/tabs\" tabs selected=\"removed\"\n",
            RestoreRejection::Unresolved("selected".to_owned()),
        ),
        (
            "entry \"main/tabs\" tabs future=true selected=\"scope\"\n",
            RestoreRejection::UnknownValue("future".to_owned()),
        ),
        (
            "entry \"main/tabs\" tabs selected=3\n",
            RestoreRejection::Value("selected".to_owned()),
        ),
    ] {
        let text = format!("pert-ui-workspace 1\n{document}");
        let report = ui
            .restore_workspace(&Workspace::decode(&text).unwrap())
            .unwrap();
        let rejected: Vec<_> = report
            .rejected()
            .map(|(path, reason)| (path.to_string(), reason.clone()))
            .collect();
        assert_eq!(rejected, [("main/tabs".to_owned(), expected)], "{text}");
        ui.flush(Duration::ZERO).unwrap();
        assert!(visible(&ui, "a"), "a rejection leaves the component alone");
    }
}

#[test]
fn a_kind_that_no_longer_matches_is_ignored_rather_than_applied() {
    let mut ui = runtime();
    ui.mount_tree(Element::new("main", Column::new([tabs()])))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let document = "pert-ui-workspace 1\nentry \"main/tabs\" split extent=0.25 rule=\"ratio\"\n";
    let report = ui
        .restore_workspace(&Workspace::decode(document).unwrap())
        .unwrap();
    let ignored: Vec<_> = report.ignored().map(|(_, reason)| reason.clone()).collect();
    assert_eq!(
        ignored,
        [IgnoredEntry::KindMismatch {
            found: "split".to_owned(),
            mounted: "tabs".to_owned(),
        }],
    );
}

#[test]
fn a_later_mount_takes_its_entry_once_and_only_once() {
    let mut ui = runtime();
    let root = ui
        .mount_tree(Element::new("main", Column::new([])))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let document = "pert-ui-workspace 1\nentry \"main/late\" scroll x=0.0 y=40.0\n";
    let report = ui
        .restore_workspace(&Workspace::decode(document).unwrap())
        .unwrap();
    assert_eq!(report.pending().count(), 1);
    let late = || {
        Element::new(
            "late",
            ScrollArea::new(Element::new("content", Label::new("Tall"))),
        )
    };
    ui.reconcile_children(&root, vec![late()]).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(
        ui.workspace_report()
            .applied()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["main/late"],
    );
    assert_eq!(ui.workspace_report().pending().count(), 0);
    // Removing and recreating the view must not resurrect the saved offset: the
    // entry was spent, and live state is authoritative from then on.
    ui.reconcile_children(&root, Vec::new()).unwrap();
    ui.reconcile_children(&root, vec![late()]).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(ui.workspace_report().applied().count(), 1);
}

#[test]
fn a_view_that_mounts_during_a_drag_reports_its_entry_busy() {
    let mut ui = runtime();
    let strip = || {
        Element::new(
            "tabs",
            Tabs::new([
                Tab::new("a", "Tab a", Element::new("a", Label::new("A"))),
                Tab::new("b", "Tab b", Element::new("b", Label::new("B"))),
            ])
            .reorderable(|_, _| Ok(false)),
        )
    };
    let root = ui
        .mount_tree(Element::new("main", Column::new([strip()])))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let document = "pert-ui-workspace 1\nentry \"main/late\" scroll x=0.0 y=40.0\n";
    let report = ui
        .restore_workspace(&Workspace::decode(document).unwrap())
        .unwrap();
    assert_eq!(report.pending().count(), 1);
    // Dragging the first title past the threshold starts a drag session.
    for kind in [
        PointerKind::Press(PointerButton::Primary),
        PointerKind::Move,
    ] {
        let x = if kind == PointerKind::Move { 16.0 } else { 8.0 };
        ui.pointer_input(PointerEvent::new(Point::new(x, 18.0).unwrap(), kind))
            .unwrap();
        ui.flush(ui.now()).unwrap();
    }
    assert!(ui.drag_active());
    let late = Element::new(
        "late",
        ScrollArea::new(Element::new("content", Label::new("Tall"))),
    );
    ui.reconcile_children(&root, vec![strip(), late]).unwrap();
    ui.flush(ui.now()).unwrap();
    let report = ui.workspace_report();
    assert_eq!(
        report
            .rejected()
            .map(|(path, reason)| (path.to_string(), reason.clone()))
            .collect::<Vec<_>>(),
        [("main/late".to_owned(), RestoreRejection::Busy)],
    );
    assert_eq!(report.applied().count(), 0);
    assert_eq!(report.pending().count(), 0);
}

#[test]
fn a_document_that_violates_the_schema_changes_nothing() {
    let mut ui = runtime();
    ui.mount_tree(Element::new("main", Column::new([tabs()])))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let before = ui.capture_workspace().unwrap();
    for (text, expected) in [
        (
            "pert-ui-workspace 2\nentry \"main/tabs\" tabs selected=\"scope\"\n",
            WorkspaceError::UnsupportedVersion {
                found: 2,
                supported: 1,
            },
        ),
        (
            "pert-ui-workspace 1\nentry \"main/tabs\" tabs selected=\"scope\n",
            WorkspaceError::Malformed("unterminated text".into()),
        ),
        (
            "pert-ui-workspace 1\nentry \"main/tabs\" tabs selected=\"a\"\nentry \"main/tabs\" tabs selected=\"b\"\n",
            WorkspaceError::DuplicatePath("main/tabs".into()),
        ),
    ] {
        assert_eq!(Workspace::decode(text), Err(expected), "{text}");
    }
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(ui.capture_workspace().unwrap(), before);
}

#[test]
fn saved_geometry_is_a_preference_that_layout_clamps() {
    let mut ui = runtime();
    ui.mount_tree(Element::new(
        "main",
        Column::new([Element::new(
            "scroll",
            ScrollArea::new(Element::new("c", Label::new("C"))),
        )
        .part("kept")]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let document = "pert-ui-workspace 1\nentry \"main/kept\" scroll x=0.0 y=100000.0\n";
    let report = ui
        .restore_workspace(&Workspace::decode(document).unwrap())
        .unwrap();
    assert!(report.is_complete(), "{report:?}");
    ui.flush(Duration::ZERO).unwrap();
    let captured = ui.capture_workspace().unwrap();
    let offset = entry(&captured, "main/kept").number("y").unwrap();
    assert!(
        offset < 100_000.0,
        "an offset saved against a taller viewport clamps to this one: {offset}",
    );
}

#[test]
fn settings_travel_with_the_document() {
    let mut ui = runtime();
    ui.mount_tree(Element::new("main", Column::new([])))
        .unwrap();
    ui.theme().set_mode(ThemeMode::Light).unwrap();
    ui.theme().set_density(Density::Compact).unwrap();
    ui.ui_settings()
        .set_zoom(UiZoom::new(1.25).unwrap())
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let workspace = ui.capture_workspace().unwrap();

    let mut other = runtime();
    other
        .mount_tree(Element::new("main", Column::new([])))
        .unwrap();
    let report = other.restore_workspace(&workspace).unwrap();
    assert!(report.is_complete(), "{report:?}");
    assert_eq!(other.theme().mode().unwrap(), ThemeMode::Light);
    assert_eq!(other.theme().density().unwrap(), Density::Compact);
    assert_eq!(other.ui_settings().zoom().unwrap().get(), 1.25);

    // A host that owns these itself drops them and keeps the arrangement.
    let mut third = runtime();
    third
        .mount_tree(Element::new("main", Column::new([])))
        .unwrap();
    let mut without = workspace.clone();
    without.set_settings(WorkspaceSettings::default()).unwrap();
    let report = third.restore_workspace(&without).unwrap();
    assert!(report.is_complete(), "{report:?}");
    assert_eq!(third.theme().mode().unwrap(), ThemeMode::Dark);
}

#[test]
fn restoring_a_hidden_view_schedules_no_work_for_it() {
    let mut ui = runtime();
    ui.mount_tree(Element::new(
        "main",
        Tabs::new([
            Tab::new("first", "First", Element::new("a", Label::new("A"))),
            Tab::new(
                "second",
                "Second",
                Element::new(
                    "hidden",
                    ScrollArea::new(Element::new("b", Label::new("B"))),
                ),
            ),
        ]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(!visible(&ui, "hidden"));
    let document = "pert-ui-workspace 1\nentry \"main/second/hidden\" scroll x=0.0 y=8.0\n";
    let report = ui
        .restore_workspace(&Workspace::decode(document).unwrap())
        .unwrap();
    assert!(report.is_complete(), "{report:?}");
    let flush = ui.flush(Duration::ZERO).unwrap();
    assert_eq!(
        flush.next_deadline, None,
        "no frame was requested for hidden content"
    );
    assert_eq!(
        ui.inspect()
            .components
            .iter()
            .find(|component| component.key == "hidden")
            .and_then(|component| component.dirty),
        Some(Invalidation::Layout),
        "the hidden view retains the work until it is revealed",
    );
}

#[test]
fn ordinary_keys_a_document_cannot_hold_do_not_break_capture_or_restore() {
    let mut ui = runtime();
    let long = "/home/user/projects/".repeat(20);
    let deep = (0..80).fold(Element::new("leaf", Label::new("Deep")), |child, depth| {
        Element::new(format!("level-{depth}"), Column::new([child]))
    });
    ui.mount_tree(Element::new(
        "main",
        Column::new([
            Element::new(long, Label::new("A long row key")),
            Element::new("tab\there", Label::new("A control character")),
            Element::new("deep", Column::new([deep])),
            Element::new(
                "scroll",
                ScrollArea::new(Element::new("body", Label::new("Body"))),
            ),
        ]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let workspace = ui.capture_workspace().unwrap();
    assert_eq!(paths(&workspace), ["main/scroll"]);
    let report = ui.restore_workspace(&workspace).unwrap();
    assert_eq!(report.applied().count(), 1);
    assert!(report.is_complete());
}

#[test]
fn a_participant_whose_key_a_document_cannot_hold_rejects_capture() {
    let mut ui = runtime();
    ui.mount_tree(Element::new(
        "main",
        Column::new([Element::new(
            "tab\there",
            ScrollArea::new(Element::new("body", Label::new("Body"))),
        )]),
    ))
    .unwrap();
    assert!(matches!(
        ui.capture_workspace(),
        Err(WorkspaceError::ControlCharacter)
    ));
}

/// A participant whose state fails from its `fail_from`-th call onwards, so
/// one test can fail the classification and another only the encoding.
struct Fragile {
    calls: Rc<Cell<usize>>,
    fail_from: usize,
}
impl Component for Fragile {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(10.0, 10.0)?)
    }
    fn workspace_state(&self) -> Result<Option<ViewState>, WorkspaceError> {
        let call = self.calls.get();
        self.calls.set(call + 1);
        if call >= self.fail_from {
            return Err(WorkspaceError::TextTooLong);
        }
        Ok(Some(ViewState::new("fragile")?))
    }
    fn restore_workspace_state(&mut self, _: &ViewState) -> Result<Invalidation, RestoreRejection> {
        Ok(Invalidation::Paint)
    }
}
fn fragile(fail_from: usize) -> (Element, Rc<Cell<usize>>) {
    let calls = Rc::new(Cell::new(0));
    let element = Element::new(
        "main",
        Column::new([Element::new(
            "custom",
            Fragile {
                calls: calls.clone(),
                fail_from,
            },
        )]),
    );
    (element, calls)
}
fn participant<T: std::fmt::Debug>(result: Result<T, WorkspaceError>) -> (String, WorkspaceError) {
    match result {
        Err(WorkspaceError::Participant { path, source }) => (path, *source),
        other => panic!("expected a participant failure, got {other:?}"),
    }
}

#[test]
fn a_participant_failing_to_classify_fails_capture_and_restore_by_path() {
    let mut ui = runtime();
    let (element, _) = fragile(0);
    ui.mount_tree(element).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let expected = ("main/custom".to_owned(), WorkspaceError::TextTooLong);
    assert_eq!(participant(ui.capture_workspace()), expected);
    // Restore refuses before any setting changes.
    let saved = Workspace::new(
        WorkspaceSettings::default().with_theme_mode(Some(ThemeMode::Light)),
        [],
    )
    .unwrap();
    assert_eq!(participant(ui.restore_workspace(&saved)), expected);
    assert_eq!(ui.theme().mode().unwrap(), ThemeMode::Dark);
}

#[test]
fn capture_asks_each_participant_once_and_fails_on_that_answer() {
    let mut ui = runtime();
    let (element, calls) = fragile(1);
    ui.mount_tree(element).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    // Classification and encoding are one answer: a second call that would
    // fail, or answer differently, is never made.
    assert_eq!(paths(&ui.capture_workspace().unwrap()), ["main/custom"]);
    assert_eq!(calls.get(), 1);
    let mut ui = runtime();
    let (element, _) = fragile(0);
    ui.mount_tree(element).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(
        participant(ui.capture_workspace()),
        ("main/custom".to_owned(), WorkspaceError::TextTooLong)
    );
}

/// A participant whose answers come from a closure called with the number of
/// earlier calls, for answers that change between calls.
struct Scripted {
    calls: Rc<Cell<usize>>,
    answer: Box<dyn Fn(usize) -> Result<Option<ViewState>, WorkspaceError>>,
}
impl Component for Scripted {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(10.0, 10.0)?)
    }
    fn workspace_state(&self) -> Result<Option<ViewState>, WorkspaceError> {
        let call = self.calls.get();
        self.calls.set(call + 1);
        (self.answer)(call)
    }
    fn restore_workspace_state(&mut self, _: &ViewState) -> Result<Invalidation, RestoreRejection> {
        Ok(Invalidation::Paint)
    }
}
fn scripted(
    answer: impl Fn(usize) -> Result<Option<ViewState>, WorkspaceError> + 'static,
) -> (Element, Rc<Cell<usize>>) {
    let calls = Rc::new(Cell::new(0));
    let element = Element::new(
        "main",
        Column::new([Element::new(
            "custom",
            Scripted {
                calls: calls.clone(),
                answer: Box::new(answer),
            },
        )]),
    );
    (element, calls)
}

#[test]
fn a_participant_that_stops_participating_mid_capture_is_still_captured() {
    let mut ui = runtime();
    // Breaks the stable-answer rule: participates once, then not.
    let (element, _) =
        scripted(|call| Ok((call == 0).then(|| ViewState::new("scripted").unwrap())));
    ui.mount_tree(element).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(paths(&ui.capture_workspace().unwrap()), ["main/custom"]);
}

#[test]
fn explicit_restore_applies_with_the_classified_state() {
    let mut ui = runtime();
    // Only the first answer succeeds; a second call during restore would
    // have turned the entry into a rejection inside a successful report.
    let (element, calls) = fragile(1);
    ui.mount_tree(element).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let saved = Workspace::new(
        WorkspaceSettings::default(),
        [(
            WorkspacePath::new(["main", "custom"]).unwrap(),
            ViewState::new("fragile").unwrap(),
        )],
    )
    .unwrap();
    let report = ui.restore_workspace(&saved).unwrap();
    assert_eq!(report.applied().count(), 1);
    assert_eq!(report.rejected().count(), 0);
    assert_eq!(calls.get(), 1);
}

#[test]
fn a_participant_cannot_write_while_restore_classifies_it() {
    let mut ui = runtime();
    let signal = ui.signal(0_u32);
    let refused = Rc::new(Cell::new(false));
    let (written, seen) = (signal.clone(), refused.clone());
    let (element, _) = scripted(move |_| {
        seen.set(seen.get() || written.set(7).is_err());
        Ok(Some(ViewState::new("scripted").unwrap()))
    });
    ui.mount_tree(element).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let saved = Workspace::new(WorkspaceSettings::default(), []).unwrap();
    assert!(ui.restore_workspace(&saved).unwrap().is_complete());
    assert!(refused.get(), "the write was not refused");
    assert_eq!(signal.get().unwrap(), 0);
}

#[test]
fn a_retained_entry_for_a_failing_participant_is_rejected_when_it_mounts() {
    let mut ui = runtime();
    ui.mount_tree(Element::new("other", Label::new("Other")))
        .unwrap();
    let saved = Workspace::new(
        WorkspaceSettings::default(),
        [(
            WorkspacePath::new(["main", "custom"]).unwrap(),
            ViewState::new("fragile").unwrap(),
        )],
    )
    .unwrap();
    assert_eq!(ui.restore_workspace(&saved).unwrap().pending().count(), 1);
    let (element, _) = fragile(0);
    ui.mount_tree(element).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let report = ui.workspace_report();
    assert_eq!(report.pending().count(), 0);
    let rejected: Vec<_> = report.rejected().collect();
    assert_eq!(rejected.len(), 1);
    assert_eq!(
        *rejected[0].1,
        RestoreRejection::State(WorkspaceError::TextTooLong)
    );
}

#[test]
fn tabs_whose_selected_key_cannot_be_stored_fail_capture_by_path() {
    let mut ui = runtime();
    let long = "t".repeat(Workspace::MAX_STRING_BYTES + 1);
    ui.mount_tree(Element::new(
        "main",
        Tabs::new([Tab::new(
            long,
            "Long",
            Element::new("body", Label::new("Body")),
        )]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(
        participant(ui.capture_workspace()),
        ("main".to_owned(), WorkspaceError::TextTooLong)
    );
}

struct WorkspacePreviewSource;
impl Component for WorkspacePreviewSource {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if event.kind() == PointerKind::Press(PointerButton::Primary) {
            cx.start_drag(
                DragPayload::new(()),
                DropOperations::MOVE,
                DragPreview::element(Element::new(
                    "items",
                    ScrollArea::new(Element::new("body", Label::new("Preview"))),
                )),
            )?;
            return Ok(InputOutcome::Handled);
        }
        Ok(InputOutcome::Ignored)
    }
}
#[test]
fn a_preview_mount_cannot_consume_pending_state_for_a_real_view() {
    let mut ui = runtime();
    let source = ui
        .mount_tree(Element::new("source", WorkspacePreviewSource))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let state = ViewState::new("scroll")
        .unwrap()
        .with("x", WorkspaceValue::number(0.0).unwrap())
        .unwrap()
        .with("y", WorkspaceValue::number(17.0).unwrap())
        .unwrap();
    let saved = Workspace::new(
        WorkspaceSettings::default(),
        [(WorkspacePath::new(["items"]).unwrap(), state)],
    )
    .unwrap();
    assert_eq!(ui.restore_workspace(&saved).unwrap().pending().count(), 1);
    ui.pointer_input(PointerEvent::new(
        Point::new(10.0, 10.0).unwrap(),
        PointerKind::Press(PointerButton::Primary),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(ui.workspace_report().pending().count(), 1);
    assert!(
        !ui.inspect()
            .components
            .iter()
            .filter(|c| c.key == "items")
            .collect::<Vec<_>>()
            .is_empty()
    );
    ui.cancel_input().unwrap();
    ui.unmount(&source).unwrap();
    ui.mount_tree(Element::new(
        "items",
        ScrollArea::new(Element::new("body", TallRestoredContent)),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(ui.workspace_report().pending().count(), 0);
    assert!(ui.workspace_report().is_complete());
    let captured = ui.capture_workspace().unwrap();
    assert_eq!(
        captured
            .entries()
            .find(|(path, _)| path.to_string() == "items")
            .unwrap()
            .1
            .number("y")
            .unwrap(),
        17.0
    );
}

struct TallRestoredContent;
impl Component for TallRestoredContent {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(100.0, 1000.0)?)
    }
}

struct ObservedPreviewSource(Rc<Cell<Option<DragOutcome>>>);
impl Component for ObservedPreviewSource {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(100.0, cx.metric(MetricRole::ControlHeight))?)
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        WorkspacePreviewSource.pointer_input(cx, event)
    }
    fn drag_finished(
        &mut self,
        _: &mut EventContext,
        outcome: DragOutcome,
    ) -> Result<(), RuntimeError> {
        self.0.set(Some(outcome));
        Ok(())
    }
}
#[test]
fn restoring_changed_geometry_settings_cancels_drag_but_noop_settings_do_not() {
    for (settings, cancels) in [
        (
            WorkspaceSettings::default().with_zoom(Some(UiZoom::new(1.25).unwrap())),
            true,
        ),
        (
            WorkspaceSettings::default().with_zoom(Some(UiZoom::default())),
            false,
        ),
        (
            WorkspaceSettings::default().with_density(Some(Density::Compact)),
            true,
        ),
        (
            WorkspaceSettings::default().with_density(Some(Density::Standard)),
            false,
        ),
        (
            WorkspaceSettings::default().with_theme_mode(Some(ThemeMode::Light)),
            false,
        ),
    ] {
        let mut ui = runtime();
        let outcome = Rc::new(Cell::new(None));
        ui.mount_tree(Element::new(
            "source",
            ObservedPreviewSource(outcome.clone()),
        ))
        .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        ui.pointer_input(PointerEvent::new(
            Point::new(10.0, 10.0).unwrap(),
            PointerKind::Press(PointerButton::Primary),
        ))
        .unwrap();
        assert!(ui.drag_active());
        let report = ui
            .restore_workspace(&Workspace::new(settings, []).unwrap())
            .unwrap();
        assert!(report.is_complete());
        ui.flush(Duration::ZERO).unwrap();
        assert_eq!(ui.drag_active(), !cancels);
        assert_eq!(outcome.get(), cancels.then_some(DragOutcome::Cancelled));
    }
}

#[test]
fn restoring_zoom_cancels_a_captured_gesture_without_a_drag_session() {
    struct Captured(Rc<Cell<usize>>);
    impl Component for Captured {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn pointer_input(
            &mut self,
            cx: &mut EventContext,
            event: PointerEvent,
        ) -> Result<InputOutcome, RuntimeError> {
            if event.kind() == PointerKind::Press(PointerButton::Primary) {
                cx.capture_pointer()?;
            }
            Ok(InputOutcome::Handled)
        }
        fn cancel_input(&mut self, _: &mut EventContext) {
            self.0.set(self.0.get() + 1);
        }
    }
    let mut ui = runtime();
    let cancelled = Rc::new(Cell::new(0));
    ui.mount_tree(Element::new("captured", Captured(cancelled.clone())))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.pointer_input(PointerEvent::new(
        Point::new(10.0, 10.0).unwrap(),
        PointerKind::Press(PointerButton::Primary),
    ))
    .unwrap();
    assert!(!ui.drag_active());
    let before = cancelled.get();
    assert!(
        ui.restore_workspace(
            &Workspace::new(
                WorkspaceSettings::default().with_zoom(Some(UiZoom::new(1.25).unwrap())),
                []
            )
            .unwrap()
        )
        .unwrap()
        .is_complete()
    );
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(cancelled.get(), before + 1);
}

#[test]
fn every_workspace_value_is_built_through_a_checking_constructor() {
    assert_eq!(
        WorkspaceValue::number(f64::NAN),
        Err(WorkspaceError::NotFinite)
    );
    assert!(WorkspaceValue::text("line\nbreak").is_err());
    let inner = WorkspaceValue::map([("j".to_owned(), WorkspaceValue::integer(1))]).unwrap();
    assert_eq!(
        WorkspaceValue::map([("m".to_owned(), inner)]),
        Err(WorkspaceError::NestedMap)
    );
    let number = WorkspaceValue::number(0.5).unwrap();
    assert_eq!(number.as_number(), Some(0.5));
    assert_eq!(WorkspaceValue::flag(true).as_flag(), Some(true));
}
