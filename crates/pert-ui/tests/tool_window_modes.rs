//! Tool window layouts, third slice: floating frames, tool windows in their
//! own native windows, their permissions, change reports and workspace
//! persistence. Native windows are driven headlessly: the test plays the
//! host, taking the runtime's presentation requests and committing them.
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

fn text(ui: &Runtime, key: &str, name: &str) -> String {
    match property(ui, key, name) {
        InspectValue::Text(text) => text,
        other => panic!("{name} is {other:?}"),
    }
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

fn child_rect(ui: &Runtime, layout: &WidgetId, key: &str) -> Rect {
    let child = ui
        .children(layout)
        .unwrap()
        .into_iter()
        .find(|child| ui.key(child).unwrap() == key)
        .unwrap_or_else(|| panic!("{key} is not a child"));
    ui.bounds(&child).unwrap().unwrap()
}

fn child(ui: &Runtime, layout: &WidgetId, key: &str) -> WidgetId {
    ui.children(layout)
        .unwrap()
        .into_iter()
        .find(|child| ui.key(child).unwrap() == key)
        .unwrap_or_else(|| panic!("{key} is not a child"))
}

fn window(key: &str) -> ToolWindow {
    ToolWindow::new(key, format!("{key} title"), button(&format!("{key}-body"))).icon(Icon::Folder)
}

/// A left stripe with `files` and `search` in its start group.
fn layout() -> ToolWindowLayout {
    ToolWindowLayout::new(button("main"))
        .edge(ToolEdge::Left, EdgeMode::Stripe)
        .window(window("files"))
        .window(window("search"))
}

fn logged(layout: ToolWindowLayout, log: &Log) -> ToolWindowLayout {
    let log = log.clone();
    layout.on_change(move |_, change| {
        log.borrow_mut().push(change.clone());
        Ok(())
    })
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

fn open_options(ui: &mut Runtime, root: &WidgetId, panel: &str) {
    let options = find(ui, &find(ui, root, panel), "options");
    let trigger = find(ui, &options, "button");
    ui.focus(&trigger).unwrap();
    flush(ui);
    press(ui, KeyCode::Enter);
}

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

/// Plays the native host for the queued detach of `view`: commits it and
/// reports it presented. Returns the new window.
fn host_detach(ui: &mut Runtime, view: &WidgetId) -> NativeWindowId {
    let primary = ui.primary_window();
    let requests = ui.take_presentation_requests().unwrap();
    let mut window = None;
    for request in requests {
        let PresentationAction::Detach {
            view: requested,
            options,
        } = request.action()
        else {
            continue;
        };
        assert_eq!(requested, view, "the panel itself moves");
        let options = options.clone();
        let prepared = ui.prepare_detach(view, options).unwrap();
        let committed = ui.commit_detach(prepared).unwrap();
        request
            .complete(PresentationStatus::Presented(committed.clone()))
            .unwrap();
        window = Some(committed);
    }
    ui.select_window(&primary).unwrap();
    flush(ui);
    window.expect("a detach was requested")
}

/// Flushes every window, as the native App does.
fn flush_all(ui: &mut Runtime, windows: &[NativeWindowId]) {
    let primary = ui.primary_window();
    for window in windows {
        if ui.select_window(window).is_ok() {
            flush(ui);
        }
    }
    ui.select_window(&primary).unwrap();
    flush(ui);
}

#[test]
fn float_and_dock_from_the_options_menu() -> TestResult {
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
    let docked = child_rect(&ui, &root, "files");
    log.borrow_mut().clear();

    open_options(&mut ui, &root, "files");
    let items = menu_items(&ui);
    let tail: Vec<(&str, bool)> = items
        .iter()
        .rev()
        .take(3)
        .rev()
        .map(|(name, enabled)| (name.as_str(), *enabled))
        .collect();
    assert_eq!(
        tail,
        [("Dock", false), ("Float", true), ("Open in window", true)],
        "the current mode is disabled"
    );
    choose(&mut ui, "Float");
    assert_eq!(controller.mode("files")?, ToolWindowMode::Floating);
    let floating = child_rect(&ui, &root, "files");
    assert_ne!(floating, docked, "the panel left its edge");
    assert!(
        floating.x() > docked.x() + docked.width() / 2.0,
        "the default frame is centered: {floating:?}"
    );
    assert!(shown(&ui, "files-body"));
    assert_eq!(text(&ui, "frame", "modes"), "files:floating");
    assert_eq!(text(&ui, "files", "mode"), "floating");

    // A floating tool window does not share its group: expanding the other
    // one keeps both open.
    controller.expand("search")?;
    flush(&mut ui);
    assert!(controller.is_expanded("files")? && controller.is_expanded("search")?);

    open_options(&mut ui, &root, "files");
    choose(&mut ui, "Dock");
    assert_eq!(controller.mode("files")?, ToolWindowMode::Docked);
    assert!(controller.is_expanded("files")?);
    assert!(
        !controller.is_expanded("search")?,
        "docking again collapses the one expanded in its group"
    );
    let changes: Vec<(String, ToolWindowChangeKind, ToolWindowMode, ChangeOrigin)> = log
        .borrow()
        .iter()
        .map(|c| (c.key.clone(), c.kind, c.mode, c.origin))
        .collect();
    assert!(changes.contains(&(
        "files".into(),
        ToolWindowChangeKind::Mode,
        ToolWindowMode::Floating,
        ChangeOrigin::User
    )));
    assert!(changes.contains(&(
        "files".into(),
        ToolWindowChangeKind::Mode,
        ToolWindowMode::Docked,
        ChangeOrigin::User
    )));
    assert!(changes.contains(&(
        "search".into(),
        ToolWindowChangeKind::Expansion,
        ToolWindowMode::Docked,
        ChangeOrigin::User
    )));
    Ok(())
}

#[test]
fn a_floating_frame_moves_and_resizes_and_escape_cancels() -> TestResult {
    let mut ui = ui();
    let controller = ToolWindowController::new();
    let root = ui.mount_tree(Element::new("frame", layout().controller(&controller)))?;
    flush(&mut ui);
    controller.set_mode("files", ToolWindowMode::Floating)?;
    controller.expand("files")?;
    flush(&mut ui);
    let body = find(&ui, &root, "files-body");
    let start = child_rect(&ui, &root, "files");

    // Drag the header: the frame follows, and stays on release.
    let (x, y) = (start.x() + 24.0, start.y() + 8.0);
    pointer(&mut ui, x, y, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, x + 40.0, y + 30.0, PointerKind::Move);
    pointer(
        &mut ui,
        x + 40.0,
        y + 30.0,
        PointerKind::Release(PointerButton::Primary),
    );
    let moved = child_rect(&ui, &root, "files");
    assert_eq!(
        (moved.x(), moved.y(), moved.width(), moved.height()),
        (
            start.x() + 40.0,
            start.y() + 30.0,
            start.width(),
            start.height()
        )
    );

    // Drag the corner grip: the frame grows.
    let (gx, gy) = (
        moved.x() + moved.width() - 2.0,
        moved.y() + moved.height() - 2.0,
    );
    pointer(&mut ui, gx, gy, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, gx + 20.0, gy + 10.0, PointerKind::Move);
    pointer(
        &mut ui,
        gx + 20.0,
        gy + 10.0,
        PointerKind::Release(PointerButton::Primary),
    );
    let resized = child_rect(&ui, &root, "files");
    assert_eq!(
        (resized.width(), resized.height()),
        (moved.width() + 20.0, moved.height() + 10.0)
    );

    // Escape during a move returns the frame from before it.
    let (x, y) = (resized.x() + 24.0, resized.y() + 8.0);
    pointer(&mut ui, x, y, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, x + 100.0, y + 50.0, PointerKind::Move);
    assert_ne!(child_rect(&ui, &root, "files"), resized);
    press(&mut ui, KeyCode::Escape);
    pointer(
        &mut ui,
        x + 100.0,
        y + 50.0,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(child_rect(&ui, &root, "files"), resized);

    // Alt+arrows move it, Alt+Shift+arrows resize it, with focus inside.
    ui.focus(&body)?;
    flush(&mut ui);
    ui.key_input(KeyCode::AltLeft, KeyAction::Press)?;
    press(&mut ui, KeyCode::ArrowRight);
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Press)?;
    press(&mut ui, KeyCode::ArrowDown);
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Release)?;
    ui.key_input(KeyCode::AltLeft, KeyAction::Release)?;
    flush(&mut ui);
    let keyed = child_rect(&ui, &root, "files");
    assert_eq!(keyed.x(), resized.x() + 8.0);
    assert_eq!(keyed.height(), resized.height() + 8.0);
    // The content was never remounted while the frame moved.
    assert_eq!(find(&ui, &root, "files-body"), body);
    Ok(())
}

#[test]
fn window_mode_detaches_the_panel_and_closing_returns_it_to_the_stripe() -> TestResult {
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
    let panel = child(&ui, &root, "files");
    let body = find(&ui, &root, "files-body");
    log.borrow_mut().clear();

    open_options(&mut ui, &root, "files");
    choose(&mut ui, "Open in window");
    assert_eq!(controller.mode("files")?, ToolWindowMode::Window);
    let window = host_detach(&mut ui, &panel);
    assert_eq!(text(&ui, "frame", "detached"), "files");
    assert_eq!(
        ui.window_of(&body)?,
        window,
        "the content moved with its state"
    );
    assert_eq!(find(&ui, &root, "files-body"), body);
    assert!(controller.is_expanded("files")?);

    // The user closes the native window: the host returns the panel, and the
    // tool window goes back to its stripe, still in window mode.
    log.borrow_mut().clear();
    ui.redock(&window)?;
    let primary = ui.primary_window();
    ui.select_window(&primary)?;
    flush(&mut ui);
    assert!(!controller.is_expanded("files")?);
    assert_eq!(controller.mode("files")?, ToolWindowMode::Window);
    assert_eq!(text(&ui, "frame", "detached"), "");
    assert_eq!(find(&ui, &root, "files-body"), body, "never remounted");
    assert!(log.borrow().iter().any(|c| c.key == "files"
        && c.kind == ToolWindowChangeKind::Expansion
        && !c.expanded
        && c.origin == ChangeOrigin::Presentation));

    // Expanding it again opens a new window.
    controller.expand("files")?;
    flush(&mut ui);
    let again = host_detach(&mut ui, &panel);
    assert_ne!(again, window);
    Ok(())
}

#[test]
fn collapsing_or_docking_a_detached_tool_window_asks_its_window_to_return() -> TestResult {
    let mut ui = ui();
    let log = Log::default();
    let controller = ToolWindowController::new();
    let root = ui.mount_tree(Element::new(
        "frame",
        logged(layout(), &log).controller(&controller),
    ))?;
    flush(&mut ui);
    controller.set_mode("files", ToolWindowMode::Window)?;
    controller.expand("files")?;
    flush(&mut ui);
    let panel = child(&ui, &root, "files");
    let window = host_detach(&mut ui, &panel);

    // Collapsing it: the panel asks its own window to return.
    controller.collapse("files")?;
    flush_all(&mut ui, std::slice::from_ref(&window));
    let requests = ui.take_presentation_requests()?;
    assert!(
        requests.iter().any(
            |request| matches!(request.action(), PresentationAction::Redock(w) if *w == window)
        ),
        "{requests:?}"
    );
    log.borrow_mut().clear();
    ui.redock(&window)?;
    let primary = ui.primary_window();
    ui.select_window(&primary)?;
    flush(&mut ui);
    assert!(!controller.is_expanded("files")?);
    assert!(
        log.borrow()
            .iter()
            .all(|c| c.origin != ChangeOrigin::Presentation),
        "a return the panel asked for reports nothing more: {:?}",
        log.borrow()
    );
    drop(requests);

    // Docking it from window mode while detached returns it docked and open.
    controller.expand("files")?;
    flush(&mut ui);
    let window = host_detach(&mut ui, &panel);
    controller.set_mode("files", ToolWindowMode::Docked)?;
    flush_all(&mut ui, std::slice::from_ref(&window));
    let requests = ui.take_presentation_requests()?;
    assert!(
        requests.iter().any(
            |request| matches!(request.action(), PresentationAction::Redock(w) if *w == window)
        )
    );
    ui.redock(&window)?;
    ui.select_window(&primary)?;
    flush(&mut ui);
    assert!(controller.is_expanded("files")?);
    assert_eq!(controller.mode("files")?, ToolWindowMode::Docked);
    assert!(shown(&ui, "files-body"));
    Ok(())
}

#[test]
fn a_failed_detach_returns_the_tool_window_to_docking() -> TestResult {
    let mut ui = ui();
    let log = Log::default();
    let controller = ToolWindowController::new();
    ui.mount_tree(Element::new(
        "frame",
        logged(layout(), &log).controller(&controller),
    ))?;
    flush(&mut ui);
    controller.set_mode("files", ToolWindowMode::Window)?;
    controller.expand("files")?;
    flush(&mut ui);
    log.borrow_mut().clear();
    let requests = ui.take_presentation_requests()?;
    assert_eq!(requests.len(), 1);
    for request in requests {
        request.complete(PresentationStatus::Failed(WindowError::Creation(
            "no display".into(),
        )))?;
    }
    flush(&mut ui);
    assert_eq!(controller.mode("files")?, ToolWindowMode::Docked);
    assert!(controller.is_expanded("files")?, "it shows docked instead");
    assert!(shown(&ui, "files-body"));
    assert!(log.borrow().iter().any(|c| c.key == "files"
        && c.kind == ToolWindowChangeKind::Mode
        && c.mode == ToolWindowMode::Docked
        && c.origin == ChangeOrigin::Presentation));
    Ok(())
}

#[test]
fn locked_permissions_deny_floating_and_detaching_but_not_the_application() -> TestResult {
    let mut ui = ui();
    let controller = ToolWindowController::new();
    let root = ui.mount_tree(Element::new(
        "frame",
        layout()
            .window(window("notes").floatable(false))
            .controller(&controller),
    ))?;
    flush(&mut ui);
    controller.expand("notes")?;
    flush(&mut ui);
    open_options(&mut ui, &root, "notes");
    let items = menu_items(&ui);
    let tail: Vec<(&str, bool)> = items
        .iter()
        .rev()
        .take(3)
        .rev()
        .map(|(name, enabled)| (name.as_str(), *enabled))
        .collect();
    assert_eq!(
        tail,
        [("Dock", false), ("Float", false), ("Open in window", true)],
        "a tool window that may not float"
    );
    press(&mut ui, KeyCode::Escape);

    // The application still floats it, but the user cannot move the frame.
    controller.set_mode("notes", ToolWindowMode::Floating)?;
    flush(&mut ui);
    let start = child_rect(&ui, &root, "notes");
    let (x, y) = (start.x() + 24.0, start.y() + 8.0);
    pointer(&mut ui, x, y, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, x + 40.0, y + 30.0, PointerKind::Move);
    pointer(
        &mut ui,
        x + 40.0,
        y + 30.0,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(child_rect(&ui, &root, "notes"), start);

    // A locked layout offers no options menu at all, and a dynamic denial
    // disables the rows.
    let mut ui = self::ui();
    let root = ui.mount_tree(Element::new(
        "frame",
        layout().window(window("notes").expanded(true)).locked(),
    ))?;
    flush(&mut ui);
    assert!(
        ui.find_descendant(&find(&ui, &root, "notes"), "options")?
            .is_none(),
        "nothing to choose"
    );

    let mut ui = self::ui();
    let allowed = ui.signal(true);
    let controller = ToolWindowController::new();
    let root = ui.mount_tree(Element::new(
        "frame",
        layout()
            .detachable(allowed.read_only())
            .controller(&controller),
    ))?;
    flush(&mut ui);
    controller.expand("files")?;
    flush(&mut ui);
    allowed.set(false)?;
    flush(&mut ui);
    open_options(&mut ui, &root, "files");
    assert_eq!(
        menu_items(&ui).last(),
        Some(&("Open in window".to_owned(), false))
    );
    Ok(())
}

#[test]
fn workspace_round_trips_modes_and_floating_frames() -> TestResult {
    let mut ui = ui();
    let controller = ToolWindowController::new();
    let root = ui.mount_tree(Element::new("frame", layout().controller(&controller)))?;
    flush(&mut ui);
    controller.set_mode("files", ToolWindowMode::Floating)?;
    controller.expand("files")?;
    controller.set_mode("search", ToolWindowMode::Window)?;
    flush(&mut ui);
    // Put a frame of its own on `files`.
    let start = child_rect(&ui, &root, "files");
    let (x, y) = (start.x() + 24.0, start.y() + 8.0);
    pointer(&mut ui, x, y, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, x - 50.0, y - 40.0, PointerKind::Move);
    pointer(
        &mut ui,
        x - 50.0,
        y - 40.0,
        PointerKind::Release(PointerButton::Primary),
    );
    let framed = child_rect(&ui, &root, "files");
    let saved = ui.capture_workspace()?;
    let (path, state) = saved.entries().next().unwrap();
    let path = path.clone();
    assert_eq!(
        state
            .map("modes")?
            .get("files")
            .and_then(WorkspaceValue::as_text),
        Some("floating")
    );
    assert_eq!(
        state
            .map("modes")?
            .get("search")
            .and_then(WorkspaceValue::as_text),
        Some("window")
    );
    assert_eq!(
        state
            .map("float-x")?
            .get("files")
            .and_then(WorkspaceValue::as_number),
        Some(f64::from(framed.x()))
    );

    controller.set_mode("files", ToolWindowMode::Docked)?;
    controller.set_mode("search", ToolWindowMode::Docked)?;
    flush(&mut ui);
    let encoded = Workspace::decode(&saved.encode())?;
    assert!(ui.restore_workspace(&encoded)?.is_complete());
    flush(&mut ui);
    assert_eq!(controller.mode("files")?, ToolWindowMode::Floating);
    assert_eq!(controller.mode("search")?, ToolWindowMode::Window);
    assert_eq!(child_rect(&ui, &root, "files"), framed);
    assert_eq!(ui.capture_workspace()?.encode(), saved.encode());
    assert!(
        ui.take_presentation_requests()?.is_empty(),
        "a restore opens no native window"
    );

    // A document that expands a tool window in window mode restores it
    // collapsed: native windows are not reopened.
    let empty = || WorkspaceValue::map(Vec::<(String, WorkspaceValue)>::new()).unwrap();
    let state = ViewState::new("tool-windows")?
        .with(
            "expanded",
            WorkspaceValue::map([("search".to_owned(), WorkspaceValue::flag(true))])?,
        )?
        .with("extents", empty())?
        .with(
            "modes",
            WorkspaceValue::map([("search".to_owned(), WorkspaceValue::text("window")?)])?,
        )?;
    assert!(
        ui.restore_workspace(&Workspace::new(
            WorkspaceSettings::default(),
            [(path.clone(), state)]
        )?)?
        .is_complete()
    );
    flush(&mut ui);
    assert!(!controller.is_expanded("search")?);
    assert!(ui.take_presentation_requests()?.is_empty());

    let restore = |ui: &mut Runtime, extra: Vec<(&str, WorkspaceValue)>| {
        let mut state = ViewState::new("tool-windows")
            .unwrap()
            .with("expanded", empty())
            .unwrap()
            .with("extents", empty())
            .unwrap();
        for (field, value) in extra {
            state = state.with(field, value).unwrap();
        }
        ui.restore_workspace(
            &Workspace::new(WorkspaceSettings::default(), [(path.clone(), state)]).unwrap(),
        )
        .unwrap()
        .rejected()
        .map(|(_, reason)| reason.clone())
        .collect::<Vec<_>>()
    };
    let numbers = |value: f64| {
        WorkspaceValue::map([("files".to_owned(), WorkspaceValue::number(value).unwrap())]).unwrap()
    };
    assert_eq!(
        restore(
            &mut ui,
            vec![(
                "modes",
                WorkspaceValue::map([("files".to_owned(), WorkspaceValue::text("orbit").unwrap())])
                    .unwrap()
            )]
        ),
        [RestoreRejection::Value("modes".into())]
    );
    assert_eq!(
        restore(&mut ui, vec![("float-x", numbers(10.0))]),
        [RestoreRejection::Value("float-x".into())],
        "a frame needs all four numbers"
    );
    assert_eq!(
        restore(
            &mut ui,
            vec![
                ("float-x", numbers(10.0)),
                ("float-y", numbers(10.0)),
                ("float-width", numbers(-5.0)),
                ("float-height", numbers(100.0)),
            ]
        ),
        [RestoreRejection::OutOfRange("float-width".into())]
    );
    Ok(())
}

#[test]
fn re_description_keeps_modes_frames_and_detached_panels() -> TestResult {
    let mut ui = ui();
    let controller = ToolWindowController::new();
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([Element::new("frame", layout().controller(&controller))]),
    ))?;
    flush(&mut ui);
    let frame_id = child(&ui, &root, "frame");
    controller.set_mode("files", ToolWindowMode::Floating)?;
    controller.expand("files")?;
    controller.set_mode("search", ToolWindowMode::Window)?;
    controller.expand("search")?;
    flush(&mut ui);
    let panel = child(&ui, &frame_id, "search");
    let window = host_detach(&mut ui, &panel);
    let frame = child_rect(&ui, &frame_id, "files");
    let body = find(&ui, &frame_id, "files-body");

    ui.reconcile_children(
        &root,
        vec![Element::new(
            "frame",
            layout()
                .window(window_named("extra"))
                .controller(&controller),
        )],
    )?;
    flush(&mut ui);
    assert_eq!(controller.mode("files")?, ToolWindowMode::Floating);
    assert_eq!(controller.mode("search")?, ToolWindowMode::Window);
    // The layout's own height follows its parent's measurement, which the
    // new tool window changes; the frame keeps its place and width.
    let kept = child_rect(&ui, &frame_id, "files");
    assert_eq!((kept.x(), kept.width()), (frame.x(), frame.width()));
    assert_eq!(find(&ui, &frame_id, "files-body"), body);
    assert_eq!(ui.window_of(&panel)?, window, "the detached panel stays");
    Ok(())
}

