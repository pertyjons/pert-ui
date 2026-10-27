//! Traversal scopes (`Component::descendant_tab_stops`) and the Tab offer
//! (`Component::tab_navigation`) on both key paths.
use crate::support;
use pert_ui::testing::HostIntegration;
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
    /// The `repeat` flag of each offer, when logged.
    offers: Option<Rc<RefCell<Vec<bool>>>>,
    /// Lets repeats pass instead of claiming them.
    passes_repeats: bool,
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
            offers: None,
            passes_repeats: false,
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
        direction: FocusDirection,
        transition: KeyTransition,
    ) -> Result<InputOutcome, RuntimeError> {
        let reverse = direction == FocusDirection::Backward;
        let repeat = transition == KeyTransition::Repeated;
        if let Some(offers) = &self.offers {
            offers.borrow_mut().push(repeat);
        }
        if repeat && self.passes_repeats {
            return Ok(InputOutcome::Ignored);
        }
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
    ui.focus_next(FocusDirection::Forward).unwrap();
    assert_eq!(focused(&ui).as_deref(), Some("scope"));
    ui.focus_next(FocusDirection::Forward).unwrap();
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
    ui.register_command(
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
    ui.register_held_input(
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

/// Tabs from the focused component until focus returns to `start` or
/// `limit` presses have passed; the keys focused on the way, `start`'s
/// return included.
fn walk_around(ui: &mut Runtime, start: &str, native: bool, limit: usize) -> Vec<String> {
    let mut seen = Vec::new();
    for _ in 0..limit {
        tab(ui, false, native);
        let key = focused(ui).unwrap_or_default();
        let back = key == start;
        seen.push(key);
        if back {
            break;
        }
    }
    seen
}

/// `before`, a focusable scope holding `a` and `b`, and `after`.
fn scoped_column(key: &str) -> Element {
    Element::new(
        key,
        Column::new([
            button("before"),
            Element::new(
                "scope",
                Wrapper::new(
                    "scope",
                    Element::new("inner", Column::new([button("a"), button("b")])),
                )
                .scope(true),
            ),
            button("after"),
        ]),
    )
}

/// Plays the native host for every queued detach: commits it and reports
/// it presented. Returns the new windows; the primary window is selected
/// again afterwards.
fn host_detaches(ui: &mut Runtime) -> Vec<NativeWindowId> {
    let primary = ui.primary_window();
    let mut windows = Vec::new();
    for request in ui.take_presentation_requests().unwrap() {
        let PresentationAction::Detach { view, options } = request.action() else {
            continue;
        };
        let prepared = ui.prepare_detach(view, options.clone()).unwrap();
        let committed = ui.commit_detach(prepared).unwrap();
        request
            .complete(PresentationStatus::Presented(committed.clone()))
            .unwrap();
        windows.push(committed);
    }
    ui.select_window(&primary).unwrap();
    frame(ui);
    windows
}

#[test]
fn a_scope_inside_a_detached_view_is_one_stop_there() {
    for native in [false, true] {
        let mut ui = runtime();
        let root = ui
            .mount_tree(Element::new(
                "tabs",
                Tabs::new([
                    Tab::new("editor", "Editor", scoped_column("page")),
                    Tab::new("other", "Other", button("other-button")),
                ]),
            ))
            .unwrap();
        frame(&mut ui);
        let view = ui.children(&root).unwrap()[0].clone();
        let prepared = ui
            .prepare_detach(&view, NativeWindowOptions::new("Editor").unwrap())
            .unwrap();
        let detached = ui.commit_detach(prepared).unwrap();
        ui.flush_windows(Duration::ZERO).unwrap();
        assert_eq!(ui.window_of(&view).unwrap(), detached);
        let before = ui.find_descendant(&root, "before").unwrap().unwrap();
        ui.focus(&before).unwrap();
        // Focus moved the keyboard to the detached window.
        assert_eq!(ui.current_window(), detached);
        frame(&mut ui);
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
    }
}

#[test]
fn a_view_detached_from_inside_a_scope_keeps_its_stops() {
    for native in [false, true] {
        let mut ui = runtime();
        let root = ui
            .mount_tree(Element::new(
                "root",
                Column::new([
                    button("outside"),
                    Element::new(
                        "scope",
                        Wrapper::new(
                            "scope",
                            Element::new(
                                "tabs",
                                Tabs::new([
                                    Tab::new(
                                        "editor",
                                        "Editor",
                                        Element::new(
                                            "page",
                                            Column::new([button("p"), button("q")]),
                                        ),
                                    ),
                                    Tab::new("other", "Other", button("other-button")),
                                ]),
                            ),
                        )
                        .scope(true),
                    ),
                ]),
            ))
            .unwrap();
        frame(&mut ui);
        // Docked, the scope keeps the tab strip and the page out of traversal.
        let outside = ui.find_descendant(&root, "outside").unwrap().unwrap();
        ui.focus(&outside).unwrap();
        assert_eq!(walk(&mut ui, false, native, 2), ["scope", "outside"]);

        let tabs = ui.find_descendant(&root, "tabs").unwrap().unwrap();
        let view = ui.children(&tabs).unwrap()[0].clone();
        let prepared = ui
            .prepare_detach(&view, NativeWindowOptions::new("Editor").unwrap())
            .unwrap();
        let detached = ui.commit_detach(prepared).unwrap();
        ui.flush_windows(Duration::ZERO).unwrap();
        assert_eq!(ui.window_of(&view).unwrap(), detached);
        let p = ui.find_descendant(&root, "p").unwrap().unwrap();
        ui.focus(&p).unwrap();
        assert_eq!(ui.current_window(), detached);
        frame(&mut ui);
        // The native window root starts a scope of its own.
        assert_eq!(
            walk(&mut ui, false, native, 2),
            ["q", "p"],
            "native {native}"
        );
        assert_eq!(
            walk(&mut ui, true, native, 2),
            ["q", "p"],
            "native {native}"
        );
    }
}

/// A tool window layout whose `files` window holds [`scoped_column`].
fn tool_layout(controller: &ToolWindowController) -> ToolWindowLayout {
    ToolWindowLayout::new(button("main"))
        .edge(ToolEdge::Left, EdgeMode::Stripe)
        .window(ToolWindow::new("files", "Files", scoped_column("page")))
        .controller(controller)
}

#[test]
fn a_scope_inside_a_tool_window_is_one_stop_in_every_mode() {
    for mode in [
        ToolWindowMode::Docked,
        ToolWindowMode::Floating,
        ToolWindowMode::Window,
    ] {
        for native in [false, true] {
            let case = format!("{mode:?}, native {native}");
            let mut ui = runtime();
            let controller = ToolWindowController::new();
            let root = ui
                .mount_tree(Element::new("frame", tool_layout(&controller)))
                .unwrap();
            frame(&mut ui);
            controller.set_mode("files", mode).unwrap();
            controller.expand("files").unwrap();
            frame(&mut ui);
            assert_eq!(controller.mode("files").unwrap(), mode, "{case}");
            let detached = host_detaches(&mut ui);
            let before = ui.find_descendant(&root, "before").unwrap().unwrap();
            if mode == ToolWindowMode::Window {
                assert_eq!(detached.len(), 1, "{case}: the panel was detached");
                assert_eq!(ui.window_of(&before).unwrap(), detached[0], "{case}");
            } else {
                assert!(detached.is_empty(), "{case}");
                assert_eq!(
                    ui.window_of(&before).unwrap(),
                    ui.primary_window(),
                    "{case}"
                );
            }
            let shown = ui
                .inspect()
                .components
                .iter()
                .any(|c| c.key == "a" && c.visible);
            assert!(shown, "{case}: the tool window's content is shown");
            ui.focus(&before).unwrap();
            frame(&mut ui);
            let stops = walk_around(&mut ui, "before", native, 30);
            assert_eq!(
                stops.last().map(String::as_str),
                Some("before"),
                "{case}: {stops:?}"
            );
            assert_eq!(stops[..2], ["scope", "after"], "{case}: {stops:?}");
            assert!(
                !stops.iter().any(|key| key == "a" || key == "b"),
                "{case}: {stops:?}"
            );
        }
    }
}

/// Whether a component keyed `key` is presented.
fn shown(ui: &Runtime, key: &str) -> bool {
    ui.inspect()
        .components
        .iter()
        .any(|component| component.key == key && component.visible)
}

#[test]
fn window_view_and_application_dialogs_opened_inside_a_scope_keep_their_stops() {
    for kind in ["window", "view", "application"] {
        for native in [false, true] {
            let case = format!("{kind} dialog, native {native}");
            let mut ui = runtime();
            let controller = DialogController::new(&ui);
            // A view dialog is clipped to its owner's allocation, so the
            // owner is large enough to show the whole dialog.
            let dialog = Dialog::new(
                controller.clone(),
                Element::new(
                    "anchor",
                    Container::default().preferred_size(Size::from([300, 300])),
                ),
                Element::new("content", Column::new([button("ok"), button("cancel")])),
            );
            let dialog = match kind {
                "view" => dialog.view_modal(),
                "application" => dialog.application_modal(),
                _ => dialog,
            };
            let root = ui
                .mount_tree(Element::new(
                    "root",
                    Column::new([
                        button("before"),
                        Element::new(
                            "scope",
                            Wrapper::new("scope", Element::new("owner", dialog)).scope(true),
                        ),
                    ]),
                ))
                .unwrap();
            frame(&mut ui);
            // Closed, the scope is one stop and the dialog has none.
            let before = ui.find_descendant(&root, "before").unwrap().unwrap();
            ui.focus(&before).unwrap();
            assert_eq!(
                walk(&mut ui, false, native, 2),
                ["scope", "before"],
                "{case}"
            );

            controller.open().unwrap();
            frame(&mut ui);
            assert_eq!(
                controller.status().get().unwrap(),
                DialogStatus::Open,
                "{case}"
            );
            assert!(shown(&ui, "ok") && shown(&ui, "cancel"), "{case}");
            let ok = ui.find_descendant(&root, "ok").unwrap().unwrap();
            ui.focus(&ok).unwrap();
            frame(&mut ui);
            let stops = walk(&mut ui, false, native, 4);
            let back = walk(&mut ui, true, native, 4);
            // Focus inside the modal keeps traversal inside it.
            assert_eq!(stops, ["cancel", "ok", "cancel", "ok"], "{case}");
            assert_eq!(back, ["cancel", "ok", "cancel", "ok"], "{case}");
            if kind == "view" {
                // A view dialog blocks only its owner, so the page outside
                // the scope stays focusable, and traversal from there reaches
                // the dialog in its deferred place.
                ui.focus(&before).unwrap();
                frame(&mut ui);
                assert_eq!(focused(&ui).as_deref(), Some("before"), "{case}");
                assert_eq!(walk(&mut ui, false, native, 2), ["scope", "ok"], "{case}");
            }
        }
    }
}

/// `before`, a focusable scope holding a popover owner and `a`, and `after`.
/// The popover's trigger opens it; its content holds `first` and `second`.
fn scoped_popover(ui: &mut Runtime) -> (WidgetId, Signal<bool>) {
    let open = ui.signal(false);
    let show = open.clone();
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
                                Element::new(
                                    "popover",
                                    Popover::new(
                                        open.clone(),
                                        Element::new(
                                            "trigger",
                                            Button::new("Open", move |_| show.set(true)),
                                        ),
                                        Element::new(
                                            "content",
                                            Column::new([button("first"), button("second")]),
                                        ),
                                    ),
                                ),
                                button("a"),
                            ]),
                        ),
                    )
                    .scope(true),
                ),
                button("after"),
            ]),
        ))
        .unwrap();
    frame(ui);
    (root, open)
}

