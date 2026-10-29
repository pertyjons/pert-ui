//! `MultiSelect` highlight ownership between the pointer and the keyboard, and
//! the return of focus to the search field after a completed row click.
//!
//! Only the person's own pointer motion or press over a row hands the
//! highlight to the pointer; layout moving rows under a stationary pointer,
//! a repeated host position, the wheel and the list's own scroll keys never
//! take it from the keyboard. Every case drives public input and checks the
//! highlighted row together with the values the chooser writes and reports.
use crate::support;
use pert_ui::testing::HostIntegration;
use pert_ui::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use support::{frame, widget};

use MultiSelectCommit::{EachToggle, OnClose};

/// What the fixture's `on_change` handler does after recording the report.
#[derive(Clone, Copy, PartialEq)]
enum Handler {
    Record,
    Fail,
    FocusAfter,
}

type Reports = Rc<RefCell<Vec<Vec<String>>>>;
type Filter = Rc<dyn Fn(&str, &Choice<String>) -> bool>;

/// `O01`…: options labelled `Onn` with the values `onn`.
fn choices(numbers: impl IntoIterator<Item = usize>) -> Vec<Choice<String>> {
    numbers
        .into_iter()
        .map(|n| Choice::new(format!("o{n:02}"), format!("O{n:02}")))
        .collect()
}