fn window_named(key: &str) -> ToolWindow {
    window(key)
}

#[test]
fn floating_gestures_block_restore_and_shrink_to_valid_preferences() -> TestResult {
    let mut ui = ui();
    let controller = ToolWindowController::new();
    let root = ui.mount_tree(Element::new("frame", layout().controller(&controller)))?;
    flush(&mut ui);
    controller.set_mode("files", ToolWindowMode::Floating)?;
    controller.expand("files")?;
    flush(&mut ui);
    let saved = ui.capture_workspace()?;
    let start = child_rect(&ui, &root, "files");
    let (x, y) = (start.x() + 24.0, start.y() + 8.0);
    pointer(&mut ui, x, y, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, x + 10.0, y + 10.0, PointerKind::Move);
    assert!(
        ui.restore_workspace(&saved)?
            .rejected()
            .any(|(_, rejection)| *rejection == RestoreRejection::Busy)
    );
    press(&mut ui, KeyCode::Escape);
    assert_eq!(ui.capture_workspace()?, saved);
    let (x, y) = (
        start.x() + start.width() - 2.0,
        start.y() + start.height() - 2.0,
    );
    pointer(&mut ui, x, y, PointerKind::Press(PointerButton::Primary));
    assert!(
        ui.restore_workspace(&saved)?
            .rejected()
            .any(|(_, rejection)| *rejection == RestoreRejection::Busy)
    );
    pointer(&mut ui, x - 1000.0, y - 1000.0, PointerKind::Move);
    pointer(
        &mut ui,
        x - 1000.0,
        y - 1000.0,
        PointerKind::Release(PointerButton::Primary),
    );
    let shrunk = ui.capture_workspace()?;
    assert!(ui.restore_workspace(&shrunk)?.is_complete());
    assert_eq!(ui.capture_workspace()?, shrunk);
    assert_eq!(
        (
            child_rect(&ui, &root, "files").width(),
            child_rect(&ui, &root, "files").height()
        ),
        (160.0, 120.0)
    );
    ui.key_input(KeyCode::AltLeft, KeyAction::Press)?;
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Press)?;
    for _ in 0..32 {
        press(&mut ui, KeyCode::ArrowLeft);
        press(&mut ui, KeyCode::ArrowUp);
    }
    ui.key_input(KeyCode::AltLeft, KeyAction::Release)?;
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Release)?;
    let keyboard_shrunk = ui.capture_workspace()?;
    assert!(ui.restore_workspace(&keyboard_shrunk)?.is_complete());
    controller.set_mode("files", ToolWindowMode::Window)?;
    flush(&mut ui);
    let requests = ui.take_presentation_requests()?;
    assert_eq!(requests.len(), 1);
    if let PresentationAction::Detach { options, .. } = requests[0].action() {
        assert_eq!(options.initial_size(), Size::new(160.0, 120.0)?);
    } else {
        panic!("expected detach");
    }
    Ok(())
}

