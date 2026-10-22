//! Tool window layouts: optional edges and stripes, expansion with retained
//! state and size, focus, reveals (also nested), permissions from locked to
//! open, change origins, keyed identity and workspace persistence.
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn ui() -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())
        .unwrap();
    ui
}

fn flush(ui: &mut Runtime) {
    ui.flush(Duration::ZERO).unwrap();
}

fn button(key: &str) -> Element {
    Element::new(key, Button::new(key, |_| Ok(())))
}

fn find(ui: &Runtime, root: &WidgetId, key: &str) -> WidgetId {
    ui.find_descendant(root, key)
        .unwrap()
        .unwrap_or_else(|| panic!("{key} is not mounted"))
}

fn shown(ui: &Runtime, key: &str) -> bool {
    ui.inspect()
        .components
        .iter()
        .any(|component| component.key == key && component.visible)
}

fn details(ui: &Runtime, key: &str) -> ComponentDetails {
    ui.inspect()
        .components
        .into_iter()
        .find(|component| component.key == key)
        .unwrap_or_else(|| panic!("no component keyed {key}"))
        .details
}

fn rect(ui: &Runtime, root: &WidgetId, key: &str) -> Rect {
    ui.bounds(&find(ui, root, key)).unwrap().unwrap()
}

fn press(ui: &mut Runtime, code: KeyCode) {
    ui.key_input(code, KeyAction::Press).unwrap();
    ui.key_input(code, KeyAction::Release).unwrap();
    flush(ui);
}

fn pointer(ui: &mut Runtime, x: f32, y: f32, kind: PointerKind) {
    ui.pointer_input(PointerEvent::new(Point::new(x, y).unwrap(), kind))
        .unwrap();
    flush(ui);
}

fn click(ui: &mut Runtime, x: f32, y: f32) {
    pointer(ui, x, y, PointerKind::Press(PointerButton::Primary));
    pointer(ui, x, y, PointerKind::Release(PointerButton::Primary));
}

fn center(rect: Rect) -> (f32, f32) {
    (
        rect.x() + rect.width() / 2.0,
        rect.y() + rect.height() / 2.0,
    )
}

fn window(key: &str, edge: ToolEdge) -> ToolWindow {
    ToolWindow::new(key, format!("{key} title"), button(&format!("{key}-body")))
        .icon(Icon::Folder)
        .edge(edge)
}

/// A layout with a stripe on the left holding `files` and `search`, and a
/// stripe on the bottom holding `log`.
fn layout() -> ToolWindowLayout {
    ToolWindowLayout::new(button("main"))
        .edge(ToolEdge::Left, EdgeMode::Stripe)
        .edge(ToolEdge::Bottom, EdgeMode::Stripe)
        .window(window("files", ToolEdge::Left))
        .window(window("search", ToolEdge::Left))
        .window(window("log", ToolEdge::Bottom))
}

fn stripe_items(ui: &Runtime, key: &str) -> Vec<SemanticItem> {
    details(ui, key).semantics.unwrap().items
}

#[test]
fn edges_hold_nothing_by_default_and_the_center_fills() -> TestResult {
    let mut ui = ui();
    let root = ui.mount_tree(Element::new("frame", ToolWindowLayout::new(button("main"))))?;
    flush(&mut ui);
    assert_eq!(rect(&ui, &root, "main").width(), 800.0);
    assert_eq!(rect(&ui, &root, "main").height(), 600.0);
    for edge in ["left", "right", "top", "bottom"] {
        assert!(
            ui.find_descendant(&root, &format!("stripe-{edge}"))?
                .is_none()
        );
    }

    // Every edge with a stripe: stripes take the outer edges, the center the rest.
    let mut ui = self::ui();
    let mut all = ToolWindowLayout::new(button("main"));
    for (edge, key) in [
        (ToolEdge::Left, "l"),
        (ToolEdge::Right, "r"),
        (ToolEdge::Top, "t"),
        (ToolEdge::Bottom, "b"),
    ] {
        all = all
            .edge(edge, EdgeMode::Stripe)
            .window(window(key, edge).expanded(true));
    }
    let root = ui.mount_tree(Element::new("frame", all))?;
    flush(&mut ui);
    let left = rect(&ui, &root, "stripe-left");
    let right = rect(&ui, &root, "stripe-right");
    let top = rect(&ui, &root, "stripe-top");
    let bottom = rect(&ui, &root, "stripe-bottom");
    assert_eq!((left.x(), left.height()), (0.0, 600.0));
    assert_eq!((right.x() + right.width(), right.height()), (800.0, 600.0));
    assert_eq!(top.x(), left.width());
    assert_eq!(bottom.y() + bottom.height(), 600.0);
    let l = rect(&ui, &root, "l");
    let t = rect(&ui, &root, "t");
    let main = rect(&ui, &root, "main");
    assert_eq!(l.x(), left.width());
    assert!(
        t.x() >= l.x() + l.width(),
        "top panels sit between the side panels"
    );
    assert!(main.x() > l.x() + l.width() && main.y() > t.y() + t.height());

    // Panels without a stripe: no stripe, but the panel shows when expanded.
    let mut ui = self::ui();
    let root = ui.mount_tree(Element::new(
        "frame",
        ToolWindowLayout::new(button("main"))
            .edge(ToolEdge::Right, EdgeMode::Panels)
            .window(window("props", ToolEdge::Right).expanded(true)),
    ))?;
    flush(&mut ui);
    assert!(ui.find_descendant(&root, "stripe-right")?.is_none());
    assert!(shown(&ui, "props-body"));
    assert_eq!(
        rect(&ui, &root, "props").x() + rect(&ui, &root, "props").width(),
        800.0
    );
    Ok(())
}

