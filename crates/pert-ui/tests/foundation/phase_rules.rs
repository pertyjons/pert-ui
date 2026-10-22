//! Which `Runtime` operations a callback with runtime access may call:
//! deferred tasks and command actions, through the public facade.
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};

fn ui() -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0).unwrap())
        .unwrap();
    ui
}

fn mount(ui: &mut Runtime) -> (WidgetId, WidgetId) {
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([Element::new(
                "tabs",
                Tabs::new([
                    Tab::new("a", "A", Element::new("a-body", Label::new("A"))),
                    Tab::new(
                        "b",
                        "B",
                        Element::new("b-body", Button::new("Inside B", |_| Ok(()))),
                    ),
                ]),
            )]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let tabs = ui.find_descendant(&root, "tabs").unwrap().unwrap();
    let target = ui.find_descendant(&root, "b-body").unwrap().unwrap();
    (tabs, target)
}

type Log = Rc<RefCell<Vec<String>>>;

#[test]
fn a_deferred_task_may_reveal_focus_and_flush() {
    let mut ui = ui();
    let (tabs, target) = mount(&mut ui);
    let log = Log::default();
    let (l, t, g) = (log.clone(), tabs.clone(), target.clone());
    let receipt = ui
        .defer(move |ui| {
            l.borrow_mut()
                .push(format!("{:?}", ui.reveal_child(&t, "b")?));
            ui.flush(ui.now())?;
            l.borrow_mut()
                .push(format!("{:?}", ui.reveal_and_focus(&g)?));
            ui.focus(&g)?;
            Ok(())
        })
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(receipt.status(), TaskStatus::Succeeded);
    assert_eq!(
        *log.borrow(),
        ["Changed", "RevealedAndFocused { activation: None }"]
    );
    assert_eq!(ui.focused(), Some(target));
}

#[test]
fn a_flush_nested_in_a_task_leaves_newly_queued_tasks_for_the_next_flush() {
    // A task that queues another and then settles the tree (an explicit
    // flush, or a reveal, which flushes) must not run the new task inside
    // itself: `defer` promises that tasks queued during delivery wait.
    for settle in ["flush", "reveal"] {
        let mut ui = ui();
        let (tabs, _) = mount(&mut ui);
        let log = Log::default();
        let (outer, t) = (log.clone(), tabs.clone());
        let inner_receipt = Rc::new(RefCell::new(None));
        let slot = inner_receipt.clone();
        let receipt = ui
            .defer(move |ui| {
                let inner = outer.clone();
                *slot.borrow_mut() = Some(ui.defer(move |_| {
                    inner.borrow_mut().push("inner".into());
                    Ok(())
                })?);
                if settle == "flush" {
                    ui.flush(ui.now())?;
                } else {
                    ui.reveal_child(&t, "b")?;
                }
                outer.borrow_mut().push("outer done".into());
                Ok(())
            })
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        assert_eq!(receipt.status(), TaskStatus::Succeeded, "{settle}");
        assert_eq!(*log.borrow(), ["outer done"], "{settle}");
        let inner = inner_receipt.borrow_mut().take().unwrap();
        assert_eq!(inner.status(), TaskStatus::Pending, "{settle}");
        ui.flush(Duration::ZERO).unwrap();
        assert_eq!(inner.status(), TaskStatus::Succeeded, "{settle}");
        assert_eq!(*log.borrow(), ["outer done", "inner"], "{settle}");
    }
}

#[test]
fn tasks_queued_before_a_nested_flush_still_run_in_order() {
    let mut ui = ui();
    mount(&mut ui);
    let log = Log::default();
    let first = log.clone();
    let one = ui
        .defer(move |ui| {
            first.borrow_mut().push("first".into());
            ui.flush(ui.now()).map(|_| ())
        })
        .unwrap();
    let second = log.clone();
    let two = ui
        .defer(move |_| {
            second.borrow_mut().push("second".into());
            Ok(())
        })
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(*log.borrow(), ["first", "second"]);
    assert_eq!(one.status(), TaskStatus::Succeeded);
    assert_eq!(two.status(), TaskStatus::Succeeded);
}

#[test]
fn a_command_action_may_reveal_and_flush() {
    let mut ui = ui();
    let (tabs, target) = mount(&mut ui);
    let t = tabs.clone();
    let g = target.clone();
    let command = ui
        .register_command(
            "Go to B",
            CommandScope::Application,
            Vec::new(),
            move |ui, _| {
                ui.reveal_child(&t, "b")?;
                ui.flush(ui.now())?;
                ui.reveal_and_focus(&g).map(|_| ())
            },
        )
        .unwrap();
    let context = ui.command_context(CommandSource::Menu);
    assert_eq!(
        ui.invoke_command(&command, context).unwrap(),
        CommandOutcome::Invoked
    );
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(ui.focused(), Some(target));
}

#[test]
fn a_task_queued_with_every_root_hidden_is_delivered_rather_than_due_forever() {
    let mut ui = ui();
    mount(&mut ui);
    ui.flush(Duration::ZERO).unwrap();
    ui.set_window_visible(false).unwrap();
    let ran = Rc::new(RefCell::new(false));
    let seen = ran.clone();
    let receipt = ui
        .defer(move |_| {
            *seen.borrow_mut() = true;
            Ok(())
        })
        .unwrap();
    assert_eq!(ui.application_deadline(), Some(ui.now()));
    let report = ui.flush_windows(ui.now()).unwrap();
    assert!(*ran.borrow(), "the task did not run");
    assert_eq!(receipt.status(), TaskStatus::Succeeded);
    assert_eq!(report.next_deadline, None);
}
