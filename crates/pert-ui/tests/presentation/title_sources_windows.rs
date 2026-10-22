//! `TextSource` titles and messages of windows, dialogs and notifications
//! through the public facade: a binding is followed without redescription,
//! the same binding keeps its text across redescription, fixed and bound text
//! replace each other, and a blank bound window title keeps the last title.
use pert_ui::*;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn runtime() -> Result<Runtime, Box<dyn std::error::Error>> {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0)?)?;
    Ok(ui)
}

fn flush(ui: &mut Runtime) {
    ui.flush(Duration::ZERO).unwrap();
}

fn component(ui: &Runtime, key: &str) -> ComponentInspection {
    ui.inspect()
        .components
        .into_iter()
        .find(|c| c.key == key)
        .unwrap_or_else(|| panic!("no component {key}"))
}

fn name(ui: &Runtime, key: &str) -> Option<String> {
    component(ui, key).details.semantics.and_then(|s| s.name)
}

fn text(ui: &Runtime, key: &str, property: &str) -> String {
    match &component(ui, key).details.properties[property] {
        InspectValue::Text(text) => text.clone(),
        other => panic!("unexpected {property} {other:?}"),
    }
}

/// The texts of the title-bar buttons, in tree order.
fn button_texts(ui: &Runtime) -> Vec<String> {
    ui.inspect()
        .components
        .into_iter()
        .filter(|c| c.key == "button" && c.details.properties.contains_key("enabled"))
        .filter_map(|c| match c.details.properties.get("text") {
            Some(InspectValue::Text(text)) => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn window(ui: &Runtime, title: impl Into<TextSource>) -> Element {
    Element::new(
        "layer",
        WindowLayer::new([FloatingWindow::new(
            ui,
            "mixer",
            title,
            Element::new("body", Label::new("Channels")),
        )]),
    )
}

#[test]
fn a_floating_window_title_follows_its_binding() -> TestResult {
    let mut ui = runtime()?;
    let title = ui.signal("Mixer".to_owned());
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([window(&ui, title.read_only())]),
    ))?;
    flush(&mut ui);
    assert_eq!(text(&ui, "mixer", "title"), "Mixer");
    assert_eq!(name(&ui, "mixer").as_deref(), Some("Mixer"));
    title.set("Mischpult".to_owned())?;
    flush(&mut ui);
    assert_eq!(text(&ui, "mixer", "title"), "Mischpult");
    assert_eq!(text(&ui, "label", "text"), "Mischpult", "the bar draws it");

    // The same binding keeps the read title across redescription.
    ui.reconcile_children(&root, vec![window(&ui, title.read_only())])?;
    assert_eq!(text(&ui, "mixer", "title"), "Mischpult");
    flush(&mut ui);

    // Fixed text replaces the binding, and a binding replaces fixed text.
    ui.reconcile_children(&root, vec![window(&ui, "Fixed")])?;
    flush(&mut ui);
    title.set("Ignored".to_owned())?;
    flush(&mut ui);
    assert_eq!(text(&ui, "mixer", "title"), "Fixed");
    assert_eq!(text(&ui, "label", "text"), "Fixed");
    ui.reconcile_children(&root, vec![window(&ui, title.read_only())])?;
    flush(&mut ui);
    assert_eq!(text(&ui, "mixer", "title"), "Ignored");
    assert_eq!(text(&ui, "label", "text"), "Ignored");
    Ok(())
}

#[test]
fn a_blank_bound_window_title_keeps_the_last_title_and_fails() -> TestResult {
    let mut ui = runtime()?;
    let title = ui.signal("Mixer".to_owned());
    ui.mount_tree(Element::new(
        "root",
        Column::new([window(&ui, title.read_only())]),
    ))?;
    flush(&mut ui);
    title.set("  ".to_owned())?;
    assert_eq!(
        ui.flush(Duration::ZERO),
        Err(RuntimeError::EmptyWindowTitle)
    );
    assert_eq!(text(&ui, "mixer", "title"), "Mixer");
    assert_eq!(text(&ui, "label", "text"), "Mixer");
    title.set("Mixer 2".to_owned())?;
    flush(&mut ui);
    assert_eq!(text(&ui, "mixer", "title"), "Mixer 2");
    Ok(())
}

#[test]
fn a_blank_fixed_window_title_is_still_refused_at_description() -> TestResult {
    let mut ui = runtime()?;
    assert_eq!(
        ui.mount_tree(window(&ui, " ")).err(),
        Some(RuntimeError::EmptyWindowTitle)
    );
    Ok(())
}

#[test]
fn window_button_labels_follow_their_bindings() -> TestResult {
    let mut ui = runtime()?;
    let close = ui.signal("Close".to_owned());
    let minimize = ui.signal("Minimize".to_owned());
    ui.mount_tree(Element::new(
        "layer",
        WindowLayer::new([FloatingWindow::new(
            &ui,
            "mixer",
            "Mixer",
            Element::new("body", Label::new("Channels")),
        )
        .minimizable(true)
        .minimize_label(minimize.read_only())
        .close_label(close.read_only())]),
    ))?;
    flush(&mut ui);
    assert_eq!(button_texts(&ui), ["Minimize", "Close"]);
    close.set("Schließen".to_owned())?;
    minimize.set("Minimieren".to_owned())?;
    flush(&mut ui);
    assert_eq!(button_texts(&ui), ["Minimieren", "Schließen"]);
    Ok(())
}

#[test]
fn an_alert_dialog_follows_its_bound_texts() -> TestResult {
    let mut ui = runtime()?;
    let state = DialogController::new(&ui);
    let title = ui.signal("Delete the track?".to_owned());
    let message = ui.signal("This cannot be undone.".to_owned());
    let accept = ui.signal("Delete".to_owned());
    let cancel = ui.signal("Keep".to_owned());
    let describe = || {
        Element::new(
            "prompt",
            AlertDialog::new(
                state.clone(),
                Element::new("open", Button::new("Delete…", |_| Ok(()))),
                title.read_only(),
                message.read_only(),
                accept.read_only(),
            )
            .cancel_label(cancel.read_only()),
        )
    };
    let root = ui.mount_tree(Element::new("root", Column::new([describe()])))?;
    flush(&mut ui);
    state.open()?;
    flush(&mut ui);
    assert_eq!(name(&ui, "prompt").as_deref(), Some("Delete the track?"));
    title.set("Spur löschen?".to_owned())?;
    message.set("Das lässt sich nicht rückgängig machen.".to_owned())?;
    accept.set("Löschen".to_owned())?;
    cancel.set("Behalten".to_owned())?;
    flush(&mut ui);
    assert_eq!(name(&ui, "prompt").as_deref(), Some("Spur löschen?"));
    assert_eq!(text(&ui, "title", "text"), "Spur löschen?");
    assert_eq!(
        text(&ui, "message", "text"),
        "Das lässt sich nicht rückgängig machen."
    );
    assert_eq!(text(&ui, "accept", "text"), "Löschen");
    assert_eq!(text(&ui, "cancel", "text"), "Behalten");

    // Describing it again with the same bindings keeps the read texts.
    ui.reconcile_children(&root, vec![describe()])?;
    assert_eq!(name(&ui, "prompt").as_deref(), Some("Spur löschen?"));
    flush(&mut ui);
    assert_eq!(text(&ui, "title", "text"), "Spur löschen?");
    Ok(())
}

fn tool_layout(title: impl Into<TextSource>) -> ToolWindowLayout {
    ToolWindowLayout::new(Element::new("main", Button::new("Main", |_| Ok(()))))
        .edge(ToolEdge::Left, EdgeMode::Stripe)
        .window(
            ToolWindow::new(
                "files",
                title,
                Element::new("files-body", Button::new("Body", |_| Ok(()))),
            )
            .expanded(true),
        )
}

fn stripe_title(ui: &Runtime) -> String {
    name_items(ui, "stripe-left")
}

fn name_items(ui: &Runtime, key: &str) -> String {
    component(ui, key)
        .details
        .semantics
        .unwrap()
        .items
        .into_iter()
        .filter_map(|item| item.name)
        .collect::<Vec<_>>()
        .join(",")
}

#[test]
fn a_tool_window_title_follows_its_binding_in_every_part() -> TestResult {
    let mut ui = runtime()?;
    let title = ui.signal("Files".to_owned());
    let root = ui.mount_tree(Element::new("frame", tool_layout(title.read_only())))?;
    flush(&mut ui);
    assert_eq!(stripe_title(&ui), "Files");
    assert_eq!(name(&ui, "files").as_deref(), Some("Files"));
    title.set("Dateien".to_owned())?;
    flush(&mut ui);
    assert_eq!(stripe_title(&ui), "Dateien");
    assert_eq!(name(&ui, "files").as_deref(), Some("Dateien"));
    let hide = ui.find_descendant(&root, "hide-button")?.unwrap();
    let hide_key = ui.key(&hide)?.to_owned();
    let hide_name = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == hide_key)
        .and_then(|c| c.details.semantics)
        .and_then(|s| s.name)
        .unwrap();
    assert!(hide_name.contains("Dateien"), "{hide_name}");
    Ok(())
}