fn values(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

/// The application side of the fixture: the bindings the chooser is
/// described with and what its handler records and does.
#[derive(Clone)]
struct Parts {
    value: Signal<Vec<String>>,
    options: Signal<Vec<Choice<String>>>,
    allowed: Signal<bool>,
    reports: Reports,
    handler: Rc<Cell<Handler>>,
    after: Rc<RefCell<Option<WidgetId>>>,
    commit: MultiSelectCommit,
    /// Filler lines after the button; enough of them make the page scroll.
    filler: usize,
}

impl Parts {
    fn chooser(&self, ui: &Runtime, filter: Option<Filter>) -> MultiSelect<String> {
        let (reports, handler, after) = (
            self.reports.clone(),
            self.handler.clone(),
            self.after.clone(),
        );
        let chooser = MultiSelect::bound(ui, self.value.clone(), self.options.read_only())
            .commit(self.commit)
            .enabled(&self.allowed)
            .on_change(move |cx, values| {
                reports.borrow_mut().push(values.to_vec());
                match handler.get() {
                    Handler::Record => Ok(()),
                    Handler::Fail => Err(RuntimeError::BorrowConflict),
                    Handler::FocusAfter => {
                        cx.request_focus_for(after.borrow().as_ref().expect("after is mounted"))
                    }
                }
            });
        match filter {
            Some(filter) => chooser.filter(move |query, choice| filter(query, choice)),
            None => chooser,
        }
    }

    /// The form's children: a lead, the chooser unless removed, a button and
    /// the filler.
    fn form(&self, ui: &Runtime, filter: Option<Filter>, with_chooser: bool) -> Vec<Element> {
        let lead =
            Column::new((0..3).map(|n| Element::new(format!("lead{n}"), Label::new("Lead"))));
        let filler = Column::new(
            (0..self.filler).map(|n| Element::new(format!("line{n}"), Label::new("Filler"))),
        );
        let mut children = vec![Element::new("lead", lead)];
        if with_chooser {
            children.push(Element::new("tags", self.chooser(ui, filter)));
        }
        children.push(Element::new("after", Button::new("After", |_| Ok(()))));
        children.push(Element::new("filler", filler));
        children
    }
}

/// One composed root: a scroll area whose form holds the chooser over bound
/// options and a bound permission. The page fits unless a test asks for
/// filler. Only the wheel test needs an overflowing page. The separate
/// popup_press_focus tests cover the surface boundary that keeps a short-list
/// press from transferring focus to this outside ancestor.
struct Fx {
    ui: Runtime,
    root: WidgetId,
    p: Parts,
}

impl Fx {
    fn new(options: Vec<Choice<String>>, commit: MultiSelectCommit) -> Self {
        Self::with_filler(options, commit, 0)
    }

    fn with_filler(options: Vec<Choice<String>>, commit: MultiSelectCommit, filler: usize) -> Self {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(480.0, 560.0, 1.0).unwrap())
            .unwrap();
        let p = Parts {
            value: ui.signal(Vec::new()),
            options: ui.signal(options),
            allowed: ui.signal(true),
            reports: Reports::default(),
            handler: Rc::new(Cell::new(Handler::Record)),
            after: Rc::new(RefCell::new(None)),
            commit,
            filler,
        };
        let form = Element::new("form", Column::new(p.form(&ui, None, true)));
        let root = ui
            .mount_tree(Element::new("page", ScrollArea::new(form)))
            .unwrap();
        frame(&mut ui);
        *p.after.borrow_mut() = Some(widget(&ui, &root, "after"));
        Self { ui, root, p }
    }

    fn rows(count: usize, commit: MultiSelectCommit) -> Self {
        Self::new(choices(1..=count), commit)
    }

    /// Describes the chooser again under the same key, with `filter`.
    fn redescribe(&mut self, filter: Option<Filter>) {
        let form = widget(&self.ui, &self.root, "form");
        let children = self.p.form(&self.ui, filter, true);
        self.ui.reconcile_children(&form, children).unwrap();
        frame(&mut self.ui);
    }

    /// Removes the chooser from the form.
    fn remove_chooser(&mut self) {
        let form = widget(&self.ui, &self.root, "form");
        let children = self.p.form(&self.ui, None, false);
        self.ui.reconcile_children(&form, children).unwrap();
        frame(&mut self.ui);
    }

    fn tags(&self) -> WidgetId {
        widget(&self.ui, &self.root, "tags")
    }

    fn list(&self) -> WidgetId {
        widget(&self.ui, &self.root, "list")
    }

    fn search(&self) -> WidgetId {
        widget(&self.ui, &self.root, "search")
    }

    fn search_details(&self) -> ComponentDetails {
        let search = self.ui.inspection_id(&self.search()).unwrap();
        self.ui
            .inspect()
            .components
            .into_iter()
            .find(|c| c.id == search)
            .map(|c| c.details)
            .unwrap()
    }

    fn open(&mut self) {
        let trigger = widget(&self.ui, &self.root, "anchor");
        self.ui.focus(&trigger).unwrap();
        frame(&mut self.ui);
        self.key(KeyCode::Enter);
        assert!(self.is_open(), "the trigger opens the popup");
        assert_eq!(self.focus().as_deref(), Some("search"));
    }

    fn is_open(&self) -> bool {
        let tags = self.tags();
        self.ui
            .inspect()
            .components
            .into_iter()
            .find(|c| c.key == "tags" && self.ui.inspection_id(&tags).ok() == Some(c.id))
            .is_some_and(|c| c.details.properties["open"] == InspectValue::Bool(true))
    }

    fn key(&mut self, code: KeyCode) {
        support::key(&mut self.ui, code);
        frame(&mut self.ui);
    }

    /// Holds `code` through `repeats` repeats, then releases it.
    fn hold(&mut self, code: KeyCode, repeats: usize) {
        self.ui.key_input(code, KeyAction::Press).unwrap();
        for _ in 0..repeats {
            self.ui.key_input(code, KeyAction::Repeat).unwrap();
        }
        self.ui.key_input(code, KeyAction::Release).unwrap();
        frame(&mut self.ui);
    }

    fn shift_tab(&mut self) {
        self.ui
            .key_input(KeyCode::ShiftLeft, KeyAction::Press)
            .unwrap();
        self.key(KeyCode::Tab);
        self.ui
            .key_input(KeyCode::ShiftLeft, KeyAction::Release)
            .unwrap();
        frame(&mut self.ui);
    }

    fn pointer(&mut self, point: Point, kind: PointerKind) -> Result<InputOutcome, RuntimeError> {
        self.ui.pointer_input(PointerEvent::new(point, kind))
    }

    /// A host move to `point`.
    fn hover(&mut self, point: Point) {
        self.pointer(point, PointerKind::Move).unwrap();
        frame(&mut self.ui);
    }

    /// The host reports the pointer leaving the window at `point`.
    fn leave_window(&mut self, point: Point) {
        self.pointer(point, PointerKind::Leave).unwrap();
        frame(&mut self.ui);
    }

    fn wheel(&mut self, point: Point, down: f32) {
        self.pointer(point, PointerKind::Scroll(Point::new(0.0, down).unwrap()))
            .unwrap();
        frame(&mut self.ui);
    }

    fn press(&mut self, point: Point) {
        self.pointer(point, PointerKind::Press(PointerButton::Primary))
            .unwrap();
        frame(&mut self.ui);
    }

    fn release(&mut self, point: Point) {
        self.pointer(point, PointerKind::Release(PointerButton::Primary))
            .unwrap();
        frame(&mut self.ui);
    }

    /// A host move to `point`, then a primary click there.
    fn click(&mut self, point: Point) {
        self.hover(point);
        self.press(point);
        self.release(point);
    }

    fn text_of(details: &ComponentDetails) -> Option<String> {
        match details.properties.get("text") {
            Some(InspectValue::Text(text)) => Some(text.clone()),
            _ => None,
        }
    }

    /// The visible rows: (label, highlighted).
    fn shown(&self) -> Vec<(String, bool)> {
        self.ui
            .inspect()
            .components
            .iter()
            .filter(|c| c.visible && c.details.properties.contains_key("checked"))
            .map(|c| {
                (
                    Self::text_of(&c.details).unwrap(),
                    c.details.properties["highlighted"] == InspectValue::Bool(true),
                )
            })
            .collect()
    }

    fn highlighted(&self) -> Option<String> {
        self.shown()
            .into_iter()
            .find(|(_, highlighted)| *highlighted)
            .map(|(label, _)| label)
    }

    fn row(&self, label: &str) -> WidgetId {
        let key = self
            .ui
            .inspect()
            .components
            .into_iter()
            .find(|c| {
                c.details.properties.contains_key("checked")
                    && Self::text_of(&c.details).as_deref() == Some(label)
            })
            .map(|c| c.key)
            .unwrap_or_else(|| panic!("no row {label}"));
        self.ui
            .find_descendant(&self.list(), &key)
            .unwrap()
            .unwrap()
    }

    fn bounds(&self, id: &WidgetId) -> Rect {
        self.ui.bounds(id).unwrap().unwrap()
    }

    fn center(&self, label: &str) -> Point {
        let b = self.bounds(&self.row(label));
        Point::new(b.x() + b.width() / 2.0, b.y() + b.height() / 2.0).unwrap()
    }

    /// A point inside the row, `dx` right of its center.
    fn beside(&self, label: &str, dx: f32) -> Point {
        let c = self.center(label);
        Point::new(c.x() + dx, c.y()).unwrap()
    }

    /// A point two pixels above the row's bottom edge, which any scroll down
    /// by at least that much moves out of the row.
    fn near_bottom(&self, label: &str) -> Point {
        let b = self.bounds(&self.row(label));
        Point::new(b.x() + b.width() / 2.0, b.y() + b.height() - 2.0).unwrap()
    }

    /// The label of the row painted under `point`, inside the list's clip.
    fn row_at(&self, point: Point) -> Option<String> {
        if !self.bounds(&self.list()).contains(point) {
            return None;
        }
        self.shown()
            .into_iter()
            .map(|(label, _)| label)
            .find(|label| self.bounds(&self.row(label)).contains(point))
    }

    fn row_height(&self) -> f32 {
        let (first, _) = self.shown().into_iter().next().unwrap();
        self.bounds(&self.row(&first)).height()
    }

    fn focus(&self) -> Option<String> {
        self.ui
            .focused()
            .map(|id| self.ui.key(&id).unwrap().to_owned())
    }

    /// The search field's text, as its semantics report it.
    fn query(&self) -> Option<SemanticValue> {
        self.search_details().semantics.and_then(|s| s.value)
    }

    fn composing(&self) -> bool {
        self.search_details().properties.get("composing") == Some(&InspectValue::Bool(true))
    }

    fn value(&self) -> Vec<String> {
        self.p.value.get().unwrap()
    }

    fn reports(&self) -> Vec<Vec<String>> {
        self.p.reports.borrow().clone()
    }
}

