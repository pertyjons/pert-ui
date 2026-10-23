//! Re-describing a transient surface owner under the same key applies the new
//! description to the mounted owner: dialogs and toast hosts keep their
//! controller or queue instead of failing as a second owner, and open signals,
//! visibility, accessible names, modality and fixed labels follow the new
//! description.
use crate::support;
use pert_ui::*;
use support::{frame, press, widget};

type Error = Box<dyn std::error::Error>;
type TestResult = Result<(), Error>;

fn ui() -> Result<Runtime, Error> {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
    Ok(ui)
}
/// Mounts `owner` in a column and returns the column and the owner.
fn mount(ui: &mut Runtime, owner: Element) -> Result<(WidgetId, WidgetId), Error> {
    let root = ui.mount_tree(Element::new("root", Column::new([owner])))?;
    frame(ui);
    let owner = ui.children(&root)?[0].clone();
    Ok((root, owner))
}
/// Re-describes the column's only child and checks that it kept its identity.
fn redescribe(
    ui: &mut Runtime,
    root: &WidgetId,
    owner: &WidgetId,
    next: Element,
) -> Result<(), Error> {
    ui.reconcile_children(root, vec![next])?;
    ui.flush(ui.now())?;
    assert_eq!(&ui.children(root)?[0], owner, "reconciled, not remounted");
    Ok(())
}
/// The inspection details of the only component whose address ends with
/// `suffix`. A structural component has no address of its own and is matched
/// by its key when `suffix` is a single key.
fn details(ui: &Runtime, suffix: &[&str]) -> Result<ComponentDetails, Error> {
    let mut matches: Vec<_> = ui
        .inspect()
        .components
        .into_iter()
        .filter(|c| match &c.path {
            Some(path) => {
                path.len() >= suffix.len()
                    && path[path.len() - suffix.len()..]
                        .iter()
                        .zip(suffix)
                        .all(|(a, b)| a == b)
            }
            None => suffix == [c.key.as_str()],
        })
        .collect();
    match matches.len() {
        1 => Ok(matches.remove(0).details),
        0 => Err(format!("no component at {suffix:?}").into()),
        _ => Err(format!("several components at {suffix:?}").into()),
    }
}
fn property(ui: &Runtime, suffix: &[&str], name: &str) -> Result<InspectValue, Error> {
    details(ui, suffix)?
        .properties
        .get(name)
        .cloned()
        .ok_or_else(|| format!("no property {name} at {suffix:?}").into())
}
fn name(ui: &Runtime, suffix: &[&str]) -> Result<Option<String>, Error> {
    Ok(details(ui, suffix)?.semantics.and_then(|s| s.name))
}
fn dialog(state: &DialogController, body: &str) -> Dialog {
    Dialog::new(
        state.clone(),
        Element::new("opener", Label::new("Opener")),
        Element::new("message", Label::new(body)),
    )
}

#[test]
fn an_open_dialog_can_be_described_again_with_its_controller() -> TestResult {
    let mut ui = ui()?;
    let state = DialogController::new(&ui);
    let (root, owner) = mount(&mut ui, Element::new("owner", dialog(&state, "First")))?;
    state.open()?;
    frame(&mut ui);
    let message = widget(&ui, &root, "message");

    redescribe(
        &mut ui,
        &root,
        &owner,
        Element::new("owner", dialog(&state, "Second").require_response()),
    )?;
    assert_eq!(
        state.status().get()?,
        DialogStatus::Open,
        "still one session"
    );
    assert_eq!(
        widget(&ui, &root, "message"),
        message,
        "content reconciled by key"
    );
    assert_eq!(
        property(&ui, &["message"], "text")?,
        InspectValue::Text("Second".into())
    );
    // The adopted require_response consumes Escape without closing.
    press(&mut ui, KeyCode::Escape);
    assert_eq!(state.status().get()?, DialogStatus::Open);
    state.cancel()?;
    frame(&mut ui);
    assert_eq!(state.take_outcome()?, Some(DialogOutcome::Cancelled));
    Ok(())
}

