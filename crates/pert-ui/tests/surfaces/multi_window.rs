//! Public facade contracts for retained multi-window presentation.
use pert_ui::*;
use std::{cell::Cell, rc::Rc, time::Duration};

/// Test results carry whichever error actually occurred, not a stand-in variant.
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

struct Counter(Rc<Cell<usize>>);
impl Component for Counter {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.0.set(self.0.get() + 1);
        cx.animate()
    }
}
fn tabs(ui: &mut Runtime, value: Signal<String>) -> TestResult<(WidgetId, WidgetId)> {
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0)?)?;
    let root = ui.mount_tree(Element::new(
        "tabs",
        Tabs::new([
            Tab::new(
                "editor",
                "Editor",
                Element::new("text", TextField::new(value)),
            ),
            Tab::new(
                "other",
                "Other",
                Element::new("other-label", Label::new("Other")),
            ),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    let view = ui.children(&root)?[0].clone();
    Ok((root, view))
}
#[test]
fn detach_retains_identity_bindings_and_independent_visibility() -> TestResult {
    let mut ui = Runtime::new();
    let text = ui.signal("Before".to_owned());
    let (owner, view) = tabs(&mut ui, text.clone())?;
    let children = ui.children(&view)?;
    let primary = ui.primary_window();
    let prepared = ui.prepare_detach(&view, NativeWindowOptions::new("Editor")?)?;
    let detached = ui.commit_detach(prepared)?;
    ui.flush(Duration::ZERO)?;
    ui.paint()?;
    assert_eq!(ui.children(&view)?, children);
    assert_eq!(ui.window_of(&view)?, detached);
    ui.select_window(&primary)?;
    ui.flush(Duration::ZERO)?;
    let other = ui.children(&owner)?[1].clone();
    assert!(
        ui.inspect()
            .components
            .iter()
            .find(|c| c.key == "other")
            .unwrap()
            .visible
    );
    assert_eq!(ui.window_of(&other)?, primary);
    ui.set_window_visible(false)?;
    text.set("Shared update".to_owned())?;
    ui.select_window(&detached)?;
    ui.flush(Duration::ZERO)?;
    assert!(
        ui.inspect()
            .components
            .iter()
            .find(|c| c.key == "editor")
            .unwrap()
            .visible
    );
    assert!(ui.paint().is_ok());
    ui.select_window(&primary)?;
    ui.set_window_visible(true)?;
    ui.redock(&detached)?;
    ui.flush(Duration::ZERO)?;
    assert!(!ui.contains_window(&detached));
    assert_eq!(ui.window_of(&view)?, primary);
    assert_eq!(ui.children(&view)?, children);
    assert!(
        !ui.inspect()
            .components
            .iter()
            .find(|c| c.key == "other")
            .unwrap()
            .visible
    );
    assert_eq!(text.get()?, "Shared update");
    assert_eq!(ui.select_window(&detached), Err(RuntimeError::Unmounted));
    // Another runtime's identities are foreign, whether mounted or retired,
    // even where the serials of two primary windows coincide.
    let mut other = Runtime::new();
    assert_eq!(
        other.select_window(&primary),
        Err(RuntimeError::ForeignRuntime)
    );
    assert_eq!(
        other.select_window(&detached),
        Err(RuntimeError::ForeignRuntime)
    );
    assert_eq!(other.redock(&detached), Err(RuntimeError::ForeignRuntime));
    assert_eq!(
        ui.select_window(&other.primary_window()),
        Err(RuntimeError::ForeignRuntime)
    );
    assert_eq!(other.current_window(), other.primary_window());
    Ok(())
}
#[test]
fn abandoned_preparation_and_stale_completion_do_not_move_or_resurrect() -> TestResult {
    let mut ui = Runtime::new();
    let value = ui.signal("State".to_owned());
    let (owner, view) = tabs(&mut ui, value)?;
    let primary = ui.primary_window();
    let abandoned = ui.prepare_detach(&view, NativeWindowOptions::new("Editor")?)?;
    let unused = abandoned.destination();
    drop(abandoned);
    assert_eq!(ui.window_of(&view)?, primary);
    assert!(!ui.contains_window(&unused));
    let stale = ui.prepare_detach(&view, NativeWindowOptions::new("Editor")?)?;
    ui.unmount(&owner)?;
    assert_eq!(ui.commit_detach(stale), Err(RuntimeError::Unmounted));
    assert_eq!(ui.native_windows(), vec![primary]);
    Ok(())
}
#[test]
fn removing_owner_retires_detached_descendants_and_bindings() -> TestResult {
    let mut ui = Runtime::new();
    let text = ui.signal("State".to_owned());
    let (owner, view) = tabs(&mut ui, text.clone())?;
    let prepared = ui.prepare_detach(&view, NativeWindowOptions::new("Editor")?)?;
    let detached = ui.commit_detach(prepared)?;
    ui.flush(Duration::ZERO)?;
    ui.select_window(&ui.primary_window())?;
    ui.unmount(&owner)?;
    assert!(!ui.contains_window(&detached));
    assert!(ui.inspect().components.is_empty());
    text.set("After removal".to_owned())?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(ui.next_deadline(), None);
    assert_eq!(ui.window_of(&view), Err(RuntimeError::Unmounted));
    Ok(())
}
#[test]
fn detached_tasks_run_in_their_window_and_cancel_when_it_closes() -> TestResult {
    let mut ui = Runtime::new();
    let text = ui.signal("State".to_owned());
    let (owner, view) = tabs(&mut ui, text)?;
    let prepared = ui.prepare_detach(&view, NativeWindowOptions::new("Editor")?)?;
    let detached = ui.commit_detach(prepared)?;
    ui.flush(Duration::ZERO)?;
    let primary = ui.primary_window();
    let seen = Rc::new(std::cell::RefCell::new(None));
    let window = seen.clone();
    let task = ui.event(&view, |cx| {
        cx.defer_detached(move |ui| {
            *window.borrow_mut() = Some(ui.current_window());
            Ok(())
        })
    })??;
    ui.select_window(&primary)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(task.status(), TaskStatus::Completed);
    assert_eq!(seen.borrow().as_ref(), Some(&detached));
    assert_eq!(ui.current_window(), primary);
    // Retiring the window cancels an operation queued in it.
    let task = ui.event(&view, |cx| {
        cx.defer_detached(|_| panic!("a closed window must not run"))
    })??;
    ui.select_window(&primary)?;
    ui.unmount(&owner)?;
    assert!(!ui.contains_window(&detached));
    ui.flush(Duration::ZERO)?;
    assert_eq!(task.status(), TaskStatus::Cancelled);
    Ok(())
}
#[test]
fn window_deadlines_do_not_tick_other_roots() -> TestResult {
    let mut ui = Runtime::new();
    let count = Rc::new(Cell::new(0));
    let view = ui.mount_tree(Element::new("live", Counter(count.clone())))?;
    ui.flush(Duration::ZERO)?;
    let prepared = ui.prepare_detach(&view, NativeWindowOptions::new("Live")?)?;
    let detached = ui.commit_detach(prepared)?;
    ui.flush(Duration::ZERO)?;
    let before = count.get();
    let deadline = ui.next_deadline().unwrap();
    ui.select_window(&ui.primary_window())?;
    ui.flush(deadline)?;
    assert_eq!(count.get(), before);
    ui.select_window(&detached)?;
    ui.flush(deadline)?;
    assert_eq!(count.get(), before + 1);
    ui.set_window_visible(false)?;
    ui.flush(deadline)?;
    assert_eq!(ui.next_deadline(), None);
    Ok(())
}
#[test]
fn abandoned_native_requests_have_reactive_terminal_cancellation() -> TestResult {
    let mut ui = Runtime::new();
    let view = ui.mount_tree(Element::new("view", Label::new("Hello")))?;
    let receipt = ui.request_detach(&view, NativeWindowOptions::new("Detached")?)?;
    assert_eq!(receipt.status(), PresentationStatus::Pending);
    drop(ui.take_presentation_requests()?);
    assert_eq!(
        receipt.status(),
        PresentationStatus::Failed(WindowError::Cancelled)
    );
    assert_eq!(ui.window_of(&view)?, ui.primary_window());
    Ok(())
}

#[test]
fn target_identity_selects_correct_focus_and_inspection_window() -> TestResult {
    let mut ui = Runtime::new();
    let text = ui.signal("State".to_owned());
    let (_, view) = tabs(&mut ui, text)?;
    let prepared = ui.prepare_detach(&view, NativeWindowOptions::new("Editor")?)?;
    let detached = ui.commit_detach(prepared)?;
    ui.flush_windows(Duration::ZERO)?;
    let viewport = ui.children(&view)?[0].clone();
    let field = ui.children(&viewport)?[0].clone();
    ui.select_window(&ui.primary_window())?;
    ui.focus(&field)?;
    assert_eq!(ui.current_window(), detached);
    ui.select_window(&ui.primary_window())?;
    assert!(ui.focused().is_none());
    let snapshot = ui.inspect();
    let editor = snapshot
        .components
        .iter()
        .find(|c| c.key == "text")
        .unwrap();
    assert!(editor.focused);
    let root = snapshot
        .components
        .iter()
        .find(|c| c.key == "editor")
        .unwrap();
    assert!(root.parent.is_some());
    assert!(root.presentation_parent.is_none());
    assert_eq!(editor.window, root.window);
    ui.set_visible(&field, false)?;
    ui.select_window(&detached)?;
    assert!(ui.focused().is_none());
    Ok(())
}

#[test]
fn tab_drag_release_requests_one_retained_view_and_escape_cancels() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 500.0, 1.0).unwrap())?;
    let tabs = ui.mount_tree(Element::new(
        "tabs",
        Tabs::new([
            Tab::new(
                "first",
                "First",
                Element::new("a", Label::new("First content")),
            ),
            Tab::new(
                "second",
                "Second",
                Element::new("b", Label::new("Second content")),
            ),
        ])
        .detachable(true),
    ))?;
    ui.flush(Duration::ZERO)?;
    let view = ui.children(&tabs)?[0].clone();
    for cancelled in [true, false] {
        ui.pointer_input(PointerEvent::new(
            Point::new(20.0, 15.0).unwrap(),
            PointerKind::Press(PointerButton::Primary),
        ))?;
        ui.pointer_input(PointerEvent::new(
            Point::new(40.0, 110.0).unwrap(),
            PointerKind::Move,
        ))?;
        if cancelled {
            ui.key_input(KeyCode::Escape, KeyAction::Press)?;
            ui.key_input(KeyCode::Escape, KeyAction::Release)?;
        }
        ui.pointer_input(PointerEvent::new(
            Point::new(40.0, 110.0).unwrap(),
            PointerKind::Release(PointerButton::Primary),
        ))?;
        let mut requests = ui.take_presentation_requests()?;
        assert_eq!(requests.len(), usize::from(!cancelled));
        if let Some(request) = requests.pop() {
            match request.action() {
                PresentationAction::Detach {
                    view: requested,
                    options,
                } => {
                    assert_eq!(requested, &view);
                    ui.flush(Duration::ZERO)?;
                    let prepared = ui.prepare_detach(requested, options.clone())?;
                    let detached = ui.commit_detach(prepared)?;
                    request.complete(PresentationStatus::Presented(detached.clone()))?;
                    assert_eq!(ui.window_of(&view)?, detached);
                    ui.redock(&detached)?;
                }
                _ => panic!("expected detach"),
            }
        }
        ui.flush_windows(Duration::ZERO)?;
    }
    assert_eq!(ui.window_of(&view)?, ui.primary_window());
    assert_eq!(ui.children(&tabs)?[0], view);
    Ok(())
}
#[test]
fn tab_drag_out_is_a_public_drag_session_the_strip_forbids_over_itself() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 500.0, 1.0).unwrap())?;
    let tab = |key: &str| {
        Tab::new(
            key,
            format!("Tab {key}"),
            Element::new("body", Label::new("Content")),
        )
        .into_element()
    };
    let tabs = ui.mount_tree(
        Element::new("tabs", Tabs::new([]).detachable(true))
            .children([tab("first"), tab("second")]),
    )?;
    ui.flush(Duration::ZERO)?;
    let first = ui.children(&tabs)?[0].clone();
    let press = |ui: &mut Runtime| {
        ui.pointer_input(PointerEvent::new(
            Point::new(20.0, 15.0).unwrap(),
            PointerKind::Press(PointerButton::Primary),
        ))
    };
    let moved = |ui: &mut Runtime, x: f32, y: f32| {
        ui.pointer_input(PointerEvent::new(
            Point::new(x, y).unwrap(),
            PointerKind::Move,
        ))?;
        ui.flush(Duration::ZERO)
    };
    let release = |ui: &mut Runtime, event: PointerEvent| {
        ui.pointer_input(event)?;
        ui.flush(Duration::ZERO)
    };

    // Leaving the strip starts an ordinary session with the strip as its source.
    press(&mut ui)?;
    moved(&mut ui, 40.0, 30.0)?;
    assert!(!ui.drag_active(), "within the strip a press stays a click");
    moved(&mut ui, 60.0, 180.0)?;
    let strip = ui
        .inspect()
        .components
        .iter()
        .find(|item| item.key == "tabs")
        .unwrap()
        .id;
    let session = ui.inspect().drag.expect("a drag session");
    assert_eq!(session.source, strip);
    assert!(session.preview.is_some(), "the title follows the pointer");
    assert_eq!(session.target, None);
    assert_eq!(ui.cursor_icon(), CursorIcon::Grabbing);
    assert_eq!(
        detail(&ui, "detaching"),
        Some(InspectValue::Text("first".into()))
    );

    // Back on the strip the drop is forbidden and a release detaches nothing.
    moved(&mut ui, 200.0, 20.0)?;
    assert_eq!(ui.inspect().drag.unwrap().target, Some(strip));
    assert_eq!(ui.cursor_icon(), CursorIcon::NoDrop);
    assert_eq!(detail(&ui, "detaching"), None);
    release(
        &mut ui,
        PointerEvent::new(
            Point::new(200.0, 20.0).unwrap(),
            PointerKind::Release(PointerButton::Primary),
        ),
    )?;
    assert!(!ui.drag_active());
    assert!(ui.take_presentation_requests()?.is_empty());

    // A release outside the window is unclaimed, and detaches the tab.
    press(&mut ui)?;
    moved(&mut ui, 60.0, 180.0)?;
    release(
        &mut ui,
        PointerEvent::new(
            Point::new(60.0, -40.0).unwrap(),
            PointerKind::Release(PointerButton::Primary),
        )
        .outside(),
    )?;
    let requests = ui.take_presentation_requests()?;
    assert_eq!(requests.len(), 1);
    assert!(matches!(
        requests[0].action(),
        PresentationAction::Detach { view, .. } if view == &first
    ));
    assert_eq!(detail(&ui, "detaching"), None);
    drop(requests);
    ui.flush(Duration::ZERO)?;

    // A tab removed and recreated under the same key during its session is a
    // different view, and is not detached afterwards.
    let second = ui.children(&tabs)?[1].clone();
    ui.pointer_input(PointerEvent::new(
        Point::new(20.0, 15.0).unwrap(),
        PointerKind::Move,
    ))?;
    press(&mut ui)?;
    moved(&mut ui, 60.0, 180.0)?;
    assert_eq!(
        detail(&ui, "detaching"),
        Some(InspectValue::Text("first".into()))
    );
    ui.reconcile_children(&tabs, vec![tab("second")])?;
    ui.flush(Duration::ZERO)?;
    assert!(ui.drag_active(), "the strip, not the tab, is the source");
    ui.reconcile_children(&tabs, vec![tab("first"), tab("second")])?;
    ui.flush(Duration::ZERO)?;
    let recreated = ui.children(&tabs)?[0].clone();
    assert_ne!(recreated, first);
    release(
        &mut ui,
        PointerEvent::new(
            Point::new(60.0, 180.0).unwrap(),
            PointerKind::Release(PointerButton::Primary),
        ),
    )?;
    assert!(ui.take_presentation_requests()?.is_empty());
    assert_eq!(ui.children(&tabs)?, [recreated, second]);
    Ok(())
}

