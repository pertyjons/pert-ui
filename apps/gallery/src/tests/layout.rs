//! The Layout category's pages.
use super::support::*;
use pert_ui::*;
use std::time::Duration;

#[test]
fn live_view_suspends_and_counter_survives_hiding_and_tab_switches() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "state");
    let counter = find(&ui, &root, "retained-counter");
    let increment = find(&ui, &counter, "increment");
    click(&mut ui, &increment);
    assert!(text(&ui, "count").contains(&"Count: 1".into()));
    let run = find(&ui, &find(&ui, &root, "run-pulse"), "toggle");
    click(&mut ui, &run);
    assert!(ui.next_deadline().is_some());
    let before_tick = ui.paint().unwrap();
    ui.flush(Duration::from_secs(1)).unwrap();
    assert_ne!(before_tick.commands(), ui.paint().unwrap().commands());
    let show = find(&ui, &find(&ui, &root, "show-view"), "toggle");
    click(&mut ui, &show);
    assert!(ui.next_deadline().is_none());
    let pulse_updates = |ui: &Runtime| {
        ui.inspect()
            .components
            .iter()
            .find(|item| item.key == "pulse")
            .unwrap()
            .details
            .properties["updates"]
            .clone()
    };
    let hidden = pulse_updates(&ui);
    ui.flush(Duration::from_secs(10)).unwrap();
    assert_eq!(pulse_updates(&ui), hidden);
    click(&mut ui, &show);
    assert_eq!(find(&ui, &root, "retained-counter"), counter);
    assert!(text(&ui, "count").contains(&"Count: 1".into()));
    open(&mut ui, &root, "layout");
    assert!(ui.next_deadline().is_none());
    open(&mut ui, &root, "state");
    assert!(ui.next_deadline().is_some());
    click(&mut ui, &run);
    assert!(ui.next_deadline().is_none());
}
#[test]
fn split_locks_and_region_layout_edge_controls_change_real_components() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "splits");
    let button = find(&ui, &find(&ui, &root, "outer-lock"), "toggle");
    click(&mut ui, &button);
    let snapshot = ui.inspect();
    let outer = snapshot
        .components
        .iter()
        .find(|item| item.key == "outer-split")
        .unwrap();
    assert_eq!(
        outer.details.properties.get("resizable"),
        Some(&InspectValue::Bool(false))
    );
    open(&mut ui, &root, "shells");
    for key in ["lock-top", "lock-bottom", "lock-left", "lock-right"] {
        let button = find(&ui, &find(&ui, &root, key), "toggle");
        click(&mut ui, &button);
    }
    let snapshot = ui.inspect();
    let locked = Some(&InspectValue::Bool(true));
    assert!(
        snapshot
            .components
            .iter()
            .filter(|item| item.key.starts_with("split-"))
            .all(|item| item.details.properties.get("resizable") != locked)
    );
}
#[test]
fn the_layout_wide_switch_locks_every_region_edge_and_restores_them() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "shells");
    let resizable = |ui: &Runtime| -> Vec<InspectValue> {
        ui.inspect()
            .components
            .into_iter()
            .filter(|item| item.key.starts_with("split-"))
            .filter_map(|item| item.details.properties.get("resizable").cloned())
            .collect()
    };
    let before = resizable(&ui);
    assert!(before.contains(&InspectValue::Bool(true)));
    let button = find(&ui, &find(&ui, &root, "lock-all"), "toggle");
    click(&mut ui, &button);
    assert!(
        resizable(&ui)
            .iter()
            .all(|value| *value == InspectValue::Bool(false))
    );
    click(&mut ui, &button);
    assert_eq!(resizable(&ui), before);
}
#[test]
fn nested_tab_counters_and_scroll_reveal_use_the_gallery_controls() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "state");
    let nested = find(&ui, &root, "nested-tabs");
    let first = ui.children(&nested).unwrap()[0].clone();
    let increment = find(&ui, &first, "increment");
    click(&mut ui, &increment);
    select(&mut ui, &nested, 5);
    select(&mut ui, &nested, 0);
    assert_eq!(ui.children(&nested).unwrap()[0], first);
    assert!(text(&ui, "count").contains(&"Count: 1".into()));
    // Alt+Right moves the selected view one place; the application describes
    // the strip again and the view keeps its identity and its count.
    ui.focus(&nested).unwrap();
    ui.key_input(KeyCode::AltLeft, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Release)
        .unwrap();
    ui.key_input(KeyCode::AltLeft, KeyAction::Release).unwrap();
    settle(&mut ui);
    let moved = ui.children(&nested).unwrap();
    assert_eq!(moved[1], first);
    assert_eq!(ui.key(&moved[0]).unwrap(), "view-1");
    assert!(text(&ui, "count").contains(&"Count: 1".into()));
    open(&mut ui, &root, "scroll");
    let last = find(&ui, &find(&ui, &root, "cell-39"), "action");
    click(&mut ui, &last);
    let [x, y, w, h] = ui.bounds(&last).unwrap().unwrap().values();
    assert!(x < 960.0 && x + w > 0.0 && y < 640.0 && y + h > 0.0);
    assert_eq!(text(&ui, "status"), ["Cell 40 activated".to_owned()]);
    // The grid is allocated its fixed columns' width, so revealing the last
    // cell leaves every cell inside it.
    assert!(
        ui.validation_report().issues.is_empty(),
        "{:?}",
        ui.validation_report().issues
    );
    ui.paint().unwrap();
}
#[test]
fn content_split_drag_then_pointer_switch_to_input_settles_and_preserves_state() {
    let (mut ui, root) = mounted();
    for key in ["content-first", "content-second"] {
        open(&mut ui, &root, "splits");
        let toggle = find(&ui, &root, "change-content");
        click(&mut ui, &toggle);
        let split = find(&ui, &find(&ui, &root, key), "split");
        ui.focus(&split).unwrap();
        ui.flush(Duration::ZERO).unwrap();
        let children = ui.children(&split).unwrap();
        let [x, y, width, height] = ui.bounds(&children[0]).unwrap().unwrap().values();
        assert!(width > 0.0 && width < 632.0);
        let start = Point::new(x + width + 4.0, y + height / 2.0).unwrap();
        // Nearly collapse the first pane, then release before clicking the fixed tab strip.
        let end = Point::new(x + 5.0, start.y()).unwrap();
        for (point, kind) in [
            (start, PointerKind::Press(PointerButton::Primary)),
            (end, PointerKind::Move),
            (end, PointerKind::Release(PointerButton::Primary)),
        ] {
            ui.pointer_input(PointerEvent::new(point, kind)).unwrap();
        }
        ui.flush(Duration::ZERO).unwrap();
        let dragged = ui.bounds(&children[0]).unwrap().unwrap().width();
        assert!(dragged < 10.0);
        let point = input_category_point(&mut ui, &root);
        pointer_click(&mut ui, point);
        let selected = ui
            .inspect()
            .components
            .into_iter()
            .find(|item| item.key == "gallery-tabs")
            .unwrap();
        assert_eq!(
            selected.details.properties.get("selected"),
            Some(&InspectValue::Text("controls-category".into()))
        );
        open(&mut ui, &root, "input");
        let increase = find(&ui, &find(&ui, &root, "editor-0"), "increase");
        click(&mut ui, &increase);
        assert!(first_parameter(&mut ui).base > 0.0);
        assert!(ui.next_deadline().is_none());
        assert!(ui.validation_report().issues.is_empty());
        ui.paint().unwrap();
        open(&mut ui, &root, "splits");
        assert_eq!(ui.bounds(&children[0]).unwrap().unwrap().width(), dragged);
    }
}