// ---- A stationary pointer never takes the highlight from the keyboard ----

#[test]
fn a_pointer_parked_where_the_popup_opens_leaves_the_keyboard_highlight_alone() {
    let mut fx = Fx::rows(4, EachToggle);
    fx.open();
    let gamma = fx.center("O03");
    fx.key(KeyCode::Escape);
    assert!(!fx.is_open());
    // The pointer rests where O03's row (P13's Gamma) will appear.
    fx.hover(gamma);
    fx.open();
    assert_eq!(fx.row_at(gamma).as_deref(), Some("O03"));
    assert_eq!(fx.highlighted().as_deref(), Some("O01"));
    fx.key(KeyCode::ArrowDown);
    assert_eq!(fx.highlighted().as_deref(), Some("O02"));
    // A host repeating the same position is still a stationary pointer.
    fx.hover(gamma);
    assert_eq!(fx.highlighted().as_deref(), Some("O02"));
    fx.key(KeyCode::Enter);
    assert_eq!(fx.value(), values(&["o02"]));
    assert_eq!(fx.reports(), [values(&["o02"])]);
}

#[test]
fn a_long_list_under_a_parked_pointer_follows_every_navigation_key() {
    for leave in [false, true] {
        let mut fx = Fx::rows(20, EachToggle);
        fx.open();
        let parked = fx.center("O03");
        fx.key(KeyCode::Escape);
        fx.hover(parked);
        if leave {
            // Control: the pointer left the window before the popup opened.
            fx.leave_window(parked);
        }
        fx.open();
        assert_eq!(fx.highlighted().as_deref(), Some("O01"));
        for _ in 0..5 {
            fx.key(KeyCode::ArrowDown);
        }
        // A held Down: its press and six repeats move seven more.
        fx.hold(KeyCode::ArrowDown, 6);
        assert_eq!(fx.highlighted().as_deref(), Some("O13"), "leave={leave}");
        if !leave {
            assert_ne!(
                fx.row_at(parked).as_deref(),
                Some("O03"),
                "the list scrolled"
            );
        }
        fx.key(KeyCode::Enter);
        fx.key(KeyCode::PageUp);
        assert_eq!(fx.highlighted().as_deref(), Some("O05"));
        fx.key(KeyCode::PageDown);
        assert_eq!(fx.highlighted().as_deref(), Some("O13"));
        fx.key(KeyCode::ArrowUp);
        if !leave {
            // The host repeats the parked position. (After the control's
            // Leave the same move would be a re-entry over a row that never
            // saw the position, which counts as motion.)
            fx.hover(parked);
        }
        assert_eq!(fx.highlighted().as_deref(), Some("O12"), "leave={leave}");
        fx.key(KeyCode::Enter);
        assert_eq!(fx.value(), values(&["o13", "o12"]), "leave={leave}");
        assert_eq!(fx.reports().len(), 2);
    }
}

#[test]
fn genuine_motion_after_keyboard_navigation_takes_the_highlight() {
    let mut fx = Fx::rows(6, EachToggle);
    fx.open();
    fx.key(KeyCode::ArrowDown);
    fx.key(KeyCode::ArrowDown);
    assert_eq!(fx.highlighted().as_deref(), Some("O03"));
    let first = fx.center("O01");
    fx.hover(first);
    assert_eq!(fx.highlighted().as_deref(), Some("O01"));
    fx.hover(fx.beside("O01", 6.0));
    assert_eq!(fx.highlighted().as_deref(), Some("O01"));
    // Row to row: the second row has not seen this position.
    let second = fx.center("O02");
    fx.hover(second);
    assert_eq!(fx.highlighted().as_deref(), Some("O02"));
    fx.key(KeyCode::ArrowDown);
    assert_eq!(fx.highlighted().as_deref(), Some("O03"));
    // Over the search field and back to the very same position in the row.
    let search = fx.bounds(&fx.search());
    fx.hover(Point::new(search.x() + 10.0, search.y() + 5.0).unwrap());
    assert_eq!(fx.highlighted().as_deref(), Some("O03"));
    fx.hover(second);
    assert_eq!(fx.highlighted().as_deref(), Some("O02"));
    fx.key(KeyCode::Enter);
    assert_eq!(fx.value(), values(&["o02"]));
    assert_eq!(fx.reports(), [values(&["o02"])]);
}

#[test]
fn motion_inside_the_row_the_keyboard_highlighted_takes_ownership() {
    let mut fx = Fx::rows(20, EachToggle);
    fx.open();
    let parked = fx.center("O02");
    fx.key(KeyCode::Escape);
    fx.hover(parked);
    fx.open();
    fx.key(KeyCode::ArrowDown);
    assert_eq!(fx.highlighted().as_deref(), Some("O02"));
    assert_eq!(fx.row_at(parked).as_deref(), Some("O02"));
    // Genuine motion over the row that already has the highlight.
    let at = fx.beside("O02", 6.0);
    fx.hover(at);
    fx.wheel(at, 2.0 * fx.row_height());
    let under = fx.row_at(at).unwrap();
    assert_ne!(under, "O02", "the wheel scrolled the list");
    assert_eq!(fx.highlighted(), Some(under.clone()), "the pointer owns it");
    fx.key(KeyCode::Enter);
    assert_eq!(fx.value(), [under.to_lowercase()]);
}

