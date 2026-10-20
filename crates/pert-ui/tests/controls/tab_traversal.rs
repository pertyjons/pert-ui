//! Traversal scopes (`Component::descendant_tab_stops`) and the Tab offer
//! (`Component::tab_navigation`) on both key paths.
use crate::support;
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};
use support::frame;

/// Each claim of Tab: the claimer's name and whether it was Shift+Tab.
type Claims = Rc<RefCell<Vec<(String, bool)>>>;

/// One child laid out in its allocation; optionally a stop itself, keeping
/// its descendants out of traversal and claiming Tab when asked to.
struct Wrapper {
    child: Option<Element>,
    focusable: bool,
    scope: bool,
    claims: Option<Claims>,
    name: &'static str,
    /// Writes this signal, then fails, when offered Tab.
    fails: Option<Signal<u32>>,
    resets: Rc<std::cell::Cell<u32>>,
}
impl Wrapper {
    fn new(name: &'static str, child: Element) -> Self {
        Self {
            child: Some(child),
            focusable: false,
            scope: false,
            claims: None,
            name,
            fails: None,
            resets: Rc::default(),
        }
    }
    fn scope(mut self, focusable: bool) -> Self {
        self.scope = true;
        self.focusable = focusable;
        self
    }
    fn claims(mut self, log: &Claims) -> Self {
        self.claims = Some(log.clone());
        self
    }
}
impl Component for Wrapper {
    fn take_children(&mut self) -> Vec<Element> {
        self.child.take().into_iter().collect()
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn focusable(&self) -> bool {
        self.focusable
    }
    fn descendant_tab_stops(&self) -> bool {
        !self.scope
    }
    fn measure(&self, cx: &MeasureContext<'_>, space: Constraints) -> Result<Size, RuntimeError> {
        let child = if cx.is_empty() {
            Size::new(0.0, 0.0)?
        } else {
            cx.measure(0, space)?
        };
        Ok(Size::new(child.width(), child.height().max(24.0))?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        if !cx.is_empty() {
            cx.place(0, Rect::from_size(size))?;
        }
        Ok(())
    }
    fn input_reset(&mut self) {
        self.resets.set(self.resets.get() + 1);
    }
    fn tab_navigation(
        &mut self,
        _: &mut EventContext,
        reverse: bool,
        _: bool,
    ) -> Result<InputOutcome, RuntimeError> {
        if let Some(written) = &self.fails {
            written.update(|n| *n += 1)?;
            return Err(RuntimeError::Exhausted);
        }
        match &self.claims {
            Some(log) => {
                log.borrow_mut().push((self.name.to_owned(), reverse));
                Ok(InputOutcome::Handled)
            }
            None => Ok(InputOutcome::Ignored),
        }
    }
}

fn button(key: &str) -> Element {
    Element::new(key, Button::new(key.to_owned(), |_| Ok(())))
}

fn runtime() -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 600.0, 1.0).unwrap())
        .unwrap();
    ui
}

fn focused(ui: &Runtime) -> Option<String> {
    ui.focused().map(|id| ui.key(&id).unwrap().to_owned())
}

/// Presses Tab or Shift+Tab on the headless or the native path.
fn tab(ui: &mut Runtime, reverse: bool, native: bool) {
    let shift = |ui: &mut Runtime, action| {
        if native {
            ui.command_key_input(CommandKeyInput::new(
                KeyCode::ShiftLeft,
                action,
                CommandKey::Named(KeyCode::ShiftLeft),
            ))
            .unwrap();
        } else {
            ui.key_input(KeyCode::ShiftLeft, action).unwrap();
        }
    };
    if reverse {
        shift(ui, KeyAction::Press);
    }
    for action in [KeyAction::Press, KeyAction::Release] {
        if native {
            ui.command_key_input(CommandKeyInput::new(
                KeyCode::Tab,
                action,
                CommandKey::Named(KeyCode::Tab),
            ))
            .unwrap();
        } else {
            ui.key_input(KeyCode::Tab, action).unwrap();
        }
    }
    if reverse {
        shift(ui, KeyAction::Release);
    }
    frame(ui);
}

/// The keys focused by successive Tab or Shift+Tab presses.
fn walk(ui: &mut Runtime, reverse: bool, native: bool, steps: usize) -> Vec<String> {
    (0..steps)
        .map(|_| {
            tab(ui, reverse, native);
            focused(ui).unwrap_or_default()
        })
        .collect()
}

fn scoped_page(scope_focusable: bool) -> Element {
    Element::new(
        "root",
        Column::new([
            button("before"),
            Element::new(
                "scope",
                Wrapper::new(
                    "scope",
                    Element::new("inner", Column::new([button("a"), button("b")])),
                )
                .scope(scope_focusable),
            ),
            button("after"),
        ]),
    )
}