#[test]
#[ignore = "manual CPU timing probe; excludes native event delivery and GPU presentation"]
fn split_drag_timings() {
    use std::time::Instant;
    for key in [
        "middle-split",
        "outer-split",
        "content-first",
        "content-second",
    ] {
        let (mut ui, root) = mounted();
        open(&mut ui, &root, "splits");
        let target = find(&ui, &root, key);
        let split = if key.starts_with("content-") {
            find(&ui, &target, "split")
        } else {
            target
        };
        ui.focus(&split).unwrap();
        ui.flush(Duration::ZERO).unwrap();
        let first = ui.children(&split).unwrap()[0].clone();
        let [x, y, w, h] = ui.bounds(&first).unwrap().unwrap().values();
        let horizontal = key != "outer-split";
        let start = Point::new(
            if horizontal { x + w + 4.0 } else { x + 10.0 },
            if horizontal { y + 10.0 } else { y + h + 4.0 },
        )
        .unwrap();
        ui.pointer_input(PointerEvent::new(
            start,
            PointerKind::Press(PointerButton::Primary),
        ))
        .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        ui.paint().unwrap();
        let mut input = Duration::ZERO;
        let mut layout = Duration::ZERO;
        let mut paint = Duration::ZERO;
        let mut worst = Duration::ZERO;
        for index in 0..120 {
            let offset =
                ((index % 60) as f32 + 1.0) * if key == "content-second" { -1.0 } else { 1.0 };
            let point = Point::new(
                start.x() + if horizontal { offset } else { 0.0 },
                start.y() + if horizontal { 0.0 } else { offset },
            )
            .unwrap();
            let t = Instant::now();
            ui.pointer_input(PointerEvent::new(point, PointerKind::Move))
                .unwrap();
            input += t.elapsed();
            let t = Instant::now();
            ui.flush(Duration::ZERO).unwrap();
            let elapsed = t.elapsed();
            layout += elapsed;
            worst = worst.max(elapsed);
            let t = Instant::now();
            ui.paint().unwrap();
            paint += t.elapsed();
        }
        let changed = ui.bounds(&first).unwrap().unwrap().values();
        assert_ne!(
            if horizontal { changed[2] } else { changed[3] },
            if horizontal { w } else { h }
        );
        ui.pointer_input(PointerEvent::new(
            start,
            PointerKind::Release(PointerButton::Primary),
        ))
        .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        let point = input_category_point(&mut ui, &root);
        let t = Instant::now();
        pointer_click(&mut ui, point);
        let switch_layout = t.elapsed();
        let t = Instant::now();
        ui.paint().unwrap();
        let switch_paint = t.elapsed();
        eprintln!("{key}: tab switch layout={switch_layout:?}, paint={switch_paint:?}");
        eprintln!(
            "{key}: 120 moves: input={input:?}, flush/layout={layout:?}, paint={paint:?}, worst flush={worst:?}"
        );
    }
}

