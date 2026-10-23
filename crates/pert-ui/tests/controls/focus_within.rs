//! `Component::focus_within_changed` contracts: which ancestors are told,
//! in which order relative to `focus_lost`/`focus_gained`, on which occasions,
//! and how errors propagate.
use pert_ui::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

type Error = Box<dyn std::error::Error>;
type TestResult = Result<(), Error>;
type Log = Rc<RefCell<Vec<String>>>;

/// What a probe does with a pressed Enter.
#[derive(Clone, Copy, PartialEq)]
enum Enter {
    Ignore,
    /// Become non-focusable while focused, as an editor whose session ended,
    /// and let the key bubble.
    Unfocus,
    /// Take focus back, as an owner ending an editor's session does.
    TakeFocus,
}

struct Probe {
    name: &'static str,
    focusable: bool,
    enter: Enter,
    fail_on_leave: Option<RuntimeError>,
    /// Focusability a test switches from outside, on top of `focusable`.
    gate: Option<Rc<Cell<bool>>>,
    /// Set by Enter under `Enter::Unfocus`; applied in the next update, as a
    /// component whose state changes there becomes non-focusable.
    unfocus: bool,
    log: Log,
    children: Vec<Element>,
}

impl Probe {
    fn element(name: &'static str, log: &Log, children: Vec<Element>) -> Element {
        Element::new(
            name,
            Self {
                name,
                focusable: name != "root",
                enter: Enter::Ignore,
                fail_on_leave: None,
                gate: None,
                unfocus: false,
                log: log.clone(),
                children,
            },
        )
    }
}

impl Component for Probe {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn focusable(&self) -> bool {
        self.focusable && self.gate.as_ref().is_none_or(|gate| gate.get())
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        if std::mem::take(&mut self.unfocus) {
            self.focusable = false;
        }
        Ok(())
    }
    fn pointer_input(
        &mut self,
        _: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if matches!(event.kind(), PointerKind::Press(_)) {
            self.log.borrow_mut().push(format!("{} pressed", self.name));
            return Ok(InputOutcome::Handled);
        }
        Ok(InputOutcome::Ignored)
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(100.0, 60.0)?)
    }
    // Children share the height in rows, so the validator sees no overlap.
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        let row = size.height() / cx.len().max(1) as f32;
        for index in 0..cx.len() {
            cx.place(
                index,
                Rect::new(0.0, row * index as f32, size.width(), row)?,
            )?;
        }
        Ok(())
    }
    fn cancel_input(&mut self, _: &mut EventContext) {
        self.log
            .borrow_mut()
            .push(format!("{} cancelled", self.name));
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        if event.code != KeyCode::Enter || event.transition != KeyTransition::Pressed {
            return Ok(InputOutcome::Ignored);
        }
        match self.enter {
            Enter::Ignore => Ok(InputOutcome::Ignored),
            Enter::Unfocus => {
                self.unfocus = true;
                cx.invalidate(Invalidation::Paint);
                Ok(InputOutcome::Ignored)
            }
            Enter::TakeFocus => {
                cx.request_focus();
                Ok(InputOutcome::Handled)
            }
        }
    }
    fn focus_gained(&mut self, _: &mut EventContext) {
        self.log.borrow_mut().push(format!("{} gained", self.name));
    }
    fn focus_lost(&mut self, _: &mut EventContext) -> Result<(), RuntimeError> {
        self.log.borrow_mut().push(format!("{} lost", self.name));
        Ok(())
    }
    fn focus_within_changed(
        &mut self,
        _: &mut EventContext,
        within: bool,
    ) -> Result<(), RuntimeError> {
        let change = if within { "entered" } else { "left" };
        self.log
            .borrow_mut()
            .push(format!("{} {change}", self.name));
        if let (Some(error), false) = (&self.fail_on_leave, within) {
            return Err(error.clone());
        }
        Ok(())
    }
}

struct Tree {
    ui: Runtime,
    root: WidgetId,
    log: Log,
}