#[test]
fn revoking_float_permission_cancels_and_restores_an_active_move() -> TestResult {
    let mut ui = ui();
    let allowed = ui.signal(true);
    let controller = ToolWindowController::new();
    let root = ui.mount_tree(Element::new(
        "frame",
        layout()
            .floatable(allowed.read_only())
            .controller(&controller),
    ))?;
    flush(&mut ui);
    controller.set_mode("files", ToolWindowMode::Floating)?;
    controller.expand("files")?;
    flush(&mut ui);
    let saved = ui.capture_workspace()?;
    let start = child_rect(&ui, &root, "files");
    let (x, y) = (start.x() + 24.0, start.y() + 8.0);
    pointer(&mut ui, x, y, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, x + 30.0, y + 20.0, PointerKind::Move);
    allowed.set(false)?;
    flush(&mut ui);
    assert_eq!(child_rect(&ui, &root, "files"), start);
    assert_eq!(ui.capture_workspace()?, saved);
    pointer(&mut ui, x + 80.0, y + 70.0, PointerKind::Move);
    assert_eq!(child_rect(&ui, &root, "files"), start);
    assert!(ui.restore_workspace(&saved)?.is_complete());
    Ok(())
}

struct MutateToolsInUpdate(ToolWindowController);
impl Component for MutateToolsInUpdate {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        assert_eq!(self.0.expand("files"), Err(RuntimeError::WriteDuringUpdate));
        assert_eq!(self.0.set_names(true), Err(RuntimeError::WriteDuringUpdate));
        assert_eq!(
            self.0.set_mode("files", ToolWindowMode::Floating),
            Err(RuntimeError::WriteDuringUpdate)
        );
        assert_eq!(
            self.0.reorder("files", 1),
            Err(RuntimeError::WriteDuringUpdate)
        );
        assert_eq!(
            self.0.move_to("files", ToolEdge::Bottom, StripeGroup::End),
            Err(RuntimeError::WriteDuringUpdate)
        );
        assert_eq!(
            self.0.set_overlay("files", true),
            Err(RuntimeError::WriteDuringUpdate)
        );
        Ok(())
    }
}
#[test]
fn tool_controller_rejects_update_writes_before_changing_the_model() -> TestResult {
    let mut ui = ui();
    let controller = ToolWindowController::new();
    ui.mount_tree(Element::new("frame", layout().controller(&controller)))?;
    flush(&mut ui);
    let before = ui.capture_workspace()?;
    let probe = ui.mount("probe", MutateToolsInUpdate(controller.clone()))?;
    flush(&mut ui);
    assert_eq!(ui.capture_workspace()?, before);
    ui.unmount(&probe)?;
    controller.expand("files")?;
    assert!(controller.is_expanded("files")?);
    Ok(())
}