#[test]
fn a_single_gallery_view_detaches_through_its_command_and_redocks() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "state");
    let view = find(&ui, &root, "detachable-view");
    let increment = find(&ui, &find(&ui, &view, "detachable-counter"), "increment");
    click(&mut ui, &increment);
    assert!(text(&ui, "count").contains(&"Count: 1".into()));

    let action = find(&ui, &root, "detach-action");
    assert_eq!(
        text(&ui, "command-status"),
        ["No button command invoked yet.".to_owned()]
    );
    click(&mut ui, &action);
    // The button's command receipt is shown, not dropped.
    assert_eq!(
        text(&ui, "command-status"),
        ["Button command: done".to_owned()]
    );
    let mut requests = ui.take_presentation_requests().unwrap();
    assert_eq!(requests.len(), 1, "one detach request per activation");
    let request = requests.pop().unwrap();
    let PresentationAction::Detach {
        view: requested,
        options,
    } = request.action()
    else {
        panic!("expected a detach request");
    };
    assert_eq!(requested, &view);
    let prepared = ui.prepare_detach(requested, options.clone()).unwrap();
    let detached = ui.commit_detach(prepared).unwrap();
    request
        .complete(PresentationStatus::Presented(detached.clone()))
        .unwrap();
    ui.flush_windows(Duration::ZERO).unwrap();
    assert_eq!(ui.window_of(&view).unwrap(), detached);
    assert!(text(&ui, "detach-status")[0].starts_with("Window operation: Presented"));

    // The same incarnation keeps its state and stays interactive in its own window.
    assert!(!ui.paint().unwrap().masks().is_empty());
    click(&mut ui, &increment);
    assert!(text(&ui, "count").contains(&"Count: 2".into()));

    // Asking again reports the existing window instead of duplicating content.
    let primary = ui.primary_window();
    ui.select_window(&primary).unwrap();
    ui.flush_windows(Duration::ZERO).unwrap();
    click(&mut ui, &action);
    assert!(ui.take_presentation_requests().unwrap().is_empty());
    assert_eq!(
        text(&ui, "status"),
        ["That view already has its own window.".to_owned()]
    );

    ui.redock(&detached).unwrap();
    ui.flush_windows(Duration::ZERO).unwrap();
    assert_eq!(ui.window_of(&view).unwrap(), primary);
    assert!(text(&ui, "count").contains(&"Count: 2".into()));
    assert!(ui.validation_report().issues.is_empty());

    // The neighboring view forbids its own window, so the same request is refused.
    let guarded = find(&ui, &root, "guarded-view");
    let guarded_action = find(&ui, &root, "guarded-action");
    click(&mut ui, &guarded_action);
    assert!(ui.take_presentation_requests().unwrap().is_empty());
    assert_eq!(
        text(&ui, "status"),
        ["Detach refused: view is not detachable".to_owned()]
    );
    // The command ran and reported the refusal itself; it did not fail.
    assert_eq!(
        text(&ui, "command-status"),
        ["Button command: done".to_owned()]
    );
    assert_eq!(
        ui.prepare_detach(&guarded, NativeWindowOptions::new("Protected").unwrap())
            .err(),
        Some(RuntimeError::NotDetachable)
    );
    assert_eq!(ui.window_of(&guarded).unwrap(), primary);

    // One of the six nested retained views is excluded from its detachable strip.
    let nested = find(&ui, &root, "nested-tabs");
    let details = &ui
        .inspect()
        .components
        .into_iter()
        .find(|item| ui.key(&nested).unwrap() == item.key)
        .unwrap()
        .details;
    assert_eq!(
        details.properties.get("movable_tabs"),
        Some(&InspectValue::Number(5.0))
    );
}

#[test]
fn the_transforms_page_zooms_pans_and_keeps_its_controls_working() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "transforms");
    // The text page draws on a canvas keyed "canvas" as well.
    let canvas = find(&ui, &find(&ui, &root, "canvas-stage"), "canvas");
    let card = find(&ui, &canvas, "card-a");
    let button = find(&ui, &card, "press");
    let before = ui.bounds(&button).unwrap().unwrap();
    let start = {
        let [x, y, w, h] = before.values();
        let [cx, cy, _, _] = ui.bounds(&canvas).unwrap().unwrap().values();
        [x - cx, y - cy, w, h]
    };
    // Zoom in over an empty part of the canvas near its top-left corner, which
    // stays put, so the first card remains inside the canvas. A plain wheel
    // pans; the primary modifier makes it zoom.
    let [cx, cy, _, _] = ui.bounds(&canvas).unwrap().unwrap().values();
    let corner = (cx + 4.0, cy + 4.0);
    let primary = KeyCode::primary();
    ui.key_input(primary, KeyAction::Press).unwrap();
    pointer_at(
        &mut ui,
        corner.0,
        corner.1,
        PointerKind::Scroll(Point::new(0.0, -100.0).unwrap()),
    );
    ui.key_input(primary, KeyAction::Release).unwrap();
    assert_eq!(text(&ui, "zoom-readout")[0].split('%').next(), Some("141"));
    let zoomed = ui.bounds(&button).unwrap().unwrap();
    assert!((zoomed.width() - before.width() * 2f32.sqrt()).abs() < 1e-3);
    // The control still works where it is now shown.
    let (x, y) = center_of(&ui, &button);
    pointer_at(&mut ui, x, y, PointerKind::Press(PointerButton::Primary));
    pointer_at(&mut ui, x, y, PointerKind::Release(PointerButton::Primary));
    assert_eq!(text(&ui, "press-status"), ["Oscillator: pressed"]);
    // Dragging the background pans by the distance dragged.
    pointer_at(
        &mut ui,
        corner.0,
        corner.1,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer_at(&mut ui, corner.0 + 30.0, corner.1 + 10.0, PointerKind::Move);
    pointer_at(
        &mut ui,
        corner.0 + 30.0,
        corner.1 + 10.0,
        PointerKind::Release(PointerButton::Primary),
    );
    let panned = ui.bounds(&button).unwrap().unwrap();
    assert!((panned.x() - zoomed.x() - 30.0).abs() < 1e-3);
    assert!((panned.y() - zoomed.y() - 10.0).abs() < 1e-3);
    // Fit shows both cards; Reset returns to where the page started.
    let fit = find(&ui, &root, "canvas-fit");
    click(&mut ui, &fit);
    let [cx, cy, cw, ch] = ui.bounds(&canvas).unwrap().unwrap().values();
    for key in ["card-a", "card-b"] {
        let [x, y, w, h] = ui
            .bounds(&find(&ui, &canvas, key))
            .unwrap()
            .unwrap()
            .values();
        assert!(
            x >= cx && y >= cy && x + w <= cx + cw && y + h <= cy + ch,
            "{key}"
        );
    }
    let reset = find(&ui, &root, "canvas-reset");
    click(&mut ui, &reset);
    // Focusing the button may scroll the page, so compare within the canvas.
    let relative = |ui: &Runtime| {
        let [x, y, w, h] = ui.bounds(&button).unwrap().unwrap().values();
        let [cx, cy, _, _] = ui.bounds(&canvas).unwrap().unwrap().values();
        [x - cx, y - cy, w, h]
    };
    assert_eq!(relative(&ui), start);
    assert_eq!(text(&ui, "zoom-readout"), ["100% at 0, 0"]);
    assert!(ui.validation_report().issues.is_empty());
    assert!(ui.next_deadline().is_none());
}