impl Tree {
    /// root ─┬─ group ─┬─ x
    ///       │         └─ y
    ///       └─ z
    fn new() -> Result<Self, Error> {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
        let log: Log = Rc::default();
        let root = ui.mount_tree(Probe::element(
            "root",
            &log,
            vec![
                Probe::element(
                    "group",
                    &log,
                    vec![
                        Probe::element("x", &log, vec![]),
                        Probe::element("y", &log, vec![]),
                    ],
                ),
                Probe::element("z", &log, vec![]),
            ],
        ))?;
        ui.flush(ui.now())?;
        Ok(Self { ui, root, log })
    }
    fn id(&self, key: &str) -> Result<WidgetId, Error> {
        Ok(self
            .ui
            .find_descendant(&self.root, key)?
            .ok_or(key.to_owned())?)
    }
    fn focus(&mut self, key: &str) -> Result<(), RuntimeError> {
        let id = self.id(key).expect("mounted");
        self.ui.focus(&id)
    }
    fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.log.borrow_mut())
    }
}

#[test]
fn entering_tells_ancestors_outermost_first_before_focus_gained() -> TestResult {
    let mut tree = Tree::new()?;
    tree.focus("x")?;
    assert_eq!(tree.take(), ["root entered", "group entered", "x gained"]);
    Ok(())
}

#[test]
fn leaving_tells_only_ancestors_that_lose_focus_innermost_first_after_focus_lost() -> TestResult {
    let mut tree = Tree::new()?;
    tree.focus("x")?;
    tree.take();
    tree.focus("z")?;
    assert_eq!(
        tree.take(),
        ["x lost", "group left", "z gained"],
        "root keeps focus within"
    );
    Ok(())
}

#[test]
fn moving_between_descendants_or_to_the_ancestor_itself_tells_nobody() -> TestResult {
    let mut tree = Tree::new()?;
    tree.focus("x")?;
    tree.take();
    tree.focus("y")?;
    assert_eq!(tree.take(), ["x lost", "y gained"]);
    tree.focus("group")?;
    assert_eq!(
        tree.take(),
        ["y lost", "group gained"],
        "group itself is now focused"
    );
    tree.focus("x")?;
    assert_eq!(tree.take(), ["group lost", "x gained"]);
    Ok(())
}

#[test]
fn window_focus_loss_tells_every_ancestor() -> TestResult {
    let mut tree = Tree::new()?;
    tree.focus("x")?;
    tree.take();
    tree.ui.clear_focus();
    assert_eq!(
        tree.take(),
        [
            "x lost",
            "group left",
            "root left",
            "group cancelled",
            "root cancelled"
        ],
        "each ancestor hears that focus left before its own input is cancelled"
    );
    Ok(())
}

#[test]
fn hiding_and_removal_cancel_without_telling_ancestors() -> TestResult {
    let mut tree = Tree::new()?;
    tree.focus("x")?;
    tree.take();
    let x = tree.id("x")?;
    tree.ui.set_visible(&x, false)?;
    tree.ui.flush(tree.ui.now())?;
    assert!(
        !tree.take().iter().any(|entry| entry.ends_with("left")),
        "hiding is a cancellation"
    );

    tree.ui.set_visible(&x, true)?;
    tree.focus("y")?;
    tree.take();
    let group = tree.id("group")?;
    tree.ui
        .reconcile_children(&group, vec![Probe::element("x", &tree.log.clone(), vec![])])?;
    tree.ui.flush(tree.ui.now())?;
    assert!(
        !tree.take().iter().any(|entry| entry.ends_with("left")),
        "removal is a cancellation"
    );
    Ok(())
}

#[test]
fn an_ancestor_error_does_not_veto_the_transfer_or_stop_other_notifications() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
    let log: Log = Rc::default();
    let failing = |name, children| {
        Element::new(
            name,
            Probe {
                name,
                focusable: false,
                enter: Enter::Ignore,
                fail_on_leave: Some(RuntimeError::Exhausted),
                gate: None,
                unfocus: false,
                log: log.clone(),
                children,
            },
        )
    };
    let root = ui.mount_tree(Probe::element(
        "root",
        &log,
        vec![
            failing(
                "outer",
                vec![failing("inner", vec![Probe::element("x", &log, vec![])])],
            ),
            Probe::element("z", &log, vec![]),
        ],
    ))?;
    ui.flush(ui.now())?;
    let x = ui.find_descendant(&root, "x")?.ok_or("x")?;
    let z = ui.find_descendant(&root, "z")?.ok_or("z")?;
    ui.focus(&x)?;
    log.borrow_mut().clear();
    assert_eq!(ui.focus(&z), Err(RuntimeError::Exhausted));
    assert_eq!(ui.focused(), Some(z.clone()), "the transfer happened");
    assert_eq!(
        *log.borrow(),
        ["x lost", "inner left", "outer left", "z gained"],
        "every ancestor was told despite the first error"
    );
    assert_eq!(
        ui.flush(ui.now()),
        Err(RuntimeError::Exhausted),
        "the second error was queued, not dropped"
    );
    ui.flush(ui.now())?;

    ui.focus(&x)?;
    ui.clear_focus();
    assert_eq!(
        ui.flush(ui.now()),
        Err(RuntimeError::Exhausted),
        "queued for flush"
    );
    assert_eq!(
        ui.flush(ui.now()),
        Err(RuntimeError::Exhausted),
        "one per ancestor"
    );
    ui.flush(ui.now())?;
    Ok(())
}