#[test]
fn a_popover_owned_inside_a_scope_is_reached_in_its_deferred_place_and_closes_when_left() {
    for native in [false, true] {
        for reverse in [false, true] {
            let case = format!("native {native}, reverse {reverse}");
            let mut ui = runtime();
            let (root, open) = scoped_popover(&mut ui);
            // Closed, the trigger is inside the scope and no stop.
            let before = ui.find_descendant(&root, "before").unwrap().unwrap();
            ui.focus(&before).unwrap();
            assert_eq!(
                walk(&mut ui, false, native, 3),
                ["scope", "after", "before"],
                "{case}"
            );

            // Opened from the trigger, which explicit focus still reaches.
            let trigger = ui.find_descendant(&root, "trigger").unwrap().unwrap();
            ui.focus(&trigger).unwrap();
            support::press(&mut ui, KeyCode::Enter);
            frame(&mut ui);
            assert!(open.get().unwrap(), "{case}: opened");
            assert_eq!(focused(&ui).as_deref(), Some("first"), "{case}: admission");
            // The popover is a scope of its own: Tab moves inside it.
            tab(&mut ui, false, native);
            assert_eq!(focused(&ui).as_deref(), Some("second"), "{case}");
            assert!(open.get().unwrap(), "{case}: open while focus stays inside");
            tab(&mut ui, true, native);
            assert_eq!(focused(&ui).as_deref(), Some("first"), "{case}");
            assert!(open.get().unwrap(), "{case}");
            // Leaving: the popup's place is after the ordinary content, so
            // Shift+Tab from its first stop reaches `after` and Tab from its
            // last wraps to `before`.
            if reverse {
                tab(&mut ui, true, native);
                assert_eq!(focused(&ui).as_deref(), Some("after"), "{case}");
            } else {
                tab(&mut ui, false, native);
                tab(&mut ui, false, native);
                assert_eq!(focused(&ui).as_deref(), Some("before"), "{case}");
            }
            assert!(
                !open.get().unwrap(),
                "{case}: dismissed when traversal left"
            );
            assert!(!shown(&ui, "first"), "{case}");

            // A popover is open only while focus is inside it: focus moved
            // to ordinary content dismisses it before the next Tab
            // traverses, so traversal cannot enter it from there.
            let trigger = ui.find_descendant(&root, "trigger").unwrap().unwrap();
            ui.focus(&trigger).unwrap();
            support::press(&mut ui, KeyCode::Enter);
            frame(&mut ui);
            assert!(open.get().unwrap(), "{case}: reopened");
            let after = ui.find_descendant(&root, "after").unwrap().unwrap();
            ui.focus(&after).unwrap();
            assert!(open.get().unwrap(), "{case}: still open before the Tab");
            tab(&mut ui, false, native);
            assert!(!open.get().unwrap(), "{case}: dismissed on departure");
            assert_eq!(focused(&ui).as_deref(), Some("before"), "{case}");
        }
    }
}