#[test]
fn a_redescribed_tool_window_keeps_or_replaces_its_title() -> TestResult {
    let mut ui = runtime()?;
    let title = ui.signal("Files".to_owned());
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([Element::new("frame", tool_layout(title.read_only()))]),
    ))?;
    flush(&mut ui);
    title.set("Dateien".to_owned())?;
    flush(&mut ui);
    ui.reconcile_children(
        &root,
        vec![Element::new("frame", tool_layout(title.read_only()))],
    )?;
    flush(&mut ui);
    assert_eq!(stripe_title(&ui), "Dateien");
    assert_eq!(name(&ui, "files").as_deref(), Some("Dateien"));
    ui.reconcile_children(&root, vec![Element::new("frame", tool_layout("Fixed"))])?;
    flush(&mut ui);
    title.set("Ignored".to_owned())?;
    flush(&mut ui);
    assert_eq!(stripe_title(&ui), "Fixed");
    assert_eq!(name(&ui, "files").as_deref(), Some("Fixed"));
    Ok(())
}

#[test]
fn a_banner_message_follows_its_binding() -> TestResult {
    let mut ui = runtime()?;
    let message = ui.signal("Saved".to_owned());
    ui.mount_tree(Element::new(
        "root",
        Column::new([Element::new("banner", Banner::new(message.read_only(), []))]),
    ))?;
    flush(&mut ui);
    assert_eq!(name(&ui, "banner").as_deref(), Some("Saved"));
    message.set("Exported".to_owned())?;
    flush(&mut ui);
    assert_eq!(name(&ui, "banner").as_deref(), Some("Exported"));
    assert_eq!(text(&ui, "message", "text"), "Exported");
    Ok(())
}