#[test]
fn the_timeline_zooms_time_and_amplitude_while_rows_keep_their_height() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "transforms");
    let timeline = find(&ui, &root, "timeline");
    ui.focus(&timeline).unwrap();
    settle(&mut ui);
    let before = ui.bounds(&timeline).unwrap().unwrap();
    let (x, y) = center_of(&ui, &timeline);
    let readout = |ui: &Runtime| text(ui, "timeline-readout")[0].clone();
    assert!(readout(&ui).contains("32 px/s"));
    let primary = KeyCode::primary();
    // Control with the wheel doubles time's scale at the pointer.
    ui.key_input(primary, KeyAction::Press).unwrap();
    pointer_at(
        &mut ui,
        x,
        y,
        PointerKind::Scroll(Point::new(0.0, -200.0).unwrap()),
    );
    assert!(readout(&ui).contains("64 px/s"), "{}", readout(&ui));
    assert!(readout(&ui).ends_with("×1.0"));
    // With Shift, the amplitude zooms and time stays put.
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Press).unwrap();
    pointer_at(
        &mut ui,
        x,
        y,
        PointerKind::Scroll(Point::new(0.0, -200.0).unwrap()),
    );
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Release)
        .unwrap();
    ui.key_input(primary, KeyAction::Release).unwrap();
    assert!(readout(&ui).contains("64 px/s") && readout(&ui).ends_with("×2.0"));
    // A pinch zooms time too.
    pointer_at(
        &mut ui,
        x,
        y,
        PointerKind::Pinch(Magnification::new(2.0).unwrap()),
    );
    assert!(readout(&ui).contains("128 px/s"));
    // Dragging pans time; the start moves earlier by the distance dragged.
    let start = readout(&ui);
    pointer_at(&mut ui, x, y, PointerKind::Press(PointerButton::Primary));
    pointer_at(&mut ui, x + 128.0, y, PointerKind::Move);
    pointer_at(
        &mut ui,
        x + 128.0,
        y,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_ne!(readout(&ui), start);
    // Shift with a pinch zooms the amplitude.
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Press).unwrap();
    pointer_at(
        &mut ui,
        x,
        y,
        PointerKind::Pinch(Magnification::new(2.0).unwrap()),
    );
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Release)
        .unwrap();
    assert!(readout(&ui).ends_with("×4.0"), "{}", readout(&ui));
    press(&mut ui, KeyCode::Home);
    // The whole hour across the timeline's width.
    let per_second = f64::from(ui.bounds(&timeline).unwrap().unwrap().width()) / 3600.0;
    assert_eq!(
        readout(&ui),
        format!("From 0:00.000 at {per_second:.2} px/s, amplitude ×1.0")
    );
    // Zoom is a mapping: the timeline's own geometry never changed.
    let after = ui.bounds(&timeline).unwrap().unwrap();
    assert_eq!(
        (after.width(), after.height()),
        (before.width(), before.height())
    );
    assert!(ui.validation_report().issues.is_empty());
    assert!(ui.next_deadline().is_none());
}

fn click_in(ui: &mut Runtime, root: &WidgetId, key: &str) {
    let button = find(ui, root, key);
    click(ui, &button);
}

fn navigation_strip_selection(ui: &Runtime) -> Option<InspectValue> {
    ui.inspect()
        .components
        .iter()
        .find(|component| component.key == "nav-strip")
        .and_then(|component| component.details.properties.get("selected").cloned())
}

fn reveal_readout(ui: &Runtime) -> Vec<String> {
    text(ui, "reveal-status")
}