/// A container whose [`Component::reveal`] fails, as a scroll viewport whose
/// scrolling is refused.
struct FailingReveal {
    children: Vec<Element>,
}

impl Component for FailingReveal {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(100.0, 30.0)?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        for index in 0..cx.len() {
            cx.place(index, Rect::new(0.0, 0.0, size.width(), size.height())?)?;
        }
        Ok(())
    }
    fn can_reveal(&self) -> bool {
        true
    }
    fn reveal(&mut self, _: &mut EventContext, _: Rect) -> Result<bool, RuntimeError> {
        Err(RuntimeError::RevealQueueFull)
    }
}

#[test]
fn a_reveal_error_queues_the_callback_error_it_is_returned_before() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
    let log: Log = Rc::default();
    let root = ui.mount_tree(Probe::element(
        "root",
        &log,
        vec![
            Element::new(
                "outer",
                Probe {
                    name: "outer",
                    focusable: false,
                    enter: Enter::Ignore,
                    fail_on_leave: Some(RuntimeError::Exhausted),
                    gate: None,
                    unfocus: false,
                    log: log.clone(),
                    children: vec![Probe::element("x", &log, vec![])],
                },
            ),
            Element::new(
                "viewport",
                FailingReveal {
                    children: vec![Probe::element("z", &log, vec![])],
                },
            ),
        ],
    ))?;
    ui.flush(ui.now())?;
    let x = ui.find_descendant(&root, "x")?.ok_or("x")?;
    let z = ui.find_descendant(&root, "z")?.ok_or("z")?;
    ui.focus(&x)?;
    assert_eq!(
        ui.focus(&z),
        Err(RuntimeError::RevealQueueFull),
        "the reveal error is returned"
    );
    assert_eq!(ui.focused(), Some(z), "the transfer happened");
    assert_eq!(
        ui.flush(ui.now()),
        Err(RuntimeError::Exhausted),
        "the held focus_within_changed error was queued, not dropped"
    );
    ui.flush(ui.now())?;
    Ok(())
}

fn probe(name: &'static str, enter: Enter, log: &Log, children: Vec<Element>) -> Element {
    Element::new(
        name,
        Probe {
            name,
            focusable: true,
            enter,
            fail_on_leave: None,
            gate: None,
            unfocus: false,
            log: log.clone(),
            children,
        },
    )
}

#[test]
fn taking_focus_back_from_a_child_that_became_unfocusable_tells_no_ancestor() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
    let log: Log = Rc::default();
    let root = ui.mount_tree(Probe::element(
        "root",
        &log,
        vec![
            probe(
                "row",
                Enter::TakeFocus,
                &log,
                vec![probe("editor", Enter::Unfocus, &log, vec![])],
            ),
            Probe::element("z", &log, vec![]),
        ],
    ))?;
    ui.flush(ui.now())?;
    let editor = ui.find_descendant(&root, "editor")?.ok_or("editor")?;
    ui.focus(&editor)?;
    log.borrow_mut().clear();
    ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    ui.flush(ui.now())?;
    let told: Vec<_> = log
        .borrow()
        .iter()
        .filter(|entry| entry.ends_with("entered") || entry.ends_with("left"))
        .cloned()
        .collect();
    assert!(
        log.borrow().contains(&"editor cancelled".to_owned())
            && !log.borrow().contains(&"editor lost".to_owned()),
        "focus was cleared silently, not transferred: {:?}",
        log.borrow()
    );
    assert!(
        told.is_empty(),
        "root kept focus within throughout: {told:?}"
    );
    assert_eq!(log.borrow().last().map(String::as_str), Some("row gained"));
    log.borrow_mut().clear();
    let z = ui.find_descendant(&root, "z")?.ok_or("z")?;
    ui.focus(&z)?;
    assert_eq!(*log.borrow(), ["row lost", "z gained"]);
    Ok(())
}