#[test]
fn invalid_descriptions_are_rejected_before_mounting() {
    for (layout, expected) in [
        (
            // A tool window on an edge that holds nothing.
            ToolWindowLayout::new(button("main")).window(window("files", ToolEdge::Left)),
            LayoutError::UnavailableEdge,
        ),
        (
            // A reserved key.
            ToolWindowLayout::new(button("main"))
                .edge(ToolEdge::Left, EdgeMode::Stripe)
                .window(window("center", ToolEdge::Left)),
            LayoutError::ReservedKey,
        ),
        (
            // A blank key.
            ToolWindowLayout::new(button("main"))
                .edge(ToolEdge::Left, EdgeMode::Stripe)
                .window(window(" ", ToolEdge::Left)),
            LayoutError::BlankKey,
        ),
        (
            // Two tool windows sharing a key.
            ToolWindowLayout::new(button("main"))
                .edge(ToolEdge::Left, EdgeMode::Stripe)
                .window(window("files", ToolEdge::Left))
                .window(window("files", ToolEdge::Left)),
            LayoutError::DuplicateKey,
        ),
        (
            // Content keyed like the header.
            ToolWindowLayout::new(button("main"))
                .edge(ToolEdge::Left, EdgeMode::Stripe)
                .window(ToolWindow::new("files", "Files", button("hide"))),
            LayoutError::ReservedKey,
        ),
    ] {
        let mut ui = ui();
        let result = ui.mount_tree(Element::new("frame", layout));
        assert_eq!(result.err(), Some(RuntimeError::Layout(expected)));
    }
}

#[test]
fn a_stripe_button_expands_and_collapses_keeping_state_and_size() -> TestResult {
    let mut ui = ui();
    let text = ui.signal(String::new());
    let root = ui.mount_tree(Element::new(
        "frame",
        ToolWindowLayout::new(button("main"))
            .edge(ToolEdge::Left, EdgeMode::Stripe)
            .window(
                ToolWindow::new(
                    "notes",
                    "Notes",
                    Element::new("editor", TextField::new(text.clone())),
                )
                .icon(Icon::Edit),
            ),
    ))?;
    flush(&mut ui);
    assert!(!shown(&ui, "editor"));
    let stripe = rect(&ui, &root, "stripe-left");
    let (x, y) = (stripe.x() + stripe.width() / 2.0, stripe.y() + 10.0);
    click(&mut ui, x, y);
    assert!(shown(&ui, "editor"));
    // Expanding from the stripe moves focus into the panel.
    let editor = find(&ui, &root, "editor");
    assert_eq!(ui.focused(), Some(editor.clone()));
    ui.text_input("draft")?;
    flush(&mut ui);

    // Resize through the divider, then collapse and expand again.
    let divider = rect(&ui, &root, "divider-left");
    let before = rect(&ui, &root, "notes").width();
    let (dx, dy) = center(divider);
    pointer(&mut ui, dx, dy, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, dx + 40.0, dy, PointerKind::Move);
    pointer(
        &mut ui,
        dx + 40.0,
        dy,
        PointerKind::Release(PointerButton::Primary),
    );
    let resized = rect(&ui, &root, "notes").width();
    assert_eq!(resized, before + 40.0);

    click(&mut ui, x, y);
    assert!(!shown(&ui, "editor"));
    assert!(!shown(&ui, "divider-left"));
    assert_eq!(rect(&ui, &root, "main").x(), stripe.width());
    click(&mut ui, x, y);
    assert!(shown(&ui, "editor"));
    assert_eq!(
        find(&ui, &root, "editor"),
        editor,
        "the content kept its identity"
    );
    assert_eq!(rect(&ui, &root, "notes").width(), resized, "and its size");
    assert_eq!(
        details(&ui, "editor").properties.get("text"),
        Some(&InspectValue::Text("draft".into())),
        "and its draft"
    );
    Ok(())
}