/// A component presenting its second child as a tooltip surface while
/// `shown`, as a custom tooltip host would; the tooltip holds a button so
/// that its exclusion from traversal is observable.
struct TooltipHost {
    children: Vec<Element>,
}
impl Component for TooltipHost {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn child_surface(&self, key: &str) -> Option<SurfaceKind> {
        (key == "tip").then_some(SurfaceKind::Tooltip)
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(100.0, 32.0)?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        for index in 0..cx.len() {
            let y = if index == 0 { 0.0 } else { size.height() };
            cx.place(index, Rect::new(0.0, y, 100.0, 32.0)?)?;
        }
        Ok(())
    }
}

#[test]
fn tooltips_stay_out_of_traversal_inside_and_outside_a_scope() {
    let tip = |key: &str| {
        Element::new(
            key,
            TooltipHost {
                children: vec![
                    button(&format!("{key}-anchor")),
                    Element::new("tip", Column::new([button(&format!("{key}-tip"))])),
                ],
            },
        )
    };
    for native in [false, true] {
        let mut ui = runtime();
        let root = ui
            .mount_tree(Element::new(
                "root",
                Column::new([
                    button("before"),
                    tip("plain"),
                    Element::new("scope", Wrapper::new("scope", tip("scoped")).scope(true)),
                    button("after"),
                ]),
            ))
            .unwrap();
        frame(&mut ui);
        // The tooltips are presented, and their buttons are focusable.
        assert!(shown(&ui, "plain-tip") && shown(&ui, "scoped-tip"));
        let before = ui.find_descendant(&root, "before").unwrap().unwrap();
        ui.focus(&before).unwrap();
        assert_eq!(
            walk(&mut ui, false, native, 4),
            ["plain-anchor", "scope", "after", "before"],
            "native {native}"
        );
        assert_eq!(
            walk(&mut ui, true, native, 4),
            ["after", "scope", "plain-anchor", "before"],
            "native {native}"
        );
    }
}