#[test]
fn restore_reports_final_changes_without_collapsing_a_retained_open_panel() -> TestResult {
    let mut ui = ui();
    let log = Log::default();
    let controller = ToolWindowController::new();
    let root = ui.mount_tree(Element::new(
        "frame",
        logged(layout(), &log).controller(&controller),
    ))?;
    flush(&mut ui);
    controller.expand("files")?;
    controller.set_mode("search", ToolWindowMode::Floating)?;
    controller.expand("search")?;
    flush(&mut ui);
    let body = find(&ui, &root, "files-body");
    ui.focus(&body)?;
    flush(&mut ui);
    log.borrow_mut().clear();
    let state = ViewState::new("tool-windows")?
        .with(
            "expanded",
            WorkspaceValue::map([("files".to_owned(), WorkspaceValue::flag(true))])?,
        )?
        .with(
            "extents",
            WorkspaceValue::map(Vec::<(String, WorkspaceValue)>::new())?,
        )?
        .with(
            "modes",
            WorkspaceValue::map([("search".to_owned(), WorkspaceValue::text("docked")?)])?,
        )?;
    let saved = Workspace::new(
        WorkspaceSettings::default(),
        [(WorkspacePath::new(["frame"])?, state)],
    )?;
    assert!(ui.restore_workspace(&saved)?.is_complete());
    flush(&mut ui);
    assert!(controller.is_expanded("files")?);
    assert!(!controller.is_expanded("search")?);
    assert_eq!(ui.focused(), Some(body));
    assert!(!log.borrow().iter().any(|change| change.key == "files"));
    Ok(())
}