#[test]
fn tab_drag_out_tolerance_and_titles_follow_what_the_strip_can_answer() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 500.0, 1.0).unwrap())?;
    // Something above the strip, so the pointer can rest just above it on
    // content the strip does not contain.
    let root = ui.mount_tree(Element::new(
        "page",
        Column::new([
            Element::new("header", Label::new("Header")),
            Element::new(
                "tabs",
                Tabs::new([
                    Tab::new("first", "Settings", Element::new("a", Label::new("A")))
                        .rich_title(RichText::new().icon(Icon::Settings)),
                    Tab::new("second", "Second", Element::new("b", Label::new("B"))),
                ])
                .detachable(true),
            ),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    let tabs = ui.children(&root)?[1].clone();
    let strip = ui.bounds(&tabs)?.unwrap();
    let body = ui.bounds(&ui.children(&tabs)?[0])?.unwrap();
    let (x, top, bottom) = (strip.x() + 10.0, strip.y(), body.y());
    assert!(top >= 8.0, "room above the strip");
    let at = |y: f32| Point::new(x, y).unwrap();
    let step = |ui: &mut Runtime, kind: PointerKind, y: f32| {
        ui.pointer_input(PointerEvent::new(at(y), kind))?;
        ui.flush(Duration::ZERO)
    };

    // Just above the strip is not the strip's to answer, so the cursor, the
    // emptied slot and the release all say the tab would detach there.
    step(
        &mut ui,
        PointerKind::Press(PointerButton::Primary),
        top + 10.0,
    )?;
    step(&mut ui, PointerKind::Move, top - 4.0)?;
    assert!(ui.drag_active());
    assert_eq!(ui.cursor_icon(), CursorIcon::Grabbing);
    assert_eq!(
        detail(&ui, "detaching"),
        Some(InspectValue::Text("first".into()))
    );
    // Just below the strip is, and forbids the drop.
    step(&mut ui, PointerKind::Move, bottom + 4.0)?;
    assert_eq!(ui.cursor_icon(), CursorIcon::NoDrop);
    assert_eq!(detail(&ui, "detaching"), None);
    step(
        &mut ui,
        PointerKind::Release(PointerButton::Primary),
        bottom + 4.0,
    )?;
    assert!(ui.take_presentation_requests()?.is_empty());

    // An icon-only title still has a preview to show and a name for its window.
    step(
        &mut ui,
        PointerKind::Press(PointerButton::Primary),
        top + 10.0,
    )?;
    step(&mut ui, PointerKind::Move, 300.0)?;
    let scene = ui.paint().unwrap();
    assert!(
        scene.commands().iter().any(|command| matches!(
            command,
            DrawCommand::DrawMask { rect, .. } if rect.y() >= 300.0
        )),
        "the preview draws the icon under the pointer"
    );
    step(&mut ui, PointerKind::Move, top - 4.0)?;
    step(
        &mut ui,
        PointerKind::Release(PointerButton::Primary),
        top - 4.0,
    )?;
    let requests = ui.take_presentation_requests()?;
    assert_eq!(requests.len(), 1);
    match requests[0].action() {
        PresentationAction::Detach { options, .. } => assert_eq!(options.title(), "Settings"),
        _ => panic!("expected detach"),
    }
    Ok(())
}
#[test]
fn failed_close_redock_keeps_detached_window_and_remove_policy_cleans_owner() -> TestResult {
    let mut ui = Runtime::new();
    let text = ui.signal("Retained".to_owned());
    let (owner, view) = tabs(&mut ui, text)?;
    let prepared = ui.prepare_detach(&view, NativeWindowOptions::new("Editor")?)?;
    let detached = ui.commit_detach(prepared)?;
    ui.set_visible(&owner, false)?;
    ui.select_window(&detached)?;
    assert_eq!(ui.close_detached_window(), Err(RuntimeError::Hidden));
    assert!(ui.contains_window(&detached));
    ui.set_visible(&owner, true)?;
    ui.redock(&detached)?;
    ui.flush(Duration::ZERO)?;
    let prepared = ui.prepare_detach(
        &view,
        NativeWindowOptions::new("Editor")?.close_policy(DetachedClosePolicy::RemoveView),
    )?;
    let detached = ui.commit_detach(prepared)?;
    assert_eq!(ui.close_detached_window()?, PresentationStatus::Closed);
    assert!(!ui.contains_window(&detached));
    assert_eq!(ui.window_of(&view), Err(RuntimeError::Unmounted));
    Ok(())
}

/// Coverage drawn in one role inside the 36-pixel tab strip. Tab and surface
/// foregrounds share a color, so the strip is separated by geometry.
fn strip_masks(scene: &DisplayList, role: ColorRole) -> Vec<Rect> {
    let wanted = Palette::dark().color(role);
    scene
        .commands()
        .iter()
        .filter_map(|command| match command {
            DrawCommand::DrawMask { rect, color, .. } if *color == wanted => Some(*rect),
            _ => None,
        })
        .filter(|rect| {
            let [_, top, _, height] = rect.values();
            top + height <= 36.0
        })
        .collect()
}

#[test]
fn detachable_titles_show_a_drag_marker_and_detached_titles_read_as_empty_slots() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 400.0, 1.0).unwrap())?;
    let strip = |detachable| {
        Element::new(
            "tabs",
            Tabs::new([
                Tab::new("first", "First", Element::new("a", Label::new("First"))),
                Tab::new("second", "Second", Element::new("b", Label::new("Second"))),
            ])
            .detachable(detachable),
        )
    };
    // The reserved marker slot widens every title, so revealing it never reflows.
    let plain = ui.mount_tree(strip(false))?;
    ui.flush(Duration::ZERO)?;
    let narrow = strip_masks(&ui.paint().unwrap(), ColorRole::OnTab);
    ui.unmount(&plain)?;
    let tabs = ui.mount_tree(strip(true))?;
    ui.flush(Duration::ZERO)?;

    let resting = ui.paint().unwrap();
    let titles = strip_masks(&resting, ColorRole::OnTab);
    assert_eq!((titles.len(), narrow.len()), (2, 2), "one mask per title");
    assert_eq!(titles[0].x(), narrow[0].x());
    assert!(
        titles[1].x() > narrow[1].x(),
        "the marker slot widens each title"
    );
    assert!(strip_masks(&resting, ColorRole::Muted).is_empty());

    // Hovering a docked title offers the drag affordance inside the reserved slot.
    let hover = Point::new(20.0, 15.0).unwrap();
    ui.pointer_input(PointerEvent::new(hover, PointerKind::Move))?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(ui.cursor_icon(), CursorIcon::Move);
    let hovered = ui.paint().unwrap();
    assert_eq!(strip_masks(&hovered, ColorRole::OnTab), titles);
    let marker = strip_masks(&hovered, ColorRole::Muted);
    assert_eq!(marker.len(), 1, "one drag marker, in the hovered title");
    // A missing icon glyph would still reserve the slot but cover no pixels.
    let covered = hovered
        .commands()
        .iter()
        .filter_map(|command| match command {
            DrawCommand::DrawMask { mask, rect, .. } if *rect == marker[0] => hovered.mask(*mask),
            _ => None,
        })
        .flat_map(|mask| mask.pixels().iter().map(|value| u64::from(*value)))
        .sum::<u64>();
    assert!(covered > 0, "the marker rasterizes");
    let [x, _, _, _] = marker[0].values();
    let [title_x, _, title_width, _] = titles[0].values();
    assert!(x >= title_x + title_width && x < titles[1].x());

    // Dragging inside the strip is still an ordinary selection gesture.
    ui.pointer_input(PointerEvent::new(
        hover,
        PointerKind::Press(PointerButton::Primary),
    ))?;
    ui.pointer_input(PointerEvent::new(
        Point::new(40.0, 20.0).unwrap(),
        PointerKind::Move,
    ))?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(detail(&ui, "detaching"), None);
    let inside = strip_masks(&ui.paint().unwrap(), ColorRole::Primary).len();

    // Leaving the strip empties the source slot before the release commits it.
    ui.pointer_input(PointerEvent::new(
        Point::new(60.0, 180.0).unwrap(),
        PointerKind::Move,
    ))?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(
        detail(&ui, "detaching"),
        Some(InspectValue::Text("first".into()))
    );
    let leaving = ui.paint().unwrap();
    assert_eq!(strip_masks(&leaving, ColorRole::OnTab).len(), 1);
    assert_eq!(strip_masks(&leaving, ColorRole::Muted), vec![titles[0]]);
    assert_eq!(
        strip_masks(&leaving, ColorRole::Primary).len(),
        inside + 1,
        "only the marker is added to the focused card"
    );

    // Escape restores the ordinary presentation without detaching anything.
    ui.key_input(KeyCode::Escape, KeyAction::Press)?;
    ui.key_input(KeyCode::Escape, KeyAction::Release)?;
    ui.pointer_input(PointerEvent::new(
        Point::new(60.0, 180.0).unwrap(),
        PointerKind::Release(PointerButton::Primary),
    ))?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(detail(&ui, "detaching"), None);
    assert_eq!(
        detail(&ui, "detached_tabs"),
        Some(InspectValue::Number(0.0))
    );
    assert!(ui.take_presentation_requests()?.is_empty());

    // A detached title keeps its place as a muted, emptied slot.
    let view = ui.children(&tabs)?[0].clone();
    let prepared = ui.prepare_detach(&view, NativeWindowOptions::new("First")?)?;
    let window = ui.commit_detach(prepared)?;
    let primary = ui.primary_window();
    ui.select_window(&primary)?;
    ui.flush_windows(Duration::ZERO)?;
    let detached = ui.paint().unwrap();
    assert_eq!(strip_masks(&detached, ColorRole::OnTab).len(), 1);
    assert_eq!(strip_masks(&detached, ColorRole::Muted), vec![titles[0]]);
    assert_eq!(
        detail(&ui, "detached_tabs"),
        Some(InspectValue::Number(1.0))
    );
    // The whole title stays the hit area, and a detached one activates instead.
    ui.pointer_input(PointerEvent::new(hover, PointerKind::Move))?;
    assert_eq!(ui.cursor_icon(), CursorIcon::Default);

    ui.redock(&window)?;
    ui.flush_windows(Duration::ZERO)?;
    assert_eq!(
        detail(&ui, "detached_tabs"),
        Some(InspectValue::Number(0.0))
    );
    assert_eq!(
        strip_masks(&ui.paint().unwrap(), ColorRole::OnTab).len(),
        2,
        "a re-docked title returns to ordinary tab text"
    );
    Ok(())
}

