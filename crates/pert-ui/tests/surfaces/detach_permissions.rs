//! Detach permissions through the public facade: `Element::with_detachable`,
//! `Tab::detachable` and `Tabs::detachable` take a `Permission`, the runtime
//! reads its current value when a detach is asked for, the strip's drag
//! affordance follows a binding, revocation during a drag-out forbids the
//! drop, and an already detached view keeps its window.
use pert_ui::testing::HostIntegration;
use pert_ui::*;
use std::time::Duration;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn flush(ui: &mut Runtime) -> TestResult {
    ui.flush(ui.now())?;
    Ok(())
}

fn strip(tab: Permission, strip: Permission) -> Tabs {
    Tabs::new([
        Tab::new(
            "first",
            "First",
            Element::new("a", Label::new("First content")),
        )
        .detachable(tab),
        Tab::new(
            "second",
            "Second",
            Element::new("b", Label::new("Second content")),
        ),
    ])
    .detachable(strip)
}

fn mount(ui: &mut Runtime, tabs: Tabs) -> TestResult<WidgetId> {
    ui.set_viewport(Viewport::new(800.0, 500.0, 1.0)?)?;
    let root = ui.mount_tree(Element::new("tabs", tabs))?;
    flush(ui)?;
    Ok(root)
}

/// Whether the strip currently offers the first tab's drag-out, by its
/// Move cursor over the first title.
fn offers_drag(ui: &mut Runtime) -> TestResult<bool> {
    ui.pointer_input(PointerEvent::new(
        Point::new(20.0, 15.0)?,
        PointerKind::Move,
    ))?;
    Ok(ui.cursor_icon() == CursorIcon::Move)
}

/// Presses the first title, drags it below the strip and returns whether
/// releasing there asked for a detach, after `between` ran mid-drag.
fn drag_out(
    ui: &mut Runtime,
    between: impl FnOnce(&mut Runtime) -> TestResult,
) -> TestResult<bool> {
    ui.pointer_input(PointerEvent::new(
        Point::new(20.0, 15.0)?,
        PointerKind::Press(PointerButton::Primary),
    ))?;
    ui.pointer_input(PointerEvent::new(
        Point::new(40.0, 110.0)?,
        PointerKind::Move,
    ))?;
    between(ui)?;
    ui.pointer_input(PointerEvent::new(
        Point::new(40.0, 110.0)?,
        PointerKind::Release(PointerButton::Primary),
    ))?;
    flush(ui)?;
    let requests = ui.take_presentation_requests()?;
    Ok(requests
        .iter()
        .any(|request| matches!(request.action(), PresentationAction::Detach { .. })))
}

#[test]
fn a_revoked_tab_permission_refuses_every_detach_path_until_granted_again() -> TestResult {
    let mut ui = Runtime::new();
    let allowed = ui.signal(true);
    let root = mount(
        &mut ui,
        strip(allowed.read_only().into(), Permission::allowed()),
    )?;
    let view = ui.children(&root)?[0].clone();
    assert!(offers_drag(&mut ui)?);

    allowed.set(false)?;
    flush(&mut ui)?;
    assert!(!offers_drag(&mut ui)?, "the affordance follows the binding");
    assert_eq!(
        ui.prepare_detach(&view, NativeWindowOptions::new("First")?)
            .err(),
        Some(RuntimeError::NotDetachable)
    );
    assert_eq!(
        ui.request_detach(&view, NativeWindowOptions::new("First")?)
            .err(),
        Some(RuntimeError::NotDetachable)
    );
    assert!(!drag_out(&mut ui, |_| Ok(()))?, "no drag-out is offered");

    allowed.set(true)?;
    flush(&mut ui)?;
    assert!(offers_drag(&mut ui)?);
    let prepared = ui.prepare_detach(&view, NativeWindowOptions::new("First")?)?;
    let window = ui.commit_detach(prepared)?;
    assert_eq!(ui.window_of(&view)?, window);
    Ok(())
}