// ---- The wheel and the list's own keys take no new owner ----

#[test]
fn the_wheel_keeps_a_keyboard_highlight_and_moves_a_pointer_highlight() {
    for pointer in [false, true] {
        let mut fx = Fx::rows(20, EachToggle);
        fx.open();
        let at = fx.center("O02");
        fx.hover(at);
        assert_eq!(fx.highlighted().as_deref(), Some("O02"));
        if !pointer {
            fx.key(KeyCode::ArrowDown);
        }
        let expected_before = if pointer { "O02" } else { "O03" };
        assert_eq!(fx.highlighted().as_deref(), Some(expected_before));
        fx.wheel(at, 2.0 * fx.row_height());
        let under = fx.row_at(at).unwrap();
        assert_eq!(under, "O04", "pointer={pointer}");
        let expected = if pointer { "O04" } else { "O03" };
        assert_eq!(
            fx.highlighted().as_deref(),
            Some(expected),
            "pointer={pointer}"
        );
        fx.key(KeyCode::Enter);
        assert_eq!(fx.value(), [expected.to_lowercase()], "pointer={pointer}");
        assert_eq!(fx.reports().len(), 1);
    }
}

#[test]
fn a_wheel_at_the_list_limit_bubbles_without_taking_the_highlight() {
    let mut fx = Fx::with_filler(choices(1..=20), EachToggle, 40);
    fx.open();
    // At the list's end, twelve rows down, this is two pixels above the
    // bottom of O15, so the page scrolling the popup up puts O16 there.
    let at = fx.near_bottom("O03");
    fx.hover(at);
    for _ in 0..3 {
        fx.key(KeyCode::PageDown);
    }
    assert_eq!(fx.highlighted().as_deref(), Some("O20"));
    let before = fx.row_at(at);
    let page_before = fx.bounds(&fx.tags());
    // The list is at its end, so the wheel reaches the scrolling page.
    fx.wheel(at, 12.0);
    let page_after = fx.bounds(&fx.tags());
    assert!(fx.is_open());
    assert!(
        page_after.y() < page_before.y(),
        "the page scrolled: {page_before:?} -> {page_after:?}"
    );
    assert_ne!(fx.row_at(at), before, "another row is under the pointer");
    assert_eq!(fx.highlighted().as_deref(), Some("O20"));
    fx.key(KeyCode::Enter);
    assert_eq!(fx.value(), values(&["o20"]));
}

#[test]
fn the_focused_list_scroll_keys_follow_the_wheel_policy() {
    for pointer in [false, true] {
        let mut fx = Fx::rows(20, EachToggle);
        fx.open();
        let at = fx.center("O02");
        fx.hover(at);
        if !pointer {
            fx.key(KeyCode::ArrowUp);
        }
        let owner = if pointer { "O02" } else { "O01" };
        assert_eq!(fx.highlighted().as_deref(), Some(owner));
        // Tab moves to the overflowing list, whose own keys scroll it.
        fx.key(KeyCode::Tab);
        assert_eq!(fx.focus().as_deref(), Some("list"));
        fx.key(KeyCode::ArrowDown);
        assert_eq!(
            fx.row_at(at).as_deref(),
            Some("O03"),
            "the list scrolled a row"
        );
        fx.key(KeyCode::PageDown);
        let under = fx.row_at(at).unwrap();
        assert_ne!(under, "O03", "the list scrolled a page");
        if pointer {
            assert_eq!(
                fx.highlighted(),
                Some(under.clone()),
                "rows follow the pointer"
            );
        } else {
            assert_ne!(
                fx.highlighted(),
                Some(under.clone()),
                "the keyboard keeps it"
            );
        }
        assert!(fx.value().is_empty(), "scrolling toggles nothing");
        fx.shift_tab();
        assert_eq!(fx.focus().as_deref(), Some("search"));
        fx.key(KeyCode::Enter);
        let chosen = if pointer {
            under.to_lowercase()
        } else {
            "o01".into()
        };
        assert_eq!(fx.value(), [chosen], "pointer={pointer}");
    }
}

// ---- Data, labels, filters and the query ----

fn accept_all() -> Filter {
    Rc::new(|_: &str, _: &Choice<String>| true)
}

fn without_o06() -> Filter {
    Rc::new(|_: &str, choice: &Choice<String>| choice.value() != "o06")
}

