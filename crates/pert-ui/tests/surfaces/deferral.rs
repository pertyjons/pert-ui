//! Deferral across surfaces and native windows: work an application callback
//! queues with `EventContext::defer` runs after its surface closes and after
//! its detached window closes or redocks, routed to its widget's window, else
//! to the window it was queued in while that window is open.
//!
//! Two kinds of test, marked on each: **new-behavior** tests cover what only
//! the rules of layering part L4a do, and name the assertion that a compiling
//! mutation restoring the old rule makes fail; **contract** tests check rules
//! the part keeps.
use pert_ui::testing::HostIntegration;
use pert_ui::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
type Seen = Rc<RefCell<Option<NativeWindowId>>>;

/// What a closing surface's callback leaves: the widget whose context it
/// received, the receipt of the operation it queued and whether that ran.
#[derive(Clone, Default)]
struct Probe {
    origin: Rc<RefCell<Option<WidgetId>>>,
    receipt: Rc<RefCell<Option<TaskReceipt>>>,
    ran: Rc<Cell<bool>>,
}
impl Probe {
    /// Records the callback's widget and queues the operation with `defer`.
    fn queue(&self, cx: &EventContext) -> Result<(), RuntimeError> {
        *self.origin.borrow_mut() = Some(cx.id());
        let ran = self.ran.clone();
        *self.receipt.borrow_mut() = Some(cx.defer(move |_| {
            ran.set(true);
            Ok(())
        })?);
        Ok(())
    }
    fn status(&self) -> Option<TaskStatus> {
        self.receipt.borrow().as_ref().map(TaskReceipt::status)
    }
}

/// Whether `id` is mounted and effectively visible.
fn shown(ui: &Runtime, id: &WidgetId) -> bool {
    ui.inspection_id(id).is_ok_and(|serial| {
        ui.inspect()
            .components
            .iter()
            .any(|component| component.id == serial && component.visible)
    })
}

fn viewport(ui: &Runtime) -> TestResult {
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0)?)?;
    Ok(())
}

/// Sends `code` from inside a flush, as a native check's driver delivers it:
/// the callback's operation is queued during delivery, so it waits for the
/// next flush, and the rest of this flush settles the surface's closure.
/// Then checks that the callback's widget is hidden or unmounted, delivers
/// the operation in a second flush and returns the probe's status.
fn choose_then_deliver(ui: &mut Runtime, code: KeyCode, probe: &Probe) -> TestResult {
    let _input = ui.defer(move |ui| {
        ui.key_input(code, KeyAction::Press)?;
        ui.key_input(code, KeyAction::Release)?;
        Ok(())
    })?;
    ui.flush(ui.now())?;
    let origin = probe.origin.borrow().clone().ok_or("the callback ran")?;
    assert!(
        !shown(ui, &origin),
        "precondition: the surface closed before delivery"
    );
    assert_eq!(probe.status(), Some(TaskStatus::Pending));
    ui.flush(ui.now())?;
    Ok(())
}

/// New-behavior test 1, `Select::on_change`. Named assertion: after the
/// second flush the receipt reads `Succeeded` and the operation ran.
/// Mutation: `EventContext::defer` records `Owner::Shown`.
#[test]
fn select_work_runs_after_its_list_closed() -> TestResult {
    let mut ui = Runtime::new();
    viewport(&ui)?;
    let value = ui.signal("a");
    let probe = Probe::default();
    let queue = probe.clone();
    let select = Select::new(
        &ui,
        value.clone(),
        [Choice::new("a", "A"), Choice::new("b", "B")],
    )?
    .on_change(move |cx: &mut EventContext, _: &&str| queue.queue(cx));
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([Element::new("control", select)]),
    ))?;
    ui.flush(ui.now())?;
    let anchor = ui.find_descendant(&root, "select")?.ok_or("anchor")?;
    ui.focus(&anchor)?;
    for code in [KeyCode::Enter, KeyCode::ArrowDown] {
        ui.key_input(code, KeyAction::Press)?;
        ui.key_input(code, KeyAction::Release)?;
        ui.flush(ui.now())?;
    }
    choose_then_deliver(&mut ui, KeyCode::Enter, &probe)?;
    assert_eq!(value.get()?, "b");
    assert_eq!(probe.status(), Some(TaskStatus::Succeeded));
    assert!(probe.ran.get());
    Ok(())
}

