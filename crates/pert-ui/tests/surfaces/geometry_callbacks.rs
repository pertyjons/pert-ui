//! Live geometry observation through the public facade: `ScrollArea::on_scroll`,
//! `SplitPane::on_resize`, `RegionLayout::on_resize` and
//! `PropertyGrid::on_resize` report committed values with their origin from an
//! event phase, never a preview or a cancelled gesture, at most once per
//! flush, follow callback replacement and removal, and propagate a callback
//! error without replaying the change.
use crate::support;
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};
use support::{frame, key, widget};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type Log<T> = Rc<RefCell<Vec<(T, ChangeOrigin)>>>;

fn runtime(width: f32, height: f32) -> Result<Runtime, Box<dyn std::error::Error>> {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(width, height, 1.0)?)?;
    Ok(ui)
}

fn pointer(ui: &mut Runtime, x: f32, y: f32, kind: PointerKind) -> Result<(), RuntimeError> {
    ui.pointer_input(PointerEvent::new(Point::new(x, y)?, kind))?;
    Ok(())
}

// ---- ScrollArea ----

/// Twenty rows of 20 logical pixels, the last one a button that can be revealed.
fn rows(count: usize) -> Element {
    let mut rows: Vec<Element> = (0..count)
        .map(|index| {
            Element::new(
                format!("row-{index}"),
                Container::new(Element::new("text", Label::new(format!("Row {index}"))))
                    .preferred_size(Size::from([100, 20])),
            )
        })
        .collect();
    rows.push(Element::new("last", Button::new("Last", |_| Ok(()))));
    Element::new("rows", Column::new(rows))
}

fn scrolling(content: Element, log: &Log<Point>, axes: ScrollAxes) -> Element {
    let log = log.clone();
    Element::new(
        "area",
        ScrollArea::new(content)
            .axes(axes)
            .on_scroll(move |_, offset, origin| {
                log.borrow_mut().push((offset, origin));
                Ok(())
            }),
    )
}

fn offsets(log: &Log<Point>) -> Vec<(f32, ChangeOrigin)> {
    log.borrow().iter().map(|(p, o)| (p.y(), *o)).collect()
}

#[test]
fn scrolling_reports_user_changes_once_per_flush_with_the_offset() -> TestResult {
    let mut ui = runtime(200.0, 100.0)?;
    let log = Log::default();
    let root = ui.mount_tree(Element::new(
        "root",
        View::new(scrolling(rows(20), &log, ScrollAxes::Vertical)),
    ))?;
    frame(&mut ui);
    assert!(log.borrow().is_empty(), "mounting reports nothing");

    // The wheel: each input dispatch flushes, so each step reports once.
    for _ in 0..3 {
        pointer(
            &mut ui,
            50.0,
            50.0,
            PointerKind::Scroll(Point::new(0.0, 15.0)?),
        )?;
    }
    assert_eq!(
        offsets(&log),
        [
            (15.0, ChangeOrigin::User),
            (30.0, ChangeOrigin::User),
            (45.0, ChangeOrigin::User)
        ]
    );
    log.borrow_mut().clear();

    // Keys.
    let area = widget(&ui, &root, "area");
    ui.focus(&area)?;
    key(&mut ui, KeyCode::ArrowDown);
    frame(&mut ui);
    assert_eq!(offsets(&log), [(85.0, ChangeOrigin::User)]);
    log.borrow_mut().clear();

    // A wheel step at the end reports nothing: the offset does not change.
    key(&mut ui, KeyCode::End);
    frame(&mut ui);
    let end = log.borrow().last().map(|(p, _)| p.y()).ok_or("no end")?;
    log.borrow_mut().clear();
    pointer(
        &mut ui,
        50.0,
        50.0,
        PointerKind::Scroll(Point::new(0.0, 40.0)?),
    )?;
    frame(&mut ui);
    assert!(log.borrow().is_empty(), "already at {end}");

    // The scrollbar: a press on the track moves the thumb there.
    key(&mut ui, KeyCode::Home);
    frame(&mut ui);
    log.borrow_mut().clear();
    pointer(
        &mut ui,
        196.0,
        90.0,
        PointerKind::Press(PointerButton::Primary),
    )?;
    pointer(
        &mut ui,
        196.0,
        90.0,
        PointerKind::Release(PointerButton::Primary),
    )?;
    frame(&mut ui);
    assert_eq!(log.borrow().len(), 1);
    assert_eq!(log.borrow()[0].1, ChangeOrigin::User);
    assert!(log.borrow()[0].0.y() > 0.0);
    Ok(())
}