#[test]
fn an_invalid_detach_candidate_falls_back_and_does_not_skip_a_valid_one() -> TestResult {
    let mut ui = ui();
    let controller = ToolWindowController::new();
    ui.mount_tree(Element::new(
        "frame",
        ToolWindowLayout::new(button("main"))
            .edge(ToolEdge::Left, EdgeMode::Stripe)
            .window(ToolWindow::new("bad", "", button("bad-body")))
            .window(window("good"))
            .controller(&controller),
    ))?;
    flush(&mut ui);
    for key in ["bad", "good"] {
        controller.set_mode(key, ToolWindowMode::Window)?;
        controller.expand(key)?;
    }
    assert_eq!(
        ui.flush(Duration::ZERO),
        Err(RuntimeError::EmptyWindowTitle)
    );
    assert_eq!(controller.mode("bad")?, ToolWindowMode::Docked);
    assert_eq!(controller.mode("good")?, ToolWindowMode::Window);
    let requests = ui.take_presentation_requests()?;
    assert_eq!(requests.len(), 1);
    assert!(matches!(
        requests[0].action(),
        PresentationAction::Detach { .. }
    ));
    Ok(())
}

#[test]
fn a_foreign_permission_build_error_preserves_its_type_and_existing_children() -> TestResult {
    let mut ui = ui();
    let controller = ToolWindowController::new();
    let root = ui.mount_tree(Element::new(
        "outer",
        Column::new([Element::new("frame", layout().controller(&controller))]),
    ))?;
    flush(&mut ui);
    let frame = find(&ui, &root, "frame");
    let before = ui.children(&frame)?;
    let foreign = Runtime::new().signal(true);
    let local = ui.signal(true);
    assert_eq!(
        ui.reconcile_children(
            &root,
            vec![Element::new(
                "frame",
                ToolWindowLayout::new(button("main"))
                    .edge(ToolEdge::Left, EdgeMode::Stripe)
                    .toggles(local.read_only())
                    .window(window("files").toggles(foreign.read_only()))
                    .controller(&controller)
            )]
        ),
        Err(RuntimeError::ForeignRuntime)
    );
    assert_eq!(ui.children(&frame)?, before);
    flush(&mut ui);
    controller.expand("files")?;
    flush(&mut ui);
    assert!(shown(&ui, "files-body"));
    Ok(())
}