/// Presses and releases `code` on the headless or the native path.
fn stroke(ui: &mut Runtime, code: KeyCode, native: bool) {
    for action in [KeyAction::Press, KeyAction::Release] {
        if native {
            ui.command_key_input(CommandKeyInput::new(code, action, CommandKey::Named(code)))
                .unwrap();
        } else {
            ui.key_input(code, action).unwrap();
        }
    }
    frame(ui);
}

fn menu_bar(ui: &Runtime, content: Element) -> MenuBar {
    MenuBar::new(
        ui,
        vec![
            MenuItem::submenu(
                "file",
                "File",
                vec![MenuItem::new("new", "New", |_| Ok(()))],
            ),
            MenuItem::submenu(
                "edit",
                "Edit",
                vec![MenuItem::new("undo", "Undo", |_| Ok(()))],
            ),
        ],
    )
    .unwrap()
    .content(content)
}

#[test]
fn the_menu_bar_and_f10_are_unchanged_with_a_scope_active() {
    for native in [false, true] {
        // A scope in the bar's content.
        let mut ui = runtime();
        let bar = menu_bar(&ui, scoped_column("page"));
        let root = ui.mount_tree(Element::new("bar", bar)).unwrap();
        frame(&mut ui);
        let before = ui.find_descendant(&root, "before").unwrap().unwrap();
        ui.focus(&before).unwrap();
        // The titles are no stops at rest.
        assert_eq!(
            walk(&mut ui, false, native, 3),
            ["scope", "after", "before"],
            "native {native}"
        );
        // F10 enters menu mode from scoped content and returns focus to it.
        let a = ui.find_descendant(&root, "a").unwrap().unwrap();
        ui.focus(&a).unwrap();
        stroke(&mut ui, KeyCode::F10, native);
        assert_eq!(
            focused(&ui).as_deref(),
            Some("title/file"),
            "native {native}"
        );
        stroke(&mut ui, KeyCode::ArrowRight, native);
        assert_eq!(
            focused(&ui).as_deref(),
            Some("title/edit"),
            "native {native}"
        );
        stroke(&mut ui, KeyCode::F10, native);
        assert_eq!(focused(&ui).as_deref(), Some("a"), "native {native}");
        // Tab in menu mode leaves the titles for the next stop and ends it.
        stroke(&mut ui, KeyCode::F10, native);
        assert_eq!(
            focused(&ui).as_deref(),
            Some("title/file"),
            "native {native}"
        );
        tab(&mut ui, false, native);
        assert_eq!(focused(&ui).as_deref(), Some("before"), "native {native}");
        assert_eq!(
            walk(&mut ui, false, native, 3),
            ["scope", "after", "before"],
            "native {native}"
        );

        // The whole bar inside a scope: F10 still reaches the titles, and
        // traversal from a title continues around the scope.
        let mut ui = runtime();
        let bar = menu_bar(
            &ui,
            Element::new("inner", Column::new([button("a"), button("b")])),
        );
        let root = ui
            .mount_tree(Element::new(
                "root",
                Column::new([
                    button("outside"),
                    Element::new(
                        "scope",
                        Wrapper::new("scope", Element::new("bar", bar)).scope(true),
                    ),
                    button("after"),
                ]),
            ))
            .unwrap();
        frame(&mut ui);
        let a = ui.find_descendant(&root, "a").unwrap().unwrap();
        for (reverse, landed) in [(false, "after"), (true, "scope")] {
            ui.focus(&a).unwrap();
            stroke(&mut ui, KeyCode::F10, native);
            assert_eq!(
                focused(&ui).as_deref(),
                Some("title/file"),
                "native {native}"
            );
            tab(&mut ui, reverse, native);
            assert_eq!(focused(&ui).as_deref(), Some(landed), "native {native}");
        }
        assert_eq!(
            walk(&mut ui, false, native, 3),
            ["after", "outside", "scope"],
            "native {native}"
        );
    }
}