#[test]
fn revealing_a_child_reports_reveal() -> TestResult {
    let mut ui = runtime(200.0, 100.0)?;
    let log = Log::default();
    let root = ui.mount_tree(Element::new(
        "root",
        View::new(scrolling(rows(20), &log, ScrollAxes::Vertical)),
    ))?;
    frame(&mut ui);
    let last = widget(&ui, &root, "last");
    ui.reveal(&last)?;
    frame(&mut ui);
    assert_eq!(log.borrow().len(), 1);
    assert_eq!(log.borrow()[0].1, ChangeOrigin::Reveal);
    assert!(log.borrow()[0].0.y() > 0.0);
    Ok(())
}

#[test]
fn a_restore_reports_restore_with_the_clamped_offset() -> TestResult {
    // Capture a scrolled viewport.
    let mut source = runtime(200.0, 100.0)?;
    let ignored = Log::default();
    let root = source.mount_tree(Element::new(
        "root",
        View::new(scrolling(rows(20), &ignored, ScrollAxes::Vertical)),
    ))?;
    frame(&mut source);
    let area = widget(&source, &root, "area");
    source.focus(&area)?;
    key(&mut source, KeyCode::End);
    frame(&mut source);
    let saved = source.capture_workspace()?;

    // Restoring into a viewport with less content: arrangement clamps the
    // restored offset, and the one report keeps the restore's origin.
    let mut ui = runtime(200.0, 100.0)?;
    let log = Log::default();
    ui.mount_tree(Element::new(
        "root",
        View::new(scrolling(rows(10), &log, ScrollAxes::Vertical)),
    ))?;
    frame(&mut ui);
    let report = ui.restore_workspace(&saved)?;
    assert_eq!(report.rejected().count(), 0);
    frame(&mut ui);
    assert_eq!(log.borrow().len(), 1, "{:?}", log.borrow());
    let (offset, origin) = log.borrow()[0];
    assert_eq!(origin, ChangeOrigin::Restore);
    // Ten rows and the button no longer reach the saved offset.
    assert!(offset.y() < ignored.borrow().last().ok_or("no source scroll")?.0.y());
    Ok(())
}

#[test]
fn clamping_after_redescription_reports_redescription() -> TestResult {
    let mut ui = runtime(200.0, 100.0)?;
    let log = Log::default();
    let root = ui.mount_tree(Element::new(
        "root",
        View::new(scrolling(rows(20), &log, ScrollAxes::Vertical)),
    ))?;
    frame(&mut ui);
    let area = widget(&ui, &root, "area");
    ui.focus(&area)?;
    key(&mut ui, KeyCode::End);
    frame(&mut ui);
    log.borrow_mut().clear();

    // Less content: the offset clamps.
    ui.reconcile_children(&root, vec![scrolling(rows(6), &log, ScrollAxes::Vertical)])?;
    frame(&mut ui);
    assert_eq!(log.borrow().len(), 1);
    assert_eq!(log.borrow()[0].1, ChangeOrigin::Redescription);

    // No vertical axis: the offset returns to zero.
    log.borrow_mut().clear();
    ui.reconcile_children(&root, vec![scrolling(rows(6), &log, ScrollAxes::None)])?;
    frame(&mut ui);
    assert_eq!(offsets(&log), [(0.0, ChangeOrigin::Redescription)]);
    Ok(())
}