fn toast_host(ui: &mut Runtime, queue: &ToastController) -> Result<WidgetId, RuntimeError> {
    let root = ui.mount_tree(Element::new(
        "host",
        ToastHost::new(
            queue.clone(),
            Element::new("content", Label::new("Content")),
        ),
    ))?;
    ui.flush(Duration::ZERO)?;
    Ok(root)
}

#[test]
fn a_bound_toast_shows_its_current_text_like_a_fixed_toast() -> TestResult {
    let mut bound = runtime()?;
    let queue = ToastController::new(&bound);
    let message = bound.signal("Exporting 10%".to_owned());
    let label = bound.signal("Stop".to_owned());
    let receipt =
        queue.push(Toast::new(message.read_only()).action(label.read_only(), |_| Ok(())))?;
    toast_host(&mut bound, &queue)?;
    message.set("Exporting 90%".to_owned())?;
    label.set("Halt".to_owned())?;
    bound.flush(Duration::ZERO)?;
    assert_eq!(
        receipt.status(),
        ToastStatus::Visible,
        "a text change keeps the toast"
    );

    let mut fixed = runtime()?;
    let reference = ToastController::new(&fixed);
    let _receipt = reference.push(Toast::new("Exporting 90%").action("Halt", |_| Ok(())))?;
    toast_host(&mut fixed, &reference)?;
    assert_eq!(bound.paint()?, fixed.paint()?);
    Ok(())
}

#[test]
fn toasts_with_one_bound_message_stay_separate_toasts() -> TestResult {
    let mut ui = runtime()?;
    let queue = ToastController::new(&ui);
    let message = ui.signal("Saved".to_owned());
    let first = queue.push(Toast::new(message.read_only()))?;
    let second = queue.push(Toast::new(message.read_only()))?;
    assert_ne!(first, second);
    toast_host(&mut ui, &queue)?;
    assert_eq!(
        component(&ui, "toasts").details.properties["visible_count"],
        InspectValue::Number(2.0)
    );
    Ok(())
}

#[test]
fn an_oversized_bound_toast_message_fails_the_host_until_it_fits() -> TestResult {
    let mut ui = runtime()?;
    let queue = ToastController::new(&ui);
    let message = ui.signal("Saved".to_owned());
    let receipt = queue.push(Toast::new(message.read_only()))?;
    toast_host(&mut ui, &queue)?;
    message.set("x".repeat(TextEditError::MAX_TEXT_BYTES + 1))?;
    assert_eq!(
        ui.flush(Duration::ZERO),
        Err(RuntimeError::TextEdit(TextEditError::TooLarge))
    );
    assert_eq!(receipt.status(), ToastStatus::Visible);
    message.set("Exported".to_owned())?;
    ui.flush(Duration::ZERO)?;
    let mut fixed = runtime()?;
    let reference = ToastController::new(&fixed);
    let _receipt = reference.push(Toast::new("Exported"))?;
    toast_host(&mut fixed, &reference)?;
    assert_eq!(ui.paint()?, fixed.paint()?);
    Ok(())
}

#[test]
fn a_toast_text_change_keeps_an_armed_activation() -> TestResult {
    let mut ui = runtime()?;
    let queue = ToastController::new(&ui);
    let message = ui.signal("Exporting 10%".to_owned());
    let stopped = std::rc::Rc::new(std::cell::Cell::new(false));
    let flag = stopped.clone();
    let receipt = queue.push(Toast::new(message.read_only()).action("Stop", move |_| {
        flag.set(true);
        Ok(())
    }))?;
    let host = toast_host(&mut ui, &queue)?;
    ui.focus(&ui.find_descendant(&host, "toasts")?.ok_or("no toasts")?)?;
    ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    message.set("Exporting 50%".to_owned())?;
    ui.flush(Duration::ZERO)?;
    ui.key_input(KeyCode::Enter, KeyAction::Release)?;
    ui.flush(Duration::ZERO)?;
    assert!(stopped.get(), "the press armed before the change completed");
    assert_eq!(receipt.status(), ToastStatus::ActionSucceeded);
    Ok(())
}