#[test]
fn the_navigation_page_reveals_selects_by_key_and_jumps_across_pages() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "navigation");
    let demo = page(&ui, &root, "navigation");
    let role = |ui: &Runtime, key: &str| {
        ui.inspect()
            .components
            .into_iter()
            .find(|component| component.key == key)
            .and_then(|component| component.details.semantics)
            .map(|semantics| (semantics.role, semantics.disabled))
    };
    assert_eq!(
        role(&ui, "reveal-output"),
        Some((SemanticRole::Link, false))
    );
    assert_eq!(
        role(&ui, "jump-unavailable"),
        Some((SemanticRole::Link, true))
    );

    assert_eq!(
        text(&ui, "selection-log"),
        ["Strip selection: general, initially".to_owned()]
    );
    click_in(&mut ui, &demo, "reveal-output");
    assert_eq!(reveal_readout(&ui), ["Revealed".to_owned()]);
    assert_eq!(
        text(&ui, "selection-log"),
        ["Strip selection: mixer, by a reveal".to_owned()]
    );
    assert_eq!(
        navigation_strip_selection(&ui),
        Some(InspectValue::Text("mixer".into()))
    );
    let output = find(&ui, &demo, "output-level");
    let [_, y, _, h] = ui.bounds(&output).unwrap().unwrap().values();
    assert!(y >= 0.0 && y + h <= 640.0, "scrolled into view at {y}");
    assert_ne!(ui.focused(), Some(output.clone()));

    click_in(&mut ui, &demo, "focus-output");
    assert_eq!(reveal_readout(&ui), ["Revealed and focused".to_owned()]);
    assert_eq!(ui.focused(), Some(output));

    // The advanced option's view follows the application's toggle.
    click_in(&mut ui, &demo, "reveal-expert");
    assert_eq!(
        reveal_readout(&ui),
        ["Refused: the application controls that view's visibility".to_owned()]
    );
    assert_eq!(
        navigation_strip_selection(&ui),
        Some(InspectValue::Text("advanced".into())),
        "the strip opened before the refusal stays open"
    );
    let show = find(&ui, &demo, "show-advanced");
    click_in(&mut ui, &show, "toggle");
    click_in(&mut ui, &demo, "reveal-expert");
    assert_eq!(reveal_readout(&ui), ["Revealed and focused".to_owned()]);

    click_in(&mut ui, &demo, "select-general");
    assert_eq!(
        navigation_strip_selection(&ui),
        Some(InspectValue::Text("general".into()))
    );
    assert_eq!(
        text(&ui, "status"),
        ["Select General by key: Changed".to_owned()]
    );

    // The internal link opens another category and its page.
    click_in(&mut ui, &demo, "jump-collections");
    let collections = page(&ui, &root, "collections");
    let key = ui.key(&collections).unwrap().to_owned();
    assert!(
        ui.inspect()
            .components
            .iter()
            .any(|component| component.key == key && component.visible),
        "the Collections page is shown"
    );
    // The hidden readout catches up when its page is shown again.
    open(&mut ui, &root, "navigation");
    assert_eq!(reveal_readout(&ui), ["Revealed".to_owned()]);
    assert!(ui.validation_report().issues.is_empty());
}

fn shown_in(ui: &Runtime, root: &WidgetId, key: &str) -> bool {
    // Fails on a key repeated below `root`, so the key names one component.
    let _ = find(ui, root, key);
    ui.inspect()
        .components
        .iter()
        .any(|component| component.key == key && component.visible)
}

#[test]
fn the_sections_page_toggles_keeps_hidden_state_and_follows_the_switch() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "sections");
    let demo = page(&ui, &root, "sections");
    let remembered = find(&ui, &demo, "remembered");
    assert!(shown_in(&ui, &demo, "section-note"));
    // A draft in the note survives collapsing its section.
    let note = find(&ui, &demo, "section-note");
    ui.focus(&note).unwrap();
    ui.text_input("kept").unwrap();
    settle(&mut ui);
    ui.focus(&remembered).unwrap();
    press(&mut ui, KeyCode::Enter);
    assert!(!shown_in(&ui, &demo, "section-note"));
    press(&mut ui, KeyCode::Space);
    assert_eq!(find(&ui, &demo, "section-note"), note);
    assert_eq!(text(&ui, "section-note"), ["kept".to_owned()]);

    // The bound section follows the page's switch.
    assert!(!shown_in(&ui, &demo, "bound-body"));
    let switch = find(&ui, &demo, "bound-switch");
    let toggle = find(&ui, &switch, "toggle");
    click(&mut ui, &toggle);
    assert!(shown_in(&ui, &demo, "bound-body"));

    // The single-mode accordion keeps one section open.
    let single = find(&ui, &demo, "single");
    ui.focus(&single).unwrap();
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::Enter);
    assert!(shown_in(&ui, &demo, "single-display"));
    assert!(!shown_in(&ui, &demo, "single-general"));
    assert!(ui.validation_report().issues.is_empty());
    assert!(ui.next_deadline().is_none());
}

#[test]
fn the_navigation_page_reveals_into_a_closed_accordion_section() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "navigation");
    let demo = page(&ui, &root, "navigation");
    click_in(&mut ui, &demo, "reveal-output");
    let buried = find(&ui, &demo, "buried-option");
    click_in(&mut ui, &demo, "reveal-buried");
    assert_eq!(reveal_readout(&ui), ["Revealed and focused".to_owned()]);
    assert_eq!(
        navigation_strip_selection(&ui),
        Some(InspectValue::Text("general".into()))
    );
    assert_eq!(ui.focused(), Some(buried));
    assert!(
        !shown_in(&ui, &demo, "general-text"),
        "single mode closed Summary"
    );
}

#[test]
fn the_navigation_page_moves_back_and_forward_through_its_history() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "navigation");
    let demo = page(&ui, &root, "navigation");
    let strip = find(&ui, &demo, "history-strip");
    // The demo's own controls, not the gallery's in the menu bar.
    let disabled = |ui: &Runtime, key: &str| {
        ui.inspect()
            .components
            .into_iter()
            .find(|component| {
                component.key == key
                    && component.visible
                    && component.path.as_ref().is_some_and(|path| {
                        path.iter().any(|segment| segment == "history-controls")
                    })
            })
            .and_then(|component| component.details.semantics)
            .map(|semantics| semantics.disabled)
    };
    assert_eq!(text(&ui, "history-shown"), ["Showing Alpha".to_owned()]);
    assert_eq!(disabled(&ui, "back"), Some(true));
    // The user opens Beta, then Gamma: two visits.
    ui.focus(&strip).unwrap();
    press(&mut ui, KeyCode::ArrowRight);
    press(&mut ui, KeyCode::ArrowRight);
    assert_eq!(text(&ui, "history-shown"), ["Showing Gamma".to_owned()]);
    let back = find(&ui, &demo, "back");
    click(&mut ui, &back);
    assert_eq!(text(&ui, "history-shown"), ["Showing Beta".to_owned()]);
    // Restoring is a reveal and records nothing, so forward still leads on.
    assert_eq!(disabled(&ui, "forward"), Some(false));
    click(&mut ui, &back);
    assert_eq!(text(&ui, "history-shown"), ["Showing Alpha".to_owned()]);
    assert_eq!(disabled(&ui, "back"), Some(true));
    // The list goes straight to Gamma.
    let list = find(&ui, &demo, "history");
    click(&mut ui, &list);
    ui.text_input("gam").unwrap();
    settle(&mut ui);
    // Native check drivers may deliver Enter during a flush. Closing the
    // choosing row must not cancel the demo's queued restoration.
    let _enter = ui
        .defer(|ui| {
            ui.key_input(KeyCode::Enter, KeyAction::Press)?;
            ui.key_input(KeyCode::Enter, KeyAction::Release)?;
            Ok(())
        })
        .unwrap();
    settle(&mut ui);
    settle(&mut ui);
    assert_eq!(text(&ui, "history-shown"), ["Showing Gamma".to_owned()]);
    assert_eq!(disabled(&ui, "forward"), Some(true));
    assert!(ui.validation_report().issues.is_empty());
}