/// Sends one Tab transition on the headless or the native path.
fn tab_action(ui: &mut Runtime, action: KeyAction, native: bool) {
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
    frame(ui);
}

/// A claimer around `field`, then `after` and `last`; returns the claimer's
/// offer log.
fn repeat_page(ui: &mut Runtime, passes_repeats: bool) -> (WidgetId, Rc<RefCell<Vec<bool>>>) {
    let claims = Rc::new(RefCell::new(Vec::new()));
    let offers = Rc::new(RefCell::new(Vec::new()));
    let mut claimer = Wrapper::new("claimer", button("field")).claims(&claims);
    claimer.offers = Some(offers.clone());
    claimer.passes_repeats = passes_repeats;
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new("claimer", claimer),
                button("after"),
                button("last"),
            ]),
        ))
        .unwrap();
    frame(ui);
    let field = ui.find_descendant(&root, "field").unwrap().unwrap();
    ui.focus(&field).unwrap();
    (root, offers)
}

#[test]
fn tab_repeats_are_offered_like_presses_and_follow_the_focus() {
    for native in [false, true] {
        // Every repeat of a claimed Tab is offered again, as a repeat, and
        // claimed: focus never moves.
        let mut ui = runtime();
        let (_, offers) = repeat_page(&mut ui, false);
        tab_action(&mut ui, KeyAction::Press, native);
        tab_action(&mut ui, KeyAction::Repeat, native);
        tab_action(&mut ui, KeyAction::Repeat, native);
        assert_eq!(*offers.borrow(), [false, true, true], "native {native}");
        assert_eq!(focused(&ui).as_deref(), Some("field"), "native {native}");
        tab_action(&mut ui, KeyAction::Release, native);
        assert_eq!(focused(&ui).as_deref(), Some("field"), "native {native}");
        assert_eq!(offers.borrow().len(), 3, "a release is never offered");

        // A claimer that lets repeats pass: the claimed press stays put, the
        // first repeat traverses, and the next repeat is no longer offered
        // to it because focus has left it.
        let mut ui = runtime();
        let (_, offers) = repeat_page(&mut ui, true);
        tab_action(&mut ui, KeyAction::Press, native);
        assert_eq!(focused(&ui).as_deref(), Some("field"), "native {native}");
        tab_action(&mut ui, KeyAction::Repeat, native);
        assert_eq!(focused(&ui).as_deref(), Some("after"), "native {native}");
        tab_action(&mut ui, KeyAction::Repeat, native);
        assert_eq!(focused(&ui).as_deref(), Some("last"), "native {native}");
        tab_action(&mut ui, KeyAction::Release, native);
        assert_eq!(*offers.borrow(), [false, true], "native {native}");

        // An orphan repeat, with no press held, is neither offered nor
        // traverses.
        let mut ui = runtime();
        let (_, offers) = repeat_page(&mut ui, false);
        let result = if native {
            ui.command_key_input(CommandKeyInput::new(
                KeyCode::Tab,
                KeyAction::Repeat,
                CommandKey::Named(KeyCode::Tab),
            ))
            .map(|_| ())
        } else {
            ui.key_input(KeyCode::Tab, KeyAction::Repeat).map(|_| ())
        };
        assert_eq!(result, Ok(()), "native {native}");
        frame(&mut ui);
        assert!(offers.borrow().is_empty(), "native {native}");
        assert_eq!(focused(&ui).as_deref(), Some("field"), "native {native}");
    }
}

