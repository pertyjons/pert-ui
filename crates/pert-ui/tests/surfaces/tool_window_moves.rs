//! Tool window layouts, second slice: moving tool windows between edges and
//! groups through the panel's options menu and by dragging stripe buttons,
//! reordering within a stripe, overlay mode, their permissions, change
//! reports, re-description and workspace persistence.
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type Log = Rc<RefCell<Vec<ToolWindowChange>>>;

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

fn property(ui: &Runtime, key: &str, name: &str) -> InspectValue {
    ui.inspect()
        .components
        .into_iter()
        .find(|component| component.key == key)
        .unwrap_or_else(|| panic!("no component keyed {key}"))
        .details
        .properties
        .remove(name)
        .unwrap_or_else(|| panic!("{key} has no {name}"))
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

/// Drags from one point to another in steps, releasing at the end.
fn drag(ui: &mut Runtime, from: (f32, f32), to: (f32, f32)) {
    pointer(
        ui,
        from.0,
        from.1,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(ui, from.0 + 6.0, from.1 + 6.0, PointerKind::Move);
    pointer(
        ui,
        (from.0 + to.0) / 2.0,
        (from.1 + to.1) / 2.0,
        PointerKind::Move,
    );
    pointer(ui, to.0, to.1, PointerKind::Move);
    pointer(ui, to.0, to.1, PointerKind::Release(PointerButton::Primary));
}

fn window(key: &str, edge: ToolEdge) -> ToolWindow {
    ToolWindow::new(key, format!("{key} title"), button(&format!("{key}-body")))
        .icon(Icon::Folder)
        .edge(edge)
}

/// Left and right stripes, panels at the bottom; `files` and `search` on
/// the left, `outline` on the right, `log` at the bottom.
fn layout() -> ToolWindowLayout {
    ToolWindowLayout::new(button("main"))
        .edge(ToolEdge::Left, EdgeMode::Stripe)
        .edge(ToolEdge::Right, EdgeMode::Stripe)
        .edge(ToolEdge::Bottom, EdgeMode::Panels)
        .window(window("files", ToolEdge::Left))
        .window(window("search", ToolEdge::Left))
        .window(window("outline", ToolEdge::Right))
        .window(window("log", ToolEdge::Bottom))
}

fn logged(layout: ToolWindowLayout, log: &Log) -> ToolWindowLayout {
    let log = log.clone();
    layout.on_change(move |_, change| {
        log.borrow_mut().push(change.clone());
        Ok(())
    })
}

/// The bounds of a layout's own child, not a nested layout's.
fn child_rect(ui: &Runtime, layout: &WidgetId, key: &str) -> Rect {
    let child = ui
        .children(layout)
        .unwrap()
        .into_iter()
        .find(|child| ui.key(child).unwrap() == key)
        .unwrap_or_else(|| panic!("{key} is not a child"));
    ui.bounds(&child).unwrap().unwrap()
}

/// The center of the `index`-th button of a vertical stripe of square
/// icon-only buttons, counted from the start.
fn button_at(ui: &Runtime, root: &WidgetId, stripe: &str, index: usize) -> (f32, f32) {
    let stripe = child_rect(ui, root, stripe);
    let side = stripe.width();
    (
        stripe.x() + side / 2.0,
        stripe.y() + side * (index as f32 + 0.5),
    )
}

fn placement(ui: &Runtime) -> String {
    match property(ui, "frame", "placement") {
        InspectValue::Text(text) => text,
        other => panic!("placement is {other:?}"),
    }
}

/// The names of the visible menu's items and whether each is enabled.
fn menu_items(ui: &Runtime) -> Vec<(String, bool)> {
    ui.inspect()
        .components
        .into_iter()
        .find(|c| c.key == "items" && c.visible)
        .and_then(|c| c.details.semantics)
        .map(|semantics| {
            semantics
                .items
                .into_iter()
                .filter_map(|item| Some((item.name?, !item.disabled)))
                .filter(|(name, _)| !name.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// Opens a panel's options menu from the keyboard.
fn open_options(ui: &mut Runtime, root: &WidgetId, panel: &str) {
    let options = find(ui, &find(ui, root, panel), "options");
    let trigger = find(ui, &options, "button");
    ui.focus(&trigger).unwrap();
    flush(ui);
    press(ui, KeyCode::Enter);
}

/// Chooses an item of the open menu by name with arrow keys and Enter.
fn choose(ui: &mut Runtime, name: &str) {
    let enabled: Vec<String> = menu_items(ui)
        .into_iter()
        .filter(|(_, enabled)| *enabled)
        .map(|(name, _)| name)
        .collect();
    let steps = enabled
        .iter()
        .position(|item| item == name)
        .unwrap_or_else(|| panic!("{name} is not an enabled item of {enabled:?}"));
    for _ in 0..steps {
        press(ui, KeyCode::ArrowDown);
    }
    press(ui, KeyCode::Enter);
}

#[test]
fn the_options_menu_moves_reorders_and_switches_modes() -> TestResult {
    let mut ui = ui();
    let log = Log::default();
    let controller = ToolWindowController::new();
    let root = ui.mount_tree(Element::new(
        "frame",
        logged(layout(), &log).controller(&controller),
    ))?;
    flush(&mut ui);
    controller.expand("files")?;
    flush(&mut ui);
    log.borrow_mut().clear();

    open_options(&mut ui, &root, "files");
    let items = menu_items(&ui);
    let names: Vec<&str> = items.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Move to Left",
            "Move to Right",
            "Move to Bottom",
            "Move to start group",
            "Move to end group",
            "Move earlier",
            "Move later",
            "Overlay",
            "Dock",
            "Float",
            "Open in window"
        ],
        "every edge that holds panels, both groups, reordering and the modes"
    );
    let enabled: Vec<bool> = items.iter().map(|(_, enabled)| *enabled).collect();
    assert_eq!(
        enabled,
        [
            false, true, true, false, true, false, true, true, false, true, true
        ],
        "the current edge and group, and moving the first one earlier, are disabled"
    );
    choose(&mut ui, "Move later");
    assert_eq!(
        controller.placement("files")?,
        (ToolEdge::Left, StripeGroup::Start, 1)
    );
    assert!(placement(&ui).starts_with("search:left:start,files:left:start"));

    open_options(&mut ui, &root, "files");
    choose(&mut ui, "Move to Right");
    assert_eq!(
        controller.placement("files")?,
        (ToolEdge::Right, StripeGroup::Start, 1)
    );
    assert!(
        controller.is_expanded("files")?,
        "it stays expanded where it lands"
    );
    assert!(shown(&ui, "files-body"));
    let panel = rect(&ui, &root, "files");
    assert!(
        panel.x() > 400.0,
        "the panel is on the right now: {panel:?}"
    );

    open_options(&mut ui, &root, "files");
    choose(&mut ui, "Move to end group");
    assert_eq!(controller.placement("files")?.1, StripeGroup::End);

    open_options(&mut ui, &root, "files");
    choose(&mut ui, "Overlay");
    assert!(controller.is_overlay("files")?);

    let kinds: Vec<(ToolWindowChangeKind, ChangeOrigin)> =
        log.borrow().iter().map(|c| (c.kind, c.origin)).collect();
    assert_eq!(
        kinds,
        [
            (ToolWindowChangeKind::Placement, ChangeOrigin::User),
            (ToolWindowChangeKind::Placement, ChangeOrigin::User),
            (ToolWindowChangeKind::Placement, ChangeOrigin::User),
            (ToolWindowChangeKind::Mode, ChangeOrigin::User),
        ]
    );
    let last = log.borrow().last().cloned().unwrap();
    assert_eq!(
        (last.edge, last.group, last.overlay, last.expanded),
        (ToolEdge::Right, StripeGroup::End, true, true)
    );
    Ok(())
}

#[test]
fn the_header_places_its_options_menu_beside_the_hide_button() -> TestResult {
    let mut ui = ui();
    let controller = ToolWindowController::new();
    let root = ui.mount_tree(Element::new("frame", layout().controller(&controller)))?;
    flush(&mut ui);
    controller.expand("files")?;
    flush(&mut ui);
    let panel = find(&ui, &root, "files");
    let options = ui.bounds(&find(&ui, &panel, "options"))?.unwrap();
    let hide = ui.bounds(&find(&ui, &panel, "hide"))?.unwrap();
    assert!(options.width() > 0.0, "{options:?}");
    assert!(
        options.x() + options.width() <= hide.x(),
        "options {options:?} must end before hide {hide:?}"
    );
    assert!(
        ui.validation_report().issues.is_empty(),
        "{:?}",
        ui.validation_report().issues
    );
    Ok(())
}

#[test]
fn moving_an_expanded_tool_window_collapses_the_one_in_its_new_group() -> TestResult {
    let mut ui = ui();
    let log = Log::default();
    let controller = ToolWindowController::new();
    ui.mount_tree(Element::new(
        "frame",
        logged(layout(), &log).controller(&controller),
    ))?;
    flush(&mut ui);
    controller.expand("files")?;
    controller.expand("outline")?;
    flush(&mut ui);
    log.borrow_mut().clear();
    assert!(controller.move_to("files", ToolEdge::Right, StripeGroup::Start)?);
    flush(&mut ui);
    assert!(controller.is_expanded("files")?);
    assert!(!controller.is_expanded("outline")?);
    let changes: Vec<(String, ToolWindowChangeKind, ChangeOrigin)> = log
        .borrow()
        .iter()
        .map(|c| (c.key.clone(), c.kind, c.origin))
        .collect();
    assert_eq!(
        changes,
        [
            (
                "outline".into(),
                ToolWindowChangeKind::Expansion,
                ChangeOrigin::Application
            ),
            (
                "files".into(),
                ToolWindowChangeKind::Placement,
                ChangeOrigin::Application
            ),
        ]
    );
    // An edge without panels is refused; moving nowhere changes nothing.
    assert_eq!(
        controller
            .move_to("files", ToolEdge::Top, StripeGroup::Start)
            .err(),
        Some(RuntimeError::Layout(LayoutError::UnavailableEdge))
    );
    assert_eq!(
        controller
            .move_to("missing", ToolEdge::Right, StripeGroup::Start)
            .err(),
        Some(RuntimeError::Layout(LayoutError::UnknownKey))
    );
    assert!(!controller.move_to("files", ToolEdge::Right, StripeGroup::Start)?);
    assert!(controller.reorder("search", 9)?.eq(&false));
    assert!(controller.reorder("files", 0)?);
    assert_eq!(
        controller.placement("files")?,
        (ToolEdge::Right, StripeGroup::Start, 0)
    );
    Ok(())
}

#[test]
fn dragging_a_stripe_button_moves_and_reorders() -> TestResult {
    let mut ui = ui();
    let log = Log::default();
    let controller = ToolWindowController::new();
    let root = ui.mount_tree(Element::new(
        "frame",
        logged(layout(), &log).controller(&controller),
    ))?;
    flush(&mut ui);

    // Within the left stripe: drop `files` below `search`.
    let files = button_at(&ui, &root, "stripe-left", 0);
    let below = button_at(&ui, &root, "stripe-left", 1);
    drag(&mut ui, files, (below.0, below.1 + 6.0));
    assert_eq!(
        controller.placement("files")?,
        (ToolEdge::Left, StripeGroup::Start, 1)
    );
    assert!(!controller.is_expanded("files")?, "a drag is not a click");

    // To the right stripe, before `outline`.
    let files = button_at(&ui, &root, "stripe-left", 1);
    let outline = button_at(&ui, &root, "stripe-right", 0);
    drag(&mut ui, files, (outline.0, outline.1 - 6.0));
    assert_eq!(
        controller.placement("files")?,
        (ToolEdge::Right, StripeGroup::Start, 0)
    );

    // Far down the right stripe lands in its end group.
    let files = button_at(&ui, &root, "stripe-right", 0);
    let stripe = rect(&ui, &root, "stripe-right");
    drag(
        &mut ui,
        files,
        (
            stripe.x() + stripe.width() / 2.0,
            stripe.y() + stripe.height() - 4.0,
        ),
    );
    assert_eq!(
        controller.placement("files")?,
        (ToolEdge::Right, StripeGroup::End, 0)
    );
    assert!(
        log.borrow()
            .iter()
            .all(|c| c.kind == ToolWindowChangeKind::Placement && c.origin == ChangeOrigin::User)
    );
    assert_eq!(log.borrow().len(), 3);
    Ok(())
}

#[test]
fn escape_cancels_a_drag_and_another_layout_rejects_it() -> TestResult {
    let mut ui = ui();
    let controller = ToolWindowController::new();
    let inner_controller = ToolWindowController::new();
    let inner = ToolWindowLayout::new(button("inner-main"))
        .edge(ToolEdge::Left, EdgeMode::Stripe)
        .window(window("inner-files", ToolEdge::Left))
        .controller(&inner_controller);
    let outer = ToolWindowLayout::new(Element::new("inner", inner))
        .edge(ToolEdge::Left, EdgeMode::Stripe)
        .edge(ToolEdge::Right, EdgeMode::Stripe)
        .window(window("files", ToolEdge::Left))
        .window(window("outline", ToolEdge::Right))
        .controller(&controller);
    let root = ui.mount_tree(Element::new("frame", outer))?;
    flush(&mut ui);

    let files = button_at(&ui, &root, "stripe-left", 0);
    let outline = button_at(&ui, &root, "stripe-right", 0);
    pointer(
        &mut ui,
        files.0,
        files.1,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, files.0 + 8.0, files.1 + 8.0, PointerKind::Move);
    pointer(&mut ui, outline.0, outline.1, PointerKind::Move);
    assert!(ui.inspect().drag.is_some(), "a drag session is running");
    press(&mut ui, KeyCode::Escape);
    pointer(
        &mut ui,
        outline.0,
        outline.1,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(
        controller.placement("files")?,
        (ToolEdge::Left, StripeGroup::Start, 0)
    );

    // The nested layout's stripe belongs to another layout.
    let inner_root = find(&ui, &root, "inner");
    let inner_stripe = find(&ui, &inner_root, "stripe-left");
    let target = ui.bounds(&inner_stripe)?.unwrap();
    drag(
        &mut ui,
        files,
        (target.x() + target.width() / 2.0, target.y() + 40.0),
    );
    assert_eq!(
        controller.placement("files")?,
        (ToolEdge::Left, StripeGroup::Start, 0)
    );
    assert_eq!(
        inner_controller.placement("inner-files")?,
        (ToolEdge::Left, StripeGroup::Start, 0)
    );
    Ok(())
}

#[test]
fn an_overlay_panel_lies_over_the_center_and_hides_when_focus_leaves() -> TestResult {
    let mut ui = ui();
    let log = Log::default();
    let controller = ToolWindowController::new();
    let root = ui.mount_tree(Element::new(
        "frame",
        logged(
            layout().window(
                window("notes", ToolEdge::Left)
                    .group(StripeGroup::End)
                    .overlay(true),
            ),
            &log,
        )
        .controller(&controller),
    ))?;
    flush(&mut ui);
    let center_before = rect(&ui, &root, "main");
    assert!(controller.is_overlay("notes")?);

    // Expanding from the stripe moves focus in; the center keeps its size.
    let notes = button_at(&ui, &root, "stripe-left", 2);
    let stripe = rect(&ui, &root, "stripe-left");
    let notes = (notes.0, stripe.y() + stripe.height() - stripe.width() / 2.0);
    click(&mut ui, notes.0, notes.1);
    assert!(shown(&ui, "notes-body"));
    assert_eq!(rect(&ui, &root, "main"), center_before);
    let panel = rect(&ui, &root, "notes");
    assert!(panel.x() < center_before.x() + 10.0 && panel.x() + panel.width() > center_before.x());
    assert_eq!(property(&ui, "notes", "overlay"), InspectValue::Bool(true));

    // Focus moving to the center hides it.
    ui.focus(&find(&ui, &root, "main"))?;
    flush(&mut ui);
    assert!(!shown(&ui, "notes-body"));
    assert_eq!(
        log.borrow().last().map(|c| (c.kind, c.expanded, c.origin)),
        Some((ToolWindowChangeKind::Expansion, false, ChangeOrigin::User))
    );

    // A click on its stripe button while it holds focus collapses it once:
    // the press hides it as focus leaves, and the release does not reopen it.
    click(&mut ui, notes.0, notes.1);
    assert!(shown(&ui, "notes-body"));
    click(&mut ui, notes.0, notes.1);
    assert!(!shown(&ui, "notes-body"));

    // Focus moving from the panel to the stripe keeps it open; focus then
    // leaving the stripe for the center hides it.
    controller.expand("notes")?;
    flush(&mut ui);
    ui.focus(&find(&ui, &root, "notes-body"))?;
    flush(&mut ui);
    ui.focus(&find(&ui, &root, "stripe-left"))?;
    flush(&mut ui);
    assert!(shown(&ui, "notes-body"));
    ui.focus(&find(&ui, &root, "main"))?;
    flush(&mut ui);
    assert!(!shown(&ui, "notes-body"));
    // Expanded by the application without focus, it stays open.
    controller.expand("notes")?;
    flush(&mut ui);
    ui.focus(&find(&ui, &root, "main"))?;
    flush(&mut ui);
    assert!(shown(&ui, "notes-body"));
    controller.collapse("notes")?;

    // Switching to docked gives it space from the center.
    controller.set_overlay("notes", false)?;
    controller.expand("notes")?;
    flush(&mut ui);
    assert!(rect(&ui, &root, "main").x() > center_before.x());
    Ok(())
}

#[test]
fn an_overlay_panel_stays_open_while_its_options_menu_is_used() -> TestResult {
    let mut ui = ui();
    let controller = ToolWindowController::new();
    let root = ui.mount_tree(Element::new(
        "frame",
        layout()
            .window(
                window("notes", ToolEdge::Left)
                    .group(StripeGroup::End)
                    .overlay(true),
            )
            .controller(&controller),
    ))?;
    flush(&mut ui);
    controller.expand("notes")?;
    flush(&mut ui);
    ui.focus(&find(&ui, &root, "notes-body"))?;
    flush(&mut ui);
    open_options(&mut ui, &root, "notes");
    assert!(
        shown(&ui, "notes-body"),
        "opening the panel's own menu keeps it open"
    );
    choose(&mut ui, "Overlay");
    assert!(!controller.is_overlay("notes")?);
    assert!(controller.is_expanded("notes")?);
    Ok(())
}

#[test]
fn locked_and_denied_permissions_restrict_moving_reordering_dragging_and_modes() -> TestResult {
    let mut ui = ui();
    let controller = ToolWindowController::new();
    let root = ui.mount_tree(Element::new(
        "frame",
        layout().locked().controller(&controller),
    ))?;
    flush(&mut ui);
    controller.expand("files")?;
    flush(&mut ui);
    let panel = find(&ui, &root, "files");
    assert!(
        ui.find_descendant(&panel, "options")?.is_none(),
        "a locked layout has no options menu"
    );
    let files = button_at(&ui, &root, "stripe-left", 0);
    let outline = button_at(&ui, &root, "stripe-right", 0);
    drag(&mut ui, files, outline);
    assert_eq!(
        controller.placement("files")?,
        (ToolEdge::Left, StripeGroup::Start, 0)
    );
    // The application still moves it.
    assert!(controller.move_to("files", ToolEdge::Right, StripeGroup::Start)?);
    ui.unmount(&root)?;

    // Bound permissions, each denied in turn.
    let movable = ui.signal(false);
    let reorderable = ui.signal(true);
    let overlayable = ui.signal(false);
    let draggable = ui.signal(true);
    let controller = ToolWindowController::new();
    let root = ui.mount_tree(Element::new(
        "frame",
        layout()
            .movable(movable.read_only())
            .reorderable(reorderable.read_only())
            .overlayable(overlayable.read_only())
            .draggable(draggable.read_only())
            .floatable(false)
            .detachable(false)
            .controller(&controller),
    ))?;
    flush(&mut ui);
    controller.expand("files")?;
    flush(&mut ui);
    open_options(&mut ui, &root, "files");
    let enabled: Vec<String> = menu_items(&ui)
        .into_iter()
        .filter(|(_, enabled)| *enabled)
        .map(|(name, _)| name)
        .collect();
    assert_eq!(enabled, ["Move later"], "only reordering is allowed");
    press(&mut ui, KeyCode::Escape);

    // A drag to another stripe is forbidden while moving is denied, but a
    // drag within the stripe reorders.
    let files = button_at(&ui, &root, "stripe-left", 0);
    drag(&mut ui, files, outline);
    assert_eq!(controller.placement("files")?.0, ToolEdge::Left);
    let below = button_at(&ui, &root, "stripe-left", 1);
    drag(&mut ui, files, (below.0, below.1 + 6.0));
    assert_eq!(
        controller.placement("files")?,
        (ToolEdge::Left, StripeGroup::Start, 1)
    );

    // Granting moving at run time allows the drag; denying dragging stops it.
    movable.set(true)?;
    flush(&mut ui);
    let files = button_at(&ui, &root, "stripe-left", 1);
    drag(&mut ui, files, outline);
    assert_eq!(controller.placement("files")?.0, ToolEdge::Right);
    draggable.set(false)?;
    flush(&mut ui);
    let outline_files = button_at(&ui, &root, "stripe-right", 0);
    let left = button_at(&ui, &root, "stripe-left", 0);
    drag(&mut ui, outline_files, left);
    assert_eq!(controller.placement("files")?.0, ToolEdge::Right);

    // A tool window's own permission is stricter than the layout's.
    ui.unmount(&root)?;
    let controller = ToolWindowController::new();
    let root = ui.mount_tree(Element::new(
        "frame",
        ToolWindowLayout::new(button("main"))
            .edge(ToolEdge::Left, EdgeMode::Stripe)
            .edge(ToolEdge::Right, EdgeMode::Stripe)
            .window(window("files", ToolEdge::Left).movable(false))
            .window(window("outline", ToolEdge::Right))
            .controller(&controller),
    ))?;
    flush(&mut ui);
    let files = button_at(&ui, &root, "stripe-left", 0);
    let outline = button_at(&ui, &root, "stripe-right", 0);
    drag(&mut ui, files, outline);
    assert_eq!(controller.placement("files")?.0, ToolEdge::Left);
    Ok(())
}

#[test]
fn placement_survives_redescription_until_the_edge_goes_away() -> TestResult {
    let mut ui = ui();
    let controller = ToolWindowController::new();
    let root = ui.mount_tree(Element::new(
        "column",
        Column::new([Element::new("frame", layout().controller(&controller))]),
    ))?;
    flush(&mut ui);
    controller.move_to("files", ToolEdge::Right, StripeGroup::Start)?;
    controller.set_overlay("search", true)?;
    controller.reorder("outline", 0)?;
    flush(&mut ui);

    // Re-described with a new tool window and a new edge for `log`, which
    // the user never moved.
    let next = || {
        ToolWindowLayout::new(button("main"))
            .edge(ToolEdge::Left, EdgeMode::Stripe)
            .edge(ToolEdge::Right, EdgeMode::Stripe)
            .edge(ToolEdge::Bottom, EdgeMode::Panels)
            .window(window("files", ToolEdge::Left))
            .window(window("search", ToolEdge::Left))
            .window(window("outline", ToolEdge::Right))
            .window(window("log", ToolEdge::Right))
            .window(window("tasks", ToolEdge::Left))
            .controller(&controller)
    };
    ui.reconcile_children(&root, vec![Element::new("frame", next())])?;
    flush(&mut ui);
    assert_eq!(
        controller.placement("files")?.0,
        ToolEdge::Right,
        "a move survives"
    );
    assert_eq!(
        controller.placement("outline")?,
        (ToolEdge::Right, StripeGroup::Start, 0)
    );
    assert_eq!(
        controller.placement("log")?.0,
        ToolEdge::Right,
        "an unmoved one follows"
    );
    assert!(controller.is_overlay("search")?, "a chosen mode survives");
    assert_eq!(controller.placement("tasks")?.0, ToolEdge::Left);

    // When the right edge stops holding panels, the move falls back.
    ui.reconcile_children(
        &root,
        vec![Element::new(
            "frame",
            ToolWindowLayout::new(button("main"))
                .edge(ToolEdge::Left, EdgeMode::Stripe)
                .window(window("files", ToolEdge::Left))
                .controller(&controller),
        )],
    )?;
    flush(&mut ui);
    assert_eq!(controller.placement("files")?.0, ToolEdge::Left);
    Ok(())
}

#[test]
fn workspace_round_trips_placement_order_and_modes() -> TestResult {
    let mut ui = ui();
    let log = Log::default();
    let controller = ToolWindowController::new();
    let root = ui.mount_tree(Element::new(
        "frame",
        logged(layout(), &log).controller(&controller),
    ))?;
    flush(&mut ui);
    controller.move_to("files", ToolEdge::Right, StripeGroup::End)?;
    controller.reorder("outline", 0)?;
    controller.set_overlay("search", true)?;
    flush(&mut ui);
    let saved = ui.capture_workspace()?;
    let (_, state) = saved.entries().next().unwrap();
    assert_eq!(
        state
            .map("edges")?
            .get("files")
            .and_then(WorkspaceValue::as_text),
        Some("right")
    );
    assert_eq!(
        state
            .map("groups")?
            .get("files")
            .and_then(WorkspaceValue::as_text),
        Some("end")
    );
    assert_eq!(state.map("order")?.len(), 4);
    assert_eq!(
        state
            .map("overlay")?
            .get("search")
            .and_then(WorkspaceValue::as_flag),
        Some(true)
    );

    // Change it back, then restore.
    controller.move_to("files", ToolEdge::Left, StripeGroup::Start)?;
    controller.set_overlay("search", false)?;
    flush(&mut ui);
    log.borrow_mut().clear();
    let encoded = Workspace::decode(&saved.encode())?;
    assert!(ui.restore_workspace(&encoded)?.is_complete());
    flush(&mut ui);
    assert_eq!(
        controller.placement("files")?,
        (ToolEdge::Right, StripeGroup::End, 0)
    );
    assert!(controller.is_overlay("search")?);
    assert_eq!(ui.capture_workspace()?.encode(), saved.encode());
    assert!(
        log.borrow()
            .iter()
            .all(|c| c.origin == ChangeOrigin::Restore)
    );
    assert!(
        log.borrow()
            .iter()
            .any(|c| c.kind == ToolWindowChangeKind::Placement && c.key == "files")
    );

    // A document from before placement existed still restores: the tool
    // windows return to their described places and modes, and the ones it
    // leaves out of `expanded` collapse.
    let path = WorkspacePath::new(["frame"])?;
    let old = ViewState::new("tool-windows")?
        .with(
            "expanded",
            WorkspaceValue::map([("log".to_owned(), WorkspaceValue::flag(true))])?,
        )?
        .with(
            "extents",
            WorkspaceValue::map(Vec::<(String, WorkspaceValue)>::new())?,
        )?;
    assert!(
        ui.restore_workspace(&Workspace::new(
            WorkspaceSettings::default(),
            [(path.clone(), old)]
        )?)?
        .is_complete()
    );
    flush(&mut ui);
    assert!(controller.is_expanded("log")?);
    assert_eq!(
        controller.placement("files")?,
        (ToolEdge::Left, StripeGroup::Start, 0)
    );
    assert!(!controller.is_overlay("search")?);
    // Back to the saved arrangement for the rejection checks below.
    assert!(ui.restore_workspace(&encoded)?.is_complete());
    flush(&mut ui);

    // `expanded` and `extents` are required.
    let partial = ViewState::new("tool-windows")?.with(
        "extents",
        WorkspaceValue::map(Vec::<(String, WorkspaceValue)>::new())?,
    )?;
    assert_eq!(
        ui.restore_workspace(&Workspace::new(
            WorkspaceSettings::default(),
            [(path.clone(), partial)]
        )?)?
        .rejected()
        .map(|(_, reason)| reason.clone())
        .collect::<Vec<_>>(),
        [RestoreRejection::Value("expanded".into())]
    );

    let restore = |ui: &mut Runtime, field: &str, value: WorkspaceValue| {
        let empty = || WorkspaceValue::map(Vec::<(String, WorkspaceValue)>::new()).unwrap();
        let mut state = ViewState::new("tool-windows")
            .unwrap()
            .with(field, value)
            .unwrap();
        for required in ["expanded", "extents"] {
            if field != required {
                state = state.with(required, empty()).unwrap();
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
    let texts = |entries: &[(&str, &str)]| {
        WorkspaceValue::map(
            entries
                .iter()
                .map(|(key, text)| ((*key).to_owned(), WorkspaceValue::text(*text).unwrap())),
        )
        .unwrap()
    };
    assert_eq!(
        restore(&mut ui, "edges", texts(&[("files", "middle")])),
        [RestoreRejection::Value("edges".into())]
    );
    assert_eq!(
        restore(&mut ui, "edges", texts(&[("files", "top")])),
        [RestoreRejection::OutOfRange("edges".into())],
        "the top edge holds no panels"
    );
    assert_eq!(
        restore(&mut ui, "groups", texts(&[("files", "middle")])),
        [RestoreRejection::Value("groups".into())]
    );
    assert_eq!(
        restore(&mut ui, "edges", texts(&[("ghost", "left")])),
        [RestoreRejection::Unresolved("ghost".into())]
    );
    let positions = |entries: &[(&str, i64)]| {
        WorkspaceValue::map(
            entries
                .iter()
                .map(|(key, position)| ((*key).to_owned(), WorkspaceValue::integer(*position))),
        )
        .unwrap()
    };
    assert_eq!(
        restore(&mut ui, "order", positions(&[("files", 0), ("search", 0)])),
        [RestoreRejection::OutOfRange("order".into())]
    );
    assert_eq!(
        restore(&mut ui, "order", positions(&[("files", -1)])),
        [RestoreRejection::OutOfRange("order".into())]
    );
    assert_eq!(
        restore(
            &mut ui,
            "overlay",
            WorkspaceValue::map([("search".to_owned(), WorkspaceValue::integer(1))])?
        ),
        [RestoreRejection::Value("overlay".into())]
    );
    // A tiny negative extent is negative, not -0.0.
    assert_eq!(
        restore(
            &mut ui,
            "extents",
            WorkspaceValue::map([("files".to_owned(), WorkspaceValue::number(-1e-50)?)])?
        ),
        [RestoreRejection::OutOfRange("extents".into())]
    );
    // A rejected document changed nothing.
    assert_eq!(
        controller.placement("files")?,
        (ToolEdge::Right, StripeGroup::End, 0)
    );
    let _ = root;
    Ok(())
}

#[test]
fn an_overlay_hides_after_the_stripe_that_held_focus_is_removed() -> TestResult {
    let mut ui = ui();
    let controller = ToolWindowController::new();
    let parent = ui.mount_tree(Element::new(
        "column",
        Column::new([Element::new("frame", layout().controller(&controller))]),
    ))?;
    flush(&mut ui);
    let root = find(&ui, &parent, "frame");
    controller.set_overlay("files", true)?;
    controller.expand("files")?;
    flush(&mut ui);
    ui.focus(&find(&ui, &root, "files-body"))?;
    flush(&mut ui);
    ui.focus(&find(&ui, &root, "stripe-left"))?;
    flush(&mut ui);
    assert!(controller.is_expanded("files")?, "a stripe keeps it open");
    // The left edge loses its stripe while the stripe holds focus.
    let panels = ToolWindowLayout::new(button("main"))
        .edge(ToolEdge::Left, EdgeMode::Panels)
        .edge(ToolEdge::Right, EdgeMode::Stripe)
        .edge(ToolEdge::Bottom, EdgeMode::Panels)
        .window(window("files", ToolEdge::Left))
        .window(window("search", ToolEdge::Left))
        .window(window("outline", ToolEdge::Right))
        .window(window("log", ToolEdge::Bottom))
        .controller(&controller);
    ui.reconcile_children(&parent, vec![Element::new("frame", panels)])?;
    flush(&mut ui);
    ui.focus(&find(&ui, &root, "main"))?;
    flush(&mut ui);
    assert!(
        !controller.is_expanded("files")?,
        "focus is in neither the overlay nor a stripe"
    );
    Ok(())
}

#[test]
fn revoking_the_drag_permission_during_a_drag_forbids_the_drop() -> TestResult {
    let mut ui = ui();
    let draggable = ui.signal(true);
    let controller = ToolWindowController::new();
    let root = ui.mount_tree(Element::new(
        "frame",
        layout()
            .draggable(draggable.read_only())
            .controller(&controller),
    ))?;
    flush(&mut ui);
    let files = button_at(&ui, &root, "stripe-left", 0);
    let outline = button_at(&ui, &root, "stripe-right", 1);
    pointer(
        &mut ui,
        files.0,
        files.1,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, files.0 + 6.0, files.1 + 6.0, PointerKind::Move);
    pointer(&mut ui, files.0 + 30.0, files.1 + 30.0, PointerKind::Move);
    draggable.set(false)?;
    flush(&mut ui);
    pointer(&mut ui, outline.0, outline.1, PointerKind::Move);
    pointer(
        &mut ui,
        outline.0,
        outline.1,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(controller.placement("files")?.0, ToolEdge::Left);
    Ok(())
}

#[test]
fn a_restored_order_keeps_omitted_tool_windows_in_their_described_slots() -> TestResult {
    let mut ui = ui();
    let log = Log::default();
    let controller = ToolWindowController::new();
    ui.mount_tree(Element::new(
        "frame",
        logged(layout(), &log).controller(&controller),
    ))?;
    flush(&mut ui);
    let path = WorkspacePath::new(["frame"])?;
    let restore = |ui: &mut Runtime, order: &[(&str, i64)]| {
        let empty = || WorkspaceValue::map(Vec::<(String, WorkspaceValue)>::new()).unwrap();
        let state = ViewState::new("tool-windows")
            .unwrap()
            .with("expanded", empty())
            .unwrap()
            .with("extents", empty())
            .unwrap()
            .with(
                "order",
                WorkspaceValue::map(order.iter().map(|(key, position)| {
                    ((*key).to_owned(), WorkspaceValue::integer(*position))
                }))
                .unwrap(),
            )
            .unwrap();
        ui.restore_workspace(
            &Workspace::new(WorkspaceSettings::default(), [(path.clone(), state)]).unwrap(),
        )
        .unwrap()
        .rejected()
        .map(|(_, reason)| reason.clone())
        .collect::<Vec<_>>()
    };
    // `files` takes the third slot; the others fill the rest in described
    // order.
    assert!(restore(&mut ui, &[("files", 2)]).is_empty());
    flush(&mut ui);
    assert_eq!(
        placement(&ui),
        "search:left:start,outline:right:start,files:left:start,log:bottom:start"
    );
    // An order-only change within one group is reported as a placement.
    log.borrow_mut().clear();
    assert!(restore(&mut ui, &[("search", 0), ("files", 1)]).is_empty());
    flush(&mut ui);
    assert!(placement(&ui).starts_with("search:left:start,files:left:start"));
    let placed: Vec<(String, ChangeOrigin)> = log
        .borrow()
        .iter()
        .filter(|c| c.kind == ToolWindowChangeKind::Placement)
        .map(|c| (c.key.clone(), c.origin))
        .collect();
    assert!(
        placed.contains(&("files".to_owned(), ChangeOrigin::Restore)),
        "{placed:?}"
    );
    // A position outside the layout is refused.
    assert_eq!(
        restore(&mut ui, &[("files", 99)]),
        [RestoreRejection::OutOfRange("order".into())]
    );
    Ok(())
}