#[test]
fn a_redescribed_modality_applies_from_the_next_session() -> TestResult {
    let mut ui = ui()?;
    let state = DialogController::new(&ui);
    let (root, owner) = mount(&mut ui, Element::new("owner", dialog(&state, "Body")))?;
    state.open()?;
    frame(&mut ui);
    redescribe(
        &mut ui,
        &root,
        &owner,
        Element::new("owner", dialog(&state, "Body").view_modal()),
    )?;
    assert_eq!(
        property(&ui, &["owner"], "window_modal")?,
        InspectValue::Bool(true),
        "the open session keeps the scope it was admitted with"
    );
    state.accept()?;
    frame(&mut ui);
    state.take_outcome()?;
    state.open()?;
    frame(&mut ui);
    assert_eq!(
        property(&ui, &["owner"], "window_modal")?,
        InspectValue::Bool(false)
    );
    assert_eq!(property(&ui, &["owner"], "open")?, InspectValue::Bool(true));
    Ok(())
}

#[test]
fn a_second_mounted_dialog_owner_is_busy_until_the_first_unmounts() -> TestResult {
    let mut ui = ui()?;
    let state = DialogController::new(&ui);
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new("first", dialog(&state, "First")),
            Element::new("second", dialog(&state, "Second")),
        ]),
    ))?;
    assert_eq!(ui.flush(ui.now()), Err(RuntimeError::DialogBusy));
    state.open()?;
    ui.reconcile_children(
        &root,
        vec![Element::new("second", dialog(&state, "Second"))],
    )?;
    assert_eq!(
        state.take_outcome()?,
        Some(DialogOutcome::Dismissed(SurfaceDismissal::OwnerRemoved)),
        "removing the owner ends its session"
    );
    frame(&mut ui);
    state.open()?;
    frame(&mut ui);
    assert_eq!(
        property(&ui, &["second"], "open")?,
        InspectValue::Bool(true),
        "the remaining owner took the controller over"
    );
    Ok(())
}

#[test]
fn a_dialog_described_with_another_controller_dismisses_the_old_session() -> TestResult {
    let mut ui = ui()?;
    let old = DialogController::new(&ui);
    let new = DialogController::new(&ui);
    let (root, owner) = mount(&mut ui, Element::new("owner", dialog(&old, "Body")))?;
    old.open()?;
    frame(&mut ui);
    redescribe(
        &mut ui,
        &root,
        &owner,
        Element::new("owner", dialog(&new, "Body")),
    )?;
    assert_eq!(
        old.take_outcome()?,
        Some(DialogOutcome::Dismissed(SurfaceDismissal::OwnerRemoved))
    );
    assert_eq!(
        property(&ui, &["owner"], "open")?,
        InspectValue::Bool(false)
    );
    new.open()?;
    frame(&mut ui);
    assert_eq!(property(&ui, &["owner"], "open")?, InspectValue::Bool(true));
    Ok(())
}

#[test]
fn a_redescribed_alert_dialog_takes_its_title_and_labels() -> TestResult {
    let mut ui = ui()?;
    let state = DialogController::new(&ui);
    let alert = |title: &str, cancel: &str| {
        AlertDialog::new(
            state.clone(),
            Element::new("opener", Label::new("Opener")),
            title,
            "Proceed?",
            "Delete",
        )
        .cancel_label(cancel)
    };
    let (root, owner) = mount(&mut ui, Element::new("alert", alert("Delete?", "Cancel")))?;
    state.open()?;
    frame(&mut ui);
    redescribe(
        &mut ui,
        &root,
        &owner,
        Element::new("alert", alert("Radera?", "Avbryt")),
    )?;
    assert_eq!(name(&ui, &["alert"])?.as_deref(), Some("Radera?"));
    assert_eq!(
        property(&ui, &["title"], "text")?,
        InspectValue::Text("Radera?".into())
    );
    assert_eq!(
        property(&ui, &["cancel"], "text")?,
        InspectValue::Text("Avbryt".into())
    );
    // The relabelled button still cancels this session.
    ui.focus(&widget(&ui, &root, "cancel"))?;
    press(&mut ui, KeyCode::Enter);
    assert_eq!(state.take_outcome()?, Some(DialogOutcome::Cancelled));
    Ok(())
}