#[test]
fn changed_options_labels_and_filters_keep_the_current_owner() {
    for pointer in [false, true] {
        let mut fx = Fx::rows(6, EachToggle);
        fx.open();
        let at = fx.center("O03");
        fx.hover(at);
        if !pointer {
            fx.key(KeyCode::ArrowDown);
            fx.key(KeyCode::ArrowDown);
        }
        let owner = if pointer { "O03" } else { "O05" };
        assert_eq!(fx.highlighted().as_deref(), Some(owner));
        // Reversed options put O04 under the pointer.
        fx.p.options.set(choices((1..=6).rev())).unwrap();
        frame(&mut fx.ui);
        assert_eq!(fx.row_at(at).as_deref(), Some("O04"));
        let expected = if pointer { "O04" } else { "O05" };
        assert_eq!(
            fx.highlighted().as_deref(),
            Some(expected),
            "pointer={pointer}"
        );
        // A relabelled option keeps its row and the highlight by value.
        let mut relabelled = choices((1..=6).rev());
        relabelled[2] = Choice::new("o04".to_owned(), "Fourth");
        fx.p.options.set(relabelled).unwrap();
        frame(&mut fx.ui);
        let expected = if pointer { "Fourth" } else { "O05" };
        assert_eq!(
            fx.highlighted().as_deref(),
            Some(expected),
            "pointer={pointer}"
        );
        // Describing the chooser again replaces its filter; that is not a
        // keyboard action either.
        fx.redescribe(Some(accept_all()));
        assert_eq!(
            fx.highlighted().as_deref(),
            Some(expected),
            "pointer={pointer}"
        );
        fx.redescribe(Some(without_o06()));
        assert_eq!(fx.row_at(at).as_deref(), Some("O03"), "the rows moved up");
        let expected = if pointer { "O03" } else { "O05" };
        assert_eq!(
            fx.highlighted().as_deref(),
            Some(expected),
            "pointer={pointer}"
        );
        fx.key(KeyCode::Enter);
        assert_eq!(fx.value(), [expected.to_lowercase()], "pointer={pointer}");
        assert_eq!(fx.reports().len(), 1);
    }
}

#[test]
fn editing_or_clearing_the_query_hands_the_highlight_back_to_the_keyboard() {
    let mut fx = Fx::rows(20, EachToggle);
    fx.open();
    let at = fx.center("O03");
    fx.hover(at);
    assert_eq!(fx.highlighted().as_deref(), Some("O03"));
    fx.ui.text_input("1").unwrap();
    frame(&mut fx.ui);
    assert_eq!(fx.query(), Some(SemanticValue::Text("1".into())));
    assert_eq!(fx.row_at(at).as_deref(), Some("O11"), "the matches moved");
    // O03 no longer matches, so the first match is highlighted.
    assert_eq!(fx.highlighted().as_deref(), Some("O01"));
    fx.key(KeyCode::Enter);
    assert_eq!(fx.value(), values(&["o01"]));
    // The pointer takes the highlight again by moving, then Escape clears the
    // query and every option moves back under it.
    let ten = fx.center("O10");
    fx.hover(ten);
    assert_eq!(fx.highlighted().as_deref(), Some("O10"));
    fx.key(KeyCode::Escape);
    assert!(fx.is_open(), "the first Escape only clears the query");
    assert_eq!(fx.query(), Some(SemanticValue::Text(String::new())));
    assert_eq!(fx.row_at(ten).as_deref(), Some("O02"));
    assert_ne!(fx.highlighted().as_deref(), Some("O02"), "O10 keeps it");
    fx.key(KeyCode::Enter);
    assert_eq!(fx.value(), values(&["o01", "o10"]));
    assert_eq!(fx.reports().len(), 2);
}

// ---- Leaving the panel and the window ----

#[test]
fn a_pointer_that_left_the_panel_takes_nothing_when_the_popup_grows_under_it() {
    let mut fx = Fx::rows(3, EachToggle);
    fx.open();
    fx.hover(fx.center("O02"));
    assert_eq!(fx.highlighted().as_deref(), Some("O02"));
    // Just below the popup, over the page.
    let popup = fx.bounds(&widget(&fx.ui, &fx.root, "popup"));
    let below = Point::new(fx.center("O02").x(), popup.y() + popup.height() + 3.0).unwrap();
    fx.hover(below);
    assert_eq!(fx.row_at(below), None);
    // More options grow the popup under the stationary pointer.
    fx.p.options.set(choices(1..=6)).unwrap();
    frame(&mut fx.ui);
    let under = fx.row_at(below).expect("a row grew under the pointer");
    assert_ne!(under, "O02");
    assert_eq!(fx.highlighted().as_deref(), Some("O02"));
    fx.key(KeyCode::Enter);
    assert_eq!(fx.value(), values(&["o02"]));
    assert_eq!(fx.reports(), [values(&["o02"])]);
}

#[test]
fn reentering_the_window_at_the_same_position_over_the_same_row_is_stationary() {
    let mut fx = Fx::rows(20, EachToggle);
    fx.open();
    let at = fx.center("O03");
    fx.hover(at);
    fx.key(KeyCode::ArrowDown);
    assert_eq!(fx.highlighted().as_deref(), Some("O04"));
    fx.leave_window(at);
    fx.hover(at);
    assert_eq!(fx.row_at(at).as_deref(), Some("O03"));
    assert_eq!(fx.highlighted().as_deref(), Some("O04"));
    fx.key(KeyCode::Enter);
    assert_eq!(fx.value(), values(&["o04"]));
}

/// Accepted residual: a row the keyboard scrolled under the position where
/// the pointer left the window has not seen that position, so re-entering
/// there counts as motion over it.
#[test]
fn reentering_over_a_row_scrolled_there_meanwhile_counts_as_motion() {
    let mut fx = Fx::rows(20, EachToggle);
    fx.open();
    let at = fx.center("O03");
    fx.hover(at);
    fx.key(KeyCode::ArrowDown);
    fx.leave_window(at);
    fx.key(KeyCode::PageDown);
    assert_eq!(fx.highlighted().as_deref(), Some("O12"));
    let under = fx.row_at(at).unwrap();
    assert!(!["O03", "O12"].contains(&under.as_str()), "{under}");
    fx.hover(at);
    assert_eq!(fx.highlighted(), Some(under.clone()));
    fx.key(KeyCode::Enter);
    assert_eq!(fx.value(), [under.to_lowercase()]);
}