fn detail(ui: &Runtime, property: &str) -> Option<InspectValue> {
    ui.inspect()
        .components
        .iter()
        .find(|item| item.key == "tabs")?
        .details
        .properties
        .get(property)
        .cloned()
}

#[test]
fn a_forbidden_view_offers_no_gesture_and_rejects_every_detach_request() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 400.0, 1.0).unwrap())?;
    // An ordinary view can refuse its own window, whoever asks for it.
    let guarded =
        ui.mount_tree(Element::new("guarded", Label::new("Stays docked")).with_detachable(false))?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(
        ui.prepare_detach(&guarded, NativeWindowOptions::new("Guarded")?)
            .err(),
        Some(RuntimeError::NotDetachable)
    );
    assert_eq!(
        ui.request_detach(&guarded, NativeWindowOptions::new("Guarded")?)
            .err(),
        Some(RuntimeError::NotDetachable)
    );
    assert!(ui.take_presentation_requests()?.is_empty());
    ui.unmount(&guarded)?;

    // One excluded tab does not disable the rest of a detachable strip.
    let tab = |key: &str, movable: bool| {
        Tab::new(
            key,
            format!("Tab {key}"),
            Element::new("body", Label::new("Content")),
        )
        .detachable(movable)
    };
    let tabs = ui.mount_tree(
        Element::new("tabs", Tabs::new([]).detachable(true)).children([
            tab("free", true).into_element(),
            tab("fixed", false).into_element(),
        ]),
    )?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(detail(&ui, "movable_tabs"), Some(InspectValue::Number(1.0)));
    let titles = strip_masks(&ui.paint().unwrap(), ColorRole::OnTab);
    assert_eq!(titles.len(), 2);

    // Hovering the excluded title offers neither the cursor nor the marker.
    let over = |rect: Rect| {
        let [x, y, _, height] = rect.values();
        Point::new(x + 2.0, y + height / 2.0).unwrap()
    };
    ui.pointer_input(PointerEvent::new(over(titles[1]), PointerKind::Move))?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(ui.cursor_icon(), CursorIcon::Default);
    assert!(strip_masks(&ui.paint().unwrap(), ColorRole::Muted).is_empty());

    // Dragging it out of the strip stays an ordinary, cancelled selection gesture.
    ui.pointer_input(PointerEvent::new(
        over(titles[1]),
        PointerKind::Press(PointerButton::Primary),
    ))?;
    ui.pointer_input(PointerEvent::new(
        Point::new(60.0, 180.0).unwrap(),
        PointerKind::Move,
    ))?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(detail(&ui, "detaching"), None);
    ui.pointer_input(PointerEvent::new(
        Point::new(60.0, 180.0).unwrap(),
        PointerKind::Release(PointerButton::Primary),
    ))?;
    assert!(ui.take_presentation_requests()?.is_empty());
    let fixed = ui.children(&tabs)?[1].clone();
    assert_eq!(
        ui.prepare_detach(&fixed, NativeWindowOptions::new("Tab fixed")?)
            .err(),
        Some(RuntimeError::NotDetachable)
    );

    // The permitted sibling still detaches through the same strip.
    ui.pointer_input(PointerEvent::new(over(titles[0]), PointerKind::Move))?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(ui.cursor_icon(), CursorIcon::Move);

    // Reconciliation flips the permission without remounting the retained view.
    ui.reconcile_children(
        &tabs,
        vec![
            tab("free", false).into_element(),
            tab("fixed", true).into_element(),
        ],
    )?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(ui.children(&tabs)?[1], fixed, "state is retained");
    assert_eq!(detail(&ui, "movable_tabs"), Some(InspectValue::Number(1.0)));
    assert_eq!(ui.cursor_icon(), CursorIcon::Default);
    // Only the description's refusal was lifted; an unselected tab is still hidden.
    assert_eq!(
        ui.prepare_detach(&fixed, NativeWindowOptions::new("Tab fixed")?)
            .err(),
        Some(RuntimeError::Hidden)
    );
    Ok(())
}

