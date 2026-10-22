//! Consumer contracts for changing descriptions and dynamic application state.
use pert_ui::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};
type Result = std::result::Result<(), Box<dyn std::error::Error>>;

struct Rows(&'static str);
impl ListItems for Rows {
    fn len(&self) -> usize {
        1
    }
    fn key(&self, i: usize) -> Option<String> {
        (i == 0).then(|| "row".into())
    }
    fn build(&self, i: usize) -> Option<Element> {
        self.key(i).map(|key| Element::new(key, Label::new(self.0)))
    }
}
#[test]
fn replacing_a_source_updates_same_key_content_without_remounting() -> Result {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let items = ui.signal(ListSource::new(Rows("Before")));
    let root = ui.mount_tree(Element::new(
        "list",
        VirtualList::new(items.read_only(), 24.0)?,
    ))?;
    ui.flush(Duration::ZERO)?;
    let row = ui.children(&root)?[0].clone();
    items.set(ListSource::new(Rows("After")))?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(ui.children(&root)?[0], row);
    let node = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == "row")
        .unwrap();
    assert_eq!(
        node.details.properties["text"],
        InspectValue::Text("After".into())
    );
    Ok(())
}

struct Outline;
impl TreeItems for Outline {
    fn roots(&self) -> Vec<String> {
        vec!["parent".into()]
    }
    fn children(&self, key: &str) -> Vec<String> {
        if key == "parent" {
            vec!["child".into()]
        } else {
            vec![]
        }
    }
    fn can_expand(&self, key: &str) -> bool {
        key == "parent"
    }
    fn build(&self, key: &str) -> Option<Element> {
        Some(Element::new(key, Label::new(key)))
    }
}
fn tree(
    expanded: bool,
) -> std::result::Result<(Runtime, TreeController), Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let state = TreeController::new(
        &ui,
        ListSelection::new(&ui, SelectionMode::Single),
        TreeSource::new(Outline),
    )?;
    state.set_expanded("parent", expanded)?;
    ui.mount_tree(Element::new("tree", Tree::new(&state, 24.0)?))?;
    ui.flush(Duration::ZERO)?;
    Ok((ui, state))
}
#[test]
fn expanding_a_retained_tree_matches_initially_expanded_drawing() -> Result {
    let (mut ui, state) = tree(false)?;
    let collapsed = ui.paint()?;
    state.set_expanded("parent", true)?;
    ui.flush(Duration::ZERO)?;
    let expanded = ui.paint()?;
    let (mut reference, _state) = tree(true)?;
    assert_ne!(collapsed, expanded);
    assert_eq!(expanded, reference.paint()?);
    Ok(())
}

#[test]
fn derived_bindings_batch_filter_changes_and_keep_chains_current() -> Result {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let count = ui.signal(0);
    assert!(count.read_only().same_source(&count.read_only()));
    assert!(!count.read_only().same_source(&ui.signal(0).read_only()));
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let even = count.map(move |value| {
        observed.set(observed.get() + 1);
        value % 2 == 0
    })?;
    let caption = even.map(|even| {
        if *even {
            "even".to_owned()
        } else {
            "odd".to_owned()
        }
    })?;
    ui.mount_tree(Element::new("label", Label::new(caption.clone())))?;
    ui.flush(Duration::ZERO)?;
    ui.paint()?;
    count.set(2)?;
    count.set(4)?;
    assert_eq!(calls.get(), 1, "writes run no callbacks");
    assert_eq!(
        ui.flush(Duration::ZERO)?.components_updated,
        0,
        "equal result does not invalidate label"
    );
    assert_eq!(calls.get(), 2);
    count.set(5)?;
    assert_eq!(
        caption.get()?,
        "odd",
        "a chained untracked read is current before flush"
    );
    assert_eq!(ui.flush(Duration::ZERO)?.components_updated, 1);
    let other = ui.signal(10);
    let sum = count.combine(&other.read_only(), |a, b| a + b)?;
    count.set(7)?;
    other.set(20)?;
    assert_eq!(sum.get()?, 27);
    let foreign = Runtime::new().signal(0);
    assert!(matches!(
        count
            .read_only()
            .combine(&foreign.read_only(), |a, b| a + b),
        Err(RuntimeError::ForeignRuntime)
    ));
    Ok(())
}

#[test]
fn button_properties_change_without_losing_identity_and_disable_cancels_press() -> Result {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let count = ui.signal(0);
    let enabled = ui.signal(true);
    let caption = count.map(|n| format!("Save {n}"))?;
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let id = ui.mount_tree(Element::new(
        "button",
        Button::new(caption, move |_| {
            observed.set(observed.get() + 1);
            Ok(())
        })
        .enabled(enabled.read_only()),
    ))?;
    ui.flush(Duration::ZERO)?;
    ui.focus(&id)?;
    ui.key_input(KeyCode::Space, KeyAction::Press)?;
    enabled.set(false)?;
    count.set(1)?;
    ui.flush(Duration::ZERO)?;
    ui.key_input(KeyCode::Space, KeyAction::Release)?;
    assert_eq!(calls.get(), 0);
    assert_ne!(ui.focused(), Some(id.clone()));
    enabled.set(true)?;
    ui.flush(Duration::ZERO)?;
    ui.focus(&id)?;
    ui.key_input(KeyCode::Space, KeyAction::Press)?;
    ui.key_input(KeyCode::Space, KeyAction::Release)?;
    assert_eq!(calls.get(), 1);
    Ok(())
}