#[test]
fn a_scope_is_one_stop_in_both_directions_on_both_paths() {
    for native in [false, true] {
        let mut ui = runtime();
        let root = ui.mount_tree(scoped_page(true)).unwrap();
        frame(&mut ui);
        let before = ui.find_descendant(&root, "before").unwrap().unwrap();
        ui.focus(&before).unwrap();
        assert_eq!(
            walk(&mut ui, false, native, 3),
            ["scope", "after", "before"],
            "native {native}"
        );
        assert_eq!(
            walk(&mut ui, true, native, 3),
            ["after", "scope", "before"],
            "native {native}"
        );
        // A scope that is not a stop itself is skipped with its content.
        let mut ui = runtime();
        let root = ui.mount_tree(scoped_page(false)).unwrap();
        frame(&mut ui);
        let before = ui.find_descendant(&root, "before").unwrap().unwrap();
        ui.focus(&before).unwrap();
        assert_eq!(walk(&mut ui, false, native, 2), ["after", "before"]);
    }
}

#[test]
fn focus_next_honors_the_scope_and_focus_inside_it_still_works() {
    let mut ui = runtime();
    let root = ui.mount_tree(scoped_page(true)).unwrap();
    frame(&mut ui);
    let before = ui.find_descendant(&root, "before").unwrap().unwrap();
    ui.focus(&before).unwrap();
    ui.focus_next(false).unwrap();
    assert_eq!(focused(&ui).as_deref(), Some("scope"));
    ui.focus_next(false).unwrap();
    assert_eq!(focused(&ui).as_deref(), Some("after"));

    // Explicit focus reaches the content; traversal continues from its place.
    let b = ui.find_descendant(&root, "b").unwrap().unwrap();
    ui.focus(&b).unwrap();
    assert_eq!(focused(&ui).as_deref(), Some("b"));
    tab(&mut ui, false, false);
    assert_eq!(focused(&ui).as_deref(), Some("after"));
    let a = ui.find_descendant(&root, "a").unwrap().unwrap();
    ui.focus(&a).unwrap();
    tab(&mut ui, true, false);
    assert_eq!(focused(&ui).as_deref(), Some("scope"));
}

#[test]
fn nested_and_root_mounted_scopes_compose() {
    let mut ui = runtime();
    // The root itself is the scope.
    let root = ui
        .mount_tree(Element::new(
            "root",
            Wrapper::new(
                "root",
                Element::new(
                    "column",
                    Column::new([
                        button("a"),
                        Element::new("inner", Wrapper::new("inner", button("b")).scope(true)),
                    ]),
                ),
            )
            .scope(true),
        ))
        .unwrap();
    frame(&mut ui);
    ui.focus(&root).unwrap();
    // Only the root is a stop; traversal wraps to it.
    assert_eq!(walk(&mut ui, false, false, 2), ["root", "root"]);
}

#[test]
fn a_dialog_opened_inside_a_scope_keeps_its_stops() {
    let mut ui = runtime();
    let dialog = DialogController::new(&ui);
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                button("before"),
                Element::new(
                    "scope",
                    Wrapper::new(
                        "scope",
                        Element::new(
                            "owner",
                            Dialog::new(
                                dialog.clone(),
                                Element::new("anchor", Label::new("Main")),
                                Element::new(
                                    "content",
                                    Column::new([button("ok"), button("cancel")]),
                                ),
                            ),
                        ),
                    )
                    .scope(true),
                ),
            ]),
        ))
        .unwrap();
    dialog.open().unwrap();
    frame(&mut ui);
    let ok = ui.find_descendant(&root, "ok").unwrap().unwrap();
    ui.focus(&ok).unwrap();
    for native in [false, true] {
        let stops = walk(&mut ui, false, native, 2);
        assert_eq!(stops, ["cancel", "ok"], "native {native}");
    }
}