#[test]
fn the_gallery_addresses_name_the_components_its_links_and_commands_use() {
    use crate::gallery::{find_at, gallery_tabs, page_address};
    use crate::pages::layout::NavigationTargets;
    let (ui, root) = mounted();
    let top = gallery_tabs(&ui, &root).unwrap();
    let demo = page(&ui, &root, "navigation");
    // Each address names the one component with that key on its page, which
    // a key search confirms because these keys are unique there.
    for (key, path) in NavigationTargets::ADDRESSES {
        let address = page_address("layout-category", "navigation", path);
        let found = find_at(&ui, &top, &address).unwrap();
        assert_eq!(found, find(&ui, &demo, key), "{address:?}");
    }
    // The pages the history and the Go to commands select, and the views
    // the detach commands move.
    for category in ["layout-category", "data-category", "text-category"] {
        let strip = find_at(&ui, &top, &[category, "category-pages"]).unwrap();
        assert_eq!(ui.key(&strip).unwrap(), "category-pages");
    }
    for view in ["detachable-view", "guarded-view"] {
        let address = page_address(
            "layout-category",
            "state",
            &["detach-view", "content", view],
        );
        assert_eq!(
            ui.key(&find_at(&ui, &top, &address).unwrap()).unwrap(),
            view
        );
    }
    // A key that pages repeat is told apart by its address, and an address
    // that names nothing is an error naming it.
    assert!(ui.find_descendant(&top, "help").is_err());
    let missing = find_at(
        &ui,
        &top,
        &page_address("layout-category", "navigation", &["moved"]),
    );
    let message = missing
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();
    assert!(
        message.contains("navigation › page › sections › moved"),
        "{message}"
    );
}

#[test]
fn the_navigation_page_reveals_into_its_custom_drawer() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "navigation");
    let demo = page(&ui, &root, "navigation");
    let setting = find(&ui, &demo, "drawer-setting");
    assert!(
        !shown_in(&ui, &demo, "drawer-setting"),
        "the drawer starts closed"
    );
    click_in(&mut ui, &demo, "reveal-drawer");
    assert_eq!(text(&ui, "drawer-status"), ["Revealed and focused"]);
    assert!(shown_in(&ui, &demo, "drawer-setting"));
    assert_eq!(ui.focused(), Some(setting));
    // The reveal opened the drawer through the state its own switch shows.
    let switch = |ui: &Runtime| {
        ui.inspect()
            .components
            .into_iter()
            .find(|component| component.key == "drawer-open")
            .and_then(|component| component.details.semantics)
            .and_then(|semantics| semantics.value)
    };
    assert_eq!(switch(&ui), Some(SemanticValue::Bool(true)));
    click_in(&mut ui, &demo, "drawer-open");
    assert_eq!(switch(&ui), Some(SemanticValue::Bool(false)));
    assert!(
        !shown_in(&ui, &demo, "drawer-setting"),
        "the switch closes it"
    );
    assert!(ui.validation_report().issues.is_empty());
}

#[test]
fn the_navigation_page_has_a_rich_link() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "navigation");
    let demo = page(&ui, &root, "navigation");
    let semantics = ui
        .inspect()
        .components
        .into_iter()
        .find(|component| component.key == "jump-rich")
        .and_then(|component| component.details.semantics)
        .expect("the rich link reports semantics");
    assert_eq!(semantics.role, SemanticRole::Link);
    assert_eq!(
        semantics.name.as_deref(),
        Some("Reveal the mixer's Output level")
    );
    click_in(&mut ui, &demo, "jump-rich");
    assert_eq!(reveal_readout(&ui), ["Revealed and focused".to_owned()]);
    assert_eq!(ui.focused(), Some(find(&ui, &demo, "output-level")));
}

/// The names of the rows the nonmodal palette's list shows, empty while it
/// is closed.
fn nonmodal_palette_rows(ui: &Runtime) -> Vec<String> {
    let inspection = ui.inspect();
    let Some(palette) = inspection
        .components
        .iter()
        .find(|component| component.key == "nonmodal-palette")
    else {
        return Vec::new();
    };
    let inside = |component: &ComponentInspection| {
        let mut parent = component.parent;
        while let Some(id) = parent {
            if id == palette.id {
                return true;
            }
            parent = inspection
                .components
                .iter()
                .find(|candidate| candidate.id == id)
                .and_then(|candidate| candidate.parent);
        }
        false
    };
    inspection
        .components
        .iter()
        .filter(|component| component.visible && inside(component))
        .filter_map(|component| component.details.semantics.as_ref())
        .filter(|semantics| semantics.role == SemanticRole::ListBox)
        .flat_map(|semantics| semantics.items.iter().filter_map(|item| item.name.clone()))
        .collect()
}