#[test]
fn a_recreated_panel_receives_requests_to_return_from_its_native_window() -> TestResult {
    let mut ui = ui();
    let controller = ToolWindowController::new();
    let root = ui.mount_tree(Element::new(
        "outer",
        Column::new([Element::new("frame", layout().controller(&controller))]),
    ))?;
    flush(&mut ui);
    let old = find(&ui, &root, "files");
    ui.reconcile_children(
        &root,
        vec![Element::new(
            "frame",
            ToolWindowLayout::new(button("main"))
                .edge(ToolEdge::Left, EdgeMode::Stripe)
                .controller(&controller),
        )],
    )?;
    flush(&mut ui);
    ui.reconcile_children(
        &root,
        vec![Element::new("frame", layout().controller(&controller))],
    )?;
    flush(&mut ui);
    let panel = find(&ui, &root, "files");
    assert_ne!(panel, old);
    controller.set_mode("files", ToolWindowMode::Window)?;
    controller.expand("files")?;
    flush(&mut ui);
    let window = host_detach(&mut ui, &panel);
    controller.collapse("files")?;
    flush_all(&mut ui, std::slice::from_ref(&window));
    assert!(ui.take_presentation_requests()?.iter().any(|request| matches!(request.action(), PresentationAction::Redock(actual) if *actual == window)));
    Ok(())
}