/// New-behavior test 1, a `QuickPick` choice answering `Done`. Named
/// assertion and mutation as `select_work_runs_after_its_list_closed`.
#[test]
fn quick_pick_work_runs_after_the_picker_closed() -> TestResult {
    let mut ui = Runtime::new();
    viewport(&ui)?;
    let picker = QuickPickController::new(&ui);
    let probe = Probe::default();
    let queue = probe.clone();
    ui.mount_tree(Element::new(
        "app",
        QuickPick::new(
            picker.clone(),
            Element::new("doc", Label::new("Document")),
            move |cx, _| {
                queue.queue(cx)?;
                Ok(PickChoice::Done)
            },
        ),
    ))?;
    picker.open(vec![PickItem::new("knobs", "Knobs")])?;
    ui.flush(ui.now())?;
    choose_then_deliver(&mut ui, KeyCode::Enter, &probe)?;
    assert_eq!(picker.status()?, PickStatus::Closed);
    assert_eq!(probe.status(), Some(TaskStatus::Succeeded));
    assert!(probe.ran.get());
    Ok(())
}

/// New-behavior test 1, a `MenuItem::on_change` row. Named assertion and
/// mutation as `select_work_runs_after_its_list_closed`.
#[test]
fn menu_row_work_runs_after_the_menu_closed() -> TestResult {
    let mut ui = Runtime::new();
    viewport(&ui)?;
    let grid = ui.signal(false);
    let probe = Probe::default();
    let queue = probe.clone();
    let menu = Menu::new(
        &ui,
        "View",
        vec![
            MenuItem::toggle("grid", grid.clone(), "Show grid")
                .on_change(move |cx, _| queue.queue(cx)),
        ],
    )?;
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([Element::new("menu", menu)]),
    ))?;
    ui.flush(ui.now())?;
    let trigger = ui.find_descendant(&root, "trigger")?.ok_or("trigger")?;
    ui.focus(&trigger)?;
    ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    ui.key_input(KeyCode::Enter, KeyAction::Release)?;
    ui.flush(ui.now())?;
    choose_then_deliver(&mut ui, KeyCode::Enter, &probe)?;
    assert!(grid.get()?);
    assert_eq!(probe.status(), Some(TaskStatus::Succeeded));
    assert!(probe.ran.get());
    Ok(())
}

/// New-behavior test 1, a modal `Dialog`'s Save button that closes its
/// dialog. Named assertion and mutation as
/// `select_work_runs_after_its_list_closed`.
#[test]
fn dialog_save_work_runs_after_the_dialog_closed() -> TestResult {
    let mut ui = Runtime::new();
    viewport(&ui)?;
    let dialog = DialogController::new(&ui);
    let probe = Probe::default();
    let (queue, closing) = (probe.clone(), dialog.clone());
    let root = ui.mount_tree(Element::new(
        "root",
        Dialog::new(
            dialog.clone(),
            Element::new("opener", Button::new("Open", |_| Ok(()))),
            Element::new(
                "save",
                Button::new("Save", move |cx| {
                    closing.accept()?;
                    queue.queue(cx)
                }),
            ),
        ),
    ))?;
    ui.flush(ui.now())?;
    dialog.open()?;
    ui.flush(ui.now())?;
    let save = ui.find_descendant(&root, "save")?.ok_or("save")?;
    ui.focus(&save)?;
    choose_then_deliver(&mut ui, KeyCode::Space, &probe)?;
    assert_eq!(
        dialog.take_outcome()?,
        Some(DialogOutcome::Accepted),
        "the dialog closed with Save"
    );
    assert_eq!(probe.status(), Some(TaskStatus::Succeeded));
    assert!(probe.ran.get());
    Ok(())
}

/// A root view in the primary window, a column holding the button `child`,
/// laid out.
fn view(ui: &mut Runtime) -> TestResult<(WidgetId, WidgetId)> {
    viewport(ui)?;
    let view = ui.mount_tree(Element::new(
        "view",
        Column::new([Element::new("child", Button::new("Go", |_| Ok(())))]),
    ))?;
    ui.flush(Duration::ZERO)?;
    let child = ui.find(&view, &["child"])?.ok_or("child is mounted")?;
    Ok((view, child))
}