#[test]
fn moving_the_focused_subtree_into_its_own_window_tells_every_ancestor() -> TestResult {
    let mut tree = Tree::new()?;
    tree.focus("x")?;
    tree.take();
    let group = tree.id("group")?;
    let prepared = tree
        .ui
        .prepare_detach(&group, NativeWindowOptions::new("Group")?)?;
    let detached = tree.ui.commit_detach(prepared)?;
    assert_eq!(tree.ui.window_of(&group)?, detached);
    let log = tree.take();
    assert_eq!(&log[..3], ["x lost", "group left", "root left"]);
    Ok(())
}

#[test]
fn a_focus_policy_error_keeps_the_view_from_moving() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
    let log: Log = Rc::default();
    let root = ui.mount_tree(Probe::element(
        "root",
        &log,
        vec![
            Element::new(
                "outer",
                Probe {
                    name: "outer",
                    focusable: false,
                    enter: Enter::Ignore,
                    fail_on_leave: Some(RuntimeError::Exhausted),
                    gate: None,
                    unfocus: false,
                    log: log.clone(),
                    children: vec![Probe::element("x", &log, vec![])],
                },
            ),
            Probe::element("z", &log, vec![]),
        ],
    ))?;
    ui.flush(ui.now())?;
    let x = ui.find_descendant(&root, "x")?.ok_or("x")?;
    let outer = ui.find_descendant(&root, "outer")?.ok_or("outer")?;
    ui.focus(&x)?;
    let primary = ui.primary_window();
    let prepared = ui.prepare_detach(&outer, NativeWindowOptions::new("Outer")?)?;
    assert_eq!(ui.commit_detach(prepared), Err(RuntimeError::Exhausted));
    assert_eq!(
        ui.window_of(&outer)?,
        primary,
        "the view stayed in its window"
    );
    Ok(())
}

#[test]
fn cancelling_input_in_place_keeps_what_ancestors_were_told() -> TestResult {
    let mut tree = Tree::new()?;
    tree.focus("x")?;
    tree.ui.cancel_input()?;
    tree.take();
    tree.focus("z")?;
    let log = tree.take();
    assert!(log.contains(&"group left".to_owned()), "{log:?}");
    assert!(!log.contains(&"root entered".to_owned()), "{log:?}");
    Ok(())
}

#[test]
fn a_press_is_delivered_although_its_focus_transfer_failed_twice() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
    let log: Log = Rc::default();
    let failing = |name, error, children| {
        Element::new(
            name,
            Probe {
                name,
                focusable: false,
                enter: Enter::Ignore,
                fail_on_leave: Some(error),
                gate: None,
                unfocus: false,
                log: log.clone(),
                children,
            },
        )
    };
    let root = ui.mount_tree(Probe::element(
        "root",
        &log,
        vec![
            failing(
                "outer",
                RuntimeError::Exhausted,
                vec![failing(
                    "inner",
                    RuntimeError::Hidden,
                    vec![Probe::element("x", &log, vec![])],
                )],
            ),
            Probe::element("z", &log, vec![]),
        ],
    ))?;
    ui.flush(ui.now())?;
    ui.paint()?;
    let x = ui.find_descendant(&root, "x")?.ok_or("x")?;
    let z = ui.find_descendant(&root, "z")?.ok_or("z")?;
    ui.focus(&x)?;
    log.borrow_mut().clear();
    let [left, top, width, height] = ui.bounds(&z)?.ok_or("z unplaced")?.values();
    let point = Point::new(left + width / 2.0, top + height / 2.0)?;
    let press = ui.pointer_input(PointerEvent::new(
        point,
        PointerKind::Press(PointerButton::Primary),
    ));
    assert_eq!(
        press.err(),
        Some(RuntimeError::Hidden),
        "the innermost ancestor's error comes first"
    );
    assert!(
        log.borrow().contains(&"z pressed".to_owned()),
        "{:?}",
        log.borrow()
    );
    assert_eq!(
        ui.flush(ui.now()),
        Err(RuntimeError::Exhausted),
        "the second waits"
    );
    Ok(())
}