#[test]
fn revocation_during_a_drag_out_forbids_the_drop() -> TestResult {
    let mut ui = Runtime::new();
    let allowed = ui.signal(true);
    mount(
        &mut ui,
        strip(Permission::allowed(), allowed.read_only().into()),
    )?;
    let detached = drag_out(&mut ui, |ui| {
        allowed.set(false)?;
        flush(ui)
    })?;
    assert!(!detached, "the release detaches nothing");
    // Granted again, the same gesture detaches.
    allowed.set(true)?;
    flush(&mut ui)?;
    assert!(drag_out(&mut ui, |_| Ok(()))?);
    Ok(())
}

#[test]
fn an_already_detached_view_keeps_its_window_and_redocks() -> TestResult {
    let mut ui = Runtime::new();
    let allowed = ui.signal(true);
    let root = mount(
        &mut ui,
        strip(allowed.read_only().into(), Permission::allowed()),
    )?;
    let view = ui.children(&root)?[0].clone();
    let prepared = ui.prepare_detach(&view, NativeWindowOptions::new("First")?)?;
    let window = ui.commit_detach(prepared)?;
    allowed.set(false)?;
    ui.flush_windows(Duration::ZERO)?;
    assert_eq!(ui.window_of(&view)?, window, "the window stays");
    ui.redock(&window)?;
    assert_eq!(ui.window_of(&view)?, ui.primary_window());
    Ok(())
}

#[test]
fn strip_and_tab_permissions_combine_and_redescription_replaces() -> TestResult {
    let mut ui = Runtime::new();
    let strip_allowed = ui.signal(true);
    let tab_allowed = ui.signal(true);
    let root = mount(
        &mut ui,
        strip(
            tab_allowed.read_only().into(),
            strip_allowed.read_only().into(),
        ),
    )?;
    assert!(offers_drag(&mut ui)?);
    strip_allowed.set(false)?;
    flush(&mut ui)?;
    assert!(!offers_drag(&mut ui)?, "the strip denies");
    strip_allowed.set(true)?;
    tab_allowed.set(false)?;
    flush(&mut ui)?;
    assert!(!offers_drag(&mut ui)?, "the tab denies");

    // A fixed denial combined with a binding cannot be lifted by it.
    let combined = Tab::new("x", "X", Element::new("x", Label::new("x")))
        .detachable(false)
        .detachable(tab_allowed.read_only())
        .into_element();
    tab_allowed.set(true)?;
    assert!(!combined.detachable());
    let _ = root;
    Ok(())
}

#[test]
fn a_redescribed_strip_takes_the_new_permissions() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 500.0, 1.0)?)?;
    let allowed = ui.signal(false);
    let column = ui.mount_tree(Element::new(
        "column",
        Column::new([Element::new(
            "tabs",
            strip(allowed.read_only().into(), Permission::allowed()),
        )]),
    ))?;
    flush(&mut ui)?;
    let tabs = ui.children(&column)?[0].clone();
    assert!(!offers_drag(&mut ui)?);
    // A new description with a fixed allowance replaces the bound denial.
    ui.reconcile_children(
        &column,
        vec![Element::new(
            "tabs",
            strip(Permission::allowed(), Permission::allowed()),
        )],
    )?;
    flush(&mut ui)?;
    assert!(offers_drag(&mut ui)?);
    let view = ui.children(&tabs)?[0].clone();
    assert!(
        ui.prepare_detach(&view, NativeWindowOptions::new("First")?)
            .is_ok()
    );
    Ok(())
}

#[test]
fn a_foreign_binding_fails_the_strip_update_and_the_request() -> TestResult {
    let mut ui = Runtime::new();
    let elsewhere = Runtime::new();
    let foreign = elsewhere.signal(true);
    ui.set_viewport(Viewport::new(800.0, 500.0, 1.0)?)?;
    let root = ui.mount_tree(Element::new(
        "tabs",
        strip(foreign.read_only().into(), Permission::allowed()),
    ))?;
    assert_eq!(ui.flush(ui.now()), Err(RuntimeError::ForeignRuntime));
    let view = ui.children(&root)?[0].clone();
    assert_eq!(
        ui.prepare_detach(&view, NativeWindowOptions::new("First")?)
            .err(),
        Some(RuntimeError::ForeignRuntime)
    );
    Ok(())
}