#[test]
fn a_redescribed_popover_follows_its_new_open_signal() -> TestResult {
    let mut ui = ui()?;
    let old = ui.signal(false);
    let new = ui.signal(true);
    let popover = |open: &Signal<bool>| {
        Popover::new(
            open.clone(),
            Element::new("anchor", Label::new("Anchor")),
            Element::new("content", Label::new("Content")),
        )
    };
    let (root, owner) = mount(&mut ui, Element::new("popover", popover(&old)))?;
    assert_eq!(
        property(&ui, &["popover"], "open")?,
        InspectValue::Bool(false)
    );
    redescribe(
        &mut ui,
        &root,
        &owner,
        Element::new("popover", popover(&new)),
    )?;
    frame(&mut ui);
    assert_eq!(
        property(&ui, &["popover"], "open")?,
        InspectValue::Bool(true)
    );
    // Escape dismisses through the adopted signal, not the replaced one.
    press(&mut ui, KeyCode::Escape);
    assert!(!new.get()?);
    old.set(true)?;
    frame(&mut ui);
    assert_eq!(
        property(&ui, &["popover"], "open")?,
        InspectValue::Bool(false)
    );
    Ok(())
}

#[test]
fn a_redescribed_banner_takes_its_message_and_visibility() -> TestResult {
    let mut ui = ui()?;
    let shown = ui.signal(true);
    let hidden = ui.signal(false);
    let (root, owner) = mount(
        &mut ui,
        Element::new(
            "banner",
            Banner::new("Saved", []).visible_signal(shown.read_only()),
        ),
    )?;
    assert_eq!(name(&ui, &["banner"])?.as_deref(), Some("Saved"));
    redescribe(
        &mut ui,
        &root,
        &owner,
        Element::new(
            "banner",
            Banner::new("Offline", []).visible_signal(hidden.read_only()),
        ),
    )?;
    let semantics = details(&ui, &["banner"])?
        .semantics
        .ok_or("no banner semantics")?;
    assert_eq!(semantics.name.as_deref(), Some("Offline"));
    assert_eq!(semantics.expanded, Some(false), "follows the new signal");
    // Removing the signal keeps it visible.
    redescribe(
        &mut ui,
        &root,
        &owner,
        Element::new("banner", Banner::new("Offline", [])),
    )?;
    assert_eq!(
        details(&ui, &["banner"])?
            .semantics
            .and_then(|s| s.expanded),
        Some(true)
    );
    Ok(())
}

fn host(queue: &ToastController, dismiss: &str) -> ToastHost {
    ToastHost::new(
        queue.clone(),
        Element::new("content", Label::new("Content")),
    )
    .dismiss_label(dismiss)
}

#[test]
fn a_toast_host_can_be_described_again_with_its_queue() -> TestResult {
    let mut ui = ui()?;
    let queue = ToastController::new(&ui);
    let (root, owner) = mount(&mut ui, Element::new("host", host(&queue, "Dismiss")))?;
    let receipt = queue.push(Toast::new("Saved"))?;
    frame(&mut ui);
    assert_eq!(receipt.status(), ToastStatus::Visible);
    redescribe(
        &mut ui,
        &root,
        &owner,
        Element::new("host", host(&queue, "Stäng")),
    )?;
    assert_eq!(receipt.status(), ToastStatus::Visible, "the toast survives");
    assert_eq!(
        property(&ui, &["toasts"], "dismiss_label")?,
        InspectValue::Text("Stäng".into())
    );
    let later = queue.push(Toast::new("Exported"))?;
    frame(&mut ui);
    assert_eq!(later.status(), ToastStatus::Visible, "the queue stays open");
    Ok(())
}

#[test]
fn a_second_toast_host_on_one_queue_fails_its_updates() -> TestResult {
    let mut ui = ui()?;
    let queue = ToastController::new(&ui);
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new("first", host(&queue, "Dismiss")),
            Element::new("second", host(&queue, "Dismiss")),
        ]),
    ))?;
    assert_eq!(
        ui.flush(ui.now()),
        Err(RuntimeError::DuplicateKey("toasts".into()))
    );
    let receipt = queue.push(Toast::new("Saved"))?;
    // Removing the first host closes the queue, as removal always does.
    ui.reconcile_children(&root, vec![Element::new("second", host(&queue, "Dismiss"))])?;
    assert_eq!(receipt.status(), ToastStatus::OwnerRemoved);
    assert_eq!(ui.flush(ui.now()), Err(RuntimeError::Unmounted));
    Ok(())
}