#[test]
fn revoked_floating_permission_cancels_before_an_unsettled_pointer_release() -> TestResult {
    for resize in [false, true] {
        let mut ui = ui();
        let allowed = ui.signal(true);
        let controller = ToolWindowController::new();
        let root = ui.mount_tree(Element::new(
            "frame",
            layout()
                .floatable(allowed.read_only())
                .controller(&controller),
        ))?;
        flush(&mut ui);
        controller.set_mode("files", ToolWindowMode::Floating)?;
        controller.expand("files")?;
        flush(&mut ui);
        let saved = ui.capture_workspace()?;
        let start = child_rect(&ui, &root, "files");
        let (x, y) = if resize {
            (
                start.x() + start.width() - 2.0,
                start.y() + start.height() - 2.0,
            )
        } else {
            (start.x() + 24.0, start.y() + 8.0)
        };
        pointer(&mut ui, x, y, PointerKind::Press(PointerButton::Primary));
        allowed.set(false)?;
        for kind in [
            PointerKind::Move,
            PointerKind::Release(PointerButton::Primary),
        ] {
            ui.pointer_input(PointerEvent::new(Point::new(x + 30.0, y + 20.0)?, kind))?;
        }
        flush(&mut ui);
        assert_eq!(ui.capture_workspace()?, saved);
        assert_eq!(child_rect(&ui, &root, "files"), start);
    }
    Ok(())
}