/// Accepted residual: a row remembers the position of the Leave layout sent
/// it when it moved away, so a single jump back to that position after the
/// keyboard scrolled the row back under it counts as standing still.
#[test]
fn a_row_scrolled_back_under_its_last_position_treats_a_jump_there_as_stationary() {
    let mut fx = Fx::rows(20, EachToggle);
    fx.open();
    let at = fx.center("O03");
    fx.hover(at);
    let o03 = fx.row("O03");
    fx.key(KeyCode::PageDown);
    assert_eq!(fx.highlighted().as_deref(), Some("O11"));
    assert_ne!(fx.row_at(at).as_deref(), Some("O03"), "O03 moved away");
    let search = fx.bounds(&fx.search());
    fx.hover(Point::new(search.x() + 10.0, search.y() + 5.0).unwrap());
    for _ in 0..10 {
        fx.key(KeyCode::ArrowUp);
    }
    assert_eq!(fx.highlighted().as_deref(), Some("O01"));
    assert_eq!(
        fx.row_at(at).as_deref(),
        Some("O03"),
        "O03 is back under the point"
    );
    assert_eq!(fx.row("O03"), o03, "the same retained row");
    fx.hover(at);
    assert_eq!(fx.highlighted().as_deref(), Some("O01"));
    fx.key(KeyCode::Enter);
    assert_eq!(fx.value(), values(&["o01"]));
}

// ---- Disabled options and a denied dropdown ----

fn with_disabled(count: usize, disabled: usize) -> Vec<Choice<String>> {
    choices(1..=count)
        .into_iter()
        .enumerate()
        .map(|(index, choice)| {
            if index + 1 == disabled {
                choice.disabled()
            } else {
                choice
            }
        })
        .collect()
}

#[test]
fn motion_over_a_disabled_option_takes_ownership_but_never_chooses_it() {
    let mut fx = Fx::new(with_disabled(6, 3), EachToggle);
    fx.open();
    fx.key(KeyCode::ArrowDown);
    assert_eq!(fx.highlighted().as_deref(), Some("O02"));
    let at = fx.center("O03");
    fx.hover(at);
    assert_eq!(
        fx.highlighted().as_deref(),
        Some("O02"),
        "a disabled row is not chosen"
    );
    // The motion did hand the highlight to the pointer: reversed options put
    // enabled O04 under it, and the highlight follows.
    let mut reversed = with_disabled(6, 3);
    reversed.reverse();
    fx.p.options.set(reversed).unwrap();
    frame(&mut fx.ui);
    assert_eq!(fx.row_at(at).as_deref(), Some("O04"));
    assert_eq!(fx.highlighted().as_deref(), Some("O04"));
    // Clicking the disabled option toggles nothing.
    fx.click(fx.center("O03"));
    assert!(fx.value().is_empty());
    assert!(fx.reports().is_empty());
    assert_eq!(fx.focus().as_deref(), Some("search"));
}

#[test]
fn a_press_on_a_disabled_option_takes_ownership_without_arming() {
    let mut fx = Fx::new(with_disabled(6, 3), EachToggle);
    fx.open();
    let at = fx.center("O03");
    fx.key(KeyCode::Escape);
    fx.hover(at);
    fx.open();
    fx.key(KeyCode::ArrowDown);
    assert_eq!(fx.highlighted().as_deref(), Some("O02"));
    // A press and release where the pointer rests, without motion.
    fx.press(at);
    fx.release(at);
    assert!(fx.value().is_empty());
    assert_eq!(fx.highlighted().as_deref(), Some("O02"));
    let mut reversed = with_disabled(6, 3);
    reversed.reverse();
    fx.p.options.set(reversed).unwrap();
    frame(&mut fx.ui);
    assert_eq!(fx.row_at(at).as_deref(), Some("O04"));
    assert_eq!(
        fx.highlighted().as_deref(),
        Some("O04"),
        "the press took ownership"
    );
    fx.key(KeyCode::Enter);
    assert_eq!(fx.value(), values(&["o04"]));
}

/// An incomplete click leaves focus where the press put it: on a long list,
/// the list itself.
#[test]
fn a_click_on_a_disabled_option_of_a_long_list_leaves_the_list_focused() {
    let mut fx = Fx::new(with_disabled(20, 3), EachToggle);
    fx.open();
    fx.click(fx.center("O03"));
    assert_eq!(fx.focus().as_deref(), Some("list"));
    assert!(fx.value().is_empty());
    assert!(fx.reports().is_empty());
}

#[test]
fn a_denied_dropdown_only_remembers_the_pointer_and_hands_back_the_highlight() {
    let mut fx = Fx::rows(6, EachToggle);
    fx.open();
    fx.hover(fx.center("O02"));
    assert_eq!(fx.highlighted().as_deref(), Some("O02"));
    fx.p.allowed.set(false).unwrap();
    frame(&mut fx.ui);
    let at = fx.center("O04");
    fx.hover(at);
    fx.click(at);
    assert!(fx.value().is_empty(), "a denied dropdown refuses the click");
    fx.p.allowed.set(true).unwrap();
    frame(&mut fx.ui);
    assert_eq!(fx.highlighted().as_deref(), Some("O02"));
    // The denied row remembered the position, so a host repeating it after
    // the restore is a resting pointer.
    fx.hover(at);
    assert_eq!(fx.highlighted().as_deref(), Some("O02"));
    // Reversed options put O03 under the stationary pointer.
    fx.p.options.set(choices((1..=6).rev())).unwrap();
    frame(&mut fx.ui);
    assert_eq!(fx.row_at(at).as_deref(), Some("O03"));
    assert_eq!(fx.highlighted().as_deref(), Some("O02"));
    fx.key(KeyCode::Enter);
    assert_eq!(fx.value(), values(&["o02"]));
    assert_eq!(fx.reports(), [values(&["o02"])]);
}