#[test]
fn an_input_action_can_open_and_close_a_document_with_new_parameters() -> Result {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let document = Rc::new(RefCell::new(None));
    let owner = document.clone();
    let slot: Rc<RefCell<Option<WidgetId>>> = Rc::default();
    let target = slot.clone();
    let receipt = Rc::new(RefCell::new(None));
    let observed = receipt.clone();
    let id = ParameterId::new(1, 1, "amount")?;
    let spec = ParameterSpec::new("Amount", 0.0, 1.0, 0.5)?;
    let button = Element::new(
        "open",
        Button::new("Open", move |cx| {
            let owner = owner.clone();
            let target = target.clone();
            let id = id.clone();
            let spec = spec.clone();
            *observed.borrow_mut() = Some(cx.defer(move |ui| {
                let (parameter, host) = ui.parameter(id, spec)?;
                let parent = target.borrow().clone().ok_or(RuntimeError::Unmounted)?;
                ui.reconcile_children(
                    &parent,
                    vec![Element::new("document", Slider::new(parameter.clone()))],
                )?;
                let root = ui.children(&parent)?[0].clone();
                *owner.borrow_mut() = Some((root, parameter, host));
                Ok(())
            })?);
            Ok(())
        }),
    );
    let root = ui.mount_tree(Element::new(
        "application",
        Column::new([button, Element::new("documents", Container::default())]),
    ))?;
    let children = ui.children(&root)?;
    let button = children[0].clone();
    *slot.borrow_mut() = Some(children[1].clone());
    ui.flush(Duration::ZERO)?;
    ui.focus(&button)?;
    ui.key_input(KeyCode::Space, KeyAction::Press)?;
    ui.key_input(KeyCode::Space, KeyAction::Release)?;
    assert!(document.borrow().is_none());
    ui.flush(Duration::ZERO)?;
    assert_eq!(
        receipt.borrow().as_ref().unwrap().status(),
        TaskStatus::Succeeded
    );
    let (root, parameter, host) = document.borrow_mut().take().unwrap();
    let close = ui.defer(move |ui| {
        ui.unmount(&root)?;
        drop(host);
        Ok(())
    })?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(close.status(), TaskStatus::Succeeded);
    assert!(!parameter.snapshot().get()?.available);
    Ok(())
}

#[test]
fn deferred_operations_are_bounded_cancelled_and_never_replayed() -> Result {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let task = ui.defer(|_| Err(RuntimeError::EmptyKey))?;
    let next = ui.defer(|_| Ok(()))?;
    assert_eq!(ui.flush(Duration::ZERO), Err(RuntimeError::EmptyKey));
    assert_eq!(task.status(), TaskStatus::Failed(RuntimeError::EmptyKey));
    ui.flush(Duration::ZERO)?;
    assert_eq!(next.status(), TaskStatus::Succeeded);
    let mut tasks = Vec::new();
    for _ in 0..Runtime::MAX_TASKS {
        tasks.push(ui.defer(|_| Ok(()))?);
    }
    assert!(matches!(
        ui.defer(|_| Ok(())),
        Err(RuntimeError::TaskQueueFull)
    ));
    ui.shutdown();
    assert!(tasks.iter().all(|t| t.status() == TaskStatus::Cancelled));
    Ok(())
}

#[test]
fn bound_button_caption_keeps_adornments_and_tasks_cancel_with_owner() -> Result {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
    let caption = ui.signal("Save".to_owned());
    let id = ui.mount_tree(Element::new(
        "button",
        Button::new(caption.read_only(), |_| Ok(())).leading_icon(Icon::Add),
    ))?;
    ui.flush(Duration::ZERO)?;
    let bound = ui.paint()?;
    let mut reference = Runtime::new();
    reference.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
    reference.mount_tree(Element::new(
        "button",
        Button::new("Save", |_| Ok(())).leading_icon(Icon::Add),
    ))?;
    reference.flush(Duration::ZERO)?;
    assert_eq!(bound, reference.paint()?);
    let task = ui.event(&id, |cx| cx.defer(|_| panic!("removed owner must not run")))??;
    ui.unmount(&id)?;
    ui.mount_tree(Element::new(
        "button",
        Button::new("Replacement", |_| Ok(())),
    ))?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(task.status(), TaskStatus::Cancelled);
    let later = Rc::new(RefCell::new(None));
    let receipt = later.clone();
    let first = ui.defer(move |ui| {
        *receipt.borrow_mut() = Some(ui.defer(|_| Ok(()))?);
        Ok(())
    })?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(first.status(), TaskStatus::Succeeded);
    assert_eq!(
        later.borrow().as_ref().unwrap().status(),
        TaskStatus::Pending
    );
    ui.flush(Duration::ZERO)?;
    assert_eq!(
        later.borrow().as_ref().unwrap().status(),
        TaskStatus::Succeeded
    );
    Ok(())
}