#[test]
fn the_navigation_page_opens_a_nonmodal_palette_with_its_own_matcher() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "navigation");
    let demo = page(&ui, &root, "navigation");
    click_in(&mut ui, &demo, "palette-open");
    assert!(
        !nonmodal_palette_rows(&ui).is_empty(),
        "the palette is open"
    );
    // Every typed word must begin a word of the name.
    ui.text_input("go col").unwrap();
    settle(&mut ui);
    let rows = nonmodal_palette_rows(&ui);
    assert!(
        rows.iter().any(|row| row.starts_with("Go to: Collections")),
        "{rows:?}"
    );
    assert!(
        rows.iter().all(|row| row.starts_with("Go to: Col")),
        "{rows:?}"
    );
    // Text inside a word does not match, unlike the default fuzzy matcher.
    press(&mut ui, KeyCode::Escape);
    ui.text_input("llect").unwrap();
    settle(&mut ui);
    assert!(nonmodal_palette_rows(&ui).is_empty());
    // The page stays usable: the surface is nonmodal and closes when focus
    // leaves it.
    click_in(&mut ui, &demo, "reveal-output");
    assert_eq!(reveal_readout(&ui), ["Revealed".to_owned()]);
    assert!(nonmodal_palette_rows(&ui).is_empty(), "the palette closed");
    assert!(ui.validation_report().issues.is_empty());
}

#[test]
fn the_tool_windows_page_expands_resizes_and_keeps_the_locked_layout_fixed() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "tool-windows");
    let demo = page(&ui, &root, "tool-windows");
    let open_stage = find(&ui, &demo, "open-stage");
    let locked_stage = find(&ui, &demo, "locked-stage");
    let initial_width = ui.bounds(&open_stage).unwrap().unwrap().width();
    assert_eq!(
        initial_width,
        ui.bounds(&locked_stage).unwrap().unwrap().width()
    );
    assert!(ui.bounds(&open_stage).unwrap().unwrap().height() >= 440.0);
    assert!(shown_in(&ui, &open_stage, "open-files-body"));
    click_in(&mut ui, &open_stage, "open-files-open");
    assert_eq!(text(&ui, "open-main-selection"), ["src/main.rs".to_owned()]);
    assert_eq!(
        text(&ui, "locked-main-selection"),
        ["src/main.rs".to_owned()]
    );
    click_in(&mut ui, &open_stage, "open-main-button");
    assert_eq!(
        text(&ui, "locked-main-activity"),
        ["Reviewed src/main.rs in the main view".to_owned()]
    );
    // The open layout: the stripe's second button expands Search and moves
    // focus into it; Escape returns to the main content.
    let stripe = find(&ui, &open_stage, "stripe-left");
    ui.focus(&stripe).unwrap();
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::Enter);
    assert!(shown_in(&ui, &open_stage, "open-search-body"));
    assert!(!shown_in(&ui, &open_stage, "open-files-body"));
    assert_eq!(
        ui.focused(),
        Some(find(&ui, &open_stage, "open-search-body"))
    );
    press(&mut ui, KeyCode::Escape);
    assert_eq!(
        ui.focused(),
        Some(find(&ui, &open_stage, "open-main-button"))
    );
    assert_eq!(
        text(&ui, "last-change"),
        ["open: open-search expanded (User)".to_owned()]
    );

    // The locked layout's stripe cannot toggle, but its controller can.
    let locked_stripe = find(&ui, &locked_stage, "stripe-left");
    ui.focus(&locked_stripe).unwrap();
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::Enter);
    assert!(!shown_in(&ui, &locked_stage, "locked-search-body"));
    click_in(&mut ui, &demo, "toggle-locked-properties");
    assert!(shown_in(&ui, &locked_stage, "locked-properties-body"));
    assert_eq!(
        text(&ui, "locked-properties-selection"),
        ["src/main.rs".to_owned()]
    );

    // Names on both stripes follow the page's switch.
    let narrow = ui.bounds(&stripe).unwrap().unwrap().width();
    let switch = find(&ui, &demo, "show-names");
    let toggle = find(&ui, &switch, "toggle");
    click(&mut ui, &toggle);
    assert!(ui.bounds(&stripe).unwrap().unwrap().width() > narrow);

    // The nested layout: revealing the inner tool window opens both.
    let item = find(&ui, &demo, "nested-outline-body");
    ui.reveal_and_focus(&item).unwrap();
    settle(&mut ui);
    assert!(shown_in(&ui, &demo, "nested-outline-body"));
    press(&mut ui, KeyCode::Escape);
    assert_eq!(ui.focused(), Some(find(&ui, &demo, "inner-editor")));
    ui.set_viewport(Viewport::new(1120.0, 640.0, 1.0).unwrap())
        .unwrap();
    settle(&mut ui);
    assert!(ui.bounds(&open_stage).unwrap().unwrap().width() > initial_width);
    assert_eq!(
        ui.bounds(&open_stage).unwrap().unwrap().width(),
        ui.bounds(&locked_stage).unwrap().unwrap().width()
    );
    assert!(ui.validation_report().issues.is_empty());
    assert!(ui.next_deadline().is_none());
}