/// Motion while the dropdown is denied still counts once it is allowed again:
/// the row the pointer left remembers where its Leave took the pointer, so a
/// jump back to the row is motion.
#[test]
fn motion_made_while_denied_still_counts_after_the_restore() {
    let mut fx = Fx::rows(6, EachToggle);
    fx.open();
    let at = fx.center("O02");
    fx.hover(at);
    fx.key(KeyCode::ArrowDown);
    assert_eq!(fx.highlighted().as_deref(), Some("O03"));
    fx.p.allowed.set(false).unwrap();
    frame(&mut fx.ui);
    fx.hover(fx.center("O05"));
    fx.p.allowed.set(true).unwrap();
    frame(&mut fx.ui);
    assert_eq!(fx.highlighted().as_deref(), Some("O03"));
    fx.hover(at);
    assert_eq!(fx.highlighted().as_deref(), Some("O02"));
    fx.key(KeyCode::Enter);
    assert_eq!(fx.value(), values(&["o02"]));
}

/// With no rows mounted only the panel sees the dropdown being denied.
#[test]
fn a_denial_while_no_option_matches_still_hands_back_the_highlight() {
    let mut fx = Fx::rows(3, EachToggle);
    fx.open();
    let at = fx.center("O01");
    fx.hover(at);
    assert_eq!(fx.highlighted().as_deref(), Some("O01"));
    fx.p.options.set(Vec::new()).unwrap();
    frame(&mut fx.ui);
    assert!(fx.shown().is_empty());
    fx.p.allowed.set(false).unwrap();
    frame(&mut fx.ui);
    fx.p.allowed.set(true).unwrap();
    frame(&mut fx.ui);
    fx.p.options.set(choices([4, 1, 2, 3])).unwrap();
    frame(&mut fx.ui);
    assert_eq!(fx.row_at(at).as_deref(), Some("O04"));
    assert_eq!(fx.highlighted().as_deref(), Some("O01"));
    fx.key(KeyCode::Enter);
    assert_eq!(fx.value(), values(&["o01"]));
}

// ---- Closing, hiding and reopening ----

#[test]
fn closing_or_hiding_then_reopening_starts_with_the_keyboard_highlight() {
    for hide in [false, true] {
        let mut fx = Fx::rows(6, EachToggle);
        fx.open();
        let at = fx.center("O03");
        fx.hover(at);
        assert_eq!(fx.highlighted().as_deref(), Some("O03"));
        if hide {
            let tags = fx.tags();
            fx.ui.set_visible(&tags, false).unwrap();
            frame(&mut fx.ui);
            fx.ui.set_visible(&tags, true).unwrap();
            frame(&mut fx.ui);
        } else {
            fx.key(KeyCode::Escape);
        }
        assert!(!fx.is_open(), "hide={hide}");
        fx.open();
        assert_eq!(fx.row_at(at).as_deref(), Some("O03"));
        assert_eq!(fx.highlighted().as_deref(), Some("O01"), "hide={hide}");
        fx.key(KeyCode::ArrowDown);
        assert_eq!(fx.highlighted().as_deref(), Some("O02"), "hide={hide}");
        fx.key(KeyCode::Enter);
        assert_eq!(fx.value(), values(&["o02"]), "hide={hide}");
    }
}

// ---- A completed click returns focus to the search field ----

#[test]
fn a_completed_click_returns_focus_to_search_on_short_and_long_lists() {
    for count in [6, 20] {
        for commit in [EachToggle, OnClose] {
            let case = format!("{count} rows, {commit:?}");
            let mut fx = Fx::rows(count, commit);
            fx.open();
            fx.click(fx.center("O02"));
            assert_eq!(fx.focus().as_deref(), Some("search"), "{case}");
            assert_eq!(fx.highlighted().as_deref(), Some("O02"), "{case}");
            let bound = |chosen: &[&str]| {
                if commit == EachToggle {
                    values(chosen)
                } else {
                    Vec::new()
                }
            };
            assert_eq!(fx.value(), bound(&["o02"]), "{case}");
            fx.key(KeyCode::ArrowDown);
            assert_eq!(fx.highlighted().as_deref(), Some("O03"), "{case}");
            fx.key(KeyCode::Enter);
            assert_eq!(fx.value(), bound(&["o02", "o03"]), "{case}");
            fx.ui.text_input("O0").unwrap();
            frame(&mut fx.ui);
            assert_eq!(fx.query(), Some(SemanticValue::Text("O0".into())), "{case}");
            let reported: Vec<Vec<String>> = match commit {
                EachToggle => vec![values(&["o02"]), values(&["o02", "o03"])],
                _ => Vec::new(),
            };
            assert_eq!(fx.reports(), reported, "{case}");
            // OnClose writes its draft once, at a committing close.
            fx.click(Point::new(470.0, 550.0).unwrap());
            assert!(!fx.is_open(), "{case}");
            assert_eq!(fx.value(), values(&["o02", "o03"]), "{case}");
            let reports = fx.reports();
            assert_eq!(reports.last(), Some(&values(&["o02", "o03"])), "{case}");
            assert_eq!(
                reports.len(),
                if commit == EachToggle { 2 } else { 1 },
                "{case}"
            );
        }
    }
}

#[test]
fn a_click_returns_focus_to_search_after_tab_moved_it_to_the_list() {
    let mut fx = Fx::rows(20, EachToggle);
    fx.open();
    fx.key(KeyCode::Tab);
    assert_eq!(fx.focus().as_deref(), Some("list"));
    fx.click(fx.center("O02"));
    assert_eq!(fx.focus().as_deref(), Some("search"));
    assert_eq!(fx.value(), values(&["o02"]));
    fx.key(KeyCode::ArrowDown);
    fx.key(KeyCode::Enter);
    assert_eq!(fx.value(), values(&["o02", "o03"]));
}