#[test]
fn a_toast_host_described_with_another_queue_closes_the_old_one() -> TestResult {
    let mut ui = ui()?;
    let old = ToastController::new(&ui);
    let new = ToastController::new(&ui);
    let (root, owner) = mount(&mut ui, Element::new("host", host(&old, "Dismiss")))?;
    let receipt = old.push(Toast::new("Saved"))?;
    frame(&mut ui);
    redescribe(
        &mut ui,
        &root,
        &owner,
        Element::new("host", host(&new, "Dismiss")),
    )?;
    assert_eq!(receipt.status(), ToastStatus::OwnerRemoved);
    assert!(matches!(
        old.push(Toast::new("Late")),
        Err(RuntimeError::Unmounted)
    ));
    let current = new.push(Toast::new("Exported"))?;
    frame(&mut ui);
    assert_eq!(current.status(), ToastStatus::Visible);
    assert_eq!(
        property(&ui, &["toasts"], "visible_count")?,
        InspectValue::Number(1.0)
    );
    Ok(())
}

#[test]
fn a_key_held_on_the_old_queue_does_not_activate_the_new_queue() -> TestResult {
    let mut ui = ui()?;
    let old = ToastController::new(&ui);
    let new = ToastController::new(&ui);
    let ran = std::rc::Rc::new(std::cell::Cell::new(false));
    let flag = ran.clone();
    old.push(Toast::new("Saved").action("Undo", |_| Ok(())))?;
    // The same message and label, and the same local ID in its own queue.
    let current = new.push(Toast::new("Saved").action("Undo", move |_| {
        flag.set(true);
        Ok(())
    }))?;
    let (root, owner) = mount(&mut ui, Element::new("host", host(&old, "Dismiss")))?;
    frame(&mut ui);
    ui.focus(&widget(&ui, &root, "toasts"))?;
    ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    redescribe(
        &mut ui,
        &root,
        &owner,
        Element::new("host", host(&new, "Dismiss")),
    )?;
    let _ = ui.key_input(KeyCode::Enter, KeyAction::Release);
    frame(&mut ui);
    assert!(!ran.get(), "the press belonged to the old queue's toast");
    assert_eq!(current.status(), ToastStatus::Visible);
    Ok(())
}

#[test]
fn a_redescribed_window_takes_its_title_and_chrome_labels() -> TestResult {
    let mut ui = ui()?;
    fn window(ui: &Runtime, title: &str) -> FloatingWindow {
        FloatingWindow::new(
            ui,
            "mixer",
            title,
            Element::new("channels", Label::new("Channels")),
        )
        .minimizable(true)
    }
    let first = window(&ui, "Mixer");
    let controller = first.controller();
    let (root, owner) = mount(&mut ui, Element::new("layer", WindowLayer::new([first])))?;
    assert_eq!(
        property(&ui, &["window-close", "button"], "text")?,
        InspectValue::Text("Close window".into())
    );
    let second = window(&ui, "Mischpult")
        .minimize_label("Minimieren oder wiederherstellen")
        .close_label("Fenster schließen");
    let detached = second.controller();
    redescribe(
        &mut ui,
        &root,
        &owner,
        Element::new("layer", WindowLayer::new([second])),
    )?;
    frame(&mut ui);
    assert_eq!(
        property(&ui, &["mixer"], "title")?,
        InspectValue::Text("Mischpult".into())
    );
    assert_eq!(
        property(&ui, &["window-minimize", "button"], "text")?,
        InspectValue::Text("Minimieren oder wiederherstellen".into())
    );
    assert_eq!(
        property(&ui, &["window-close", "button"], "text")?,
        InspectValue::Text("Fenster schließen".into())
    );
    assert_eq!(
        detached.status().get()?,
        WindowStatus::Closed,
        "the new description's own controller is never attached"
    );
    // The relabelled button acts on the controller the window kept.
    let minimize = ui
        .find_descendant(&widget(&ui, &root, "window-minimize"), "button")?
        .ok_or("no minimize button")?;
    ui.focus(&minimize)?;
    press(&mut ui, KeyCode::Enter);
    assert_eq!(controller.status().get()?, WindowStatus::Minimized);
    Ok(())
}