#[test]
fn detached_tasks_outlive_their_widget_but_not_shutdown() -> Result {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
    let id = ui.mount_tree(Element::new("button", Button::new("Go", |_| Ok(()))))?;
    ui.flush(Duration::ZERO)?;
    let ran = Rc::new(Cell::new(false));
    let flag = ran.clone();
    let bound = ui.event(&id, |cx| cx.defer(|_| panic!("removed owner must not run")))??;
    let detached = ui.event(&id, |cx| {
        cx.defer_detached(move |_| {
            flag.set(true);
            Ok(())
        })
    })??;
    // The widget goes before the flush, as a picker closes after a choice.
    ui.unmount(&id)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(bound.status(), TaskStatus::Cancelled);
    assert_eq!(detached.status(), TaskStatus::Succeeded);
    assert!(ran.get());
    let id = ui.mount_tree(Element::new("button", Button::new("Go", |_| Ok(()))))?;
    ui.flush(Duration::ZERO)?;
    let late = ui.event(&id, |cx| {
        cx.defer_detached(|_| panic!("shutdown must cancel"))
    })??;
    ui.shutdown();
    assert_eq!(late.status(), TaskStatus::Cancelled);
    Ok(())
}

#[test]
fn derived_diamond_is_consistent_and_drop_releases_captured_state() -> Result {
    let mut ui = Runtime::new();
    let input = ui.signal(1);
    let left = input.map(|v| v * 2)?;
    let right = input.map(|v| v * 3)?;
    let sum = left.combine(&right, |a, b| a + b)?;
    input.set(4)?;
    assert_eq!(sum.get()?, 20);
    let lifetime = Rc::new(());
    let weak = Rc::downgrade(&lifetime);
    let derived = input.map(move |v| {
        let _ = &lifetime;
        v + 1
    })?;
    input.set(5)?;
    drop(derived);
    assert!(weak.upgrade().is_none());
    ui.flush(Duration::ZERO)?;
    let writer = input.clone();
    let guarded = input.map(move |_| writer.set(9))?;
    assert_eq!(guarded.get()?, Err(RuntimeError::WriteDuringUpdate));
    assert_eq!(input.get()?, 5);
    Ok(())
}

#[test]
fn unchanged_descriptions_retain_drawing_and_only_new_callback_is_applied() -> Result {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
    let calls = Rc::new(Cell::new(0));
    let describe = |calls: Rc<Cell<usize>>, amount| {
        Element::new(
            "row",
            Row::new([
                Element::new("label", Label::new("Static")),
                Element::new(
                    "button",
                    Button::new("Action", move |_| {
                        calls.set(calls.get() + amount);
                        Ok(())
                    }),
                ),
            ]),
        )
    };
    let root = ui.mount_tree(Element::new(
        "root",
        Container::new(describe(calls.clone(), 1)),
    ))?;
    ui.flush(Duration::ZERO)?;
    let scene = ui.paint()?;
    let row = ui.children(&root)?[0].clone();
    let button = ui.children(&row)?[1].clone();
    ui.reconcile_children(&root, vec![describe(calls.clone(), 2)])?;
    assert_eq!(ui.flush(Duration::ZERO)?.components_updated, 0);
    assert!(!ui.needs_paint());
    assert_eq!(ui.paint()?, scene);
    ui.focus(&button)?;
    ui.key_input(KeyCode::Space, KeyAction::Press)?;
    ui.key_input(KeyCode::Space, KeyAction::Release)?;
    assert_eq!(
        calls.get(),
        2,
        "replacement callback, retained interaction owner"
    );
    Ok(())
}

#[test]
fn inspection_does_not_evaluate_a_dirty_derived_callback() -> Result {
    struct Probe(ReadSignal<i32>);
    impl Component for Probe {
        fn update(&mut self, _: &mut UpdateContext) -> std::result::Result<(), RuntimeError> {
            Ok(())
        }
        fn inspect(&self) -> ComponentDetails {
            let mut details = ComponentDetails::default();
            details.properties.insert(
                "observed".into(),
                InspectValue::Text(format!("{:?}", self.0.get())),
            );
            details
        }
    }
    let mut ui = Runtime::new();
    let input = ui.signal(1);
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let derived = input.map(move |v| {
        count.set(count.get() + 1);
        v * 2
    })?;
    ui.mount("probe", Probe(derived))?;
    ui.flush(Duration::ZERO)?;
    input.set(3)?;
    let snapshot = ui.inspect();
    assert_eq!(calls.get(), 1);
    assert_eq!(
        snapshot.components[0].details.properties["observed"],
        InspectValue::Text("Err(WriteDuringInspection)".into())
    );
    ui.flush(Duration::ZERO)?;
    assert_eq!(calls.get(), 2);
    assert_eq!(
        ui.inspect().components[0].details.properties["observed"],
        InspectValue::Text("Ok(6)".into())
    );
    Ok(())
}