#[test]
fn an_ancestor_moved_into_its_own_window_is_not_told_from_the_old_one() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
    let log: Log = Rc::default();
    let open = Rc::new(Cell::new(false));
    let gated = |name, children| {
        Element::new(
            name,
            Probe {
                name,
                focusable: true,
                enter: Enter::Ignore,
                fail_on_leave: None,
                gate: Some(open.clone()),
                unfocus: false,
                log: log.clone(),
                children,
            },
        )
    };
    let root = ui.mount_tree(Probe::element(
        "root",
        &log,
        vec![
            Element::new(
                "group",
                Probe {
                    name: "group",
                    focusable: false,
                    enter: Enter::Ignore,
                    fail_on_leave: None,
                    gate: None,
                    unfocus: false,
                    log: log.clone(),
                    children: vec![probe("x", Enter::Unfocus, &log, vec![]), gated("y", vec![])],
                },
            ),
            gated("z", vec![]),
        ],
    ))?;
    ui.flush(ui.now())?;
    let find = |ui: &Runtime, key: &str| ui.find_descendant(&root, key);
    let x = find(&ui, "x")?.ok_or("x")?;
    ui.focus(&x)?;
    ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    ui.flush(ui.now())?;
    assert_eq!(
        ui.focused(),
        None,
        "nothing else was focusable: a silent clear"
    );

    let group = find(&ui, "group")?.ok_or("group")?;
    let prepared = ui.prepare_detach(&group, NativeWindowOptions::new("Group")?)?;
    let detached = ui.commit_detach(prepared)?;
    open.set(true);
    let y = find(&ui, "y")?.ok_or("y")?;
    ui.focus(&y)?;
    assert_eq!(ui.window_of(&y)?, detached);
    log.borrow_mut().clear();
    ui.select_window(&ui.primary_window())?;
    let z = find(&ui, "z")?.ok_or("z")?;
    ui.focus(&z)?;
    assert!(
        !log.borrow().contains(&"group left".to_owned()),
        "group holds focus in its own window: {:?}",
        log.borrow()
    );
    Ok(())
}

#[test]
fn a_component_that_stops_being_focusable_hands_focus_to_its_nearest_focusable_ancestor()
-> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
    let log: Log = Rc::default();
    let root = ui.mount_tree(Probe::element(
        "root",
        &log,
        vec![
            Probe::element("first", &log, vec![]),
            probe(
                "row",
                Enter::Ignore,
                &log,
                vec![probe("editor", Enter::Unfocus, &log, vec![])],
            ),
        ],
    ))?;
    ui.flush(ui.now())?;
    let editor = ui.find_descendant(&root, "editor")?.ok_or("editor")?;
    let row = ui.find_descendant(&root, "row")?.ok_or("row")?;
    ui.focus(&editor)?;
    ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    ui.flush(ui.now())?;
    assert_eq!(ui.focused(), Some(row), "not the window's first control");
    Ok(())
}

#[test]
fn hiding_the_focused_component_hands_focus_to_its_nearest_focusable_ancestor() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
    let log: Log = Rc::default();
    let root = ui.mount_tree(Probe::element(
        "root",
        &log,
        vec![
            Probe::element("first", &log, vec![]),
            Probe::element(
                "container",
                &log,
                vec![Probe::element("child", &log, vec![])],
            ),
        ],
    ))?;
    ui.flush(ui.now())?;
    let child = ui.find_descendant(&root, "child")?.ok_or("child")?;
    let container = ui.find_descendant(&root, "container")?.ok_or("container")?;
    ui.focus(&child)?;
    ui.set_visible(&child, false)?;
    ui.flush(ui.now())?;
    assert_eq!(
        ui.focused(),
        Some(container),
        "not the window's first control"
    );
    Ok(())
}

#[test]
fn reparenting_the_focused_subtree_tells_the_old_and_new_ancestors() -> TestResult {
    let mut tree = Tree::new()?;
    tree.focus("x")?;
    tree.take();
    let (x, z) = (tree.id("x")?, tree.id("z")?);
    tree.ui.reparent(&x, &z, 0)?;
    assert_eq!(
        tree.take(),
        ["group left", "z entered"],
        "root holds focus within before and after"
    );
    assert_eq!(tree.ui.focused(), Some(x), "focus stays where it was");
    tree.ui.flush(tree.ui.now())?;
    // Later transfers are measured against the new path.
    tree.focus("y")?;
    assert_eq!(
        tree.take(),
        ["x lost", "z left", "group entered", "y gained"]
    );
    Ok(())
}