#[test]
fn a_redescribed_viewport_replaces_or_removes_its_callback() -> TestResult {
    let mut ui = runtime(200.0, 100.0)?;
    let first = Log::default();
    let second = Log::default();
    let root = ui.mount_tree(Element::new(
        "root",
        View::new(scrolling(rows(20), &first, ScrollAxes::Vertical)),
    ))?;
    frame(&mut ui);
    ui.reconcile_children(
        &root,
        vec![scrolling(rows(20), &second, ScrollAxes::Vertical)],
    )?;
    frame(&mut ui);
    let area = widget(&ui, &root, "area");
    ui.focus(&area)?;
    key(&mut ui, KeyCode::ArrowDown);
    frame(&mut ui);
    assert!(first.borrow().is_empty());
    assert_eq!(offsets(&second), [(40.0, ChangeOrigin::User)]);

    // Without a callback nothing is reported, and a callback added later
    // starts from the offset it finds.
    ui.reconcile_children(&root, vec![Element::new("area", ScrollArea::new(rows(20)))])?;
    frame(&mut ui);
    key(&mut ui, KeyCode::ArrowDown);
    frame(&mut ui);
    assert_eq!(second.borrow().len(), 1);
    ui.reconcile_children(
        &root,
        vec![scrolling(rows(20), &second, ScrollAxes::Vertical)],
    )?;
    frame(&mut ui);
    assert_eq!(
        second.borrow().len(),
        1,
        "adding a callback reports nothing"
    );
    key(&mut ui, KeyCode::ArrowDown);
    frame(&mut ui);
    assert_eq!(
        second.borrow().last(),
        Some(&(Point::new(0.0, 120.0)?, ChangeOrigin::User))
    );
    Ok(())
}

#[test]
fn a_failing_scroll_callback_is_not_replayed() -> TestResult {
    let mut ui = runtime(200.0, 100.0)?;
    let calls = Rc::new(RefCell::new(0));
    let counter = calls.clone();
    let root = ui.mount_tree(Element::new(
        "root",
        View::new(Element::new(
            "area",
            ScrollArea::new(rows(20)).on_scroll(move |_, _, _| {
                *counter.borrow_mut() += 1;
                Err(RuntimeError::BorrowConflict)
            }),
        )),
    ))?;
    frame(&mut ui);
    let area = widget(&ui, &root, "area");
    ui.focus(&area)?;
    let mut failed = false;
    for action in [KeyAction::Press, KeyAction::Release] {
        failed |= ui.key_input(KeyCode::ArrowDown, action).is_err();
    }
    failed |= ui.flush(ui.now()).is_err();
    assert!(failed, "the callback's error propagates");
    for _ in 0..3 {
        ui.flush(ui.now())?;
    }
    assert_eq!(*calls.borrow(), 1, "the change is not replayed");
    let offset = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == "area")
        .ok_or("no area")?
        .details
        .properties["scroll_y"]
        .clone();
    assert_eq!(offset, InspectValue::Number(40.0), "the offset stays");
    Ok(())
}

// ---- SplitPane ----

fn split(sizing: SplitSizing, log: &Log<SplitRule>) -> Element {
    let log = log.clone();
    Element::new(
        "split",
        SplitPane::new(
            SplitAxis::LeftRight,
            Element::new("a", Label::new("A")),
            Element::new("b", Label::new("B")),
        )
        .sizing(sizing)
        .on_resize(move |_, sizing, origin| {
            log.borrow_mut().push((sizing.rule(), origin));
            Ok(())
        }),
    )
}

fn first(value: f32) -> Result<SplitSizing, GeometryError> {
    Ok(SplitSizing::first(Length::new(value)?))
}