/// The release regains the highlight for the pointer, so rows that layout
/// moves under it afterwards follow it again.
#[test]
fn a_release_after_a_search_key_regains_the_highlight_for_the_pointer() {
    for commit in [EachToggle, OnClose] {
        let mut fx = Fx::rows(6, commit);
        fx.open();
        let at = fx.center("O02");
        fx.hover(at);
        fx.press(at);
        fx.key(KeyCode::ArrowDown);
        assert_eq!(fx.highlighted().as_deref(), Some("O03"));
        assert_eq!(fx.focus().as_deref(), Some("search"));
        fx.release(at);
        assert_eq!(fx.highlighted().as_deref(), Some("O02"));
        let toggled = |chosen: &[&str]| {
            if commit == EachToggle {
                values(chosen)
            } else {
                Vec::new()
            }
        };
        assert_eq!(fx.value(), toggled(&["o02"]));
        assert_eq!(
            fx.reports(),
            if commit == EachToggle {
                vec![values(&["o02"])]
            } else {
                Vec::new()
            }
        );
        fx.p.options.set(choices((1..=6).rev())).unwrap();
        frame(&mut fx.ui);
        assert_eq!(fx.row_at(at).as_deref(), Some("O05"));
        assert_eq!(fx.highlighted().as_deref(), Some("O05"), "{commit:?}");
        fx.key(KeyCode::Enter);
        assert_eq!(fx.value(), toggled(&["o02", "o05"]), "{commit:?}");
    }
}

/// On a long list the press focuses the list, whose own Down scrolls the
/// captured row away from the pointer: the release is outside it and
/// cancels the click.
#[test]
fn a_long_list_press_scrolled_away_by_the_list_keys_cancels_the_click() {
    let mut fx = Fx::rows(20, EachToggle);
    fx.open();
    let at = fx.near_bottom("O03");
    fx.hover(at);
    fx.press(at);
    assert_eq!(fx.focus().as_deref(), Some("list"));
    fx.key(KeyCode::ArrowDown);
    assert_ne!(fx.row_at(at).as_deref(), Some("O03"), "O03 scrolled away");
    fx.release(at);
    assert!(fx.value().is_empty());
    assert!(fx.reports().is_empty());
    assert_eq!(fx.focus().as_deref(), Some("list"));
}

#[test]
fn a_release_outside_the_pressed_row_or_after_a_denial_returns_no_focus() {
    for deny in [false, true] {
        let mut fx = Fx::rows(20, EachToggle);
        fx.open();
        let at = fx.center("O02");
        fx.hover(at);
        fx.press(at);
        assert_eq!(fx.focus().as_deref(), Some("list"));
        let release = if deny {
            fx.p.allowed.set(false).unwrap();
            frame(&mut fx.ui);
            at
        } else {
            let away = fx.center("O05");
            fx.hover(away);
            away
        };
        fx.release(release);
        assert!(fx.value().is_empty(), "deny={deny}");
        assert!(fx.reports().is_empty(), "deny={deny}");
        assert_eq!(fx.focus().as_deref(), Some("list"), "deny={deny}");
    }
}

#[test]
fn a_focus_request_from_on_change_wins_over_the_return_to_search() {
    let mut fx = Fx::rows(20, EachToggle);
    fx.p.handler.set(Handler::FocusAfter);
    fx.open();
    fx.click(fx.center("O02"));
    assert_eq!(fx.focus().as_deref(), Some("after"));
    assert!(!fx.is_open(), "focus leaving the popup closes it");
    assert_eq!(fx.value(), values(&["o02"]));
    assert_eq!(fx.reports(), [values(&["o02"])]);
}

#[test]
fn a_failing_on_change_keeps_the_toggle_and_the_return_to_search() {
    let mut fx = Fx::rows(20, EachToggle);
    fx.p.handler.set(Handler::Fail);
    fx.open();
    let at = fx.center("O02");
    fx.hover(at);
    fx.press(at);
    assert_eq!(fx.focus().as_deref(), Some("list"));
    let released = fx.pointer(at, PointerKind::Release(PointerButton::Primary));
    assert!(
        matches!(released, Err(RuntimeError::BorrowConflict)),
        "{released:?}"
    );
    assert_eq!(fx.value(), values(&["o02"]), "the write stays");
    frame(&mut fx.ui);
    assert_eq!(fx.focus().as_deref(), Some("search"));
    frame(&mut fx.ui);
    assert_eq!(fx.reports().len(), 1, "nothing is replayed");
    assert!(fx.is_open());
}

#[test]
fn removing_the_chooser_during_a_held_click_releases_without_error_or_focus_change() {
    let mut fx = Fx::rows(20, EachToggle);
    fx.open();
    let at = fx.center("O02");
    fx.hover(at);
    fx.press(at);
    fx.remove_chooser();
    let focused = fx.ui.focused();
    fx.pointer(at, PointerKind::Release(PointerButton::Primary))
        .unwrap();
    frame(&mut fx.ui);
    assert_eq!(fx.ui.focused(), focused);
    assert!(fx.value().is_empty());
    assert!(fx.reports().is_empty());
}

/// Pressing a long list moves focus to the list, which ends an unfinished
/// composition in the search field; the completed click returns focus but
/// not the composition. A short list keeps both.
#[test]
fn a_press_that_focuses_the_long_list_drops_the_search_composition() {
    for count in [6, 20] {
        let mut fx = Fx::rows(count, EachToggle);
        fx.open();
        fx.ui
            .text_preedit(&TextPreedit::new("ka", None).unwrap())
            .unwrap();
        frame(&mut fx.ui);
        assert!(fx.composing());
        fx.click(fx.center("O02"));
        assert_eq!(fx.focus().as_deref(), Some("search"), "{count} rows");
        assert_eq!(fx.composing(), count == 6, "{count} rows");
        assert_eq!(fx.value(), values(&["o02"]), "{count} rows");
    }
}