#[test]
fn the_tool_windows_page_moves_tool_windows_and_hides_its_overlay() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "tool-windows");
    let demo = page(&ui, &root, "tool-windows");
    let open_stage = find(&ui, &demo, "open-stage");
    // The open Files panel's options menu moves it to the right edge; its
    // first enabled item is the first edge it is not on.
    let files = find(&ui, &open_stage, "open-files");
    let options = find(&ui, &files, "options");
    let trigger = find(&ui, &options, "button");
    ui.focus(&trigger).unwrap();
    press(&mut ui, KeyCode::Enter);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(
        text(&ui, "last-change"),
        ["open: open-files moved to Right Start (User)".to_owned()]
    );
    assert!(shown_in(&ui, &open_stage, "open-files-body"));
    // The locked layout's panels have no options menu.
    let locked_stage = find(&ui, &demo, "locked-stage");
    click_in(&mut ui, &demo, "toggle-locked-properties");
    let properties = find(&ui, &locked_stage, "locked-properties");
    assert!(
        ui.find_descendant(&properties, "options")
            .unwrap()
            .is_none()
    );
    // Notes is an overlay: focus leaving it hides it.
    let stripe = find(&ui, &open_stage, "stripe-bottom");
    ui.focus(&stripe).unwrap();
    press(&mut ui, KeyCode::End);
    press(&mut ui, KeyCode::Enter);
    assert!(shown_in(&ui, &open_stage, "open-notes-body"));
    ui.focus(&find(&ui, &open_stage, "open-main-button"))
        .unwrap();
    settle(&mut ui);
    assert!(!shown_in(&ui, &open_stage, "open-notes-body"));
    assert!(ui.validation_report().issues.is_empty());
    assert!(ui.next_deadline().is_none());
}

#[test]
fn the_tool_windows_page_floats_a_tool_window_and_asks_for_its_own_window() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "tool-windows");
    let demo = page(&ui, &root, "tool-windows");
    let open_stage = find(&ui, &demo, "open-stage");
    let files = find(&ui, &open_stage, "open-files");
    let docked = ui.bounds(&files).unwrap().unwrap();
    // The options menu ends with Dock, Float and Open in window.
    let menu = |ui: &mut Runtime| {
        let options = find(ui, &files, "options");
        let trigger = find(ui, &options, "button");
        ui.focus(&trigger).unwrap();
        press(ui, KeyCode::Enter);
        press(ui, KeyCode::End);
    };
    menu(&mut ui);
    press(&mut ui, KeyCode::ArrowUp);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(
        text(&ui, "last-change"),
        ["open: open-files floated (User)".to_owned()]
    );
    let floating = ui.bounds(&files).unwrap().unwrap();
    assert_ne!(floating, docked, "the panel floats over the layout");
    assert!(shown_in(&ui, &open_stage, "open-files-body"));
    menu(&mut ui);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(
        text(&ui, "last-change"),
        ["open: open-files set to open in its own window (User)".to_owned()]
    );
    let requests = ui.take_presentation_requests().unwrap();
    assert!(
        requests
            .iter()
            .any(|request| matches!(request.action(), PresentationAction::Detach { view, .. } if *view == files)),
        "the panel asks for its own native window"
    );
}

/// The border box of the widget keyed `key` below `parent`.
fn border(ui: &Runtime, parent: &WidgetId, key: &str) -> [f32; 4] {
    let id = find(ui, parent, key);
    ui.bounds(&id).unwrap().unwrap().values()
}

#[test]
fn the_layout_page_grows_justifies_and_stretches_its_flex_samples() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "layout");
    let page = page(&ui, &root, "layout");
    // The growing tile takes the spare width and the spacer pushes Help to the
    // stage's end.
    let grow = find(&ui, &page, "grow-row");
    let [sx, _, sw, _] = ui.bounds(&grow).unwrap().unwrap().values();
    let [_, _, name_w, _] = border(&ui, &grow, "name");
    let [_, _, grows_w, _] = border(&ui, &grow, "grows");
    let [hx, _, hw, _] = border(&ui, &grow, "help");
    assert!(grows_w > 2.0 * name_w, "{grows_w} vs {name_w}");
    assert!(
        (hx + hw - (sx + sw)).abs() < 1.0,
        "Help ends at the stage's end"
    );
    // SpaceBetween puts the last tile at the end.
    let spread = find(&ui, &page, "spread-row");
    let [px, _, pw, _] = ui.bounds(&spread).unwrap().unwrap().values();
    let [cx, _, cw, _] = border(&ui, &spread, "c");
    assert!((cx + cw - (px + pw)).abs() < 1.0);
    // Stretch gives both tiles the row's height.
    let stretch = find(&ui, &page, "stretch-row");
    let [_, _, _, short_h] = border(&ui, &stretch, "short");
    let [_, _, _, tall_h] = border(&ui, &stretch, "tall");
    assert_eq!(short_h, tall_h);
    assert!(short_h >= 90.0, "{short_h}");
    // The column grows its body into the stage.
    let column = find(&ui, &page, "grow-column");
    let [_, _, _, header_h] = border(&ui, &column, "header");
    let [_, _, _, body_h] = border(&ui, &column, "body");
    assert!(body_h > header_h, "{body_h} vs {header_h}");
}

#[test]
fn the_bound_box_follows_its_role_binding() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "layout");
    let page = page(&ui, &root, "layout");
    // The fill is recorded when the box paints.
    let background = |ui: &mut Runtime| {
        ui.paint().unwrap();
        let id = ui.inspection_id(&find(ui, &page, "bound-box")).unwrap();
        ui.inspect()
            .components
            .into_iter()
            .find(|c| c.id == id)
            .unwrap()
            .details
            .box_model
            .background
    };
    let before = background(&mut ui);
    assert!(before.is_some());
    let toggle = find(&ui, &find(&ui, &page, "mark-box"), "toggle");
    click(&mut ui, &toggle);
    let after = background(&mut ui);
    assert_ne!(before, after);
    click(&mut ui, &toggle);
    assert_eq!(background(&mut ui), before);
}