#[test]
fn the_innermost_claim_wins_and_an_unclaimed_tab_traverses() {
    for native in [false, true] {
        let mut ui = runtime();
        let outer = Rc::new(RefCell::new(Vec::new()));
        let inner = Rc::new(RefCell::new(Vec::new()));
        let root = ui
            .mount_tree(Element::new(
                "root",
                Column::new([
                    Element::new(
                        "outer",
                        Wrapper::new(
                            "outer",
                            Element::new(
                                "inner",
                                Wrapper::new("inner", button("field")).claims(&inner),
                            ),
                        )
                        .claims(&outer),
                    ),
                    button("after"),
                ]),
            ))
            .unwrap();
        frame(&mut ui);
        let field = ui.find_descendant(&root, "field").unwrap().unwrap();
        ui.focus(&field).unwrap();
        tab(&mut ui, false, native);
        tab(&mut ui, true, native);
        assert_eq!(focused(&ui).as_deref(), Some("field"), "native {native}");
        assert_eq!(
            *inner.borrow(),
            [("inner".to_owned(), false), ("inner".to_owned(), true)],
            "native {native}"
        );
        assert!(outer.borrow().is_empty(), "native {native}");

        // Without a claim the key traverses as before.
        let mut ui = runtime();
        let root = ui
            .mount_tree(Element::new(
                "root",
                Column::new([
                    Element::new("outer", Wrapper::new("outer", button("field"))),
                    button("after"),
                ]),
            ))
            .unwrap();
        frame(&mut ui);
        let field = ui.find_descendant(&root, "field").unwrap().unwrap();
        ui.focus(&field).unwrap();
        tab(&mut ui, false, native);
        assert_eq!(focused(&ui).as_deref(), Some("after"), "native {native}");
    }
}

#[test]
fn the_page_behind_a_modal_cannot_claim_tab() {
    let mut ui = runtime();
    let claims = Rc::new(RefCell::new(Vec::new()));
    let dialog = DialogController::new(&ui);
    let root = ui
        .mount_tree(Element::new(
            "page",
            Wrapper::new(
                "page",
                Element::new(
                    "owner",
                    Dialog::new(
                        dialog.clone(),
                        Element::new("anchor", Label::new("Main")),
                        Element::new("content", Column::new([button("ok"), button("cancel")])),
                    ),
                ),
            )
            .claims(&claims),
        ))
        .unwrap();
    dialog.open().unwrap();
    frame(&mut ui);
    let ok = ui.find_descendant(&root, "ok").unwrap().unwrap();
    ui.focus(&ok).unwrap();
    tab(&mut ui, false, true);
    assert_eq!(focused(&ui).as_deref(), Some("cancel"));
    assert!(claims.borrow().is_empty());
}

#[test]
fn a_capture_binding_on_tab_wins_over_the_offer() {
    let mut ui = runtime();
    let claims = Rc::new(RefCell::new(Vec::new()));
    let captured = ui.signal(0_u32);
    let count = captured.clone();
    let _capture = ui
        .register_command(
            "Capture Tab",
            CommandScope::Capture,
            vec![KeyChord::named(KeyCode::Tab)],
            move |_, _| count.update(|n| *n += 1),
        )
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new(
                    "claimer",
                    Wrapper::new("claimer", button("field")).claims(&claims),
                ),
                button("after"),
            ]),
        ))
        .unwrap();
    frame(&mut ui);
    let field = ui.find_descendant(&root, "field").unwrap().unwrap();
    ui.focus(&field).unwrap();
    tab(&mut ui, false, true);
    assert_eq!(captured.get().unwrap(), 1);
    assert!(claims.borrow().is_empty());
    assert_eq!(focused(&ui).as_deref(), Some("field"));
}

#[test]
fn an_error_from_the_offer_propagates_cleans_up_and_nothing_traverses() {
    for native in [false, true] {
        let mut ui = runtime();
        let written = ui.signal(0_u32);
        let mut failing = Wrapper::new("failing", button("field"));
        failing.fails = Some(written.clone());
        let resets = failing.resets.clone();
        let root = ui
            .mount_tree(Element::new(
                "root",
                Column::new([Element::new("failing", failing), button("after")]),
            ))
            .unwrap();
        frame(&mut ui);
        let field = ui.find_descendant(&root, "field").unwrap().unwrap();
        ui.focus(&field).unwrap();
        let before = resets.get();
        let result = if native {
            ui.command_key_input(CommandKeyInput::new(
                KeyCode::Tab,
                KeyAction::Press,
                CommandKey::Named(KeyCode::Tab),
            ))
            .map(|_| ())
        } else {
            ui.key_input(KeyCode::Tab, KeyAction::Press).map(|_| ())
        };
        assert_eq!(result, Err(RuntimeError::Exhausted), "native {native}");
        // The failing recipient is cleaned up; its committed write stays.
        assert!(resets.get() > before, "native {native}");
        assert_eq!(written.get().unwrap(), 1, "native {native}");
        ui.key_input(KeyCode::Tab, KeyAction::Release).unwrap();
        ui.flush(Duration::ZERO).unwrap();
        assert_eq!(focused(&ui).as_deref(), Some("field"), "native {native}");
    }
}