#[test]
fn a_split_reports_a_drag_only_when_it_is_released() -> TestResult {
    let mut ui = runtime(400.0, 200.0)?;
    let log = Log::default();
    ui.mount_tree(Element::new("root", View::new(split(first(150.0)?, &log))))?;
    frame(&mut ui);
    // The divider lies at 150..158.
    pointer(
        &mut ui,
        154.0,
        50.0,
        PointerKind::Press(PointerButton::Primary),
    )?;
    pointer(&mut ui, 204.0, 50.0, PointerKind::Move)?;
    frame(&mut ui);
    assert!(log.borrow().is_empty(), "a preview is not reported");
    pointer(
        &mut ui,
        204.0,
        50.0,
        PointerKind::Release(PointerButton::Primary),
    )?;
    frame(&mut ui);
    assert_eq!(
        *log.borrow(),
        [(SplitRule::First(Length::new(200.0)?), ChangeOrigin::User)]
    );

    // Escape cancels a drag and reports nothing.
    log.borrow_mut().clear();
    pointer(
        &mut ui,
        204.0,
        50.0,
        PointerKind::Press(PointerButton::Primary),
    )?;
    pointer(&mut ui, 104.0, 50.0, PointerKind::Move)?;
    key(&mut ui, KeyCode::Escape);
    pointer(
        &mut ui,
        104.0,
        50.0,
        PointerKind::Release(PointerButton::Primary),
    )?;
    frame(&mut ui);
    assert!(log.borrow().is_empty(), "{:?}", log.borrow());
    Ok(())
}

#[test]
fn a_split_reports_restore_and_a_new_described_rule() -> TestResult {
    let mut source = runtime(400.0, 200.0)?;
    let ignored = Log::default();
    source.mount_tree(Element::new(
        "root",
        View::new(split(first(150.0)?, &ignored)),
    ))?;
    frame(&mut source);
    pointer(
        &mut source,
        154.0,
        50.0,
        PointerKind::Press(PointerButton::Primary),
    )?;
    pointer(&mut source, 104.0, 50.0, PointerKind::Move)?;
    pointer(
        &mut source,
        104.0,
        50.0,
        PointerKind::Release(PointerButton::Primary),
    )?;
    frame(&mut source);
    let saved = source.capture_workspace()?;

    let mut ui = runtime(400.0, 200.0)?;
    let log = Log::default();
    let root = ui.mount_tree(Element::new("root", View::new(split(first(150.0)?, &log))))?;
    frame(&mut ui);
    let report = ui.restore_workspace(&saved)?;
    assert_eq!(report.rejected().count(), 0);
    frame(&mut ui);
    assert_eq!(
        *log.borrow(),
        [(SplitRule::First(Length::new(100.0)?), ChangeOrigin::Restore)]
    );

    // The same described rule keeps the preference and reports nothing; a
    // new one replaces it.
    log.borrow_mut().clear();
    ui.reconcile_children(&root, vec![split(first(150.0)?, &log)])?;
    frame(&mut ui);
    assert!(log.borrow().is_empty());
    ui.reconcile_children(&root, vec![split(first(250.0)?, &log)])?;
    frame(&mut ui);
    assert_eq!(
        *log.borrow(),
        [(
            SplitRule::First(Length::new(250.0)?),
            ChangeOrigin::Redescription
        )]
    );
    Ok(())
}

// ---- RegionLayout ----

fn regions(left: f32, log: &Log<(RegionEdge, SplitRule)>) -> Result<Element, GeometryError> {
    let log = log.clone();
    Ok(Element::new(
        "regions",
        RegionLayout::new()
            .left(Element::new("left", Label::new("Left")))
            .center(Element::new("center", Label::new("Center")))
            .left_width(Length::new(left)?)
            .left_resizable(true)
            .on_resize(move |_, edge, sizing, origin| {
                log.borrow_mut().push(((edge, sizing.rule()), origin));
                Ok(())
            }),
    ))
}

#[test]
fn a_region_layout_reports_its_edges() -> TestResult {
    let mut ui = runtime(400.0, 200.0)?;
    let log = Log::default();
    let root = ui.mount_tree(Element::new("root", View::new(regions(120.0, &log)?)))?;
    frame(&mut ui);
    let divider = widget(&ui, &root, "split-left");
    ui.focus(&divider)?;
    key(&mut ui, KeyCode::ArrowRight);
    frame(&mut ui);
    assert_eq!(
        *log.borrow(),
        [(
            (RegionEdge::Left, SplitRule::First(Length::new(128.0)?)),
            ChangeOrigin::User
        )]
    );
    log.borrow_mut().clear();
    ui.reconcile_children(&root, vec![regions(160.0, &log)?])?;
    frame(&mut ui);
    assert_eq!(
        *log.borrow(),
        [(
            (RegionEdge::Left, SplitRule::First(Length::new(160.0)?)),
            ChangeOrigin::Redescription
        )]
    );
    Ok(())
}

