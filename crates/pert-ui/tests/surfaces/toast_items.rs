//! A toast stack in inspection: one semantic item per visible toast, with its
//! name, actions, selection and window bounds, and keyboard activation as the
//! `ToastHost` page documents it.
use pert_ui::testing::Driver;
use pert_ui::*;
use std::{cell::Cell, rc::Rc, time::Duration};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A host around a label, so Tab's only stop is the stack.
fn host(ui: &mut Runtime, queue: &ToastController) -> Result<WidgetId, RuntimeError> {
    ui.set_viewport(Viewport::new(640.0, 480.0, 1.0)?)?;
    ui.mount_tree(Element::new(
        "host",
        ToastHost::new(queue.clone(), Element::new("body", Label::new("Content"))),
    ))
}

fn names(items: &[SemanticItem]) -> Vec<Option<&str>> {
    items.iter().map(|item| item.name.as_deref()).collect()
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

#[test]
fn visible_toasts_are_items_with_names_actions_order_and_bounds() -> TestResult {
    let mut ui = Runtime::new();
    let queue = ToastController::new(&ui);
    let root = host(&mut ui, &queue)?;
    queue.push(Toast::new("Saved"))?;
    queue.push(Toast::new("Upload failed").action("Retry", |_| Ok(())))?;
    let gap = {
        let theme = ui.theme();
        theme
            .metrics(theme.density()?)?
            .metric(MetricRole::GroupPadding)
    };
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let stack = driver.find(&root, &["toasts"])?;
    let semantics = driver.semantics(&stack)?;
    let items = &semantics.items;

    assert_eq!(names(items), [Some("Saved"), Some("Upload failed")]);
    assert_eq!(items[0].actions, [SemanticAction::Dismiss]);
    assert_eq!(
        items[1].actions,
        [SemanticAction::Activate, SemanticAction::Dismiss]
    );
    assert_eq!(
        items.iter().map(|item| item.selected).collect::<Vec<_>>(),
        [true, false],
        "the first toast is the one the keys act on"
    );

    // The toasts fill the stack's width and stack top to bottom, one
    // GroupPadding apart, the last ending at the stack's bottom.
    let [sx, sy, sw, sh] = driver.runtime().bounds(&stack)?.ok_or("laid out")?.values();
    let (Some(first), Some(second)) = (items[0].bounds, items[1].bounds) else {
        panic!("both toasts have bounds: {items:?}");
    };
    for [x, _, w, h] in [first, second] {
        assert!(
            close(x, sx) && close(w, sw) && h > 0.0,
            "{first:?} {second:?} in {sx} {sw}"
        );
    }
    assert!(
        close(first[1], sy),
        "{first:?} starts at the stack's top {sy}"
    );
    assert!(
        close(second[1], first[1] + first[3] + gap),
        "{second:?} follows {first:?} one gap of {gap} below"
    );
    assert!(
        close(second[1] + second[3], sy + sh),
        "{second:?} ends at {}",
        sy + sh
    );

    let labels = InspectValue::List(vec![
        InspectValue::Text(String::new()),
        InspectValue::Text("Retry".into()),
    ]);
    let stack_record = driver
        .runtime()
        .inspect()
        .components
        .into_iter()
        .find(|component| component.key == "toasts")
        .ok_or("the stack is inspected")?;
    assert_eq!(
        stack_record.details.properties.get("action_labels"),
        Some(&labels)
    );
    Ok(())
}

#[test]
fn a_bound_message_change_shows_in_the_next_inspection() -> TestResult {
    let mut ui = Runtime::new();
    let queue = ToastController::new(&ui);
    let root = host(&mut ui, &queue)?;
    let message = ui.signal(String::from("Uploading 10%"));
    queue.push(Toast::new(message.read_only()))?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let stack = driver.find(&root, &["toasts"])?;
    let before = driver.semantics(&stack)?.items;
    assert_eq!(names(&before), [Some("Uploading 10%")]);

    message.set("Uploading 90%, a message long enough to wrap onto a second line".into())?;
    driver.settle()?;
    let after = driver.semantics(&stack)?.items;
    assert_eq!(
        names(&after),
        [Some(
            "Uploading 90%, a message long enough to wrap onto a second line"
        )]
    );
    let height = |items: &[SemanticItem]| items[0].bounds.map(|[.., h]| h);
    assert!(
        height(&after) > height(&before),
        "the wrapped message's toast is taller: {before:?} {after:?}"
    );
    Ok(())
}

#[test]
fn an_expired_toast_leaves_the_items() -> TestResult {
    let mut ui = Runtime::new();
    let queue = ToastController::new(&ui);
    let root = host(&mut ui, &queue)?;
    let first = queue.push(Toast::new("First"))?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    driver.advance(Duration::from_secs(2))?;
    let second = queue.push(Toast::new("Second"))?;
    driver.settle()?;
    let stack = driver.find(&root, &["toasts"])?;
    assert_eq!(
        names(&driver.semantics(&stack)?.items),
        [Some("First"), Some("Second")]
    );

    driver.advance(Duration::from_secs(3))?;
    assert_eq!(
        (first.status(), second.status()),
        (ToastStatus::Expired, ToastStatus::Visible)
    );
    let items = driver.semantics(&stack)?.items;
    assert_eq!(names(&items), [Some("Second")]);
    assert!(items[0].selected, "the surviving toast is the selected one");
    let [_, sy, ..] = driver.runtime().bounds(&stack)?.ok_or("laid out")?.values();
    let Some([_, y, ..]) = items[0].bounds else {
        panic!("the surviving toast has bounds: {items:?}");
    };
    assert!(
        close(y, sy),
        "the surviving toast moved to the stack's top: {y} {sy}"
    );
    Ok(())
}

#[test]
fn the_stack_keys_activate_and_dismiss_the_selected_item() -> TestResult {
    let mut ui = Runtime::new();
    let queue = ToastController::new(&ui);
    let root = host(&mut ui, &queue)?;
    let retries = Rc::new(Cell::new(0));
    let retry = retries.clone();
    let saved = queue.push(Toast::new("Saved"))?;
    let failed = queue.push(Toast::new("Upload failed").action("Retry", move |_| {
        retry.set(retry.get() + 1);
        Ok(())
    }))?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let stack = driver.find(&root, &["toasts"])?;

    driver.key(KeyCode::Tab)?;
    assert_eq!(driver.runtime().focused(), Some(stack.clone()));
    driver.key(KeyCode::ArrowDown)?;
    let items = driver.semantics(&stack)?.items;
    assert_eq!(
        items.iter().map(|item| item.selected).collect::<Vec<_>>(),
        [false, true],
        "Down selects the toast with the action"
    );
    driver.key(KeyCode::Enter)?;
    assert_eq!(
        (retries.get(), failed.status()),
        (1, ToastStatus::ActionSucceeded)
    );
    let items = driver.semantics(&stack)?.items;
    assert_eq!(names(&items), [Some("Saved")]);
    assert!(items[0].selected);

    driver.key(KeyCode::Delete)?;
    assert_eq!(saved.status(), ToastStatus::Dismissed);
    assert!(driver.semantics(&stack)?.items.is_empty());
    Ok(())
}