#[test]
fn a_held_tab_registration_wins_over_the_offer() {
    let mut ui = runtime();
    let claims = Rc::new(RefCell::new(Vec::new()));
    let held = Rc::new(std::cell::Cell::new(0));
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new(
                    "claimer",
                    Wrapper::new("claimer", button("field")).claims(&claims),
                ),
                button("after"),
            ]),
        ))
        .unwrap();
    frame(&mut ui);
    let field = ui.find_descendant(&root, "field").unwrap().unwrap();
    let count = held.clone();
    let _registration = ui
        .register_held_input(
            &field,
            vec![KeyCode::Tab],
            Modifiers::default(),
            move |_, _, _| {
                count.set(count.get() + 1);
                Ok(InputOutcome::Handled)
            },
            |_, _| {},
        )
        .unwrap();
    ui.focus(&field).unwrap();
    tab(&mut ui, false, true);
    assert!(held.get() > 0);
    assert!(claims.borrow().is_empty());
    assert_eq!(focused(&ui).as_deref(), Some("field"));
}

#[test]
fn pointer_presses_and_restoration_are_unaffected_by_a_scope() {
    let mut ui = runtime();
    let shown = ui.signal(true);
    let visible = shown.clone();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                button("before"),
                Element::new(
                    "scope",
                    Wrapper::new(
                        "scope",
                        Element::new(
                            "inner",
                            Column::new([
                                button("a"),
                                Element::new("maybe", Show::new(&visible, |_| button("b"))),
                            ]),
                        ),
                    )
                    .scope(true),
                ),
                button("after"),
            ]),
        ))
        .unwrap();
    frame(&mut ui);
    // A press on scoped content focuses it.
    let b = ui.find_descendant(&root, "b").unwrap().unwrap();
    let [x, y, w, h] = ui.bounds(&b).unwrap().unwrap().values();
    let point = Point::new(x + w / 2.0, y + h / 2.0).unwrap();
    ui.pointer_input(PointerEvent::new(
        point,
        PointerKind::Press(PointerButton::Primary),
    ))
    .unwrap();
    ui.pointer_input(PointerEvent::new(
        point,
        PointerKind::Release(PointerButton::Primary),
    ))
    .unwrap();
    frame(&mut ui);
    assert_eq!(focused(&ui).as_deref(), Some("b"));
    // Removing it restores focus by the usual rule: the nearest focusable
    // ancestor, which is the scope.
    shown.set(false).unwrap();
    frame(&mut ui);
    assert_eq!(focused(&ui).as_deref(), Some("scope"));
}

#[test]
fn a_dialog_opened_inside_a_scope_takes_its_initial_focus() {
    let mut ui = runtime();
    let dialog = DialogController::new(&ui);
    ui.mount_tree(Element::new(
        "root",
        Column::new([
            button("before"),
            Element::new(
                "scope",
                Wrapper::new(
                    "scope",
                    Element::new(
                        "owner",
                        Dialog::new(
                            dialog.clone(),
                            Element::new("anchor", Label::new("Main")),
                            Element::new("content", Column::new([button("ok"), button("cancel")])),
                        ),
                    ),
                )
                .scope(true),
            ),
        ]),
    ))
    .unwrap();
    frame(&mut ui);
    dialog.open().unwrap();
    frame(&mut ui);
    let initial = focused(&ui);
    assert!(
        matches!(initial.as_deref(), Some("ok" | "cancel")),
        "{initial:?}"
    );
}

struct Rows(usize);
impl ListItems for Rows {
    fn len(&self) -> usize {
        self.0
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < self.0).then(|| format!("row-{index}"))
    }
    fn build(&self, index: usize) -> Option<Element> {
        (index < self.0).then(|| button("check"))
    }
}

#[test]
fn a_list_without_item_tab_stops_is_one_stop() {
    for item_stops in [true, false] {
        let mut ui = runtime();
        let items = ui.signal(ListSource::new(Rows(3)));
        let selection = ListSelection::new(&ui, SelectionMode::Single);
        let list = VirtualList::new(&items, 28.0)
            .unwrap()
            .selection(selection)
            .item_tab_stops(item_stops);
        let root = ui
            .mount_tree(Element::new(
                "root",
                Column::new([
                    button("before"),
                    // Three short rows: an unbounded height mounts them all.
                    Element::new("body", list),
                    button("after"),
                ]),
            ))
            .unwrap();
        frame(&mut ui);
        let before = ui.find_descendant(&root, "before").unwrap().unwrap();
        ui.focus(&before).unwrap();
        let stops = walk(&mut ui, false, false, 5);
        if item_stops {
            // The list keys each built row by its source key.
            assert_eq!(stops, ["body", "row-0", "row-1", "row-2", "after"]);
        } else {
            assert_eq!(stops, ["body", "after", "before", "body", "after"]);
        }
    }
}