/// Moves `view` into a new native window with `policy`, without flushing, so
/// an operation queued before stays queued.
fn detach(
    ui: &mut Runtime,
    view: &WidgetId,
    policy: DetachedClosePolicy,
) -> TestResult<NativeWindowId> {
    let options = NativeWindowOptions::new("Detached")?.close_policy(policy);
    let prepared = ui.prepare_detach(view, options)?;
    Ok(ui.commit_detach(prepared)?)
}

/// A second detached window holding a label, opened before [`view`] mounts
/// so the two roots never share the primary window.
fn other_window(ui: &mut Runtime) -> TestResult<NativeWindowId> {
    viewport(ui)?;
    let other = ui.mount_tree(Element::new("other", Label::new("Other")))?;
    ui.flush(Duration::ZERO)?;
    let window = detach(ui, &other, DetachedClosePolicy::Redock)?;
    ui.flush(Duration::ZERO)?;
    ui.select_window(&ui.primary_window())?;
    Ok(window)
}

/// An operation that records the window selected while it runs.
fn record(seen: &Seen) -> impl FnOnce(&mut Runtime) -> Result<(), RuntimeError> + 'static {
    let seen = seen.clone();
    move |ui| {
        *seen.borrow_mut() = Some(ui.current_window());
        Ok(())
    }
}

/// Queues `defer` from `child` in its detached window, then closes that
/// window as the native host does before it flushes, and flushes with the
/// window of `other` selected. Returns the receipt, the window the operation
/// ran in and the window the flush had selected.
fn close_before_the_flush(
    policy: DetachedClosePolicy,
    shown_only: bool,
) -> TestResult<(TaskReceipt, Option<NativeWindowId>, NativeWindowId, Runtime)> {
    let mut ui = Runtime::new();
    let flushing = other_window(&mut ui)?;
    let (view, child) = view(&mut ui)?;
    let window = detach(&mut ui, &view, policy)?;
    ui.flush(Duration::ZERO)?;
    let seen = Seen::default();
    ui.select_window(&window)?;
    let receipt = ui.event(&child, |cx| {
        if shown_only {
            cx.defer_while_shown(record(&seen))
        } else {
            cx.defer(record(&seen))
        }
    })??;
    assert_eq!(ui.close_detached_window()?, PresentationStatus::Closed);
    assert!(!ui.contains_window(&window));
    ui.select_window(&flushing)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(ui.current_window(), flushing, "the selection is restored");
    let ran_in = seen.borrow_mut().take();
    Ok((receipt, ran_in, flushing, ui))
}

/// New-behavior test 3: a detached window closed before the flush no longer
/// cancels `defer`. With `Redock`, named assertions: `Succeeded`, and the
/// operation ran in the primary window, where the widget now is. With
/// `RemoveView`: `Succeeded`, and it ran in the window the flush had
/// selected. Mutation: `defer` is cancelled when its origin window is not
/// mounted (the former `defer_detached` rule).
#[test]
fn defer_runs_after_its_detached_window_closes() -> TestResult {
    let (receipt, ran_in, _, ui) = close_before_the_flush(DetachedClosePolicy::Redock, false)?;
    assert_eq!(receipt.status(), TaskStatus::Succeeded, "Redock");
    assert_eq!(ran_in, Some(ui.primary_window()), "Redock");

    let (receipt, ran_in, flushing, _ui) =
        close_before_the_flush(DetachedClosePolicy::RemoveView, false)?;
    assert_eq!(receipt.status(), TaskStatus::Succeeded, "RemoveView");
    assert_eq!(ran_in, Some(flushing), "RemoveView");
    Ok(())
}

/// Contract test: `defer_while_shown` keeps the former `defer` rule across
/// windows. After a `Redock` close it runs in the primary window; after a
/// `RemoveView` close it is cancelled.
#[test]
fn defer_while_shown_follows_its_widget_through_a_closing_window() -> TestResult {
    let (receipt, ran_in, _, ui) = close_before_the_flush(DetachedClosePolicy::Redock, true)?;
    assert_eq!(receipt.status(), TaskStatus::Succeeded, "Redock");
    assert_eq!(ran_in, Some(ui.primary_window()), "Redock");

    let (receipt, ran_in, _, _ui) = close_before_the_flush(DetachedClosePolicy::RemoveView, true)?;
    assert_eq!(receipt.status(), TaskStatus::Cancelled, "RemoveView");
    assert_eq!(ran_in, None, "RemoveView");
    Ok(())
}