#[test]
fn each_group_shows_one_tool_window_and_two_groups_share_the_edge() -> TestResult {
    let mut ui = ui();
    let controller = ToolWindowController::new();
    let root = ui.mount_tree(Element::new(
        "frame",
        ToolWindowLayout::new(button("main"))
            .edge(ToolEdge::Left, EdgeMode::Stripe)
            .window(window("a", ToolEdge::Left).expanded(true))
            .window(window("b", ToolEdge::Left).expanded(true))
            .window(window("c", ToolEdge::Left).group(StripeGroup::End))
            .controller(&controller),
    ))?;
    flush(&mut ui);
    // Two start-group windows described expanded: the first wins.
    assert!(shown(&ui, "a-body") && !shown(&ui, "b-body"));
    assert!(controller.expand("b")?);
    flush(&mut ui);
    assert!(!shown(&ui, "a-body") && shown(&ui, "b-body"));
    assert!(controller.expand("c")?);
    flush(&mut ui);
    let (b, c) = (rect(&ui, &root, "b"), rect(&ui, &root, "c"));
    assert_eq!(b.height(), c.height());
    assert_eq!(
        b.y() + b.height(),
        c.y(),
        "the end group's panel sits below"
    );
    // The end group's button sits at the end of the stripe.
    let stripe = rect(&ui, &root, "stripe-left");
    let items = stripe_items(&ui, "stripe-left");
    assert_eq!(items.len(), 3);
    assert_eq!(items[2].name.as_deref(), Some("c title"));
    assert!(stripe.height() == 600.0);
    assert!(!controller.toggle("c")?);
    assert!(matches!(
        controller.expand("unknown"),
        Err(RuntimeError::Layout(LayoutError::UnknownKey))
    ));
    Ok(())
}

#[test]
fn stripe_keys_move_between_buttons_and_toggle() -> TestResult {
    let mut ui = ui();
    let root = ui.mount_tree(Element::new("frame", layout()))?;
    flush(&mut ui);
    let stripe = find(&ui, &root, "stripe-left");
    ui.focus(&stripe)?;
    flush(&mut ui);
    press(&mut ui, KeyCode::ArrowDown);
    let items = stripe_items(&ui, "stripe-left");
    assert!(items[1].selected && !items[0].selected);
    press(&mut ui, KeyCode::Enter);
    assert!(shown(&ui, "search-body"));
    // Focus moved into the panel; Escape returns it to the center.
    assert_eq!(ui.focused(), Some(find(&ui, &root, "search-body")));
    press(&mut ui, KeyCode::Escape);
    assert_eq!(ui.focused(), Some(find(&ui, &root, "main")));
    assert!(shown(&ui, "search-body"), "Escape does not collapse");
    ui.focus(&stripe)?;
    press(&mut ui, KeyCode::Home);
    press(&mut ui, KeyCode::Space);
    assert!(shown(&ui, "files-body") && !shown(&ui, "search-body"));
    Ok(())
}

#[test]
fn collapsing_a_focused_tool_window_returns_focus_to_the_center() -> TestResult {
    let mut ui = ui();
    let controller = ToolWindowController::new();
    let root = ui.mount_tree(Element::new("frame", layout().controller(&controller)))?;
    flush(&mut ui);
    // The hide button.
    controller.expand("files")?;
    flush(&mut ui);
    ui.focus(&find(&ui, &root, "files-body"))?;
    flush(&mut ui);
    let hide = find(&ui, &find(&ui, &root, "files"), "hide-button");
    ui.focus(&hide)?;
    press(&mut ui, KeyCode::Enter);
    assert!(!shown(&ui, "files-body"));
    assert_eq!(ui.focused(), Some(find(&ui, &root, "main")));

    // An application collapse.
    controller.expand("files")?;
    flush(&mut ui);
    ui.focus(&find(&ui, &root, "files-body"))?;
    flush(&mut ui);
    controller.collapse("files")?;
    flush(&mut ui);
    assert!(!shown(&ui, "files-body"));
    assert_eq!(ui.focused(), Some(find(&ui, &root, "main")));
    Ok(())
}

#[test]
fn revealing_into_a_collapsed_tool_window_expands_it_also_when_nested() -> TestResult {
    let mut ui = ui();
    let inner = ToolWindowLayout::new(button("inner-main"))
        .edge(ToolEdge::Left, EdgeMode::Stripe)
        .window(window("outline", ToolEdge::Left));
    let root = ui.mount_tree(Element::new(
        "frame",
        ToolWindowLayout::new(button("main"))
            .edge(ToolEdge::Right, EdgeMode::Stripe)
            .window(
                ToolWindow::new("structure", "Structure", Element::new("nested", inner))
                    .edge(ToolEdge::Right),
            ),
    ))?;
    flush(&mut ui);
    let target = find(&ui, &root, "outline-body");
    assert!(!shown(&ui, "outline-body"));
    assert!(matches!(
        ui.reveal_and_focus(&target)?,
        RevealOutcome::RevealedAndFocused { .. }
    ));
    flush(&mut ui);
    assert!(shown(&ui, "structure") && shown(&ui, "outline-body"));
    assert_eq!(ui.focused(), Some(target.clone()));
    // Escape in the inner tool window returns to the inner layout's center.
    press(&mut ui, KeyCode::Escape);
    assert_eq!(ui.focused(), Some(find(&ui, &root, "inner-main")));
    Ok(())
}