#[test]
fn floating_restore_rejects_extent_underflow_before_changing_preferences() -> TestResult {
    let mut ui = ui();
    let controller = ToolWindowController::new();
    ui.mount_tree(Element::new("frame", layout().controller(&controller)))?;
    flush(&mut ui);
    let before = ui.capture_workspace()?;
    for field in ["float-width", "float-height"] {
        let mut state = ViewState::new("tool-windows")?
            .with(
                "expanded",
                WorkspaceValue::map([("files".to_owned(), WorkspaceValue::flag(true))])?,
            )?
            .with(
                "extents",
                WorkspaceValue::map(Vec::<(String, WorkspaceValue)>::new())?,
            )?;
        for (name, value) in [
            ("float-x", 10.0),
            ("float-y", 20.0),
            ("float-width", 160.0),
            ("float-height", 120.0),
        ] {
            state = state.with(
                name,
                WorkspaceValue::map([(
                    "files".to_owned(),
                    WorkspaceValue::number(if name == field { 1e-300 } else { value })?,
                )])?,
            )?;
        }
        let report = ui.restore_workspace(&Workspace::new(
            WorkspaceSettings::default(),
            [(WorkspacePath::new(["frame"])?, state)],
        )?)?;
        assert!(
            report
                .rejected()
                .any(|(_, rejection)| *rejection == RestoreRejection::OutOfRange(field.to_owned()))
        );
        assert_eq!(ui.capture_workspace()?, before);
    }
    Ok(())
}

/// The retitles queued for `window`, in order.
fn retitles(requests: &[PresentationRequest], window: &NativeWindowId) -> Vec<String> {
    requests
        .iter()
        .filter_map(|request| match request.action() {
            PresentationAction::Retitle { window: w, title } if w == window => Some(title.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_detached_tool_window_retitles_its_window_when_its_title_changes() -> TestResult {
    let mut ui = ui();
    let title = ui.signal("Files".to_owned());
    let controller = ToolWindowController::new();
    let root = ui.mount_tree(Element::new(
        "frame",
        ToolWindowLayout::new(button("main"))
            .edge(ToolEdge::Left, EdgeMode::Stripe)
            .window(ToolWindow::new(
                "files",
                title.read_only(),
                button("files-body"),
            ))
            .controller(&controller),
    ))?;
    flush(&mut ui);
    controller.set_mode("files", ToolWindowMode::Window)?;
    controller.expand("files")?;
    flush(&mut ui);
    let panel = child(&ui, &root, "files");
    let window = host_detach(&mut ui, &panel);
    flush_all(&mut ui, std::slice::from_ref(&window));
    assert!(
        retitles(&ui.take_presentation_requests()?, &window).is_empty(),
        "the window keeps the title it opened with"
    );

    title.set("Dateien".to_owned())?;
    flush_all(&mut ui, std::slice::from_ref(&window));
    title.set("Fichiers".to_owned())?;
    flush_all(&mut ui, std::slice::from_ref(&window));
    let requests = ui.take_presentation_requests()?;
    assert_eq!(
        retitles(&requests, &window),
        ["Fichiers"],
        "queued retitles coalesce to the latest title"
    );
    for request in requests {
        request.complete(PresentationStatus::Presented(window.clone()))?;
    }
    flush_all(&mut ui, std::slice::from_ref(&window));
    assert!(retitles(&ui.take_presentation_requests()?, &window).is_empty());
    Ok(())
}

#[test]
fn a_title_change_while_the_window_opens_retitles_it_once_open() -> TestResult {
    let mut ui = ui();
    let title = ui.signal("Files".to_owned());
    let controller = ToolWindowController::new();
    let root = ui.mount_tree(Element::new(
        "frame",
        ToolWindowLayout::new(button("main"))
            .edge(ToolEdge::Left, EdgeMode::Stripe)
            .window(ToolWindow::new(
                "files",
                title.read_only(),
                button("files-body"),
            ))
            .controller(&controller),
    ))?;
    flush(&mut ui);
    controller.set_mode("files", ToolWindowMode::Window)?;
    controller.expand("files")?;
    flush(&mut ui);
    // The detach request carries "Files"; the title changes before the host
    // opens the window.
    title.set("Dateien".to_owned())?;
    flush(&mut ui);
    let panel = child(&ui, &root, "files");
    let window = host_detach(&mut ui, &panel);
    flush_all(&mut ui, std::slice::from_ref(&window));
    assert_eq!(
        retitles(&ui.take_presentation_requests()?, &window),
        ["Dateien"]
    );
    Ok(())
}