/// Presents its second child as a nonmodal surface of `kind` while `open`
/// holds; any dismissal closes it, as the surface contract requires of a
/// nonmodal owner, and is logged.
struct SurfaceHost {
    kind: SurfaceKind,
    open: Signal<bool>,
    shown: bool,
    dismissals: Rc<RefCell<Vec<SurfaceDismissal>>>,
    children: Vec<Element>,
}
impl Component for SurfaceHost {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.shown = cx.read(&self.open)?;
        Ok(())
    }
    fn child_visible(&self, key: &str) -> bool {
        key != "surface" || self.shown
    }
    fn child_surface(&self, key: &str) -> Option<SurfaceKind> {
        (key == "surface").then_some(self.kind)
    }
    fn dismiss_surface(
        &mut self,
        _: &mut EventContext,
        reason: SurfaceDismissal,
    ) -> Result<(), RuntimeError> {
        self.dismissals.borrow_mut().push(reason);
        self.open.set(false)?;
        self.shown = false;
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(100.0, 32.0)?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        for index in 0..cx.len() {
            let y = if index == 0 { 0.0 } else { size.height() };
            cx.place(index, Rect::new(0.0, y, 100.0, 64.0)?)?;
        }
        Ok(())
    }
}

#[test]
fn a_notification_owned_inside_a_scope_is_entered_in_its_deferred_place() {
    for native in [false, true] {
        for reverse in [false, true] {
            let case = format!("native {native}, reverse {reverse}");
            let mut ui = runtime();
            let open = ui.signal(false);
            let dismissals = Rc::new(RefCell::new(Vec::new()));
            let host = SurfaceHost {
                kind: SurfaceKind::Notification,
                open: open.clone(),
                shown: false,
                dismissals: dismissals.clone(),
                children: vec![
                    button("owner-anchor"),
                    Element::new("surface", Column::new([button("first"), button("second")])),
                ],
            };
            let root = ui
                .mount_tree(Element::new(
                    "root",
                    Column::new([
                        button("before"),
                        Element::new(
                            "scope",
                            Wrapper::new("scope", Element::new("host", host)).scope(true),
                        ),
                        button("after"),
                    ]),
                ))
                .unwrap();
            frame(&mut ui);
            // Closed, the scope is one stop and its owner's anchor is none.
            let before = ui.find_descendant(&root, "before").unwrap().unwrap();
            ui.focus(&before).unwrap();
            assert_eq!(
                walk(&mut ui, false, native, 3),
                ["scope", "after", "before"],
                "{case}"
            );
            // Admission leaves focus in the ordinary content.
            let start = if reverse { "before" } else { "after" };
            let start = ui.find_descendant(&root, start).unwrap().unwrap();
            ui.focus(&start).unwrap();
            open.set(true).unwrap();
            frame(&mut ui);
            assert!(shown(&ui, "first") && shown(&ui, "second"), "{case}");
            assert_eq!(ui.focused(), Some(start), "{case}");
            // Traversal enters the surface after the ordinary content: Tab
            // from `after` reaches its first stop, and Shift+Tab from
            // `before`, the stop after its place once the order wraps,
            // reaches its last.
            let stops = walk(&mut ui, reverse, native, 6);
            if reverse {
                assert_eq!(
                    stops,
                    ["second", "first", "after", "scope", "before", "second"],
                    "{case}"
                );
            } else {
                assert_eq!(
                    stops,
                    ["first", "second", "before", "scope", "after", "first"],
                    "{case}"
                );
            }
            // A notification is not dismissed by focus leaving it.
            assert!(open.get().unwrap(), "{case}");
            assert!(dismissals.borrow().is_empty(), "{case}");
        }
    }
}