// ---- PropertyGrid ----

fn grid(log: &Log<Option<Length>>, value: Signal<String>) -> Element {
    let log = log.clone();
    Element::new(
        "grid",
        PropertyGrid::new([Property::new(
            "name",
            "Name",
            Element::new("input", TextField::new(value)),
        )])
        .label_width(Length::from(100))
        .on_resize(move |_, width, origin| {
            log.borrow_mut().push((width, origin));
            Ok(())
        }),
    )
}

#[test]
fn a_property_grid_reports_committed_widths_and_restores() -> TestResult {
    let mut ui = runtime(480.0, 200.0)?;
    let log = Log::default();
    let value = ui.signal(String::new());
    let root = ui.mount_tree(Element::new("root", View::new(grid(&log, value))))?;
    frame(&mut ui);
    let grid_id = widget(&ui, &root, "grid");
    ui.focus(&grid_id)?;
    // Held keys preview; the release commits once.
    ui.key_input(KeyCode::ArrowRight, KeyAction::Press)?;
    ui.key_input(KeyCode::ArrowRight, KeyAction::Repeat)?;
    frame(&mut ui);
    assert!(log.borrow().is_empty(), "a preview is not reported");
    ui.key_input(KeyCode::ArrowRight, KeyAction::Release)?;
    frame(&mut ui);
    assert_eq!(
        *log.borrow(),
        [(Some(Length::new(116.0)?), ChangeOrigin::User)]
    );

    // Escape cancels a resize in progress and reports nothing.
    log.borrow_mut().clear();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Press)?;
    key(&mut ui, KeyCode::Escape);
    ui.key_input(KeyCode::ArrowRight, KeyAction::Release)?;
    frame(&mut ui);
    assert!(log.borrow().is_empty(), "{:?}", log.borrow());

    // A restore reports Restore.
    let saved = ui.capture_workspace()?;
    let mut other = runtime(480.0, 200.0)?;
    let restored = Log::default();
    let value = other.signal(String::new());
    other.mount_tree(Element::new("root", View::new(grid(&restored, value))))?;
    frame(&mut other);
    let report = other.restore_workspace(&saved)?;
    assert_eq!(report.rejected().count(), 0);
    frame(&mut other);
    assert_eq!(
        *restored.borrow(),
        [(Some(Length::new(116.0)?), ChangeOrigin::Restore)]
    );
    Ok(())
}

#[test]
fn a_shared_width_written_by_the_application_is_not_reported() -> TestResult {
    let mut ui = runtime(480.0, 200.0)?;
    let log = Log::default();
    let width = ui.signal(Length::new(100.0)?);
    let value = ui.signal(String::new());
    let seen = log.clone();
    let root = ui.mount_tree(Element::new(
        "grid",
        PropertyGrid::new([Property::new(
            "name",
            "Name",
            Element::new("input", TextField::new(value)),
        )])
        .shared_label_width(width.clone())
        .on_resize(move |_, width, origin| {
            seen.borrow_mut().push((width, origin));
            Ok(())
        }),
    ))?;
    ui.flush(Duration::ZERO)?;
    width.set(Length::new(140.0)?)?;
    frame(&mut ui);
    assert!(log.borrow().is_empty());
    // Its own committed resize is reported.
    ui.focus(&root)?;
    key(&mut ui, KeyCode::ArrowLeft);
    frame(&mut ui);
    assert_eq!(
        *log.borrow(),
        [(Some(Length::new(132.0)?), ChangeOrigin::User)]
    );
    assert_eq!(width.get()?, Length::new(132.0)?);
    Ok(())
}