#[test]
fn locked_layouts_and_denied_permissions_restrict_only_the_user() -> TestResult {
    let mut ui = ui();
    let controller = ToolWindowController::new();
    let root = ui.mount_tree(Element::new(
        "frame",
        layout().locked().controller(&controller),
    ))?;
    flush(&mut ui);
    let stripe = find(&ui, &root, "stripe-left");
    assert!(ui.focus(&stripe).is_ok(), "explicit focus still works");
    let stripe_rect = rect(&ui, &root, "stripe-left");
    click(&mut ui, stripe_rect.x() + 10.0, stripe_rect.y() + 10.0);
    assert!(!shown(&ui, "files-body"), "a locked stripe does not toggle");
    assert!(
        stripe_items(&ui, "stripe-left")
            .iter()
            .all(|item| item.disabled)
    );
    // The application still can, and the panel offers no hide button.
    controller.expand("files")?;
    flush(&mut ui);
    assert!(shown(&ui, "files-body"));
    assert!(
        ui.find_descendant(&find(&ui, &root, "files"), "hide")?
            .is_none()
    );
    let divider = find(&ui, &root, "divider-left");
    assert_eq!(
        details(&ui, "divider-left").properties.get("resizable"),
        Some(&InspectValue::Bool(false))
    );
    let before = rect(&ui, &root, "files").width();
    let (dx, dy) = center(ui.bounds(&divider)?.unwrap());
    pointer(&mut ui, dx, dy, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, dx + 50.0, dy, PointerKind::Move);
    pointer(
        &mut ui,
        dx + 50.0,
        dy,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(rect(&ui, &root, "files").width(), before);

    // A tool window's own denial combines with an open layout.
    let mut ui = self::ui();
    let root = ui.mount_tree(Element::new(
        "frame",
        ToolWindowLayout::new(button("main"))
            .edge(ToolEdge::Left, EdgeMode::Stripe)
            .window(
                window("fixed", ToolEdge::Left)
                    .toggles(false)
                    .resizable(false)
                    .expanded(true),
            )
            .window(window("free", ToolEdge::Left)),
    ))?;
    flush(&mut ui);
    let items = stripe_items(&ui, "stripe-left");
    assert!(items[0].disabled && !items[1].disabled);
    assert_eq!(
        details(&ui, "divider-left").properties.get("resizable"),
        Some(&InspectValue::Bool(false))
    );
    assert!(
        ui.find_descendant(&find(&ui, &root, "fixed"), "hide")?
            .is_none()
    );
    Ok(())
}

#[test]
fn a_revoked_resize_permission_cancels_a_held_drag() -> TestResult {
    let mut ui = ui();
    let resizable = ui.signal(true);
    let root = ui.mount_tree(Element::new(
        "frame",
        layout().resizable(resizable.read_only()),
    ))?;
    flush(&mut ui);
    let stripe = rect(&ui, &root, "stripe-left");
    click(&mut ui, stripe.x() + 10.0, stripe.y() + 10.0);
    let before = rect(&ui, &root, "files").width();
    let (dx, dy) = center(rect(&ui, &root, "divider-left"));
    pointer(&mut ui, dx, dy, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, dx + 30.0, dy, PointerKind::Move);
    assert_eq!(rect(&ui, &root, "files").width(), before + 30.0);
    resizable.set(false)?;
    flush(&mut ui);
    assert_eq!(rect(&ui, &root, "files").width(), before, "restored");
    pointer(&mut ui, dx + 60.0, dy, PointerKind::Move);
    pointer(
        &mut ui,
        dx + 60.0,
        dy,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(rect(&ui, &root, "files").width(), before);
    resizable.set(true)?;
    flush(&mut ui);

    // Arrow keys on a focused divider resize; Escape restores.
    let divider = find(&ui, &root, "divider-left");
    ui.focus(&divider)?;
    ui.key_input(KeyCode::ArrowRight, KeyAction::Press)?;
    flush(&mut ui);
    assert_eq!(rect(&ui, &root, "files").width(), before + 8.0);
    press(&mut ui, KeyCode::Escape);
    ui.key_input(KeyCode::ArrowRight, KeyAction::Release)?;
    flush(&mut ui);
    assert_eq!(rect(&ui, &root, "files").width(), before);
    Ok(())
}

type Log = Rc<RefCell<Vec<(String, bool, ChangeOrigin)>>>;

fn logged(layout: ToolWindowLayout, log: &Log) -> ToolWindowLayout {
    let log = log.clone();
    layout.on_change(move |_, change| {
        log.borrow_mut()
            .push((change.key.clone(), change.expanded, change.origin));
        Ok(())
    })
}

#[test]
fn changes_report_their_origin() -> TestResult {
    let mut ui = ui();
    let seen: Log = Rc::default();
    let controller = ToolWindowController::new();
    let parent = ui.mount_tree(Element::new(
        "column",
        Column::new([Element::new(
            "frame",
            logged(layout().controller(&controller), &seen),
        )]),
    ))?;
    flush(&mut ui);
    let root = find(&ui, &parent, "frame");
    let saved = ui.capture_workspace()?;
    let stripe = rect(&ui, &root, "stripe-left");
    click(&mut ui, stripe.x() + 10.0, stripe.y() + 10.0);
    controller.expand("search")?;
    flush(&mut ui);
    let target = find(&ui, &root, "log-body");
    ui.reveal(&target)?;
    flush(&mut ui);
    assert!(ui.restore_workspace(&saved)?.is_complete());
    flush(&mut ui);
    assert_eq!(
        *seen.borrow(),
        [
            ("files".into(), true, ChangeOrigin::User),
            ("files".into(), false, ChangeOrigin::Application),
            ("search".into(), true, ChangeOrigin::Application),
            ("log".into(), true, ChangeOrigin::Reveal),
            ("log".into(), false, ChangeOrigin::Restore),
            ("search".into(), false, ChangeOrigin::Restore),
        ]
    );

    // A new description that moves an expanded tool window into a group that
    // already shows one collapses it.
    controller.expand("files")?;
    controller.expand("log")?;
    flush(&mut ui);
    seen.borrow_mut().clear();
    ui.reconcile_children(
        &parent,
        vec![Element::new(
            "frame",
            logged(
                ToolWindowLayout::new(button("main"))
                    .edge(ToolEdge::Left, EdgeMode::Stripe)
                    .edge(ToolEdge::Bottom, EdgeMode::Stripe)
                    .window(window("files", ToolEdge::Left))
                    .window(window("search", ToolEdge::Left))
                    .window(window("log", ToolEdge::Left))
                    .controller(&controller),
                &seen,
            ),
        )],
    )?;
    flush(&mut ui);
    assert_eq!(
        *seen.borrow(),
        [("log".into(), false, ChangeOrigin::Redescription)]
    );
    assert!(shown(&ui, "files-body") && !shown(&ui, "log-body"));
    Ok(())
}

#[test]
fn tool_windows_keep_their_state_by_key_under_redescription() -> TestResult {
    let mut ui = ui();
    let parent = ui.mount_tree(Element::new(
        "column",
        Column::new([Element::new("frame", layout())]),
    ))?;
    flush(&mut ui);
    let root = find(&ui, &parent, "frame");
    let stripe = rect(&ui, &root, "stripe-left");
    click(&mut ui, stripe.x() + 10.0, stripe.y() + 10.0);
    let files_body = find(&ui, &root, "files-body");
    // Reorder, retitle, add and remove tool windows.
    ui.reconcile_children(
        &parent,
        vec![Element::new(
            "frame",
            ToolWindowLayout::new(button("main"))
                .edge(ToolEdge::Left, EdgeMode::Stripe)
                .window(window("new", ToolEdge::Left).group(StripeGroup::End))
                .window(
                    ToolWindow::new("files", "Renamed files", button("files-body"))
                        .icon(Icon::Folder),
                )
                .window(window("search", ToolEdge::Left)),
        )],
    )?;
    flush(&mut ui);
    assert!(shown(&ui, "files-body"));
    assert_eq!(find(&ui, &root, "files-body"), files_body);
    let names: Vec<_> = stripe_items(&ui, "stripe-left")
        .into_iter()
        .map(|item| item.name.unwrap())
        .collect();
    assert_eq!(names, ["Renamed files", "search title", "new title"]);
    assert!(ui.find_descendant(&root, "log")?.is_none());
    assert!(ui.find_descendant(&root, "stripe-bottom")?.is_none());

    // The conformance check: a re-description is applied in place and looks
    // like a fresh mount of the new description.
    let mut check = self::ui();
    check_redescription(
        &mut check,
        Element::new("frame", layout()),
        Element::new("frame", layout().locked()),
        Element::new("frame", layout().locked()),
    )?;
    Ok(())
}

#[test]
fn workspace_round_trips_and_rejects_invalid_documents() -> TestResult {
    let mut ui = ui();
    let root = ui.mount_tree(Element::new("frame", layout().show_names(true)))?;
    flush(&mut ui);
    let stripe = rect(&ui, &root, "stripe-left");
    click(&mut ui, stripe.x() + 10.0, stripe.y() + 10.0);
    let (dx, dy) = center(rect(&ui, &root, "divider-left"));
    pointer(&mut ui, dx, dy, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, dx + 20.5, dy, PointerKind::Move);
    pointer(
        &mut ui,
        dx + 20.5,
        dy,
        PointerKind::Release(PointerButton::Primary),
    );
    let width = rect(&ui, &root, "files").width();
    let saved = ui.capture_workspace()?;
    let (path, state) = saved.entries().next().unwrap();
    assert_eq!(path.to_string(), "frame");
    assert_eq!(state.kind(), "tool-windows");
    assert_eq!(state.map("expanded")?.len(), 3);
    assert!(state.map("extents")?.contains_key("files"));
    assert!(state.flag("names")?);
    let encoded = Workspace::decode(&saved.encode())?;

    // Change everything, then restore.
    click(&mut ui, stripe.x() + 10.0, stripe.y() + 10.0);
    assert!(!shown(&ui, "files-body"));
    assert!(ui.restore_workspace(&encoded)?.is_complete());
    flush(&mut ui);
    assert!(shown(&ui, "files-body"));
    assert_eq!(rect(&ui, &root, "files").width(), width);
    let again = ui.capture_workspace()?;
    assert_eq!(
        again.encode(),
        saved.encode(),
        "restore then capture is exact"
    );

    let path = WorkspacePath::new(["frame"])?;
    // `expanded` and `extents` are required; a case that leaves one out
    // gets an empty map for it.
    let restore = |ui: &mut Runtime, mut state: ViewState| {
        for required in ["expanded", "extents"] {
            if state.get(required).is_none() {
                let empty = WorkspaceValue::map(Vec::<(String, WorkspaceValue)>::new()).unwrap();
                state = state.with(required, empty).unwrap();
            }
        }
        ui.restore_workspace(
            &Workspace::new(WorkspaceSettings::default(), [(path.clone(), state)]).unwrap(),
        )
        .unwrap()
        .rejected()
        .map(|(_, reason)| reason.clone())
        .collect::<Vec<_>>()
    };
    let flags = |entries: &[(&str, bool)]| {
        WorkspaceValue::map(
            entries
                .iter()
                .map(|(key, flag)| ((*key).to_owned(), WorkspaceValue::flag(*flag))),
        )
        .unwrap()
    };
    let base = || ViewState::new("tool-windows").unwrap();
    assert_eq!(
        restore(&mut ui, base().with("height", WorkspaceValue::flag(true))?),
        [RestoreRejection::UnknownValue("height".into())]
    );
    assert_eq!(
        restore(&mut ui, base().with("expanded", flags(&[("ghost", true)]))?),
        [RestoreRejection::Unresolved("ghost".into())]
    );
    assert_eq!(
        restore(
            &mut ui,
            base().with("expanded", flags(&[("files", true), ("search", true)]))?
        ),
        [RestoreRejection::Value("expanded".into())]
    );
    assert_eq!(
        restore(
            &mut ui,
            base().with(
                "extents",
                WorkspaceValue::map([("files".to_owned(), WorkspaceValue::number(-1.0)?)])?
            )?
        ),
        [RestoreRejection::OutOfRange("extents".into())]
    );
    assert_eq!(
        restore(
            &mut ui,
            base().with(
                "expanded",
                WorkspaceValue::map([("files".to_owned(), WorkspaceValue::integer(1))])?
            )?
        ),
        [RestoreRejection::Value("expanded".into())]
    );
    // Busy while a divider is held.
    let (dx, dy) = center(rect(&ui, &root, "divider-left"));
    pointer(&mut ui, dx, dy, PointerKind::Press(PointerButton::Primary));
    assert_eq!(
        restore(
            &mut ui,
            base().with("expanded", flags(&[("files", false)]))?
        ),
        [RestoreRejection::Busy]
    );
    pointer(
        &mut ui,
        dx,
        dy,
        PointerKind::Release(PointerButton::Primary),
    );
    assert!(
        shown(&ui, "files-body"),
        "rejected documents changed nothing"
    );

    // Opting out.
    let mut other = self::ui();
    other.mount_tree(Element::new("frame", layout().persist(false)))?;
    other.flush(Duration::ZERO)?;
    assert!(other.capture_workspace()?.is_empty());
    Ok(())
}

#[test]
fn names_follow_the_controller_or_the_application_binding() -> TestResult {
    let mut ui = ui();
    let controller = ToolWindowController::new();
    let root = ui.mount_tree(Element::new("frame", layout().controller(&controller)))?;
    flush(&mut ui);
    let narrow = rect(&ui, &root, "stripe-left").width();
    assert!(!controller.names()?);
    assert!(controller.set_names(true)?);
    flush(&mut ui);
    assert!(rect(&ui, &root, "stripe-left").width() > narrow);
    assert_eq!(
        details(&ui, "stripe-left").properties.get("names"),
        Some(&InspectValue::Bool(true))
    );

    let mut ui = self::ui();
    let names = ui.signal(false);
    let controller = ToolWindowController::new();
    let root = ui.mount_tree(Element::new(
        "frame",
        layout()
            .bind_show_names(names.clone())
            .controller(&controller),
    ))?;
    flush(&mut ui);
    let narrow = rect(&ui, &root, "stripe-left").width();
    controller.set_names(true)?;
    assert!(names.get()?, "the controller writes the binding");
    flush(&mut ui);
    assert!(rect(&ui, &root, "stripe-left").width() > narrow);
    let saved = ui.capture_workspace()?;
    assert!(saved.entries().next().unwrap().1.get("names").is_none());
    assert!(matches!(
        ToolWindowController::new().names(),
        Err(RuntimeError::Unmounted)
    ));
    Ok(())
}

#[test]
fn request_focus_within_focuses_the_first_focusable_descendant() -> TestResult {
    type Target = Rc<RefCell<Option<WidgetId>>>;
    struct Requester(Target);
    impl Component for Requester {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn focusable(&self) -> bool {
            true
        }
        fn keyboard_event(
            &mut self,
            cx: &mut EventContext,
            event: KeyEvent,
            _: &KeyboardState,
        ) -> Result<InputOutcome, RuntimeError> {
            if event.code == KeyCode::Enter && event.transition == KeyTransition::Pressed {
                if let Some(target) = self.0.borrow().as_ref() {
                    cx.request_focus_within(target)?;
                }
                return Ok(InputOutcome::Handled);
            }
            Ok(InputOutcome::Ignored)
        }
    }
    let mut ui = ui();
    let target: Target = Rc::default();
    let root = ui.mount_tree(Element::new(
        "column",
        Column::new([
            Element::new("requester", Requester(target.clone())),
            Element::new(
                "group",
                Column::new([
                    Element::new("caption", Label::new("Not focusable")),
                    button("first"),
                    button("second"),
                ]),
            ),
            button("after"),
        ]),
    ))?;
    flush(&mut ui);
    *target.borrow_mut() = Some(find(&ui, &root, "group"));
    ui.focus(&find(&ui, &root, "requester"))?;
    press(&mut ui, KeyCode::Enter);
    assert_eq!(ui.focused(), Some(find(&ui, &root, "first")));

    // A focusable target takes focus itself.
    *target.borrow_mut() = Some(find(&ui, &root, "second"));
    ui.focus(&find(&ui, &root, "requester"))?;
    press(&mut ui, KeyCode::Enter);
    assert_eq!(ui.focused(), Some(find(&ui, &root, "second")));

    // A target with nothing focusable inside changes nothing.
    *target.borrow_mut() = Some(find(&ui, &root, "caption"));
    ui.focus(&find(&ui, &root, "requester"))?;
    press(&mut ui, KeyCode::Enter);
    assert_eq!(ui.focused(), Some(find(&ui, &root, "requester")));
    Ok(())
}

#[test]
fn an_invalid_redescription_keeps_the_mounted_layout() -> TestResult {
    let mut ui = ui();
    let controller = ToolWindowController::new();
    let parent = ui.mount_tree(Element::new(
        "column",
        Column::new([Element::new("frame", layout().controller(&controller))]),
    ))?;
    flush(&mut ui);
    controller.expand("files")?;
    flush(&mut ui);
    let root = find(&ui, &parent, "frame");
    let stripe = find(&ui, &root, "stripe-left");
    let panel = find(&ui, &root, "files");
    let body = find(&ui, &root, "files-body");
    let search = find(&ui, &root, "search");
    // The left edge now holds nothing, although its tool windows stay there.
    let invalid = || {
        ToolWindowLayout::new(button("main"))
            .edge(ToolEdge::Bottom, EdgeMode::Stripe)
            .window(window("files", ToolEdge::Left))
            .window(window("search", ToolEdge::Left))
            .window(window("log", ToolEdge::Bottom))
            .controller(&controller)
    };
    // The description is rejected before anything changes.
    assert!(matches!(
        ui.reconcile_children(&parent, vec![Element::new("frame", invalid())]),
        Err(RuntimeError::Layout(LayoutError::UnavailableEdge))
    ));
    flush(&mut ui);
    // An invalid description that also leaves out a tool window removes it
    // neither: `search` keeps its identity.
    let leaving_out = ToolWindowLayout::new(button("main"))
        .window(window("files", ToolEdge::Left))
        .controller(&controller);
    assert!(matches!(
        ui.reconcile_children(&parent, vec![Element::new("frame", leaving_out)]),
        Err(RuntimeError::Layout(LayoutError::UnavailableEdge))
    ));
    flush(&mut ui);
    assert_eq!(find(&ui, &root, "search"), search, "nothing was unmounted");
    assert_eq!(find(&ui, &root, "stripe-left"), stripe, "the stripe stays");
    assert_eq!(find(&ui, &root, "files"), panel);
    assert_eq!(find(&ui, &root, "files-body"), body);
    assert!(shown(&ui, "stripe-left") && shown(&ui, "files-body"));
    assert!(controller.is_expanded("files")?);
    // A valid description again recovers, keeping the state.
    ui.reconcile_children(
        &parent,
        vec![Element::new("frame", layout().controller(&controller))],
    )?;
    flush(&mut ui);
    assert_eq!(find(&ui, &root, "files-body"), body);
    assert!(shown(&ui, "files-body"));
    Ok(())
}

#[test]
fn a_hidden_nested_panel_gives_up_focus_ownership() -> TestResult {
    let mut ui = ui();
    let outer = ToolWindowController::new();
    let inner_controller = ToolWindowController::new();
    let inner = ToolWindowLayout::new(button("inner-main"))
        .edge(ToolEdge::Left, EdgeMode::Stripe)
        .window(window("outline", ToolEdge::Left))
        .controller(&inner_controller);
    let root = ui.mount_tree(Element::new(
        "frame",
        ToolWindowLayout::new(button("main"))
            .edge(ToolEdge::Right, EdgeMode::Stripe)
            .window(
                ToolWindow::new("structure", "Structure", Element::new("nested", inner))
                    .edge(ToolEdge::Right),
            )
            .controller(&outer),
    ))?;
    flush(&mut ui);
    outer.expand("structure")?;
    flush(&mut ui);
    inner_controller.expand("outline")?;
    flush(&mut ui);
    ui.focus(&find(&ui, &root, "outline-body"))?;
    flush(&mut ui);
    // Collapsing the outer tool window hides the inner panel that held focus.
    outer.collapse("structure")?;
    flush(&mut ui);
    assert_eq!(ui.focused(), Some(find(&ui, &root, "main")));
    outer.expand("structure")?;
    flush(&mut ui);
    // Collapsing the inner one now must not move focus out of the center.
    inner_controller.collapse("outline")?;
    flush(&mut ui);
    assert_eq!(ui.focused(), Some(find(&ui, &root, "main")));
    Ok(())
}

#[test]
fn a_replaced_or_removed_controller_detaches() -> TestResult {
    let mut ui = ui();
    let first = ToolWindowController::new();
    let second = ToolWindowController::new();
    let parent = ui.mount_tree(Element::new(
        "column",
        Column::new([Element::new("frame", layout().controller(&first))]),
    ))?;
    flush(&mut ui);
    assert!(first.expand("files")?);
    ui.reconcile_children(
        &parent,
        vec![Element::new("frame", layout().controller(&second))],
    )?;
    flush(&mut ui);
    assert!(matches!(
        first.collapse("files"),
        Err(RuntimeError::Unmounted)
    ));
    assert!(second.collapse("files")?);
    ui.reconcile_children(&parent, vec![Element::new("frame", layout())])?;
    flush(&mut ui);
    assert!(matches!(
        second.expand("files"),
        Err(RuntimeError::Unmounted)
    ));
    // Describing the same controller again keeps it attached.
    ui.reconcile_children(
        &parent,
        vec![Element::new("frame", layout().controller(&first))],
    )?;
    flush(&mut ui);
    ui.reconcile_children(
        &parent,
        vec![Element::new("frame", layout().controller(&first))],
    )?;
    flush(&mut ui);
    assert!(first.expand("files")?);
    Ok(())
}

/// A left edge holding `files` and an expanded end-group overlay, restored to
/// fractional extents, with its captured workspace.
fn overlay_fixture() -> Result<(Runtime, WidgetId, Workspace), Box<dyn std::error::Error>> {
    let mut ui = ui();
    let root = ui.mount_tree(Element::new(
        "frame",
        ToolWindowLayout::new(button("main"))
            .edge(ToolEdge::Left, EdgeMode::Stripe)
            .window(window("files", ToolEdge::Left).expanded(true))
            .window(
                window("overlay", ToolEdge::Left)
                    .group(StripeGroup::End)
                    .overlay(true)
                    .expanded(true),
            ),
    ))?;
    flush(&mut ui);
    let state = ViewState::new("tool-windows")?
        .with(
            "expanded",
            WorkspaceValue::map([
                ("files".to_owned(), WorkspaceValue::flag(true)),
                ("overlay".to_owned(), WorkspaceValue::flag(true)),
            ])?,
        )?
        .with(
            "extents",
            WorkspaceValue::map([
                ("files".to_owned(), WorkspaceValue::number(100.00000001)?),
                ("overlay".to_owned(), WorkspaceValue::number(80.00000001)?),
            ])?,
        )?;
    assert!(
        ui.restore_workspace(&Workspace::new(
            WorkspaceSettings::default(),
            [(WorkspacePath::new(["frame"])?, state)]
        )?)?
        .is_complete()
    );
    flush(&mut ui);
    let saved = ui.capture_workspace()?;
    Ok((ui, root, saved))
}

#[test]
fn cancelling_an_edge_resize_preserves_overlay_and_exact_restored_preferences() -> TestResult {
    let (mut ui, root, saved) = overlay_fixture()?;
    let divider = find(&ui, &root, "divider-left");
    ui.focus(&divider)?;
    flush(&mut ui);
    ui.key_input(KeyCode::ArrowRight, KeyAction::Press)?;
    flush(&mut ui);
    assert_ne!(ui.capture_workspace()?, saved);
    press(&mut ui, KeyCode::Escape);
    ui.key_input(KeyCode::ArrowRight, KeyAction::Release)?;
    flush(&mut ui);
    assert_eq!(ui.capture_workspace()?, saved);
    Ok(())
}

#[test]
fn cancelling_an_edge_resize_drag_with_escape_preserves_the_overlay() -> TestResult {
    // TW-11's original reproduction: a pointer drag, then Escape.
    let (mut ui, root, saved) = overlay_fixture()?;
    let (x, y) = center(rect(&ui, &root, "divider-left"));
    pointer(&mut ui, x, y, PointerKind::Move);
    pointer(&mut ui, x, y, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, x + 40.0, y, PointerKind::Move);
    assert_ne!(ui.capture_workspace()?, saved, "the drag resizes");
    press(&mut ui, KeyCode::Escape);
    pointer(
        &mut ui,
        x + 40.0,
        y,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(
        ui.capture_workspace()?,
        saved,
        "Escape restores both extents"
    );
    Ok(())
}