#[test]
fn deferred_redock_during_flush_skips_the_retired_root() -> TestResult {
    let mut ui = Runtime::new();
    let value = ui.signal(String::new());
    let (_, view) = tabs(&mut ui, value)?;
    let prepared = ui.prepare_detach(&view, NativeWindowOptions::new("Editor")?)?;
    let detached = ui.commit_detach(prepared)?;
    ui.select_window(&ui.primary_window())?;
    let receipt = ui.defer(move |ui| ui.redock(&detached))?;
    ui.flush_windows(Duration::ZERO)?;
    assert_eq!(receipt.status(), TaskStatus::Completed);
    assert_eq!(ui.window_of(&view)?, ui.primary_window());
    Ok(())
}

/// Two views side by side in the main window: an editor with a focusable field
/// and a sibling that owns sustained keys.
fn siblings(ui: &mut Runtime) -> TestResult<(WidgetId, WidgetId, WidgetId)> {
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0)?)?;
    let text = ui.signal("Draft".to_owned());
    let root = ui.mount_tree(Element::new(
        "row",
        Row::new([
            Element::new(
                "editor",
                View::new(Element::new("field", TextField::new(text))),
            ),
            Element::new(
                "sibling",
                View::new(Element::new("label", Label::new("Keys"))),
            ),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    let children = ui.children(&root)?;
    let field = ui.children(&children[0])?[0].clone();
    Ok((children[0].clone(), children[1].clone(), field))
}

fn command_key(ui: &mut Runtime, action: KeyAction) -> Result<KeyDispatch, RuntimeError> {
    ui.command_key_input(CommandKeyInput::new(
        KeyCode::KeyA,
        action,
        CommandKey::Character('a'),
    ))
}

#[test]
fn moving_a_view_leaves_input_owned_elsewhere_in_its_window_alone() -> TestResult {
    let mut ui = Runtime::new();
    let (editor, sibling, field) = siblings(&mut ui)?;
    let events = Rc::new(std::cell::RefCell::new(Vec::new()));
    let cancellations = Rc::new(std::cell::RefCell::new(Vec::new()));
    let (written, cancelled) = (events.clone(), cancellations.clone());
    let _held = ui.register_held_input(
        &sibling,
        vec![KeyCode::KeyA],
        Modifiers::default(),
        move |key, event, _| {
            written.borrow_mut().push((key, event));
            Ok(InputOutcome::Handled)
        },
        move |key, reason| cancelled.borrow_mut().push((key, reason)),
    )?;
    command_key(&mut ui, KeyAction::Press)?;
    ui.focus(&field)?;

    // Detaching the focused editor ends its focus, not the sibling's held key.
    let primary = ui.primary_window();
    let prepared = ui.prepare_detach(&editor, NativeWindowOptions::new("Editor")?)?;
    let detached = ui.commit_detach(prepared)?;
    ui.select_window(&primary)?;
    assert!(ui.focused().is_none());
    assert!(ui.keyboard().is_pressed(KeyCode::KeyA));
    assert!(cancellations.borrow().is_empty());
    command_key(&mut ui, KeyAction::Release)?;
    assert_eq!(
        *events.borrow(),
        vec![
            (KeyCode::KeyA, KeyTransition::Pressed),
            (KeyCode::KeyA, KeyTransition::Released)
        ]
    );

    // Re-docking likewise leaves a held key owned in the main window alone and
    // returns focus to the editor that had it.
    command_key(&mut ui, KeyAction::Press)?;
    ui.redock(&detached)?;
    ui.flush(Duration::ZERO)?;
    assert!(cancellations.borrow().is_empty());
    assert!(ui.keyboard().is_pressed(KeyCode::KeyA));
    assert_eq!(ui.focused(), Some(field));
    Ok(())
}

/// Focusable content whose focus loss always fails.
struct FailingFocusLoss;
impl Component for FailingFocusLoss {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn focusable(&self) -> bool {
        true
    }
    fn focus_lost(&mut self, _: &mut EventContext) -> Result<(), RuntimeError> {
        Err(RuntimeError::StalePresentation)
    }
}

#[test]
fn failed_movement_keeps_the_previously_selected_window() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0)?)?;
    let owner = ui.mount_tree(Element::new(
        "row",
        Row::new([
            Element::new(
                "editor",
                View::new(Element::new("label", Label::new("Editor"))),
            ),
            Element::new(
                "failing",
                View::new(Element::new("content", FailingFocusLoss)),
            ),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    let [view, failing] = <[WidgetId; 2]>::try_from(ui.children(&owner)?)
        .map_err(|children| format!("expected two views, found {children:?}"))?;
    let primary = ui.primary_window();
    let content = ui.children(&failing)?[0].clone();
    let prepared = ui.prepare_detach(&view, NativeWindowOptions::new("Editor")?)?;
    let detached = ui.commit_detach(prepared)?;

    // A re-dock refused because the owner is hidden leaves the main window
    // selected, as it was before the call.
    ui.select_window(&primary)?;
    ui.set_visible(&owner, false)?;
    assert_eq!(ui.redock(&detached), Err(RuntimeError::Hidden));
    assert_eq!(ui.current_window(), primary);
    assert!(ui.contains_window(&detached));
    ui.set_visible(&owner, true)?;

    // A detach whose cleanup fails after it started leaves the detached window
    // selected, as it was before the call, and the view in its source window.
    ui.focus(&content)?;
    let prepared = ui.prepare_detach(&failing, NativeWindowOptions::new("Failing")?)?;
    ui.select_window(&detached)?;
    assert_eq!(
        ui.commit_detach(prepared),
        Err(RuntimeError::StalePresentation)
    );
    assert_eq!(ui.current_window(), detached);
    assert_eq!(ui.window_of(&failing)?, primary);
    Ok(())
}

#[test]
fn movement_rejects_detached_roots_and_the_main_window() -> TestResult {
    let mut ui = Runtime::new();
    let text = ui.signal("State".to_owned());
    let (_, view) = tabs(&mut ui, text)?;
    let primary = ui.primary_window();
    assert_eq!(ui.redock(&primary), Err(RuntimeError::NotDetached));
    ui.select_window(&primary)?;
    assert_eq!(ui.close_detached_window(), Err(RuntimeError::NotDetached));

    let prepared = ui.prepare_detach(
        &view,
        NativeWindowOptions::new("Editor")?.close_policy(DetachedClosePolicy::KeepOpen),
    )?;
    let detached = ui.commit_detach(prepared)?;
    assert_eq!(
        ui.prepare_detach(&view, NativeWindowOptions::new("Again")?)
            .err(),
        Some(RuntimeError::AlreadyDetached)
    );
    // A window that keeps itself open rejects an accepted close and stays.
    assert_eq!(
        ui.close_detached_window()?,
        PresentationStatus::CloseRejected
    );
    assert!(ui.contains_window(&detached));
    assert_eq!(ui.window_of(&view)?, detached);
    Ok(())
}

#[test]
fn the_window_request_queue_refuses_beyond_its_capacity() -> TestResult {
    let mut ui = Runtime::new();
    let view = ui.mount_tree(Element::new("view", Label::new("Hello")))?;
    let primary = ui.primary_window();
    let mut receipts = Vec::new();
    for _ in 0..Runtime::MAX_WINDOW_REQUESTS {
        receipts.push(ui.request_close(&primary)?);
    }
    assert_eq!(
        ui.request_detach(&view, NativeWindowOptions::new("Full")?)
            .err(),
        Some(RuntimeError::WindowRequestFull)
    );
    assert_eq!(
        ui.request_close(&primary).err(),
        Some(RuntimeError::WindowRequestFull)
    );
    // Every admitted request is still delivered, and taking them frees the queue.
    let requests = ui.take_presentation_requests()?;
    assert_eq!(requests.len(), Runtime::MAX_WINDOW_REQUESTS);
    assert!(matches!(
        requests[0].action(),
        PresentationAction::Close(window) if *window == primary
    ));
    drop(requests);
    assert!(
        receipts
            .iter()
            .all(|receipt| receipt.status() == PresentationStatus::Failed(WindowError::Cancelled))
    );
    let next = ui.request_detach(&view, NativeWindowOptions::new("Room")?)?;
    assert_eq!(next.status(), PresentationStatus::Pending);
    Ok(())
}

#[test]
fn presentation_receipts_are_equal_only_for_the_same_request() -> TestResult {
    let ui = Runtime::new();
    let primary = ui.primary_window();
    let first = ui.request_close(&primary)?;
    let second = ui.request_close(&primary)?;
    assert_eq!(first, first.clone());
    assert_ne!(first, second);
    // A signal holding the latest receipt notifies for a new request even when
    // both end in the same status.
    let latest = ui.signal(first.clone());
    assert!(!latest.set_changed(first.clone())?);
    assert!(latest.set_changed(second)?);
    assert!(format!("{first:?}").contains("Pending"));
    drop(ui.take_presentation_requests()?);
    Ok(())
}

#[test]
fn detach_rejects_current_zoom_overflow_before_moving_the_view() -> TestResult {
    let mut ui = Runtime::new();
    let text = ui.signal(String::new());
    let (_, view) = tabs(&mut ui, text)?;
    let primary = ui.primary_window();
    let options = NativeWindowOptions::new("Extreme")?.size(Size::new(f32::MAX, 300.0)?)?;
    let prepared = ui.prepare_detach(&view, options)?;
    ui.ui_settings().set_zoom(UiZoom::new(0.75)?)?;
    let before = ui.capture_workspace()?;
    assert!(matches!(
        ui.commit_detach(prepared),
        Err(RuntimeError::Geometry(_))
    ));
    assert_eq!(ui.window_of(&view)?, primary);
    assert_eq!(ui.current_window(), primary);
    assert_eq!(ui.capture_workspace()?, before);
    ui.flush(Duration::ZERO)?;
    ui.paint()?;
    let retry = ui.prepare_detach(&view, NativeWindowOptions::new("Valid")?)?;
    let detached = ui.commit_detach(retry)?;
    assert_eq!(ui.window_of(&view)?, detached);
    Ok(())
}

struct ZoomOnFocusLoss {
    settings: UiSettings,
    zoom: UiZoom,
}
impl Component for ZoomOnFocusLoss {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn focusable(&self) -> bool {
        true
    }
    fn focus_lost(&mut self, _: &mut EventContext) -> Result<(), RuntimeError> {
        self.settings.set_zoom(self.zoom)?;
        Ok(())
    }
}
#[test]
fn detach_uses_the_zoom_after_focus_cleanup_callbacks() -> TestResult {
    for (zoom, width, succeeds) in [(2.0, 800.0, true), (0.75, f32::MAX, false)] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(800.0, 600.0, 1.0)?)?;
        let view = ui.mount_tree(Element::new(
            "view",
            ZoomOnFocusLoss {
                settings: ui.ui_settings(),
                zoom: UiZoom::new(zoom)?,
            },
        ))?;
        ui.flush(Duration::ZERO)?;
        ui.focus(&view)?;
        let primary = ui.primary_window();
        let prepared = ui.prepare_detach(
            &view,
            NativeWindowOptions::new("View")?.size(Size::new(width, 300.0)?)?,
        )?;
        let result = ui.commit_detach(prepared);
        if succeeds {
            let detached = result?;
            assert_eq!(ui.window_of(&view)?, detached);
            assert_eq!(ui.viewport().width(), width / zoom as f32);
            assert_eq!(ui.viewport().scale(), zoom);
        } else {
            assert!(matches!(result, Err(RuntimeError::Geometry(_))));
            assert_eq!(ui.window_of(&view)?, primary);
            assert_eq!(ui.current_window(), primary);
        }
    }
    Ok(())
}
#[test]
fn refused_focus_in_another_window_keeps_the_selected_window() -> TestResult {
    let mut ui = Runtime::new();
    let text = ui.signal("Text".to_owned());
    let (_owner, view) = tabs(&mut ui, text)?;
    let primary = ui.primary_window();
    let prepared = ui.prepare_detach(&view, NativeWindowOptions::new("Editor")?)?;
    let detached = ui.commit_detach(prepared)?;
    ui.flush(Duration::ZERO)?;
    let field = ui
        .find_descendant(&view, "text")?
        .expect("the detached view holds the field");
    ui.set_visible(&field, false)?;
    ui.select_window(&primary)?;
    ui.flush(Duration::ZERO)?;
    assert!(matches!(ui.focus(&field), Err(RuntimeError::Hidden)));
    assert_eq!(ui.current_window(), primary, "the refusal changes nothing");
    ui.set_visible(&field, true)?;
    ui.flush(Duration::ZERO)?;
    ui.focus(&field)?;
    assert_eq!(
        ui.current_window(),
        detached,
        "a transfer selects its window"
    );
    assert_eq!(ui.focused(), Some(field));
    Ok(())
}