/// New-behavior test 4: once its widget is gone, `defer` runs in the window
/// it was queued in while that window is open, whether the widget's identity
/// is retained (an unmounted node) or dropped. Named assertions: the
/// operation ran in the detached window, and the primary window is selected
/// again after the flush. Mutation: routing omits the origin-window step, so
/// the operation runs with the primary window selected.
#[test]
fn defer_falls_back_to_its_origin_window() -> TestResult {
    for retain in [true, false] {
        let mut ui = Runtime::new();
        let (view, child) = view(&mut ui)?;
        let window = detach(&mut ui, &view, DetachedClosePolicy::Redock)?;
        ui.flush(Duration::ZERO)?;
        let seen = Seen::default();
        ui.select_window(&window)?;
        let receipt = ui.event(&child, |cx| cx.defer(record(&seen)))??;
        let kept = if retain {
            ui.unmount(&child)?;
            Some(child)
        } else {
            drop(child);
            ui.reconcile_children(&view, vec![Element::new("kept", Label::new("Kept"))])?;
            None
        };
        assert!(ui.contains_window(&window), "the origin window stays open");
        ui.select_window(&ui.primary_window())?;
        ui.flush(Duration::ZERO)?;
        assert_eq!(receipt.status(), TaskStatus::Succeeded, "retain: {retain}");
        assert_eq!(*seen.borrow(), Some(window), "retain: {retain}");
        assert_eq!(ui.current_window(), ui.primary_window(), "retain: {retain}");
        drop(kept);
    }
    Ok(())
}

/// New-behavior test 5: a widget detached and then hidden before the flush.
/// The former `defer` would cancel (hidden), the former `defer_detached`
/// would run in the primary window. Named assertions: `Succeeded`, and the
/// operation ran in the new window. Mutations: `Owner::Shown` (fails on
/// `Succeeded`) and routing to the origin window (fails on the window).
#[test]
fn defer_follows_its_widget_into_a_new_window_even_when_hidden() -> TestResult {
    let mut ui = Runtime::new();
    let (view, child) = view(&mut ui)?;
    let seen = Seen::default();
    let receipt = ui.event(&child, |cx| cx.defer(record(&seen)))??;
    let window = detach(&mut ui, &view, DetachedClosePolicy::Redock)?;
    ui.set_visible(&child, false)?;
    assert_eq!(receipt.status(), TaskStatus::Pending, "precondition");
    ui.select_window(&ui.primary_window())?;
    ui.flush_windows(Duration::ZERO)?;
    assert_eq!(receipt.status(), TaskStatus::Succeeded);
    assert_eq!(*seen.borrow(), Some(window));
    Ok(())
}

/// Contract test: a shown widget detached between enqueue and delivery runs
/// its `defer` and `defer_while_shown` operations in its new window, as the
/// former `defer` did. Contract mutation: `defer` routes to its origin window
/// while that is open; the `defer` window assertion fails.
#[test]
fn deferred_work_follows_its_widget_into_a_new_window() -> TestResult {
    for shown_only in [false, true] {
        let mut ui = Runtime::new();
        let (view, child) = view(&mut ui)?;
        let seen = Seen::default();
        let receipt = ui.event(&child, |cx| {
            if shown_only {
                cx.defer_while_shown(record(&seen))
            } else {
                cx.defer(record(&seen))
            }
        })??;
        let window = detach(&mut ui, &view, DetachedClosePolicy::Redock)?;
        assert_eq!(receipt.status(), TaskStatus::Pending, "precondition");
        ui.select_window(&ui.primary_window())?;
        ui.flush_windows(Duration::ZERO)?;
        assert_eq!(receipt.status(), TaskStatus::Succeeded, "{shown_only}");
        assert_eq!(*seen.borrow(), Some(window), "shown only: {shown_only}");
    }
    Ok(())
}

/// Contract test: `Runtime::defer` does not change the selection; it runs
/// with whichever window the delivering flush has selected.
#[test]
fn runtime_defer_runs_in_the_flushing_window() -> TestResult {
    let mut ui = Runtime::new();
    let (view, _) = view(&mut ui)?;
    let window = detach(&mut ui, &view, DetachedClosePolicy::Redock)?;
    ui.flush(Duration::ZERO)?;
    let seen = Seen::default();
    ui.select_window(&ui.primary_window())?;
    let receipt = ui.defer(record(&seen))?;
    ui.select_window(&window)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(receipt.status(), TaskStatus::Succeeded);
    assert_eq!(*seen.borrow(), Some(window));
    Ok(())
}
